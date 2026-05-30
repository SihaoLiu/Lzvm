use std::fmt;

use crate::guest_instruction::{
    decode_guest_instruction, fetch_guest_instruction, GuestInstructionError, RiscvBranchKind,
    RiscvEncodedInstruction, RiscvInstruction, RiscvOpImmKind, RiscvOpKind,
};
use crate::guest_memory::GuestMemoryImage;

const GUEST_REGISTER_COUNT: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineState {
    pc: u64,
    registers: [u64; GUEST_REGISTER_COUNT],
}

impl GuestMachineState {
    pub fn new(entry_address: u64) -> Self {
        Self {
            pc: entry_address,
            registers: [0; GUEST_REGISTER_COUNT],
        }
    }

    pub fn pc(&self) -> u64 {
        self.pc
    }

    pub fn register(&self, index: usize) -> Option<u64> {
        self.registers.get(index).copied()
    }

    pub fn registers(&self) -> &[u64; GUEST_REGISTER_COUNT] {
        &self.registers
    }

    pub fn set_register(&mut self, index: usize, value: u64) -> Result<(), GuestMachineError> {
        if index >= GUEST_REGISTER_COUNT {
            return Err(GuestMachineError::InvalidRegisterIndex { index });
        }
        if index != 0 {
            self.registers[index] = value;
        }
        Ok(())
    }

    fn set_pc(&mut self, pc: u64) {
        self.pc = pc;
    }

    fn write_decoded_register(&mut self, index: u8, value: u64) {
        let index = usize::from(index);
        if index != 0 {
            self.registers[index] = value;
        }
    }

    fn read_decoded_register(&self, index: u8) -> u64 {
        self.registers[usize::from(index)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineReport {
    pub address: u64,
    pub instruction: RiscvInstruction,
    pub next_pc: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestMachineError {
    Fetch(GuestInstructionError),
    InvalidRegisterIndex {
        index: usize,
    },
    ProgramCounterOverflow {
        address: u64,
        byte_len: usize,
    },
    UnsupportedInstructionLength {
        address: u64,
        halfword: u16,
    },
    UnsupportedInstruction {
        address: u64,
        instruction: RiscvInstruction,
    },
}

impl fmt::Display for GuestMachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch(error) => write!(f, "guest machine instruction fetch failed: {error}"),
            Self::InvalidRegisterIndex { index } => {
                write!(f, "guest machine register index is invalid: {index}")
            }
            Self::ProgramCounterOverflow { address, byte_len } => write!(
                f,
                "guest machine program counter overflows: address {address}, byte length {byte_len}"
            ),
            Self::UnsupportedInstructionLength { address, halfword } => write!(
                f,
                "guest machine instruction length is unsupported: address {address}, halfword {halfword:#06x}"
            ),
            Self::UnsupportedInstruction {
                address,
                instruction,
            } => write!(
                f,
                "guest machine instruction is unsupported: address {address}, instruction {instruction:?}"
            ),
        }
    }
}

impl std::error::Error for GuestMachineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fetch(error) => Some(error),
            Self::InvalidRegisterIndex { .. }
            | Self::ProgramCounterOverflow { .. }
            | Self::UnsupportedInstructionLength { .. }
            | Self::UnsupportedInstruction { .. } => None,
        }
    }
}

impl From<GuestInstructionError> for GuestMachineError {
    fn from(error: GuestInstructionError) -> Self {
        Self::Fetch(error)
    }
}

pub fn advance_guest_machine(
    memory: &GuestMemoryImage,
    state: &mut GuestMachineState,
) -> Result<GuestMachineReport, GuestMachineError> {
    let address = state.pc();
    let fetched = fetch_guest_instruction(memory, address)?;
    let byte_len = match fetched.byte_len() {
        Some(byte_len) => byte_len,
        None => {
            let RiscvEncodedInstruction::UnsupportedLong(halfword) = fetched.encoded else {
                unreachable!("instruction encoding without byte length must be explicit")
            };
            return Err(GuestMachineError::UnsupportedInstructionLength { address, halfword });
        }
    };
    let sequential_pc = address
        .checked_add(byte_len as u64)
        .ok_or(GuestMachineError::ProgramCounterOverflow { address, byte_len })?;
    let instruction = decode_guest_instruction(fetched);
    let mut next_state = state.clone();
    next_state.set_pc(sequential_pc);

    execute_guest_instruction(address, sequential_pc, instruction, &mut next_state)?;
    let next_pc = next_state.pc();
    *state = next_state;

    Ok(GuestMachineReport {
        address,
        instruction,
        next_pc,
    })
}

fn execute_guest_instruction(
    address: u64,
    sequential_pc: u64,
    instruction: RiscvInstruction,
    state: &mut GuestMachineState,
) -> Result<(), GuestMachineError> {
    match instruction {
        RiscvInstruction::Lui { rd, immediate } => {
            state.write_decoded_register(rd, immediate as u64);
        }
        RiscvInstruction::Auipc { rd, immediate } => {
            state.write_decoded_register(rd, address.wrapping_add_signed(immediate));
        }
        RiscvInstruction::Jal { rd, offset } => {
            state.write_decoded_register(rd, sequential_pc);
            state.set_pc(address.wrapping_add_signed(offset));
        }
        RiscvInstruction::Jalr { rd, rs1, offset } => {
            let target = state.read_decoded_register(rs1).wrapping_add_signed(offset) & !1;
            state.write_decoded_register(rd, sequential_pc);
            state.set_pc(target);
        }
        RiscvInstruction::Branch {
            kind,
            rs1,
            rs2,
            offset,
        } => {
            if branch_is_taken(
                kind,
                state.read_decoded_register(rs1),
                state.read_decoded_register(rs2),
            ) {
                state.set_pc(address.wrapping_add_signed(offset));
            }
        }
        RiscvInstruction::OpImm {
            kind,
            rd,
            rs1,
            immediate,
        } => {
            let value = execute_op_imm(kind, state.read_decoded_register(rs1), immediate).ok_or(
                GuestMachineError::UnsupportedInstruction {
                    address,
                    instruction,
                },
            )?;
            state.write_decoded_register(rd, value);
        }
        RiscvInstruction::Op { kind, rd, rs1, rs2 } => {
            let value = execute_op(
                kind,
                state.read_decoded_register(rs1),
                state.read_decoded_register(rs2),
            )
            .ok_or(GuestMachineError::UnsupportedInstruction {
                address,
                instruction,
            })?;
            state.write_decoded_register(rd, value);
        }
        RiscvInstruction::CompressedUnknown { .. }
        | RiscvInstruction::IllegalCompressed { .. }
        | RiscvInstruction::UnsupportedLong { .. }
        | RiscvInstruction::Load { .. }
        | RiscvInstruction::Store { .. }
        | RiscvInstruction::OpImm32 { .. }
        | RiscvInstruction::Op32 { .. }
        | RiscvInstruction::Fence { .. }
        | RiscvInstruction::Ecall
        | RiscvInstruction::Ebreak
        | RiscvInstruction::Unknown { .. } => {
            return Err(GuestMachineError::UnsupportedInstruction {
                address,
                instruction,
            });
        }
    }

    Ok(())
}

fn branch_is_taken(kind: RiscvBranchKind, lhs: u64, rhs: u64) -> bool {
    match kind {
        RiscvBranchKind::Beq => lhs == rhs,
        RiscvBranchKind::Bne => lhs != rhs,
        RiscvBranchKind::Blt => (lhs as i64) < (rhs as i64),
        RiscvBranchKind::Bge => (lhs as i64) >= (rhs as i64),
        RiscvBranchKind::Bltu => lhs < rhs,
        RiscvBranchKind::Bgeu => lhs >= rhs,
    }
}

fn execute_op_imm(kind: RiscvOpImmKind, rs1: u64, immediate: i64) -> Option<u64> {
    let value = match kind {
        RiscvOpImmKind::Addi => rs1.wrapping_add_signed(immediate),
        RiscvOpImmKind::Slti => u64::from((rs1 as i64) < immediate),
        RiscvOpImmKind::Sltiu => u64::from(rs1 < immediate as u64),
        RiscvOpImmKind::Xori => rs1 ^ immediate as u64,
        RiscvOpImmKind::Ori => rs1 | immediate as u64,
        RiscvOpImmKind::Andi => rs1 & immediate as u64,
        RiscvOpImmKind::Slli => rs1.wrapping_shl((immediate as u32) & 0x3f),
        RiscvOpImmKind::Srli => rs1.wrapping_shr((immediate as u32) & 0x3f),
        RiscvOpImmKind::Srai => ((rs1 as i64) >> ((immediate as u32) & 0x3f)) as u64,
    };
    Some(value)
}

fn execute_op(kind: RiscvOpKind, rs1: u64, rs2: u64) -> Option<u64> {
    match kind {
        RiscvOpKind::Add => Some(rs1.wrapping_add(rs2)),
        RiscvOpKind::Sub => Some(rs1.wrapping_sub(rs2)),
        RiscvOpKind::Sll => Some(rs1.wrapping_shl((rs2 as u32) & 0x3f)),
        RiscvOpKind::Slt => Some(u64::from((rs1 as i64) < (rs2 as i64))),
        RiscvOpKind::Sltu => Some(u64::from(rs1 < rs2)),
        RiscvOpKind::Xor => Some(rs1 ^ rs2),
        RiscvOpKind::Srl => Some(rs1.wrapping_shr((rs2 as u32) & 0x3f)),
        RiscvOpKind::Sra => Some(((rs1 as i64) >> ((rs2 as u32) & 0x3f)) as u64),
        RiscvOpKind::Or => Some(rs1 | rs2),
        RiscvOpKind::And => Some(rs1 & rs2),
        RiscvOpKind::Mul
        | RiscvOpKind::Mulh
        | RiscvOpKind::Mulhsu
        | RiscvOpKind::Mulhu
        | RiscvOpKind::Div
        | RiscvOpKind::Divu
        | RiscvOpKind::Rem
        | RiscvOpKind::Remu => None,
    }
}

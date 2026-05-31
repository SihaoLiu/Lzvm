use std::fmt;

use crate::guest_instruction::{
    RiscvBranchKind, RiscvInstruction, RiscvLoadKind, RiscvOp32Kind, RiscvOpImm32Kind,
    RiscvOpImmKind, RiscvOpKind, RiscvStoreKind,
};
use crate::guest_machine::GuestMachineReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZiskMainSource {
    LastC,
    Memory(u64),
    Immediate(u64),
    Register(u8),
    Indirect(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZiskMainStore {
    None,
    Memory(i64),
    Register(u8),
    Indirect(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZiskMainOp {
    Flag,
    CopyB,
    Ltu,
    Lt,
    Eq,
    Add,
    Sub,
    AddW,
    SubW,
    And,
    Or,
    Xor,
    Sll,
    Srl,
    Sra,
    SllW,
    SrlW,
    SraW,
    SignExtendB,
    SignExtendH,
    SignExtendW,
}

impl ZiskMainOp {
    pub const fn code(self) -> u8 {
        match self {
            Self::Flag => 0x00,
            Self::CopyB => 0x01,
            Self::Ltu => 0x06,
            Self::Lt => 0x07,
            Self::Eq => 0x09,
            Self::Add => 0x0a,
            Self::Sub => 0x0b,
            Self::AddW => 0x1a,
            Self::SubW => 0x1b,
            Self::And => 0x0e,
            Self::Or => 0x0f,
            Self::Xor => 0x10,
            Self::Sll => 0x21,
            Self::Srl => 0x22,
            Self::Sra => 0x23,
            Self::SllW => 0x24,
            Self::SrlW => 0x25,
            Self::SraW => 0x26,
            Self::SignExtendB => 0x27,
            Self::SignExtendH => 0x28,
            Self::SignExtendW => 0x29,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZiskMainInstruction {
    pub pc: u64,
    pub a: ZiskMainSource,
    pub b: ZiskMainSource,
    pub op: ZiskMainOp,
    pub store: ZiskMainStore,
    pub store_pc: bool,
    pub set_pc: bool,
    pub jmp_offset1: i64,
    pub jmp_offset2: i64,
    pub ind_width: u64,
    pub m32: bool,
    pub is_external_op: bool,
    pub is_precompiled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZiskMainLowerError {
    InvalidInstructionByteLen {
        pc: u64,
        byte_len: usize,
    },
    InconsistentSequentialNextPc {
        pc: u64,
        next_pc: u64,
        instruction_byte_len: usize,
    },
    UnsupportedInstruction {
        instruction: RiscvInstruction,
    },
}

impl fmt::Display for ZiskMainLowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInstructionByteLen { pc, byte_len } => write!(
                f,
                "Zisk Main lowering instruction at pc {pc} has invalid byte length {byte_len}"
            ),
            Self::InconsistentSequentialNextPc {
                pc,
                next_pc,
                instruction_byte_len,
            } => write!(
                f,
                "Zisk Main lowering instruction at pc {pc} has next pc {next_pc}, which is not consistent with byte length {instruction_byte_len}"
            ),
            Self::UnsupportedInstruction { instruction } => {
                write!(
                    f,
                    "Zisk Main lowering does not support instruction {instruction:?}"
                )
            }
        }
    }
}

impl std::error::Error for ZiskMainLowerError {}

pub fn lower_guest_report(
    report: &GuestMachineReport,
) -> Result<ZiskMainInstruction, ZiskMainLowerError> {
    let instruction_size = instruction_size_offset(report.address, report.instruction_byte_len)?;
    match report.instruction {
        RiscvInstruction::Branch {
            kind,
            rs1,
            rs2,
            offset,
        } => {
            return Ok(lower_branch(
                report.address,
                instruction_size,
                kind,
                rs1,
                rs2,
                offset,
            ));
        }
        RiscvInstruction::Jal { rd, offset } => {
            return Ok(lower_jal(report.address, instruction_size, rd, offset));
        }
        RiscvInstruction::Jalr { rd, rs1, offset } => {
            return lower_jalr(report.address, instruction_size, rd, rs1, offset);
        }
        _ => {}
    }
    validate_sequential_next_pc(report)?;
    match report.instruction {
        RiscvInstruction::Lui { rd, immediate } => {
            Ok(lower_lui(report.address, instruction_size, rd, immediate))
        }
        RiscvInstruction::Auipc { rd, immediate } => {
            Ok(lower_auipc(report.address, instruction_size, rd, immediate))
        }
        RiscvInstruction::OpImm {
            kind,
            rd,
            rs1,
            immediate,
        } => match kind {
            RiscvOpImmKind::Addi => Ok(lower_addi(
                report.address,
                instruction_size,
                rd,
                rs1,
                immediate,
            )),
            _ => Ok(binary_immediate_op(
                report.address,
                instruction_size,
                rd,
                rs1,
                immediate,
                op_imm_kind(kind),
            )),
        },
        RiscvInstruction::OpImm32 {
            kind,
            rd,
            rs1,
            immediate,
        } => Ok(binary_immediate_word_op(
            report.address,
            instruction_size,
            rd,
            rs1,
            immediate,
            op_imm_32_kind(kind),
        )),
        RiscvInstruction::Op { kind, rd, rs1, rs2 } => match op_kind(kind) {
            Some(op) => Ok(binary_register_op(
                report.address,
                instruction_size,
                rd,
                rs1,
                rs2,
                op,
            )),
            None => Err(ZiskMainLowerError::UnsupportedInstruction {
                instruction: report.instruction,
            }),
        },
        RiscvInstruction::Op32 { kind, rd, rs1, rs2 } => match op_32_kind(kind) {
            Some(op) => Ok(binary_register_word_op(
                report.address,
                instruction_size,
                rd,
                rs1,
                rs2,
                op,
            )),
            None => Err(ZiskMainLowerError::UnsupportedInstruction {
                instruction: report.instruction,
            }),
        },
        RiscvInstruction::Load {
            kind,
            rd,
            rs1,
            offset,
        } => match load_op_width(kind) {
            Some((op, width)) => Ok(lower_load(
                report.address,
                instruction_size,
                rd,
                rs1,
                offset,
                op,
                width,
            )),
            None => Err(ZiskMainLowerError::UnsupportedInstruction {
                instruction: report.instruction,
            }),
        },
        RiscvInstruction::Store {
            kind,
            rs1,
            rs2,
            offset,
        } => Ok(lower_store(
            report.address,
            instruction_size,
            rs1,
            rs2,
            offset,
            store_width(kind),
        )),
        _ => Err(ZiskMainLowerError::UnsupportedInstruction {
            instruction: report.instruction,
        }),
    }
}

fn validate_sequential_next_pc(report: &GuestMachineReport) -> Result<(), ZiskMainLowerError> {
    let expected_next_pc = report
        .address
        .checked_add(report.instruction_byte_len as u64)
        .filter(|next_pc| *next_pc == report.next_pc);
    match expected_next_pc {
        Some(_) => Ok(()),
        None => Err(ZiskMainLowerError::InconsistentSequentialNextPc {
            pc: report.address,
            next_pc: report.next_pc,
            instruction_byte_len: report.instruction_byte_len,
        }),
    }
}

fn instruction_size_offset(pc: u64, byte_len: usize) -> Result<i64, ZiskMainLowerError> {
    match byte_len {
        2 | 4 => Ok(byte_len as i64),
        _ => Err(ZiskMainLowerError::InvalidInstructionByteLen { pc, byte_len }),
    }
}

fn lower_addi(
    pc: u64,
    instruction_size: i64,
    rd: u8,
    rs1: u8,
    immediate: i64,
) -> ZiskMainInstruction {
    if rd == 0 {
        return base_instruction(
            pc,
            register_source(rs1),
            ZiskMainSource::Immediate(immediate as u64),
            ZiskMainOp::Flag,
            ZiskMainStore::None,
            instruction_size,
        );
    }
    if rs1 == 0 {
        return base_instruction(
            pc,
            ZiskMainSource::Immediate(0),
            ZiskMainSource::Immediate(immediate as u64),
            ZiskMainOp::CopyB,
            register_store(rd),
            instruction_size,
        );
    }
    if immediate == 0 {
        return base_instruction(
            pc,
            ZiskMainSource::Immediate(0),
            register_source(rs1),
            ZiskMainOp::CopyB,
            register_store(rd),
            instruction_size,
        );
    }
    base_instruction(
        pc,
        register_source(rs1),
        ZiskMainSource::Immediate(immediate as u64),
        ZiskMainOp::Add,
        register_store(rd),
        instruction_size,
    )
}

fn binary_register_op(
    pc: u64,
    instruction_size: i64,
    rd: u8,
    rs1: u8,
    rs2: u8,
    op: ZiskMainOp,
) -> ZiskMainInstruction {
    base_instruction(
        pc,
        register_source(rs1),
        register_source(rs2),
        op,
        register_store(rd),
        instruction_size,
    )
}

fn binary_immediate_op(
    pc: u64,
    instruction_size: i64,
    rd: u8,
    rs1: u8,
    immediate: i64,
    op: ZiskMainOp,
) -> ZiskMainInstruction {
    base_instruction(
        pc,
        register_source(rs1),
        ZiskMainSource::Immediate(immediate as u64),
        op,
        register_store(rd),
        instruction_size,
    )
}

fn binary_register_word_op(
    pc: u64,
    instruction_size: i64,
    rd: u8,
    rs1: u8,
    rs2: u8,
    op: ZiskMainOp,
) -> ZiskMainInstruction {
    let mut instruction = binary_register_op(pc, instruction_size, rd, rs1, rs2, op);
    instruction.m32 = true;
    instruction
}

fn binary_immediate_word_op(
    pc: u64,
    instruction_size: i64,
    rd: u8,
    rs1: u8,
    immediate: i64,
    op: ZiskMainOp,
) -> ZiskMainInstruction {
    let mut instruction = binary_immediate_op(pc, instruction_size, rd, rs1, immediate, op);
    instruction.m32 = true;
    instruction
}

fn lower_load(
    pc: u64,
    instruction_size: i64,
    rd: u8,
    rs1: u8,
    offset: i64,
    op: ZiskMainOp,
    width: u64,
) -> ZiskMainInstruction {
    let mut instruction = base_instruction(
        pc,
        register_source(rs1),
        ZiskMainSource::Indirect(offset),
        op,
        register_store(rd),
        instruction_size,
    );
    instruction.ind_width = width;
    instruction
}

fn lower_store(
    pc: u64,
    instruction_size: i64,
    rs1: u8,
    rs2: u8,
    offset: i64,
    width: u64,
) -> ZiskMainInstruction {
    let mut instruction = base_instruction(
        pc,
        register_source(rs1),
        register_source(rs2),
        ZiskMainOp::CopyB,
        ZiskMainStore::Indirect(offset),
        instruction_size,
    );
    instruction.ind_width = width;
    instruction
}

fn lower_lui(pc: u64, instruction_size: i64, rd: u8, immediate: i64) -> ZiskMainInstruction {
    base_instruction(
        pc,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Immediate(immediate as u64),
        ZiskMainOp::CopyB,
        register_store(rd),
        instruction_size,
    )
}

fn lower_auipc(pc: u64, instruction_size: i64, rd: u8, immediate: i64) -> ZiskMainInstruction {
    let (store, store_pc) = register_pc_store(rd);
    let mut instruction = base_instruction(
        pc,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Immediate(0),
        ZiskMainOp::Flag,
        store,
        instruction_size,
    );
    instruction.store_pc = store_pc;
    instruction.jmp_offset1 = instruction_size;
    instruction.jmp_offset2 = immediate;
    instruction
}

fn lower_jal(pc: u64, instruction_size: i64, rd: u8, offset: i64) -> ZiskMainInstruction {
    let (store, store_pc) = register_pc_store(rd);
    let mut instruction = base_instruction(
        pc,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Immediate(0),
        ZiskMainOp::Flag,
        store,
        instruction_size,
    );
    instruction.store_pc = store_pc;
    instruction.jmp_offset1 = offset;
    instruction.jmp_offset2 = instruction_size;
    instruction
}

fn lower_jalr(
    pc: u64,
    instruction_size: i64,
    rd: u8,
    rs1: u8,
    offset: i64,
) -> Result<ZiskMainInstruction, ZiskMainLowerError> {
    if offset % 2 != 0 {
        return Err(ZiskMainLowerError::UnsupportedInstruction {
            instruction: RiscvInstruction::Jalr { rd, rs1, offset },
        });
    }
    let (store, store_pc) = register_pc_store(rd);
    let mut instruction = base_instruction(
        pc,
        ZiskMainSource::Immediate(!1),
        register_source(rs1),
        ZiskMainOp::And,
        store,
        instruction_size,
    );
    instruction.store_pc = store_pc;
    instruction.set_pc = true;
    instruction.jmp_offset1 = offset;
    instruction.jmp_offset2 = instruction_size;
    Ok(instruction)
}

fn lower_branch(
    pc: u64,
    instruction_size: i64,
    kind: RiscvBranchKind,
    rs1: u8,
    rs2: u8,
    offset: i64,
) -> ZiskMainInstruction {
    let (op, jmp_offset1, jmp_offset2) = branch_op_offsets(kind, instruction_size, offset);
    let mut instruction = base_instruction(
        pc,
        register_source(rs1),
        register_source(rs2),
        op,
        ZiskMainStore::None,
        instruction_size,
    );
    instruction.jmp_offset1 = jmp_offset1;
    instruction.jmp_offset2 = jmp_offset2;
    instruction
}

fn branch_op_offsets(
    kind: RiscvBranchKind,
    instruction_size: i64,
    offset: i64,
) -> (ZiskMainOp, i64, i64) {
    match kind {
        RiscvBranchKind::Beq => (ZiskMainOp::Eq, offset, instruction_size),
        RiscvBranchKind::Bne => (ZiskMainOp::Eq, instruction_size, offset),
        RiscvBranchKind::Blt => (ZiskMainOp::Lt, offset, instruction_size),
        RiscvBranchKind::Bge => (ZiskMainOp::Lt, instruction_size, offset),
        RiscvBranchKind::Bltu => (ZiskMainOp::Ltu, offset, instruction_size),
        RiscvBranchKind::Bgeu => (ZiskMainOp::Ltu, instruction_size, offset),
    }
}

fn load_op_width(kind: RiscvLoadKind) -> Option<(ZiskMainOp, u64)> {
    match kind {
        RiscvLoadKind::Lb => Some((ZiskMainOp::SignExtendB, 1)),
        RiscvLoadKind::Lh => Some((ZiskMainOp::SignExtendH, 2)),
        RiscvLoadKind::Lw => Some((ZiskMainOp::SignExtendW, 4)),
        RiscvLoadKind::Lbu => Some((ZiskMainOp::CopyB, 1)),
        RiscvLoadKind::Lhu => Some((ZiskMainOp::CopyB, 2)),
        RiscvLoadKind::Lwu => Some((ZiskMainOp::CopyB, 4)),
        RiscvLoadKind::Ld => Some((ZiskMainOp::CopyB, 8)),
    }
}

fn op_imm_kind(kind: RiscvOpImmKind) -> ZiskMainOp {
    match kind {
        RiscvOpImmKind::Addi => ZiskMainOp::Add,
        RiscvOpImmKind::Slti => ZiskMainOp::Lt,
        RiscvOpImmKind::Sltiu => ZiskMainOp::Ltu,
        RiscvOpImmKind::Xori => ZiskMainOp::Xor,
        RiscvOpImmKind::Ori => ZiskMainOp::Or,
        RiscvOpImmKind::Andi => ZiskMainOp::And,
        RiscvOpImmKind::Slli => ZiskMainOp::Sll,
        RiscvOpImmKind::Srli => ZiskMainOp::Srl,
        RiscvOpImmKind::Srai => ZiskMainOp::Sra,
    }
}

fn op_imm_32_kind(kind: RiscvOpImm32Kind) -> ZiskMainOp {
    match kind {
        RiscvOpImm32Kind::Addiw => ZiskMainOp::AddW,
        RiscvOpImm32Kind::Slliw => ZiskMainOp::SllW,
        RiscvOpImm32Kind::Srliw => ZiskMainOp::SrlW,
        RiscvOpImm32Kind::Sraiw => ZiskMainOp::SraW,
    }
}

fn op_kind(kind: RiscvOpKind) -> Option<ZiskMainOp> {
    match kind {
        RiscvOpKind::Add => Some(ZiskMainOp::Add),
        RiscvOpKind::Sub => Some(ZiskMainOp::Sub),
        RiscvOpKind::Sll => Some(ZiskMainOp::Sll),
        RiscvOpKind::Slt => Some(ZiskMainOp::Lt),
        RiscvOpKind::Sltu => Some(ZiskMainOp::Ltu),
        RiscvOpKind::Xor => Some(ZiskMainOp::Xor),
        RiscvOpKind::Srl => Some(ZiskMainOp::Srl),
        RiscvOpKind::Sra => Some(ZiskMainOp::Sra),
        RiscvOpKind::Or => Some(ZiskMainOp::Or),
        RiscvOpKind::And => Some(ZiskMainOp::And),
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

fn op_32_kind(kind: RiscvOp32Kind) -> Option<ZiskMainOp> {
    match kind {
        RiscvOp32Kind::Addw => Some(ZiskMainOp::AddW),
        RiscvOp32Kind::Subw => Some(ZiskMainOp::SubW),
        RiscvOp32Kind::Sllw => Some(ZiskMainOp::SllW),
        RiscvOp32Kind::Srlw => Some(ZiskMainOp::SrlW),
        RiscvOp32Kind::Sraw => Some(ZiskMainOp::SraW),
        RiscvOp32Kind::Mulw
        | RiscvOp32Kind::Divw
        | RiscvOp32Kind::Divuw
        | RiscvOp32Kind::Remw
        | RiscvOp32Kind::Remuw => None,
    }
}

fn store_width(kind: RiscvStoreKind) -> u64 {
    match kind {
        RiscvStoreKind::Sb => 1,
        RiscvStoreKind::Sh => 2,
        RiscvStoreKind::Sw => 4,
        RiscvStoreKind::Sd => 8,
    }
}

fn base_instruction(
    pc: u64,
    a: ZiskMainSource,
    b: ZiskMainSource,
    op: ZiskMainOp,
    store: ZiskMainStore,
    instruction_size: i64,
) -> ZiskMainInstruction {
    ZiskMainInstruction {
        pc,
        a,
        b,
        op,
        store,
        store_pc: false,
        set_pc: false,
        jmp_offset1: instruction_size,
        jmp_offset2: instruction_size,
        ind_width: 0,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    }
}

fn register_source(index: u8) -> ZiskMainSource {
    if index == 0 {
        ZiskMainSource::Immediate(0)
    } else {
        ZiskMainSource::Register(index)
    }
}

fn register_store(index: u8) -> ZiskMainStore {
    if index == 0 {
        ZiskMainStore::None
    } else {
        ZiskMainStore::Register(index)
    }
}

fn register_pc_store(index: u8) -> (ZiskMainStore, bool) {
    let store = register_store(index);
    let store_pc = matches!(store, ZiskMainStore::Register(_));
    (store, store_pc)
}

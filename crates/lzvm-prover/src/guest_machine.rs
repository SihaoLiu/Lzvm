use std::collections::BTreeMap;
use std::fmt;

use crate::guest_instruction::{
    decode_guest_instruction, fetch_guest_instruction, GuestInstructionError, RiscvAmoKind,
    RiscvAmoWidth, RiscvBranchKind, RiscvEncodedInstruction, RiscvInstruction, RiscvLoadKind,
    RiscvOp32Kind, RiscvOpImm32Kind, RiscvOpImmKind, RiscvOpKind, RiscvStoreKind,
};
use crate::guest_memory::{
    GuestMemoryError, GuestMemoryImage, GuestMemoryReader, GuestMemorySegment,
};

const GUEST_REGISTER_COUNT: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineMemory {
    entry_address: u64,
    segments: Vec<GuestMachineMemorySegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestMachineMemorySegment {
    program_header_index: u16,
    virtual_address: u64,
    memory_size: u64,
    initialized_bytes: Vec<u8>,
    written_bytes: BTreeMap<u64, u8>,
}

impl GuestMachineMemory {
    pub fn from_image(image: &GuestMemoryImage) -> Self {
        Self {
            entry_address: image.entry_address(),
            segments: image
                .segments()
                .iter()
                .map(GuestMachineMemorySegment::from_image_segment)
                .collect(),
        }
    }

    pub fn entry_address(&self) -> u64 {
        self.entry_address
    }

    pub fn read_range_into(&self, address: u64, bytes: &mut [u8]) -> Result<(), GuestMemoryError> {
        let byte_len = bytes.len();
        let end_address = checked_address_end(address, byte_len)?;
        for segment in &self.segments {
            let segment_end = segment.end_address()?;
            if address >= segment.virtual_address && end_address <= segment_end {
                segment.read_range_into(address, bytes);
                return Ok(());
            }
        }
        Err(GuestMemoryError::AddressNotMapped { address, byte_len })
    }

    pub fn write_range(&mut self, address: u64, bytes: &[u8]) -> Result<(), GuestMemoryError> {
        let byte_len = bytes.len();
        let end_address = checked_address_end(address, byte_len)?;
        for segment in &mut self.segments {
            let segment_end = segment.end_address()?;
            if address >= segment.virtual_address && end_address <= segment_end {
                segment.write_range(address, bytes);
                return Ok(());
            }
        }
        Err(GuestMemoryError::AddressNotMapped { address, byte_len })
    }
}

impl GuestMemoryReader for GuestMachineMemory {
    fn read_range_into(&self, address: u64, bytes: &mut [u8]) -> Result<(), GuestMemoryError> {
        GuestMachineMemory::read_range_into(self, address, bytes)
    }
}

impl GuestMachineMemorySegment {
    fn from_image_segment(segment: &GuestMemorySegment) -> Self {
        Self {
            program_header_index: segment.program_header_index(),
            virtual_address: segment.virtual_address(),
            memory_size: segment.memory_size(),
            initialized_bytes: segment.initialized_bytes().to_vec(),
            written_bytes: BTreeMap::new(),
        }
    }

    fn read_range_into(&self, address: u64, bytes: &mut [u8]) {
        let start = address - self.virtual_address;
        let initialized_len = self.initialized_bytes.len() as u64;
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = start + index as u64;
            *byte = self
                .written_bytes
                .get(&offset)
                .copied()
                .or_else(|| {
                    (offset < initialized_len).then(|| {
                        self.initialized_bytes
                            [usize::try_from(offset).expect("initialized offset fits usize")]
                    })
                })
                .unwrap_or(0);
        }
    }

    fn write_range(&mut self, address: u64, bytes: &[u8]) {
        let start = address - self.virtual_address;
        for (index, byte) in bytes.iter().copied().enumerate() {
            self.written_bytes.insert(start + index as u64, byte);
        }
    }

    fn end_address(&self) -> Result<u64, GuestMemoryError> {
        self.virtual_address.checked_add(self.memory_size).ok_or(
            GuestMemoryError::SegmentMemoryRangeOverflow {
                program_header_index: self.program_header_index,
                virtual_address: self.virtual_address,
                memory_size: self.memory_size,
            },
        )
    }
}

fn checked_address_end(address: u64, byte_len: usize) -> Result<u64, GuestMemoryError> {
    let byte_len_u64 = u64::try_from(byte_len)
        .map_err(|_| GuestMemoryError::AddressRangeOverflow { address, byte_len })?;
    address
        .checked_add(byte_len_u64)
        .ok_or(GuestMemoryError::AddressRangeOverflow { address, byte_len })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineState {
    pc: u64,
    registers: [u64; GUEST_REGISTER_COUNT],
    reservation: Option<GuestMemoryReservation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GuestMemoryReservation {
    address: u64,
    width: RiscvAmoWidth,
}

impl GuestMachineState {
    pub fn new(entry_address: u64) -> Self {
        Self {
            pc: entry_address,
            registers: [0; GUEST_REGISTER_COUNT],
            reservation: None,
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

    fn set_reservation(&mut self, address: u64, width: RiscvAmoWidth) {
        self.reservation = Some(GuestMemoryReservation { address, width });
    }

    fn clear_reservation(&mut self) {
        self.reservation = None;
    }

    fn clear_reservation_if_overlaps(&mut self, address: u64, byte_len: usize) {
        if self.reservation_overlaps(address, byte_len) {
            self.clear_reservation();
        }
    }

    fn reservation_matches(&self, address: u64, width: RiscvAmoWidth) -> bool {
        self.reservation
            .is_some_and(|reservation| reservation.address == address && reservation.width == width)
    }

    fn reservation_overlaps(&self, address: u64, byte_len: usize) -> bool {
        let Some(reservation) = self.reservation else {
            return false;
        };
        let byte_len = byte_len as u64;
        let reservation_len = amo_width_byte_len(reservation.width) as u64;
        let end = address.saturating_add(byte_len);
        let reservation_end = reservation.address.saturating_add(reservation_len);
        address < reservation_end && reservation.address < end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineReport {
    pub address: u64,
    pub instruction: RiscvInstruction,
    pub next_pc: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineRunReport {
    pub executed_instructions: u64,
    pub halt: GuestMachineHalt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestMachineHalt {
    Ecall { address: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestMachineError {
    Fetch(GuestInstructionError),
    Memory(GuestMemoryError),
    InvalidRegisterIndex {
        index: usize,
    },
    ProgramCounterOverflow {
        address: u64,
        byte_len: usize,
    },
    MisalignedAtomicAccess {
        address: u64,
        width: RiscvAmoWidth,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestMachineRunError {
    Instruction(GuestMachineError),
    InstructionLimitExceeded { instruction_limit: u64, pc: u64 },
}

impl fmt::Display for GuestMachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch(error) => write!(f, "guest machine instruction fetch failed: {error}"),
            Self::Memory(error) => write!(f, "guest machine memory access failed: {error}"),
            Self::InvalidRegisterIndex { index } => {
                write!(f, "guest machine register index is invalid: {index}")
            }
            Self::ProgramCounterOverflow { address, byte_len } => write!(
                f,
                "guest machine program counter overflows: address {address}, byte length {byte_len}"
            ),
            Self::MisalignedAtomicAccess { address, width } => write!(
                f,
                "guest machine atomic memory access is misaligned: address {address}, width {width:?}"
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

impl fmt::Display for GuestMachineRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instruction(error) => write!(f, "guest machine run instruction failed: {error}"),
            Self::InstructionLimitExceeded {
                instruction_limit,
                pc,
            } => write!(
                f,
                "guest machine run exceeded instruction limit {instruction_limit} at pc {pc}"
            ),
        }
    }
}

impl std::error::Error for GuestMachineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fetch(error) => Some(error),
            Self::Memory(error) => Some(error),
            Self::InvalidRegisterIndex { .. }
            | Self::ProgramCounterOverflow { .. }
            | Self::MisalignedAtomicAccess { .. }
            | Self::UnsupportedInstructionLength { .. }
            | Self::UnsupportedInstruction { .. } => None,
        }
    }
}

impl std::error::Error for GuestMachineRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Instruction(error) => Some(error),
            Self::InstructionLimitExceeded { .. } => None,
        }
    }
}

impl From<GuestMemoryError> for GuestMachineError {
    fn from(error: GuestMemoryError) -> Self {
        Self::Memory(error)
    }
}

impl From<GuestInstructionError> for GuestMachineError {
    fn from(error: GuestInstructionError) -> Self {
        Self::Fetch(error)
    }
}

impl From<GuestMachineError> for GuestMachineRunError {
    fn from(error: GuestMachineError) -> Self {
        Self::Instruction(error)
    }
}

pub fn run_guest_machine(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    instruction_limit: u64,
) -> Result<GuestMachineRunReport, GuestMachineRunError> {
    let mut executed_instructions = 0_u64;
    loop {
        let current = decode_current_guest_instruction(memory, state.pc())?;
        if current == RiscvInstruction::Ecall {
            return Ok(GuestMachineRunReport {
                executed_instructions,
                halt: GuestMachineHalt::Ecall {
                    address: state.pc(),
                },
            });
        }
        if executed_instructions == instruction_limit {
            return Err(GuestMachineRunError::InstructionLimitExceeded {
                instruction_limit,
                pc: state.pc(),
            });
        }
        advance_guest_machine(memory, state)?;
        executed_instructions += 1;
    }
}

pub fn advance_guest_machine(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
) -> Result<GuestMachineReport, GuestMachineError> {
    let address = state.pc();
    let (byte_len, instruction) = fetch_decode_guest_instruction(memory, address)?;
    let sequential_pc = address
        .checked_add(byte_len as u64)
        .ok_or(GuestMachineError::ProgramCounterOverflow { address, byte_len })?;
    let mut next_state = state.clone();
    next_state.set_pc(sequential_pc);

    execute_guest_instruction(memory, address, sequential_pc, instruction, &mut next_state)?;
    let next_pc = next_state.pc();
    *state = next_state;

    Ok(GuestMachineReport {
        address,
        instruction,
        next_pc,
    })
}

fn decode_current_guest_instruction(
    memory: &GuestMachineMemory,
    address: u64,
) -> Result<RiscvInstruction, GuestMachineError> {
    let (_, instruction) = fetch_decode_guest_instruction(memory, address)?;
    Ok(instruction)
}

fn fetch_decode_guest_instruction(
    memory: &GuestMachineMemory,
    address: u64,
) -> Result<(usize, RiscvInstruction), GuestMachineError> {
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
    let instruction = decode_guest_instruction(fetched);
    Ok((byte_len, instruction))
}

fn execute_guest_instruction(
    memory: &mut GuestMachineMemory,
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
            );
            state.write_decoded_register(rd, value);
        }
        RiscvInstruction::OpImm32 {
            kind,
            rd,
            rs1,
            immediate,
        } => {
            let value = execute_op_imm_32(kind, state.read_decoded_register(rs1), immediate);
            state.write_decoded_register(rd, value);
        }
        RiscvInstruction::Op32 { kind, rd, rs1, rs2 } => {
            let value = execute_op_32(
                kind,
                state.read_decoded_register(rs1),
                state.read_decoded_register(rs2),
            );
            state.write_decoded_register(rd, value);
        }
        RiscvInstruction::Load {
            kind,
            rd,
            rs1,
            offset,
        } => {
            let address = state.read_decoded_register(rs1).wrapping_add_signed(offset);
            let value = read_guest_load(memory, kind, address)?;
            state.write_decoded_register(rd, value);
        }
        RiscvInstruction::Store {
            kind,
            rs1,
            rs2,
            offset,
        } => {
            let address = state.read_decoded_register(rs1).wrapping_add_signed(offset);
            write_guest_store(memory, kind, address, state.read_decoded_register(rs2))?;
            state.clear_reservation_if_overlaps(address, store_byte_len(kind));
        }
        RiscvInstruction::Amo {
            kind,
            width,
            rd,
            rs1,
            rs2,
            ..
        } => {
            let address = state.read_decoded_register(rs1);
            let loaded = read_guest_amo(memory, width, address)?;
            let stored = execute_amo(kind, width, loaded, state.read_decoded_register(rs2));
            write_guest_amo(memory, width, address, stored)?;
            state.clear_reservation_if_overlaps(address, amo_width_byte_len(width));
            state.write_decoded_register(rd, amo_result(width, loaded));
        }
        RiscvInstruction::LoadReserved { width, rd, rs1, .. } => {
            let address = state.read_decoded_register(rs1);
            let loaded = read_guest_amo(memory, width, address)?;
            state.write_decoded_register(rd, amo_result(width, loaded));
            state.set_reservation(address, width);
        }
        RiscvInstruction::StoreConditional {
            width,
            rd,
            rs1,
            rs2,
            ..
        } => {
            let address = state.read_decoded_register(rs1);
            ensure_atomic_aligned(width, address)?;
            if state.reservation_matches(address, width) {
                write_guest_amo(memory, width, address, state.read_decoded_register(rs2))?;
                state.write_decoded_register(rd, 0);
            } else {
                state.write_decoded_register(rd, 1);
            }
            state.clear_reservation();
        }
        RiscvInstruction::Fence { .. } => {}
        RiscvInstruction::CompressedUnknown { .. }
        | RiscvInstruction::IllegalCompressed { .. }
        | RiscvInstruction::UnsupportedLong { .. }
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

fn read_guest_load(
    memory: &GuestMachineMemory,
    kind: RiscvLoadKind,
    address: u64,
) -> Result<u64, GuestMachineError> {
    let value = match kind {
        RiscvLoadKind::Lb => {
            let mut bytes = [0_u8; 1];
            memory.read_range_into(address, &mut bytes)?;
            i64::from(bytes[0] as i8) as u64
        }
        RiscvLoadKind::Lh => {
            let mut bytes = [0_u8; 2];
            memory.read_range_into(address, &mut bytes)?;
            i64::from(i16::from_le_bytes(bytes)) as u64
        }
        RiscvLoadKind::Lw => {
            let mut bytes = [0_u8; 4];
            memory.read_range_into(address, &mut bytes)?;
            i64::from(i32::from_le_bytes(bytes)) as u64
        }
        RiscvLoadKind::Ld => {
            let mut bytes = [0_u8; 8];
            memory.read_range_into(address, &mut bytes)?;
            u64::from_le_bytes(bytes)
        }
        RiscvLoadKind::Lbu => {
            let mut bytes = [0_u8; 1];
            memory.read_range_into(address, &mut bytes)?;
            u64::from(bytes[0])
        }
        RiscvLoadKind::Lhu => {
            let mut bytes = [0_u8; 2];
            memory.read_range_into(address, &mut bytes)?;
            u64::from(u16::from_le_bytes(bytes))
        }
        RiscvLoadKind::Lwu => {
            let mut bytes = [0_u8; 4];
            memory.read_range_into(address, &mut bytes)?;
            u64::from(u32::from_le_bytes(bytes))
        }
    };
    Ok(value)
}

fn write_guest_store(
    memory: &mut GuestMachineMemory,
    kind: RiscvStoreKind,
    address: u64,
    value: u64,
) -> Result<(), GuestMachineError> {
    let bytes = value.to_le_bytes();
    match kind {
        RiscvStoreKind::Sb => memory.write_range(address, &bytes[..1])?,
        RiscvStoreKind::Sh => memory.write_range(address, &bytes[..2])?,
        RiscvStoreKind::Sw => memory.write_range(address, &bytes[..4])?,
        RiscvStoreKind::Sd => memory.write_range(address, &bytes)?,
    }
    Ok(())
}

fn store_byte_len(kind: RiscvStoreKind) -> usize {
    match kind {
        RiscvStoreKind::Sb => 1,
        RiscvStoreKind::Sh => 2,
        RiscvStoreKind::Sw => 4,
        RiscvStoreKind::Sd => 8,
    }
}

fn amo_width_byte_len(width: RiscvAmoWidth) -> usize {
    match width {
        RiscvAmoWidth::Word => 4,
        RiscvAmoWidth::Doubleword => 8,
    }
}

fn ensure_atomic_aligned(width: RiscvAmoWidth, address: u64) -> Result<(), GuestMachineError> {
    let byte_len = amo_width_byte_len(width) as u64;
    if address.is_multiple_of(byte_len) {
        Ok(())
    } else {
        Err(GuestMachineError::MisalignedAtomicAccess { address, width })
    }
}

fn read_guest_amo(
    memory: &GuestMachineMemory,
    width: RiscvAmoWidth,
    address: u64,
) -> Result<u64, GuestMachineError> {
    ensure_atomic_aligned(width, address)?;
    let value = match width {
        RiscvAmoWidth::Word => {
            let mut bytes = [0_u8; 4];
            memory.read_range_into(address, &mut bytes)?;
            u64::from(u32::from_le_bytes(bytes))
        }
        RiscvAmoWidth::Doubleword => {
            let mut bytes = [0_u8; 8];
            memory.read_range_into(address, &mut bytes)?;
            u64::from_le_bytes(bytes)
        }
    };
    Ok(value)
}

fn write_guest_amo(
    memory: &mut GuestMachineMemory,
    width: RiscvAmoWidth,
    address: u64,
    value: u64,
) -> Result<(), GuestMachineError> {
    ensure_atomic_aligned(width, address)?;
    match width {
        RiscvAmoWidth::Word => memory.write_range(address, &(value as u32).to_le_bytes())?,
        RiscvAmoWidth::Doubleword => memory.write_range(address, &value.to_le_bytes())?,
    }
    Ok(())
}

fn execute_amo(kind: RiscvAmoKind, width: RiscvAmoWidth, loaded: u64, operand: u64) -> u64 {
    match width {
        RiscvAmoWidth::Word => u64::from(execute_amo_word(kind, loaded as u32, operand as u32)),
        RiscvAmoWidth::Doubleword => execute_amo_doubleword(kind, loaded, operand),
    }
}

fn execute_amo_word(kind: RiscvAmoKind, loaded: u32, operand: u32) -> u32 {
    match kind {
        RiscvAmoKind::Add => loaded.wrapping_add(operand),
        RiscvAmoKind::Swap => operand,
        RiscvAmoKind::Xor => loaded ^ operand,
        RiscvAmoKind::Or => loaded | operand,
        RiscvAmoKind::And => loaded & operand,
        RiscvAmoKind::Min => (loaded as i32).min(operand as i32) as u32,
        RiscvAmoKind::Max => (loaded as i32).max(operand as i32) as u32,
        RiscvAmoKind::Minu => loaded.min(operand),
        RiscvAmoKind::Maxu => loaded.max(operand),
    }
}

fn execute_amo_doubleword(kind: RiscvAmoKind, loaded: u64, operand: u64) -> u64 {
    match kind {
        RiscvAmoKind::Add => loaded.wrapping_add(operand),
        RiscvAmoKind::Swap => operand,
        RiscvAmoKind::Xor => loaded ^ operand,
        RiscvAmoKind::Or => loaded | operand,
        RiscvAmoKind::And => loaded & operand,
        RiscvAmoKind::Min => (loaded as i64).min(operand as i64) as u64,
        RiscvAmoKind::Max => (loaded as i64).max(operand as i64) as u64,
        RiscvAmoKind::Minu => loaded.min(operand),
        RiscvAmoKind::Maxu => loaded.max(operand),
    }
}

fn amo_result(width: RiscvAmoWidth, loaded: u64) -> u64 {
    match width {
        RiscvAmoWidth::Word => sign_extend_word(loaded as u32),
        RiscvAmoWidth::Doubleword => loaded,
    }
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

fn execute_op_imm_32(kind: RiscvOpImm32Kind, rs1: u64, immediate: i64) -> u64 {
    match kind {
        RiscvOpImm32Kind::Addiw => sign_extend_word(rs1.wrapping_add_signed(immediate) as u32),
        RiscvOpImm32Kind::Slliw => {
            sign_extend_word((rs1 as u32).wrapping_shl((immediate as u32) & 0x1f))
        }
        RiscvOpImm32Kind::Srliw => {
            sign_extend_word((rs1 as u32).wrapping_shr((immediate as u32) & 0x1f))
        }
        RiscvOpImm32Kind::Sraiw => {
            sign_extend_word(((rs1 as u32 as i32) >> ((immediate as u32) & 0x1f)) as u32)
        }
    }
}

fn execute_op(kind: RiscvOpKind, rs1: u64, rs2: u64) -> u64 {
    match kind {
        RiscvOpKind::Add => rs1.wrapping_add(rs2),
        RiscvOpKind::Sub => rs1.wrapping_sub(rs2),
        RiscvOpKind::Sll => rs1.wrapping_shl((rs2 as u32) & 0x3f),
        RiscvOpKind::Slt => u64::from((rs1 as i64) < (rs2 as i64)),
        RiscvOpKind::Sltu => u64::from(rs1 < rs2),
        RiscvOpKind::Xor => rs1 ^ rs2,
        RiscvOpKind::Srl => rs1.wrapping_shr((rs2 as u32) & 0x3f),
        RiscvOpKind::Sra => ((rs1 as i64) >> ((rs2 as u32) & 0x3f)) as u64,
        RiscvOpKind::Or => rs1 | rs2,
        RiscvOpKind::And => rs1 & rs2,
        RiscvOpKind::Mul => rs1.wrapping_mul(rs2),
        RiscvOpKind::Mulh => (((rs1 as i64 as i128) * (rs2 as i64 as i128)) >> 64) as u64,
        RiscvOpKind::Mulhsu => (((rs1 as i64 as i128) * (rs2 as i128)) >> 64) as u64,
        RiscvOpKind::Mulhu => (((rs1 as u128) * (rs2 as u128)) >> 64) as u64,
        RiscvOpKind::Div => signed_divide(rs1 as i64, rs2 as i64) as u64,
        RiscvOpKind::Divu => {
            if rs2 == 0 {
                u64::MAX
            } else {
                rs1 / rs2
            }
        }
        RiscvOpKind::Rem => signed_remainder(rs1 as i64, rs2 as i64) as u64,
        RiscvOpKind::Remu => {
            if rs2 == 0 {
                rs1
            } else {
                rs1 % rs2
            }
        }
    }
}

fn execute_op_32(kind: RiscvOp32Kind, rs1: u64, rs2: u64) -> u64 {
    match kind {
        RiscvOp32Kind::Addw => sign_extend_word((rs1 as u32).wrapping_add(rs2 as u32)),
        RiscvOp32Kind::Subw => sign_extend_word((rs1 as u32).wrapping_sub(rs2 as u32)),
        RiscvOp32Kind::Sllw => sign_extend_word((rs1 as u32).wrapping_shl((rs2 as u32) & 0x1f)),
        RiscvOp32Kind::Srlw => sign_extend_word((rs1 as u32).wrapping_shr((rs2 as u32) & 0x1f)),
        RiscvOp32Kind::Sraw => {
            sign_extend_word(((rs1 as u32 as i32) >> ((rs2 as u32) & 0x1f)) as u32)
        }
        RiscvOp32Kind::Mulw => sign_extend_word((rs1 as u32).wrapping_mul(rs2 as u32)),
        RiscvOp32Kind::Divw => signed_divide_word(rs1 as u32 as i32, rs2 as u32 as i32),
        RiscvOp32Kind::Divuw => {
            if rs2 as u32 == 0 {
                u64::MAX
            } else {
                sign_extend_word((rs1 as u32) / (rs2 as u32))
            }
        }
        RiscvOp32Kind::Remw => signed_remainder_word(rs1 as u32 as i32, rs2 as u32 as i32),
        RiscvOp32Kind::Remuw => {
            if rs2 as u32 == 0 {
                sign_extend_word(rs1 as u32)
            } else {
                sign_extend_word((rs1 as u32) % (rs2 as u32))
            }
        }
    }
}

fn sign_extend_word(value: u32) -> u64 {
    i64::from(value as i32) as u64
}

fn signed_divide(dividend: i64, divisor: i64) -> i64 {
    if divisor == 0 {
        -1
    } else if dividend == i64::MIN && divisor == -1 {
        i64::MIN
    } else {
        dividend / divisor
    }
}

fn signed_remainder(dividend: i64, divisor: i64) -> i64 {
    if divisor == 0 {
        dividend
    } else if dividend == i64::MIN && divisor == -1 {
        0
    } else {
        dividend % divisor
    }
}

fn signed_divide_word(dividend: i32, divisor: i32) -> u64 {
    let quotient = if divisor == 0 {
        -1
    } else if dividend == i32::MIN && divisor == -1 {
        i32::MIN
    } else {
        dividend / divisor
    };
    sign_extend_word(quotient as u32)
}

fn signed_remainder_word(dividend: i32, divisor: i32) -> u64 {
    let remainder = if divisor == 0 {
        dividend
    } else if dividend == i32::MIN && divisor == -1 {
        0
    } else {
        dividend % divisor
    };
    sign_extend_word(remainder as u32)
}

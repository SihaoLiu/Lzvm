use std::fmt;

use crate::guest_memory::{GuestMemoryError, GuestMemoryReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestInstructionError {
    MisalignedFetch { address: u64 },
    Memory(GuestMemoryError),
}

impl fmt::Display for GuestInstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MisalignedFetch { address } => {
                write!(
                    f,
                    "guest instruction fetch address is misaligned: {address}"
                )
            }
            Self::Memory(error) => write!(f, "guest instruction fetch failed: {error}"),
        }
    }
}

impl std::error::Error for GuestInstructionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Memory(error) => Some(error),
            Self::MisalignedFetch { .. } => None,
        }
    }
}

impl From<GuestMemoryError> for GuestInstructionError {
    fn from(error: GuestMemoryError) -> Self {
        Self::Memory(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchedGuestInstruction {
    pub address: u64,
    pub encoded: RiscvEncodedInstruction,
}

impl FetchedGuestInstruction {
    pub fn byte_len(&self) -> Option<usize> {
        self.encoded.byte_len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvEncodedInstruction {
    Compressed(u16),
    Standard(u32),
    UnsupportedLong(u16),
}

impl RiscvEncodedInstruction {
    pub fn byte_len(&self) -> Option<usize> {
        match self {
            Self::Compressed(_) => Some(2),
            Self::Standard(_) => Some(4),
            Self::UnsupportedLong(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvInstruction {
    CompressedUnknown {
        halfword: u16,
        quadrant: u8,
        funct3: u8,
    },
    IllegalCompressed {
        halfword: u16,
    },
    UnsupportedLong {
        halfword: u16,
    },
    Lui {
        rd: u8,
        immediate: i64,
    },
    Auipc {
        rd: u8,
        immediate: i64,
    },
    Jal {
        rd: u8,
        offset: i64,
    },
    Jalr {
        rd: u8,
        rs1: u8,
        offset: i64,
    },
    Branch {
        kind: RiscvBranchKind,
        rs1: u8,
        rs2: u8,
        offset: i64,
    },
    Load {
        kind: RiscvLoadKind,
        rd: u8,
        rs1: u8,
        offset: i64,
    },
    Store {
        kind: RiscvStoreKind,
        rs1: u8,
        rs2: u8,
        offset: i64,
    },
    OpImm {
        kind: RiscvOpImmKind,
        rd: u8,
        rs1: u8,
        immediate: i64,
    },
    OpImm32 {
        kind: RiscvOpImm32Kind,
        rd: u8,
        rs1: u8,
        immediate: i64,
    },
    Op {
        kind: RiscvOpKind,
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Op32 {
        kind: RiscvOp32Kind,
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Fence {
        kind: RiscvFenceKind,
        mode: u8,
        predecessor: u8,
        successor: u8,
    },
    Ecall,
    Ebreak,
    Unknown {
        word: u32,
        opcode: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvBranchKind {
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvLoadKind {
    Lb,
    Lh,
    Lw,
    Ld,
    Lbu,
    Lhu,
    Lwu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvStoreKind {
    Sb,
    Sh,
    Sw,
    Sd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvOpImmKind {
    Addi,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi,
    Slli,
    Srli,
    Srai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvOpImm32Kind {
    Addiw,
    Slliw,
    Srliw,
    Sraiw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvOpKind {
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvOp32Kind {
    Addw,
    Subw,
    Sllw,
    Srlw,
    Sraw,
    Mulw,
    Divw,
    Divuw,
    Remw,
    Remuw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvFenceKind {
    Fence,
    FenceTso,
    FenceI,
}

pub fn fetch_guest_instruction(
    memory: &(impl GuestMemoryReader + ?Sized),
    address: u64,
) -> Result<FetchedGuestInstruction, GuestInstructionError> {
    if !address.is_multiple_of(2) {
        return Err(GuestInstructionError::MisalignedFetch { address });
    }

    let mut low = [0_u8; 2];
    memory.read_range_into(address, &mut low)?;
    let low = u16::from_le_bytes(low);
    let encoded = if low & 0b11 == 0b11 {
        if low & 0b11100 == 0b11100 {
            return Ok(FetchedGuestInstruction {
                address,
                encoded: RiscvEncodedInstruction::UnsupportedLong(low),
            });
        }
        let mut bytes = [0_u8; 4];
        bytes[..2].copy_from_slice(&low.to_le_bytes());
        memory.read_range_into(address + 2, &mut bytes[2..])?;
        RiscvEncodedInstruction::Standard(u32::from_le_bytes(bytes))
    } else {
        RiscvEncodedInstruction::Compressed(low)
    };

    Ok(FetchedGuestInstruction { address, encoded })
}

pub fn decode_guest_instruction(instruction: FetchedGuestInstruction) -> RiscvInstruction {
    match instruction.encoded {
        RiscvEncodedInstruction::Compressed(halfword) => decode_compressed_instruction(halfword),
        RiscvEncodedInstruction::Standard(word) => decode_riscv_instruction(word),
        RiscvEncodedInstruction::UnsupportedLong(halfword) => {
            RiscvInstruction::UnsupportedLong { halfword }
        }
    }
}

pub fn decode_riscv_instruction(word: u32) -> RiscvInstruction {
    let opcode = opcode(word);
    match opcode {
        0x03 => decode_load(word),
        0x0f => decode_fence(word),
        0x13 => decode_op_imm(word),
        0x17 => RiscvInstruction::Auipc {
            rd: rd(word),
            immediate: u_immediate(word),
        },
        0x1b => decode_op_imm_32(word),
        0x23 => decode_store(word),
        0x33 => decode_op(word),
        0x37 => RiscvInstruction::Lui {
            rd: rd(word),
            immediate: u_immediate(word),
        },
        0x3b => decode_op_32(word),
        0x63 => decode_branch(word),
        0x67 => decode_jalr(word),
        0x6f => RiscvInstruction::Jal {
            rd: rd(word),
            offset: j_immediate(word),
        },
        0x73 if word == 0x0000_0073 => RiscvInstruction::Ecall,
        0x73 if word == 0x0010_0073 => RiscvInstruction::Ebreak,
        _ => RiscvInstruction::Unknown { word, opcode },
    }
}

fn decode_compressed_instruction(halfword: u16) -> RiscvInstruction {
    if halfword == 0 {
        return RiscvInstruction::IllegalCompressed { halfword };
    }
    RiscvInstruction::CompressedUnknown {
        halfword,
        quadrant: (halfword & 0b11) as u8,
        funct3: ((halfword >> 13) & 0b111) as u8,
    }
}

fn decode_fence(word: u32) -> RiscvInstruction {
    if rd(word) != 0 || rs1(word) != 0 {
        return unknown(word);
    }
    let mode = ((word >> 28) & 0x0f) as u8;
    let predecessor = ((word >> 24) & 0x0f) as u8;
    let successor = ((word >> 20) & 0x0f) as u8;
    let kind = match funct3(word) {
        0 if mode == 0 => RiscvFenceKind::Fence,
        0 if mode == 8 && predecessor == 3 && successor == 3 => RiscvFenceKind::FenceTso,
        1 if mode == 0 && predecessor == 0 && successor == 0 => RiscvFenceKind::FenceI,
        _ => return unknown(word),
    };
    RiscvInstruction::Fence {
        kind,
        mode,
        predecessor,
        successor,
    }
}

fn decode_load(word: u32) -> RiscvInstruction {
    let Some(kind) = (match funct3(word) {
        0 => Some(RiscvLoadKind::Lb),
        1 => Some(RiscvLoadKind::Lh),
        2 => Some(RiscvLoadKind::Lw),
        3 => Some(RiscvLoadKind::Ld),
        4 => Some(RiscvLoadKind::Lbu),
        5 => Some(RiscvLoadKind::Lhu),
        6 => Some(RiscvLoadKind::Lwu),
        _ => None,
    }) else {
        return unknown(word);
    };
    RiscvInstruction::Load {
        kind,
        rd: rd(word),
        rs1: rs1(word),
        offset: i_immediate(word),
    }
}

fn decode_store(word: u32) -> RiscvInstruction {
    let Some(kind) = (match funct3(word) {
        0 => Some(RiscvStoreKind::Sb),
        1 => Some(RiscvStoreKind::Sh),
        2 => Some(RiscvStoreKind::Sw),
        3 => Some(RiscvStoreKind::Sd),
        _ => None,
    }) else {
        return unknown(word);
    };
    RiscvInstruction::Store {
        kind,
        rs1: rs1(word),
        rs2: rs2(word),
        offset: s_immediate(word),
    }
}

fn decode_branch(word: u32) -> RiscvInstruction {
    let Some(kind) = (match funct3(word) {
        0 => Some(RiscvBranchKind::Beq),
        1 => Some(RiscvBranchKind::Bne),
        4 => Some(RiscvBranchKind::Blt),
        5 => Some(RiscvBranchKind::Bge),
        6 => Some(RiscvBranchKind::Bltu),
        7 => Some(RiscvBranchKind::Bgeu),
        _ => None,
    }) else {
        return unknown(word);
    };
    RiscvInstruction::Branch {
        kind,
        rs1: rs1(word),
        rs2: rs2(word),
        offset: b_immediate(word),
    }
}

fn decode_jalr(word: u32) -> RiscvInstruction {
    if funct3(word) != 0 {
        return unknown(word);
    }
    RiscvInstruction::Jalr {
        rd: rd(word),
        rs1: rs1(word),
        offset: i_immediate(word),
    }
}

fn decode_op_imm(word: u32) -> RiscvInstruction {
    let Some(kind) = (match funct3(word) {
        0 => Some(RiscvOpImmKind::Addi),
        1 if shift_funct6(word) == 0 => Some(RiscvOpImmKind::Slli),
        2 => Some(RiscvOpImmKind::Slti),
        3 => Some(RiscvOpImmKind::Sltiu),
        4 => Some(RiscvOpImmKind::Xori),
        5 if shift_funct6(word) == 0 => Some(RiscvOpImmKind::Srli),
        5 if shift_funct6(word) == 0x10 => Some(RiscvOpImmKind::Srai),
        6 => Some(RiscvOpImmKind::Ori),
        7 => Some(RiscvOpImmKind::Andi),
        _ => None,
    }) else {
        return unknown(word);
    };
    RiscvInstruction::OpImm {
        kind,
        rd: rd(word),
        rs1: rs1(word),
        immediate: op_imm_immediate(kind, word),
    }
}

fn decode_op_imm_32(word: u32) -> RiscvInstruction {
    let Some(kind) = (match funct3(word) {
        0 => Some(RiscvOpImm32Kind::Addiw),
        1 if funct7(word) == 0 => Some(RiscvOpImm32Kind::Slliw),
        5 if funct7(word) == 0 => Some(RiscvOpImm32Kind::Srliw),
        5 if funct7(word) == 0x20 => Some(RiscvOpImm32Kind::Sraiw),
        _ => None,
    }) else {
        return unknown(word);
    };
    RiscvInstruction::OpImm32 {
        kind,
        rd: rd(word),
        rs1: rs1(word),
        immediate: op_imm_32_immediate(kind, word),
    }
}

fn decode_op(word: u32) -> RiscvInstruction {
    let Some(kind) = (match (funct7(word), funct3(word)) {
        (0x00, 0) => Some(RiscvOpKind::Add),
        (0x20, 0) => Some(RiscvOpKind::Sub),
        (0x00, 1) => Some(RiscvOpKind::Sll),
        (0x00, 2) => Some(RiscvOpKind::Slt),
        (0x00, 3) => Some(RiscvOpKind::Sltu),
        (0x00, 4) => Some(RiscvOpKind::Xor),
        (0x00, 5) => Some(RiscvOpKind::Srl),
        (0x20, 5) => Some(RiscvOpKind::Sra),
        (0x00, 6) => Some(RiscvOpKind::Or),
        (0x00, 7) => Some(RiscvOpKind::And),
        (0x01, 0) => Some(RiscvOpKind::Mul),
        (0x01, 1) => Some(RiscvOpKind::Mulh),
        (0x01, 2) => Some(RiscvOpKind::Mulhsu),
        (0x01, 3) => Some(RiscvOpKind::Mulhu),
        (0x01, 4) => Some(RiscvOpKind::Div),
        (0x01, 5) => Some(RiscvOpKind::Divu),
        (0x01, 6) => Some(RiscvOpKind::Rem),
        (0x01, 7) => Some(RiscvOpKind::Remu),
        _ => None,
    }) else {
        return unknown(word);
    };
    RiscvInstruction::Op {
        kind,
        rd: rd(word),
        rs1: rs1(word),
        rs2: rs2(word),
    }
}

fn decode_op_32(word: u32) -> RiscvInstruction {
    let Some(kind) = (match (funct7(word), funct3(word)) {
        (0x00, 0) => Some(RiscvOp32Kind::Addw),
        (0x20, 0) => Some(RiscvOp32Kind::Subw),
        (0x00, 1) => Some(RiscvOp32Kind::Sllw),
        (0x00, 5) => Some(RiscvOp32Kind::Srlw),
        (0x20, 5) => Some(RiscvOp32Kind::Sraw),
        (0x01, 0) => Some(RiscvOp32Kind::Mulw),
        (0x01, 4) => Some(RiscvOp32Kind::Divw),
        (0x01, 5) => Some(RiscvOp32Kind::Divuw),
        (0x01, 6) => Some(RiscvOp32Kind::Remw),
        (0x01, 7) => Some(RiscvOp32Kind::Remuw),
        _ => None,
    }) else {
        return unknown(word);
    };
    RiscvInstruction::Op32 {
        kind,
        rd: rd(word),
        rs1: rs1(word),
        rs2: rs2(word),
    }
}

fn unknown(word: u32) -> RiscvInstruction {
    RiscvInstruction::Unknown {
        word,
        opcode: opcode(word),
    }
}

fn opcode(word: u32) -> u8 {
    (word & 0x7f) as u8
}

fn rd(word: u32) -> u8 {
    ((word >> 7) & 0x1f) as u8
}

fn funct3(word: u32) -> u8 {
    ((word >> 12) & 0x07) as u8
}

fn rs1(word: u32) -> u8 {
    ((word >> 15) & 0x1f) as u8
}

fn rs2(word: u32) -> u8 {
    ((word >> 20) & 0x1f) as u8
}

fn funct7(word: u32) -> u8 {
    ((word >> 25) & 0x7f) as u8
}

fn shift_funct6(word: u32) -> u8 {
    ((word >> 26) & 0x3f) as u8
}

fn i_immediate(word: u32) -> i64 {
    sign_extend((word >> 20) as u64, 12)
}

fn op_imm_immediate(kind: RiscvOpImmKind, word: u32) -> i64 {
    match kind {
        RiscvOpImmKind::Slli | RiscvOpImmKind::Srli | RiscvOpImmKind::Srai => {
            i64::from((word >> 20) & 0x3f)
        }
        RiscvOpImmKind::Addi
        | RiscvOpImmKind::Slti
        | RiscvOpImmKind::Sltiu
        | RiscvOpImmKind::Xori
        | RiscvOpImmKind::Ori
        | RiscvOpImmKind::Andi => i_immediate(word),
    }
}

fn op_imm_32_immediate(kind: RiscvOpImm32Kind, word: u32) -> i64 {
    match kind {
        RiscvOpImm32Kind::Addiw => i_immediate(word),
        RiscvOpImm32Kind::Slliw | RiscvOpImm32Kind::Srliw | RiscvOpImm32Kind::Sraiw => {
            i64::from((word >> 20) & 0x1f)
        }
    }
}

fn s_immediate(word: u32) -> i64 {
    let low = (word >> 7) & 0x1f;
    let high = (word >> 25) & 0x7f;
    sign_extend(u64::from(low | (high << 5)), 12)
}

fn b_immediate(word: u32) -> i64 {
    let bit_11 = (word >> 7) & 0x01;
    let bits_4_1 = (word >> 8) & 0x0f;
    let bits_10_5 = (word >> 25) & 0x3f;
    let bit_12 = (word >> 31) & 0x01;
    let value = (bits_4_1 << 1) | (bits_10_5 << 5) | (bit_11 << 11) | (bit_12 << 12);
    sign_extend(u64::from(value), 13)
}

fn u_immediate(word: u32) -> i64 {
    sign_extend(u64::from(word & 0xffff_f000), 32)
}

fn j_immediate(word: u32) -> i64 {
    let bits_19_12 = (word >> 12) & 0xff;
    let bit_11 = (word >> 20) & 0x01;
    let bits_10_1 = (word >> 21) & 0x03ff;
    let bit_20 = (word >> 31) & 0x01;
    let value = (bits_10_1 << 1) | (bit_11 << 11) | (bits_19_12 << 12) | (bit_20 << 20);
    sign_extend(u64::from(value), 21)
}

fn sign_extend(value: u64, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((value << shift) as i64) >> shift
}

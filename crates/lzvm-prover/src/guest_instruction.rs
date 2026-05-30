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
    Amo {
        kind: RiscvAmoKind,
        width: RiscvAmoWidth,
        rd: u8,
        rs1: u8,
        rs2: u8,
        acquire: bool,
        release: bool,
    },
    LoadReserved {
        width: RiscvAmoWidth,
        rd: u8,
        rs1: u8,
        acquire: bool,
        release: bool,
    },
    StoreConditional {
        width: RiscvAmoWidth,
        rd: u8,
        rs1: u8,
        rs2: u8,
        acquire: bool,
        release: bool,
    },
    CsrRead {
        csr: RiscvCsr,
        rd: u8,
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
pub enum RiscvAmoKind {
    Add,
    Swap,
    Xor,
    Or,
    And,
    Min,
    Max,
    Minu,
    Maxu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvAmoWidth {
    Word,
    Doubleword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiscvCsr {
    Cycle,
    Time,
    Instret,
    Misa,
    Mvendorid,
    Marchid,
    Mimpid,
    Mhartid,
}

impl RiscvCsr {
    fn from_number(number: u16) -> Option<Self> {
        match number {
            0x0c00 => Some(Self::Cycle),
            0x0c01 => Some(Self::Time),
            0x0c02 => Some(Self::Instret),
            0x0301 => Some(Self::Misa),
            0x0f11 => Some(Self::Mvendorid),
            0x0f12 => Some(Self::Marchid),
            0x0f13 => Some(Self::Mimpid),
            0x0f14 => Some(Self::Mhartid),
            _ => None,
        }
    }
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
        0x2f => decode_amo(word),
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
        0x73 => decode_system(word),
        _ => RiscvInstruction::Unknown { word, opcode },
    }
}

fn decode_compressed_instruction(halfword: u16) -> RiscvInstruction {
    if halfword == 0 {
        return RiscvInstruction::IllegalCompressed { halfword };
    }
    match (((halfword >> 13) & 0b111) as u8, (halfword & 0b11) as u8) {
        (0, 0) => decode_compressed_addi4spn(halfword),
        (2, 0) => decode_compressed_lw(halfword),
        (3, 0) => decode_compressed_ld(halfword),
        (6, 0) => decode_compressed_sw(halfword),
        (7, 0) => decode_compressed_sd(halfword),
        (0, 1) => decode_compressed_addi(halfword),
        (1, 1) => decode_compressed_addiw(halfword),
        (2, 1) => decode_compressed_li(halfword),
        (3, 1) => decode_compressed_lui_or_addi16sp(halfword),
        (4, 1) => decode_compressed_shift_logical(halfword),
        (5, 1) => decode_compressed_jump(halfword),
        (6, 1) => decode_compressed_branch(RiscvBranchKind::Beq, halfword),
        (7, 1) => decode_compressed_branch(RiscvBranchKind::Bne, halfword),
        (0, 2) => decode_compressed_slli(halfword),
        (2, 2) => decode_compressed_lwsp(halfword),
        (3, 2) => decode_compressed_ldsp(halfword),
        (4, 2) => decode_compressed_register_control(halfword),
        (6, 2) => decode_compressed_swsp(halfword),
        (7, 2) => decode_compressed_sdsp(halfword),
        _ => compressed_unknown(halfword),
    }
}

fn compressed_unknown(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::CompressedUnknown {
        halfword,
        quadrant: (halfword & 0b11) as u8,
        funct3: ((halfword >> 13) & 0b111) as u8,
    }
}

fn decode_compressed_addi4spn(halfword: u16) -> RiscvInstruction {
    let immediate = compressed_addi4spn_immediate(halfword);
    if immediate == 0 {
        return compressed_unknown(halfword);
    }
    RiscvInstruction::OpImm {
        kind: RiscvOpImmKind::Addi,
        rd: compressed_register(halfword >> 2),
        rs1: 2,
        immediate,
    }
}

fn decode_compressed_lw(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::Load {
        kind: RiscvLoadKind::Lw,
        rd: compressed_register(halfword >> 2),
        rs1: compressed_register(halfword >> 7),
        offset: compressed_lw_sw_offset(halfword),
    }
}

fn decode_compressed_ld(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::Load {
        kind: RiscvLoadKind::Ld,
        rd: compressed_register(halfword >> 2),
        rs1: compressed_register(halfword >> 7),
        offset: compressed_ld_sd_offset(halfword),
    }
}

fn decode_compressed_sw(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::Store {
        kind: RiscvStoreKind::Sw,
        rs1: compressed_register(halfword >> 7),
        rs2: compressed_register(halfword >> 2),
        offset: compressed_lw_sw_offset(halfword),
    }
}

fn decode_compressed_sd(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::Store {
        kind: RiscvStoreKind::Sd,
        rs1: compressed_register(halfword >> 7),
        rs2: compressed_register(halfword >> 2),
        offset: compressed_ld_sd_offset(halfword),
    }
}

fn decode_compressed_addi(halfword: u16) -> RiscvInstruction {
    let rd = ((halfword >> 7) & 0x1f) as u8;
    RiscvInstruction::OpImm {
        kind: RiscvOpImmKind::Addi,
        rd,
        rs1: rd,
        immediate: compressed_addi_immediate(halfword),
    }
}

fn decode_compressed_addiw(halfword: u16) -> RiscvInstruction {
    let rd = ((halfword >> 7) & 0x1f) as u8;
    if rd == 0 {
        return compressed_unknown(halfword);
    }
    RiscvInstruction::OpImm32 {
        kind: RiscvOpImm32Kind::Addiw,
        rd,
        rs1: rd,
        immediate: compressed_addi_immediate(halfword),
    }
}

fn decode_compressed_li(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::OpImm {
        kind: RiscvOpImmKind::Addi,
        rd: ((halfword >> 7) & 0x1f) as u8,
        rs1: 0,
        immediate: compressed_addi_immediate(halfword),
    }
}

fn decode_compressed_lui_or_addi16sp(halfword: u16) -> RiscvInstruction {
    if compressed_ci_immediate_payload(halfword) == 0 {
        return compressed_unknown(halfword);
    }
    let rd = ((halfword >> 7) & 0x1f) as u8;
    if rd == 2 {
        return RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd,
            rs1: rd,
            immediate: compressed_addi16sp_immediate(halfword),
        };
    }
    if rd == 0 {
        return compressed_unknown(halfword);
    }
    RiscvInstruction::Lui {
        rd,
        immediate: compressed_lui_immediate(halfword),
    }
}

fn decode_compressed_shift_logical(halfword: u16) -> RiscvInstruction {
    let rd_rs1 = compressed_register(halfword >> 7);
    match ((halfword >> 10) & 0x3) as u8 {
        0 => RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Srli,
            rd: rd_rs1,
            rs1: rd_rs1,
            immediate: compressed_shift_immediate(halfword),
        },
        1 => RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Srai,
            rd: rd_rs1,
            rs1: rd_rs1,
            immediate: compressed_shift_immediate(halfword),
        },
        2 => RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Andi,
            rd: rd_rs1,
            rs1: rd_rs1,
            immediate: compressed_addi_immediate(halfword),
        },
        3 => decode_compressed_register_arithmetic(halfword),
        _ => unreachable!("compressed funct2 is two bits"),
    }
}

fn decode_compressed_register_arithmetic(halfword: u16) -> RiscvInstruction {
    let rd_rs1 = compressed_register(halfword >> 7);
    let rs2 = compressed_register(halfword >> 2);
    match (((halfword >> 12) & 1) != 0, (halfword >> 5) & 0x3) {
        (false, 0) => RiscvInstruction::Op {
            kind: RiscvOpKind::Sub,
            rd: rd_rs1,
            rs1: rd_rs1,
            rs2,
        },
        (false, 1) => RiscvInstruction::Op {
            kind: RiscvOpKind::Xor,
            rd: rd_rs1,
            rs1: rd_rs1,
            rs2,
        },
        (false, 2) => RiscvInstruction::Op {
            kind: RiscvOpKind::Or,
            rd: rd_rs1,
            rs1: rd_rs1,
            rs2,
        },
        (false, 3) => RiscvInstruction::Op {
            kind: RiscvOpKind::And,
            rd: rd_rs1,
            rs1: rd_rs1,
            rs2,
        },
        (true, 0) => RiscvInstruction::Op32 {
            kind: RiscvOp32Kind::Subw,
            rd: rd_rs1,
            rs1: rd_rs1,
            rs2,
        },
        (true, 1) => RiscvInstruction::Op32 {
            kind: RiscvOp32Kind::Addw,
            rd: rd_rs1,
            rs1: rd_rs1,
            rs2,
        },
        (true, _) => compressed_unknown(halfword),
        _ => unreachable!("compressed arithmetic kind is two bits"),
    }
}

fn decode_compressed_jump(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::Jal {
        rd: 0,
        offset: compressed_jump_offset(halfword),
    }
}

fn decode_compressed_branch(kind: RiscvBranchKind, halfword: u16) -> RiscvInstruction {
    RiscvInstruction::Branch {
        kind,
        rs1: compressed_register(halfword >> 7),
        rs2: 0,
        offset: compressed_branch_offset(halfword),
    }
}

fn decode_compressed_slli(halfword: u16) -> RiscvInstruction {
    let rd = ((halfword >> 7) & 0x1f) as u8;
    let immediate = compressed_shift_immediate(halfword);
    if rd == 0 || immediate == 0 {
        return compressed_unknown(halfword);
    }
    RiscvInstruction::OpImm {
        kind: RiscvOpImmKind::Slli,
        rd,
        rs1: rd,
        immediate,
    }
}

fn decode_compressed_lwsp(halfword: u16) -> RiscvInstruction {
    let rd = ((halfword >> 7) & 0x1f) as u8;
    if rd == 0 {
        return compressed_unknown(halfword);
    }
    RiscvInstruction::Load {
        kind: RiscvLoadKind::Lw,
        rd,
        rs1: 2,
        offset: compressed_lwsp_offset(halfword),
    }
}

fn decode_compressed_ldsp(halfword: u16) -> RiscvInstruction {
    let rd = ((halfword >> 7) & 0x1f) as u8;
    if rd == 0 {
        return compressed_unknown(halfword);
    }
    RiscvInstruction::Load {
        kind: RiscvLoadKind::Ld,
        rd,
        rs1: 2,
        offset: compressed_ldsp_offset(halfword),
    }
}

fn decode_compressed_swsp(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::Store {
        kind: RiscvStoreKind::Sw,
        rs1: 2,
        rs2: ((halfword >> 2) & 0x1f) as u8,
        offset: compressed_swsp_offset(halfword),
    }
}

fn decode_compressed_sdsp(halfword: u16) -> RiscvInstruction {
    RiscvInstruction::Store {
        kind: RiscvStoreKind::Sd,
        rs1: 2,
        rs2: ((halfword >> 2) & 0x1f) as u8,
        offset: compressed_sdsp_offset(halfword),
    }
}

fn decode_compressed_register_control(halfword: u16) -> RiscvInstruction {
    let rd_rs1 = ((halfword >> 7) & 0x1f) as u8;
    let rs2 = ((halfword >> 2) & 0x1f) as u8;
    match (((halfword >> 12) & 1) != 0, rd_rs1, rs2) {
        (false, 0, 0) => compressed_unknown(halfword),
        (false, rs1, 0) => RiscvInstruction::Jalr {
            rd: 0,
            rs1,
            offset: 0,
        },
        (false, rd, rs2) => RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd,
            rs1: 0,
            rs2,
        },
        (true, 0, 0) => RiscvInstruction::Ebreak,
        (true, rs1, 0) => RiscvInstruction::Jalr {
            rd: 1,
            rs1,
            offset: 0,
        },
        (true, rd, rs2) => RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd,
            rs1: rd,
            rs2,
        },
    }
}

fn compressed_addi_immediate(halfword: u16) -> i64 {
    sign_extend(u64::from(compressed_ci_immediate_payload(halfword)), 6)
}

fn compressed_shift_immediate(halfword: u16) -> i64 {
    i64::from(compressed_ci_immediate_payload(halfword))
}

fn compressed_lui_immediate(halfword: u16) -> i64 {
    sign_extend(u64::from(compressed_ci_immediate_payload(halfword)), 6) << 12
}

fn compressed_ci_immediate_payload(halfword: u16) -> u16 {
    ((halfword >> 2) & 0x1f) | (((halfword >> 12) & 1) << 5)
}

fn compressed_addi16sp_immediate(halfword: u16) -> i64 {
    let bit_4 = (halfword >> 6) & 1;
    let bit_5 = (halfword >> 2) & 1;
    let bit_6 = (halfword >> 5) & 1;
    let bits_8_7 = (halfword >> 3) & 0x3;
    let bit_9 = (halfword >> 12) & 1;
    let value = (bit_4 << 4) | (bit_5 << 5) | (bit_6 << 6) | (bits_8_7 << 7) | (bit_9 << 9);
    sign_extend(u64::from(value), 10)
}

fn compressed_register(encoded: u16) -> u8 {
    ((encoded & 0x7) as u8) + 8
}

fn compressed_addi4spn_immediate(halfword: u16) -> i64 {
    let bits_5_4 = (halfword >> 11) & 0x3;
    let bits_9_6 = (halfword >> 7) & 0xf;
    let bit_2 = (halfword >> 6) & 1;
    let bit_3 = (halfword >> 5) & 1;
    i64::from((bit_2 << 2) | (bit_3 << 3) | (bits_5_4 << 4) | (bits_9_6 << 6))
}

fn compressed_lw_sw_offset(halfword: u16) -> i64 {
    let bits_5_3 = (halfword >> 10) & 0x7;
    let bit_2 = (halfword >> 6) & 1;
    let bit_6 = (halfword >> 5) & 1;
    i64::from((bit_2 << 2) | (bits_5_3 << 3) | (bit_6 << 6))
}

fn compressed_ld_sd_offset(halfword: u16) -> i64 {
    let bits_5_3 = (halfword >> 10) & 0x7;
    let bits_7_6 = (halfword >> 5) & 0x3;
    i64::from((bits_5_3 << 3) | (bits_7_6 << 6))
}

fn compressed_lwsp_offset(halfword: u16) -> i64 {
    let bits_4_2 = (halfword >> 4) & 0x7;
    let bit_5 = (halfword >> 12) & 1;
    let bits_7_6 = (halfword >> 2) & 0x3;
    i64::from((bits_4_2 << 2) | (bit_5 << 5) | (bits_7_6 << 6))
}

fn compressed_ldsp_offset(halfword: u16) -> i64 {
    let bits_4_3 = (halfword >> 5) & 0x3;
    let bit_5 = (halfword >> 12) & 1;
    let bits_8_6 = (halfword >> 2) & 0x7;
    i64::from((bits_4_3 << 3) | (bit_5 << 5) | (bits_8_6 << 6))
}

fn compressed_swsp_offset(halfword: u16) -> i64 {
    let bits_5_2 = (halfword >> 9) & 0xf;
    let bits_7_6 = (halfword >> 7) & 0x3;
    i64::from((bits_5_2 << 2) | (bits_7_6 << 6))
}

fn compressed_sdsp_offset(halfword: u16) -> i64 {
    let bits_5_3 = (halfword >> 10) & 0x7;
    let bits_8_6 = (halfword >> 7) & 0x7;
    i64::from((bits_5_3 << 3) | (bits_8_6 << 6))
}

fn compressed_jump_offset(halfword: u16) -> i64 {
    let bit_11 = (halfword >> 12) & 1;
    let bit_4 = (halfword >> 11) & 1;
    let bits_9_8 = (halfword >> 9) & 0x3;
    let bit_10 = (halfword >> 8) & 1;
    let bit_6 = (halfword >> 7) & 1;
    let bit_7 = (halfword >> 6) & 1;
    let bits_3_1 = (halfword >> 3) & 0x7;
    let bit_5 = (halfword >> 2) & 1;
    let value = (bits_3_1 << 1)
        | (bit_4 << 4)
        | (bit_5 << 5)
        | (bit_6 << 6)
        | (bit_7 << 7)
        | (bits_9_8 << 8)
        | (bit_10 << 10)
        | (bit_11 << 11);
    sign_extend(u64::from(value), 12)
}

fn compressed_branch_offset(halfword: u16) -> i64 {
    let bit_8 = (halfword >> 12) & 1;
    let bits_4_3 = (halfword >> 10) & 0x3;
    let bits_7_6 = (halfword >> 5) & 0x3;
    let bits_2_1 = (halfword >> 3) & 0x3;
    let bit_5 = (halfword >> 2) & 1;
    let value = (bits_2_1 << 1) | (bits_4_3 << 3) | (bit_5 << 5) | (bits_7_6 << 6) | (bit_8 << 8);
    sign_extend(u64::from(value), 9)
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

fn decode_amo(word: u32) -> RiscvInstruction {
    let Some(width) = (match funct3(word) {
        2 => Some(RiscvAmoWidth::Word),
        3 => Some(RiscvAmoWidth::Doubleword),
        _ => None,
    }) else {
        return unknown(word);
    };
    let funct5 = ((word >> 27) & 0x1f) as u8;
    let acquire = ((word >> 26) & 1) != 0;
    let release = ((word >> 25) & 1) != 0;
    if funct5 == 0x02 {
        if rs2(word) != 0 {
            return unknown(word);
        }
        return RiscvInstruction::LoadReserved {
            width,
            rd: rd(word),
            rs1: rs1(word),
            acquire,
            release,
        };
    }
    if funct5 == 0x03 {
        return RiscvInstruction::StoreConditional {
            width,
            rd: rd(word),
            rs1: rs1(word),
            rs2: rs2(word),
            acquire,
            release,
        };
    }
    let Some(kind) = (match funct5 {
        0 => Some(RiscvAmoKind::Add),
        1 => Some(RiscvAmoKind::Swap),
        4 => Some(RiscvAmoKind::Xor),
        8 => Some(RiscvAmoKind::Or),
        12 => Some(RiscvAmoKind::And),
        16 => Some(RiscvAmoKind::Min),
        20 => Some(RiscvAmoKind::Max),
        24 => Some(RiscvAmoKind::Minu),
        28 => Some(RiscvAmoKind::Maxu),
        _ => None,
    }) else {
        return unknown(word);
    };
    RiscvInstruction::Amo {
        kind,
        width,
        rd: rd(word),
        rs1: rs1(word),
        rs2: rs2(word),
        acquire,
        release,
    }
}

fn decode_system(word: u32) -> RiscvInstruction {
    if word == 0x0000_0073 {
        return RiscvInstruction::Ecall;
    }
    if word == 0x0010_0073 {
        return RiscvInstruction::Ebreak;
    }
    let Some(csr) = RiscvCsr::from_number(((word >> 20) & 0x0fff) as u16) else {
        return unknown(word);
    };
    match funct3(word) {
        2 | 3 if rs1(word) == 0 => RiscvInstruction::CsrRead { csr, rd: rd(word) },
        _ => unknown(word),
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

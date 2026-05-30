use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_prover::guest_instruction::{
    decode_guest_instruction, decode_riscv_instruction, fetch_guest_instruction,
    FetchedGuestInstruction, GuestInstructionError, RiscvBranchKind, RiscvEncodedInstruction,
    RiscvFenceKind, RiscvInstruction, RiscvLoadKind, RiscvOp32Kind, RiscvOpImm32Kind,
    RiscvOpImmKind, RiscvOpKind, RiscvStoreKind,
};
use lzvm_prover::guest_memory::load_guest_memory_image;

fn sample_guest_image() -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x8000_0000_u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_guest_image_with_program_headers(program_headers: &[[u8; 56]]) -> Vec<u8> {
    let mut bytes = sample_guest_image();
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(program_headers.len() as u16).to_le_bytes());
    for header in program_headers {
        bytes.extend_from_slice(header);
    }
    bytes
}

#[derive(Debug, Clone, Copy)]
struct ProgramHeaderFixture {
    kind: u32,
    flags: u32,
    file_offset: u64,
    virtual_address: u64,
    physical_address: u64,
    file_size: u64,
    memory_size: u64,
    align: u64,
}

fn program_header(header: ProgramHeaderFixture) -> [u8; 56] {
    let mut bytes = [0_u8; 56];
    bytes[0..4].copy_from_slice(&header.kind.to_le_bytes());
    bytes[4..8].copy_from_slice(&header.flags.to_le_bytes());
    bytes[8..16].copy_from_slice(&header.file_offset.to_le_bytes());
    bytes[16..24].copy_from_slice(&header.virtual_address.to_le_bytes());
    bytes[24..32].copy_from_slice(&header.physical_address.to_le_bytes());
    bytes[32..40].copy_from_slice(&header.file_size.to_le_bytes());
    bytes[40..48].copy_from_slice(&header.memory_size.to_le_bytes());
    bytes[48..56].copy_from_slice(&header.align.to_le_bytes());
    bytes
}

#[test]
fn fetches_standard_instructions_from_guest_memory() {
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 8,
        memory_size: 16,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&0xfff0_0093_u32.to_le_bytes());
    image.extend_from_slice(&0x0080_006f_u32.to_le_bytes());
    let info = parse_guest_image(&image).expect("guest image should parse");
    let memory = load_guest_memory_image(&image, &info).expect("guest memory should load");

    assert_eq!(
        fetch_guest_instruction(&memory, 0x8000_0000).expect("first instruction should fetch"),
        FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Standard(0xfff0_0093),
        }
    );
    assert_eq!(
        decode_guest_instruction(
            fetch_guest_instruction(&memory, 0x8000_0004).expect("second instruction should fetch")
        ),
        RiscvInstruction::Jal { rd: 0, offset: 8 }
    );
    assert_eq!(
        decode_guest_instruction(
            fetch_guest_instruction(&memory, 0x8000_0008)
                .expect("zero tail instruction should fetch")
        ),
        RiscvInstruction::IllegalCompressed { halfword: 0 }
    );
}

#[test]
fn fetches_compressed_and_halfword_aligned_standard_instructions() {
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 6,
        memory_size: 8,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&0x0001_u16.to_le_bytes());
    image.extend_from_slice(&0x0010_0093_u32.to_le_bytes());
    let info = parse_guest_image(&image).expect("guest image should parse");
    let memory = load_guest_memory_image(&image, &info).expect("guest memory should load");

    assert_eq!(
        fetch_guest_instruction(&memory, 0x8000_0000).expect("compressed instruction should fetch"),
        FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x0001),
        }
    );
    assert_eq!(
        fetch_guest_instruction(&memory, 0x8000_0000)
            .expect("compressed instruction should fetch")
            .byte_len(),
        Some(2)
    );
    assert_eq!(
        decode_guest_instruction(
            fetch_guest_instruction(&memory, 0x8000_0000)
                .expect("compressed instruction should fetch")
        ),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 0,
            rs1: 0,
            immediate: 0,
        }
    );
    assert_eq!(
        fetch_guest_instruction(&memory, 0x8000_0002)
            .expect("standard instruction should fetch from halfword boundary"),
        FetchedGuestInstruction {
            address: 0x8000_0002,
            encoded: RiscvEncodedInstruction::Standard(0x0010_0093),
        }
    );
    assert_eq!(
        fetch_guest_instruction(&memory, 0x8000_0002)
            .expect("standard instruction should fetch from halfword boundary")
            .byte_len(),
        Some(4)
    );
    assert!(matches!(
        fetch_guest_instruction(&memory, 0x8000_0001),
        Err(GuestInstructionError::MisalignedFetch {
            address: 0x8000_0001
        })
    ));
}

#[test]
fn keeps_unsupported_long_instruction_encodings_visible() {
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 6,
        memory_size: 6,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&0x001f_u16.to_le_bytes());
    image.extend_from_slice(&0x003f_u16.to_le_bytes());
    image.extend_from_slice(&0x007f_u16.to_le_bytes());
    let info = parse_guest_image(&image).expect("guest image should parse");
    let memory = load_guest_memory_image(&image, &info).expect("guest memory should load");

    for (offset, halfword) in [(0_u64, 0x001f_u16), (2, 0x003f), (4, 0x007f)] {
        assert_eq!(
            fetch_guest_instruction(&memory, 0x8000_0000 + offset)
                .expect("long encoding prefix should fetch"),
            FetchedGuestInstruction {
                address: 0x8000_0000 + offset,
                encoded: RiscvEncodedInstruction::UnsupportedLong(halfword),
            }
        );
        assert_eq!(
            fetch_guest_instruction(&memory, 0x8000_0000 + offset)
                .expect("long encoding prefix should fetch")
                .byte_len(),
            None
        );
    }
}

#[test]
fn rejects_misaligned_and_unmapped_instruction_fetches() {
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 4,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&0x0010_0093_u32.to_le_bytes());
    let info = parse_guest_image(&image).expect("guest image should parse");
    let memory = load_guest_memory_image(&image, &info).expect("guest memory should load");

    assert!(matches!(
        fetch_guest_instruction(&memory, 0x8000_0001),
        Err(GuestInstructionError::MisalignedFetch {
            address: 0x8000_0001
        })
    ));
    assert!(matches!(
        fetch_guest_instruction(&memory, 0x8000_1000),
        Err(GuestInstructionError::Memory(_))
    ));
}

#[test]
fn decodes_common_riscv_instruction_formats() {
    assert_eq!(
        decode_riscv_instruction(0x1234_52b7),
        RiscvInstruction::Lui {
            rd: 5,
            immediate: 0x1234_5000,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0xffff_f2b7),
        RiscvInstruction::Lui {
            rd: 5,
            immediate: -4096,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0xfff0_0093),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 0,
            immediate: -1,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x4032_d313),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Srai,
            rd: 6,
            rs1: 5,
            immediate: 3,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x4212_d313),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Srai,
            rd: 6,
            rs1: 5,
            immediate: 33,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x0080_006f),
        RiscvInstruction::Jal { rd: 0, offset: 8 }
    );
    assert_eq!(
        decode_riscv_instruction(0xffdf_f06f),
        RiscvInstruction::Jal { rd: 0, offset: -4 }
    );
    assert_eq!(
        decode_riscv_instruction(0x0002_8067),
        RiscvInstruction::Jalr {
            rd: 0,
            rs1: 5,
            offset: 0,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0xfe20_8ce3),
        RiscvInstruction::Branch {
            kind: RiscvBranchKind::Beq,
            rs1: 1,
            rs2: 2,
            offset: -8,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x0081_3023),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 2,
            rs2: 8,
            offset: 0,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0xff81_3023),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 2,
            rs2: 24,
            offset: -32,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x0001_3383),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Ld,
            rd: 7,
            rs1: 2,
            offset: 0,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0xfe01_3383),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Ld,
            rd: 7,
            rs1: 2,
            offset: -32,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x00b5_0533),
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 10,
            rs1: 10,
            rs2: 11,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x0000_0073),
        RiscvInstruction::Ecall
    );
    assert_eq!(
        decode_riscv_instruction(0x0010_0073),
        RiscvInstruction::Ebreak
    );
}

#[test]
fn decodes_rv64_word_and_fence_instructions() {
    assert_eq!(
        decode_riscv_instruction(0xfff2_831b),
        RiscvInstruction::OpImm32 {
            kind: RiscvOpImm32Kind::Addiw,
            rd: 6,
            rs1: 5,
            immediate: -1,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x4032_d31b),
        RiscvInstruction::OpImm32 {
            kind: RiscvOpImm32Kind::Sraiw,
            rd: 6,
            rs1: 5,
            immediate: 3,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x00b5_053b),
        RiscvInstruction::Op32 {
            kind: RiscvOp32Kind::Addw,
            rd: 10,
            rs1: 10,
            rs2: 11,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x40b5_553b),
        RiscvInstruction::Op32 {
            kind: RiscvOp32Kind::Sraw,
            rd: 10,
            rs1: 10,
            rs2: 11,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x02b5_053b),
        RiscvInstruction::Op32 {
            kind: RiscvOp32Kind::Mulw,
            rd: 10,
            rs1: 10,
            rs2: 11,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x0ff0_000f),
        RiscvInstruction::Fence {
            kind: RiscvFenceKind::Fence,
            mode: 0,
            predecessor: 0xf,
            successor: 0xf,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x8330_000f),
        RiscvInstruction::Fence {
            kind: RiscvFenceKind::FenceTso,
            mode: 8,
            predecessor: 3,
            successor: 3,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x0000_100f),
        RiscvInstruction::Fence {
            kind: RiscvFenceKind::FenceI,
            mode: 0,
            predecessor: 0,
            successor: 0,
        }
    );
}

#[test]
fn decodes_compressed_addi_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x0001),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 0,
            rs1: 0,
            immediate: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x009d),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 1,
            immediate: 7,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x10fd),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 1,
            immediate: -1,
        }
    );
}

#[test]
fn decodes_compressed_addi4spn_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x0040),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 8,
            rs1: 2,
            immediate: 4,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x1ffc),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 15,
            rs1: 2,
            immediate: 1020,
        }
    );
}

#[test]
fn keeps_reserved_compressed_addi4spn_forms_visible() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x0004),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x0004,
            quadrant: 0,
            funct3: 0,
        }
    );
}

#[test]
fn decodes_compressed_addiw_and_slli_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x2085),
        }),
        RiscvInstruction::OpImm32 {
            kind: RiscvOpImm32Kind::Addiw,
            rd: 1,
            rs1: 1,
            immediate: 1,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x3ffd),
        }),
        RiscvInstruction::OpImm32 {
            kind: RiscvOpImm32Kind::Addiw,
            rd: 31,
            rs1: 31,
            immediate: -1,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x2081),
        }),
        RiscvInstruction::OpImm32 {
            kind: RiscvOpImm32Kind::Addiw,
            rd: 1,
            rs1: 1,
            immediate: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x0086),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Slli,
            rd: 1,
            rs1: 1,
            immediate: 1,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x1ffe),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Slli,
            rd: 31,
            rs1: 31,
            immediate: 63,
        }
    );
}

#[test]
fn keeps_reserved_compressed_addiw_and_slli_forms_visible() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x2005),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x2005,
            quadrant: 1,
            funct3: 1,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x0082),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x0082,
            quadrant: 2,
            funct3: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x0006),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x0006,
            quadrant: 2,
            funct3: 0,
        }
    );
}

#[test]
fn decodes_compressed_li_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x419d),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 3,
            rs1: 0,
            immediate: 7,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x51fd),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 3,
            rs1: 0,
            immediate: -1,
        }
    );
}

#[test]
fn decodes_compressed_lui_and_addi16sp_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x6185),
        }),
        RiscvInstruction::Lui {
            rd: 3,
            immediate: 4096,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x71fd),
        }),
        RiscvInstruction::Lui {
            rd: 3,
            immediate: -4096,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x6141),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 2,
            rs1: 2,
            immediate: 16,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x717d),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 2,
            rs1: 2,
            immediate: -16,
        }
    );
    for (halfword, immediate) in [
        (0x6105, 32),
        (0x6121, 64),
        (0x6109, 128),
        (0x6111, 256),
        (0x7101, -512),
    ] {
        assert_eq!(
            decode_guest_instruction(FetchedGuestInstruction {
                address: 0x8000_0000,
                encoded: RiscvEncodedInstruction::Compressed(halfword),
            }),
            RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 2,
                rs1: 2,
                immediate,
            }
        );
    }
}

#[test]
fn keeps_reserved_compressed_lui_forms_visible() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x6001),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x6001,
            quadrant: 1,
            funct3: 3,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x6101),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x6101,
            quadrant: 1,
            funct3: 3,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x6005),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x6005,
            quadrant: 1,
            funct3: 3,
        }
    );
}

#[test]
fn decodes_compressed_load_and_store_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x4080),
        }),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Lw,
            rd: 8,
            rs1: 9,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x5ffc),
        }),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Lw,
            rd: 15,
            rs1: 15,
            offset: 124,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x6080),
        }),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Ld,
            rd: 8,
            rs1: 9,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x7ffc),
        }),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Ld,
            rd: 15,
            rs1: 15,
            offset: 248,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xc080),
        }),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sw,
            rs1: 9,
            rs2: 8,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xdffc),
        }),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sw,
            rs1: 15,
            rs2: 15,
            offset: 124,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xe080),
        }),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 9,
            rs2: 8,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xfffc),
        }),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 15,
            rs2: 15,
            offset: 248,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x4182),
        }),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Lw,
            rd: 3,
            rs1: 2,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x5ffe),
        }),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Lw,
            rd: 31,
            rs1: 2,
            offset: 252,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x6182),
        }),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Ld,
            rd: 3,
            rs1: 2,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x7ffe),
        }),
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Ld,
            rd: 31,
            rs1: 2,
            offset: 504,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xc00e),
        }),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sw,
            rs1: 2,
            rs2: 3,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xdffe),
        }),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sw,
            rs1: 2,
            rs2: 31,
            offset: 252,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xe00e),
        }),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 2,
            rs2: 3,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xfffe),
        }),
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 2,
            rs2: 31,
            offset: 504,
        }
    );
}

#[test]
fn keeps_reserved_compressed_stack_load_forms_visible() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x4002),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x4002,
            quadrant: 2,
            funct3: 2,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x6002),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x6002,
            quadrant: 2,
            funct3: 3,
        }
    );
}

#[test]
fn decodes_compressed_register_control_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x8282),
        }),
        RiscvInstruction::Jalr {
            rd: 0,
            rs1: 5,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x9282),
        }),
        RiscvInstruction::Jalr {
            rd: 1,
            rs1: 5,
            offset: 0,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x831e),
        }),
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 6,
            rs1: 0,
            rs2: 7,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x931e),
        }),
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 6,
            rs1: 6,
            rs2: 7,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x801e),
        }),
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 0,
            rs1: 0,
            rs2: 7,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x901e),
        }),
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 0,
            rs1: 0,
            rs2: 7,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x9002),
        }),
        RiscvInstruction::Ebreak
    );
}

#[test]
fn keeps_reserved_compressed_register_control_forms_visible() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x8002),
        }),
        RiscvInstruction::CompressedUnknown {
            halfword: 0x8002,
            quadrant: 2,
            funct3: 4,
        }
    );
}

#[test]
fn decodes_compressed_jump_and_branch_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xa011),
        }),
        RiscvInstruction::Jal { rd: 0, offset: 4 }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xbffd),
        }),
        RiscvInstruction::Jal { rd: 0, offset: -2 }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xc011),
        }),
        RiscvInstruction::Branch {
            kind: RiscvBranchKind::Beq,
            rs1: 8,
            rs2: 0,
            offset: 4,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xdc7d),
        }),
        RiscvInstruction::Branch {
            kind: RiscvBranchKind::Beq,
            rs1: 8,
            rs2: 0,
            offset: -2,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xe391),
        }),
        RiscvInstruction::Branch {
            kind: RiscvBranchKind::Bne,
            rs1: 15,
            rs2: 0,
            offset: 4,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0xfffd),
        }),
        RiscvInstruction::Branch {
            kind: RiscvBranchKind::Bne,
            rs1: 15,
            rs2: 0,
            offset: -2,
        }
    );
}

#[test]
fn decodes_compressed_shift_logical_instructions() {
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x8005),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Srli,
            rd: 8,
            rs1: 8,
            immediate: 1,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x93fd),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Srli,
            rd: 15,
            rs1: 15,
            immediate: 63,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x8405),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Srai,
            rd: 8,
            rs1: 8,
            immediate: 1,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x97fd),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Srai,
            rd: 15,
            rs1: 15,
            immediate: 63,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x987d),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Andi,
            rd: 8,
            rs1: 8,
            immediate: -1,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x8bfd),
        }),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Andi,
            rd: 15,
            rs1: 15,
            immediate: 31,
        }
    );
    for (halfword, kind) in [
        (0x8c05, RiscvOpKind::Sub),
        (0x8c25, RiscvOpKind::Xor),
        (0x8c45, RiscvOpKind::Or),
        (0x8c65, RiscvOpKind::And),
    ] {
        assert_eq!(
            decode_guest_instruction(FetchedGuestInstruction {
                address: 0x8000_0000,
                encoded: RiscvEncodedInstruction::Compressed(halfword),
            }),
            RiscvInstruction::Op {
                kind,
                rd: 8,
                rs1: 8,
                rs2: 9,
            }
        );
    }
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x9c05),
        }),
        RiscvInstruction::Op32 {
            kind: RiscvOp32Kind::Subw,
            rd: 8,
            rs1: 8,
            rs2: 9,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0x9c25),
        }),
        RiscvInstruction::Op32 {
            kind: RiscvOp32Kind::Addw,
            rd: 8,
            rs1: 8,
            rs2: 9,
        }
    );
    for halfword in [0x9c45, 0x9c65] {
        assert_eq!(
            decode_guest_instruction(FetchedGuestInstruction {
                address: 0x8000_0000,
                encoded: RiscvEncodedInstruction::Compressed(halfword),
            }),
            RiscvInstruction::CompressedUnknown {
                halfword,
                quadrant: 1,
                funct3: 4,
            }
        );
    }
}

#[test]
fn keeps_unknown_riscv_words_visible() {
    assert_eq!(
        decode_riscv_instruction(0xffff_ffff),
        RiscvInstruction::Unknown {
            word: 0xffff_ffff,
            opcode: 0x7f,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0xfe32_d313),
        RiscvInstruction::Unknown {
            word: 0xfe32_d313,
            opcode: 0x13,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x0010_100f),
        RiscvInstruction::Unknown {
            word: 0x0010_100f,
            opcode: 0x0f,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x0ff0_008f),
        RiscvInstruction::Unknown {
            word: 0x0ff0_008f,
            opcode: 0x0f,
        }
    );
    assert_eq!(
        decode_riscv_instruction(0x8330_800f),
        RiscvInstruction::Unknown {
            word: 0x8330_800f,
            opcode: 0x0f,
        }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::Compressed(0),
        }),
        RiscvInstruction::IllegalCompressed { halfword: 0 }
    );
    assert_eq!(
        decode_guest_instruction(FetchedGuestInstruction {
            address: 0x8000_0000,
            encoded: RiscvEncodedInstruction::UnsupportedLong(0x001f),
        }),
        RiscvInstruction::UnsupportedLong { halfword: 0x001f }
    );
}

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

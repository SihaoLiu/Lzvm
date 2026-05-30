use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_prover::guest_instruction::RiscvInstruction;
use lzvm_prover::guest_machine::{
    run_guest_machine, GuestMachineError, GuestMachineHalt, GuestMachineMemory,
    GuestMachineRunError, GuestMachineState,
};
use lzvm_prover::guest_memory::{load_guest_memory_image, GuestMemoryImage};

const ENTRY: u64 = 0x8000_0000;

fn sample_guest_image() -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&ENTRY.to_le_bytes());
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

fn guest_memory_image_with_words(words: &[u32]) -> GuestMemoryImage {
    let mut code = Vec::with_capacity(words.len() * 4);
    for word in words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    guest_memory_image_with_bytes(&code)
}

fn guest_memory_image_with_bytes(code: &[u8]) -> GuestMemoryImage {
    let header = program_header(120, code.len() as u64);
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(code);
    let info = parse_guest_image(&image).expect("guest image should parse");
    load_guest_memory_image(&image, &info).expect("guest memory should load")
}

fn program_header(file_offset: u64, file_size: u64) -> [u8; 56] {
    let mut bytes = [0_u8; 56];
    bytes[0..4].copy_from_slice(&1_u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&5_u32.to_le_bytes());
    bytes[8..16].copy_from_slice(&file_offset.to_le_bytes());
    bytes[16..24].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[24..32].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[32..40].copy_from_slice(&file_size.to_le_bytes());
    bytes[40..48].copy_from_slice(&file_size.to_le_bytes());
    bytes[48..56].copy_from_slice(&0x1000_u64.to_le_bytes());
    bytes
}

fn guest_machine_memory_with_words(words: &[u32]) -> GuestMachineMemory {
    GuestMachineMemory::from_image(&guest_memory_image_with_words(words))
}

fn guest_machine_memory_with_bytes(code: &[u8]) -> GuestMachineMemory {
    GuestMachineMemory::from_image(&guest_memory_image_with_bytes(code))
}

fn encode_i(immediate: i16, rs1: u8, funct3: u8, rd: u8, opcode: u8) -> u32 {
    assert!((-2048..=2047).contains(&immediate));
    assert!(rs1 < 32);
    assert!(funct3 < 8);
    assert!(rd < 32);
    assert!(opcode < 128);
    (((immediate as i32 as u32) & 0x0fff) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | u32::from(opcode)
}

fn addi(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 0, rd, 0x13)
}

fn compressed_addi(rd: u8, immediate: i8) -> u16 {
    assert!(rd < 32);
    assert!((-32..=31).contains(&immediate));
    let immediate = immediate as i16 as u16;
    0b01 | (((immediate >> 5) & 1) << 12) | (u16::from(rd) << 7) | ((immediate & 0x1f) << 2)
}

fn compressed_li(rd: u8, immediate: i8) -> u16 {
    assert!(rd < 32);
    assert!((-32..=31).contains(&immediate));
    let immediate = immediate as i16 as u16;
    (0b010 << 13)
        | 0b01
        | (((immediate >> 5) & 1) << 12)
        | (u16::from(rd) << 7)
        | ((immediate & 0x1f) << 2)
}

fn compressed_lui(rd: u8, immediate: i8) -> u16 {
    assert!(rd < 32);
    assert_ne!(rd, 2);
    assert!((-32..=31).contains(&immediate));
    assert_ne!(immediate, 0);
    let immediate = immediate as i16 as u16;
    (0b011 << 13)
        | 0b01
        | (((immediate >> 5) & 1) << 12)
        | (u16::from(rd) << 7)
        | ((immediate & 0x1f) << 2)
}

fn compressed_addi16sp(immediate: i16) -> u16 {
    assert!((-512..=496).contains(&immediate));
    assert_ne!(immediate, 0);
    assert_eq!(immediate & 0x0f, 0);
    let immediate = immediate as u16;
    (0b011 << 13)
        | 0b01
        | (((immediate >> 9) & 1) << 12)
        | (2 << 7)
        | (((immediate >> 4) & 1) << 6)
        | (((immediate >> 6) & 1) << 5)
        | (((immediate >> 7) & 0x3) << 3)
        | (((immediate >> 5) & 1) << 2)
}

fn compressed_lw(rd: u8, rs1: u8, offset: u16) -> u16 {
    assert!((8..=15).contains(&rd));
    assert!((8..=15).contains(&rs1));
    assert!(offset <= 124);
    assert_eq!(offset & 0x3, 0);
    (0b010 << 13)
        | (((offset >> 3) & 0x7) << 10)
        | (u16::from(rs1 - 8) << 7)
        | (((offset >> 2) & 1) << 6)
        | (((offset >> 6) & 1) << 5)
        | (u16::from(rd - 8) << 2)
}

fn compressed_ld(rd: u8, rs1: u8, offset: u16) -> u16 {
    assert!((8..=15).contains(&rd));
    assert!((8..=15).contains(&rs1));
    assert!(offset <= 248);
    assert_eq!(offset & 0x7, 0);
    (0b011 << 13)
        | (((offset >> 3) & 0x7) << 10)
        | (u16::from(rs1 - 8) << 7)
        | (((offset >> 6) & 0x3) << 5)
        | (u16::from(rd - 8) << 2)
}

fn compressed_sw(rs2: u8, rs1: u8, offset: u16) -> u16 {
    assert!((8..=15).contains(&rs2));
    assert!((8..=15).contains(&rs1));
    assert!(offset <= 124);
    assert_eq!(offset & 0x3, 0);
    (0b110 << 13)
        | (((offset >> 3) & 0x7) << 10)
        | (u16::from(rs1 - 8) << 7)
        | (((offset >> 2) & 1) << 6)
        | (((offset >> 6) & 1) << 5)
        | (u16::from(rs2 - 8) << 2)
}

fn compressed_sd(rs2: u8, rs1: u8, offset: u16) -> u16 {
    assert!((8..=15).contains(&rs2));
    assert!((8..=15).contains(&rs1));
    assert!(offset <= 248);
    assert_eq!(offset & 0x7, 0);
    (0b111 << 13)
        | (((offset >> 3) & 0x7) << 10)
        | (u16::from(rs1 - 8) << 7)
        | (((offset >> 6) & 0x3) << 5)
        | (u16::from(rs2 - 8) << 2)
}

fn compressed_ldsp(rd: u8, offset: u16) -> u16 {
    assert!((1..32).contains(&rd));
    assert!(offset <= 504);
    assert_eq!(offset & 0x7, 0);
    (0b011 << 13)
        | 0b10
        | (((offset >> 5) & 1) << 12)
        | (u16::from(rd) << 7)
        | (((offset >> 3) & 0x3) << 5)
        | (((offset >> 6) & 0x7) << 2)
}

fn compressed_lwsp(rd: u8, offset: u16) -> u16 {
    assert!((1..32).contains(&rd));
    assert!(offset <= 252);
    assert_eq!(offset & 0x3, 0);
    (0b010 << 13)
        | 0b10
        | (((offset >> 5) & 1) << 12)
        | (u16::from(rd) << 7)
        | (((offset >> 2) & 0x7) << 4)
        | (((offset >> 6) & 0x3) << 2)
}

fn compressed_sdsp(rs2: u8, offset: u16) -> u16 {
    assert!(rs2 < 32);
    assert!(offset <= 504);
    assert_eq!(offset & 0x7, 0);
    (0b111 << 13)
        | 0b10
        | (((offset >> 3) & 0x7) << 10)
        | (((offset >> 6) & 0x7) << 7)
        | (u16::from(rs2) << 2)
}

fn compressed_swsp(rs2: u8, offset: u16) -> u16 {
    assert!(rs2 < 32);
    assert!(offset <= 252);
    assert_eq!(offset & 0x3, 0);
    (0b110 << 13)
        | 0b10
        | (((offset >> 2) & 0xf) << 9)
        | (((offset >> 6) & 0x3) << 7)
        | (u16::from(rs2) << 2)
}

fn compressed_jr(rs1: u8) -> u16 {
    assert!((1..32).contains(&rs1));
    (0b1000 << 12) | (u16::from(rs1) << 7) | 0b10
}

fn compressed_jalr(rs1: u8) -> u16 {
    assert!((1..32).contains(&rs1));
    (0b1001 << 12) | (u16::from(rs1) << 7) | 0b10
}

fn compressed_mv(rd: u8, rs2: u8) -> u16 {
    assert!(rd < 32);
    assert!((1..32).contains(&rs2));
    (0b1000 << 12) | (u16::from(rd) << 7) | (u16::from(rs2) << 2) | 0b10
}

fn compressed_add(rd: u8, rs2: u8) -> u16 {
    assert!(rd < 32);
    assert!((1..32).contains(&rs2));
    (0b1001 << 12) | (u16::from(rd) << 7) | (u16::from(rs2) << 2) | 0b10
}

fn compressed_ebreak() -> u16 {
    0x9002
}

fn compressed_jump(offset: i16) -> u16 {
    assert!((-2048..=2046).contains(&offset));
    assert_eq!(offset & 1, 0);
    let offset = offset as u16;
    (0b101 << 13)
        | 0b01
        | (((offset >> 11) & 1) << 12)
        | (((offset >> 4) & 1) << 11)
        | (((offset >> 8) & 0x3) << 9)
        | (((offset >> 10) & 1) << 8)
        | (((offset >> 6) & 1) << 7)
        | (((offset >> 7) & 1) << 6)
        | (((offset >> 1) & 0x7) << 3)
        | (((offset >> 5) & 1) << 2)
}

fn compressed_beqz(rs1: u8, offset: i16) -> u16 {
    compressed_branch(0b110, rs1, offset)
}

fn compressed_bnez(rs1: u8, offset: i16) -> u16 {
    compressed_branch(0b111, rs1, offset)
}

fn compressed_srli(rd: u8, shamt: u8) -> u16 {
    compressed_shift(0b00, rd, shamt)
}

fn compressed_srai(rd: u8, shamt: u8) -> u16 {
    compressed_shift(0b01, rd, shamt)
}

fn compressed_shift(funct2: u16, rd: u8, shamt: u8) -> u16 {
    assert!(funct2 < 2);
    assert!((8..=15).contains(&rd));
    assert!(shamt < 64);
    (0b100 << 13)
        | 0b01
        | ((u16::from(shamt >> 5) & 1) << 12)
        | (funct2 << 10)
        | (u16::from(rd - 8) << 7)
        | ((u16::from(shamt) & 0x1f) << 2)
}

fn compressed_andi(rd: u8, immediate: i8) -> u16 {
    assert!((8..=15).contains(&rd));
    assert!((-32..=31).contains(&immediate));
    let immediate = immediate as i16 as u16;
    (0b100 << 13)
        | 0b01
        | (((immediate >> 5) & 1) << 12)
        | (0b10 << 10)
        | (u16::from(rd - 8) << 7)
        | ((immediate & 0x1f) << 2)
}

fn compressed_sub(rd: u8, rs2: u8) -> u16 {
    compressed_register_arithmetic(0b00, rd, rs2)
}

fn compressed_xor(rd: u8, rs2: u8) -> u16 {
    compressed_register_arithmetic(0b01, rd, rs2)
}

fn compressed_or(rd: u8, rs2: u8) -> u16 {
    compressed_register_arithmetic(0b10, rd, rs2)
}

fn compressed_and(rd: u8, rs2: u8) -> u16 {
    compressed_register_arithmetic(0b11, rd, rs2)
}

fn compressed_subw(rd: u8, rs2: u8) -> u16 {
    compressed_register_arithmetic_word(0b00, rd, rs2)
}

fn compressed_addw(rd: u8, rs2: u8) -> u16 {
    compressed_register_arithmetic_word(0b01, rd, rs2)
}

fn compressed_register_arithmetic(kind: u16, rd: u8, rs2: u8) -> u16 {
    assert!(kind < 4);
    assert!((8..=15).contains(&rd));
    assert!((8..=15).contains(&rs2));
    (0b100 << 13)
        | 0b01
        | (0b11 << 10)
        | (u16::from(rd - 8) << 7)
        | (kind << 5)
        | (u16::from(rs2 - 8) << 2)
}

fn compressed_register_arithmetic_word(kind: u16, rd: u8, rs2: u8) -> u16 {
    assert!(kind < 2);
    assert!((8..=15).contains(&rd));
    assert!((8..=15).contains(&rs2));
    (0b100 << 13)
        | 0b01
        | (1 << 12)
        | (0b11 << 10)
        | (u16::from(rd - 8) << 7)
        | (kind << 5)
        | (u16::from(rs2 - 8) << 2)
}

fn compressed_branch(funct3: u16, rs1: u8, offset: i16) -> u16 {
    assert!((8..=15).contains(&rs1));
    assert!((-256..=254).contains(&offset));
    assert_eq!(offset & 1, 0);
    let offset = offset as u16;
    (funct3 << 13)
        | 0b01
        | (((offset >> 8) & 1) << 12)
        | (((offset >> 3) & 0x3) << 10)
        | (u16::from(rs1 - 8) << 7)
        | (((offset >> 6) & 0x3) << 5)
        | (((offset >> 1) & 0x3) << 3)
        | (((offset >> 5) & 1) << 2)
}

fn push_halfword(code: &mut Vec<u8>, halfword: u16) {
    code.extend_from_slice(&halfword.to_le_bytes());
}

fn push_word(code: &mut Vec<u8>, word: u32) {
    code.extend_from_slice(&word.to_le_bytes());
}

#[test]
fn runs_guest_machine_until_ecall() {
    let mut memory = guest_machine_memory_with_words(&[
        addi(1, 0, 7),
        addi(2, 1, 3),
        0x0000_0073,
        addi(3, 0, 9),
    ]);
    let mut state = GuestMachineState::new(memory.entry_address());

    let report = run_guest_machine(&mut memory, &mut state, 10).expect("guest should halt");

    assert_eq!(report.executed_instructions, 2);
    assert_eq!(report.halt, GuestMachineHalt::Ecall { address: ENTRY + 8 });
    assert_eq!(state.pc(), ENTRY + 8);
    assert_eq!(state.register(1), Some(7));
    assert_eq!(state.register(2), Some(10));
    assert_eq!(state.register(3), Some(0));
}

#[test]
fn runs_compressed_addi_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_addi(0, 0));
    push_halfword(&mut code, compressed_addi(1, 7));
    push_halfword(&mut code, compressed_addi(1, -1));
    push_word(&mut code, 0x0000_0073);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());

    let report = run_guest_machine(&mut memory, &mut state, 8).expect("guest should halt");

    assert_eq!(report.executed_instructions, 3);
    assert_eq!(report.halt, GuestMachineHalt::Ecall { address: ENTRY + 6 });
    assert_eq!(state.pc(), ENTRY + 6);
    assert_eq!(state.register(1), Some(6));
}

#[test]
fn runs_compressed_li_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_li(3, 7));
    push_halfword(&mut code, compressed_li(3, -1));
    push_word(&mut code, 0x0000_0073);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());

    let report = run_guest_machine(&mut memory, &mut state, 8).expect("guest should halt");

    assert_eq!(report.executed_instructions, 2);
    assert_eq!(report.halt, GuestMachineHalt::Ecall { address: ENTRY + 4 });
    assert_eq!(state.pc(), ENTRY + 4);
    assert_eq!(state.register(3), Some(u64::MAX));
}

#[test]
fn runs_compressed_lui_and_addi16sp_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_lui(3, 1));
    push_halfword(&mut code, compressed_lui(4, -1));
    push_halfword(&mut code, compressed_addi16sp(16));
    push_halfword(&mut code, compressed_addi16sp(-16));
    push_word(&mut code, 0x0000_0073);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());

    let report = run_guest_machine(&mut memory, &mut state, 8).expect("guest should halt");

    assert_eq!(report.executed_instructions, 4);
    assert_eq!(report.halt, GuestMachineHalt::Ecall { address: ENTRY + 8 });
    assert_eq!(state.pc(), ENTRY + 8);
    assert_eq!(state.register(2), Some(0));
    assert_eq!(state.register(3), Some(4096));
    assert_eq!(state.register(4), Some(u64::MAX << 12));
}

#[test]
fn runs_compressed_load_store_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_sw(8, 9, 20));
    push_halfword(&mut code, compressed_lw(14, 9, 20));
    push_halfword(&mut code, compressed_swsp(31, 252));
    push_halfword(&mut code, compressed_lwsp(4, 252));
    push_halfword(&mut code, compressed_sd(8, 9, 8));
    push_halfword(&mut code, compressed_ld(15, 9, 8));
    push_halfword(&mut code, compressed_sdsp(31, 16));
    push_halfword(&mut code, compressed_ldsp(3, 16));
    push_word(&mut code, 0x0000_0073);
    code.resize(384, 0);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(8, 0x1122_3344_ffee_ddcc)
        .expect("register should set");
    state
        .set_register(9, ENTRY + 64)
        .expect("register should set");
    state
        .set_register(31, 0x8877_6655_cc33_2211)
        .expect("register should set");
    state
        .set_register(2, ENTRY + 96)
        .expect("register should set");

    let report = run_guest_machine(&mut memory, &mut state, 8).expect("guest should halt");

    assert_eq!(report.executed_instructions, 8);
    assert_eq!(
        report.halt,
        GuestMachineHalt::Ecall {
            address: ENTRY + 16
        }
    );
    assert_eq!(state.pc(), ENTRY + 16);
    assert_eq!(state.register(14), Some(0xffff_ffff_ffee_ddcc));
    assert_eq!(state.register(15), Some(0x1122_3344_ffee_ddcc));
    assert_eq!(state.register(4), Some(0xffff_ffff_cc33_2211));
    assert_eq!(state.register(3), Some(0x8877_6655_cc33_2211));
    let mut compressed_stored = [0_u8; 8];
    memory
        .read_range_into(ENTRY + 84, &mut compressed_stored)
        .expect("stored bytes should read");
    assert_eq!(compressed_stored, [0xcc, 0xdd, 0xee, 0xff, 0, 0, 0, 0]);
    let mut stack_stored = [0_u8; 8];
    memory
        .read_range_into(ENTRY + 348, &mut stack_stored)
        .expect("stored bytes should read");
    assert_eq!(stack_stored, [0x11, 0x22, 0x33, 0xcc, 0, 0, 0, 0]);
}

#[test]
fn runs_compressed_register_control_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_mv(6, 7));
    push_halfword(&mut code, compressed_add(6, 7));
    push_halfword(&mut code, compressed_jalr(5));
    push_halfword(&mut code, compressed_addi(6, 2));
    push_word(&mut code, 0x0000_0073);
    code.resize(16, 0);
    push_halfword(&mut code, compressed_addi(6, 1));
    push_halfword(&mut code, compressed_jr(1));
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(5, ENTRY + 16)
        .expect("register should set");
    state.set_register(7, 9).expect("register should set");

    let report = run_guest_machine(&mut memory, &mut state, 10).expect("guest should halt");

    assert_eq!(report.executed_instructions, 6);
    assert_eq!(report.halt, GuestMachineHalt::Ecall { address: ENTRY + 8 });
    assert_eq!(state.pc(), ENTRY + 8);
    assert_eq!(state.register(1), Some(ENTRY + 6));
    assert_eq!(state.register(6), Some(21));
}

#[test]
fn runs_compressed_jump_and_branch_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_beqz(8, 4));
    push_halfword(&mut code, compressed_addi(6, 1));
    push_halfword(&mut code, compressed_bnez(15, 4));
    push_halfword(&mut code, compressed_addi(6, 2));
    push_halfword(&mut code, compressed_jump(4));
    push_halfword(&mut code, compressed_addi(6, 4));
    push_word(&mut code, 0x0000_0073);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state.set_register(15, 1).expect("register should set");

    let report = run_guest_machine(&mut memory, &mut state, 8).expect("guest should halt");

    assert_eq!(report.executed_instructions, 3);
    assert_eq!(
        report.halt,
        GuestMachineHalt::Ecall {
            address: ENTRY + 12
        }
    );
    assert_eq!(state.pc(), ENTRY + 12);
    assert_eq!(state.register(6), Some(0));
}

#[test]
fn runs_compressed_shift_logical_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_srli(8, 1));
    push_halfword(&mut code, compressed_srai(9, 63));
    push_halfword(&mut code, compressed_andi(10, -16));
    push_halfword(&mut code, compressed_sub(11, 12));
    push_halfword(&mut code, compressed_xor(13, 14));
    push_halfword(&mut code, compressed_or(15, 14));
    push_halfword(&mut code, compressed_and(12, 14));
    push_word(&mut code, 0x0000_0073);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(8, 0xffff_ffff_ffff_fffe)
        .expect("register should set");
    state
        .set_register(9, 0x8000_0000_0000_0000)
        .expect("register should set");
    state.set_register(10, 0x1234).expect("register should set");
    state.set_register(11, 0x20).expect("register should set");
    state.set_register(12, 0x05).expect("register should set");
    state.set_register(13, 0xf0).expect("register should set");
    state.set_register(14, 0x0f).expect("register should set");
    state.set_register(15, 0xf0).expect("register should set");

    let report = run_guest_machine(&mut memory, &mut state, 10).expect("guest should halt");

    assert_eq!(report.executed_instructions, 7);
    assert_eq!(
        report.halt,
        GuestMachineHalt::Ecall {
            address: ENTRY + 14
        }
    );
    assert_eq!(state.pc(), ENTRY + 14);
    assert_eq!(state.register(8), Some(0x7fff_ffff_ffff_ffff));
    assert_eq!(state.register(9), Some(u64::MAX));
    assert_eq!(state.register(10), Some(0x1230));
    assert_eq!(state.register(11), Some(0x1b));
    assert_eq!(state.register(12), Some(0x05));
    assert_eq!(state.register(13), Some(0xff));
    assert_eq!(state.register(15), Some(0xff));
}

#[test]
fn runs_compressed_word_arithmetic_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_subw(8, 9));
    push_halfword(&mut code, compressed_addw(10, 11));
    push_word(&mut code, 0x0000_0073);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(8, 0x0000_0001_0000_0000)
        .expect("register should set");
    state.set_register(9, 1).expect("register should set");
    state
        .set_register(10, 0x0000_0000_7fff_ffff)
        .expect("register should set");
    state.set_register(11, 1).expect("register should set");

    let report = run_guest_machine(&mut memory, &mut state, 8).expect("guest should halt");

    assert_eq!(report.executed_instructions, 2);
    assert_eq!(report.halt, GuestMachineHalt::Ecall { address: ENTRY + 4 });
    assert_eq!(state.pc(), ENTRY + 4);
    assert_eq!(state.register(8), Some(u64::MAX));
    assert_eq!(state.register(10), Some(0xffff_ffff_8000_0000));
}

#[test]
fn rejects_guest_runs_that_exceed_the_instruction_limit() {
    let mut memory = guest_machine_memory_with_words(&[addi(1, 0, 7), 0x0000_0073]);
    let mut state = GuestMachineState::new(memory.entry_address());

    assert_eq!(
        run_guest_machine(&mut memory, &mut state, 0),
        Err(GuestMachineRunError::InstructionLimitExceeded {
            instruction_limit: 0,
            pc: ENTRY,
        })
    );
    assert_eq!(state.pc(), ENTRY);
    assert_eq!(state.register(1), Some(0));
}

#[test]
fn allows_guest_runs_that_reach_ecall_at_the_instruction_limit() {
    let mut memory = guest_machine_memory_with_words(&[addi(1, 0, 7), 0x0000_0073]);
    let mut state = GuestMachineState::new(memory.entry_address());

    let report = run_guest_machine(&mut memory, &mut state, 1).expect("guest should halt");

    assert_eq!(report.executed_instructions, 1);
    assert_eq!(report.halt, GuestMachineHalt::Ecall { address: ENTRY + 4 });
    assert_eq!(state.pc(), ENTRY + 4);
    assert_eq!(state.register(1), Some(7));
}

#[test]
fn rejects_guest_runs_at_the_instruction_limit_before_non_halt() {
    let mut memory = guest_machine_memory_with_words(&[addi(1, 0, 7), addi(2, 1, 3)]);
    let mut state = GuestMachineState::new(memory.entry_address());

    assert_eq!(
        run_guest_machine(&mut memory, &mut state, 1),
        Err(GuestMachineRunError::InstructionLimitExceeded {
            instruction_limit: 1,
            pc: ENTRY + 4,
        })
    );
    assert_eq!(state.pc(), ENTRY + 4);
    assert_eq!(state.register(1), Some(7));
    assert_eq!(state.register(2), Some(0));
}

#[test]
fn does_not_treat_ebreak_as_guest_run_halt() {
    let mut memory = guest_machine_memory_with_words(&[0x0010_0073]);
    let mut state = GuestMachineState::new(memory.entry_address());

    assert_eq!(
        run_guest_machine(&mut memory, &mut state, 4),
        Err(GuestMachineRunError::Instruction(
            GuestMachineError::UnsupportedInstruction {
                address: ENTRY,
                instruction: RiscvInstruction::Ebreak,
            }
        ))
    );
    assert_eq!(state.pc(), ENTRY);
}

#[test]
fn does_not_treat_compressed_ebreak_as_guest_run_halt() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_ebreak());
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());

    assert_eq!(
        run_guest_machine(&mut memory, &mut state, 4),
        Err(GuestMachineRunError::Instruction(
            GuestMachineError::UnsupportedInstruction {
                address: ENTRY,
                instruction: RiscvInstruction::Ebreak,
            }
        ))
    );
    assert_eq!(state.pc(), ENTRY);
}

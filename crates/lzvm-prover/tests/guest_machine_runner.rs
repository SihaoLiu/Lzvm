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

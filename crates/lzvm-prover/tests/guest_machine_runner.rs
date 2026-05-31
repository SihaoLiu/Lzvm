use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_prover::guest_instruction::RiscvInstruction;
use lzvm_prover::guest_machine::{
    run_guest_machine, run_guest_machine_trace, run_guest_machine_trace_with_fcalls,
    run_guest_machine_with_fcalls, GuestFcallHandler, GuestFcallParam, GuestFcallRequest,
    GuestFcallResponse, GuestMachineError, GuestMachineHalt, GuestMachineMemory,
    GuestMachineRunError, GuestMachineState, GuestMemoryAccess, GuestMemoryAccessKind,
    GuestRegisterWrite, ZISK_ARCHITECTURE_ID,
};
use lzvm_prover::guest_memory::{load_guest_memory_image, GuestMemoryImage};
use lzvm_prover::zisk_fcalls::{
    ZiskInputFcallHandler, ZISK_INPUT_ADDRESS, ZISK_INPUT_READY_FCALL_ID,
    ZISK_MSB_POS_256_FCALL_ID, ZISK_SECP256K1_ECDSA_VERIFY_FCALL_ID,
};

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

fn guest_machine_memory_with_words_and_data(
    words: &[u32],
    data_offset: usize,
    data: &[u8],
) -> GuestMachineMemory {
    let mut code = Vec::with_capacity(data_offset + data.len());
    for word in words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    assert!(code.len() <= data_offset);
    code.resize(data_offset, 0);
    code.extend_from_slice(data);
    guest_machine_memory_with_bytes(&code)
}

fn push_u64_array<const N: usize>(bytes: &mut Vec<u8>, values: &[u64; N]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
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

fn load(funct3: u8, rd: u8, rs1: u8, offset: i16) -> u32 {
    encode_i(offset, rs1, funct3, rd, 0x03)
}

fn ld(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(3, rd, rs1, offset)
}

fn lbu(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(4, rd, rs1, offset)
}

fn lb(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(0, rd, rs1, offset)
}

fn store(funct3: u8, rs1: u8, rs2: u8, offset: i16) -> u32 {
    assert!((-2048..=2047).contains(&offset));
    assert!(rs1 < 32);
    assert!(rs2 < 32);
    assert!(funct3 < 8);
    let offset = offset as i32 as u32;
    (((offset >> 5) & 0x7f) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | ((offset & 0x1f) << 7)
        | 0x23
}

fn sd(rs1: u8, rs2: u8, offset: i16) -> u32 {
    store(3, rs1, rs2, offset)
}

fn encode_amo(
    funct5: u8,
    acquire: bool,
    release: bool,
    rs2: u8,
    rs1: u8,
    funct3: u8,
    rd: u8,
) -> u32 {
    assert!(funct5 < 32);
    assert!(rs2 < 32);
    assert!(rs1 < 32);
    assert!(funct3 < 8);
    assert!(rd < 32);
    (u32::from(funct5) << 27)
        | (u32::from(acquire) << 26)
        | (u32::from(release) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | 0x2f
}

fn lr_w(rd: u8, rs1: u8) -> u32 {
    encode_amo(0x02, false, false, 0, rs1, 2, rd)
}

fn sc_w(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x03, false, false, rs2, rs1, 2, rd)
}

fn amoadd_w(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x00, false, false, rs2, rs1, 2, rd)
}

fn lui(rd: u8, immediate: u32) -> u32 {
    assert!(rd < 32);
    assert_eq!(immediate & 0x0fff, 0);
    (immediate & 0xffff_f000) | (u32::from(rd) << 7) | 0x37
}

fn auipc(rd: u8, immediate: u32) -> u32 {
    assert!(rd < 32);
    assert_eq!(immediate & 0x0fff, 0);
    (immediate & 0xffff_f000) | (u32::from(rd) << 7) | 0x17
}

fn encode_csr(rd: u8, csr: u16, funct3: u8, source: u8) -> u32 {
    assert!(rd < 32);
    assert!(csr < 4096);
    assert!(funct3 < 8);
    assert!(source < 32);
    (u32::from(csr) << 20)
        | (u32::from(source) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | 0x73
}

fn csrr(rd: u8, csr: u16) -> u32 {
    encode_csr(rd, csr, 2, 0)
}

fn csrs(csr: u16, rs1: u8) -> u32 {
    encode_csr(0, csr, 2, rs1)
}

fn csrwi(csr: u16, immediate: u8) -> u32 {
    encode_csr(0, csr, 5, immediate)
}

fn compressed_addi(rd: u8, immediate: i8) -> u16 {
    assert!(rd < 32);
    assert!((-32..=31).contains(&immediate));
    let immediate = immediate as i16 as u16;
    0b01 | (((immediate >> 5) & 1) << 12) | (u16::from(rd) << 7) | ((immediate & 0x1f) << 2)
}

fn compressed_addiw(rd: u8, immediate: i8) -> u16 {
    assert!((1..32).contains(&rd));
    assert!((-32..=31).contains(&immediate));
    let immediate = immediate as i16 as u16;
    (0b001 << 13)
        | 0b01
        | (((immediate >> 5) & 1) << 12)
        | (u16::from(rd) << 7)
        | ((immediate & 0x1f) << 2)
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

fn compressed_addi4spn(rd: u8, immediate: u16) -> u16 {
    assert!((8..=15).contains(&rd));
    assert!((4..=1020).contains(&immediate));
    assert_eq!(immediate & 0x3, 0);
    (((immediate >> 4) & 0x3) << 11)
        | (((immediate >> 6) & 0xf) << 7)
        | (((immediate >> 2) & 1) << 6)
        | (((immediate >> 3) & 1) << 5)
        | (u16::from(rd - 8) << 2)
}

fn compressed_slli(rd: u8, shamt: u8) -> u16 {
    assert!((1..32).contains(&rd));
    assert!((1..64).contains(&shamt));
    0b10 | (((u16::from(shamt) >> 5) & 1) << 12)
        | (u16::from(rd) << 7)
        | ((u16::from(shamt) & 0x1f) << 2)
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
fn traces_guest_machine_until_ecall() {
    let mut memory = guest_machine_memory_with_words(&[
        addi(1, 0, 7),
        addi(2, 1, 3),
        0x0000_0073,
        addi(3, 0, 9),
    ]);
    let mut state = GuestMachineState::new(memory.entry_address());

    let trace = run_guest_machine_trace(&mut memory, &mut state, 10).expect("guest should halt");

    assert_eq!(trace.run.executed_instructions, 2);
    assert_eq!(
        trace.run.halt,
        GuestMachineHalt::Ecall { address: ENTRY + 8 }
    );
    assert_eq!(trace.reports.len(), 2);
    assert_eq!(trace.reports[0].address, ENTRY);
    assert_eq!(trace.reports[0].next_pc, ENTRY + 4);
    assert_eq!(trace.reports[1].address, ENTRY + 4);
    assert_eq!(trace.reports[1].next_pc, ENTRY + 8);
    assert_eq!(state.register(1), Some(7));
    assert_eq!(state.register(2), Some(10));
}

#[test]
fn traces_guest_machine_register_and_memory_effects_until_ecall() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut code = Vec::new();
    push_word(&mut code, addi(2, 0, 7));
    push_word(&mut code, ld(3, 1, 0));
    push_word(&mut code, sd(1, 2, 8));
    push_word(&mut code, lb(4, 1, 16));
    push_word(&mut code, 0x0000_0073);
    code.resize(data_offset, 0);
    code.extend_from_slice(&0x0123_4567_89ab_cdef_u64.to_le_bytes());
    code.extend_from_slice(&0_u64.to_le_bytes());
    code.push(0x80);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, data_address)
        .expect("address register should set");

    let trace = run_guest_machine_trace(&mut memory, &mut state, 10).expect("guest should halt");

    assert_eq!(trace.run.executed_instructions, 4);
    assert_eq!(
        trace.reports[0].register_writes,
        vec![GuestRegisterWrite { index: 2, value: 7 }]
    );
    assert!(trace.reports[0].memory_accesses.is_empty());
    assert_eq!(
        trace.reports[1].memory_accesses,
        vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Read,
            address: data_address,
            byte_len: 8,
            value: 0x0123_4567_89ab_cdef,
        }]
    );
    assert_eq!(
        trace.reports[1].register_writes,
        vec![GuestRegisterWrite {
            index: 3,
            value: 0x0123_4567_89ab_cdef,
        }]
    );
    assert_eq!(
        trace.reports[2].memory_accesses,
        vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: data_address + 8,
            byte_len: 8,
            value: 7,
        }]
    );
    assert!(trace.reports[2].register_writes.is_empty());
    assert_eq!(
        trace.reports[3].memory_accesses,
        vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Read,
            address: data_address + 16,
            byte_len: 1,
            value: 0x80,
        }]
    );
    assert_eq!(
        trace.reports[3].register_writes,
        vec![GuestRegisterWrite {
            index: 4,
            value: (-128_i64) as u64,
        }]
    );
}

#[test]
fn traces_guest_machine_atomic_effects_until_ecall() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut code = Vec::new();
    push_word(&mut code, lr_w(3, 1));
    push_word(&mut code, sc_w(4, 1, 2));
    push_word(&mut code, sc_w(5, 1, 2));
    push_word(&mut code, amoadd_w(6, 1, 7));
    push_word(&mut code, 0x0000_0073);
    code.resize(data_offset, 0);
    code.extend_from_slice(&0x8000_0001_u32.to_le_bytes());
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, data_address)
        .expect("address register should set");
    state
        .set_register(2, 0x1122_3344_5566_7788)
        .expect("value register should set");
    state
        .set_register(7, 0xffff_ffff_0000_0005)
        .expect("amo operand register should set");

    let trace = run_guest_machine_trace(&mut memory, &mut state, 10).expect("guest should halt");

    assert_eq!(trace.run.executed_instructions, 4);
    assert_eq!(
        trace.reports[0].memory_accesses,
        vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Read,
            address: data_address,
            byte_len: 4,
            value: 0x8000_0001,
        }]
    );
    assert_eq!(
        trace.reports[0].register_writes,
        vec![GuestRegisterWrite {
            index: 3,
            value: 0xffff_ffff_8000_0001,
        }]
    );
    assert_eq!(
        trace.reports[1].memory_accesses,
        vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: data_address,
            byte_len: 4,
            value: 0x5566_7788,
        }]
    );
    assert_eq!(
        trace.reports[1].register_writes,
        vec![GuestRegisterWrite { index: 4, value: 0 }]
    );
    assert!(trace.reports[2].memory_accesses.is_empty());
    assert_eq!(
        trace.reports[2].register_writes,
        vec![GuestRegisterWrite { index: 5, value: 1 }]
    );
    assert_eq!(
        trace.reports[3].memory_accesses,
        vec![
            GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Read,
                address: data_address,
                byte_len: 4,
                value: 0x5566_7788,
            },
            GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: data_address,
                byte_len: 4,
                value: 0x5566_778d,
            },
        ]
    );
    assert_eq!(
        trace.reports[3].register_writes,
        vec![GuestRegisterWrite {
            index: 6,
            value: 0x5566_7788,
        }]
    );
}

struct RecordingFcallHandler {
    requests: Vec<GuestFcallRequest>,
    results: Vec<u64>,
}

impl RecordingFcallHandler {
    fn with_results(results: Vec<u64>) -> Self {
        Self {
            requests: Vec::new(),
            results,
        }
    }
}

impl Default for RecordingFcallHandler {
    fn default() -> Self {
        Self::with_results(vec![0x2a])
    }
}

impl GuestFcallHandler for RecordingFcallHandler {
    fn handle_fcall(
        &mut self,
        request: GuestFcallRequest,
        _memory: &mut GuestMachineMemory,
    ) -> Result<GuestFcallResponse, lzvm_prover::guest_machine::GuestFcallError> {
        self.requests.push(request);
        Ok(GuestFcallResponse {
            results: self.results.clone(),
        })
    }
}

#[test]
fn runs_guest_machine_with_zisk_free_call_handler() {
    let mut memory = guest_machine_memory_with_words(&[
        addi(5, 0, 17),
        csrs(0x08f0, 5),
        csrwi(0x08c0, 7),
        csrr(6, 0x0ffe),
        0x0000_0073,
    ]);
    let mut state = GuestMachineState::new(memory.entry_address());
    let mut handler = RecordingFcallHandler::default();

    let report = run_guest_machine_with_fcalls(&mut memory, &mut state, &mut handler, 10)
        .expect("guest should halt");

    assert_eq!(report.executed_instructions, 4);
    assert_eq!(
        report.halt,
        GuestMachineHalt::Ecall {
            address: ENTRY + 16
        }
    );
    assert_eq!(state.register(6), Some(0x2a));
    assert_eq!(
        handler.requests,
        vec![GuestFcallRequest {
            function_id: 7,
            params: vec![GuestFcallParam { port: 0, value: 17 }],
        }]
    );
}

#[test]
fn traces_guest_machine_with_zisk_free_call_handler() {
    let mut memory = guest_machine_memory_with_words(&[
        addi(5, 0, 17),
        csrs(0x08f0, 5),
        csrwi(0x08c0, 7),
        csrr(6, 0x0ffe),
        0x0000_0073,
    ]);
    let mut state = GuestMachineState::new(memory.entry_address());
    let mut handler = RecordingFcallHandler::default();

    let trace = run_guest_machine_trace_with_fcalls(&mut memory, &mut state, &mut handler, 10)
        .expect("guest should halt");

    assert_eq!(trace.run.executed_instructions, 4);
    assert_eq!(
        trace.run.halt,
        GuestMachineHalt::Ecall {
            address: ENTRY + 16
        }
    );
    assert_eq!(
        trace
            .reports
            .iter()
            .map(|report| (report.address, report.next_pc))
            .collect::<Vec<_>>(),
        vec![
            (ENTRY, ENTRY + 4),
            (ENTRY + 4, ENTRY + 8),
            (ENTRY + 8, ENTRY + 12),
            (ENTRY + 12, ENTRY + 16),
        ]
    );
    assert_eq!(state.register(6), Some(0x2a));
    assert_eq!(handler.requests.len(), 1);
}

#[test]
fn zisk_dma_inputcpy_copies_free_call_results_to_guest_memory() {
    let data_offset = 256;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[
            auipc(10, 0),
            addi(10, 10, data_offset as i16),
            addi(10, 10, 3),
            csrwi(0x08c0, 7),
            csrs(0x0815, 10),
            addi(11, 10, 16),
            0x0000_0073,
        ],
        data_offset,
        &[0; 24],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    let mut handler =
        RecordingFcallHandler::with_results(vec![0x1122_3344_5566_7788, 0x99aa_bbcc_ddee_ff00]);

    let report = run_guest_machine_with_fcalls(&mut memory, &mut state, &mut handler, 16)
        .expect("guest should halt");

    let mut copied = [0_u8; 24];
    memory
        .read_range_into(data_address, &mut copied)
        .expect("data memory should read");
    assert_eq!(report.executed_instructions, 6);
    assert_eq!(
        report.halt,
        GuestMachineHalt::Ecall {
            address: ENTRY + 24
        }
    );
    assert_eq!(state.register(11), Some(data_address + 3));
    assert_eq!(
        copied,
        [
            0x00, 0x00, 0x00, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00, 0xff, 0xee,
            0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    );
    assert_eq!(
        handler.requests,
        vec![GuestFcallRequest {
            function_id: 7,
            params: vec![],
        }]
    );
}

#[test]
fn guest_machine_reads_zisk_architecture_id() {
    let mut memory = guest_machine_memory_with_words(&[csrr(5, 0x0f12), 0x0000_0073]);
    let mut state = GuestMachineState::new(memory.entry_address());

    let report = run_guest_machine(&mut memory, &mut state, 4).expect("guest should halt");

    assert_eq!(report.executed_instructions, 1);
    assert_eq!(state.register(5), Some(ZISK_ARCHITECTURE_ID));
}

fn framed_stdin_chunk(payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes.resize(bytes.len().next_multiple_of(8), 0);
    bytes
}

#[test]
fn zisk_input_ready_maps_framed_stdin_into_guest_memory() {
    let input = framed_stdin_chunk(b"abc");
    let mut memory = guest_machine_memory_with_words(&[
        lui(5, ZISK_INPUT_ADDRESS as u32),
        addi(6, 5, 15),
        csrs(0x08f0, 6),
        csrwi(0x08c0, ZISK_INPUT_READY_FCALL_ID as u8),
        ld(7, 5, 8),
        addi(6, 5, 23),
        csrs(0x08f0, 6),
        csrwi(0x08c0, ZISK_INPUT_READY_FCALL_ID as u8),
        lbu(8, 5, 16),
        lbu(9, 5, 18),
        0x0000_0073,
    ]);
    let mut state = GuestMachineState::new(memory.entry_address());
    let mut handler = ZiskInputFcallHandler::new(&input).expect("framed stdin should load");

    let report = run_guest_machine_with_fcalls(&mut memory, &mut state, &mut handler, 32)
        .expect("guest should halt");

    assert_eq!(report.executed_instructions, 10);
    assert_eq!(state.register(7), Some(3));
    assert_eq!(state.register(8), Some(u64::from(b'a')));
    assert_eq!(state.register(9), Some(u64::from(b'c')));
}

#[test]
fn zisk_input_ready_writes_into_existing_guest_memory() {
    let input = framed_stdin_chunk(b"abc");
    let mut memory = guest_machine_memory_with_words(&[
        lui(5, ZISK_INPUT_ADDRESS as u32),
        addi(6, 5, 15),
        csrs(0x08f0, 6),
        csrwi(0x08c0, ZISK_INPUT_READY_FCALL_ID as u8),
        ld(7, 5, 8),
        lbu(8, 5, 16),
        lbu(9, 5, 18),
        0x0000_0073,
    ]);
    memory
        .map_initialized_range(ZISK_INPUT_ADDRESS, vec![0; 32])
        .expect("reserved input memory should map");
    let mut state = GuestMachineState::new(memory.entry_address());
    let mut handler = ZiskInputFcallHandler::new(&input).expect("framed stdin should load");

    let report = run_guest_machine_with_fcalls(&mut memory, &mut state, &mut handler, 32)
        .expect("guest should halt");

    assert_eq!(report.executed_instructions, 7);
    assert_eq!(state.register(7), Some(3));
    assert_eq!(state.register(8), Some(u64::from(b'a')));
    assert_eq!(state.register(9), Some(u64::from(b'c')));
}

#[test]
fn zisk_fcall_secp256k1_ecdsa_verify_returns_double_scalar_mul_point() {
    let data_offset = 256;
    let mut data = Vec::new();
    push_u64_array(
        &mut data,
        &[
            0x59f2_815b_16f8_1798,
            0x029b_fcdb_2dce_28d9,
            0x55a0_6295_ce87_0b07,
            0x79be_667e_f9dc_bbac,
            0x9c47_d08f_fb10_d4b8,
            0xfd17_b448_a685_5419,
            0x5da4_fbfc_0e11_08a8,
            0x483a_da77_26a3_c465,
        ],
    );
    push_u64_array(&mut data, &[2, 0, 0, 0]);
    push_u64_array(&mut data, &[3, 0, 0, 0]);
    push_u64_array(&mut data, &[1, 0, 0, 0]);
    let mut memory = guest_machine_memory_with_words_and_data(
        &[
            auipc(10, 0),
            addi(10, 10, data_offset as i16),
            csrs(0x08f3, 10),
            addi(10, 10, 64),
            csrs(0x08f2, 10),
            addi(10, 10, 32),
            csrs(0x08f2, 10),
            addi(10, 10, 32),
            csrs(0x08f2, 10),
            csrwi(0x08c0, ZISK_SECP256K1_ECDSA_VERIFY_FCALL_ID as u8),
            csrr(5, 0x0ffe),
            csrr(6, 0x0ffe),
            csrr(7, 0x0ffe),
            csrr(8, 0x0ffe),
            csrr(9, 0x0ffe),
            csrr(10, 0x0ffe),
            csrr(11, 0x0ffe),
            csrr(12, 0x0ffe),
            0x0000_0073,
        ],
        data_offset,
        &data,
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    let mut handler = ZiskInputFcallHandler::new(&[]).expect("empty input should load");

    let report = run_guest_machine_with_fcalls(&mut memory, &mut state, &mut handler, 64)
        .expect("guest should halt");

    assert_eq!(report.executed_instructions, 18);
    assert_eq!(state.register(5), Some(0xcba8_d569_b240_efe4));
    assert_eq!(state.register(6), Some(0xe88b_84bd_dc61_9ab7));
    assert_eq!(state.register(7), Some(0x55b4_a725_0a5c_5128));
    assert_eq!(state.register(8), Some(0x2f8b_de4d_1a07_2093));
    assert_eq!(state.register(9), Some(0xdca8_7d3a_a6ac_62d6));
    assert_eq!(state.register(10), Some(0xf788_271b_ab0d_6840));
    assert_eq!(state.register(11), Some(0xd4db_a9dd_a6c9_c426));
    assert_eq!(state.register(12), Some(0xd8ac_2226_36e5_e3d6));
}

#[test]
fn zisk_fcall_msb_pos_256_returns_highest_limb_and_bit() {
    let data_offset = 256;
    let data_address = ENTRY + data_offset as u64;
    let x_address = data_address;
    let y_address = x_address + 32;
    let z_address = y_address + 32;
    let mut data = Vec::new();
    push_u64_array(&mut data, &[0, 0, 0, 0]);
    push_u64_array(&mut data, &[0, 1 << 9, 0, 0]);
    push_u64_array(&mut data, &[0, 0, 0x400, 0]);
    let mut memory = guest_machine_memory_with_words_and_data(
        &[
            auipc(10, 0),
            addi(10, 10, data_offset as i16),
            addi(11, 0, 3),
            csrs(0x08f0, 11),
            csrs(0x08f2, 10),
            addi(10, 10, 32),
            csrs(0x08f2, 10),
            addi(10, 10, 32),
            csrs(0x08f2, 10),
            csrwi(0x08c0, ZISK_MSB_POS_256_FCALL_ID as u8),
            csrr(5, 0x0ffe),
            csrr(6, 0x0ffe),
            0x0000_0073,
        ],
        data_offset,
        &data,
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    let mut handler = ZiskInputFcallHandler::new(&[]).expect("empty input should load");

    let report = run_guest_machine_with_fcalls(&mut memory, &mut state, &mut handler, 32)
        .expect("guest should halt");

    assert_eq!(report.executed_instructions, 12);
    assert_eq!(state.register(5), Some(2));
    assert_eq!(state.register(6), Some(10));
    assert_eq!(
        (x_address, y_address, z_address),
        (data_address, data_address + 32, data_address + 64)
    );
}

#[test]
fn zisk_input_ready_keeps_input_after_map_failure() {
    let input = framed_stdin_chunk(b"abc");
    let mut handler = ZiskInputFcallHandler::new(&input).expect("framed stdin should load");
    let request = GuestFcallRequest {
        function_id: ZISK_INPUT_READY_FCALL_ID,
        params: vec![GuestFcallParam {
            port: 0,
            value: ZISK_INPUT_ADDRESS + 15,
        }],
    };
    let mut overlapping_memory = guest_machine_memory_with_words(&[0x0000_0073]);
    overlapping_memory
        .map_initialized_range(ZISK_INPUT_ADDRESS, vec![0; 4])
        .expect("overlapping input memory should map");
    handler
        .handle_fcall(request.clone(), &mut overlapping_memory)
        .expect_err("short input reservation should reject");

    let mut memory = guest_machine_memory_with_words(&[0x0000_0073]);
    handler
        .handle_fcall(request, &mut memory)
        .expect("input-ready should retry mapping");
    let mut length_bytes = [0_u8; 8];
    memory
        .read_range_into(ZISK_INPUT_ADDRESS + 8, &mut length_bytes)
        .expect("framed stdin should map after retry");

    assert_eq!(u64::from_le_bytes(length_bytes), 3);
}

#[test]
fn zisk_input_ready_rejects_required_address_before_framed_stdin() {
    let input = framed_stdin_chunk(b"abc");
    let mut memory = guest_machine_memory_with_words(&[0x0000_0073]);
    let mut handler = ZiskInputFcallHandler::new(&input).expect("framed stdin should load");

    let error = handler
        .handle_fcall(
            GuestFcallRequest {
                function_id: ZISK_INPUT_READY_FCALL_ID,
                params: vec![GuestFcallParam {
                    port: 0,
                    value: ZISK_INPUT_ADDRESS + 6,
                }],
            },
            &mut memory,
        )
        .expect_err("prefix underrun should reject");

    assert_eq!(
        error.to_string(),
        "guest free-call handler failed: Zisk input-ready required address 0x40000006 is before framed stdin"
    );
}

#[test]
fn zisk_input_ready_rejects_required_address_after_framed_stdin() {
    let input = framed_stdin_chunk(b"abc");
    let mut memory = guest_machine_memory_with_words(&[0x0000_0073]);
    let mut handler = ZiskInputFcallHandler::new(&input).expect("framed stdin should load");

    let error = handler
        .handle_fcall(
            GuestFcallRequest {
                function_id: ZISK_INPUT_READY_FCALL_ID,
                params: vec![GuestFcallParam {
                    port: 0,
                    value: ZISK_INPUT_ADDRESS + 8 + input.len() as u64,
                }],
            },
            &mut memory,
        )
        .expect_err("input overrun should reject");

    assert!(error
        .to_string()
        .contains("is outside available input ending at 0x40000018"));
}

#[test]
fn zisk_input_handler_rejects_malformed_framed_stdin() {
    let error =
        ZiskInputFcallHandler::new(&[1, 2, 3]).expect_err("malformed framed stdin should reject");

    assert_eq!(
        error.to_string(),
        "framed stdin is invalid: truncated chunk length at offset 0: expected 8 bytes, found 3"
    );
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
fn runs_compressed_addi4spn_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_addi4spn(8, 4));
    push_halfword(&mut code, compressed_addi4spn(15, 1020));
    push_word(&mut code, 0x0000_0073);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state.set_register(2, 0x1000).expect("register should set");

    let report = run_guest_machine(&mut memory, &mut state, 8).expect("guest should halt");

    assert_eq!(report.executed_instructions, 2);
    assert_eq!(report.halt, GuestMachineHalt::Ecall { address: ENTRY + 4 });
    assert_eq!(state.pc(), ENTRY + 4);
    assert_eq!(state.register(8), Some(0x1004));
    assert_eq!(state.register(15), Some(0x13fc));
}

#[test]
fn runs_compressed_addiw_and_slli_instructions_until_ecall() {
    let mut code = Vec::new();
    push_halfword(&mut code, compressed_addiw(5, 1));
    push_halfword(&mut code, compressed_addiw(6, -1));
    push_halfword(&mut code, compressed_addiw(7, 0));
    push_halfword(&mut code, compressed_slli(8, 1));
    push_halfword(&mut code, compressed_slli(9, 63));
    push_word(&mut code, 0x0000_0073);
    let mut memory = guest_machine_memory_with_bytes(&code);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(5, 0x0000_0000_7fff_ffff)
        .expect("register should set");
    state.set_register(6, 0).expect("register should set");
    state
        .set_register(7, 0x0000_0000_ffff_8001)
        .expect("register should set");
    state.set_register(8, 0x8000).expect("register should set");
    state.set_register(9, 1).expect("register should set");

    let report = run_guest_machine(&mut memory, &mut state, 8).expect("guest should halt");

    assert_eq!(report.executed_instructions, 5);
    assert_eq!(
        report.halt,
        GuestMachineHalt::Ecall {
            address: ENTRY + 10
        }
    );
    assert_eq!(state.pc(), ENTRY + 10);
    assert_eq!(state.register(5), Some(0xffff_ffff_8000_0000));
    assert_eq!(state.register(6), Some(u64::MAX));
    assert_eq!(state.register(7), Some(0xffff_ffff_ffff_8001));
    assert_eq!(state.register(8), Some(0x1_0000));
    assert_eq!(state.register(9), Some(0x8000_0000_0000_0000));
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

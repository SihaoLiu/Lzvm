use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_prover::guest_instruction::{
    RiscvAmoKind, RiscvAmoWidth, RiscvBranchKind, RiscvFenceKind, RiscvInstruction, RiscvLoadKind,
    RiscvOp32Kind, RiscvOpImm32Kind, RiscvOpImmKind, RiscvOpKind, RiscvStoreKind,
};
use lzvm_prover::guest_machine::{
    advance_guest_machine, GuestMachineError, GuestMachineMemory, GuestMachineState,
};
use lzvm_prover::guest_memory::{load_guest_memory_image, GuestMemoryError, GuestMemoryImage};

const ENTRY: u64 = 0x8000_0000;
const FIRST_REGISTER: usize = 1;

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

fn guest_memory_image_with_bytes(code: &[u8]) -> GuestMemoryImage {
    guest_memory_image_with_segment_memory_size(code, code.len() as u64)
}

fn guest_memory_image_with_segment_memory_size(code: &[u8], memory_size: u64) -> GuestMemoryImage {
    assert!(code.len() as u64 <= memory_size);
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: ENTRY,
        physical_address: ENTRY,
        file_size: code.len() as u64,
        memory_size,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(code);
    let info = parse_guest_image(&image).expect("guest image should parse");
    load_guest_memory_image(&image, &info).expect("guest memory should load")
}

fn guest_machine_memory_with_bytes(code: &[u8]) -> GuestMachineMemory {
    GuestMachineMemory::from_image(&guest_memory_image_with_bytes(code))
}

fn guest_machine_memory_with_words(words: &[u32]) -> GuestMachineMemory {
    let mut code = Vec::with_capacity(words.len() * 4);
    for word in words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    guest_machine_memory_with_bytes(&code)
}

fn guest_machine_memory_with_words_and_memory_size(
    words: &[u32],
    memory_size: u64,
) -> GuestMachineMemory {
    let mut code = Vec::with_capacity(words.len() * 4);
    for word in words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    GuestMachineMemory::from_image(&guest_memory_image_with_segment_memory_size(
        &code,
        memory_size,
    ))
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

fn encode_i(immediate: i16, rs1: u8, funct3: u8, rd: u8, opcode: u8) -> u32 {
    assert!((-2048..=2047).contains(&immediate));
    assert_register(rs1);
    assert_funct3(funct3);
    assert_register(rd);
    assert_opcode(opcode);
    (((immediate as i32 as u32) & 0x0fff) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | u32::from(opcode)
}

fn encode_r(funct7: u8, rs2: u8, rs1: u8, funct3: u8, rd: u8) -> u32 {
    encode_r_with_opcode(funct7, rs2, rs1, funct3, rd, 0x33)
}

fn encode_r_with_opcode(funct7: u8, rs2: u8, rs1: u8, funct3: u8, rd: u8, opcode: u8) -> u32 {
    assert!(funct7 < 128);
    assert_register(rs2);
    assert_register(rs1);
    assert_funct3(funct3);
    assert_register(rd);
    assert_opcode(opcode);
    (u32::from(funct7) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | u32::from(opcode)
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
    assert_register(rs2);
    assert_register(rs1);
    assert_funct3(funct3);
    assert_register(rd);
    (u32::from(funct5) << 27)
        | (u32::from(acquire) << 26)
        | (u32::from(release) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | 0x2f
}

fn assert_register(register: u8) {
    assert!(register < 32);
}

fn assert_funct3(funct3: u8) {
    assert!(funct3 < 8);
}

fn assert_opcode(opcode: u8) {
    assert!(opcode < 128);
}

fn addi(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 0, rd, 0x13)
}

fn slti(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 2, rd, 0x13)
}

fn sltiu(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 3, rd, 0x13)
}

fn xori(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 4, rd, 0x13)
}

fn ori(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 6, rd, 0x13)
}

fn andi(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 7, rd, 0x13)
}

fn slli(rd: u8, rs1: u8, shamt: u8) -> u32 {
    assert!(shamt < 64);
    encode_i(i16::from(shamt), rs1, 1, rd, 0x13)
}

fn srli(rd: u8, rs1: u8, shamt: u8) -> u32 {
    assert!(shamt < 64);
    encode_i(i16::from(shamt), rs1, 5, rd, 0x13)
}

fn srai(rd: u8, rs1: u8, shamt: u8) -> u32 {
    assert!(shamt < 64);
    encode_i(0x400 | i16::from(shamt), rs1, 5, rd, 0x13)
}

fn addiw(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 0, rd, 0x1b)
}

fn slliw(rd: u8, rs1: u8, shamt: u8) -> u32 {
    assert!(shamt < 32);
    encode_i(i16::from(shamt), rs1, 1, rd, 0x1b)
}

fn srliw(rd: u8, rs1: u8, shamt: u8) -> u32 {
    assert!(shamt < 32);
    encode_i(i16::from(shamt), rs1, 5, rd, 0x1b)
}

fn sraiw(rd: u8, rs1: u8, shamt: u8) -> u32 {
    assert!(shamt < 32);
    encode_i(0x400 | i16::from(shamt), rs1, 5, rd, 0x1b)
}

fn reserved_slliw_with_imm5(rd: u8, rs1: u8) -> u32 {
    encode_i(0x020, rs1, 1, rd, 0x1b)
}

fn reserved_srliw_with_imm5(rd: u8, rs1: u8) -> u32 {
    encode_i(0x020, rs1, 5, rd, 0x1b)
}

fn reserved_sraiw_with_imm5(rd: u8, rs1: u8) -> u32 {
    encode_i(0x420, rs1, 5, rd, 0x1b)
}

fn jalr(rd: u8, rs1: u8, offset: i16) -> u32 {
    encode_i(offset, rs1, 0, rd, 0x67)
}

fn load(funct3: u8, rd: u8, rs1: u8, offset: i16) -> u32 {
    encode_i(offset, rs1, funct3, rd, 0x03)
}

fn lb(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(0, rd, rs1, offset)
}

fn lh(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(1, rd, rs1, offset)
}

fn lw(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(2, rd, rs1, offset)
}

fn ld(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(3, rd, rs1, offset)
}

fn lbu(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(4, rd, rs1, offset)
}

fn lhu(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(5, rd, rs1, offset)
}

fn lwu(rd: u8, rs1: u8, offset: i16) -> u32 {
    load(6, rd, rs1, offset)
}

fn store(funct3: u8, rs1: u8, rs2: u8, offset: i16) -> u32 {
    assert_funct3(funct3);
    assert_register(rs1);
    assert_register(rs2);
    assert!((-2048..=2047).contains(&offset));
    let offset = offset as i32 as u32;
    (((offset >> 5) & 0x7f) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | ((offset & 0x1f) << 7)
        | 0x23
}

fn sb(rs1: u8, rs2: u8, offset: i16) -> u32 {
    store(0, rs1, rs2, offset)
}

fn sh(rs1: u8, rs2: u8, offset: i16) -> u32 {
    store(1, rs1, rs2, offset)
}

fn sw(rs1: u8, rs2: u8, offset: i16) -> u32 {
    store(2, rs1, rs2, offset)
}

fn sd(rs1: u8, rs2: u8, offset: i16) -> u32 {
    store(3, rs1, rs2, offset)
}

fn lui(rd: u8, immediate: u32) -> u32 {
    assert_register(rd);
    assert_eq!(immediate & 0x0fff, 0);
    (immediate & 0xffff_f000) | (u32::from(rd) << 7) | 0x37
}

fn auipc(rd: u8, immediate: u32) -> u32 {
    assert_register(rd);
    assert_eq!(immediate & 0x0fff, 0);
    (immediate & 0xffff_f000) | (u32::from(rd) << 7) | 0x17
}

fn add(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x00, rs2, rs1, 0, rd)
}

fn sub(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x20, rs2, rs1, 0, rd)
}

fn sll(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x00, rs2, rs1, 1, rd)
}

fn slt(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x00, rs2, rs1, 2, rd)
}

fn sltu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x00, rs2, rs1, 3, rd)
}

fn xor(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x00, rs2, rs1, 4, rd)
}

fn srl(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x00, rs2, rs1, 5, rd)
}

fn sra(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x20, rs2, rs1, 5, rd)
}

fn or(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x00, rs2, rs1, 6, rd)
}

fn and(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x00, rs2, rs1, 7, rd)
}

fn mul(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x01, rs2, rs1, 0, rd)
}

fn mulh(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x01, rs2, rs1, 1, rd)
}

fn mulhsu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x01, rs2, rs1, 2, rd)
}

fn mulhu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x01, rs2, rs1, 3, rd)
}

fn div(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x01, rs2, rs1, 4, rd)
}

fn divu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x01, rs2, rs1, 5, rd)
}

fn rem(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x01, rs2, rs1, 6, rd)
}

fn remu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x01, rs2, rs1, 7, rd)
}

fn addw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x00, rs2, rs1, 0, rd, 0x3b)
}

fn subw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x20, rs2, rs1, 0, rd, 0x3b)
}

fn sllw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x00, rs2, rs1, 1, rd, 0x3b)
}

fn srlw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x00, rs2, rs1, 5, rd, 0x3b)
}

fn sraw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x20, rs2, rs1, 5, rd, 0x3b)
}

fn mulw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x01, rs2, rs1, 0, rd, 0x3b)
}

fn divw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x01, rs2, rs1, 4, rd, 0x3b)
}

fn divuw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x01, rs2, rs1, 5, rd, 0x3b)
}

fn remw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x01, rs2, rs1, 6, rd, 0x3b)
}

fn remuw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0x01, rs2, rs1, 7, rd, 0x3b)
}

fn amoadd_w(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x00, false, false, rs2, rs1, 2, rd)
}

fn amoadd_d_aqrl(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x00, true, true, rs2, rs1, 3, rd)
}

fn amoswap_d(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x01, false, false, rs2, rs1, 3, rd)
}

fn amoxor_w(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x04, false, false, rs2, rs1, 2, rd)
}

fn amoor_d_rl(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x08, false, true, rs2, rs1, 3, rd)
}

fn amoand_w_aq(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x0c, true, false, rs2, rs1, 2, rd)
}

fn amomin_w(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x10, false, false, rs2, rs1, 2, rd)
}

fn amomax_d(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x14, false, false, rs2, rs1, 3, rd)
}

fn amominu_w_rl(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x18, false, true, rs2, rs1, 2, rd)
}

fn amomaxu_d_aq(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x1c, true, false, rs2, rs1, 3, rd)
}

fn branch(funct3: u8, rs1: u8, rs2: u8, offset: i16) -> u32 {
    assert_funct3(funct3);
    assert_register(rs1);
    assert_register(rs2);
    assert!(offset % 2 == 0);
    assert!((-4096..=4094).contains(&offset));
    let offset = offset as i32 as u32;
    (((offset >> 12) & 0x01) << 31)
        | (((offset >> 5) & 0x3f) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (((offset >> 1) & 0x0f) << 8)
        | (((offset >> 11) & 0x01) << 7)
        | 0x63
}

fn beq(rs1: u8, rs2: u8, offset: i16) -> u32 {
    branch(0, rs1, rs2, offset)
}

fn bne(rs1: u8, rs2: u8, offset: i16) -> u32 {
    branch(1, rs1, rs2, offset)
}

fn blt(rs1: u8, rs2: u8, offset: i16) -> u32 {
    branch(4, rs1, rs2, offset)
}

fn bge(rs1: u8, rs2: u8, offset: i16) -> u32 {
    branch(5, rs1, rs2, offset)
}

fn bltu(rs1: u8, rs2: u8, offset: i16) -> u32 {
    branch(6, rs1, rs2, offset)
}

fn bgeu(rs1: u8, rs2: u8, offset: i16) -> u32 {
    branch(7, rs1, rs2, offset)
}

fn jal(rd: u8, offset: i32) -> u32 {
    assert_register(rd);
    assert!(offset % 2 == 0);
    assert!((-1_048_576..=1_048_574).contains(&offset));
    let offset = offset as u32;
    (((offset >> 20) & 0x01) << 31)
        | (((offset >> 1) & 0x03ff) << 21)
        | (((offset >> 11) & 0x01) << 20)
        | (((offset >> 12) & 0xff) << 12)
        | (u32::from(rd) << 7)
        | 0x6f
}

fn execute_single_word_with_registers(word: u32, registers: &[(usize, u64)]) -> GuestMachineState {
    let mut memory = guest_machine_memory_with_words(&[word]);
    let mut state = GuestMachineState::new(memory.entry_address());
    for &(index, value) in registers {
        state
            .set_register(index, value)
            .expect("register write should be valid");
    }
    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    state
}

#[test]
fn advances_integer_instructions_and_preserves_zero_register() {
    let mut memory = guest_machine_memory_with_words(&[
        addi(1, 0, 7),
        addi(0, 1, 9),
        lui(2, 0x1234_5000),
        auipc(3, 0x1000),
        addi(4, 0, 10),
        addi(5, 0, 3),
        add(6, 4, 5),
        sub(7, 4, 5),
    ]);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(0, u64::MAX)
        .expect("register write should be valid");

    let report =
        advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    assert_eq!(report.address, ENTRY);
    assert_eq!(report.next_pc, ENTRY + 4);
    assert_eq!(
        report.instruction,
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 0,
            immediate: 7,
        }
    );
    assert_eq!(state.pc(), ENTRY + 4);
    assert_eq!(state.register(0), Some(0));
    assert_eq!(state.register(1), Some(7));

    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    assert_eq!(state.register(0), Some(0));

    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    assert_eq!(state.register(2), Some(0x1234_5000));

    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    assert_eq!(state.register(3), Some(ENTRY + 12 + 0x1000));

    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    assert_eq!(state.register(6), Some(13));

    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    assert_eq!(state.pc(), ENTRY + 32);
    assert_eq!(state.register(7), Some(7));
}

#[test]
fn advances_branch_and_jump_instructions() {
    let mut memory = guest_machine_memory_with_words(&[
        addi(1, 0, 5),
        addi(2, 0, 5),
        beq(1, 2, 8),
        addi(3, 0, 111),
        jal(4, 8),
        addi(3, 0, 222),
        jalr(6, 7, 0),
        addi(5, 0, 33),
    ]);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(7, ENTRY + 29)
        .expect("register write should be valid");

    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");

    let branch = advance_guest_machine(&mut memory, &mut state).expect("branch should execute");
    assert_eq!(
        branch.instruction,
        RiscvInstruction::Branch {
            kind: RiscvBranchKind::Beq,
            rs1: 1,
            rs2: 2,
            offset: 8,
        }
    );
    assert_eq!(state.pc(), ENTRY + 16);
    assert_eq!(state.register(3), Some(0));

    let jump = advance_guest_machine(&mut memory, &mut state).expect("jump should execute");
    assert_eq!(jump.instruction, RiscvInstruction::Jal { rd: 4, offset: 8 });
    assert_eq!(state.register(4), Some(ENTRY + 20));
    assert_eq!(state.pc(), ENTRY + 24);

    advance_guest_machine(&mut memory, &mut state).expect("jump register should execute");
    assert_eq!(state.register(6), Some(ENTRY + 28));
    assert_eq!(state.pc(), ENTRY + 28);

    advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    assert_eq!(state.register(5), Some(33));
}

#[test]
fn advances_immediate_shift_and_compare_instructions() {
    let mut memory = guest_machine_memory_with_words(&[
        addi(1, 0, -1),
        slti(2, 1, 0),
        sltiu(3, 1, 0),
        xori(4, 1, -1),
        ori(5, 0, -2048),
        andi(6, 1, 2047),
        slli(7, 6, 1),
        srli(8, 1, 63),
        srai(9, 1, 63),
    ]);
    let mut state = GuestMachineState::new(memory.entry_address());

    for _ in 0..9 {
        advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");
    }

    assert_eq!(state.register(1), Some(u64::MAX));
    assert_eq!(state.register(2), Some(1));
    assert_eq!(state.register(3), Some(0));
    assert_eq!(state.register(4), Some(0));
    assert_eq!(state.register(5), Some((-2048_i64) as u64));
    assert_eq!(state.register(6), Some(2047));
    assert_eq!(state.register(7), Some(4094));
    assert_eq!(state.register(8), Some(1));
    assert_eq!(state.register(9), Some(u64::MAX));
}

#[test]
fn advances_guest_memory_load_instructions() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[
            lb(2, 1, 0),
            lbu(3, 1, 0),
            lh(4, 1, 2),
            lhu(5, 1, 2),
            lw(6, 1, 4),
            lwu(7, 1, 4),
            ld(8, 1, 8),
        ],
        data_offset,
        &[
            0x80, 0x00, 0x34, 0x80, 0x78, 0x56, 0x34, 0x80, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
            0x23, 0x01,
        ],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, data_address)
        .expect("register write should be valid");

    let first = advance_guest_machine(&mut memory, &mut state).expect("load should execute");
    assert_eq!(
        first.instruction,
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Lb,
            rd: 2,
            rs1: 1,
            offset: 0,
        }
    );
    for _ in 0..6 {
        advance_guest_machine(&mut memory, &mut state).expect("load should execute");
    }

    assert_eq!(state.register(2), Some((-128_i64) as u64));
    assert_eq!(state.register(3), Some(0x80));
    assert_eq!(state.register(4), Some((0x8034_u16 as i16 as i64) as u64));
    assert_eq!(state.register(5), Some(0x8034));
    assert_eq!(
        state.register(6),
        Some((0x8034_5678_u32 as i32 as i64) as u64)
    );
    assert_eq!(state.register(7), Some(0x8034_5678));
    assert_eq!(state.register(8), Some(0x0123_4567_89ab_cdef));
    assert_eq!(state.pc(), ENTRY + 28);
}

#[test]
fn advances_guest_memory_store_instructions() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[sb(1, 2, 0), sh(1, 2, 2), sw(1, 2, 4), sd(1, 2, 8)],
        data_offset,
        &[0; 16],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, data_address)
        .expect("register write should be valid");
    state
        .set_register(2, 0x0123_4567_89ab_cdef)
        .expect("register write should be valid");

    let first = advance_guest_machine(&mut memory, &mut state).expect("store should execute");
    assert_eq!(
        first.instruction,
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sb,
            rs1: 1,
            rs2: 2,
            offset: 0,
        }
    );
    for _ in 0..3 {
        advance_guest_machine(&mut memory, &mut state).expect("store should execute");
    }

    let mut stored = [0_u8; 16];
    memory
        .read_range_into(data_address, &mut stored)
        .expect("stored bytes should read");
    assert_eq!(
        stored,
        [
            0xef, 0x00, 0xef, 0xcd, 0xef, 0xcd, 0xab, 0x89, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
            0x23, 0x01,
        ]
    );
    assert_eq!(state.register(2), Some(0x0123_4567_89ab_cdef));
    assert_eq!(state.pc(), ENTRY + 16);
}

#[test]
fn advances_atomic_add_instructions() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[amoadd_w(3, 1, 2), amoadd_d_aqrl(4, 1, 5)],
        data_offset,
        &[
            0xff, 0xff, 0xff, 0x7f, 0, 0, 0, 0, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01,
        ],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, data_address)
        .expect("register write should be valid");
    state
        .set_register(2, 2)
        .expect("register write should be valid");
    state
        .set_register(5, 1)
        .expect("register write should be valid");

    let first =
        advance_guest_machine(&mut memory, &mut state).expect("word atomic add should execute");
    assert_eq!(
        first.instruction,
        RiscvInstruction::Amo {
            kind: RiscvAmoKind::Add,
            width: RiscvAmoWidth::Word,
            rd: 3,
            rs1: 1,
            rs2: 2,
            acquire: false,
            release: false,
        }
    );
    state
        .set_register(1, data_address + 8)
        .expect("register write should be valid");
    let second = advance_guest_machine(&mut memory, &mut state)
        .expect("doubleword atomic add should execute");
    assert_eq!(
        second.instruction,
        RiscvInstruction::Amo {
            kind: RiscvAmoKind::Add,
            width: RiscvAmoWidth::Doubleword,
            rd: 4,
            rs1: 1,
            rs2: 5,
            acquire: true,
            release: true,
        }
    );

    let mut stored = [0_u8; 16];
    memory
        .read_range_into(data_address, &mut stored)
        .expect("stored bytes should read");
    assert_eq!(
        stored,
        [0x01, 0x00, 0x00, 0x80, 0, 0, 0, 0, 0xf0, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01,]
    );
    assert_eq!(state.register(3), Some(0x7fff_ffff));
    assert_eq!(state.register(4), Some(0x0123_4567_89ab_cdef));
    assert_eq!(state.pc(), ENTRY + 8);
}

#[test]
fn advances_atomic_swap_and_logical_instructions() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[
            amoswap_d(3, 1, 2),
            amoxor_w(4, 1, 5),
            amoor_d_rl(6, 1, 7),
            amoand_w_aq(8, 1, 9),
        ],
        data_offset,
        &[
            0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0xff, 0xff, 0xaa, 0xbb,
            0xcc, 0xdd, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x0f, 0x0f, 0xf0, 0xf0,
            0xaa, 0xbb, 0xcc, 0xdd,
        ],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, data_address)
        .expect("register write should be valid");
    state
        .set_register(2, 0xaabb_ccdd_eeff_0011)
        .expect("register write should be valid");
    state
        .set_register(5, 0x00ff_00ff)
        .expect("register write should be valid");
    state
        .set_register(7, 0xff00_0000_ff00_0000)
        .expect("register write should be valid");
    state
        .set_register(9, 0x0ff0_ffff)
        .expect("register write should be valid");

    advance_guest_machine(&mut memory, &mut state).expect("swap should execute");
    state
        .set_register(1, data_address + 8)
        .expect("register write should be valid");
    advance_guest_machine(&mut memory, &mut state).expect("xor should execute");
    state
        .set_register(1, data_address + 16)
        .expect("register write should be valid");
    advance_guest_machine(&mut memory, &mut state).expect("or should execute");
    state
        .set_register(1, data_address + 24)
        .expect("register write should be valid");
    advance_guest_machine(&mut memory, &mut state).expect("and should execute");

    let mut stored = [0_u8; 32];
    memory
        .read_range_into(data_address, &mut stored)
        .expect("stored bytes should read");
    assert_eq!(
        stored,
        [
            0x11, 0x00, 0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0xff, 0x00, 0x00, 0xff, 0xaa, 0xbb,
            0xcc, 0xdd, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x0f, 0x0f, 0xf0, 0x00,
            0xaa, 0xbb, 0xcc, 0xdd,
        ]
    );
    assert_eq!(state.register(3), Some(0x0123_4567_89ab_cdef));
    assert_eq!(state.register(4), Some(0xffff_ffff_ffff_0000));
    assert_eq!(state.register(6), Some(0x0000_ff00_0000_ff00));
    assert_eq!(state.register(8), Some(0xffff_ffff_f0f0_0f0f));
    assert_eq!(state.pc(), ENTRY + 16);
}

#[test]
fn advances_atomic_min_max_instructions() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[
            amomin_w(3, 1, 2),
            amomax_d(4, 1, 5),
            amominu_w_rl(6, 1, 7),
            amomaxu_d_aq(8, 1, 9),
        ],
        data_offset,
        &[
            0x05, 0x00, 0x00, 0x00, 0xaa, 0xbb, 0xcc, 0xdd, 0xfd, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0xaa, 0xbb, 0xcc, 0xdd, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, data_address)
        .expect("register write should be valid");
    state
        .set_register(2, (-7_i64) as u64)
        .expect("register write should be valid");
    state
        .set_register(5, 4)
        .expect("register write should be valid");
    state
        .set_register(7, 7)
        .expect("register write should be valid");
    state
        .set_register(9, u64::MAX - 1)
        .expect("register write should be valid");

    advance_guest_machine(&mut memory, &mut state).expect("signed word min should execute");
    state
        .set_register(1, data_address + 8)
        .expect("register write should be valid");
    advance_guest_machine(&mut memory, &mut state).expect("signed doubleword max should execute");
    state
        .set_register(1, data_address + 16)
        .expect("register write should be valid");
    advance_guest_machine(&mut memory, &mut state).expect("unsigned word min should execute");
    state
        .set_register(1, data_address + 24)
        .expect("register write should be valid");
    advance_guest_machine(&mut memory, &mut state).expect("unsigned doubleword max should execute");

    let mut stored = [0_u8; 32];
    memory
        .read_range_into(data_address, &mut stored)
        .expect("stored bytes should read");
    assert_eq!(
        stored,
        [
            0xf9, 0xff, 0xff, 0xff, 0xaa, 0xbb, 0xcc, 0xdd, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0xaa, 0xbb, 0xcc, 0xdd, 0xfe, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff,
        ]
    );
    assert_eq!(state.register(3), Some(5));
    assert_eq!(state.register(4), Some((-3_i64) as u64));
    assert_eq!(state.register(6), Some(0xffff_ffff_ffff_0000));
    assert_eq!(state.register(8), Some(1));
    assert_eq!(state.pc(), ENTRY + 16);
}

#[test]
fn loads_zero_filled_sparse_tail_and_writes_overlay_bytes() {
    let data_address = ENTRY + 0x1000;
    let mut memory = guest_machine_memory_with_words_and_memory_size(
        &[ld(2, 1, 0), sd(1, 3, 8), ld(4, 1, 8)],
        1_u64 << 32,
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, data_address)
        .expect("register write should be valid");
    state
        .set_register(3, 0xfeed_face_cafe_beef)
        .expect("register write should be valid");

    advance_guest_machine(&mut memory, &mut state).expect("load should execute");
    advance_guest_machine(&mut memory, &mut state).expect("store should execute");
    advance_guest_machine(&mut memory, &mut state).expect("load should execute");

    assert_eq!(state.register(2), Some(0));
    assert_eq!(state.register(4), Some(0xfeed_face_cafe_beef));
    let mut bytes = [0_u8; 16];
    memory
        .read_range_into(data_address, &mut bytes)
        .expect("sparse tail should read");
    assert_eq!(
        bytes,
        [0, 0, 0, 0, 0, 0, 0, 0, 0xef, 0xbe, 0xfe, 0xca, 0xce, 0xfa, 0xed, 0xfe,]
    );
}

#[test]
fn fetches_instruction_bytes_written_by_guest_store() {
    let replacement = addi(3, 0, 7);
    let mut memory = guest_machine_memory_with_words(&[sw(1, 2, 4), 0x0000_0073]);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, ENTRY)
        .expect("register write should be valid");
    state
        .set_register(2, u64::from(replacement))
        .expect("register write should be valid");

    advance_guest_machine(&mut memory, &mut state).expect("store should execute");
    let report =
        advance_guest_machine(&mut memory, &mut state).expect("written instruction should execute");

    assert_eq!(
        report.instruction,
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 3,
            rs1: 0,
            immediate: 7,
        }
    );
    assert_eq!(state.register(3), Some(7));
    assert_eq!(state.pc(), ENTRY + 8);
}

#[test]
fn rejects_data_memory_faults_without_mutating_machine() {
    let mut memory = guest_machine_memory_with_words(&[lb(2, 1, 0)]);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, 0x9000_0000)
        .expect("register write should be valid");
    let before_state = state.clone();
    let before_memory = memory.clone();

    assert!(matches!(
        advance_guest_machine(&mut memory, &mut state),
        Err(GuestMachineError::Memory(
            GuestMemoryError::AddressNotMapped {
                address: 0x9000_0000,
                byte_len: 1,
            }
        ))
    ));
    assert_eq!(state, before_state);
    assert_eq!(memory, before_memory);

    let mut memory = guest_machine_memory_with_words_and_memory_size(&[sd(1, 2, 0)], 8);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, ENTRY + 4)
        .expect("register write should be valid");
    state
        .set_register(2, u64::MAX)
        .expect("register write should be valid");
    let before_state = state.clone();
    let before_memory = memory.clone();

    assert!(matches!(
        advance_guest_machine(&mut memory, &mut state),
        Err(GuestMachineError::Memory(
            GuestMemoryError::AddressNotMapped {
                address,
                byte_len: 8,
            }
        )) if address == ENTRY + 4
    ));
    assert_eq!(state, before_state);
    assert_eq!(memory, before_memory);
}

#[test]
fn advances_register_shift_compare_and_bitwise_instructions() {
    let cases = [
        (sll(3, 1, 2), 1, 3, 8),
        (slt(3, 1, 2), u64::MAX, 1, 1),
        (sltu(3, 1, 2), 1, u64::MAX, 1),
        (xor(3, 1, 2), 0b1010, 0b1100, 0b0110),
        (srl(3, 1, 2), 0x8000_0000_0000_0000, 63, 1),
        (sra(3, 1, 2), 0x8000_0000_0000_0000, 63, u64::MAX),
        (or(3, 1, 2), 0b1010, 0b1100, 0b1110),
        (and(3, 1, 2), 0b1010, 0b1100, 0b1000),
    ];

    for (word, first, second, expected) in cases {
        let state =
            execute_single_word_with_registers(word, &[(FIRST_REGISTER, first), (2, second)]);
        assert_eq!(state.register(3), Some(expected));
        assert_eq!(state.pc(), ENTRY + 4);
    }
}

#[test]
fn advances_rv64_word_immediate_instructions() {
    let cases = [
        (
            addiw(3, 1, 1),
            0x0000_0001_ffff_ffff,
            RiscvInstruction::OpImm32 {
                kind: RiscvOpImm32Kind::Addiw,
                rd: 3,
                rs1: 1,
                immediate: 1,
            },
            0,
        ),
        (
            addiw(3, 1, 0),
            0x0000_0000_8000_0000,
            RiscvInstruction::OpImm32 {
                kind: RiscvOpImm32Kind::Addiw,
                rd: 3,
                rs1: 1,
                immediate: 0,
            },
            0xffff_ffff_8000_0000,
        ),
        (
            slliw(3, 1, 31),
            1,
            RiscvInstruction::OpImm32 {
                kind: RiscvOpImm32Kind::Slliw,
                rd: 3,
                rs1: 1,
                immediate: 31,
            },
            0xffff_ffff_8000_0000,
        ),
        (
            srliw(3, 1, 31),
            0x0000_0000_8000_0000,
            RiscvInstruction::OpImm32 {
                kind: RiscvOpImm32Kind::Srliw,
                rd: 3,
                rs1: 1,
                immediate: 31,
            },
            1,
        ),
        (
            sraiw(3, 1, 31),
            0x0000_0000_8000_0000,
            RiscvInstruction::OpImm32 {
                kind: RiscvOpImm32Kind::Sraiw,
                rd: 3,
                rs1: 1,
                immediate: 31,
            },
            u64::MAX,
        ),
    ];

    for (word, first, expected_instruction, expected_value) in cases {
        let mut memory = guest_machine_memory_with_words(&[word]);
        let mut state = GuestMachineState::new(memory.entry_address());
        state
            .set_register(FIRST_REGISTER, first)
            .expect("register write should be valid");

        let report =
            advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");

        assert_eq!(report.instruction, expected_instruction);
        assert_eq!(state.register(3), Some(expected_value));
        assert_eq!(state.pc(), ENTRY + 4);
    }
}

#[test]
fn advances_rv64_word_register_instructions() {
    let cases = [
        (
            addw(3, 1, 2),
            0x7fff_ffff,
            1,
            RiscvOp32Kind::Addw,
            0xffff_ffff_8000_0000,
        ),
        (subw(3, 1, 2), 0, 1, RiscvOp32Kind::Subw, u64::MAX),
        (
            sllw(3, 1, 2),
            1,
            31,
            RiscvOp32Kind::Sllw,
            0xffff_ffff_8000_0000,
        ),
        (srlw(3, 1, 2), 0x8000_0000, 31, RiscvOp32Kind::Srlw, 1),
        (
            sraw(3, 1, 2),
            0x8000_0000,
            31,
            RiscvOp32Kind::Sraw,
            u64::MAX,
        ),
        (
            mulw(3, 1, 2),
            0x7fff_ffff,
            2,
            RiscvOp32Kind::Mulw,
            u64::MAX - 1,
        ),
        (
            divw(3, 1, 2),
            (-7_i64) as u64,
            3,
            RiscvOp32Kind::Divw,
            (-2_i64) as u64,
        ),
        (divuw(3, 1, 2), 7, 3, RiscvOp32Kind::Divuw, 2),
        (
            remw(3, 1, 2),
            (-7_i64) as u64,
            3,
            RiscvOp32Kind::Remw,
            u64::MAX,
        ),
        (remuw(3, 1, 2), 7, 3, RiscvOp32Kind::Remuw, 1),
    ];

    for (word, first, second, kind, expected_value) in cases {
        let mut memory = guest_machine_memory_with_words(&[word]);
        let mut state = GuestMachineState::new(memory.entry_address());
        state
            .set_register(FIRST_REGISTER, first)
            .expect("register write should be valid");
        state
            .set_register(2, second)
            .expect("register write should be valid");

        let report =
            advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");

        assert_eq!(
            report.instruction,
            RiscvInstruction::Op32 {
                kind,
                rd: 3,
                rs1: 1,
                rs2: 2,
            }
        );
        assert_eq!(state.register(3), Some(expected_value));
        assert_eq!(state.pc(), ENTRY + 4);
    }
}

#[test]
fn advances_rv64_multiply_and_divide_instructions() {
    let cases = [
        (
            mul(3, 1, 2),
            (-1_i64) as u64,
            3,
            RiscvOpKind::Mul,
            (-3_i64) as u64,
        ),
        (
            mulh(3, 1, 2),
            (-2_i64) as u64,
            3,
            RiscvOpKind::Mulh,
            u64::MAX,
        ),
        (
            mulhsu(3, 1, 2),
            (-2_i64) as u64,
            1_u64 << 63,
            RiscvOpKind::Mulhsu,
            u64::MAX,
        ),
        (mulhu(3, 1, 2), u64::MAX, 2, RiscvOpKind::Mulhu, 1),
        (
            div(3, 1, 2),
            (-7_i64) as u64,
            3,
            RiscvOpKind::Div,
            (-2_i64) as u64,
        ),
        (divu(3, 1, 2), 7, 3, RiscvOpKind::Divu, 2),
        (rem(3, 1, 2), (-7_i64) as u64, 3, RiscvOpKind::Rem, u64::MAX),
        (remu(3, 1, 2), 7, 3, RiscvOpKind::Remu, 1),
        (div(3, 1, 2), 7, 0, RiscvOpKind::Div, u64::MAX),
        (divu(3, 1, 2), 7, 0, RiscvOpKind::Divu, u64::MAX),
        (rem(3, 1, 2), 7, 0, RiscvOpKind::Rem, 7),
        (remu(3, 1, 2), 7, 0, RiscvOpKind::Remu, 7),
        (
            div(3, 1, 2),
            i64::MIN as u64,
            u64::MAX,
            RiscvOpKind::Div,
            i64::MIN as u64,
        ),
        (rem(3, 1, 2), i64::MIN as u64, u64::MAX, RiscvOpKind::Rem, 0),
    ];

    for (word, first, second, kind, expected_value) in cases {
        let mut memory = guest_machine_memory_with_words(&[word]);
        let mut state = GuestMachineState::new(memory.entry_address());
        state
            .set_register(FIRST_REGISTER, first)
            .expect("register write should be valid");
        state
            .set_register(2, second)
            .expect("register write should be valid");

        let report =
            advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");

        assert_eq!(
            report.instruction,
            RiscvInstruction::Op {
                kind,
                rd: 3,
                rs1: 1,
                rs2: 2,
            }
        );
        assert_eq!(state.register(3), Some(expected_value));
        assert_eq!(state.pc(), ENTRY + 4);
    }
}

#[test]
fn advances_rv64_word_divide_edge_cases() {
    let cases = [
        (divw(3, 1, 2), 7, 0, RiscvOp32Kind::Divw, u64::MAX),
        (divuw(3, 1, 2), 7, 0, RiscvOp32Kind::Divuw, u64::MAX),
        (remw(3, 1, 2), 7, 0, RiscvOp32Kind::Remw, 7),
        (remuw(3, 1, 2), 7, 0, RiscvOp32Kind::Remuw, 7),
        (
            divw(3, 1, 2),
            0x0000_0000_8000_0000,
            u64::MAX,
            RiscvOp32Kind::Divw,
            0xffff_ffff_8000_0000,
        ),
        (
            remw(3, 1, 2),
            0x0000_0000_8000_0000,
            u64::MAX,
            RiscvOp32Kind::Remw,
            0,
        ),
        (
            divuw(3, 1, 2),
            0xffff_ffff,
            1,
            RiscvOp32Kind::Divuw,
            u64::MAX,
        ),
    ];

    for (word, first, second, kind, expected_value) in cases {
        let mut memory = guest_machine_memory_with_words(&[word]);
        let mut state = GuestMachineState::new(memory.entry_address());
        state
            .set_register(FIRST_REGISTER, first)
            .expect("register write should be valid");
        state
            .set_register(2, second)
            .expect("register write should be valid");

        let report =
            advance_guest_machine(&mut memory, &mut state).expect("instruction should execute");

        assert_eq!(
            report.instruction,
            RiscvInstruction::Op32 {
                kind,
                rd: 3,
                rs1: 1,
                rs2: 2,
            }
        );
        assert_eq!(state.register(3), Some(expected_value));
        assert_eq!(state.pc(), ENTRY + 4);
    }
}

#[test]
fn advances_signed_unsigned_branch_variants_and_negative_offsets() {
    let cases = [
        (beq(1, 2, -4), 3, 3, ENTRY - 4),
        (bne(1, 2, 8), 3, 4, ENTRY + 8),
        (blt(1, 2, 8), u64::MAX, 1, ENTRY + 8),
        (bge(1, 2, 8), 1, u64::MAX, ENTRY + 8),
        (bltu(1, 2, 8), 1, u64::MAX, ENTRY + 8),
        (bgeu(1, 2, 8), u64::MAX, 1, ENTRY + 8),
        (bne(1, 2, -4), 5, 5, ENTRY + 4),
    ];

    for (word, first, second, expected_pc) in cases {
        let state =
            execute_single_word_with_registers(word, &[(FIRST_REGISTER, first), (2, second)]);
        assert_eq!(state.pc(), expected_pc);
    }
}

#[test]
fn advances_fence_instructions_as_noops() {
    let mut memory = guest_machine_memory_with_words(&[0x0ff0_000f, 0x8330_000f, 0x0000_100f]);
    let before_memory = memory.clone();
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, 0xfeed_face_cafe_beef)
        .expect("register write should be valid");
    let cases = [
        (RiscvFenceKind::Fence, 0, 0xf, 0xf, ENTRY, ENTRY + 4),
        (RiscvFenceKind::FenceTso, 8, 3, 3, ENTRY + 4, ENTRY + 8),
        (RiscvFenceKind::FenceI, 0, 0, 0, ENTRY + 8, ENTRY + 12),
    ];

    for (kind, mode, predecessor, successor, address, next_pc) in cases {
        let report = advance_guest_machine(&mut memory, &mut state).expect("fence should execute");
        assert_eq!(report.address, address);
        assert_eq!(report.next_pc, next_pc);
        assert_eq!(
            report.instruction,
            RiscvInstruction::Fence {
                kind,
                mode,
                predecessor,
                successor,
            }
        );
        assert_eq!(state.pc(), next_pc);
        assert_eq!(state.register(1), Some(0xfeed_face_cafe_beef));
        assert_eq!(memory, before_memory);
    }
}

#[test]
fn rejects_invalid_public_register_indexes() {
    let mut state = GuestMachineState::new(ENTRY);

    assert_eq!(state.register(32), None);
    assert!(matches!(
        state.set_register(32, 1),
        Err(GuestMachineError::InvalidRegisterIndex { index: 32 })
    ));
    assert_eq!(state.registers(), &[0_u64; 32]);
}

#[test]
fn rejects_unsupported_guest_instructions_without_mutating_state() {
    let mut memory = guest_machine_memory_with_words(&[0x0000_0073]);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, 9)
        .expect("register write should be valid");
    let before = state.clone();

    assert!(matches!(
        advance_guest_machine(&mut memory, &mut state),
        Err(GuestMachineError::UnsupportedInstruction {
            address: ENTRY,
            instruction: RiscvInstruction::Ecall,
        })
    ));
    assert_eq!(state, before);

    let mut memory = guest_machine_memory_with_words(&[0x0010_0073]);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, 9)
        .expect("register write should be valid");
    let before = state.clone();
    assert!(matches!(
        advance_guest_machine(&mut memory, &mut state),
        Err(GuestMachineError::UnsupportedInstruction {
            address: ENTRY,
            instruction: RiscvInstruction::Ebreak,
        })
    ));
    assert_eq!(state, before);

    let mut memory = guest_machine_memory_with_words(&[0x0000_000b]);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(1, 9)
        .expect("register write should be valid");
    let before = state.clone();
    assert!(matches!(
        advance_guest_machine(&mut memory, &mut state),
        Err(GuestMachineError::UnsupportedInstruction {
            address: ENTRY,
            instruction: RiscvInstruction::Unknown {
                word: 0x0000_000b,
                opcode: 0x0b,
            },
        })
    ));
    assert_eq!(state, before);

    for word in [
        reserved_slliw_with_imm5(3, 1),
        reserved_srliw_with_imm5(3, 1),
        reserved_sraiw_with_imm5(3, 1),
    ] {
        let mut memory = guest_machine_memory_with_words(&[word]);
        let mut state = GuestMachineState::new(memory.entry_address());
        state
            .set_register(1, 9)
            .expect("register write should be valid");
        let before = state.clone();
        assert!(matches!(
            advance_guest_machine(&mut memory, &mut state),
            Err(GuestMachineError::UnsupportedInstruction {
                address: ENTRY,
                instruction: RiscvInstruction::Unknown { word: actual, opcode: 0x1b },
            }) if actual == word
        ));
        assert_eq!(state, before);
    }

    let mut memory = guest_machine_memory_with_bytes(&0x6001_u16.to_le_bytes());
    let mut state = GuestMachineState::new(memory.entry_address());
    assert!(matches!(
        advance_guest_machine(&mut memory, &mut state),
        Err(GuestMachineError::UnsupportedInstruction {
            address: ENTRY,
            instruction: RiscvInstruction::CompressedUnknown {
                halfword: 0x6001,
                quadrant: 1,
                funct3: 3,
            },
        })
    ));
    assert_eq!(state.pc(), ENTRY);

    let mut memory = guest_machine_memory_with_bytes(&0x001f_u16.to_le_bytes());
    let mut state = GuestMachineState::new(memory.entry_address());
    assert!(matches!(
        advance_guest_machine(&mut memory, &mut state),
        Err(GuestMachineError::UnsupportedInstructionLength {
            address: ENTRY,
            halfword: 0x001f,
        })
    ));
    assert_eq!(state.pc(), ENTRY);
}

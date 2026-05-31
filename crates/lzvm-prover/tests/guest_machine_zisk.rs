use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_prover::guest_machine::{
    advance_guest_machine, GuestMachineError, GuestMachineMemory, GuestMachineState,
};
use lzvm_prover::guest_memory::{load_guest_memory_image, GuestMemoryImage};
use tiny_keccak::keccakf;

const ENTRY: u64 = 0x8000_0000;

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
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: ENTRY,
        physical_address: ENTRY,
        file_size: code.len() as u64,
        memory_size: code.len() as u64,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(code);
    let info = parse_guest_image(&image).expect("guest image should parse");
    load_guest_memory_image(&image, &info).expect("guest memory should load")
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
    GuestMachineMemory::from_image(&guest_memory_image_with_bytes(&code))
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64_array<const N: usize>(bytes: &mut Vec<u8>, values: &[u64; N]) {
    for value in values {
        push_u64(bytes, *value);
    }
}

fn read_u64_array<const N: usize>(memory: &GuestMachineMemory, address: u64) -> [u64; N] {
    let mut bytes = vec![0_u8; N * 8];
    memory
        .read_range_into(address, &mut bytes)
        .expect("u64 array should read");
    let mut values = [0_u64; N];
    for (value, chunk) in values.iter_mut().zip(bytes.chunks_exact(8)) {
        *value = u64::from_le_bytes(chunk.try_into().expect("chunk should be 8 bytes"));
    }
    values
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

fn add(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_with_opcode(0, rs2, rs1, 0, rd, 0x33)
}

fn addi(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 0, rd, 0x13)
}

fn csrrs(rd: u8, csr: u16, rs1: u8) -> u32 {
    assert!(csr < 4096);
    assert_register(rs1);
    assert_register(rd);
    (u32::from(csr) << 20) | (u32::from(rs1) << 15) | (2 << 12) | (u32::from(rd) << 7) | 0x73
}

#[test]
fn advances_zisk_dma_memcpy_pair() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[csrrs(0, 0x0813, 11), add(0, 10, 12)],
        data_offset,
        &[
            0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0x10, 0x11, 0x12, 0x13, 0x14, 0xbb,
            0xbb, 0xbb,
        ],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, data_address)
        .expect("destination register should set");
    state
        .set_register(11, data_address + 8)
        .expect("source register should set");
    state
        .set_register(12, 5)
        .expect("count register should set");

    advance_guest_machine(&mut memory, &mut state).expect("dma marker should execute");
    advance_guest_machine(&mut memory, &mut state).expect("dma memcpy should execute");

    let mut stored = [0_u8; 16];
    memory
        .read_range_into(data_address, &mut stored)
        .expect("copied bytes should read");
    assert_eq!(
        stored,
        [
            0x10, 0x11, 0x12, 0x13, 0x14, 0xaa, 0xaa, 0xaa, 0x10, 0x11, 0x12, 0x13, 0x14, 0xbb,
            0xbb, 0xbb,
        ]
    );
    assert_eq!(state.register(10), Some(data_address));
    assert_eq!(state.pc(), ENTRY + 8);
}

#[test]
fn advances_zisk_dma_memcmp_pair() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[csrrs(0, 0x0814, 11), add(10, 10, 12)],
        data_offset,
        &[0x01, 0x02, 0x09, 0x04, 0x01, 0x02, 0x05, 0x04],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, data_address)
        .expect("first pointer register should set");
    state
        .set_register(11, data_address + 4)
        .expect("second pointer register should set");
    state
        .set_register(12, 4)
        .expect("count register should set");

    advance_guest_machine(&mut memory, &mut state).expect("dma marker should execute");
    advance_guest_machine(&mut memory, &mut state).expect("dma memcmp should execute");

    assert_eq!(state.register(10), Some(4));
    assert_eq!(state.pc(), ENTRY + 8);
}

#[test]
fn advances_zisk_dma_memset_pair() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut memory = guest_machine_memory_with_words_and_data(
        &[csrrs(0, 0x0816, 10), addi(0, 12, 0x005a)],
        data_offset,
        &[0xaa; 8],
    );
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, data_address)
        .expect("destination register should set");
    state
        .set_register(12, 5)
        .expect("count register should set");

    advance_guest_machine(&mut memory, &mut state).expect("dma marker should execute");
    advance_guest_machine(&mut memory, &mut state).expect("dma memset should execute");

    let mut stored = [0_u8; 8];
    memory
        .read_range_into(data_address, &mut stored)
        .expect("filled bytes should read");
    assert_eq!(stored, [0x5a, 0x5a, 0x5a, 0x5a, 0x5a, 0xaa, 0xaa, 0xaa]);
    assert_eq!(state.register(10), Some(data_address));
    assert_eq!(state.pc(), ENTRY + 8);
}

#[test]
fn advances_zisk_keccak_precompile() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let mut state_words = [0_u64; 25];
    for (index, word) in state_words.iter_mut().enumerate() {
        *word = 0x0102_0304_0506_0708_u64.wrapping_add(index as u64);
    }
    let mut data = Vec::with_capacity(state_words.len() * 8);
    for word in state_words {
        data.extend_from_slice(&word.to_le_bytes());
    }
    let mut expected_words = state_words;
    keccakf(&mut expected_words);
    let mut expected = Vec::with_capacity(expected_words.len() * 8);
    for word in expected_words {
        expected.extend_from_slice(&word.to_le_bytes());
    }

    let mut memory =
        guest_machine_memory_with_words_and_data(&[csrrs(0, 0x0800, 10)], data_offset, &data);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, data_address)
        .expect("state pointer register should set");

    advance_guest_machine(&mut memory, &mut state).expect("keccak precompile should execute");

    let mut stored = vec![0_u8; expected.len()];
    memory
        .read_range_into(data_address, &mut stored)
        .expect("keccak state should read");
    assert_eq!(stored, expected);
    assert_eq!(state.register(10), Some(data_address));
    assert_eq!(state.pc(), ENTRY + 4);
}

#[test]
fn advances_zisk_arith256_mod_precompile() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let params_address = data_address;
    let a_address = data_address + 40;
    let b_address = a_address + 32;
    let c_address = b_address + 32;
    let module_address = c_address + 32;
    let d_address = module_address + 32;
    let mut data = Vec::new();
    for address in [a_address, b_address, c_address, module_address, d_address] {
        push_u64(&mut data, address);
    }
    push_u64_array(&mut data, &[2, 0, 0, 0]);
    push_u64_array(&mut data, &[3, 0, 0, 0]);
    push_u64_array(&mut data, &[5, 0, 0, 0]);
    push_u64_array(&mut data, &[7, 0, 0, 0]);
    push_u64_array(&mut data, &[0, 0, 0, 0]);

    let mut memory =
        guest_machine_memory_with_words_and_data(&[csrrs(0, 0x0802, 10)], data_offset, &data);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, params_address)
        .expect("params register should set");

    advance_guest_machine(&mut memory, &mut state).expect("arith256_mod should execute");

    assert_eq!(read_u64_array(&memory, d_address), [4, 0, 0, 0]);
    assert_eq!(state.pc(), ENTRY + 4);
}

#[test]
fn rejects_zisk_arith256_mod_precompile_with_zero_modulus() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let params_address = data_address;
    let a_address = data_address + 40;
    let b_address = a_address + 32;
    let c_address = b_address + 32;
    let module_address = c_address + 32;
    let d_address = module_address + 32;
    let mut data = Vec::new();
    for address in [a_address, b_address, c_address, module_address, d_address] {
        push_u64(&mut data, address);
    }
    push_u64_array(&mut data, &[2, 0, 0, 0]);
    push_u64_array(&mut data, &[3, 0, 0, 0]);
    push_u64_array(&mut data, &[5, 0, 0, 0]);
    push_u64_array(&mut data, &[0, 0, 0, 0]);
    push_u64_array(&mut data, &[0, 0, 0, 0]);

    let mut memory =
        guest_machine_memory_with_words_and_data(&[csrrs(0, 0x0802, 10)], data_offset, &data);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, params_address)
        .expect("params register should set");

    assert_eq!(
        advance_guest_machine(&mut memory, &mut state),
        Err(GuestMachineError::ZeroArith256Modulus { address: ENTRY })
    );
}

#[test]
fn advances_zisk_arith256_precompile() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let params_address = data_address;
    let a_address = data_address + 40;
    let b_address = a_address + 32;
    let c_address = b_address + 32;
    let dl_address = c_address + 32;
    let dh_address = dl_address + 32;
    let mut data = Vec::new();
    for address in [a_address, b_address, c_address, dl_address, dh_address] {
        push_u64(&mut data, address);
    }
    push_u64_array(&mut data, &[2, 0, 0, 0]);
    push_u64_array(&mut data, &[3, 0, 0, 0]);
    push_u64_array(&mut data, &[5, 0, 0, 0]);
    push_u64_array(&mut data, &[0, 0, 0, 0]);
    push_u64_array(&mut data, &[0, 0, 0, 0]);

    let mut memory =
        guest_machine_memory_with_words_and_data(&[csrrs(0, 0x0801, 10)], data_offset, &data);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, params_address)
        .expect("params register should set");

    advance_guest_machine(&mut memory, &mut state).expect("arith256 should execute");

    assert_eq!(read_u64_array(&memory, dl_address), [11, 0, 0, 0]);
    assert_eq!(read_u64_array(&memory, dh_address), [0, 0, 0, 0]);
    assert_eq!(state.pc(), ENTRY + 4);
}

#[test]
fn advances_zisk_secp256k1_add_precompile() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let params_address = data_address;
    let p1_address = data_address + 16;
    let p2_address = p1_address + 64;
    let mut data = Vec::new();
    push_u64(&mut data, p1_address);
    push_u64(&mut data, p2_address);
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
    push_u64_array(
        &mut data,
        &[
            0xabac_09b9_5c70_9ee5,
            0x5c77_8e4b_8cef_3ca7,
            0x3045_406e_95c0_7cd8,
            0xc604_7f94_41ed_7d6d,
            0x2364_31a9_50cf_e52a,
            0xf7f6_3265_3266_d0e1,
            0xa3c5_8419_466c_eaee,
            0x1ae1_68fe_a63d_c339,
        ],
    );

    let mut memory =
        guest_machine_memory_with_words_and_data(&[csrrs(0, 0x0803, 10)], data_offset, &data);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, params_address)
        .expect("params register should set");

    advance_guest_machine(&mut memory, &mut state).expect("secp256k1 add should execute");

    assert_eq!(
        read_u64_array::<8>(&memory, p1_address),
        [
            0x8601_f113_bce0_36f9,
            0xb531_c845_836f_99b0,
            0x4934_4f85_f89d_5229,
            0xf930_8a01_9258_c310,
            0x6cb9_fd75_84b8_e672,
            0x6500_a999_34c2_231b,
            0x0fe3_37e6_2a37_f356,
            0x388f_7b0f_632d_e814,
        ]
    );
    assert_eq!(
        read_u64_array::<8>(&memory, p2_address),
        [
            0xabac_09b9_5c70_9ee5,
            0x5c77_8e4b_8cef_3ca7,
            0x3045_406e_95c0_7cd8,
            0xc604_7f94_41ed_7d6d,
            0x2364_31a9_50cf_e52a,
            0xf7f6_3265_3266_d0e1,
            0xa3c5_8419_466c_eaee,
            0x1ae1_68fe_a63d_c339,
        ]
    );
    assert_eq!(state.pc(), ENTRY + 4);
}

#[test]
fn advances_zisk_secp256k1_dbl_precompile() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
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

    let mut memory =
        guest_machine_memory_with_words_and_data(&[csrrs(0, 0x0804, 15)], data_offset, &data);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(15, data_address)
        .expect("point register should set");

    advance_guest_machine(&mut memory, &mut state).expect("secp256k1 double should execute");

    assert_eq!(
        read_u64_array::<8>(&memory, data_address),
        [
            0xabac_09b9_5c70_9ee5,
            0x5c77_8e4b_8cef_3ca7,
            0x3045_406e_95c0_7cd8,
            0xc604_7f94_41ed_7d6d,
            0x2364_31a9_50cf_e52a,
            0xf7f6_3265_3266_d0e1,
            0xa3c5_8419_466c_eaee,
            0x1ae1_68fe_a63d_c339,
        ]
    );
    assert_eq!(state.pc(), ENTRY + 4);
}

#[test]
fn advances_zisk_add256_precompile() {
    let data_offset = 64;
    let data_address = ENTRY + data_offset as u64;
    let params_address = data_address;
    let a_address = data_address + 32;
    let b_address = a_address + 32;
    let c_address = b_address + 32;
    let mut data = Vec::new();
    push_u64(&mut data, a_address);
    push_u64(&mut data, b_address);
    push_u64(&mut data, 0);
    push_u64(&mut data, c_address);
    push_u64_array(&mut data, &[u64::MAX, u64::MAX, u64::MAX, u64::MAX]);
    push_u64_array(&mut data, &[1, 0, 0, 0]);
    push_u64_array(&mut data, &[0, 0, 0, 0]);

    let mut memory =
        guest_machine_memory_with_words_and_data(&[csrrs(5, 0x0811, 10)], data_offset, &data);
    let mut state = GuestMachineState::new(memory.entry_address());
    state
        .set_register(10, params_address)
        .expect("params register should set");

    let report = advance_guest_machine(&mut memory, &mut state).expect("add256 should execute");

    assert_eq!(read_u64_array(&memory, c_address), [0, 0, 0, 0]);
    assert_eq!(state.register(5), Some(1));
    assert_eq!(report.precompile_result, Some(1));
    assert_eq!(state.pc(), ENTRY + 4);
}

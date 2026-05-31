use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_prover::guest_instruction::{RiscvCsr, RiscvInstruction};
use lzvm_prover::guest_machine::{
    advance_guest_machine, GuestMachineMemory, GuestMachineState, ZISK_ARCHITECTURE_ID,
};
use lzvm_prover::guest_memory::{load_guest_memory_image, GuestMemoryImage};

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

fn guest_machine_memory_with_words(words: &[u32]) -> GuestMachineMemory {
    let mut code = Vec::with_capacity(words.len() * 4);
    for word in words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    GuestMachineMemory::from_image(&guest_memory_image_with_bytes(&code))
}

fn csrrs(rd: u8, csr: u16, rs1: u8) -> u32 {
    assert!(rd < 32);
    assert!(csr < 4096);
    assert!(rs1 < 32);
    (u32::from(csr) << 20) | (u32::from(rs1) << 15) | (2 << 12) | (u32::from(rd) << 7) | 0x73
}

#[test]
fn advances_machine_csr_reads() {
    const RV64IMAC_MISA: u64 = 0x8000_0000_0000_1105;

    let cases = [
        (0x0301, RiscvCsr::Misa, 10, RV64IMAC_MISA),
        (0x0f11, RiscvCsr::Mvendorid, 11, 0),
        (0x0f12, RiscvCsr::Marchid, 12, ZISK_ARCHITECTURE_ID),
        (0x0f13, RiscvCsr::Mimpid, 13, 0),
        (0x0f14, RiscvCsr::Mhartid, 14, 0),
    ];
    let words: Vec<u32> = cases
        .iter()
        .map(|(csr_number, _, rd, _)| csrrs(*rd, *csr_number, 0))
        .collect();
    let mut memory = guest_machine_memory_with_words(&words);
    let mut state = GuestMachineState::new(memory.entry_address());

    for (_, _, rd, _) in cases {
        state
            .set_register(usize::from(rd), u64::MAX)
            .expect("register write should be valid");
    }

    for (index, (_, csr, rd, value)) in cases.into_iter().enumerate() {
        let report =
            advance_guest_machine(&mut memory, &mut state).expect("csr read should execute");
        let address = ENTRY + (index as u64) * 4;

        assert_eq!(report.address, address);
        assert_eq!(report.next_pc, address + 4);
        assert_eq!(report.instruction, RiscvInstruction::CsrRead { csr, rd });
        assert_eq!(state.register(usize::from(rd)), Some(value));
        assert_eq!(state.pc(), address + 4);
    }
}

#[test]
fn advances_counter_csr_reads_with_deterministic_ticks() {
    let cases = [
        (0x0c00, RiscvCsr::Cycle, 10, 0),
        (0x0c01, RiscvCsr::Time, 11, 1),
        (0x0c02, RiscvCsr::Instret, 12, 2),
        (0x0c80, RiscvCsr::Cycleh, 13, 0),
        (0x0c81, RiscvCsr::Timeh, 14, 0),
        (0x0c82, RiscvCsr::Instreth, 15, 0),
        (0x0c00, RiscvCsr::Cycle, 16, 6),
        (0x0b00, RiscvCsr::Mcycle, 17, 7),
        (0x0b02, RiscvCsr::Minstret, 18, 8),
        (0x0b80, RiscvCsr::Mcycleh, 19, 0),
        (0x0b82, RiscvCsr::Minstreth, 20, 0),
    ];
    let words: Vec<u32> = cases
        .iter()
        .map(|(csr_number, _, rd, _)| csrrs(*rd, *csr_number, 0))
        .collect();
    let mut memory = guest_machine_memory_with_words(&words);
    let mut state = GuestMachineState::new(memory.entry_address());

    for (_, _, rd, _) in cases {
        state
            .set_register(usize::from(rd), u64::MAX)
            .expect("register write should be valid");
    }

    for (index, (_, csr, rd, value)) in cases.into_iter().enumerate() {
        let report =
            advance_guest_machine(&mut memory, &mut state).expect("csr read should execute");
        let address = ENTRY + (index as u64) * 4;

        assert_eq!(report.address, address);
        assert_eq!(report.next_pc, address + 4);
        assert_eq!(report.instruction, RiscvInstruction::CsrRead { csr, rd });
        assert_eq!(state.register(usize::from(rd)), Some(value));
        assert_eq!(state.pc(), address + 4);
    }
}

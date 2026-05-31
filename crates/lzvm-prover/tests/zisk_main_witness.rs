use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_field::Felt;
use lzvm_prover::guest_pc_trace_backend::GuestPcTraceBackend;
use lzvm_prover::witness_layout::derive_witness_trace_layout;
use lzvm_prover::witness_loader::WitnessComputeContext;
use lzvm_prover::witness_runner::run_witness_trace_with_context;
use lzvm_prover::witness_trace::WitnessTraceBuffer;
use lzvm_prover::ProveUnitSchedule;

const ENTRY: u64 = 0x8000_0000;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-zisk-main-witness-{}-{name}",
        std::process::id()
    ))
}

fn sample_guest_image_with_words(words: &[u32]) -> Vec<u8> {
    let mut code = Vec::with_capacity(words.len() * 4);
    for word in words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let header = program_header(120, code.len() as u64);
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&code);
    image
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

fn program_header(file_offset: u64, file_size: u64) -> [u8; 56] {
    program_header_at(file_offset, ENTRY, file_size)
}

fn program_header_at(file_offset: u64, virtual_address: u64, file_size: u64) -> [u8; 56] {
    let mut bytes = [0_u8; 56];
    bytes[0..4].copy_from_slice(&1_u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&5_u32.to_le_bytes());
    bytes[8..16].copy_from_slice(&file_offset.to_le_bytes());
    bytes[16..24].copy_from_slice(&virtual_address.to_le_bytes());
    bytes[24..32].copy_from_slice(&virtual_address.to_le_bytes());
    bytes[32..40].copy_from_slice(&file_size.to_le_bytes());
    bytes[40..48].copy_from_slice(&file_size.to_le_bytes());
    bytes[48..56].copy_from_slice(&0x1000_u64.to_le_bytes());
    bytes
}

fn encode_i(immediate: i16, rs1: u8, funct3: u8, rd: u8, opcode: u8) -> u32 {
    (((immediate as i32 as u32) & 0x0fff) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | u32::from(opcode)
}

fn addi(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 0, rd, 0x13)
}

fn jalr(rd: u8, rs1: u8, offset: i16) -> u32 {
    encode_i(offset, rs1, 0, rd, 0x67)
}

fn op_imm(rd: u8, rs1: u8, immediate: i16, funct3: u8) -> u32 {
    encode_i(immediate, rs1, funct3, rd, 0x13)
}

fn op_imm_shift(rd: u8, rs1: u8, shamt: u8, funct3: u8, funct6: u8) -> u32 {
    assert!(funct6 < 64);
    assert!(rd < 32);
    assert!(rs1 < 32);
    assert!(shamt < 64);
    (u32::from(funct6) << 26)
        | (u32::from(shamt) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | 0x13
}

fn slli(rd: u8, rs1: u8, shamt: u8) -> u32 {
    op_imm_shift(rd, rs1, shamt, 1, 0)
}

fn srli(rd: u8, rs1: u8, shamt: u8) -> u32 {
    op_imm_shift(rd, rs1, shamt, 5, 0)
}

fn srai(rd: u8, rs1: u8, shamt: u8) -> u32 {
    op_imm_shift(rd, rs1, shamt, 5, 0x10)
}

fn op_imm_32(rd: u8, rs1: u8, immediate: i16, funct3: u8) -> u32 {
    encode_i(immediate, rs1, funct3, rd, 0x1b)
}

fn addiw(rd: u8, rs1: u8, immediate: i16) -> u32 {
    op_imm_32(rd, rs1, immediate, 0)
}

fn op_imm_32_shift(rd: u8, rs1: u8, shamt: u8, funct3: u8, funct7: u8) -> u32 {
    assert!(funct7 < 128);
    assert!(rd < 32);
    assert!(rs1 < 32);
    assert!(shamt < 32);
    (u32::from(funct7) << 25)
        | (u32::from(shamt) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | 0x1b
}

fn slliw(rd: u8, rs1: u8, shamt: u8) -> u32 {
    op_imm_32_shift(rd, rs1, shamt, 1, 0)
}

fn srliw(rd: u8, rs1: u8, shamt: u8) -> u32 {
    op_imm_32_shift(rd, rs1, shamt, 5, 0)
}

fn sraiw(rd: u8, rs1: u8, shamt: u8) -> u32 {
    op_imm_32_shift(rd, rs1, shamt, 5, 0x20)
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

fn encode_r(funct7: u8, rs2: u8, rs1: u8, funct3: u8, rd: u8) -> u32 {
    (u32::from(funct7) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | 0x33
}

fn add(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0, rs2, rs1, 0, rd)
}

fn sub(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0x20, rs2, rs1, 0, rd)
}

fn slt(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0, rs2, rs1, 2, rd)
}

fn sltu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r(0, rs2, rs1, 3, rd)
}

fn encode_r_32(funct7: u8, rs2: u8, rs1: u8, funct3: u8, rd: u8) -> u32 {
    (u32::from(funct7) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | 0x3b
}

fn addw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_32(0, rs2, rs1, 0, rd)
}

fn subw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_32(0x20, rs2, rs1, 0, rd)
}

fn sllw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_32(0, rs2, rs1, 1, rd)
}

fn srlw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_32(0, rs2, rs1, 5, rd)
}

fn sraw(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_r_32(0x20, rs2, rs1, 5, rd)
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

fn upper_immediate(rd: u8, immediate: u32, opcode: u8) -> u32 {
    assert!(rd < 32);
    assert_eq!(immediate & 0x0fff, 0);
    (immediate & 0xffff_f000) | (u32::from(rd) << 7) | u32::from(opcode)
}

fn lui(rd: u8, immediate: u32) -> u32 {
    upper_immediate(rd, immediate, 0x37)
}

fn auipc(rd: u8, immediate: u32) -> u32 {
    upper_immediate(rd, immediate, 0x17)
}

fn jal(rd: u8, offset: i32) -> u32 {
    assert!(rd < 32);
    assert_eq!(offset & 1, 0);
    assert!((-1_048_576..=1_048_574).contains(&offset));
    let offset = offset as u32;
    (((offset >> 20) & 1) << 31)
        | (((offset >> 1) & 0x03ff) << 21)
        | (((offset >> 11) & 1) << 20)
        | (((offset >> 12) & 0xff) << 12)
        | (u32::from(rd) << 7)
        | 0x6f
}

fn branch(funct3: u8, rs1: u8, rs2: u8, offset: i16) -> u32 {
    assert!((-4096..=4094).contains(&offset));
    assert_eq!(offset & 1, 0);
    assert!(rs1 < 32);
    assert!(rs2 < 32);
    assert!(funct3 < 8);
    let offset = offset as i32 as u32;
    (((offset >> 12) & 1) << 31)
        | (((offset >> 5) & 0x3f) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (((offset >> 1) & 0x0f) << 8)
        | (((offset >> 11) & 1) << 7)
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

fn commitment_column(
    name: &str,
    stage: u32,
    stage_position: u32,
    dimension: u32,
) -> CommitmentColumn {
    CommitmentColumn {
        name: name.to_owned(),
        stage,
        dimension,
        pols_map_id: 0,
        stage_id: stage.saturating_sub(1),
        stage_position,
        intermediate: false,
        lengths: Vec::new(),
    }
}

fn sample_unit_with_zisk_main_columns_rows(row_count: u64) -> ProveUnitSchedule {
    let base_domain_size = row_count;
    let base_domain_bits = row_count.next_power_of_two().ilog2();
    let extended_domain_bits = base_domain_bits + 1;
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits,
        extended_domain_bits,
        base_domain_size,
        extended_domain_size: base_domain_size * 2,
        blowup_factor: 2,
        query_count: 1,
        proof_of_work_bits: 0,
        merkle_tree_arity: 4,
        last_level_verification: 0,
        transcript_arity: Some(4),
        hash_commits: true,
        transcript_root_challenge_draws: vec![1, 1],
        challenge_count: 1,
        evaluation_value_count: 0,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 1,
        constant_width: 0,
        stage_commit_widths: vec![27],
        commitment_columns: vec![
            commitment_column("a", 1, 0, 2),
            commitment_column("b", 1, 2, 2),
            commitment_column("c", 1, 4, 2),
            commitment_column("flag", 1, 6, 1),
            commitment_column("pc", 1, 7, 1),
            commitment_column("a_src_imm", 1, 8, 1),
            commitment_column("b_src_imm", 1, 9, 1),
            commitment_column("a_src_reg", 1, 10, 1),
            commitment_column("b_src_reg", 1, 11, 1),
            commitment_column("store_reg", 1, 12, 1),
            commitment_column("store_pc", 1, 13, 1),
            commitment_column("set_pc", 1, 14, 1),
            commitment_column("op", 1, 15, 1),
            commitment_column("jmp_offset1", 1, 16, 1),
            commitment_column("jmp_offset2", 1, 17, 1),
            commitment_column("m32", 1, 18, 1),
            commitment_column("is_external_op", 1, 19, 1),
            commitment_column("is_precompiled", 1, 20, 1),
            commitment_column("b_src_ind", 1, 21, 1),
            commitment_column("ind_width", 1, 22, 1),
            commitment_column("store_ind", 1, 23, 1),
            commitment_column("store_offset", 1, 24, 1),
            commitment_column("store_mem", 1, 25, 1),
            commitment_column("b_offset_imm0", 1, 26, 1),
        ],
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: extended_domain_bits,
            output_bits: 1,
            folding_factor: 4,
        }],
        final_layer_bits: 1,
        fixed_bytes: 0,
        constant_tree_root: None,
        pcs_material_bytes: None,
        pcs_material_plan_digest: None,
        pcs_material_fixed_column_digest: None,
        pcs_material_constant_tree_digest: None,
        pcs_material_constant_tree_root: None,
        pcs_material_fixed_byte_count: None,
        pcs_material_constant_tree_byte_count: None,
        pcs_material_leaf_byte_count: None,
        pcs_material_node_byte_count: None,
    }
}

fn sample_unit_with_zisk_main_columns() -> ProveUnitSchedule {
    sample_unit_with_zisk_main_columns_rows(3)
}

fn sample_unit_with_zisk_main_columns_without_memory_columns_rows(
    row_count: u64,
) -> ProveUnitSchedule {
    let base_domain_size = row_count;
    let base_domain_bits = row_count.next_power_of_two().ilog2();
    let extended_domain_bits = base_domain_bits + 1;
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits,
        extended_domain_bits,
        base_domain_size,
        extended_domain_size: base_domain_size * 2,
        blowup_factor: 2,
        query_count: 1,
        proof_of_work_bits: 0,
        merkle_tree_arity: 4,
        last_level_verification: 0,
        transcript_arity: Some(4),
        hash_commits: true,
        transcript_root_challenge_draws: vec![1, 1],
        challenge_count: 1,
        evaluation_value_count: 0,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 1,
        constant_width: 0,
        stage_commit_widths: vec![21],
        commitment_columns: vec![
            commitment_column("a", 1, 0, 2),
            commitment_column("b", 1, 2, 2),
            commitment_column("c", 1, 4, 2),
            commitment_column("flag", 1, 6, 1),
            commitment_column("pc", 1, 7, 1),
            commitment_column("a_src_imm", 1, 8, 1),
            commitment_column("b_src_imm", 1, 9, 1),
            commitment_column("a_src_reg", 1, 10, 1),
            commitment_column("b_src_reg", 1, 11, 1),
            commitment_column("store_reg", 1, 12, 1),
            commitment_column("store_pc", 1, 13, 1),
            commitment_column("set_pc", 1, 14, 1),
            commitment_column("op", 1, 15, 1),
            commitment_column("jmp_offset1", 1, 16, 1),
            commitment_column("jmp_offset2", 1, 17, 1),
            commitment_column("m32", 1, 18, 1),
            commitment_column("is_external_op", 1, 19, 1),
            commitment_column("is_precompiled", 1, 20, 1),
        ],
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: extended_domain_bits,
            output_bits: 1,
            folding_factor: 4,
        }],
        final_layer_bits: 1,
        fixed_bytes: 0,
        constant_tree_root: None,
        pcs_material_bytes: None,
        pcs_material_plan_digest: None,
        pcs_material_fixed_column_digest: None,
        pcs_material_constant_tree_digest: None,
        pcs_material_constant_tree_root: None,
        pcs_material_fixed_byte_count: None,
        pcs_material_constant_tree_byte_count: None,
        pcs_material_leaf_byte_count: None,
        pcs_material_node_byte_count: None,
    }
}

fn felt(value: u64) -> Felt {
    Felt::from_canonical(value).expect("canonical")
}

fn assert_cell(trace: &WitnessTraceBuffer, row: usize, column: usize, value: u64) {
    assert_eq!(trace.value(row, column), Some(felt(value)));
}

fn assert_signed_cell(trace: &WitnessTraceBuffer, row: usize, column: usize, value: i64) {
    let value = if value >= 0 {
        felt(value as u64)
    } else {
        -felt(value.unsigned_abs())
    };
    assert_eq!(trace.value(row, column), Some(value));
}

fn assert_wide(trace: &WitnessTraceBuffer, row: usize, column: usize, value: u64) {
    assert_cell(trace, row, column, value & 0xffff_ffff);
    assert_cell(trace, row, column + 1, value >> 32);
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_layout_for_supported_ops() {
    let dir = temp_dir("layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), add(3, 1, 2), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns();
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write supported rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 3);
    assert_eq!(trace.column_count(), 27);
    assert_eq!(trace.value(0, 0), Some(Felt::ZERO));
    assert_eq!(trace.value(0, 1), Some(Felt::ZERO));
    assert_cell(&trace, 0, 2, 7);
    assert_eq!(trace.value(0, 3), Some(Felt::ZERO));
    assert_cell(&trace, 0, 4, 7);
    assert_eq!(trace.value(0, 5), Some(Felt::ZERO));
    assert_eq!(trace.value(0, 6), Some(Felt::ZERO));
    assert_cell(&trace, 0, 7, ENTRY);
    assert_cell(&trace, 0, 8, 1);
    assert_cell(&trace, 0, 9, 1);
    assert_eq!(trace.value(0, 10), Some(Felt::ZERO));
    assert_eq!(trace.value(0, 11), Some(Felt::ZERO));
    assert_cell(&trace, 0, 12, 1);
    assert_cell(&trace, 0, 24, 1);
    assert_cell(&trace, 0, 15, 1);
    assert_cell(&trace, 0, 16, 4);
    assert_cell(&trace, 0, 17, 4);

    assert_cell(&trace, 1, 0, 7);
    assert_cell(&trace, 1, 2, 3);
    assert_cell(&trace, 1, 4, 10);
    assert_cell(&trace, 1, 7, ENTRY + 4);
    assert_cell(&trace, 1, 10, 1);
    assert_eq!(trace.value(1, 11), Some(Felt::ZERO));
    assert_cell(&trace, 1, 12, 1);
    assert_cell(&trace, 1, 24, 2);
    assert_cell(&trace, 1, 15, 10);

    assert_cell(&trace, 2, 0, 7);
    assert_cell(&trace, 2, 2, 10);
    assert_cell(&trace, 2, 4, 17);
    assert_cell(&trace, 2, 10, 1);
    assert_cell(&trace, 2, 11, 1);
    assert_cell(&trace, 2, 12, 1);
    assert_cell(&trace, 2, 24, 3);
    assert_cell(&trace, 2, 15, 10);
}

#[test]
fn guest_pc_trace_backend_writes_non_memory_zisk_main_rows_without_memory_columns() {
    let dir = temp_dir("layout-without-memory-columns");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), add(3, 1, 2), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_without_memory_columns_rows(3);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("non-memory Zisk Main rows should not require memory columns");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 3);
    assert_eq!(trace.column_count(), 21);
    assert_cell(&trace, 0, 4, 7);
    assert_cell(&trace, 1, 4, 10);
    assert_cell(&trace, 2, 4, 17);
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_memory_rows_for_doubleword_load_store() {
    let dir = temp_dir("memory");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let loaded_value = 0x1122_3344_5566_7788_u64;
    let code_words = [
        addi(1, 0, data_address as i16),
        ld(2, 1, 0),
        addi(3, 0, 42),
        sd(1, 3, 8),
        ld(4, 1, 8),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&loaded_value.to_le_bytes());
    data.extend_from_slice(&0_u64.to_le_bytes());
    let headers = [
        program_header_at(176, ENTRY, code.len() as u64),
        program_header_at(data_offset, data_address, data.len() as u64),
    ];
    let mut guest_image_bytes = sample_guest_image_with_program_headers(&headers);
    guest_image_bytes.resize(176, 0);
    guest_image_bytes.extend_from_slice(&code);
    guest_image_bytes.resize(data_offset as usize, 0);
    guest_image_bytes.extend_from_slice(&data);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(5);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write memory rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 5);
    assert_eq!(trace.column_count(), 27);
    assert_cell(&trace, 1, 0, data_address);
    assert_cell(&trace, 1, 2, 0x5566_7788);
    assert_cell(&trace, 1, 3, 0x1122_3344);
    assert_cell(&trace, 1, 4, 0x5566_7788);
    assert_cell(&trace, 1, 5, 0x1122_3344);
    assert_cell(&trace, 1, 12, 1);
    assert_cell(&trace, 1, 24, 2);
    assert_cell(&trace, 1, 15, 1);
    assert_cell(&trace, 1, 21, 1);
    assert_cell(&trace, 1, 22, 8);
    assert_eq!(trace.value(1, 26), Some(Felt::ZERO));

    assert_cell(&trace, 3, 0, data_address);
    assert_cell(&trace, 3, 2, 42);
    assert_cell(&trace, 3, 4, 42);
    assert_eq!(trace.value(3, 12), Some(Felt::ZERO));
    assert_cell(&trace, 3, 15, 1);
    assert_cell(&trace, 3, 22, 8);
    assert_cell(&trace, 3, 23, 1);
    assert_cell(&trace, 3, 24, 8);

    assert_cell(&trace, 4, 2, 42);
    assert_cell(&trace, 4, 4, 42);
    assert_cell(&trace, 4, 24, 4);
    assert_cell(&trace, 4, 26, 8);
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_narrow_memory_rows() {
    let dir = temp_dir("narrow-memory");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let code_words = [
        addi(1, 0, data_address as i16),
        lbu(2, 1, 0),
        lhu(3, 1, 1),
        lwu(4, 1, 3),
        sb(1, 2, 8),
        sh(1, 3, 10),
        sw(1, 4, 12),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&[0x80, 0x34, 0x80, 0xef, 0xcd, 0xab, 0x89, 0x00]);
    data.extend_from_slice(&[0; 8]);
    let headers = [
        program_header_at(176, ENTRY, code.len() as u64),
        program_header_at(data_offset, data_address, data.len() as u64),
    ];
    let mut guest_image_bytes = sample_guest_image_with_program_headers(&headers);
    guest_image_bytes.resize(176, 0);
    guest_image_bytes.extend_from_slice(&code);
    guest_image_bytes.resize(data_offset as usize, 0);
    guest_image_bytes.extend_from_slice(&data);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(7);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write narrow memory rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 7);
    assert_eq!(trace.column_count(), 27);
    assert_cell(&trace, 1, 4, 0x80);
    assert_cell(&trace, 1, 22, 1);
    assert_eq!(trace.value(1, 26), Some(Felt::ZERO));
    assert_cell(&trace, 2, 4, 0x8034);
    assert_cell(&trace, 2, 22, 2);
    assert_cell(&trace, 2, 26, 1);
    assert_cell(&trace, 3, 4, 0x89ab_cdef);
    assert_cell(&trace, 3, 22, 4);
    assert_cell(&trace, 3, 24, 4);
    assert_cell(&trace, 3, 26, 3);

    assert_cell(&trace, 4, 4, 0x80);
    assert_cell(&trace, 4, 22, 1);
    assert_cell(&trace, 4, 23, 1);
    assert_cell(&trace, 4, 24, 8);
    assert_cell(&trace, 5, 22, 2);
    assert_cell(&trace, 5, 24, 10);
    assert_cell(&trace, 6, 22, 4);
    assert_cell(&trace, 6, 24, 12);
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_signed_load_rows() {
    let dir = temp_dir("signed-loads");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let code_words = [
        addi(1, 0, data_address as i16),
        lb(2, 1, 0),
        lh(3, 1, 1),
        lw(4, 1, 3),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&[0x80, 0x34, 0x80, 0xef, 0xcd, 0xab, 0x89]);
    let headers = [
        program_header_at(176, ENTRY, code.len() as u64),
        program_header_at(data_offset, data_address, data.len() as u64),
    ];
    let mut guest_image_bytes = sample_guest_image_with_program_headers(&headers);
    guest_image_bytes.resize(176, 0);
    guest_image_bytes.extend_from_slice(&code);
    guest_image_bytes.resize(data_offset as usize, 0);
    guest_image_bytes.extend_from_slice(&data);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(4);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write signed load rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 4);
    assert_eq!(trace.column_count(), 27);
    assert_cell(&trace, 1, 2, 0x80);
    assert_cell(&trace, 1, 4, 0xffff_ff80);
    assert_cell(&trace, 1, 5, 0xffff_ffff);
    assert_cell(&trace, 1, 15, 0x27);
    assert_cell(&trace, 1, 22, 1);
    assert_eq!(trace.value(1, 26), Some(Felt::ZERO));

    assert_cell(&trace, 2, 4, 0xffff_8034);
    assert_cell(&trace, 2, 5, 0xffff_ffff);
    assert_cell(&trace, 2, 15, 0x28);
    assert_cell(&trace, 2, 22, 2);
    assert_cell(&trace, 2, 26, 1);

    assert_cell(&trace, 3, 4, 0x89ab_cdef);
    assert_cell(&trace, 3, 5, 0xffff_ffff);
    assert_cell(&trace, 3, 15, 0x29);
    assert_cell(&trace, 3, 22, 4);
    assert_cell(&trace, 3, 26, 3);
}

#[test]
fn guest_pc_trace_backend_rejects_zisk_main_memory_rows_without_memory_columns() {
    let dir = temp_dir("memory-missing-columns");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let code_words = [addi(1, 0, data_address as i16), ld(2, 1, 0), 0x0000_0073];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
    let headers = [
        program_header_at(176, ENTRY, code.len() as u64),
        program_header_at(data_offset, data_address, data.len() as u64),
    ];
    let mut guest_image_bytes = sample_guest_image_with_program_headers(&headers);
    guest_image_bytes.resize(176, 0);
    guest_image_bytes.extend_from_slice(&code);
    guest_image_bytes.resize(data_offset as usize, 0);
    guest_image_bytes.extend_from_slice(&data);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_without_memory_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let error = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect_err("Zisk Main memory rows require memory columns");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(error.to_string().contains("memory rows require"));
}

#[test]
fn guest_pc_trace_backend_rejects_unsupported_zisk_main_instruction() {
    let dir = temp_dir("unsupported");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[encode_r(1, 0, 0, 0, 1), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns();
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let error = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect_err("unsupported Zisk Main instruction should fail");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let message = error.to_string();
    assert!(message.contains("Zisk Main lowering failed"));
    assert!(message.contains("does not support instruction"));
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_alu_and_compare_rows() {
    let dir = temp_dir("alu-compare");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let code_words = [
        addi(1, 0, 10),
        addi(2, 0, 3),
        addi(8, 0, -16),
        sub(3, 1, 2),
        op_imm(4, 3, 6, 7),
        op_imm(5, 4, 1, 6),
        op_imm(6, 5, 2, 4),
        slli(7, 2, 4),
        srli(9, 7, 2),
        srai(10, 8, 2),
        op_imm(11, 8, -8, 2),
        op_imm(12, 2, 2, 3),
        slt(13, 8, 2),
        sltu(14, 2, 1),
        0x0000_0073,
    ];
    let guest_image_bytes = sample_guest_image_with_words(&code_words);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(14);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write ALU and compare rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 14);
    assert_eq!(trace.column_count(), 27);

    assert_wide(&trace, 3, 4, 7);
    assert_cell(&trace, 3, 6, 0);
    assert_cell(&trace, 3, 10, 1);
    assert_cell(&trace, 3, 11, 1);
    assert_cell(&trace, 3, 15, 0x0b);

    assert_wide(&trace, 4, 4, 6);
    assert_cell(&trace, 4, 9, 1);
    assert_cell(&trace, 4, 11, 0);
    assert_cell(&trace, 4, 15, 0x0e);

    assert_wide(&trace, 5, 4, 7);
    assert_cell(&trace, 5, 15, 0x0f);

    assert_wide(&trace, 6, 4, 5);
    assert_cell(&trace, 6, 15, 0x10);

    assert_wide(&trace, 7, 4, 48);
    assert_cell(&trace, 7, 9, 1);
    assert_cell(&trace, 7, 15, 0x21);

    assert_wide(&trace, 8, 4, 12);
    assert_cell(&trace, 8, 15, 0x22);

    assert_wide(&trace, 9, 4, (-4_i64) as u64);
    assert_cell(&trace, 9, 15, 0x23);

    assert_wide(&trace, 10, 4, 1);
    assert_cell(&trace, 10, 6, 1);
    assert_cell(&trace, 10, 15, 0x07);

    assert_wide(&trace, 11, 4, 0);
    assert_cell(&trace, 11, 6, 0);
    assert_cell(&trace, 11, 15, 0x06);

    assert_wide(&trace, 12, 4, 1);
    assert_cell(&trace, 12, 6, 1);
    assert_cell(&trace, 12, 11, 1);
    assert_cell(&trace, 12, 15, 0x07);

    assert_wide(&trace, 13, 4, 1);
    assert_cell(&trace, 13, 6, 1);
    assert_cell(&trace, 13, 15, 0x06);
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_word_alu_rows() {
    let dir = temp_dir("word-alu");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let code_words = [
        addi(1, 0, -1),
        addiw(2, 1, 1),
        addi(3, 0, 1),
        slliw(4, 3, 31),
        srliw(5, 4, 31),
        sraiw(6, 4, 31),
        addw(7, 4, 3),
        subw(8, 3, 4),
        sllw(9, 3, 3),
        srlw(10, 4, 3),
        sraw(11, 4, 3),
        0x0000_0073,
    ];
    let guest_image_bytes = sample_guest_image_with_words(&code_words);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(11);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write word ALU rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 11);
    assert_eq!(trace.column_count(), 27);

    assert_wide(&trace, 1, 0, u64::MAX);
    assert_wide(&trace, 1, 4, 0);
    assert_cell(&trace, 1, 9, 1);
    assert_cell(&trace, 1, 15, 0x1a);
    assert_cell(&trace, 1, 18, 1);

    assert_wide(&trace, 3, 4, 0xffff_ffff_8000_0000);
    assert_cell(&trace, 3, 9, 1);
    assert_cell(&trace, 3, 15, 0x24);
    assert_cell(&trace, 3, 18, 1);

    assert_wide(&trace, 4, 4, 1);
    assert_cell(&trace, 4, 15, 0x25);
    assert_cell(&trace, 4, 18, 1);

    assert_wide(&trace, 5, 4, u64::MAX);
    assert_cell(&trace, 5, 15, 0x26);
    assert_cell(&trace, 5, 18, 1);

    assert_wide(&trace, 6, 4, 0xffff_ffff_8000_0001);
    assert_cell(&trace, 6, 11, 1);
    assert_cell(&trace, 6, 15, 0x1a);
    assert_cell(&trace, 6, 18, 1);

    assert_wide(&trace, 7, 4, 0xffff_ffff_8000_0001);
    assert_cell(&trace, 7, 15, 0x1b);
    assert_cell(&trace, 7, 18, 1);

    assert_wide(&trace, 8, 4, 2);
    assert_cell(&trace, 8, 15, 0x24);
    assert_cell(&trace, 8, 18, 1);

    assert_wide(&trace, 9, 4, 0x4000_0000);
    assert_cell(&trace, 9, 15, 0x25);
    assert_cell(&trace, 9, 18, 1);

    assert_wide(&trace, 10, 4, 0xffff_ffff_c000_0000);
    assert_cell(&trace, 10, 15, 0x26);
    assert_cell(&trace, 10, 18, 1);
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_branch_rows() {
    let dir = temp_dir("branches");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let code_words = [
        addi(1, 0, 5),
        addi(2, 0, 5),
        addi(4, 0, 7),
        beq(1, 2, 8),
        addi(3, 0, 111),
        bne(1, 2, 8),
        blt(1, 4, 8),
        addi(5, 0, 222),
        bge(4, 1, 8),
        addi(6, 0, 333),
        bltu(1, 4, 8),
        addi(7, 0, 444),
        bgeu(4, 1, 8),
        addi(8, 0, 555),
        bne(1, 2, -4),
        0x0000_0073,
    ];
    let guest_image_bytes = sample_guest_image_with_words(&code_words);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(10);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write branch rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 10);
    assert_eq!(trace.column_count(), 27);

    assert_cell(&trace, 2, 7, ENTRY + 8);
    assert_cell(&trace, 2, 4, 7);

    assert_cell(&trace, 3, 0, 5);
    assert_cell(&trace, 3, 2, 5);
    assert_cell(&trace, 3, 4, 1);
    assert_cell(&trace, 3, 6, 1);
    assert_cell(&trace, 3, 7, ENTRY + 12);
    assert_cell(&trace, 3, 10, 1);
    assert_cell(&trace, 3, 11, 1);
    assert_eq!(trace.value(3, 12), Some(Felt::ZERO));
    assert_cell(&trace, 3, 15, 0x09);
    assert_cell(&trace, 3, 16, 8);
    assert_cell(&trace, 3, 17, 4);

    assert_cell(&trace, 4, 0, 5);
    assert_cell(&trace, 4, 2, 5);
    assert_cell(&trace, 4, 4, 1);
    assert_cell(&trace, 4, 6, 1);
    assert_cell(&trace, 4, 7, ENTRY + 20);
    assert_eq!(trace.value(4, 12), Some(Felt::ZERO));
    assert_cell(&trace, 4, 15, 0x09);
    assert_cell(&trace, 4, 16, 4);
    assert_cell(&trace, 4, 17, 8);

    assert_cell(&trace, 5, 0, 5);
    assert_cell(&trace, 5, 2, 7);
    assert_cell(&trace, 5, 4, 1);
    assert_cell(&trace, 5, 6, 1);
    assert_cell(&trace, 5, 7, ENTRY + 24);
    assert_cell(&trace, 5, 15, 0x07);
    assert_cell(&trace, 5, 16, 8);
    assert_cell(&trace, 5, 17, 4);

    assert_cell(&trace, 6, 0, 7);
    assert_cell(&trace, 6, 2, 5);
    assert_cell(&trace, 6, 4, 0);
    assert_cell(&trace, 6, 6, 0);
    assert_cell(&trace, 6, 7, ENTRY + 32);
    assert_cell(&trace, 6, 15, 0x07);
    assert_cell(&trace, 6, 16, 4);
    assert_cell(&trace, 6, 17, 8);

    assert_cell(&trace, 7, 0, 5);
    assert_cell(&trace, 7, 2, 7);
    assert_cell(&trace, 7, 4, 1);
    assert_cell(&trace, 7, 6, 1);
    assert_cell(&trace, 7, 7, ENTRY + 40);
    assert_cell(&trace, 7, 15, 0x06);
    assert_cell(&trace, 7, 16, 8);
    assert_cell(&trace, 7, 17, 4);

    assert_cell(&trace, 8, 0, 7);
    assert_cell(&trace, 8, 2, 5);
    assert_cell(&trace, 8, 4, 0);
    assert_cell(&trace, 8, 6, 0);
    assert_cell(&trace, 8, 7, ENTRY + 48);
    assert_cell(&trace, 8, 15, 0x06);
    assert_cell(&trace, 8, 16, 4);
    assert_cell(&trace, 8, 17, 8);

    assert_cell(&trace, 9, 0, 5);
    assert_cell(&trace, 9, 2, 5);
    assert_cell(&trace, 9, 4, 1);
    assert_cell(&trace, 9, 6, 1);
    assert_cell(&trace, 9, 7, ENTRY + 56);
    assert_cell(&trace, 9, 15, 0x09);
    assert_cell(&trace, 9, 16, 4);
    assert_signed_cell(&trace, 9, 17, -4);
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_jump_and_pc_store_rows() {
    let dir = temp_dir("jump-pc-store");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let code_words = [
        lui(1, 0x1234_5000),
        auipc(2, 0x1000),
        jal(3, 8),
        addi(4, 0, 111),
        auipc(5, 0),
        addi(5, 5, 15),
        jalr(6, 5, 2),
        addi(7, 0, 222),
        addi(8, 0, 9),
        0x0000_0073,
    ];
    let guest_image_bytes = sample_guest_image_with_words(&code_words);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(7);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write jump rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 7);
    assert_eq!(trace.column_count(), 27);

    assert_wide(&trace, 0, 2, 0x1234_5000);
    assert_wide(&trace, 0, 4, 0x1234_5000);
    assert_cell(&trace, 0, 12, 1);
    assert_eq!(trace.value(0, 13), Some(Felt::ZERO));
    assert_eq!(trace.value(0, 14), Some(Felt::ZERO));
    assert_cell(&trace, 0, 15, 0x01);

    assert_cell(&trace, 1, 4, 0);
    assert_cell(&trace, 1, 6, 1);
    assert_cell(&trace, 1, 7, ENTRY + 4);
    assert_cell(&trace, 1, 12, 1);
    assert_cell(&trace, 1, 13, 1);
    assert_eq!(trace.value(1, 14), Some(Felt::ZERO));
    assert_cell(&trace, 1, 15, 0x00);
    assert_cell(&trace, 1, 16, 4);
    assert_cell(&trace, 1, 17, 0x1000);

    assert_cell(&trace, 2, 4, 0);
    assert_cell(&trace, 2, 6, 1);
    assert_cell(&trace, 2, 7, ENTRY + 8);
    assert_cell(&trace, 2, 12, 1);
    assert_cell(&trace, 2, 13, 1);
    assert_eq!(trace.value(2, 14), Some(Felt::ZERO));
    assert_cell(&trace, 2, 15, 0x00);
    assert_cell(&trace, 2, 16, 8);
    assert_cell(&trace, 2, 17, 4);

    assert_cell(&trace, 3, 7, ENTRY + 16);
    assert_cell(&trace, 3, 13, 1);
    assert_cell(&trace, 3, 17, 0);

    assert_wide(&trace, 4, 0, ENTRY + 16);
    assert_cell(&trace, 4, 2, 15);
    assert_wide(&trace, 4, 4, ENTRY + 31);

    assert_wide(&trace, 5, 0, !1);
    assert_wide(&trace, 5, 2, ENTRY + 31);
    assert_wide(&trace, 5, 4, ENTRY + 30);
    assert_eq!(trace.value(5, 6), Some(Felt::ZERO));
    assert_cell(&trace, 5, 7, ENTRY + 24);
    assert_cell(&trace, 5, 12, 1);
    assert_cell(&trace, 5, 13, 1);
    assert_cell(&trace, 5, 14, 1);
    assert_cell(&trace, 5, 15, 0x0e);
    assert_cell(&trace, 5, 16, 2);
    assert_cell(&trace, 5, 17, 4);

    assert_cell(&trace, 6, 7, ENTRY + 32);
    assert_cell(&trace, 6, 4, 9);
}

#[test]
fn guest_pc_trace_backend_rejects_odd_offset_jalr_zisk_main_row() {
    let dir = temp_dir("odd-jalr");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let code_words = [auipc(5, 0), jalr(0, 5, 9), 0x0000_0073];
    let guest_image_bytes = sample_guest_image_with_words(&code_words);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let error = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect_err("odd-offset jalr is not a single-row Zisk Main instruction");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let message = error.to_string();
    assert!(message.contains("Zisk Main lowering failed"));
    assert!(message.contains("Jalr"));
}

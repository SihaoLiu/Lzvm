use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_field::{Felt, MODULUS};
use lzvm_prover::guest_pc_trace_backend::{
    run_guest_pc_trace_segments_with_context, GuestPcTraceBackend, GuestPcTraceSegmentRunOutput,
};
use lzvm_prover::witness_layout::derive_witness_trace_layout;
use lzvm_prover::witness_loader::{
    load_witness_library, WitnessBackend, WitnessCallError, WitnessComputeContext,
    WitnessTraceBuffers, WitnessTraceOutput, WitnessTraceProofValue, WitnessTraceUnitValue,
};
use lzvm_prover::witness_runner::{
    run_witness_trace, run_witness_trace_output_with_context, run_witness_trace_with_context,
    WitnessTraceRequest, WitnessTraceRunError, WitnessTraceRunOutput,
};
use lzvm_prover::witness_trace::WitnessTraceError;
use lzvm_prover::zisk_fcalls::{
    ZISK_INPUT_ADDRESS, ZISK_INPUT_READY_FCALL_ID, ZISK_MSB_POS_256_FCALL_ID,
};
use lzvm_prover::ProveUnitSchedule;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-witness-runner-{}-{name}", std::process::id()))
}

fn build_shared_library(dir: &Path, name: &str, source: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("fixture directory should be created");
    let source_path = dir.join(format!("{name}.c"));
    let library_path = dir.join(format!("lib{name}.so"));
    fs::write(&source_path, source).expect("fixture source should be written");
    let status = Command::new("cc")
        .arg("-shared")
        .arg("-fPIC")
        .arg(&source_path)
        .arg("-o")
        .arg(&library_path)
        .status()
        .expect("cc should run");
    assert!(status.success(), "cc should build the fixture library");
    library_path
}

const ENTRY: u64 = 0x8000_0000;

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

fn encode_r(funct7: u8, rs2: u8, rs1: u8, funct3: u8, rd: u8) -> u32 {
    assert!(funct7 < 128);
    assert!(rs2 < 32);
    assert!(rs1 < 32);
    assert!(funct3 < 8);
    assert!(rd < 32);
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

fn op_imm_shift(funct6: u8, rd: u8, rs1: u8, shamt: u8) -> u32 {
    assert!(funct6 < 64);
    assert!(rd < 32);
    assert!(rs1 < 32);
    assert!(shamt < 64);
    (u32::from(funct6) << 26)
        | (u32::from(shamt) << 20)
        | (u32::from(rs1) << 15)
        | (1 << 12)
        | (u32::from(rd) << 7)
        | 0x13
}

fn slli(rd: u8, rs1: u8, shamt: u8) -> u32 {
    op_imm_shift(0, rd, rs1, shamt)
}

fn srli(rd: u8, rs1: u8, shamt: u8) -> u32 {
    op_imm_shift(0, rd, rs1, shamt) | (5 << 12)
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

fn lr_d(rd: u8, rs1: u8) -> u32 {
    encode_amo(0x02, false, false, 0, rs1, 3, rd)
}

fn sc_d(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x03, false, false, rs2, rs1, 3, rd)
}

fn amoadd_d(rd: u8, rs1: u8, rs2: u8) -> u32 {
    encode_amo(0x00, false, false, rs2, rs1, 3, rd)
}

fn auipc(rd: u8, immediate: u32) -> u32 {
    assert!(rd < 32);
    assert_eq!(immediate & 0x0fff, 0);
    (immediate & 0xffff_f000) | (u32::from(rd) << 7) | 0x17
}

fn lui(rd: u8, immediate: u32) -> u32 {
    assert!(rd < 32);
    assert_eq!(immediate & 0x0fff, 0);
    (immediate & 0xffff_f000) | (u32::from(rd) << 7) | 0x37
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

fn csrs(csr: u16, rs1: u8) -> u32 {
    encode_csr(0, csr, 2, rs1)
}

fn csrrs(rd: u8, csr: u16, rs1: u8) -> u32 {
    encode_csr(rd, csr, 2, rs1)
}

fn csrwi(csr: u16, immediate: u8) -> u32 {
    encode_csr(0, csr, 5, immediate)
}

fn encode_b(offset: i16, rs1: u8, rs2: u8, funct3: u8) -> u32 {
    assert!((-4096..=4094).contains(&offset));
    assert_eq!(offset & 1, 0);
    assert!(rs1 < 32);
    assert!(rs2 < 32);
    assert!(funct3 < 8);
    let immediate = offset as i32 as u32;
    (((immediate >> 12) & 1) << 31)
        | (((immediate >> 5) & 0x3f) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (((immediate >> 1) & 0x0f) << 8)
        | (((immediate >> 11) & 1) << 7)
        | 0x63
}

fn bne(rs1: u8, rs2: u8, offset: i16) -> u32 {
    encode_b(offset, rs1, rs2, 1)
}

fn framed_stdin_chunk(payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes.resize(bytes.len().next_multiple_of(8), 0);
    bytes
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

fn sample_unit_with_pc_columns() -> ProveUnitSchedule {
    sample_unit_with_trace_columns(
        3,
        vec![2, 2],
        vec![
            commitment_column("pc", 1, 1, 1),
            commitment_column("next_pc", 2, 0, 1),
        ],
    )
}

fn sample_unit_with_guest_effect_columns() -> ProveUnitSchedule {
    sample_unit_with_trace_columns(
        6,
        vec![10],
        vec![
            commitment_column("pc", 1, 0, 1),
            commitment_column("next_pc", 1, 1, 1),
            commitment_column("reg_write_index", 1, 2, 1),
            commitment_column("reg_write_value", 1, 3, 1),
            commitment_column("mem_read_address", 1, 4, 1),
            commitment_column("mem_read_value", 1, 5, 1),
            commitment_column("mem_read_byte_len", 1, 6, 1),
            commitment_column("mem_write_address", 1, 7, 1),
            commitment_column("mem_write_value", 1, 8, 1),
            commitment_column("mem_write_byte_len", 1, 9, 1),
        ],
    )
}

fn sample_unit_with_register_effect_columns() -> ProveUnitSchedule {
    sample_unit_with_trace_columns(
        2,
        vec![2],
        vec![
            commitment_column("reg_write_index", 1, 0, 1),
            commitment_column("reg_write_value", 1, 1, 1),
        ],
    )
}

fn sample_unit_with_zisk_main_address_columns(base_domain_size: u64) -> ProveUnitSchedule {
    sample_unit_with_trace_columns(
        base_domain_size,
        vec![29],
        vec![
            commitment_column("a", 1, 0, 2),
            commitment_column("b", 1, 2, 2),
            commitment_column("c", 1, 4, 2),
            commitment_column("flag", 1, 6, 1),
            commitment_column("pc", 1, 7, 1),
            commitment_column("op", 1, 8, 1),
            commitment_column("store_pc", 1, 9, 1),
            commitment_column("set_pc", 1, 10, 1),
            commitment_column("a_src_reg", 1, 11, 1),
            commitment_column("b_src_reg", 1, 12, 1),
            commitment_column("store_reg", 1, 13, 1),
            commitment_column("b_src_imm", 1, 14, 1),
            commitment_column("b_src_ind", 1, 15, 1),
            commitment_column("b_offset_imm0", 1, 16, 1),
            commitment_column("store_ind", 1, 17, 1),
            commitment_column("store_offset", 1, 18, 1),
            commitment_column("air.addr1", 1, 19, 1),
            commitment_column("air.addr2", 1, 20, 1),
            commitment_column("store_mem", 1, 21, 1),
            commitment_column("ind_width", 1, 22, 1),
            commitment_column("air.a_imm1", 1, 23, 1),
            commitment_column("air.b_imm1", 1, 24, 1),
            commitment_column("is_external_op", 1, 25, 1),
            commitment_column("a_src_imm", 1, 26, 1),
            commitment_column("jmp_offset1", 1, 27, 1),
            commitment_column("jmp_offset2", 1, 28, 1),
        ],
    )
}

fn sample_unit_with_zisk_main_memory_source_columns(base_domain_size: u64) -> ProveUnitSchedule {
    sample_unit_with_trace_columns(
        base_domain_size,
        vec![31],
        vec![
            commitment_column("a", 1, 0, 2),
            commitment_column("b", 1, 2, 2),
            commitment_column("c", 1, 4, 2),
            commitment_column("flag", 1, 6, 1),
            commitment_column("pc", 1, 7, 1),
            commitment_column("op", 1, 8, 1),
            commitment_column("store_pc", 1, 9, 1),
            commitment_column("set_pc", 1, 10, 1),
            commitment_column("a_src_reg", 1, 11, 1),
            commitment_column("b_src_reg", 1, 12, 1),
            commitment_column("store_reg", 1, 13, 1),
            commitment_column("b_src_imm", 1, 14, 1),
            commitment_column("b_src_ind", 1, 15, 1),
            commitment_column("b_offset_imm0", 1, 16, 1),
            commitment_column("store_ind", 1, 17, 1),
            commitment_column("store_offset", 1, 18, 1),
            commitment_column("air.addr1", 1, 19, 1),
            commitment_column("air.addr2", 1, 20, 1),
            commitment_column("store_mem", 1, 21, 1),
            commitment_column("ind_width", 1, 22, 1),
            commitment_column("air.a_imm1", 1, 23, 1),
            commitment_column("air.b_imm1", 1, 24, 1),
            commitment_column("is_external_op", 1, 25, 1),
            commitment_column("a_src_imm", 1, 26, 1),
            commitment_column("jmp_offset1", 1, 27, 1),
            commitment_column("jmp_offset2", 1, 28, 1),
            commitment_column("a_src_mem", 1, 29, 1),
            commitment_column("b_src_mem", 1, 30, 1),
        ],
    )
}

fn sample_unit_with_zisk_main_register_step_columns(base_domain_size: u64) -> ProveUnitSchedule {
    sample_unit_with_trace_columns(
        base_domain_size,
        vec![40],
        vec![
            commitment_column("a", 1, 0, 2),
            commitment_column("b", 1, 2, 2),
            commitment_column("c", 1, 4, 2),
            commitment_column("flag", 1, 6, 1),
            commitment_column("pc", 1, 7, 1),
            commitment_column("op", 1, 8, 1),
            commitment_column("store_pc", 1, 9, 1),
            commitment_column("set_pc", 1, 10, 1),
            commitment_column("a_src_reg", 1, 11, 1),
            commitment_column("b_src_reg", 1, 12, 1),
            commitment_column("store_reg", 1, 13, 1),
            commitment_column("b_src_imm", 1, 14, 1),
            commitment_column("b_src_ind", 1, 15, 1),
            commitment_column("b_offset_imm0", 1, 16, 1),
            commitment_column("store_ind", 1, 17, 1),
            commitment_column("store_offset", 1, 18, 1),
            commitment_column("air.addr1", 1, 19, 1),
            commitment_column("air.addr2", 1, 20, 1),
            commitment_column("store_mem", 1, 21, 1),
            commitment_column("ind_width", 1, 22, 1),
            commitment_column("air.a_imm1", 1, 23, 1),
            commitment_column("air.b_imm1", 1, 24, 1),
            commitment_column("is_external_op", 1, 25, 1),
            commitment_column("a_src_imm", 1, 26, 1),
            commitment_column("jmp_offset1", 1, 27, 1),
            commitment_column("jmp_offset2", 1, 28, 1),
            commitment_column("a_reg_prev_mem_step", 1, 29, 1),
            commitment_column("b_reg_prev_mem_step", 1, 30, 1),
            commitment_column("store_reg_prev_mem_step", 1, 31, 1),
            commitment_column("store_reg_prev_value", 1, 32, 2),
        ],
    )
}

fn sample_unit_with_zisk_rom_columns(base_domain_size: u64) -> ProveUnitSchedule {
    sample_unit_with_trace_columns(
        base_domain_size,
        vec![12],
        vec![
            commitment_column("line", 1, 0, 1),
            commitment_column("a_offset_imm0", 1, 1, 1),
            commitment_column("a_imm1", 1, 2, 1),
            commitment_column("b_offset_imm0", 1, 3, 1),
            commitment_column("b_imm1", 1, 4, 1),
            commitment_column("ind_width", 1, 5, 1),
            commitment_column("op", 1, 6, 1),
            commitment_column("store_offset", 1, 7, 1),
            commitment_column("jmp_offset1", 1, 8, 1),
            commitment_column("jmp_offset2", 1, 9, 1),
            commitment_column("flags", 1, 10, 1),
            commitment_column("multiplicity", 1, 11, 1),
        ],
    )
}

fn sample_unit_with_trace_columns(
    base_domain_size: u64,
    stage_commit_widths: Vec<u32>,
    commitment_columns: Vec<CommitmentColumn>,
) -> ProveUnitSchedule {
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits: 2,
        extended_domain_bits: 3,
        base_domain_size,
        extended_domain_size: 8,
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
        stage_commit_widths,
        commitment_columns,
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: 3,
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

struct NativeBackend;

impl WitnessBackend for NativeBackend {
    fn compute(
        &self,
        buffers: &mut WitnessTraceBuffers,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        if buffers.input().len() < 2 || buffers.output().len() < 16 {
            return Err(WitnessCallError::NativeReturn { code: -1 });
        }
        let first = buffers.input()[0];
        let second = buffers.input()[1];
        let output = buffers.output_mut();
        output[0..8].copy_from_slice(&(u64::from(first) + 1).to_le_bytes());
        output[8..16].copy_from_slice(&(u64::from(second) + 1).to_le_bytes());
        Ok(WitnessTraceOutput::new(16))
    }
}

struct ShortBackend;

impl WitnessBackend for ShortBackend {
    fn compute(
        &self,
        buffers: &mut WitnessTraceBuffers,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        buffers.output_mut()[..8].copy_from_slice(&13_u64.to_le_bytes());
        Ok(WitnessTraceOutput::new(8))
    }
}

struct OverflowBackend;

impl WitnessBackend for OverflowBackend {
    fn compute(
        &self,
        _buffers: &mut WitnessTraceBuffers,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        Ok(WitnessTraceOutput::new(24))
    }
}

#[test]
fn runs_native_witness_and_parses_trace_values() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "valid",
        r#"#include <stddef.h>
typedef struct {
    const unsigned char *input_ptr;
    size_t input_len;
    unsigned char *output_ptr;
    size_t output_len;
} LzvmWitnessCall;
typedef struct {
    int status;
    size_t produced_len;
} LzvmWitnessResult;
static void write_u64_le(unsigned char *out, unsigned long long value) {
    for (size_t i = 0; i < 8; i++) {
        out[i] = (unsigned char)((value >> (i * 8)) & 0xff);
    }
}
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    if (!call || !result || call->input_len < 2 || call->output_len < 16) {
        return -1;
    }
    write_u64_le(call->output_ptr, (unsigned long long)call->input_ptr[0] + 1);
    write_u64_le(call->output_ptr + 8, (unsigned long long)call->input_ptr[1] + 1);
    result->status = 0;
    result->produced_len = 16;
    return 0;
}
"#,
    );
    let library = load_witness_library(&library_path).expect("witness library should load");

    let trace = run_witness_trace(&library, WitnessTraceRequest::new(vec![6, 8], 1, 2))
        .expect("witness trace should run and parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 1);
    assert_eq!(trace.column_count(), 2);
    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(7).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(9).expect("canonical"))
    );
}

#[test]
fn runs_witness_trace_with_native_backend() {
    let trace = run_witness_trace(&NativeBackend, WitnessTraceRequest::new(vec![6, 8], 1, 2))
        .expect("witness trace should run and parse");

    assert_eq!(trace.row_count(), 1);
    assert_eq!(trace.column_count(), 2);
    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(7).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(9).expect("canonical"))
    );
}

#[test]
fn rejects_short_witness_backend_output_before_trace_parse() {
    let error = run_witness_trace(&ShortBackend, WitnessTraceRequest::new(Vec::new(), 1, 2))
        .expect_err("short witness output should be rejected");

    assert_eq!(
        error.to_string(),
        "witness backend produced incomplete output: produced 8, expected 16"
    );
}

#[test]
fn rejects_oversized_witness_backend_output_without_panicking() {
    let result = run_witness_trace(&OverflowBackend, WitnessTraceRequest::new(Vec::new(), 1, 2));

    assert!(matches!(
        result,
        Err(WitnessTraceRunError::Call(
            WitnessCallError::OutputOverflow {
                produced_len: 24,
                output_len: 16
            }
        ))
    ));
}

#[test]
fn guest_pc_trace_backend_runs_guest_image_from_context() {
    let dir = temp_dir("guest-pc-trace");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: None,
        },
        WitnessTraceRequest::new(Vec::new(), 2, 2),
    )
    .expect("guest PC trace should run");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(ENTRY).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 0),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 1),
        Some(Felt::from_canonical(ENTRY + 8).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_uses_framed_input_for_zisk_free_calls() {
    let dir = temp_dir("guest-pc-trace-input");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(5, ZISK_INPUT_ADDRESS as u32),
        addi(6, 5, 16),
        csrs(0x08f0, 6),
        csrwi(0x08c0, ZISK_INPUT_READY_FCALL_ID as u8),
        lbu(7, 5, 16),
        bne(7, 0, 12),
        addi(1, 0, 1),
        0x0000_0073,
        addi(1, 0, 2),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: None,
        },
        WitnessTraceRequest::new(framed_stdin_chunk(&[7]), 7, 2),
    )
    .expect("guest PC trace should use witness input");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        trace.value(5, 0),
        Some(Felt::from_canonical(ENTRY + 20).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 1),
        Some(Felt::from_canonical(ENTRY + 32).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 0),
        Some(Felt::from_canonical(ENTRY + 32).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 1),
        Some(Felt::from_canonical(ENTRY + 36).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_fcall_result_source() {
    let dir = temp_dir("guest-zisk-main-fcall-result-source");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(5, ZISK_INPUT_ADDRESS as u32),
        addi(6, 5, 79),
        csrs(0x08f0, 6),
        csrwi(0x08c0, ZISK_INPUT_READY_FCALL_ID as u8),
        addi(10, 0, 2),
        csrs(0x08f0, 10),
        addi(11, 5, 16),
        csrs(0x08f2, 11),
        addi(12, 5, 48),
        csrs(0x08f2, 12),
        csrwi(0x08c0, ZISK_MSB_POS_256_FCALL_ID as u8),
        csrrs(13, 0x0ffe, 0),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_memory_source_columns(13);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let mut payload = Vec::with_capacity(64);
    for word in [0_u64, 0, 0, 1 << 9, 0, 0, 0, 0] {
        payload.extend_from_slice(&word.to_le_bytes());
    }
    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(framed_stdin_chunk(&payload)),
    )
    .expect("guest trace should write Zisk Main free-call result source");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 13);
    assert_eq!(trace.column_count(), 31);
    assert_eq!(
        trace.value(11, 2),
        Some(Felt::from_canonical(3).expect("canonical"))
    );
    assert_eq!(
        trace.value(11, 4),
        Some(Felt::from_canonical(3).expect("canonical"))
    );
    assert_eq!(
        trace.value(11, 13),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(11, 18),
        Some(Felt::from_canonical(13).expect("canonical"))
    );
    assert_eq!(
        trace.value(11, 30),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_dma_memcpy_pair() {
    let dir = temp_dir("guest-zisk-main-dma-memcpy-pair");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(11, 10, 16),
        addi(12, 0, 16),
        csrrs(0, 0x0813, 11),
        add(13, 10, 12),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(8);
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
    .expect("guest trace should write Zisk Main DMA memcpy pair");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 8);
    assert_eq!(trace.column_count(), 29);
    assert_eq!(
        trace.value(5, 2),
        Some(Felt::from_canonical(16).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 8),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 21),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 18),
        Some(Felt::from_canonical(0xa000_0f00).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 2),
        Some(Felt::from_canonical(0xa000_0010).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 4),
        Some(Felt::from_canonical(0xa000_0000).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 8),
        Some(Felt::from_canonical(0xd0).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 13),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 18),
        Some(Felt::from_canonical(13).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 25),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_expands_successful_store_conditional() {
    let dir = temp_dir("guest-zisk-main-store-conditional");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(7, 0, 9),
        lr_d(14, 10),
        sc_d(12, 10, 7),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(7);
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
    .expect("guest trace should write successful store-conditional rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 7);
    assert_eq!(trace.column_count(), 29);
    assert_eq!(
        trace.value(5, 2),
        Some(Felt::from_canonical(9).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 4),
        Some(Felt::from_canonical(9).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 7),
        Some(Felt::from_canonical(ENTRY + 20).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 8),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 17),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 18),
        Some(Felt::from_canonical(0).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 22),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 27),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 28),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 2),
        Some(Felt::from_canonical(0).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 4),
        Some(Felt::from_canonical(0).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 7),
        Some(Felt::from_canonical(ENTRY + 21).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 8),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 13),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 18),
        Some(Felt::from_canonical(12).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 27),
        Some(Felt::from_canonical(3).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 28),
        Some(Felt::from_canonical(3).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_segments_account_for_store_conditional_rows() {
    let dir = temp_dir("guest-zisk-main-store-conditional-segments");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(7, 0, 9),
        lr_d(14, 10),
        sc_d(12, 10, 7),
        addi(13, 0, 5),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(7);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let segments = run_guest_pc_trace_segments_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("guest trace should split after store-conditional rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(segments.len(), 2);
    let first = segments[0].trace();
    assert_eq!(
        first.value(6, 7),
        Some(Felt::from_canonical(ENTRY + 21).expect("canonical"))
    );
    assert_eq!(
        first.value(6, 13),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        first.value(6, 18),
        Some(Felt::from_canonical(12).expect("canonical"))
    );
    assert_eq!(
        unit_value(&segments[0], "segment_next_pc"),
        &[Felt::from_canonical(ENTRY + 24).expect("canonical")]
    );
    assert_eq!(
        unit_value(&segments[1], "segment_initial_pc"),
        &[Felt::from_canonical(ENTRY + 24).expect("canonical")]
    );
}

#[test]
fn guest_pc_trace_backend_expands_doubleword_amo_add() {
    let dir = temp_dir("guest-zisk-main-amo-add");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(7, 0, 5),
        sd(10, 7, 0),
        addi(8, 0, 3),
        amoadd_d(12, 10, 8),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(9);
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
    .expect("guest trace should write doubleword AMO add rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 9);
    assert_eq!(trace.column_count(), 29);
    assert_eq!(
        trace.value(6, 2),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 4),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 7),
        Some(Felt::from_canonical(ENTRY + 24).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 13),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 18),
        Some(Felt::from_canonical(12).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 22),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 27),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 0),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 2),
        Some(Felt::from_canonical(3).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 4),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 7),
        Some(Felt::from_canonical(ENTRY + 25).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 8),
        Some(Felt::from_canonical(0x0a).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 27),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 2),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 4),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 7),
        Some(Felt::from_canonical(ENTRY + 26).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 17),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 22),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 27),
        Some(Felt::from_canonical(2).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 28),
        Some(Felt::from_canonical(2).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_expands_aliased_doubleword_amo_add() {
    let dir = temp_dir("guest-zisk-main-aliased-amo-add");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(7, 0, 5),
        sd(10, 7, 0),
        addi(8, 0, 3),
        amoadd_d(10, 10, 8),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_memory_source_columns(10);
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
    .expect("guest trace should write aliased doubleword AMO add rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 10);
    assert_eq!(trace.column_count(), 31);
    assert_eq!(
        trace.value(6, 2),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 4),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 7),
        Some(Felt::from_canonical(ENTRY + 24).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 18),
        Some(Felt::from_canonical(0xa000_0100).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 21),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 0),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 4),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 2),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 17),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(9, 2),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(9, 4),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(9, 13),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(9, 18),
        Some(Felt::from_canonical(10).expect("canonical"))
    );
    assert_eq!(
        trace.value(9, 30),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_segments_keep_zisk_dma_prepare_lookahead() {
    let dir = temp_dir("guest-zisk-main-dma-segment-lookahead");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(11, 10, 16),
        addi(12, 0, 16),
        csrrs(0, 0x0813, 11),
        add(13, 10, 12),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(6);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let segments = run_guest_pc_trace_segments_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("guest trace should carry DMA lookahead across segments");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(segments.len(), 2);
    let first = segments[0].trace();
    assert_eq!(
        first.value(5, 2),
        Some(Felt::from_canonical(16).expect("canonical"))
    );
    assert_eq!(
        first.value(5, 8),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        first.value(5, 21),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        first.value(5, 18),
        Some(Felt::from_canonical(0xa000_0f00).expect("canonical"))
    );

    let second = segments[1].trace();
    assert_eq!(
        second.value(0, 2),
        Some(Felt::from_canonical(0xa000_0010).expect("canonical"))
    );
    assert_eq!(
        second.value(0, 4),
        Some(Felt::from_canonical(0xa000_0000).expect("canonical"))
    );
    assert_eq!(
        second.value(0, 8),
        Some(Felt::from_canonical(0xd0).expect("canonical"))
    );
    assert_eq!(
        unit_value(&segments[0], "segment_next_pc"),
        &[Felt::from_canonical(ENTRY + 24).expect("canonical")]
    );
    assert_eq!(
        unit_value(&segments[1], "segment_initial_pc"),
        &[Felt::from_canonical(ENTRY + 24).expect("canonical")]
    );
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_dma_memset_add_fill_zero() {
    let dir = temp_dir("guest-zisk-main-dma-memset-add-fill-zero");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(12, 0, 8),
        csrrs(0, 0x0816, 10),
        add(13, 10, 12),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(6);
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
    .expect("guest trace should write Zisk Main DMA memset add pair");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        trace.value(5, 2),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 4),
        Some(Felt::from_canonical(0xa000_0000).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 8),
        Some(Felt::from_canonical(0xd9).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 13),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 18),
        Some(Felt::from_canonical(13).expect("canonical"))
    );
    assert_eq!(
        trace.value(5, 27),
        Some(Felt::from_canonical(0).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_dma_memcmp_result() {
    let dir = temp_dir("guest-zisk-main-dma-memcmp-result");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(11, 10, 16),
        addi(5, 0, 1),
        sb(10, 5, 0),
        addi(12, 0, 16),
        csrrs(0, 0x0814, 11),
        add(13, 10, 12),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(9);
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
    .expect("guest trace should write Zisk Main DMA memcmp pair");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        trace.value(7, 2),
        Some(Felt::from_canonical(16).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 21),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 2),
        Some(Felt::from_canonical(0xa000_0010).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 4),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 8),
        Some(Felt::from_canonical(0xd1).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 13),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(8, 18),
        Some(Felt::from_canonical(13).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_reports_input_and_rom_proof_values() {
    let dir = temp_dir("guest-pc-proof-values-input-rom");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(5, ZISK_INPUT_ADDRESS as u32),
        addi(6, 5, 16),
        csrs(0x08f0, 6),
        csrwi(0x08c0, ZISK_INPUT_READY_FCALL_ID as u8),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let output = run_witness_trace_output_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: None,
        },
        WitnessTraceRequest::new(framed_stdin_chunk(&[7]), 4, 2),
    )
    .expect("guest PC trace should report proof values");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(proof_value(&output, "enable_rom_data"), &[Felt::ONE]);
    assert_eq!(proof_value(&output, "enable_input_data"), &[Felt::ONE]);
    assert_eq!(
        proof_value(&output, "enable_dma_64_aligned_memcpy"),
        &[Felt::ZERO]
    );
    assert_eq!(proof_value(&output, "enable_dma_unaligned"), &[Felt::ZERO]);
}

#[test]
fn guest_pc_trace_backend_reports_aligned_dma_proof_values() {
    let dir = temp_dir("guest-pc-proof-values-dma");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(11, 10, 16),
        addi(12, 0, 16),
        csrrs(0, 0x0813, 11),
        add(13, 10, 12),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let output = run_witness_trace_output_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: None,
        },
        WitnessTraceRequest::new(Vec::new(), 7, 2),
    )
    .expect("guest PC trace should report DMA proof values");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(proof_value(&output, "enable_rom_data"), &[Felt::ONE]);
    assert_eq!(proof_value(&output, "enable_input_data"), &[Felt::ZERO]);
    assert_eq!(
        proof_value(&output, "enable_dma_64_aligned_memcpy"),
        &[Felt::ONE]
    );
    assert_eq!(proof_value(&output, "enable_dma_unaligned"), &[Felt::ZERO]);
}

#[test]
fn guest_pc_trace_backend_reports_unaligned_dma_proof_values() {
    let dir = temp_dir("guest-pc-proof-values-unaligned-dma");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(11, 10, 17),
        addi(12, 0, 16),
        csrrs(0, 0x0813, 11),
        add(13, 10, 12),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let output = run_witness_trace_output_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: None,
        },
        WitnessTraceRequest::new(Vec::new(), 7, 2),
    )
    .expect("guest PC trace should report unaligned DMA proof values");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        proof_value(&output, "enable_dma_64_aligned_memcpy"),
        &[Felt::ZERO]
    );
    assert_eq!(proof_value(&output, "enable_dma_unaligned"), &[Felt::ONE]);
}

#[test]
fn guest_pc_trace_backend_uses_effective_memcmp_count_for_proof_values() {
    let dir = temp_dir("guest-pc-proof-values-memcmp-effective");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(11, 10, 16),
        addi(5, 0, 1),
        sb(11, 5, 0),
        addi(12, 0, 16),
        csrrs(0, 0x0814, 11),
        add(13, 10, 12),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let output = run_witness_trace_output_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: None,
        },
        WitnessTraceRequest::new(Vec::new(), 9, 2),
    )
    .expect("guest PC trace should classify memcmp from effective count");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        proof_value(&output, "enable_dma_64_aligned_mem"),
        &[Felt::ZERO]
    );
    assert_eq!(proof_value(&output, "enable_dma_unaligned"), &[Felt::ZERO]);
}

#[test]
fn guest_pc_trace_backend_treats_memcmp_boundary_mismatch_as_post_work() {
    let dir = temp_dir("guest-pc-proof-values-memcmp-boundary");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(10, 0xa000_0000),
        slli(10, 10, 32),
        srli(10, 10, 32),
        addi(11, 10, 16),
        addi(5, 0, 1),
        sb(11, 5, 7),
        addi(12, 0, 16),
        csrrs(0, 0x0814, 11),
        add(13, 10, 12),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let output = run_witness_trace_output_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: None,
        },
        WitnessTraceRequest::new(Vec::new(), 9, 2),
    )
    .expect("guest PC trace should classify memcmp boundary mismatch");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        proof_value(&output, "enable_dma_64_aligned_mem"),
        &[Felt::ZERO]
    );
    assert_eq!(proof_value(&output, "enable_dma_unaligned"), &[Felt::ZERO]);
}

#[test]
fn guest_pc_trace_backend_maps_zisk_ram_as_zeroed_writable_memory() {
    let dir = temp_dir("guest-pc-trace-zisk-ram");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let zisk_ram_address = 0xa043_0820_u64;
    let guest_image_bytes = sample_guest_image_with_words(&[
        lui(5, 0xa043_1000),
        slli(5, 5, 32),
        srli(5, 5, 32),
        addi(5, 5, -2016),
        ld(6, 5, 0),
        addi(7, 0, 9),
        sd(5, 7, 0),
        ld(8, 5, 0),
        0x0000_0073,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_trace_columns(
        8,
        vec![6],
        vec![
            commitment_column("mem_read_address", 1, 0, 1),
            commitment_column("mem_read_value", 1, 1, 1),
            commitment_column("mem_read_byte_len", 1, 2, 1),
            commitment_column("mem_write_address", 1, 3, 1),
            commitment_column("mem_write_value", 1, 4, 1),
            commitment_column("mem_write_byte_len", 1, 5, 1),
        ],
    );
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
    .expect("guest trace should map Zisk RAM");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        trace.value(4, 0),
        Some(Felt::from_canonical(zisk_ram_address).expect("canonical"))
    );
    assert_eq!(trace.value(4, 1), Some(Felt::ZERO));
    assert_eq!(
        trace.value(4, 2),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 3),
        Some(Felt::from_canonical(zisk_ram_address).expect("canonical"))
    );
    assert_eq!(
        trace.value(6, 4),
        Some(Felt::from_canonical(9).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 0),
        Some(Felt::from_canonical(zisk_ram_address).expect("canonical"))
    );
    assert_eq!(
        trace.value(7, 1),
        Some(Felt::from_canonical(9).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_requires_guest_image_context() {
    let error = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext::empty(),
        WitnessTraceRequest::new(Vec::new(), 1, 2),
    )
    .expect_err("missing guest image context should reject");

    assert!(error.to_string().contains("missing guest image path"));
}

#[test]
fn guest_pc_trace_backend_maps_trace_overflow_to_output_overflow() {
    let dir = temp_dir("guest-pc-trace-overflow");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let result = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: None,
        },
        WitnessTraceRequest::new(Vec::new(), 1, 2),
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(WitnessTraceRunError::Call(
            WitnessCallError::OutputOverflow {
                produced_len: 32,
                output_len: 16
            }
        ))
    ));
}

#[test]
fn guest_pc_trace_backend_reports_layout_capacity_before_larger_instruction_limit() {
    let dir = temp_dir("guest-pc-trace-layout-capacity");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 1), bne(1, 0, -4), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_trace_columns(
        2,
        vec![2],
        vec![
            commitment_column("pc", 1, 0, 1),
            commitment_column("next_pc", 1, 1, 1),
        ],
    );
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let result = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    match result.expect_err("layout capacity should reject") {
        WitnessTraceRunError::Call(WitnessCallError::Backend { message }) => assert_eq!(
            message,
            "guest PC trace backend exceeded trace layout capacity: rows 2, row width 2, required rows at least 3, required same-capacity trace instances at least 2"
        ),
        other => panic!("unexpected layout capacity result: {other:?}"),
    }
}

#[test]
fn guest_pc_trace_backend_preserves_raw_pc_pair_layout_compatibility() {
    let dir = temp_dir("guest-pc-trace-raw-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_trace_columns(2, vec![2], Vec::new());
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
    .expect("legacy raw PC pair layout should run");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 2);
    assert_eq!(trace.column_count(), 2);
    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(ENTRY).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 0),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 1),
        Some(Felt::from_canonical(ENTRY + 8).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_reports_layout_capacity_without_named_guest_columns() {
    let dir = temp_dir("guest-pc-trace-raw-layout-capacity");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 1), bne(1, 0, -4), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_trace_columns(2, vec![3], Vec::new());
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let result = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let error = result.expect_err("unmapped layout should be rejected");
    assert!(error
        .to_string()
        .contains("does not expose guest trace columns"));
}

#[test]
fn guest_pc_trace_backend_rejects_named_layout_without_guest_columns() {
    let dir = temp_dir("guest-pc-trace-unmapped-named-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[addi(1, 0, 7), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit =
        sample_unit_with_trace_columns(2, vec![3], vec![commitment_column("unrelated", 1, 0, 1)]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let result = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let error = result.expect_err("unmapped named layout should be rejected");
    assert!(error
        .to_string()
        .contains("does not expose guest trace columns"));
}

#[test]
fn guest_pc_trace_backend_writes_named_columns_from_layout() {
    let dir = temp_dir("guest-pc-trace-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_pc_columns();
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
    .expect("guest PC trace should write layout columns");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 3);
    assert_eq!(trace.column_count(), 4);
    assert_eq!(trace.value(0, 0), Some(Felt::ZERO));
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(ENTRY).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 2),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(trace.value(0, 3), Some(Felt::ZERO));
    assert_eq!(
        trace.value(1, 1),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 2),
        Some(Felt::from_canonical(ENTRY + 8).expect("canonical"))
    );
    assert_eq!(trace.value(2, 0), Some(Felt::ZERO));
    assert_eq!(trace.value(2, 1), Some(Felt::ZERO));
    assert_eq!(trace.value(2, 2), Some(Felt::ZERO));
    assert_eq!(trace.value(2, 3), Some(Felt::ZERO));
}

#[test]
fn guest_pc_trace_backend_writes_guest_effect_columns_from_layout() {
    let dir = temp_dir("guest-effect-trace-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = ENTRY + 24;
    let guest_image_bytes = sample_guest_image_with_words(&[
        auipc(1, 0),
        addi(1, 1, 24),
        lbu(2, 1, 0),
        addi(3, 0, 9),
        sb(1, 3, 0),
        0x0000_0073,
        5,
    ]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_guest_effect_columns();
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
    .expect("guest trace should write effect columns");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 6);
    assert_eq!(trace.column_count(), 10);
    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(ENTRY).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(4, 0),
        Some(Felt::from_canonical(ENTRY + 16).expect("canonical"))
    );
    assert_eq!(
        trace.value(4, 1),
        Some(Felt::from_canonical(ENTRY + 20).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 2),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 3),
        Some(Felt::from_canonical(ENTRY).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 3),
        Some(Felt::from_canonical(data_address).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 2),
        Some(Felt::from_canonical(2).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 3),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 4),
        Some(Felt::from_canonical(data_address).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 5),
        Some(Felt::from_canonical(5).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 6),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(4, 7),
        Some(Felt::from_canonical(data_address).expect("canonical"))
    );
    assert_eq!(
        trace.value(4, 8),
        Some(Felt::from_canonical(9).expect("canonical"))
    );
    assert_eq!(
        trace.value(4, 9),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(trace.value(5, 0), Some(Felt::ZERO));
    assert_eq!(trace.value(5, 9), Some(Felt::ZERO));
}

#[test]
fn guest_pc_trace_backend_writes_effect_only_layout_without_pc_columns() {
    let dir = temp_dir("guest-effect-only-trace-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[addi(1, 0, 7), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_register_effect_columns();
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
    .expect("guest trace should write effect-only layout columns");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 2);
    assert_eq!(trace.column_count(), 2);
    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(7).expect("canonical"))
    );
    assert_eq!(trace.value(1, 0), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 1), Some(Felt::ZERO));
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_address_columns() {
    let dir = temp_dir("guest-zisk-main-address-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[addi(1, 0, 7), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(2);
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
    .expect("guest trace should write Zisk Main address columns");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 2);
    assert_eq!(trace.column_count(), 29);
    assert_eq!(
        trace.value(0, 16),
        Some(Felt::from_canonical(7).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 18),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 19),
        Some(Felt::from_canonical(7).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 20),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_writes_indirect_zisk_main_address_columns() {
    let dir = temp_dir("guest-zisk-main-indirect-address-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let code_words = [addi(1, 0, 64), ld(2, 1, 8), sd(1, 2, -8), 0x0000_0073];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_address = 56_u64;
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = vec![0_u8; 24];
    data[16..24].copy_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
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
    let unit = sample_unit_with_zisk_main_address_columns(4);
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
    .expect("guest trace should write indirect Zisk Main address columns");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        trace.value(1, 15),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 16),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 19),
        Some(Felt::from_canonical(72).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 17),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 20),
        Some(Felt::from_canonical(56).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_negative_immediate_limbs() {
    let dir = temp_dir("guest-zisk-main-negative-immediate-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[lui(15, 4096), addi(15, 15, -1878), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(4);
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
    .expect("guest trace should write negative immediate limbs");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let low = 0xffff_f8aa_u64;
    assert_eq!(
        trace.value(1, 2),
        Some(Felt::from_canonical(low).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 3),
        Some(Felt::from_canonical(0xffff_ffff).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 16),
        Some(Felt::from_canonical(low).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 24),
        Some(Felt::from_canonical(0xffff_ffff).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 25),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_pads_zisk_main_rows_with_terminal_copy() {
    let dir = temp_dir("guest-zisk-main-terminal-padding-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[addi(1, 0, 7), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_address_columns(4);
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
    .expect("guest trace should pad Zisk Main rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 4);
    assert_eq!(trace.value(1, 0), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 2), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 4), Some(Felt::ZERO));
    assert_eq!(
        trace.value(1, 7),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 8),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 14),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(trace.value(1, 23), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 24), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 25), Some(Felt::ZERO));
    assert_eq!(
        trace.value(1, 26),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(trace.value(1, 27), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 28), Some(Felt::ZERO));
    assert_eq!(trace.value(2, 0), Some(Felt::ZERO));
    assert_eq!(trace.value(2, 4), Some(Felt::ZERO));
    assert_eq!(
        trace.value(2, 7),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 8),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(2, 14),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(trace.value(2, 25), Some(Felt::ZERO));
    assert_eq!(
        trace.value(2, 26),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_reports_zisk_main_unit_values_and_register_steps() {
    let dir = temp_dir("guest-zisk-main-unit-values");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_register_step_columns(4);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let output = run_witness_trace_output_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("guest trace output should include Zisk Main unit values");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let trace = output.trace();
    assert_eq!(trace.value(1, 29), Some(Felt::from_u64(3)));
    assert_eq!(trace.value(1, 30), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 31), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 32), Some(Felt::ZERO));
    assert_eq!(trace.value(1, 33), Some(Felt::ZERO));

    assert_eq!(unit_value(&output, "main_last_segment"), &[Felt::ONE]);
    assert_eq!(unit_value(&output, "main_segment"), &[Felt::ZERO]);
    assert_eq!(
        unit_value(&output, "segment_initial_pc"),
        &[Felt::from_canonical(ENTRY).expect("canonical")]
    );
    assert_eq!(
        unit_value(&output, "segment_previous_c"),
        &[Felt::ZERO, Felt::ZERO]
    );
    assert_eq!(
        unit_value(&output, "segment_next_pc"),
        &[Felt::from_canonical(ENTRY + 8).expect("canonical")]
    );
    assert_eq!(
        unit_value(&output, "segment_last_c"),
        &[Felt::ZERO, Felt::ZERO]
    );

    let last_reg_value = unit_value(&output, "last_reg_value");
    assert_eq!(last_reg_value[0..2], [Felt::from_u64(7), Felt::ZERO]);
    assert_eq!(last_reg_value[2..4], [Felt::from_u64(10), Felt::ZERO]);
    assert!(last_reg_value[4..].iter().all(|value| *value == Felt::ZERO));

    let last_reg_mem_step = unit_value(&output, "last_reg_mem_step");
    assert_eq!(last_reg_mem_step[0], Felt::from_u64(5));
    assert_eq!(last_reg_mem_step[1], Felt::from_u64(7));
    assert!(last_reg_mem_step[2..]
        .iter()
        .all(|value| *value == Felt::ZERO));
}

#[test]
fn guest_pc_trace_backend_splits_zisk_main_segments() {
    let dir = temp_dir("guest-zisk-main-segments");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), addi(3, 2, 5), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_register_step_columns(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let segments = run_guest_pc_trace_segments_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("guest trace should split Zisk Main output");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].trace_instance_index(), 0);
    assert_eq!(segments[1].trace_instance_index(), 1);

    let first = &segments[0];
    assert_eq!(first.trace().row_count(), 2);
    assert_eq!(
        first.trace().value(0, 7),
        Some(Felt::from_canonical(ENTRY).expect("canonical"))
    );
    assert_eq!(
        first.trace().value(1, 7),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(unit_value(first, "main_last_segment"), &[Felt::ZERO]);
    assert_eq!(unit_value(first, "main_segment"), &[Felt::ZERO]);
    assert_eq!(
        unit_value(first, "segment_initial_pc"),
        &[Felt::from_canonical(ENTRY).expect("canonical")]
    );
    assert_eq!(
        unit_value(first, "segment_next_pc"),
        &[Felt::from_canonical(ENTRY + 8).expect("canonical")]
    );
    assert_eq!(
        unit_value(first, "segment_last_c"),
        &[Felt::from_u64(10), Felt::ZERO]
    );
    assert_eq!(proof_value(first, "enable_rom_data"), &[Felt::ONE]);
    assert_eq!(proof_value(first, "enable_input_data"), &[Felt::ZERO]);
    let first_last_reg_mem_step = unit_value(first, "last_reg_mem_step");
    assert_eq!(first_last_reg_mem_step[0], Felt::from_u64(5));
    assert_eq!(first_last_reg_mem_step[1], Felt::from_u64(7));

    let second = &segments[1];
    assert_eq!(
        second.trace().value(0, 7),
        Some(Felt::from_canonical(ENTRY + 8).expect("canonical"))
    );
    assert_eq!(second.trace().value(0, 29), Some(Felt::from_u64(8)));
    assert_eq!(
        second.trace().value(1, 7),
        Some(Felt::from_canonical(ENTRY + 12).expect("canonical"))
    );
    assert_eq!(unit_value(second, "main_last_segment"), &[Felt::ONE]);
    assert_eq!(unit_value(second, "main_segment"), &[Felt::ONE]);
    assert_eq!(
        unit_value(second, "segment_initial_pc"),
        &[Felt::from_canonical(ENTRY + 8).expect("canonical")]
    );
    assert_eq!(
        unit_value(second, "segment_previous_c"),
        &[Felt::from_u64(10), Felt::ZERO]
    );
    assert_eq!(
        unit_value(second, "segment_next_pc"),
        &[Felt::from_canonical(ENTRY + 12).expect("canonical")]
    );
    assert_eq!(
        unit_value(second, "segment_last_c"),
        &[Felt::ZERO, Felt::ZERO]
    );
    assert_eq!(second.proof_values(), first.proof_values());

    let last_reg_value = unit_value(second, "last_reg_value");
    assert_eq!(last_reg_value[0..2], [Felt::from_u64(7), Felt::ZERO]);
    assert_eq!(last_reg_value[2..4], [Felt::from_u64(10), Felt::ZERO]);
    assert_eq!(last_reg_value[4..6], [Felt::from_u64(15), Felt::ZERO]);

    let last_reg_mem_step = unit_value(second, "last_reg_mem_step");
    assert_eq!(last_reg_mem_step[0], Felt::from_u64(8));
    assert_eq!(last_reg_mem_step[1], Felt::from_u64(9));
    assert_eq!(last_reg_mem_step[2], Felt::from_u64(11));
}

#[test]
fn guest_pc_trace_backend_single_zisk_main_output_reports_capacity() {
    let dir = temp_dir("guest-zisk-main-single-capacity");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), addi(3, 2, 5), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_register_step_columns(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let result = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    match result.expect_err("single Zisk Main output should still report capacity") {
        WitnessTraceRunError::Call(WitnessCallError::Backend { message }) => assert_eq!(
            message,
            "guest PC trace backend exceeded trace layout capacity: rows 2, row width 40, required rows at least 3, required same-capacity trace instances at least 2"
        ),
        other => panic!("unexpected single Zisk Main capacity result: {other:?}"),
    }
}

#[test]
fn guest_pc_trace_segments_reject_request_shape_mismatch() {
    let unit = sample_unit_with_zisk_main_register_step_columns(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let result = run_guest_pc_trace_segments_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: None,
            guest_image_info: None,
            trace_layout: Some(&layout),
        },
        WitnessTraceRequest::new(Vec::new(), 4, 20),
    );

    match result.expect_err("segmented request should reject mismatched shape") {
        WitnessTraceRunError::Call(WitnessCallError::Backend { message }) => assert_eq!(
            message,
            "guest PC trace segmented request shape mismatch: layout 2x40, request 4x20"
        ),
        other => panic!("unexpected segmented request shape result: {other:?}"),
    }
}

#[test]
fn guest_pc_trace_backend_single_full_zisk_main_output_preserves_contract() {
    let dir = temp_dir("guest-zisk-main-single-full-contract");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), addi(3, 2, 5), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_register_step_columns(3);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let output = run_witness_trace_output_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("single full Zisk Main output should preserve current contract");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.trace().row_count(), 3);
    assert_eq!(unit_value(&output, "main_last_segment"), &[Felt::ONE]);
    assert_eq!(unit_value(&output, "main_segment"), &[Felt::ZERO]);
    assert_eq!(
        unit_value(&output, "segment_next_pc"),
        &[Felt::from_canonical(ENTRY + 12).expect("canonical")]
    );
    assert_eq!(
        unit_value(&output, "segment_last_c"),
        &[Felt::from_u64(15), Felt::ZERO]
    );
}

#[test]
fn guest_pc_trace_backend_adds_terminal_segment_when_final_segment_is_full() {
    let dir = temp_dir("guest-zisk-main-full-final-segment");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), addi(3, 2, 5), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_register_step_columns(3);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let segments = run_guest_pc_trace_segments_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("guest trace should add a terminal Zisk Main output");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(segments.len(), 2);
    let first = &segments[0];
    assert_eq!(unit_value(first, "main_last_segment"), &[Felt::ZERO]);
    assert_eq!(
        unit_value(first, "segment_next_pc"),
        &[Felt::from_canonical(ENTRY + 12).expect("canonical")]
    );
    assert_eq!(
        unit_value(first, "segment_last_c"),
        &[Felt::from_u64(15), Felt::ZERO]
    );

    let terminal = &segments[1];
    assert_eq!(segments[1].trace_instance_index(), 1);
    assert_eq!(terminal.proof_values(), first.proof_values());
    assert_eq!(
        terminal.trace().value(0, 7),
        Some(Felt::from_canonical(ENTRY + 12).expect("canonical"))
    );
    assert_eq!(terminal.trace().value(0, 4), Some(Felt::ZERO));
    assert_eq!(unit_value(terminal, "main_last_segment"), &[Felt::ONE]);
    assert_eq!(unit_value(terminal, "main_segment"), &[Felt::ONE]);
    assert_eq!(
        unit_value(terminal, "segment_initial_pc"),
        &[Felt::from_canonical(ENTRY + 12).expect("canonical")]
    );
    assert_eq!(
        unit_value(terminal, "segment_previous_c"),
        &[Felt::from_u64(15), Felt::ZERO]
    );
    assert_eq!(
        unit_value(terminal, "segment_next_pc"),
        &[Felt::from_canonical(ENTRY + 12).expect("canonical")]
    );
    assert_eq!(
        unit_value(terminal, "segment_last_c"),
        &[Felt::ZERO, Felt::ZERO]
    );
    let last_reg_mem_step = unit_value(terminal, "last_reg_mem_step");
    assert_eq!(last_reg_mem_step[0], Felt::from_u64(12));
    assert_eq!(last_reg_mem_step[1], Felt::from_u64(12));
    assert_eq!(last_reg_mem_step[2], Felt::from_u64(12));
}

trait TraceOutputValues {
    fn unit_values(&self) -> &[WitnessTraceUnitValue];
    fn proof_values(&self) -> &[WitnessTraceProofValue];
}

impl TraceOutputValues for WitnessTraceRunOutput {
    fn unit_values(&self) -> &[WitnessTraceUnitValue] {
        self.unit_values()
    }

    fn proof_values(&self) -> &[WitnessTraceProofValue] {
        self.proof_values()
    }
}

impl TraceOutputValues for GuestPcTraceSegmentRunOutput {
    fn unit_values(&self) -> &[WitnessTraceUnitValue] {
        self.unit_values()
    }

    fn proof_values(&self) -> &[WitnessTraceProofValue] {
        self.proof_values()
    }
}

fn unit_value<'a>(output: &'a impl TraceOutputValues, name: &str) -> &'a [Felt] {
    output
        .unit_values()
        .iter()
        .find(|value| value.name() == name)
        .unwrap_or_else(|| panic!("missing unit value {name}"))
        .values()
}

fn proof_value<'a>(output: &'a impl TraceOutputValues, name: &str) -> &'a [Felt] {
    output
        .proof_values()
        .iter()
        .find(|value| value.name() == name)
        .unwrap_or_else(|| panic!("missing proof value {name}"))
        .values()
}

#[test]
fn guest_pc_trace_backend_rejects_zisk_rom_layout_without_program_trace_context() {
    let dir = temp_dir("guest-zisk-rom-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[addi(1, 0, 7), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_rom_columns(4);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let result = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let error = result.expect_err("Zisk Rom layout should require program trace context");
    assert!(error
        .to_string()
        .contains("does not expose guest trace columns"));
}

#[test]
fn guest_pc_trace_backend_writes_precompile_memory_access_layout() {
    let dir = temp_dir("guest-precompile-memory-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let params_address = data_address;
    let a_address = data_address + 32;
    let b_address = a_address + 32;
    let out_address = b_address + 32;
    let code_words = [
        addi(1, 0, params_address as i16),
        csrrs(2, 0x0811, 1),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&a_address.to_le_bytes());
    data.extend_from_slice(&b_address.to_le_bytes());
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.extend_from_slice(&out_address.to_le_bytes());
    for _ in 0..4 {
        data.extend_from_slice(&u64::MAX.to_le_bytes());
    }
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.extend_from_slice(&[0; 24]);
    data.extend_from_slice(&[0; 32]);
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
    let unit = sample_unit_with_trace_columns(
        17,
        vec![7],
        vec![
            commitment_column("precompile_mem_main_step", 1, 0, 1),
            commitment_column("precompile_mem_is_write", 1, 1, 1),
            commitment_column("precompile_mem_address", 1, 2, 1),
            commitment_column("precompile_mem_value", 1, 3, 2),
            commitment_column("precompile_mem_byte_len", 1, 5, 1),
            commitment_column("precompile_mem_selector", 1, 6, 1),
        ],
    );
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
    .expect("guest trace should write precompile memory access columns");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 17);
    assert_eq!(trace.column_count(), 7);
    assert_precompile_mem_row(&trace, 0, 1, false, params_address, a_address);
    assert_precompile_mem_row(&trace, 3, 1, false, params_address + 24, out_address);
    assert_precompile_mem_row(&trace, 4, 1, false, a_address, u64::MAX);
    assert_precompile_mem_row(&trace, 8, 1, false, b_address, 1);
    assert_precompile_mem_row(&trace, 12, 1, true, out_address, 1);
    assert_precompile_mem_row(&trace, 15, 1, true, out_address + 24, 0);
    assert_eq!(trace.value(16, 0), Some(Felt::ZERO));
    assert_eq!(trace.value(16, 1), Some(Felt::ZERO));
    assert_eq!(trace.value(16, 2), Some(Felt::ZERO));
    assert_eq!(trace.value(16, 3), Some(Felt::ZERO));
    assert_eq!(trace.value(16, 4), Some(Felt::ZERO));
    assert_eq!(trace.value(16, 5), Some(Felt::ZERO));
    assert_eq!(trace.value(16, 6), Some(Felt::ZERO));
}

#[test]
fn precompile_memory_trace_uses_expanded_main_step_after_store_conditional() {
    let dir = temp_dir("guest-precompile-memory-expanded-main-step");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let params_address = data_address;
    let a_address = data_address + 32;
    let b_address = a_address + 32;
    let out_address = b_address + 32;
    let scratch_address = out_address + 32;
    let code_words = [
        addi(10, 0, scratch_address as i16),
        addi(7, 0, 9),
        lr_d(14, 10),
        sc_d(12, 10, 7),
        addi(1, 0, params_address as i16),
        csrrs(2, 0x0811, 1),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&a_address.to_le_bytes());
    data.extend_from_slice(&b_address.to_le_bytes());
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.extend_from_slice(&out_address.to_le_bytes());
    for _ in 0..4 {
        data.extend_from_slice(&u64::MAX.to_le_bytes());
    }
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.extend_from_slice(&[0; 24]);
    data.extend_from_slice(&[0; 32]);
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
    let unit = sample_unit_with_trace_columns(
        17,
        vec![7],
        vec![
            commitment_column("precompile_mem_main_step", 1, 0, 1),
            commitment_column("precompile_mem_is_write", 1, 1, 1),
            commitment_column("precompile_mem_address", 1, 2, 1),
            commitment_column("precompile_mem_value", 1, 3, 2),
            commitment_column("precompile_mem_byte_len", 1, 5, 1),
            commitment_column("precompile_mem_selector", 1, 6, 1),
        ],
    );
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
    .expect("guest trace should account for expanded Main rows before precompile memory accesses");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_precompile_mem_row(&trace, 0, 6, false, params_address, a_address);
    assert_precompile_mem_row(&trace, 3, 6, false, params_address + 24, out_address);
    assert_precompile_mem_row(&trace, 12, 6, true, out_address, 1);
    assert_precompile_mem_row(&trace, 15, 6, true, out_address + 24, 0);
}

fn assert_precompile_mem_row(
    trace: &lzvm_prover::witness_trace::WitnessTraceBuffer,
    row: usize,
    main_step: u64,
    is_write: bool,
    address: u64,
    value: u64,
) {
    assert_eq!(
        trace.value(row, 0),
        Some(Felt::from_canonical(main_step).expect("canonical"))
    );
    assert_eq!(
        trace.value(row, 1),
        Some(Felt::from_canonical(u64::from(is_write)).expect("canonical"))
    );
    assert_eq!(
        trace.value(row, 2),
        Some(Felt::from_canonical(address).expect("canonical"))
    );
    assert_eq!(
        trace.value(row, 3),
        Some(Felt::from_canonical(value & 0xffff_ffff).expect("canonical"))
    );
    assert_eq!(
        trace.value(row, 4),
        Some(Felt::from_canonical(value >> 32).expect("canonical"))
    );
    assert_eq!(
        trace.value(row, 5),
        Some(Felt::from_canonical(8).expect("canonical"))
    );
    assert_eq!(
        trace.value(row, 6),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
}

#[test]
fn guest_pc_trace_backend_rejects_partial_layout_pc_columns() {
    let dir = temp_dir("guest-pc-trace-partial-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_trace_columns(2, vec![2], vec![commitment_column("pc", 1, 0, 1)]);
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
    .expect_err("partial PC layout should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(error.to_string().contains("missing next_pc"));
}

#[test]
fn guest_pc_trace_backend_rejects_misshaped_layout_pc_columns() {
    let dir = temp_dir("guest-pc-trace-misshaped-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_trace_columns(
        2,
        vec![3],
        vec![
            commitment_column("pc", 1, 0, 2),
            commitment_column("next_pc", 1, 2, 1),
        ],
    );
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
    .expect_err("misshaped PC layout should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let message = error.to_string();
    assert!(message.contains("pc"));
    assert!(message.contains("dimension 1"));
}

#[test]
fn guest_pc_trace_backend_rejects_partial_register_effect_columns() {
    let dir = temp_dir("guest-reg-effect-partial-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[addi(1, 0, 7), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_trace_columns(
        1,
        vec![3],
        vec![
            commitment_column("pc", 1, 0, 1),
            commitment_column("next_pc", 1, 1, 1),
            commitment_column("reg_write_index", 1, 2, 1),
        ],
    );
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
    .expect_err("partial register effect layout should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(error.to_string().contains("missing reg_write_value"));
}

#[test]
fn guest_pc_trace_backend_rejects_partial_memory_effect_columns() {
    let dir = temp_dir("guest-mem-effect-partial-layout");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[auipc(1, 0), addi(1, 1, 16), lbu(2, 1, 0), 0x0000_0073, 5]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_trace_columns(
        3,
        vec![4],
        vec![
            commitment_column("pc", 1, 0, 1),
            commitment_column("next_pc", 1, 1, 1),
            commitment_column("mem_read_address", 1, 2, 1),
            commitment_column("mem_read_byte_len", 1, 3, 1),
        ],
    );
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
    .expect_err("partial memory effect layout should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(error.to_string().contains("missing mem_read_value"));
}

#[test]
fn propagates_trace_parse_errors_from_native_output() {
    let dir = temp_dir("noncanonical");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "noncanonical",
        r#"#include <stddef.h>
typedef struct {
    const unsigned char *input_ptr;
    size_t input_len;
    unsigned char *output_ptr;
    size_t output_len;
} LzvmWitnessCall;
typedef struct {
    int status;
    size_t produced_len;
} LzvmWitnessResult;
static void write_u64_le(unsigned char *out, unsigned long long value) {
    for (size_t i = 0; i < 8; i++) {
        out[i] = (unsigned char)((value >> (i * 8)) & 0xff);
    }
}
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    (void)call;
    if (!call || !result || call->output_len < 8) {
        return -1;
    }
    write_u64_le(call->output_ptr, 0xffffffff00000001ULL);
    result->status = 0;
    result->produced_len = 8;
    return 0;
}
"#,
    );
    let library = load_witness_library(&library_path).expect("witness library should load");

    let result = run_witness_trace(&library, WitnessTraceRequest::new(Vec::new(), 1, 1));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(WitnessTraceRunError::Trace(
            WitnessTraceError::NonCanonicalElement {
                index: 0,
                value: MODULUS
            }
        ))
    ));
}

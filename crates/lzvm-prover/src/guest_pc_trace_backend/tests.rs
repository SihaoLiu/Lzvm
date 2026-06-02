use super::*;
use crate::guest_instruction::{RiscvInstruction, RiscvOpImmKind, RiscvPrecompileKind};
use crate::witness_layout::derive_witness_trace_layout;
use crate::witness_trace::parse_witness_trace;
use crate::ProveUnitSchedule;
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_field::Felt;

#[test]
fn rejects_add256_precompile_memory_access_address_mismatch() {
    let mut report = add256_report();
    report.precompile_memory_accesses[4].address += 8;

    let error = validate_zisk_main_precompile_memory_accesses(
        3,
        &report,
        ZiskMainReportEffects::from_report(&report),
        64,
    )
    .expect_err("mismatched Add256 precompile memory access should fail");

    assert!(error.to_string().contains("precompile memory access 4"));
}

#[test]
fn builds_zisk_main_segment_trace_without_serialized_roundtrip() {
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let report = addi_report();

    let written = build_layout_zisk_main_trace_segment(
        &layout,
        std::slice::from_ref(&report),
        report.next_pc,
        &ZiskMainTraceState::new(),
        None,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: true,
            previous_c: 0,
        },
    )
    .expect("segment trace should build")
    .expect("Zisk Main layout should be supported");

    assert_eq!(written.trace.row_count(), layout.row_count());
    assert_eq!(written.trace.column_count(), layout.column_count());
    assert_eq!(
        written.output.produced_len,
        written.trace.values().len() * std::mem::size_of::<u64>()
    );
    let pc_column = layout.column(1, "pc").expect("pc column").trace_column();
    assert_eq!(
        written.trace.value(0, pc_column),
        Some(Felt::from_canonical(report.address).expect("canonical pc"))
    );

    let mut bytes = vec![0; written.output.produced_len];
    serialize_trace_to_output(&written.trace, written.output.produced_len, &mut bytes)
        .expect("trace should serialize");
    let parsed = parse_witness_trace(&bytes, layout.row_count(), layout.column_count())
        .expect("serialized trace should parse");
    assert_eq!(parsed, written.trace);
}

#[test]
fn zisk_main_trace_build_uses_resolved_column_targets() {
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let report = addi_report();

    crate::witness_layout::reset_column_lookup_count();
    let written = build_layout_zisk_main_trace_segment(
        &layout,
        std::slice::from_ref(&report),
        report.next_pc,
        &ZiskMainTraceState::new(),
        None,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: true,
            previous_c: 0,
        },
    )
    .expect("segment trace should build")
    .expect("Zisk Main layout should be supported");

    assert_eq!(written.trace.row_count(), layout.row_count());
    assert_eq!(crate::witness_layout::column_lookup_count(), 0);
}

fn add256_report() -> GuestMachineReport {
    let params_address = 64;
    let a_address = 96;
    let b_address = 128;
    let c_address = 160;
    let mut precompile_memory_accesses = vec![
        memory_read(params_address, a_address),
        memory_read(params_address + 8, b_address),
        memory_read(params_address + 16, 0),
        memory_read(params_address + 24, c_address),
    ];
    precompile_memory_accesses.extend([
        memory_read(a_address, u64::MAX),
        memory_read(a_address + 8, u64::MAX),
        memory_read(a_address + 16, u64::MAX),
        memory_read(a_address + 24, u64::MAX),
        memory_read(b_address, 1),
        memory_read(b_address + 8, 0),
        memory_read(b_address + 16, 0),
        memory_read(b_address + 24, 0),
        memory_write(c_address, 0),
        memory_write(c_address + 8, 0),
        memory_write(c_address + 16, 0),
        memory_write(c_address + 24, 0),
    ]);
    GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Add256,
            rs1: 1,
            rd: 2,
        },
        next_pc: 0x8000_0004,
        register_writes: vec![GuestRegisterWrite { index: 2, value: 1 }],
        memory_accesses: Vec::new(),
        precompile_memory_accesses,
        precompile_result: Some(1),
    }
}

fn addi_report() -> GuestMachineReport {
    GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 0,
            immediate: 7,
        },
        next_pc: 0x8000_0004,
        register_writes: vec![GuestRegisterWrite { index: 1, value: 7 }],
        memory_accesses: Vec::new(),
        precompile_memory_accesses: Vec::new(),
        precompile_result: None,
    }
}

fn memory_read(address: u64, value: u64) -> GuestMemoryAccess {
    GuestMemoryAccess {
        kind: GuestMemoryAccessKind::Read,
        address,
        byte_len: 8,
        value,
    }
}

fn memory_write(address: u64, value: u64) -> GuestMemoryAccess {
    GuestMemoryAccess {
        kind: GuestMemoryAccessKind::Write,
        address,
        byte_len: 8,
        value,
    }
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

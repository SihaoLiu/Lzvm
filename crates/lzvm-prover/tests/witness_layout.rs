use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_field::Felt;
use lzvm_prover::witness_layout::{derive_witness_trace_layout, WitnessTraceLayoutError};
use lzvm_prover::witness_trace::parse_witness_trace;
use lzvm_prover::ProveUnitSchedule;

fn encode_values(values: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 8);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn sample_unit(stage_commit_widths: Vec<u32>) -> ProveUnitSchedule {
    sample_unit_with_rows(stage_commit_widths, 1024)
}

fn sample_unit_with_rows(
    stage_commit_widths: Vec<u32>,
    base_domain_size: u64,
) -> ProveUnitSchedule {
    sample_unit_with_rows_and_columns(stage_commit_widths, base_domain_size, Vec::new())
}

fn sample_unit_with_rows_and_columns(
    stage_commit_widths: Vec<u32>,
    base_domain_size: u64,
    commitment_columns: Vec<CommitmentColumn>,
) -> ProveUnitSchedule {
    let mut transcript_root_challenge_draws = vec![1; stage_commit_widths.len()];
    if let Some(first) = transcript_root_challenge_draws.first_mut() {
        *first = 2;
    }
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits: 10,
        extended_domain_bits: 13,
        base_domain_size,
        extended_domain_size: 8192,
        blowup_factor: 8,
        query_count: 4,
        proof_of_work_bits: 20,
        merkle_tree_arity: 4,
        last_level_verification: 0,
        transcript_arity: Some(4),
        hash_commits: true,
        transcript_root_challenge_draws,
        challenge_count: 6,
        evaluation_value_count: 2,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 2,
        constant_width: 5,
        stage_commit_widths,
        commitment_columns,
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0, 1, -1],
        fri_layers: vec![PcsFriLayer {
            input_bits: 13,
            output_bits: 9,
            folding_factor: 16,
        }],
        final_layer_bits: 5,
        fixed_bytes: 40960,
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

#[test]
fn derives_witness_trace_layout_from_unit_schedule() {
    let unit = sample_unit(vec![2, 3, 1]);

    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    assert_eq!(layout.row_count(), 1024);
    assert_eq!(layout.column_count(), 6);
    assert_eq!(layout.stage_count(), 3);
    assert_eq!(layout.stages()[0].stage_index, 1);
    assert_eq!(layout.stages()[0].start_column, 0);
    assert_eq!(layout.stages()[0].width, 2);
    assert_eq!(layout.stages()[1].stage_index, 2);
    assert_eq!(layout.stages()[1].start_column, 2);
    assert_eq!(layout.stages()[1].width, 3);
    assert_eq!(layout.stages()[2].stage_index, 3);
    assert_eq!(layout.stages()[2].start_column, 5);
    assert_eq!(layout.stages()[2].width, 1);
}

#[test]
fn derives_witness_trace_column_positions_from_commitment_metadata() {
    let unit = sample_unit_with_rows_and_columns(
        vec![2, 3],
        1024,
        vec![
            commitment_column("pc", 1, 1, 1),
            commitment_column("mem_value", 2, 0, 1),
            commitment_column("ext_value", 2, 1, 2),
        ],
    );

    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    assert_eq!(layout.columns().len(), 3);
    assert_eq!(layout.columns()[0].name(), "pc");
    assert_eq!(layout.columns()[0].stage_index(), 1);
    assert_eq!(layout.columns()[0].stage_position(), 1);
    assert_eq!(layout.columns()[0].trace_column(), 1);
    assert_eq!(layout.columns()[0].dimension(), 1);
    let mem_value = layout
        .column(2, "mem_value")
        .expect("column should be indexed");
    assert_eq!(mem_value.trace_column(), 2);
    let ext_value = layout
        .column(2, "ext_value")
        .expect("column should be indexed");
    assert_eq!(ext_value.stage_position(), 1);
    assert_eq!(ext_value.trace_column(), 3);
    assert_eq!(ext_value.dimension(), 2);
}

#[test]
fn rejects_commitment_columns_outside_trace_layout() {
    let unknown_stage =
        sample_unit_with_rows_and_columns(vec![2], 1024, vec![commitment_column("pc", 2, 0, 1)]);
    assert!(matches!(
        derive_witness_trace_layout(&unknown_stage),
        Err(WitnessTraceLayoutError::CommitmentColumnStageOutOfRange {
            name,
            stage_index: 2,
            stage_count: 1,
        }) if name == "pc"
    ));

    let zero_stage =
        sample_unit_with_rows_and_columns(vec![2], 1024, vec![commitment_column("pc", 0, 0, 1)]);
    assert!(matches!(
        derive_witness_trace_layout(&zero_stage),
        Err(WitnessTraceLayoutError::CommitmentColumnStageOutOfRange {
            name,
            stage_index: 0,
            stage_count: 1,
        }) if name == "pc"
    ));

    let zero_dimension =
        sample_unit_with_rows_and_columns(vec![2], 1024, vec![commitment_column("empty", 1, 0, 0)]);
    assert!(matches!(
        derive_witness_trace_layout(&zero_dimension),
        Err(WitnessTraceLayoutError::ZeroCommitmentColumnDimension {
            name,
            stage_index: 1,
        }) if name == "empty"
    ));

    let out_of_range = sample_unit_with_rows_and_columns(
        vec![2],
        1024,
        vec![commitment_column("ext_value", 1, 1, 2)],
    );
    assert!(matches!(
        derive_witness_trace_layout(&out_of_range),
        Err(WitnessTraceLayoutError::CommitmentColumnPositionOutOfRange {
            name,
            stage_index: 1,
            stage_position: 1,
            dimension: 2,
            stage_width: 2,
        }) if name == "ext_value"
    ));
}

#[test]
fn builds_witness_trace_request_from_layout() {
    let unit = sample_unit(vec![2, 3, 1]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let request = layout.request(vec![7, 8, 9]);

    assert_eq!(request.input, vec![7, 8, 9]);
    assert_eq!(request.rows, 1024);
    assert_eq!(request.columns, 6);
}

#[test]
fn extracts_stage_values_from_row_major_trace() {
    let unit = sample_unit_with_rows(vec![2, 3, 1], 2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace = parse_witness_trace(
        &encode_values(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        2,
        6,
    )
    .expect("trace should parse");

    let stage = layout.stage_trace(&trace, 2).expect("stage should extract");

    assert_eq!(stage.stage_index(), 2);
    assert_eq!(stage.row_count(), 2);
    assert_eq!(stage.column_count(), 3);
    assert_eq!(
        stage.values(),
        &[
            Felt::from_canonical(3).expect("canonical"),
            Felt::from_canonical(4).expect("canonical"),
            Felt::from_canonical(5).expect("canonical"),
            Felt::from_canonical(9).expect("canonical"),
            Felt::from_canonical(10).expect("canonical"),
            Felt::from_canonical(11).expect("canonical"),
        ]
    );
}

#[test]
fn rejects_stage_extraction_for_mismatched_trace_shape() {
    let unit = sample_unit_with_rows(vec![2, 3, 1], 2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace =
        parse_witness_trace(&encode_values(&[1, 2, 3, 4]), 2, 2).expect("trace should parse");

    assert!(matches!(
        layout.stage_trace(&trace, 1),
        Err(WitnessTraceLayoutError::TraceShapeMismatch {
            expected_rows: 2,
            expected_columns: 6,
            found_rows: 2,
            found_columns: 2
        })
    ));
}

#[test]
fn rejects_empty_witness_trace_stage_sets() {
    let unit = sample_unit(Vec::new());

    assert!(matches!(
        derive_witness_trace_layout(&unit),
        Err(WitnessTraceLayoutError::EmptyStageSet)
    ));
}

#[test]
fn rejects_zero_width_witness_trace_stages() {
    let unit = sample_unit(vec![2, 0, 1]);

    assert!(matches!(
        derive_witness_trace_layout(&unit),
        Err(WitnessTraceLayoutError::ZeroStageWidth { stage_index: 2 })
    ));
}

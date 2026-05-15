use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_prover::witness_layout::{derive_witness_trace_layout, WitnessTraceLayoutError};
use lzvm_prover::ProveUnitSchedule;

fn sample_unit(stage_commit_widths: Vec<u32>) -> ProveUnitSchedule {
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits: 10,
        extended_domain_bits: 13,
        base_domain_size: 1024,
        extended_domain_size: 8192,
        blowup_factor: 8,
        query_count: 4,
        proof_of_work_bits: 20,
        merkle_tree_arity: 4,
        transcript_arity: Some(4),
        constant_width: 5,
        stage_commit_widths,
        opening_points: vec![0, 1, -1],
        fri_layers: vec![PcsFriLayer {
            input_bits: 13,
            output_bits: 9,
            folding_factor: 16,
        }],
        final_layer_bits: 5,
        fixed_bytes: 40960,
        constant_tree_root: None,
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
fn builds_witness_trace_request_from_layout() {
    let unit = sample_unit(vec![2, 3, 1]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let request = layout.request(vec![7, 8, 9]);

    assert_eq!(request.input, vec![7, 8, 9]);
    assert_eq!(request.rows, 1024);
    assert_eq!(request.columns, 6);
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

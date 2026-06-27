use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, parse_witness_opening_segment, WitnessOpeningLevelSegment,
    WitnessOpeningQuerySegment, WitnessOpeningSegment, WitnessOpeningSegmentError,
    WitnessOpeningStageSegment, WitnessOpeningUnitSegment, WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Felt, FieldError, MODULUS};
use lzvm_prover::witness_commitment::{
    commit_witness_stage_leaves, extend_witness_stage_leaves, open_witness_stage_commitment,
    WitnessStageOpening,
};
use lzvm_prover::witness_layout::derive_witness_trace_layout;
use lzvm_prover::witness_opening::{
    load_witness_opening_segment_from_segments,
    load_witness_opening_unit_for_identity_from_segments, load_witness_opening_unit_from_segments,
    validate_witness_opening_segments, LoadWitnessOpeningSegmentError, LoadWitnessOpeningUnitError,
    ValidateWitnessOpeningSegmentsError,
};
use lzvm_prover::witness_trace::parse_witness_trace;
use lzvm_prover::ProveUnitSchedule;

const FIRST_WITNESS_OPENING_VALUE_OFFSET: usize = 12 + 8 + 12 + 12;

#[test]
fn loads_witness_opening_segment_from_segments() {
    let unit = witness_opening_unit(0);
    let segment = witness_opening_proof_segment(vec![unit.clone()]);

    let loaded =
        load_witness_opening_segment_from_segments(&[segment]).expect("segment should load");

    assert_eq!(loaded, WitnessOpeningSegment { units: vec![unit] });
}

#[test]
fn loads_witness_opening_unit_from_segments() {
    let unit = witness_opening_unit(0);
    let segment = witness_opening_proof_segment(vec![unit.clone()]);

    let loaded = load_witness_opening_unit_from_segments(0, &[segment]).expect("unit should load");

    assert_eq!(loaded, unit);
}

#[test]
fn loads_witness_opening_unit_for_identity_from_segments() {
    let mut unit = witness_opening_unit(0);
    unit.trace_instance_index = 2;
    let segment = witness_opening_proof_segment(vec![unit.clone()]);

    let loaded = load_witness_opening_unit_for_identity_from_segments(0, 2, &[segment])
        .expect("unit should load");

    assert_eq!(loaded, unit);
}

#[test]
fn rejects_witness_opening_unit_trace_identity_mismatch() {
    let mut unit = witness_opening_unit(0);
    unit.trace_instance_index = 2;
    let segment = witness_opening_proof_segment(vec![unit]);

    let error = load_witness_opening_unit_for_identity_from_segments(0, 1, &[segment])
        .expect_err("unit should require matching trace identity");

    assert_eq!(
        error,
        LoadWitnessOpeningUnitError::MissingUnit { unit_index: 0 }
    );
}

#[test]
fn rejects_missing_witness_opening_segment() {
    let error = load_witness_opening_segment_from_segments(&[]).expect_err("segment should exist");

    assert_eq!(error, LoadWitnessOpeningSegmentError::MissingSegment);
}

#[test]
fn rejects_invalid_witness_opening_segment() {
    let error = load_witness_opening_segment_from_segments(&[ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }])
    .expect_err("segment should parse");

    assert!(matches!(error, LoadWitnessOpeningSegmentError::Segment(_)));
}

#[test]
fn rejects_noncanonical_witness_opening_values_while_parsing() {
    let mut segment = witness_opening_proof_segment(vec![witness_opening_unit(0)]);
    segment.data[FIRST_WITNESS_OPENING_VALUE_OFFSET..FIRST_WITNESS_OPENING_VALUE_OFFSET + 8]
        .copy_from_slice(&MODULUS.to_le_bytes());

    let error = parse_witness_opening_segment(&segment.data)
        .expect_err("witness opening values should be canonical");

    assert_eq!(
        error,
        WitnessOpeningSegmentError::ValueNonCanonical {
            unit_index: 0,
            row_index: 3,
            stage_index: 1,
            value_index: 0,
            source: FieldError::NonCanonical { value: MODULUS },
        }
    );
}

#[test]
fn rejects_duplicate_witness_opening_segments() {
    let segment = witness_opening_proof_segment(vec![witness_opening_unit(0)]);

    let error = load_witness_opening_segment_from_segments(&[segment.clone(), segment])
        .expect_err("duplicate segment should be rejected");

    assert_eq!(error.to_string(), "duplicate witness opening segment");
}

#[test]
fn rejects_missing_witness_opening_unit() {
    let segment = witness_opening_proof_segment(vec![witness_opening_unit(1)]);

    let error =
        load_witness_opening_unit_from_segments(0, &[segment]).expect_err("unit should exist");

    assert_eq!(
        error,
        LoadWitnessOpeningUnitError::MissingUnit { unit_index: 0 }
    );
}

#[test]
fn validates_witness_opening_segments() {
    let (unit, segments) = valid_witness_opening_segments(2);

    validate_witness_opening_segments(&[unit], &segments).expect("opening should validate");
}

#[test]
fn rejects_witness_opening_row_mismatches() {
    let (unit, mut segments) = valid_witness_opening_segments(2);
    let bad_opening = witness_opening_proof_segment(vec![WitnessOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![WitnessOpeningQuerySegment {
            row_index: 1,
            stages: vec![WitnessOpeningStageSegment {
                stage_index: 1,
                values: vec![5, 6],
                siblings: Vec::new(),
            }],
        }],
    }]);
    let opening_segment = segments
        .iter_mut()
        .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
        .expect("opening segment should exist");
    *opening_segment = bad_opening;

    let error = validate_witness_opening_segments(&[unit], &segments)
        .expect_err("row mismatch should be rejected");

    assert_eq!(
        error,
        ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn validates_trace_instance_witness_opening_queries() {
    let (unit, mut segments) = valid_witness_opening_segments(2);
    replace_query_plan_trace_instance(&mut segments, 1);
    replace_opening_trace_instance(&mut segments, 1);
    replace_witness_commitment_trace_instance(&mut segments, 1);

    validate_witness_opening_segments(&[unit], &segments)
        .expect("trace instance opening should validate");
}

#[test]
fn validates_witness_opening_segments_for_same_unit_across_trace_instances() {
    let (unit, segments) = valid_witness_opening_segments(2);
    let base_commitment_segment = segments
        .iter()
        .find(|segment| segment.id == WITNESS_COMMITMENT_SEGMENT_BASE_ID)
        .expect("witness commitment segment should exist");
    let base_opening_segment = segments
        .iter()
        .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
        .expect("opening segment should exist");
    let base_opening = parse_witness_opening_segment(&base_opening_segment.data)
        .expect("opening segment should parse");
    let mut trace_one_opening = base_opening.units[0].clone();
    trace_one_opening.trace_instance_index = 1;
    let mut trace_two_opening = trace_one_opening.clone();
    trace_two_opening.trace_instance_index = 2;
    let mut trace_one_commitment = base_commitment_segment.clone();
    trace_one_commitment.id = WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1;
    let mut trace_two_commitment = base_commitment_segment.clone();
    trace_two_commitment.id = WITNESS_COMMITMENT_SEGMENT_BASE_ID + 2;
    let query_segment = ProofSegment {
        id: lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
            units: vec![
                PcsQueryPlanUnit {
                    unit_index: 0,
                    trace_instance_index: 1,
                    queries: vec![2],
                },
                PcsQueryPlanUnit {
                    unit_index: 0,
                    trace_instance_index: 2,
                    queries: vec![2],
                },
            ],
        })
        .expect("query plan should encode"),
    };
    let opening_segment = witness_opening_proof_segment(vec![trace_one_opening, trace_two_opening]);
    let segments = vec![
        query_segment,
        trace_one_commitment,
        trace_two_commitment,
        opening_segment,
    ];

    validate_witness_opening_segments(&[unit], &segments)
        .expect("same unit should validate across trace identities");
}

#[test]
fn rejects_witness_opening_unit_trace_instance_mismatch() {
    let (unit, mut segments) = valid_witness_opening_segments(2);
    replace_opening_trace_instance(&mut segments, 1);

    let error = validate_witness_opening_segments(&[unit], &segments)
        .expect_err("opening should match the query trace instance");

    assert_eq!(
        error,
        ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

fn replace_query_plan_trace_instance(segments: &mut [ProofSegment], trace_instance_index: u32) {
    let query_segment = segments
        .iter_mut()
        .find(|segment| segment.id == lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID)
        .expect("query segment should exist");
    query_segment.data = encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
        units: vec![PcsQueryPlanUnit {
            unit_index: 0,
            trace_instance_index,
            queries: vec![2],
        }],
    })
    .expect("query plan should encode");
}

fn replace_opening_trace_instance(segments: &mut [ProofSegment], trace_instance_index: u32) {
    let opening_segment = segments
        .iter_mut()
        .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
        .expect("opening segment should exist");
    let mut opening = lzvm_artifacts::witness_opening_segment::parse_witness_opening_segment(
        &opening_segment.data,
    )
    .expect("opening segment should parse");
    opening.units[0].trace_instance_index = trace_instance_index;
    opening_segment.data =
        encode_witness_opening_segment(&opening).expect("opening segment should encode");
}

fn replace_witness_commitment_trace_instance(
    segments: &mut [ProofSegment],
    trace_instance_index: u32,
) {
    let commitment_segment = segments
        .iter_mut()
        .find(|segment| segment.id == WITNESS_COMMITMENT_SEGMENT_BASE_ID)
        .expect("witness commitment segment should exist");
    commitment_segment.id = WITNESS_COMMITMENT_SEGMENT_BASE_ID + trace_instance_index;
}

fn witness_opening_proof_segment(units: Vec<WitnessOpeningUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: encode_witness_opening_segment(&WitnessOpeningSegment { units })
            .expect("segment should encode"),
    }
}

fn witness_opening_unit(unit_index: u32) -> WitnessOpeningUnitSegment {
    WitnessOpeningUnitSegment {
        unit_index,
        trace_instance_index: 0,
        queries: vec![WitnessOpeningQuerySegment {
            row_index: 3,
            stages: vec![WitnessOpeningStageSegment {
                stage_index: 1,
                values: vec![5],
                siblings: Vec::new(),
            }],
        }],
    }
}

fn valid_witness_opening_segments(query_row: u64) -> (ProveUnitSchedule, Vec<ProofSegment>) {
    let unit = sample_unit();
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace =
        parse_witness_trace(&encode_values(&[5, 9, 1, 9]), 2, 2).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");
    let leaves = extend_witness_stage_leaves(&stage, 1, 2).expect("witness leaves should extend");
    let commitment = commit_witness_stage_leaves(&leaves, 2).expect("witness stage should commit");
    let opening = open_witness_stage_commitment(&commitment, query_row, 4, 2)
        .expect("witness stage should open");

    let query_segment = ProofSegment {
        id: lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
            units: vec![PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 0,
                queries: vec![query_row],
            }],
        })
        .expect("query plan should encode"),
    };
    let commitment_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&WitnessCommitmentSegment {
            unit_index: 0,
            input_byte_count: 0,
            trace_rows: unit.base_domain_size,
            trace_columns: 2,
            stages: vec![WitnessCommitmentStageSegment {
                stage_index: 1,
                arity: unit.merkle_tree_arity,
                root: commitment.root().map(Felt::to_u64),
                tree_byte_count: commitment.tree_bytes().len() as u64,
                tree_digest: [0; 32],
            }],
        })
        .expect("witness commitment should encode"),
    };
    let opening_segment = witness_opening_proof_segment(vec![WitnessOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![WitnessOpeningQuerySegment {
            row_index: query_row,
            stages: vec![witness_opening_stage(1, &opening)],
        }],
    }]);

    (
        unit,
        vec![query_segment, commitment_segment, opening_segment],
    )
}

fn witness_opening_stage(
    stage_index: u32,
    opening: &WitnessStageOpening,
) -> WitnessOpeningStageSegment {
    WitnessOpeningStageSegment {
        stage_index,
        values: opening
            .values()
            .iter()
            .map(|value| value.to_u64())
            .collect(),
        siblings: opening
            .siblings()
            .iter()
            .map(|level| WitnessOpeningLevelSegment {
                siblings: level
                    .iter()
                    .map(|digest| digest.map(Felt::to_u64))
                    .collect(),
            })
            .collect(),
    }
}

fn sample_unit() -> ProveUnitSchedule {
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits: 1,
        extended_domain_bits: 2,
        base_domain_size: 2,
        extended_domain_size: 4,
        blowup_factor: 2,
        query_count: 1,
        proof_of_work_bits: 0,
        merkle_tree_arity: 2,
        last_level_verification: 0,
        transcript_arity: Some(2),
        hash_commits: false,
        transcript_root_challenge_draws: vec![1],
        challenge_count: 1,
        evaluation_value_count: 0,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 0,
        constant_width: 1,
        stage_commit_widths: vec![2],
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: 2,
            output_bits: 1,
            folding_factor: 2,
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

fn encode_values(values: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 8);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, parse_constant_opening_segment, ConstantOpeningLevelSegment,
    ConstantOpeningQuerySegment, ConstantOpeningSegment, ConstantOpeningSegmentError,
    ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::{ConstantTree, ConstantTreeHashKind};
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{poseidon2_hash_16, Felt, FieldError, MODULUS};
use lzvm_prover::constant_opening::{
    load_constant_opening_segment_from_segments,
    load_constant_opening_unit_for_identity_from_segments,
    load_constant_opening_unit_from_segments, validate_constant_opening_segments,
    LoadConstantOpeningSegmentError, LoadConstantOpeningUnitError,
    ValidateConstantOpeningSegmentsError,
};
use lzvm_prover::constant_tree_opening::{open_constant_tree_row, ConstantTreeOpening};
use lzvm_prover::ProveUnitSchedule;

const FIRST_CONSTANT_OPENING_VALUE_OFFSET: usize = 12 + 8 + 16;

#[test]
fn loads_constant_opening_segment_from_segments() {
    let unit = constant_opening_unit(0);
    let segment = constant_opening_proof_segment(vec![unit.clone()]);

    let loaded =
        load_constant_opening_segment_from_segments(&[segment]).expect("segment should load");

    assert_eq!(loaded, ConstantOpeningSegment { units: vec![unit] });
}

#[test]
fn loads_constant_opening_unit_from_segments() {
    let unit = constant_opening_unit(0);
    let segment = constant_opening_proof_segment(vec![unit.clone()]);

    let loaded = load_constant_opening_unit_from_segments(0, &[segment]).expect("unit should load");

    assert_eq!(loaded, unit);
}

#[test]
fn loads_constant_opening_unit_for_identity_from_segments() {
    let mut unit = constant_opening_unit(0);
    unit.trace_instance_index = 2;
    let segment = constant_opening_proof_segment(vec![unit.clone()]);

    let loaded = load_constant_opening_unit_for_identity_from_segments(0, 2, &[segment])
        .expect("unit should load");

    assert_eq!(loaded, unit);
}

#[test]
fn rejects_constant_opening_unit_trace_identity_mismatch() {
    let mut unit = constant_opening_unit(0);
    unit.trace_instance_index = 2;
    let segment = constant_opening_proof_segment(vec![unit]);

    let error = load_constant_opening_unit_for_identity_from_segments(0, 1, &[segment])
        .expect_err("unit should require matching trace identity");

    assert_eq!(
        error,
        LoadConstantOpeningUnitError::MissingUnit { unit_index: 0 }
    );
}

#[test]
fn rejects_missing_constant_opening_segment() {
    let error = load_constant_opening_segment_from_segments(&[]).expect_err("segment should exist");

    assert_eq!(error, LoadConstantOpeningSegmentError::MissingSegment);
}

#[test]
fn rejects_invalid_constant_opening_segment() {
    let error = load_constant_opening_segment_from_segments(&[ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }])
    .expect_err("segment should parse");

    assert!(matches!(error, LoadConstantOpeningSegmentError::Segment(_)));
}

#[test]
fn rejects_noncanonical_constant_opening_values_while_parsing() {
    let mut segment = constant_opening_proof_segment(vec![constant_opening_unit(0)]);
    segment.data[FIRST_CONSTANT_OPENING_VALUE_OFFSET..FIRST_CONSTANT_OPENING_VALUE_OFFSET + 8]
        .copy_from_slice(&MODULUS.to_le_bytes());

    let error = parse_constant_opening_segment(&segment.data)
        .expect_err("constant opening values should be canonical");

    assert_eq!(
        error,
        ConstantOpeningSegmentError::ValueNonCanonical {
            unit_index: 0,
            row_index: 3,
            value_index: 0,
            source: FieldError::NonCanonical { value: MODULUS },
        }
    );
}

#[test]
fn rejects_duplicate_constant_opening_segments() {
    let segment = constant_opening_proof_segment(vec![constant_opening_unit(0)]);

    let error = load_constant_opening_segment_from_segments(&[segment.clone(), segment])
        .expect_err("duplicate segment should be rejected");

    assert_eq!(error.to_string(), "duplicate constant opening segment");
}

#[test]
fn rejects_missing_constant_opening_unit() {
    let segment = constant_opening_proof_segment(vec![constant_opening_unit(1)]);

    let error =
        load_constant_opening_unit_from_segments(0, &[segment]).expect_err("unit should exist");

    assert_eq!(
        error,
        LoadConstantOpeningUnitError::MissingUnit { unit_index: 0 }
    );
}

#[test]
fn validates_constant_opening_segments() {
    let (unit, segments) = valid_constant_opening_segments(2);

    validate_constant_opening_segments(&[unit], &segments).expect("opening should validate");
}

#[test]
fn rejects_constant_opening_row_mismatches() {
    let (unit, mut segments) = valid_constant_opening_segments(2);
    let bad_opening = constant_opening_proof_segment(vec![ConstantOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![ConstantOpeningQuerySegment {
            row_index: 1,
            values: vec![3, 30],
            siblings: Vec::new(),
        }],
    }]);
    let opening_segment = segments
        .iter_mut()
        .find(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID)
        .expect("opening segment should exist");
    *opening_segment = bad_opening;

    let error = validate_constant_opening_segments(&[unit], &segments)
        .expect_err("row mismatch should be rejected");

    assert_eq!(
        error,
        ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn validates_trace_instance_constant_opening_queries() {
    let (unit, mut segments) = valid_constant_opening_segments(2);
    replace_query_plan_trace_instance(&mut segments, 1);
    replace_opening_trace_instance(&mut segments, 1);

    validate_constant_opening_segments(&[unit], &segments)
        .expect("trace instance opening should validate");
}

#[test]
fn rejects_constant_opening_unit_trace_instance_mismatch() {
    let (unit, mut segments) = valid_constant_opening_segments(2);
    replace_opening_trace_instance(&mut segments, 1);

    let error = validate_constant_opening_segments(&[unit], &segments)
        .expect_err("opening should match the query trace instance");

    assert_eq!(
        error,
        ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index: 0 }
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
        .find(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID)
        .expect("opening segment should exist");
    let mut opening = lzvm_artifacts::constant_opening_segment::parse_constant_opening_segment(
        &opening_segment.data,
    )
    .expect("opening segment should parse");
    opening.units[0].trace_instance_index = trace_instance_index;
    opening_segment.data =
        encode_constant_opening_segment(&opening).expect("opening segment should encode");
}

fn constant_opening_proof_segment(units: Vec<ConstantOpeningUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: encode_constant_opening_segment(&ConstantOpeningSegment { units })
            .expect("segment should encode"),
    }
}

fn constant_opening_unit(unit_index: u32) -> ConstantOpeningUnitSegment {
    ConstantOpeningUnitSegment {
        unit_index,
        trace_instance_index: 0,
        queries: vec![ConstantOpeningQuerySegment {
            row_index: 3,
            values: vec![5],
            siblings: Vec::new(),
        }],
    }
}

fn valid_constant_opening_segments(query_row: u64) -> (ProveUnitSchedule, Vec<ProofSegment>) {
    let (tree, root) = sample_tree();
    let opening = open_constant_tree_row(&tree, query_row, 4).expect("row should open");
    let unit = sample_unit(root);
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
    let opening_segment = constant_opening_proof_segment(vec![ConstantOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![constant_opening_query(&opening)],
    }]);

    (unit, vec![query_segment, opening_segment])
}

fn constant_opening_query(opening: &ConstantTreeOpening) -> ConstantOpeningQuerySegment {
    ConstantOpeningQuerySegment {
        row_index: opening.row_index(),
        values: opening
            .values()
            .iter()
            .map(|value| value.to_u64())
            .collect(),
        siblings: opening
            .siblings()
            .iter()
            .map(|level| ConstantOpeningLevelSegment {
                siblings: level
                    .iter()
                    .map(|digest| digest.map(Felt::to_u64))
                    .collect(),
            })
            .collect(),
    }
}

fn sample_tree() -> (ConstantTree, [Felt; 4]) {
    let rows = [
        [Felt::from_u64(1), Felt::from_u64(10)],
        [Felt::from_u64(2), Felt::from_u64(20)],
        [Felt::from_u64(3), Felt::from_u64(30)],
        [Felt::from_u64(4), Felt::from_u64(40)],
    ];
    let leaves = rows
        .iter()
        .map(|row| [row[0], row[1], Felt::ZERO, Felt::ZERO])
        .collect::<Vec<_>>();
    let state = poseidon2_hash_16([
        leaves[0][0],
        leaves[0][1],
        leaves[0][2],
        leaves[0][3],
        leaves[1][0],
        leaves[1][1],
        leaves[1][2],
        leaves[1][3],
        leaves[2][0],
        leaves[2][1],
        leaves[2][2],
        leaves[2][3],
        leaves[3][0],
        leaves[3][1],
        leaves[3][2],
        leaves[3][3],
    ]);
    let root = [state[0], state[1], state[2], state[3]];

    let mut bytes = Vec::new();
    for row in rows {
        for value in row {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for digest in &leaves {
        append_digest(&mut bytes, *digest);
    }
    append_digest(&mut bytes, root);

    (
        ConstantTree {
            hash_kind: ConstantTreeHashKind::Gl,
            extended_row_count: 4,
            constant_count: 2,
            leaf_byte_count: 64,
            node_byte_count: 160,
            bytes,
        },
        root,
    )
}

fn append_digest(out: &mut Vec<u8>, digest: [Felt; 4]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn sample_unit(root: [Felt; 4]) -> ProveUnitSchedule {
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
        merkle_tree_arity: 4,
        last_level_verification: 0,
        transcript_arity: Some(2),
        hash_commits: false,
        transcript_root_challenge_draws: vec![1],
        challenge_count: 1,
        evaluation_value_count: 0,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 0,
        constant_width: 2,
        stage_commit_widths: vec![1],
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
        pcs_material_constant_tree_root: Some(root.map(Felt::to_u64)),
        pcs_material_fixed_byte_count: None,
        pcs_material_constant_tree_byte_count: None,
        pcs_material_leaf_byte_count: None,
        pcs_material_node_byte_count: None,
    }
}

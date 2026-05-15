use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, ConstantOpeningLevelSegment, ConstantOpeningQuerySegment,
    ConstantOpeningSegment, ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::{ConstantTree, ConstantTreeHashKind};
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{poseidon2_hash_16, Felt};
use lzvm_prover::constant_opening::{
    load_constant_opening_segment_from_segments, load_constant_opening_unit_from_segments,
    validate_constant_opening_segments, LoadConstantOpeningSegmentError,
    LoadConstantOpeningUnitError, ValidateConstantOpeningSegmentsError,
};
use lzvm_prover::constant_tree_opening::{open_constant_tree_row, ConstantTreeOpening};
use lzvm_prover::ProveUnitSchedule;

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
                queries: vec![query_row],
            }],
        })
        .expect("query plan should encode"),
    };
    let opening_segment = constant_opening_proof_segment(vec![ConstantOpeningUnitSegment {
        unit_index: 0,
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

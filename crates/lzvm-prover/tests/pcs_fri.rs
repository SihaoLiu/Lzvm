use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_fri_segment::{
    PcsFriOpeningLayerSegment, PcsFriOpeningQuerySegment, PcsFriOpeningUnitSegment,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_field::{poseidon2_hash_8, Ext3, Felt, SHIFT};
use lzvm_prover::pcs_fri::{
    verify_fri_fold, verify_fri_last_level_root, verify_fri_opening_folds, verify_fri_query_path,
    PcsFriFoldError, PcsFriMerkleError, PcsFriOpeningFoldRequest,
};
use lzvm_prover::ProveUnitSchedule;

#[test]
fn verifies_binary_fri_fold_values() {
    let constant = Ext3::from_u64s([1, 2, 3]);
    let slope = Ext3::from_u64s([4, 5, 6]);
    let values = vec![constant + slope, constant - slope];
    let challenge = Ext3::from_u64s([7, 8, 9]);
    let point_inverse = SHIFT.inverse().expect("shift is nonzero");
    let expected = constant + slope * scale(challenge, point_inverse);

    let folded = verify_fri_fold(2, 1, 2, challenge, 0, &values)
        .expect("fold should verify over a binary group");

    assert_eq!(folded, expected);
}

#[test]
fn rejects_fri_fold_values_with_wrong_group_size() {
    let result = verify_fri_fold(2, 1, 2, Ext3::ONE, 0, &[Ext3::ONE]);

    assert!(matches!(
        result,
        Err(PcsFriFoldError::ValueLengthMismatch {
            expected: 2,
            found: 1
        })
    ));
}

#[test]
fn verifies_fri_query_path_against_root() {
    let values = [
        Ext3::from_u64s([1, 2, 3]),
        Ext3::from_u64s([4, 5, 6]),
        Ext3::from_u64s([7, 8, 9]),
        Ext3::from_u64s([10, 11, 12]),
    ];
    let leaves = values.map(extension_leaf);
    let last_level = [
        parent_arity2(leaves[0], leaves[1]),
        parent_arity2(leaves[2], leaves[3]),
    ];
    let root = parent_arity2(last_level[0], last_level[1]);
    let siblings = vec![vec![leaves[0]], vec![last_level[1]]];

    assert!(
        verify_fri_query_path(root, &[], 2, 1, &[values[1]], &siblings)
            .expect("path should verify against root")
    );
    assert!(
        !verify_fri_query_path(root, &[], 2, 1, &[values[2]], &siblings)
            .expect("path should check against root")
    );
}

#[test]
fn verifies_fri_query_path_against_last_level() {
    let values = [
        Ext3::from_u64s([13, 14, 15]),
        Ext3::from_u64s([16, 17, 18]),
        Ext3::from_u64s([19, 20, 21]),
        Ext3::from_u64s([22, 23, 24]),
    ];
    let leaves = values.map(extension_leaf);
    let last_level = [
        parent_arity2(leaves[0], leaves[1]),
        parent_arity2(leaves[2], leaves[3]),
    ];
    let root = parent_arity2(last_level[0], last_level[1]);
    let siblings = vec![vec![leaves[0]]];

    assert!(
        verify_fri_query_path(root, &last_level, 2, 1, &[values[1]], &siblings)
            .expect("path should verify against last level")
    );
    assert!(verify_fri_last_level_root(root, 2, &last_level)
        .expect("last level should verify against root"));
}

#[test]
fn rejects_fri_query_paths_with_wrong_sibling_count() {
    let value = Ext3::from_u64s([25, 26, 27]);
    let result = verify_fri_query_path([Felt::ZERO; 4], &[], 2, 0, &[value], &[Vec::new()]);

    assert!(matches!(
        result,
        Err(PcsFriMerkleError::InvalidSiblingCount {
            expected: 1,
            found: 0
        })
    ));
}

#[test]
fn verifies_fri_opening_fold_chain_to_final_polynomial() {
    let query_row = 3_u64;
    let schedule = ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: None,
        unit_id: None,
        group_name: None,
        unit_name: None,
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
        transcript_root_challenge_draws: vec![2, 1],
        challenge_count: 6,
        evaluation_value_count: 0,
        transcript_evaluation_challenge_draws: 2,
        constant_width: 1,
        stage_commit_widths: vec![1],
        commitment_columns: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![
            PcsFriLayer {
                input_bits: 2,
                output_bits: 1,
                folding_factor: 2,
            },
            PcsFriLayer {
                input_bits: 1,
                output_bits: 0,
                folding_factor: 2,
            },
        ],
        final_layer_bits: 0,
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
    };
    let first_challenge = Ext3::from_u64s([7, 8, 9]);
    let second_challenge = Ext3::from_u64s([11, 12, 13]);
    let mut challenges = vec![Ext3::ZERO; 9];
    challenges[7] = first_challenge;
    challenges[8] = second_challenge;

    let layer0_values = vec![Ext3::from_u64s([1, 2, 3]), Ext3::from_u64s([4, 5, 6])];
    let layer0_fold = verify_fri_fold(
        schedule.extended_domain_bits,
        schedule.fri_layers[0].output_bits,
        schedule.fri_layers[0].input_bits,
        first_challenge,
        query_row % 2,
        &layer0_values,
    )
    .expect("first fold should evaluate");
    let layer1_values = vec![layer0_fold + Ext3::ONE, layer0_fold];
    let final_value = verify_fri_fold(
        schedule.extended_domain_bits,
        schedule.fri_layers[1].output_bits,
        schedule.fri_layers[1].input_bits,
        second_challenge,
        0,
        &layer1_values,
    )
    .expect("second fold should evaluate");
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 3,
        layers: vec![
            PcsFriOpeningLayerSegment {
                layer_index: 0,
                root: [0; 4],
                last_level: Vec::new(),
                queries: vec![PcsFriOpeningQuerySegment {
                    row_index: 1,
                    values: layer0_values.iter().map(|value| value.to_u64s()).collect(),
                    siblings: Vec::new(),
                }],
            },
            PcsFriOpeningLayerSegment {
                layer_index: 1,
                root: [0; 4],
                last_level: Vec::new(),
                queries: vec![PcsFriOpeningQuerySegment {
                    row_index: 0,
                    values: layer1_values.iter().map(|value| value.to_u64s()).collect(),
                    siblings: Vec::new(),
                }],
            },
        ],
        final_polynomial: vec![final_value.to_u64s()],
    };

    let valid = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 3,
            query_rows: &[query_row],
            challenges: &challenges,
            fri: &fri,
        },
    )
    .expect("fold chain should evaluate");

    assert!(valid);

    let mut tampered = fri.clone();
    tampered.final_polynomial[0] = Ext3::ONE.to_u64s();
    let invalid = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 3,
            query_rows: &[query_row],
            challenges: &challenges,
            fri: &tampered,
        },
    )
    .expect("fold chain should evaluate mismatches");

    assert!(!invalid);
}

fn scale(value: Ext3, scalar: Felt) -> Ext3 {
    Ext3::new(value.c0 * scalar, value.c1 * scalar, value.c2 * scalar)
}

fn extension_leaf(value: Ext3) -> [Felt; 4] {
    [value.c0, value.c1, value.c2, Felt::ZERO]
}

fn parent_arity2(left: [Felt; 4], right: [Felt; 4]) -> [Felt; 4] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

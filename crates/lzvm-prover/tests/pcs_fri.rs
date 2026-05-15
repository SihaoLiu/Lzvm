use lzvm_field::{poseidon2_hash_8, Ext3, Felt, SHIFT};
use lzvm_prover::pcs_fri::{
    verify_fri_fold, verify_fri_last_level_root, verify_fri_query_path, PcsFriFoldError,
    PcsFriMerkleError,
};

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

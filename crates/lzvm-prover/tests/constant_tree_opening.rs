use lzvm_artifacts::constant_tree::{ConstantTree, ConstantTreeHashKind};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_field::{poseidon2_hash_16, Felt};
use lzvm_prover::constant_tree_opening::{
    open_constant_tree_row, verify_constant_tree_opening_root, ConstantTreeOpening,
};

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

#[test]
fn opens_and_verifies_constant_tree_rows() {
    let (tree, root) = sample_tree();

    let opening = open_constant_tree_row(&tree, 2, 4).expect("row should open");
    let valid = verify_constant_tree_opening_root(root, 4, &opening)
        .expect("opening should verify without structural errors");

    assert!(valid);
    assert_eq!(opening.values(), &[Felt::from_u64(3), Felt::from_u64(30)]);
    assert_eq!(opening.siblings().len(), 1);
    assert_eq!(opening.siblings()[0].len(), 3);
    assert_eq!(
        tree.root().expect("root should extract"),
        VerificationKeyRoot::FieldElements(root.iter().map(|value| value.to_u64()).collect())
    );
}

#[test]
fn rejects_tampered_constant_tree_opening_values() {
    let (tree, root) = sample_tree();
    let opening = open_constant_tree_row(&tree, 2, 4).expect("row should open");
    let tampered = ConstantTreeOpening::new(
        opening.row_index(),
        vec![Felt::from_u64(9), opening.values()[1]],
        opening.siblings().to_vec(),
    )
    .expect("tampered opening shape should be valid");

    let valid = verify_constant_tree_opening_root(root, 4, &tampered)
        .expect("opening should verify without structural errors");

    assert!(!valid);
}

fn append_digest(out: &mut Vec<u8>, digest: [Felt; 4]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

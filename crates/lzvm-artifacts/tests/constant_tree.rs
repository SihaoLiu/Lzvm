use lzvm_artifacts::constant_tree::{
    expected_constant_tree_byte_count, expected_constant_tree_word_count, read_constant_tree_file,
    summarize_constant_tree_file, ConstantTreeError, ConstantTreeHashKind,
};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use std::fs;
use std::path::PathBuf;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;

mod fixtures;

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-constant-tree-{}-{name}", std::process::id()))
}

#[test]
fn computes_gl_constant_tree_sizes() {
    let setup = fixtures::sample_constant_tree_setup_info();

    assert_eq!(expected_constant_tree_word_count(&setup).unwrap(), 32);
    assert_eq!(expected_constant_tree_byte_count(&setup).unwrap(), 256);
}

#[test]
fn reads_constant_tree_files_with_expected_size() {
    let setup = fixtures::sample_constant_tree_setup_info();
    let path = temp_file_path("tree.bin");
    let bytes = vec![7_u8; expected_constant_tree_byte_count(&setup).unwrap()];
    fs::write(&path, &bytes).expect("fixture should be written");

    let tree = read_constant_tree_file(&path, &setup).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(tree.hash_kind, ConstantTreeHashKind::Gl);
    assert_eq!(tree.extended_row_count, 4);
    assert_eq!(tree.constant_count, 1);
    assert_eq!(tree.leaf_byte_count, 32);
    assert_eq!(tree.node_byte_count, 224);
    assert_eq!(tree.bytes, bytes);
}

#[test]
fn extracts_roots_from_raw_tree_tails() {
    let setup = fixtures::sample_constant_tree_setup_info();
    let path = temp_file_path("rooted-tree.bin");
    let mut bytes = vec![7_u8; expected_constant_tree_byte_count(&setup).unwrap()];
    let root_values = [1_u64, 2, 3, 4];
    for (index, value) in root_values.iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    fs::write(&path, &bytes).expect("fixture should be written");

    let tree = read_constant_tree_file(&path, &setup).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(
        tree.root().expect("root should extract"),
        VerificationKeyRoot::FieldElements(root_values.to_vec())
    );
}

#[test]
fn rejects_non_canonical_roots_from_raw_tree_tails() {
    let setup = fixtures::sample_constant_tree_setup_info();
    let path = temp_file_path("bad-rooted-tree.bin");
    let mut bytes = vec![7_u8; expected_constant_tree_byte_count(&setup).unwrap()];
    let root_values = [1_u64, 2, NON_CANONICAL_FIELD, 4];
    for (index, value) in root_values.iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    fs::write(&path, &bytes).expect("fixture should be written");

    let tree = read_constant_tree_file(&path, &setup).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    let error = tree
        .root()
        .expect_err("non-canonical tree root word should reject");
    assert_eq!(
        error.to_string(),
        "constant-tree root word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_roots_in_tree_summaries() {
    let setup = fixtures::sample_constant_tree_setup_info();
    let path = temp_file_path("bad-summary-root.bin");
    let mut bytes = vec![7_u8; expected_constant_tree_byte_count(&setup).unwrap()];
    let root_values = [1_u64, NON_CANONICAL_FIELD, 3, 4];
    for (index, value) in root_values.iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    fs::write(&path, &bytes).expect("fixture should be written");

    let error = summarize_constant_tree_file(&path, &setup)
        .expect_err("non-canonical summary root word should reject");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(
        error.to_string(),
        "constant-tree root word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_constant_tree_files_with_wrong_size() {
    let setup = fixtures::sample_constant_tree_setup_info();
    let path = temp_file_path("bad-tree.bin");
    fs::write(&path, vec![0_u8; 31]).expect("fixture should be written");

    let error = read_constant_tree_file(&path, &setup).expect_err("fixture should be rejected");
    fs::remove_file(&path).expect("fixture should be removed");

    assert!(matches!(
        error,
        ConstantTreeError::InvalidByteLength {
            expected: 256,
            found: 31
        }
    ));
}

#[test]
fn rejects_invalid_merkle_arities() {
    let mut setup = fixtures::sample_constant_tree_setup_info();
    setup.stark.merkle_tree_arity = 1;

    assert!(matches!(
        expected_constant_tree_word_count(&setup),
        Err(ConstantTreeError::InvalidArity { arity: 1 })
    ));
}

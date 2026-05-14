use lzvm_artifacts::constant_tree::{
    expected_constant_tree_byte_count, expected_constant_tree_word_count, read_constant_tree_file,
    ConstantTreeError, ConstantTreeHashKind,
};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use std::fs;
use std::path::PathBuf;

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 2,
        "nConstants": 1,
        "nPublics": 0,
        "nConstraints": 0,
        "qDeg": 7,
        "openingPoints": [0],
        "mapSectionsN": {
            "const": 1,
            "cm1": 1,
            "cm2": 1,
            "cm3": 1
        },
        "challengesMap": [],
        "evMap": [],
        "boundaries": [],
        "starkStruct": {
            "nBits": 1,
            "nBitsExt": 2,
            "nQueries": 1,
            "steps": [
                {"nBits": 2},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 1,
            "powBits": 1,
            "merkleTreeArity": 2,
            "verificationHashType": "GL",
            "transcriptArity": 2,
            "merkleTreeCustom": true
        }
    }"#
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-constant-tree-{}-{name}", std::process::id()))
}

#[test]
fn computes_gl_constant_tree_sizes() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");

    assert_eq!(expected_constant_tree_word_count(&setup).unwrap(), 32);
    assert_eq!(expected_constant_tree_byte_count(&setup).unwrap(), 256);
}

#[test]
fn reads_constant_tree_files_with_expected_size() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
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
fn rejects_constant_tree_files_with_wrong_size() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
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
    let json = sample_setup_info_json().replace("\"merkleTreeArity\": 2", "\"merkleTreeArity\": 1");
    let setup = parse_unit_setup_info_json(&json).expect("setup should parse");

    assert!(matches!(
        expected_constant_tree_word_count(&setup),
        Err(ConstantTreeError::InvalidArity { arity: 1 })
    ));
}

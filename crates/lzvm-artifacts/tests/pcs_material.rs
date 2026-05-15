use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::constant_tree::parse_constant_tree_bytes;
use lzvm_artifacts::pcs_material::{
    build_pcs_setup_material, encode_pcs_setup_material, parse_pcs_setup_material,
    read_pcs_setup_material_file,
};
use lzvm_artifacts::pcs_plan::{derive_pcs_setup_plan, encode_pcs_setup_plan};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use sha2::{Digest, Sha256};

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 1,
        "nConstants": 2,
        "nPublics": 0,
        "nConstraints": 0,
        "qDeg": 3,
        "openingPoints": [0],
        "mapSectionsN": {
            "const": 2,
            "cm1": 1,
            "cm2": 1
        },
        "constPolsMap": [
            {"stage": 0, "name": "main.left", "dim": 1, "polsMapId": 0, "stageId": 0},
            {"stage": 0, "name": "main.right", "dim": 1, "polsMapId": 1, "stageId": 1}
        ],
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
            "lastLevelVerification": 2,
            "powBits": 0,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_tree_bytes() -> Vec<u8> {
    let mut bytes = vec![7_u8; 224];
    for (index, value) in [1_u64, 2, 3, 4].iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-pcs-material-{}-{name}", std::process::id()))
}

#[test]
fn builds_pcs_setup_material_from_plan_and_static_artifacts() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let fixed = [1_u8; 32];
    let tree = parse_constant_tree_bytes(sample_tree_bytes(), &setup).expect("tree should parse");

    let material = build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");
    let expected_plan_digest: [u8; 32] =
        Sha256::digest(encode_pcs_setup_plan(&plan).expect("plan should encode")).into();
    let expected_fixed_digest: [u8; 32] = Sha256::digest(fixed).into();
    let expected_tree_digest: [u8; 32] = Sha256::digest(&tree.bytes).into();

    assert_eq!(material.plan_digest, expected_plan_digest);
    assert_eq!(material.fixed_column_digest, expected_fixed_digest);
    assert_eq!(material.constant_tree_digest, expected_tree_digest);
    assert_eq!(material.constant_tree_root, [1, 2, 3, 4]);
    assert_eq!(material.fixed_byte_count, 32);
    assert_eq!(material.constant_tree_byte_count, 224);
    assert_eq!(material.leaf_byte_count, 64);
    assert_eq!(material.node_byte_count, 160);
}

#[test]
fn encodes_and_parses_pcs_setup_material() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let fixed = [1_u8; 32];
    let tree = parse_constant_tree_bytes(sample_tree_bytes(), &setup).expect("tree should parse");
    let material = build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");

    let encoded = encode_pcs_setup_material(&material).expect("material should encode");
    let parsed = parse_pcs_setup_material(&encoded).expect("material should parse");

    assert_eq!(&encoded[0..4], b"pcsm");
    assert_eq!(parsed, material);
}

#[test]
fn reads_pcs_setup_material_from_a_file_path() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let tree = parse_constant_tree_bytes(sample_tree_bytes(), &setup).expect("tree should parse");
    let material =
        build_pcs_setup_material(&plan, &[1_u8; 32], &tree).expect("material should build");
    let path = temp_file_path("material.bin");
    fs::write(
        &path,
        encode_pcs_setup_material(&material).expect("material should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_pcs_setup_material_file(&path).expect("material should read");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, material);
}

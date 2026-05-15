use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use lzvm_artifacts::verification_key::{read_verification_key_binary_file, VerificationKeyRoot};
use lzvm_setup::{write_verification_key_from_constant_tree, SetupError};

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

fn sample_root() -> VerificationKeyRoot {
    VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])
}

fn sample_tree_bytes() -> Vec<u8> {
    let mut bytes = vec![7_u8; 256];
    for (index, value) in [1_u64, 2, 3, 4].iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-setup-verkey-{}-{name}", std::process::id()))
}

fn staging_entries(parent: &Path) -> Vec<PathBuf> {
    fs::read_dir(parent)
        .expect("directory should be readable")
        .map(|entry| entry.expect("directory entry should exist").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".staging."))
        })
        .collect()
}

#[test]
fn writes_verification_key_binary_from_constant_tree_root() {
    let dir = temp_dir("write");
    let _ = fs::remove_dir_all(&dir);
    let binary_path = dir.join("base").join("unit-a.verkey.bin");
    let json_path = dir.join("base").join("unit-a.verkey.json");
    fs::create_dir_all(binary_path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(&json_path, b"stale-json").expect("stale json should be written");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");

    let report =
        write_verification_key_from_constant_tree(&binary_path, &sample_tree_bytes(), &setup)
            .expect("write should succeed");
    let binary_root =
        read_verification_key_binary_file(&binary_path).expect("binary root should read");
    let staging = staging_entries(binary_path.parent().expect("path should have a parent"));

    assert_eq!(report.binary_path, binary_path);
    assert_eq!(report.binary_bytes, 32);
    assert_eq!(report.root, sample_root());
    assert_eq!(binary_root, sample_root());
    assert!(!json_path.exists());
    assert!(staging.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn preserves_existing_verification_keys_when_tree_validation_fails() {
    let dir = temp_dir("preserve");
    let _ = fs::remove_dir_all(&dir);
    let binary_path = dir.join("base").join("unit-a.verkey.bin");
    fs::create_dir_all(binary_path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(&binary_path, b"stable-bin").expect("stable binary should be written");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");

    let result = write_verification_key_from_constant_tree(&binary_path, b"bad-tree", &setup);
    let stable_binary = fs::read(&binary_path).expect("stable binary should still exist");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(result, Err(SetupError::ConstantTree(_))));
    assert_eq!(stable_binary, b"stable-bin");
}

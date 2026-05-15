use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::{parse_constant_tree_bytes, read_constant_tree_file};
use lzvm_artifacts::fixed::{encode_fixed_columns, FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::{encode_unit_setup_info, parse_unit_setup_info_json};
use lzvm_artifacts::verification_key::{
    encode_verification_key_binary, read_verification_key_binary_file,
};
use lzvm_setup::{
    build_constant_tree_from_fixed_columns, write_constant_tree_file,
    write_constant_tree_leaves_file, write_constant_tree_native_file,
    write_verification_key_native_file, ConstantTreeLeavesWriteReport, ConstantTreeWriteReport,
    FixedExtensionBackend, VerificationKeyWriteReport,
};

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 1,
        "nConstants": 2,
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
            "nQueries": 2,
            "steps": [
                {"nBits": 2},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 0,
            "merkleTreeArity": 2,
            "verificationHashType": "GL",
            "transcriptArity": 2,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 2,
        columns: vec![
            FixedColumn {
                name: "main.left".to_owned(),
                dimensions: vec![1],
                values: vec![5, 1],
            },
            FixedColumn {
                name: "main.right".to_owned(),
                dimensions: vec![1],
                values: vec![9, 9],
            },
        ],
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-setup-constant-files-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
}

#[test]
fn writes_constant_tree_files_and_verification_key_from_paths() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.columns.bin");
    let raw_tree_path = dir.join("unit.raw.consttree");
    let root_path = dir.join("unit.root.bin");
    let leaves_path = dir.join("unit.constleaves");
    let native_tree_path = dir.join("unit.native.consttree");
    let checked_tree_path = dir.join("unit.checked.consttree");
    let key_path = dir.join("unit.verkey.bin");

    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let columns = sample_columns();
    let expected_tree =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let expected_root = parse_constant_tree_bytes(expected_tree.clone(), &setup)
        .expect("expected tree should parse")
        .root()
        .expect("expected root should derive");
    write_bytes(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_bytes(
        &columns_path,
        encode_fixed_columns(&columns).expect("columns should encode"),
    );
    write_bytes(&raw_tree_path, &expected_tree);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&expected_root).expect("root should encode"),
    );

    let leaves_report = write_constant_tree_leaves_file(
        &setup_path,
        &columns_path,
        &leaves_path,
        FixedExtensionBackend::Cpu,
    )
    .expect("leaves should write");
    let native_tree_report = write_constant_tree_native_file(
        &setup_path,
        &columns_path,
        Some(&root_path),
        &native_tree_path,
        FixedExtensionBackend::Cpu,
    )
    .expect("native tree should write");
    let checked_tree_report =
        write_constant_tree_file(&setup_path, &raw_tree_path, &root_path, &checked_tree_path)
            .expect("checked tree should write");
    let key_report = write_verification_key_native_file(&setup_path, &checked_tree_path, &key_path)
        .expect("key should write");

    let checked_tree =
        read_constant_tree_file(&checked_tree_path, &setup).expect("checked tree should parse");
    let key_root = read_verification_key_binary_file(&key_path).expect("key should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        leaves_report,
        ConstantTreeLeavesWriteReport {
            path: leaves_path,
            bytes_written: 64,
            row_count: 4,
            column_count: 2,
        }
    );
    assert_eq!(
        native_tree_report,
        ConstantTreeWriteReport {
            path: native_tree_path,
            bytes_written: expected_tree.len() as u64,
            root: expected_root.clone(),
        }
    );
    assert_eq!(
        checked_tree_report,
        ConstantTreeWriteReport {
            path: checked_tree_path,
            bytes_written: expected_tree.len() as u64,
            root: expected_root.clone(),
        }
    );
    assert_eq!(
        key_report,
        VerificationKeyWriteReport {
            binary_path: key_path,
            binary_bytes: 32,
            root: expected_root.clone(),
        }
    );
    assert_eq!(
        checked_tree.root().expect("root should extract"),
        expected_root
    );
    assert_eq!(key_root, expected_root);
}

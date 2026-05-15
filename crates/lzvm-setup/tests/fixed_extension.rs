use lzvm_artifacts::fixed::{FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use lzvm_field::{Felt, FieldError, MODULUS, SHIFT};
use std::fs;
use std::path::{Path, PathBuf};

use lzvm_setup::{extend_fixed_columns_for_constant_tree, write_constant_tree_leaves, SetupError};

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

fn words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("chunk length checked")))
        .collect()
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-setup-leaves-{}-{name}", std::process::id()))
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
fn extends_fixed_columns_into_row_major_constant_tree_leaves() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let root = Felt::root_of_unity(2).expect("root should exist");
    let two = Felt::from_u64(2);
    let three = Felt::from_u64(3);
    let shifted_root = SHIFT * root;
    let expected_left = [
        three + two * SHIFT,
        three + two * shifted_root,
        three - two * SHIFT,
        three - two * shifted_root,
    ];

    let leaves = extend_fixed_columns_for_constant_tree(&sample_columns(), &setup)
        .expect("extension should succeed");

    assert_eq!(leaves.len(), 64);
    assert_eq!(
        words(&leaves),
        vec![
            expected_left[0].to_u64(),
            9,
            expected_left[1].to_u64(),
            9,
            expected_left[2].to_u64(),
            9,
            expected_left[3].to_u64(),
            9,
        ]
    );
}

#[test]
fn rejects_non_canonical_fixed_values_before_extension() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let mut columns = sample_columns();
    columns.columns[0].values[0] = MODULUS;

    assert!(matches!(
        extend_fixed_columns_for_constant_tree(&columns, &setup),
        Err(SetupError::Field(FieldError::NonCanonical { value })) if value == MODULUS
    ));
}

#[test]
fn writes_extended_leaves_through_validated_staging() {
    let dir = temp_dir("write-leaves");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.constleaves");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");

    let report =
        write_constant_tree_leaves(&path, &sample_columns(), &setup).expect("write should succeed");
    let bytes = fs::read(&path).expect("leaf output should exist");
    let staging = staging_entries(path.parent().expect("path should have a parent"));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(report.path, path);
    assert_eq!(report.bytes_written, 64);
    assert_eq!(report.row_count, 4);
    assert_eq!(report.column_count, 2);
    assert_eq!(
        bytes,
        extend_fixed_columns_for_constant_tree(&sample_columns(), &setup)
            .expect("extension should succeed")
    );
    assert!(staging.is_empty());
}

#[test]
fn preserves_existing_extended_leaves_when_generation_fails() {
    let dir = temp_dir("preserve-leaves");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.constleaves");
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(&path, b"stable-output").expect("stable fixture should be written");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let mut columns = sample_columns();
    columns.columns[0].values[0] = MODULUS;

    let result = write_constant_tree_leaves(&path, &columns, &setup);
    let stable = fs::read(&path).expect("stable output should still exist");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(result, Err(SetupError::Field(_))));
    assert_eq!(stable, b"stable-output");
}

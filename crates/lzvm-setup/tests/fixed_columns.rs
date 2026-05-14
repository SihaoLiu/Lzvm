use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::fixed::{read_raw_fixed_column_file, FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use lzvm_setup::write_base_fixed_columns;

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
            "nBits": 2,
            "nBitsExt": 3,
            "nQueries": 2,
            "steps": [
                {"nBits": 3},
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

fn sample_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 4,
        columns: vec![
            FixedColumn {
                name: "main.left".to_owned(),
                dimensions: vec![1],
                values: vec![1, 2, 3, 4],
            },
            FixedColumn {
                name: "main.right".to_owned(),
                dimensions: vec![1],
                values: vec![10, 20, 30, 40],
            },
        ],
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-setup-{}-{name}", std::process::id()))
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
fn writes_base_fixed_columns_through_validated_staging() {
    let dir = temp_dir("write-fixed");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.const");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");

    let report =
        write_base_fixed_columns(&path, &sample_columns(), &setup).expect("write should succeed");
    let left = read_raw_fixed_column_file(&path, &setup, "group-a", "unit-a", 0)
        .expect("left should read");
    let right = read_raw_fixed_column_file(&path, &setup, "group-a", "unit-a", 1)
        .expect("right should read");
    let staging = staging_entries(path.parent().expect("path should have a parent"));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(report.path, path);
    assert_eq!(report.bytes_written, 64);
    assert_eq!(left, [1, 2, 3, 4]);
    assert_eq!(right, [10, 20, 30, 40]);
    assert!(staging.is_empty());
}

#[test]
fn preserves_existing_output_when_generation_fails() {
    let dir = temp_dir("preserve-fixed");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.const");
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(&path, b"stable-output").expect("stable fixture should be written");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let mut columns = sample_columns();
    columns.columns.pop();

    let result = write_base_fixed_columns(&path, &columns, &setup);
    let stable = fs::read(&path).expect("stable output should still exist");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(result.is_err());
    assert_eq!(stable, b"stable-output");
}

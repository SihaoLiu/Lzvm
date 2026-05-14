use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::fixed::read_raw_fixed_column_file;
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use lzvm_cli::run_cli;

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

fn sample_columns_json() -> &'static str {
    r#"{
        "group_name": "group-a",
        "unit_name": "unit-a",
        "row_count": 4,
        "columns": [
            {
                "name": "main.left",
                "dimensions": [1],
                "values": [1, 2, 3, 4]
            },
            {
                "name": "main.right",
                "dimensions": [1],
                "values": [10, 20, 30, 40]
            }
        ]
    }"#
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-write-fixed-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_fixed_columns_from_json_inputs() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.starkinfo.json");
    let columns_path = dir.join("unit.fixed.json");
    let out_path = dir.join("unit.const");
    fs::write(&setup_path, sample_setup_info_json()).expect("setup fixture should be written");
    fs::write(&columns_path, sample_columns_json()).expect("columns fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-fixed",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let left = read_raw_fixed_column_file(&out_path, &setup, "group-a", "unit-a", 0)
        .expect("left column should read");
    let right = read_raw_fixed_column_file(&out_path, &setup, "group-a", "unit-a", 1)
        .expect("right column should read");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written=64\noutput={}\n",
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(left, [1, 2, 3, 4]);
    assert_eq!(right, [10, 20, 30, 40]);
}

#[test]
fn reports_usage_for_missing_fixed_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-fixed",
            "unit.starkinfo.json",
            "unit.fixed.json",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-fixed <setup-info-json> <columns-json> <out-const>\n"
    );
}

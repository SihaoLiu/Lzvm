use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{encode_fixed_columns, FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::{encode_unit_setup_info, parse_unit_setup_info_json};
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
        "lzvm-cli-write-const-leaves-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_extended_leaves_from_binary_setup_and_columns() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.fixed.bin");
    let out_path = dir.join("unit.constleaves");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(
        &columns_path,
        encode_fixed_columns(&sample_columns()).expect("columns should encode"),
    )
    .expect("columns fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-const-leaves",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let bytes = fs::read(&out_path).expect("leaf output should exist");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written=64\nrows=4\ncolumns=2\noutput={}\n",
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(bytes.len(), 64);
}

#[test]
#[cfg(feature = "cuda")]
fn writes_extended_leaves_with_cuda_backend_option() {
    let dir = temp_dir("cuda");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.fixed.bin");
    let cpu_out = dir.join("unit.cpu.constleaves");
    let cuda_out = dir.join("unit.cuda.constleaves");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(
        &columns_path,
        encode_fixed_columns(&sample_columns()).expect("columns should encode"),
    )
    .expect("columns fixture should be written");

    let mut cpu_stdout = Vec::new();
    let mut cpu_stderr = Vec::new();
    let cpu_code = run_cli(
        &[
            "setup",
            "write-const-leaves",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            cpu_out.to_str().expect("cpu output path should be utf-8"),
        ],
        &mut cpu_stdout,
        &mut cpu_stderr,
    );
    let mut cuda_stdout = Vec::new();
    let mut cuda_stderr = Vec::new();
    let cuda_code = run_cli(
        &[
            "setup",
            "write-const-leaves",
            "--backend",
            "cuda",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            cuda_out.to_str().expect("cuda output path should be utf-8"),
        ],
        &mut cuda_stdout,
        &mut cuda_stderr,
    );

    let cpu_bytes = fs::read(&cpu_out).expect("cpu leaf output should exist");
    let cuda_bytes = fs::read(&cuda_out).expect("cuda leaf output should exist");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(cpu_code, 0);
    assert_eq!(cuda_code, 0);
    assert!(cpu_stderr.is_empty());
    assert!(cuda_stderr.is_empty());
    assert_eq!(cuda_bytes, cpu_bytes);
}

#[test]
fn reports_usage_for_missing_extended_leaves_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-const-leaves",
            "unit.setup.bin",
            "unit.fixed.bin",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-const-leaves [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-leaves>\n"
    );
}

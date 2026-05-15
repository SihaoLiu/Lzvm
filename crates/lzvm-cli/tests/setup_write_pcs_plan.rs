use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::pcs_plan::{derive_pcs_setup_plan, read_pcs_setup_plan_file};
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

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-write-pcs-plan-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_pcs_setup_plan_from_binary_setup_metadata() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let out_path = dir.join("unit.pcs-plan");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let expected = derive_pcs_setup_plan(&setup).expect("plan should derive");
    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-pcs-plan",
            setup_path.to_str().expect("setup path should be utf-8"),
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let parsed = read_pcs_setup_plan_file(&out_path).expect("PCS plan output should parse");
    let byte_count = fs::metadata(&out_path)
        .expect("PCS plan output should exist")
        .len();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(parsed, expected);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written={byte_count}\noutput={}\n",
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_pcs_setup_plan_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &["setup", "write-pcs-plan", "unit.setup.bin"],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-pcs-plan <setup-info-bin> <out-pcs-plan>\n"
    );
}

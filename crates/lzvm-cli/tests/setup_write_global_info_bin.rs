use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::global_info::{encode_global_info, parse_global_info_json};
use lzvm_cli::run_cli;

fn sample_global_info_json() -> &'static str {
    r#"{
        "name": "sample-program",
        "air_groups": ["group-a"],
        "airs": [[{"name": "unit-a", "num_rows": 2}]],
        "curve": "None",
        "latticeSize": 368,
        "aggTypes": [[]],
        "nPublics": 1,
        "numChallenges": [1],
        "numProofValues": [1],
        "proofValuesMap": [
            {"name": "proof-a", "stage": 2, "id": 7, "lengths": [2]}
        ],
        "publicsMap": [
            {"name": "public-a", "stage": 1, "lengths": [1]}
        ],
        "transcriptArity": 4
    }"#
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-write-global-info-bin-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_global_info_binary_from_json() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let global_json_path = dir.join("global.json");
    let out_path = dir.join("global.bin");
    let global = parse_global_info_json(sample_global_info_json()).expect("global should parse");
    let expected = encode_global_info(&global).expect("global should encode");
    fs::write(&global_json_path, sample_global_info_json())
        .expect("global fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-global-info-bin",
            global_json_path
                .to_str()
                .expect("global path should be utf-8"),
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let actual = fs::read(&out_path).expect("binary global output should be written");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(actual, expected);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written={}\noutput={}\n",
            expected.len(),
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_global_info_binary_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &["setup", "write-global-info-bin", "global.json"],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-global-info-bin <global-info-json> <out-global-info-bin>\n"
    );
}

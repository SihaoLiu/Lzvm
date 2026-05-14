use lzvm_artifacts::global_info::{
    parse_global_info_json, read_global_info_file, CurveKind, GlobalInfoError,
};
use std::fs;
use std::path::PathBuf;

fn sample_global_info_json() -> &'static str {
    r#"{
        "name": "sample-program",
        "air_groups": ["group-a", "group-b"],
        "airs": [
            [
                {"name": "unit-a", "num_rows": 1024},
                {"name": "unit-b", "num_rows": 2048, "hasCompressor": true}
            ],
            [
                {"name": "unit-c", "num_rows": 4096}
            ]
        ],
        "curve": "None",
        "latticeSize": 368,
        "aggTypes": [
            [{"aggType": 0}, {"aggType": 2}],
            []
        ],
        "nPublics": 2,
        "numChallenges": [1, 2, 3],
        "numProofValues": [1, 1],
        "proofValuesMap": [
            {"name": "proof-a", "stage": 1},
            {"name": "proof-b", "stage": 2}
        ],
        "publicsMap": [
            {"name": "public-a", "stage": 1},
            {"name": "public-b", "stage": 1, "lengths": [2, 3]}
        ],
        "transcriptArity": 4
    }"#
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-global-info-{}-{name}", std::process::id()))
}

#[test]
fn parses_global_info_json() {
    let info = parse_global_info_json(sample_global_info_json()).expect("fixture should parse");

    assert_eq!(info.name, "sample-program");
    assert_eq!(info.curve, CurveKind::None);
    assert_eq!(info.air_groups, vec!["group-a", "group-b"]);
    assert_eq!(info.airs.len(), 2);
    assert_eq!(info.airs[0][1].name, "unit-b");
    assert!(info.airs[0][1].has_compressor);
    assert_eq!(info.airs[1][0].num_rows, 4096);
    assert_eq!(info.aggregation_types[0][1].aggregation_type, 2);
    assert_eq!(info.n_publics, 2);
    assert_eq!(info.num_challenges, vec![1, 2, 3]);
    assert_eq!(info.proof_values_map.len(), 2);
    assert_eq!(info.stage_one_proof_value_count(), 1);
    assert_eq!(info.publics_map[1].lengths, vec![2, 3]);
    assert_eq!(info.transcript_arity, 4);
}

#[test]
fn rejects_missing_global_info_fields() {
    assert!(matches!(
        parse_global_info_json("{}"),
        Err(GlobalInfoError::MissingField { field: "name" })
    ));
}

#[test]
fn rejects_mismatched_air_group_counts() {
    let json = sample_global_info_json().replace(
        "\"air_groups\": [\"group-a\", \"group-b\"]",
        "\"air_groups\": [\"group-a\"]",
    );

    assert!(matches!(
        parse_global_info_json(&json),
        Err(GlobalInfoError::AirGroupCountMismatch {
            air_groups: 1,
            airs: 2,
            aggregation_types: 2
        })
    ));
}

#[test]
fn rejects_empty_air_groups() {
    let json = r#"{
        "name": "sample-program",
        "air_groups": ["group-a"],
        "airs": [[]],
        "curve": "None",
        "aggTypes": [[]],
        "nPublics": 0,
        "numChallenges": [0],
        "numProofValues": [],
        "proofValuesMap": [],
        "publicsMap": [],
        "transcriptArity": 4
    }"#;

    assert!(matches!(
        parse_global_info_json(json),
        Err(GlobalInfoError::EmptyAirGroup { airgroup_id: 0 })
    ));
}

#[test]
fn rejects_public_count_mismatches() {
    let json = sample_global_info_json().replace("\"nPublics\": 2", "\"nPublics\": 3");

    assert!(matches!(
        parse_global_info_json(&json),
        Err(GlobalInfoError::PublicCountMismatch {
            expected: 3,
            found: 2
        })
    ));
}

#[test]
fn rejects_invalid_transcript_arity() {
    let json =
        sample_global_info_json().replace("\"transcriptArity\": 4", "\"transcriptArity\": 0");

    assert!(matches!(
        parse_global_info_json(&json),
        Err(GlobalInfoError::InvalidTranscriptArity)
    ));
}

#[test]
fn reads_global_info_from_a_file_path() {
    let path = temp_file_path("global.json");
    fs::write(&path, sample_global_info_json()).expect("fixture should be written");

    let info = read_global_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(info.total_air_count(), 3);
}

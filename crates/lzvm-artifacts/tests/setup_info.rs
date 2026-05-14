use lzvm_artifacts::setup_info::{
    parse_unit_setup_info_json, read_unit_setup_info_file, SetupInfoError,
};
use std::fs;
use std::path::PathBuf;

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 2,
        "nConstants": 5,
        "nPublics": 3,
        "nConstraints": 8,
        "qDeg": 7,
        "openingPoints": [0, 1, -1],
        "mapSectionsN": {
            "const": 5,
            "cm1": 2,
            "cm2": 3,
            "cm3": 1
        },
        "challengesMap": [{}, {}],
        "evMap": [{}, {}, {}],
        "boundaries": [
            {"name": "first", "offsetMin": 0, "offsetMax": 3},
            {"offsetMin": -1}
        ],
        "starkStruct": {
            "nBits": 10,
            "nBitsExt": 13,
            "nQueries": 4,
            "steps": [
                {"nBits": 13},
                {"nBits": 9},
                {"nBits": 5}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 20,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-setup-info-{}-{name}", std::process::id()))
}

#[test]
fn parses_unit_setup_info_json() {
    let info = parse_unit_setup_info_json(sample_setup_info_json()).expect("fixture should parse");

    assert_eq!(info.n_stages, 2);
    assert_eq!(info.n_constants, 5);
    assert_eq!(info.n_publics, Some(3));
    assert_eq!(info.n_constraints, Some(8));
    assert_eq!(info.q_degree, 7);
    assert_eq!(info.opening_points, vec![0, 1, -1]);
    assert_eq!(info.challenge_count, 2);
    assert_eq!(info.eval_count, 3);
    assert_eq!(
        info.stage_commit_widths().expect("widths should exist"),
        vec![2, 3, 1]
    );
    assert_eq!(info.boundaries.len(), 2);
    assert_eq!(info.boundaries[0].name.as_deref(), Some("first"));
    assert_eq!(info.boundaries[1].offset_min, Some(-1));
    assert_eq!(info.stark.n_bits, 10);
    assert_eq!(info.stark.n_bits_ext, 13);
    assert_eq!(info.stark.steps.len(), 3);
    assert_eq!(info.stark.verification_hash_type.as_deref(), Some("GL"));
}

#[test]
fn rejects_missing_required_setup_fields() {
    assert!(matches!(
        parse_unit_setup_info_json("{}"),
        Err(SetupInfoError::MissingField { field: "nStages" })
    ));
}

#[test]
fn rejects_mismatched_domain_bits() {
    let json = sample_setup_info_json().replace("\"nBitsExt\": 13", "\"nBitsExt\": 9");

    assert!(matches!(
        parse_unit_setup_info_json(&json),
        Err(SetupInfoError::InvalidDomainBits {
            n_bits: 10,
            n_bits_ext: 9
        })
    ));
}

#[test]
fn rejects_fri_steps_that_do_not_start_at_extended_domain() {
    let json = sample_setup_info_json().replace("\"nBits\": 13", "\"nBits\": 12");

    assert!(matches!(
        parse_unit_setup_info_json(&json),
        Err(SetupInfoError::InvalidFriSteps)
    ));
}

#[test]
fn rejects_missing_stage_widths() {
    let json = sample_setup_info_json().replace("\"cm3\": 1", "\"other\": 1");

    assert!(matches!(
        parse_unit_setup_info_json(&json),
        Err(SetupInfoError::MissingSectionWidth { name }) if name == "cm3"
    ));
}

#[test]
fn reads_unit_setup_info_from_a_file_path() {
    let path = temp_file_path("unit.json");
    fs::write(&path, sample_setup_info_json()).expect("fixture should be written");

    let info = read_unit_setup_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(
        info.stage_commit_widths().expect("widths should exist"),
        vec![2, 3, 1]
    );
}

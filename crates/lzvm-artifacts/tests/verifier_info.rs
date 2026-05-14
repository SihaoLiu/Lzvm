use lzvm_artifacts::verifier_info::{
    parse_verifier_info_json, read_verifier_info_file, VerifierInfoError,
};
use std::fs;
use std::path::PathBuf;

fn sample_verifier_info_json() -> &'static str {
    r#"{
        "qVerifier": {
            "tmpUsed": 2,
            "line": "",
            "code": [
                {
                    "op": "mul",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [
                        {"type": "challenge", "id": 0, "stageId": 0, "dim": 3, "stage": 1},
                        {"type": "eval", "id": 2, "dim": 3}
                    ]
                },
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 1, "dim": 3},
                    "src": [{"type": "tmp", "id": 0, "dim": 3}]
                }
            ]
        },
        "queryVerifier": {
            "expId": 9,
            "stage": 3,
            "tmpUsed": 1,
            "line": "query-a",
            "code": [
                {
                    "op": "add",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [
                        {"type": "Zi", "boundaryId": 0, "dim": 1},
                        {"type": "proofvalue", "id": 1, "stage": 2, "dim": 3}
                    ]
                }
            ]
        }
    }"#
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-verifier-info-{}-{name}", std::process::id()))
}

#[test]
fn parses_verifier_info_json() {
    let info = parse_verifier_info_json(sample_verifier_info_json()).expect("fixture should parse");

    assert_eq!(info.quotient.temporary_count, 2);
    assert_eq!(info.quotient.operations.len(), 2);
    assert_eq!(info.query.expression_id, Some(9));
    assert_eq!(info.query.stage, Some(3));
    assert_eq!(info.query.line, "query-a");
    assert_eq!(info.query.operation_count(), 1);
}

#[test]
fn rejects_missing_verifier_blocks() {
    assert!(matches!(
        parse_verifier_info_json("{}"),
        Err(VerifierInfoError::MissingField { field: "qVerifier" })
    ));
}

#[test]
fn rejects_unknown_verifier_operations() {
    let json = sample_verifier_info_json().replace("\"op\": \"mul\"", "\"op\": \"unknown\"");

    assert!(matches!(
        parse_verifier_info_json(&json),
        Err(VerifierInfoError::UnknownOperation { .. })
    ));
}

#[test]
fn rejects_temporary_references_outside_declared_count() {
    let json =
        sample_verifier_info_json().replace("\"id\": 1, \"dim\": 3", "\"id\": 2, \"dim\": 3");

    assert!(matches!(
        parse_verifier_info_json(&json),
        Err(VerifierInfoError::TemporaryReferenceOutOfBounds {
            temporary_id: 2,
            temporary_count: 2
        })
    ));
}

#[test]
fn rejects_empty_verifier_code_blocks() {
    let json = r#"{
        "qVerifier": {"tmpUsed": 0, "code": []},
        "queryVerifier": {"tmpUsed": 1, "code": [{"op": "copy", "dest": {"type": "tmp", "id": 0}, "src": [{"type": "number", "value": "1"}]}]}
    }"#;

    assert!(matches!(
        parse_verifier_info_json(json),
        Err(VerifierInfoError::EmptyCodeBlock { field: "qVerifier" })
    ));
}

#[test]
fn reads_verifier_info_from_a_file_path() {
    let path = temp_file_path("verifier.json");
    fs::write(&path, sample_verifier_info_json()).expect("fixture should be written");

    let info = read_verifier_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(info.quotient.operation_count(), 2);
}

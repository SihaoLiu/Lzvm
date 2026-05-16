#[cfg(feature = "json")]
use lzvm_artifacts::verifier_info::parse_verifier_info_json;
use lzvm_artifacts::verifier_info::{
    encode_verifier_info, parse_verifier_info, read_verifier_info_binary_file,
    read_verifier_info_file, VerifierInfoError,
};
use std::fs;
use std::path::PathBuf;

mod fixtures;

#[cfg(feature = "json")]
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
#[cfg(feature = "json")]
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
#[cfg(feature = "json")]
fn rejects_missing_verifier_blocks() {
    assert!(matches!(
        parse_verifier_info_json("{}"),
        Err(VerifierInfoError::MissingField { field: "qVerifier" })
    ));
}

#[test]
#[cfg(feature = "json")]
fn rejects_unknown_verifier_operations() {
    let json = sample_verifier_info_json().replace("\"op\": \"mul\"", "\"op\": \"unknown\"");

    assert!(matches!(
        parse_verifier_info_json(&json),
        Err(VerifierInfoError::UnknownOperation { .. })
    ));
}

#[test]
#[cfg(feature = "json")]
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
#[cfg(feature = "json")]
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
    let info = fixtures::sample_verifier_info_fixture();
    let bytes = encode_verifier_info(&info).expect("fixture should encode");
    let path = temp_file_path("verifier.generic.bin");
    fs::write(&path, bytes).expect("fixture should be written");

    let info = read_verifier_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(info.quotient.operation_count(), 2);
}

#[test]
fn rejects_text_verifier_info_from_a_file_path() {
    let path = temp_file_path("verifier.json");
    fs::write(&path, "not a binary file").expect("fixture should be written");

    let error = read_verifier_info_file(&path).expect_err("text metadata should be rejected");
    fs::remove_file(&path).expect("fixture should be removed");

    assert!(matches!(error, VerifierInfoError::InvalidMagic));
}

#[test]
fn encodes_and_parses_verifier_info_binary() {
    let info = fixtures::sample_verifier_info_fixture();
    let bytes = encode_verifier_info(&info).expect("fixture should encode");

    let parsed = parse_verifier_info(&bytes).expect("binary fixture should parse");

    assert_eq!(parsed, info);
}

#[test]
fn encodes_the_current_verifier_info_format_version() {
    let info = fixtures::sample_verifier_info_fixture();
    let bytes = encode_verifier_info(&info).expect("fixture should encode");
    let version = u32::from_le_bytes(bytes[4..8].try_into().expect("slice length checked"));

    assert_eq!(version, 2);
}

#[test]
fn rejects_stale_verifier_info_format_headers() {
    let info = fixtures::sample_verifier_info_fixture();
    let mut bytes = encode_verifier_info(&info).expect("fixture should encode");
    bytes[4..8].copy_from_slice(&1_u32.to_le_bytes());

    let error = parse_verifier_info(&bytes).expect_err("stale format should be rejected");

    assert!(matches!(
        error,
        VerifierInfoError::UnsupportedVersion { found: 1, max: 2 }
    ));
}

#[test]
fn reads_verifier_info_binary_from_a_file_path() {
    let info = fixtures::sample_verifier_info_fixture();
    let bytes = encode_verifier_info(&info).expect("fixture should encode");
    let path = temp_file_path("verifier.bin");
    fs::write(&path, bytes).expect("fixture should be written");

    let direct = read_verifier_info_binary_file(&path).expect("binary fixture should parse");
    let inferred = read_verifier_info_file(&path).expect("binary fixture should parse by suffix");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(direct, info);
    assert_eq!(inferred, info);
}

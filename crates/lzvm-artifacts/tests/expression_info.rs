use lzvm_artifacts::expression_info::{
    encode_expression_info, parse_expression_info, parse_expression_info_json,
    read_expression_info_binary_file, read_expression_info_file, BoundaryKind, CodeDestination,
    CodeOperand, ExpressionInfoError,
};
use std::fs;
use std::path::PathBuf;

fn sample_expression_info_json() -> &'static str {
    r#"{
        "hintsInfo": [
            {
                "name": "hint-a",
                "fields": [
                    {
                        "name": "field-a",
                        "values": [
                            {"op": "number", "value": 7, "pos": [0]},
                            {"op": "string", "string": "tag", "pos": []},
                            {"op": "tmp", "id": 3, "dim": 1, "pos": [1, 2]}
                        ]
                    }
                ]
            }
        ],
        "expressionsCode": [
            {
                "expId": 4,
                "stage": 1,
                "line": "expr-a",
                "tmpUsed": 2,
                "dest": {"op": "cm", "stage": 1, "stageId": 0, "id": 8},
                "code": [
                    {
                        "op": "add",
                        "dest": {"type": "tmp", "id": 0, "dim": 1},
                        "src": [
                            {"type": "number", "value": "3", "dim": 1},
                            {"type": "public", "id": 0, "dim": 1}
                        ]
                    },
                    {
                        "op": "copy",
                        "dest": {"type": "tmp", "id": 1, "dim": 1},
                        "src": [{"type": "tmp", "id": 0, "dim": 1}]
                    }
                ]
            }
        ],
        "constraints": [
            {
                "tmpUsed": 1,
                "code": [
                    {
                        "op": "mul",
                        "dest": {"type": "tmp", "id": 0, "dim": 3},
                        "src": [
                            {"type": "challenge", "id": 0, "stageId": 0, "dim": 3, "stage": 1},
                            {"type": "cm", "id": 2, "prime": 0, "dim": 3}
                        ]
                    }
                ],
                "boundary": "everyFrame",
                "offsetMin": -1,
                "offsetMax": 2,
                "line": "constraint-a",
                "imPol": 1,
                "stage": 2
            }
        ]
    }"#
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-expression-info-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn parses_expression_info_json() {
    let info =
        parse_expression_info_json(sample_expression_info_json()).expect("fixture should parse");

    assert_eq!(info.hints.len(), 1);
    assert_eq!(info.hints[0].fields[0].values.len(), 3);
    assert_eq!(info.expressions.len(), 1);
    assert_eq!(info.expressions[0].expression_id, 4);
    assert_eq!(info.expressions[0].temporary_count, 2);
    assert_eq!(info.expressions[0].operations.len(), 2);
    assert_eq!(info.expressions[0].line, "expr-a");
    assert_eq!(
        info.expressions[0].operations[0].destination,
        CodeDestination::temporary(0, 1)
    );
    assert_eq!(
        info.expressions[0].operations[0].sources,
        vec![CodeOperand::number(3, 1), CodeOperand::public(0, 1)]
    );
    assert_eq!(info.constraints.len(), 1);
    assert_eq!(info.constraints[0].boundary, BoundaryKind::EveryFrame);
    assert_eq!(info.constraints[0].offset_min, Some(-1));
    assert!(info.constraints[0].intermediate);
    assert_eq!(
        info.constraints[0].operations[0].sources,
        vec![
            CodeOperand::challenge(0, Some(1), Some(0), 3),
            CodeOperand::commitment(2, 3),
        ]
    );
}

#[test]
fn rejects_missing_expression_info_arrays() {
    assert!(matches!(
        parse_expression_info_json("{}"),
        Err(ExpressionInfoError::MissingField { field: "hintsInfo" })
    ));
}

#[test]
fn rejects_duplicate_expression_ids() {
    let json = r#"{
        "hintsInfo": [],
        "expressionsCode": [
            {"expId": 4, "tmpUsed": 0, "code": []},
            {"expId": 4, "tmpUsed": 0, "code": []}
        ],
        "constraints": []
    }"#;

    assert!(matches!(
        parse_expression_info_json(json),
        Err(ExpressionInfoError::DuplicateExpressionId { expression_id: 4 })
    ));
}

#[test]
fn rejects_unknown_operations() {
    let json = sample_expression_info_json().replace("\"op\": \"add\"", "\"op\": \"unknown\"");

    assert!(matches!(
        parse_expression_info_json(&json),
        Err(ExpressionInfoError::UnknownOperation { .. })
    ));
}

#[test]
fn rejects_temporary_references_outside_the_declared_count() {
    let json =
        sample_expression_info_json().replace("\"id\": 1, \"dim\": 1", "\"id\": 2, \"dim\": 1");

    assert!(matches!(
        parse_expression_info_json(&json),
        Err(ExpressionInfoError::TemporaryReferenceOutOfBounds {
            temporary_id: 2,
            temporary_count: 2
        })
    ));
}

#[test]
fn rejects_frame_boundaries_without_offsets() {
    let json = sample_expression_info_json()
        .replace("\"offsetMin\": -1,\n                \"offsetMax\": 2,", "");

    assert!(matches!(
        parse_expression_info_json(&json),
        Err(ExpressionInfoError::MissingFrameBoundaryOffsets)
    ));
}

#[test]
fn reads_expression_info_from_a_file_path() {
    let info =
        parse_expression_info_json(sample_expression_info_json()).expect("fixture should parse");
    let bytes = encode_expression_info(&info).expect("fixture should encode");
    let path = temp_file_path("expressions.generic.bin");
    fs::write(&path, bytes).expect("fixture should be written");

    let info = read_expression_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(info.expressions[0].operation_count(), 2);
}

#[test]
fn rejects_text_expression_info_from_a_file_path() {
    let path = temp_file_path("expressions.json");
    fs::write(&path, sample_expression_info_json()).expect("fixture should be written");

    let error = read_expression_info_file(&path).expect_err("text metadata should be rejected");
    fs::remove_file(&path).expect("fixture should be removed");

    assert!(matches!(error, ExpressionInfoError::InvalidMagic));
}

#[test]
fn encodes_and_parses_expression_info_binary() {
    let info =
        parse_expression_info_json(sample_expression_info_json()).expect("fixture should parse");
    let bytes = encode_expression_info(&info).expect("fixture should encode");

    let parsed = parse_expression_info(&bytes).expect("binary fixture should parse");

    assert_eq!(parsed, info);
}

#[test]
fn encodes_the_current_expression_info_format_version() {
    let info =
        parse_expression_info_json(sample_expression_info_json()).expect("fixture should parse");
    let bytes = encode_expression_info(&info).expect("fixture should encode");
    let version = u32::from_le_bytes(bytes[4..8].try_into().expect("slice length checked"));

    assert_eq!(version, 2);
}

#[test]
fn rejects_stale_expression_info_format_headers() {
    let info =
        parse_expression_info_json(sample_expression_info_json()).expect("fixture should parse");
    let mut bytes = encode_expression_info(&info).expect("fixture should encode");
    bytes[4..8].copy_from_slice(&1_u32.to_le_bytes());

    let error = parse_expression_info(&bytes).expect_err("stale format should be rejected");

    assert!(matches!(
        error,
        ExpressionInfoError::UnsupportedVersion { found: 1, max: 2 }
    ));
}

#[test]
fn reads_expression_info_binary_from_a_file_path() {
    let info =
        parse_expression_info_json(sample_expression_info_json()).expect("fixture should parse");
    let bytes = encode_expression_info(&info).expect("fixture should encode");
    let path = temp_file_path("expressions.bin");
    fs::write(&path, bytes).expect("fixture should be written");

    let direct = read_expression_info_binary_file(&path).expect("binary fixture should parse");
    let inferred = read_expression_info_file(&path).expect("binary fixture should parse by suffix");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(direct, info);
    assert_eq!(inferred, info);
}

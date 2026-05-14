use lzvm_artifacts::expression_info::{
    parse_expression_info_json, read_expression_info_file, BoundaryKind, ExpressionInfoError,
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
    assert_eq!(info.constraints.len(), 1);
    assert_eq!(info.constraints[0].boundary, BoundaryKind::EveryFrame);
    assert_eq!(info.constraints[0].offset_min, Some(-1));
    assert_eq!(info.constraints[0].intermediate, true);
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
    let path = temp_file_path("expressions.json");
    fs::write(&path, sample_expression_info_json()).expect("fixture should be written");

    let info = read_expression_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(info.expressions[0].operation_count(), 2);
}

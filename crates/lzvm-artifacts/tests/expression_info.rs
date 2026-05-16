use lzvm_artifacts::expression_info::{
    encode_expression_info, parse_expression_info, read_expression_info_binary_file,
    read_expression_info_file, ExpressionInfoError,
};
#[cfg(feature = "json")]
use lzvm_artifacts::expression_info::{
    parse_expression_info_json, BoundaryKind, CodeDestination, CodeOperand, ExpressionDestination,
    HintPayload,
};
use std::fs;
use std::path::PathBuf;

mod fixtures;

#[cfg(feature = "json")]
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
#[cfg(feature = "json")]
fn parses_expression_info_json() {
    let info =
        parse_expression_info_json(sample_expression_info_json()).expect("fixture should parse");

    assert_eq!(info.hints.len(), 1);
    assert_eq!(info.hints[0].fields[0].values.len(), 3);
    assert_eq!(
        info.hints[0].fields[0].values[0].payload,
        HintPayload::number(7)
    );
    assert_eq!(
        info.hints[0].fields[0].values[1].payload,
        HintPayload::string("tag")
    );
    assert_eq!(
        info.hints[0].fields[0].values[2].payload,
        HintPayload::temporary(3, Some(1))
    );
    assert_eq!(info.hints[0].fields[0].values[2].positions, vec![1, 2]);
    assert_eq!(info.expressions.len(), 1);
    assert_eq!(info.expressions[0].expression_id, 4);
    assert_eq!(info.expressions[0].temporary_count, 2);
    assert_eq!(info.expressions[0].operations.len(), 2);
    assert_eq!(info.expressions[0].line, "expr-a");
    assert_eq!(
        info.expressions[0].destination,
        Some(ExpressionDestination::commitment(8, Some(1), Some(0)))
    );
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
            CodeOperand::commitment_at(2, Some(0), 3),
        ]
    );
}

#[test]
#[cfg(feature = "json")]
fn parses_native_expression_hint_payloads() {
    let json = r#"{
        "hintsInfo": [
            {
                "name": "hint-a",
                "fields": [
                    {
                        "name": "field-a",
                        "values": [
                            {"op": "cm", "id": 2, "rowOffsetIndex": 1, "stage": 2, "stageId": 0, "rowOffset": -1, "dim": 3, "airgroupId": 4, "airId": 5, "pos": [0]},
                            {"op": "custom", "id": 3, "commitId": 6, "rowOffsetIndex": 2, "stage": 2, "stageId": 1, "rowOffset": 0, "dim": 3, "airgroupId": 4, "airId": 5, "pos": [1]},
                            {"op": "const", "id": 4, "rowOffsetIndex": 0, "rowOffset": 1, "dim": 1, "airgroupId": 4, "airId": 5, "pos": [2]},
                            {"op": "challenge", "id": 5, "stage": 3, "stageId": 2, "pos": [3]},
                            {"op": "public", "id": 6, "stage": 1, "pos": [4]},
                            {"op": "airgroupvalue", "id": 7, "airgroupId": 8, "stage": 2, "dim": 3, "pos": [5]},
                            {"op": "airvalue", "id": 9, "stage": 2, "dim": 3, "pos": [6]},
                            {"op": "proofvalue", "id": 10, "stage": 2, "dim": 3, "pos": [7]}
                        ]
                    }
                ]
            }
        ],
        "expressionsCode": [],
        "constraints": []
    }"#;

    let info = parse_expression_info_json(json).expect("fixture should parse");

    let values = &info.hints[0].fields[0].values;
    assert_eq!(
        values[0].payload,
        HintPayload::Commitment {
            id: 2,
            row_offset_index: Some(1),
            row_offset: Some(-1),
            stage: Some(2),
            stage_id: Some(0),
            dimension: Some(3),
            air_group_id: Some(4),
            air_id: Some(5),
        }
    );
    assert_eq!(
        values[1].payload,
        HintPayload::CustomCommitment {
            id: 3,
            commit_id: Some(6),
            row_offset_index: Some(2),
            row_offset: Some(0),
            stage: Some(2),
            stage_id: Some(1),
            dimension: Some(3),
            air_group_id: Some(4),
            air_id: Some(5),
        }
    );
    assert_eq!(
        values[2].payload,
        HintPayload::constant(4, Some(0), Some(1), Some(1), Some(4), Some(5))
    );
    assert_eq!(
        values[3].payload,
        HintPayload::challenge(5, Some(3), Some(2))
    );
    assert_eq!(values[4].payload, HintPayload::public(6, Some(1)));
    assert_eq!(
        values[5].payload,
        HintPayload::air_group_value(7, Some(8), Some(2), Some(3))
    );
    assert_eq!(
        values[6].payload,
        HintPayload::air_value(9, Some(2), Some(3))
    );
    assert_eq!(
        values[7].payload,
        HintPayload::proof_value(10, Some(2), Some(3))
    );
    for (index, value) in values.iter().enumerate() {
        assert_eq!(value.positions, vec![index as u32]);
    }
}

#[test]
#[cfg(feature = "json")]
fn parses_non_temporary_expression_operation_references() {
    let json = r#"{
        "hintsInfo": [],
        "expressionsCode": [
            {
                "expId": 7,
                "tmpUsed": 0,
                "code": [
                    {
                        "op": "copy",
                        "dest": {"type": "q", "id": 0, "dim": 3},
                        "src": [{"type": "airgroupvalue", "id": 2, "stage": 1, "airgroupId": 3, "dim": 3}]
                    },
                    {
                        "op": "copy",
                        "dest": {"type": "f", "id": 0, "dim": 3},
                        "src": [{"type": "xDivXSubXi", "id": 4, "opening": 1, "dim": 3}]
                    }
                ]
            }
        ],
        "constraints": []
    }"#;

    let info = parse_expression_info_json(json).expect("fixture should parse");

    assert_eq!(
        info.expressions[0].operations[0].destination,
        CodeDestination::quotient(0, 3)
    );
    assert_eq!(
        info.expressions[0].operations[0].sources,
        vec![CodeOperand::air_group_value(2, Some(1), Some(3), 3)]
    );
    assert_eq!(
        info.expressions[0].operations[1].destination,
        CodeDestination::fri_expression(0, 3)
    );
    assert_eq!(
        info.expressions[0].operations[1].sources,
        vec![CodeOperand::opening_denominator(4, Some(1), 3)]
    );
}

#[test]
#[cfg(feature = "json")]
fn rejects_missing_expression_info_arrays() {
    assert!(matches!(
        parse_expression_info_json("{}"),
        Err(ExpressionInfoError::MissingField { field: "hintsInfo" })
    ));
}

#[test]
#[cfg(feature = "json")]
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
#[cfg(feature = "json")]
fn rejects_unknown_operations() {
    let json = sample_expression_info_json().replace("\"op\": \"add\"", "\"op\": \"unknown\"");

    assert!(matches!(
        parse_expression_info_json(&json),
        Err(ExpressionInfoError::UnknownOperation { .. })
    ));
}

#[test]
#[cfg(feature = "json")]
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
#[cfg(feature = "json")]
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
    let info = fixtures::sample_expression_info_fixture();
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
    fs::write(&path, "not a binary file").expect("fixture should be written");

    let error = read_expression_info_file(&path).expect_err("text metadata should be rejected");
    fs::remove_file(&path).expect("fixture should be removed");

    assert!(matches!(error, ExpressionInfoError::InvalidMagic));
}

#[test]
fn encodes_and_parses_expression_info_binary() {
    let info = fixtures::sample_expression_info_fixture();
    let bytes = encode_expression_info(&info).expect("fixture should encode");

    let parsed = parse_expression_info(&bytes).expect("binary fixture should parse");

    assert_eq!(parsed, info);
}

#[test]
fn encodes_the_current_expression_info_format_version() {
    let info = fixtures::sample_expression_info_fixture();
    let bytes = encode_expression_info(&info).expect("fixture should encode");
    let version = u32::from_le_bytes(bytes[4..8].try_into().expect("slice length checked"));

    assert_eq!(version, 5);
}

#[test]
fn rejects_stale_expression_info_format_headers() {
    let info = fixtures::sample_expression_info_fixture();
    let mut bytes = encode_expression_info(&info).expect("fixture should encode");
    bytes[4..8].copy_from_slice(&4_u32.to_le_bytes());

    let error = parse_expression_info(&bytes).expect_err("stale format should be rejected");

    assert!(matches!(
        error,
        ExpressionInfoError::UnsupportedVersion { found: 4, max: 5 }
    ));
}

#[test]
fn reads_expression_info_binary_from_a_file_path() {
    let info = fixtures::sample_expression_info_fixture();
    let bytes = encode_expression_info(&info).expect("fixture should encode");
    let path = temp_file_path("expressions.bin");
    fs::write(&path, bytes).expect("fixture should be written");

    let direct = read_expression_info_binary_file(&path).expect("binary fixture should parse");
    let inferred = read_expression_info_file(&path).expect("binary fixture should parse by suffix");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(direct, info);
    assert_eq!(inferred, info);
}

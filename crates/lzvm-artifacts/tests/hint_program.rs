use lzvm_artifacts::expression_info::{
    ConstraintCode, ExpressionCode, ExpressionInfo, HintFieldInfo as ExpressionHintFieldInfo,
    HintInfo as ExpressionHintInfo, HintPayload, HintValueInfo,
};
use lzvm_artifacts::hint_program::{
    encode_global_hint_program, encode_regular_hint_program,
    global_hint_program_from_expression_info, parse_global_hint_program,
    parse_regular_hint_program, read_global_hint_program_file,
    regular_hint_program_from_expression_info, Hint, HintField, HintOperand, HintProgram,
    HintProgramError, HintValue,
};
use lzvm_artifacts::sectioned::{
    encode_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};
use lzvm_field::FieldError;
use std::fs;
use std::path::PathBuf;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;

fn sample_global_hint_program() -> HintProgram {
    HintProgram {
        hints: vec![Hint {
            name: "hint-a".to_owned(),
            fields: vec![HintField {
                name: "values".to_owned(),
                values: vec![
                    HintValue {
                        operand: HintOperand::Number(42),
                        positions: vec![0, 2],
                    },
                    HintValue {
                        operand: HintOperand::String("label".to_owned()),
                        positions: vec![],
                    },
                    HintValue {
                        operand: HintOperand::GroupValue { group_id: 7, id: 9 },
                        positions: vec![3],
                    },
                    HintValue {
                        operand: HintOperand::Temporary {
                            id: 11,
                            dimension: None,
                        },
                        positions: vec![4, 5],
                    },
                    HintValue {
                        operand: HintOperand::Public { id: 13 },
                        positions: vec![],
                    },
                    HintValue {
                        operand: HintOperand::ProofValue { id: 15 },
                        positions: vec![8],
                    },
                ],
            }],
        }],
    }
}

fn sample_regular_hint_program() -> HintProgram {
    HintProgram {
        hints: vec![Hint {
            name: "hint-a".to_owned(),
            fields: vec![HintField {
                name: "values".to_owned(),
                values: vec![
                    HintValue {
                        operand: HintOperand::Number(42),
                        positions: vec![0, 2],
                    },
                    HintValue {
                        operand: HintOperand::String("label".to_owned()),
                        positions: vec![],
                    },
                    HintValue {
                        operand: HintOperand::Commitment {
                            id: 2,
                            row_offset_index: 1,
                        },
                        positions: vec![3],
                    },
                    HintValue {
                        operand: HintOperand::CustomCommitment {
                            id: 3,
                            row_offset_index: 2,
                            commit_id: 6,
                        },
                        positions: vec![4],
                    },
                    HintValue {
                        operand: HintOperand::Constant {
                            id: 4,
                            row_offset_index: 0,
                        },
                        positions: vec![5],
                    },
                    HintValue {
                        operand: HintOperand::Challenge { id: 5 },
                        positions: vec![6],
                    },
                    HintValue {
                        operand: HintOperand::AirGroupValue { id: 7 },
                        positions: vec![8],
                    },
                    HintValue {
                        operand: HintOperand::AirValue { id: 9 },
                        positions: vec![10],
                    },
                    HintValue {
                        operand: HintOperand::Temporary {
                            id: 11,
                            dimension: Some(3),
                        },
                        positions: vec![12],
                    },
                    HintValue {
                        operand: HintOperand::Public { id: 13 },
                        positions: vec![14],
                    },
                    HintValue {
                        operand: HintOperand::ProofValue { id: 15 },
                        positions: vec![16],
                    },
                ],
            }],
        }],
    }
}

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-hint-program-{}-{name}", std::process::id()));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(value.as_bytes());
    out.push(0);
}

fn wrap_regular_hint_section(data: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection { id: 3, data }],
    })
    .expect("fixture should encode")
}

fn wrap_global_hint_section(data: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection { id: 2, data }],
    })
    .expect("fixture should encode")
}

fn number_value_section(value: u64) -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "hint");
    push_u32(&mut section, 1);
    push_string(&mut section, "field");
    push_u32(&mut section, 1);
    push_string(&mut section, "number");
    push_u64(&mut section, value);
    push_u32(&mut section, 0);
    section
}

fn unknown_operand_file() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "hint");
    push_u32(&mut section, 1);
    push_string(&mut section, "field");
    push_u32(&mut section, 1);
    push_string(&mut section, "unknown");

    wrap_regular_hint_section(section)
}

fn truncated_hint_file() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 1);

    wrap_regular_hint_section(section)
}

fn hint_count_file(hint_count: u32) -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, hint_count);
    wrap_regular_hint_section(section)
}

fn field_count_file(field_count: u32) -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "hint");
    push_u32(&mut section, field_count);
    wrap_regular_hint_section(section)
}

fn value_count_file(value_count: u32) -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "hint");
    push_u32(&mut section, 1);
    push_string(&mut section, "field");
    push_u32(&mut section, value_count);
    wrap_regular_hint_section(section)
}

fn position_count_file(position_count: u32) -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "hint");
    push_u32(&mut section, 1);
    push_string(&mut section, "field");
    push_u32(&mut section, 1);
    push_string(&mut section, "number");
    push_u32(&mut section, 42);
    push_u32(&mut section, 0);
    push_u32(&mut section, position_count);
    wrap_regular_hint_section(section)
}

fn regular_hint_file_with_extended_operands() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "hint-a");
    push_u32(&mut section, 1);
    push_string(&mut section, "values");
    push_u32(&mut section, 9);

    push_string(&mut section, "cm");
    push_u32(&mut section, 2);
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    push_u32(&mut section, 3);

    push_string(&mut section, "custom");
    push_u32(&mut section, 3);
    push_u32(&mut section, 2);
    push_u32(&mut section, 6);
    push_u32(&mut section, 1);
    push_u32(&mut section, 4);

    push_string(&mut section, "const");
    push_u32(&mut section, 4);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 5);

    push_string(&mut section, "challenge");
    push_u32(&mut section, 5);
    push_u32(&mut section, 1);
    push_u32(&mut section, 6);

    push_string(&mut section, "airgroupvalue");
    push_u32(&mut section, 7);
    push_u32(&mut section, 1);
    push_u32(&mut section, 8);

    push_string(&mut section, "airvalue");
    push_u32(&mut section, 9);
    push_u32(&mut section, 1);
    push_u32(&mut section, 10);

    push_string(&mut section, "tmp");
    push_u32(&mut section, 11);
    push_u32(&mut section, 3);
    push_u32(&mut section, 1);
    push_u32(&mut section, 12);

    push_string(&mut section, "public");
    push_u32(&mut section, 13);
    push_u32(&mut section, 1);
    push_u32(&mut section, 14);

    push_string(&mut section, "proofvalue");
    push_u32(&mut section, 15);
    push_u32(&mut section, 1);
    push_u32(&mut section, 16);

    wrap_regular_hint_section(section)
}

fn one_value_program(operand: HintOperand) -> HintProgram {
    HintProgram {
        hints: vec![Hint {
            name: "hint-a".to_owned(),
            fields: vec![HintField {
                name: "field-a".to_owned(),
                values: vec![HintValue {
                    operand,
                    positions: vec![0],
                }],
            }],
        }],
    }
}

fn expression_info_with_hint_payloads(values: Vec<HintValueInfo>) -> ExpressionInfo {
    ExpressionInfo {
        hints: vec![ExpressionHintInfo {
            name: "hint-a".to_owned(),
            fields: vec![ExpressionHintFieldInfo {
                name: "field-a".to_owned(),
                values,
            }],
        }],
        expressions: Vec::<ExpressionCode>::new(),
        constraints: Vec::<ConstraintCode>::new(),
    }
}

#[test]
fn parses_regular_hint_sections() {
    let encoded =
        encode_regular_hint_program(&sample_regular_hint_program()).expect("fixture should encode");
    let parsed = parse_regular_hint_program(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_regular_hint_program());
}

#[test]
fn rejects_unsupported_hint_program_versions() {
    let mut encoded =
        encode_regular_hint_program(&sample_regular_hint_program()).expect("fixture should encode");
    encoded[4..8].copy_from_slice(&0_u32.to_le_bytes());

    assert!(matches!(
        parse_regular_hint_program(&encoded),
        Err(HintProgramError::Sectioned(
            SectionedError::UnsupportedVersion { found: 0, max: 1 }
        ))
    ));
}

#[test]
fn parses_regular_hint_sections_from_binary_layout() {
    let parsed = parse_regular_hint_program(&regular_hint_file_with_extended_operands())
        .expect("fixture should parse");
    let expected = HintProgram {
        hints: vec![Hint {
            name: "hint-a".to_owned(),
            fields: vec![HintField {
                name: "values".to_owned(),
                values: sample_regular_hint_program().hints[0].fields[0].values[2..].to_vec(),
            }],
        }],
    };

    assert_eq!(parsed, expected);
}

#[test]
fn parses_global_hint_sections() {
    let encoded =
        encode_global_hint_program(&sample_global_hint_program()).expect("fixture should encode");
    let parsed = parse_global_hint_program(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_global_hint_program());
}

#[test]
fn builds_regular_hint_program_from_expression_info_payloads() {
    let info = expression_info_with_hint_payloads(vec![
        HintValueInfo {
            positions: vec![0],
            payload: HintPayload::number(42),
        },
        HintValueInfo {
            positions: vec![1],
            payload: HintPayload::string("label"),
        },
        HintValueInfo {
            positions: vec![2],
            payload: HintPayload::Commitment {
                id: 3,
                row_offset_index: Some(4),
                row_offset: Some(1),
                stage: Some(2),
                stage_id: Some(0),
                dimension: Some(1),
                air_group_id: Some(5),
                air_id: Some(6),
            },
        },
        HintValueInfo {
            positions: vec![3],
            payload: HintPayload::CustomCommitment {
                id: 7,
                commit_id: Some(8),
                row_offset_index: Some(9),
                row_offset: Some(-1),
                stage: Some(3),
                stage_id: Some(1),
                dimension: Some(3),
                air_group_id: Some(10),
                air_id: Some(11),
            },
        },
        HintValueInfo {
            positions: vec![4],
            payload: HintPayload::constant(12, Some(13), Some(0), Some(1), Some(14), Some(15)),
        },
        HintValueInfo {
            positions: vec![5],
            payload: HintPayload::challenge(16, Some(2), Some(1)),
        },
        HintValueInfo {
            positions: vec![6],
            payload: HintPayload::air_group_value(17, Some(18), Some(2), Some(3)),
        },
        HintValueInfo {
            positions: vec![7],
            payload: HintPayload::air_value(19, Some(2), Some(3)),
        },
        HintValueInfo {
            positions: vec![8],
            payload: HintPayload::temporary(20, Some(3)),
        },
        HintValueInfo {
            positions: vec![9],
            payload: HintPayload::public(21, Some(1)),
        },
        HintValueInfo {
            positions: vec![10],
            payload: HintPayload::proof_value(22, Some(2), Some(3)),
        },
        HintValueInfo {
            positions: vec![11],
            payload: HintPayload::commitment_element(23, 1, Some(24), Some(0), Some(1)),
        },
    ]);

    let program = regular_hint_program_from_expression_info(&info).expect("payloads should lower");
    let encoded = encode_regular_hint_program(&program).expect("program should encode");
    let parsed = parse_regular_hint_program(&encoded).expect("program should parse");

    assert_eq!(program, parsed);
    assert_eq!(
        program.hints[0].fields[0].values[2].operand,
        HintOperand::Commitment {
            id: 3,
            row_offset_index: 4
        }
    );
    assert_eq!(
        program.hints[0].fields[0].values[3].operand,
        HintOperand::CustomCommitment {
            id: 7,
            row_offset_index: 9,
            commit_id: 8
        }
    );
    assert_eq!(
        program.hints[0].fields[0].values[11].operand,
        HintOperand::CommitmentElement {
            id: 23,
            element: 1,
            row_offset_index: 24
        }
    );
}

#[test]
fn builds_global_hint_program_from_expression_info_payloads() {
    let info = expression_info_with_hint_payloads(vec![
        HintValueInfo {
            positions: vec![0],
            payload: HintPayload::number(42),
        },
        HintValueInfo {
            positions: vec![1],
            payload: HintPayload::string("label"),
        },
        HintValueInfo {
            positions: vec![2],
            payload: HintPayload::air_group_value(7, Some(9), Some(2), Some(3)),
        },
        HintValueInfo {
            positions: vec![3],
            payload: HintPayload::temporary(11, Some(3)),
        },
        HintValueInfo {
            positions: vec![4],
            payload: HintPayload::public(13, Some(1)),
        },
        HintValueInfo {
            positions: vec![5],
            payload: HintPayload::proof_value(15, Some(2), Some(3)),
        },
    ]);

    let program = global_hint_program_from_expression_info(&info).expect("payloads should lower");
    let encoded = encode_global_hint_program(&program).expect("program should encode");
    let parsed = parse_global_hint_program(&encoded).expect("program should parse");

    assert_eq!(program, parsed);
    assert_eq!(
        program.hints[0].fields[0].values[2].operand,
        HintOperand::GroupValue { group_id: 9, id: 7 }
    );
    assert_eq!(
        program.hints[0].fields[0].values[3].operand,
        HintOperand::Temporary {
            id: 11,
            dimension: None
        }
    );
}

#[test]
fn rejects_regular_expression_hint_payloads_without_row_offset_indexes() {
    let info = expression_info_with_hint_payloads(vec![HintValueInfo {
        positions: vec![0],
        payload: HintPayload::Commitment {
            id: 3,
            row_offset_index: None,
            row_offset: Some(1),
            stage: Some(2),
            stage_id: Some(0),
            dimension: Some(1),
            air_group_id: None,
            air_id: None,
        },
    }]);

    assert!(matches!(
        regular_hint_program_from_expression_info(&info),
        Err(HintProgramError::MissingOperandField {
            op: "cm",
            field: "row_offset_index"
        })
    ));
}

#[test]
fn rejects_global_expression_hint_payloads_without_group_ids() {
    let info = expression_info_with_hint_payloads(vec![HintValueInfo {
        positions: vec![0],
        payload: HintPayload::air_group_value(7, None, Some(2), Some(3)),
    }]);

    assert!(matches!(
        global_hint_program_from_expression_info(&info),
        Err(HintProgramError::MissingOperandField {
            op: "airgroupvalue",
            field: "air_group_id"
        })
    ));
}

#[test]
fn rejects_regular_only_expression_hint_payloads_in_global_programs() {
    let info = expression_info_with_hint_payloads(vec![HintValueInfo {
        positions: vec![0],
        payload: HintPayload::Commitment {
            id: 3,
            row_offset_index: Some(4),
            row_offset: Some(1),
            stage: Some(2),
            stage_id: Some(0),
            dimension: Some(1),
            air_group_id: None,
            air_id: None,
        },
    }]);

    assert!(matches!(
        global_hint_program_from_expression_info(&info),
        Err(HintProgramError::InvalidOperandSection {
            op: "cm",
            section: "global"
        })
    ));
}

#[test]
fn rejects_missing_regular_hint_sections() {
    let file = SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection {
            id: 2,
            data: vec![0, 1, 2],
        }],
    };
    let encoded = encode_sectioned_file(&file).expect("fixture should encode");

    assert!(matches!(
        parse_regular_hint_program(&encoded),
        Err(HintProgramError::MissingHintSection { section_id: 3 })
    ));
}

#[test]
fn rejects_unknown_hint_operands() {
    assert!(matches!(
        parse_regular_hint_program(&unknown_operand_file()),
        Err(HintProgramError::UnknownOperand { .. })
    ));
}

#[test]
fn rejects_truncated_hint_sections() {
    assert!(matches!(
        parse_regular_hint_program(&truncated_hint_file()),
        Err(HintProgramError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_hint_count_that_exceeds_remaining_hint_records() {
    assert!(matches!(
        parse_regular_hint_program(&hint_count_file(1)),
        Err(HintProgramError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_field_count_that_exceeds_remaining_field_records() {
    assert!(matches!(
        parse_regular_hint_program(&field_count_file(1)),
        Err(HintProgramError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_value_records() {
    assert!(matches!(
        parse_regular_hint_program(&value_count_file(1)),
        Err(HintProgramError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_position_count_that_exceeds_remaining_positions() {
    assert!(matches!(
        parse_regular_hint_program(&position_count_file(1)),
        Err(HintProgramError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_non_canonical_regular_hint_numbers() {
    let program = one_value_program(HintOperand::Number(NON_CANONICAL_FIELD));

    assert!(matches!(
        encode_regular_hint_program(&program),
        Err(HintProgramError::NumberNonCanonical {
            value_index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_non_canonical_regular_hint_numbers_when_parsing() {
    let bytes = wrap_regular_hint_section(number_value_section(NON_CANONICAL_FIELD));

    assert!(matches!(
        parse_regular_hint_program(&bytes),
        Err(HintProgramError::NumberNonCanonical {
            value_index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_non_canonical_global_hint_numbers() {
    let program = one_value_program(HintOperand::Number(NON_CANONICAL_FIELD));

    assert!(matches!(
        encode_global_hint_program(&program),
        Err(HintProgramError::NumberNonCanonical {
            value_index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_non_canonical_global_hint_numbers_when_parsing() {
    let bytes = wrap_global_hint_section(number_value_section(NON_CANONICAL_FIELD));

    assert!(matches!(
        parse_global_hint_program(&bytes),
        Err(HintProgramError::NumberNonCanonical {
            value_index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_global_only_hint_operands_in_regular_sections() {
    let program = one_value_program(HintOperand::GroupValue { group_id: 1, id: 2 });

    assert!(matches!(
        encode_regular_hint_program(&program),
        Err(HintProgramError::InvalidOperandSection {
            op: "airgroupvalue",
            section: "regular"
        })
    ));
}

#[test]
fn rejects_regular_only_hint_operands_in_global_sections() {
    let program = one_value_program(HintOperand::Commitment {
        id: 2,
        row_offset_index: 1,
    });

    assert!(matches!(
        encode_global_hint_program(&program),
        Err(HintProgramError::InvalidOperandSection {
            op: "cm",
            section: "global"
        })
    ));
}

#[test]
fn reads_global_hint_programs_from_a_file_path() {
    let path = temp_file_path("global.bin");
    fs::write(
        &path,
        encode_global_hint_program(&sample_global_hint_program()).expect("fixture should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_global_hint_program_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_global_hint_program());
}

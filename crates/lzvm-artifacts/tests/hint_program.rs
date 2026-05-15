use lzvm_artifacts::hint_program::{
    encode_global_hint_program, encode_regular_hint_program, parse_global_hint_program,
    parse_regular_hint_program, read_global_hint_program_file, Hint, HintField, HintOperand,
    HintProgram, HintProgramError, HintValue,
};
use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use std::fs;
use std::path::PathBuf;

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
    std::env::temp_dir().join(format!("lzvm-hint-program-{}-{name}", std::process::id()))
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
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

#[test]
fn parses_regular_hint_sections() {
    let encoded =
        encode_regular_hint_program(&sample_regular_hint_program()).expect("fixture should encode");
    let parsed = parse_regular_hint_program(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_regular_hint_program());
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
        Err(HintProgramError::MissingStringTerminator { .. })
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

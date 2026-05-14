use lzvm_artifacts::hint_program::{
    encode_global_hint_program, encode_regular_hint_program, parse_global_hint_program,
    parse_regular_hint_program, read_global_hint_program_file, Hint, HintField, HintOperand,
    HintProgram, HintProgramError, HintValue,
};
use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use std::fs;
use std::path::PathBuf;

fn sample_hint_program() -> HintProgram {
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
                        operand: HintOperand::Temporary { id: 11 },
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

#[test]
fn parses_regular_hint_sections() {
    let encoded =
        encode_regular_hint_program(&sample_hint_program()).expect("fixture should encode");
    let parsed = parse_regular_hint_program(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_hint_program());
}

#[test]
fn parses_global_hint_sections() {
    let encoded =
        encode_global_hint_program(&sample_hint_program()).expect("fixture should encode");
    let parsed = parse_global_hint_program(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_hint_program());
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
fn reads_global_hint_programs_from_a_file_path() {
    let path = temp_file_path("global.bin");
    fs::write(
        &path,
        encode_global_hint_program(&sample_hint_program()).expect("fixture should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_global_hint_program_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_hint_program());
}

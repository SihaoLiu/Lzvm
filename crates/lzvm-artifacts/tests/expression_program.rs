use lzvm_artifacts::expression_program::{
    encode_expression_program, parse_expression_program, read_expression_program_file,
    ExpressionEntry, ExpressionProgram, ExpressionProgramError,
};
use lzvm_artifacts::sectioned::{
    encode_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};
use lzvm_field::FieldError;
use std::fs;
use std::path::PathBuf;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;

fn sample_program() -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 2,
        max_tmp3: 3,
        max_args: 4,
        max_ops: 5,
        entries: vec![
            ExpressionEntry {
                expression_id: 17,
                destination_dimension: 3,
                destination_id: 9,
                stage: 2,
                temp1_count: 4,
                temp3_count: 5,
                ops_offset: 0,
                ops_count: 2,
                args_offset: 0,
                args_count: 3,
                source_line: "first".to_owned(),
            },
            ExpressionEntry {
                expression_id: 18,
                destination_dimension: 1,
                destination_id: 10,
                stage: 1,
                temp1_count: 6,
                temp3_count: 7,
                ops_offset: 2,
                ops_count: 1,
                args_offset: 3,
                args_count: 1,
                source_line: "second".to_owned(),
            },
        ],
        ops: vec![11, 12, 13],
        args: vec![21, 22, 23, 24],
        numbers: vec![31, 32],
    }
}

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-expression-program-{}-{name}",
            std::process::id()
        ));
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

fn wrap_expression_section(data: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection { id: 1, data }],
    })
    .expect("fixture should encode")
}

fn counted_section(ops_len: u32, args_len: u32, numbers_len: u32, entry_count: u32) -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, ops_len);
    push_u32(&mut section, args_len);
    push_u32(&mut section, numbers_len);
    push_u32(&mut section, entry_count);
    section
}

fn invalid_span_file() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 3);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);

    push_u32(&mut section, 42);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 3);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_string(&mut section, "bad span");

    section.extend_from_slice(&[1, 2, 3]);

    wrap_expression_section(section)
}

fn section_truncated_payload_file() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);

    wrap_expression_section(section)
}

#[test]
fn parses_expression_program_sections() {
    let encoded = encode_expression_program(&sample_program()).expect("fixture should encode");
    let parsed = parse_expression_program(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_program());
}

#[test]
fn rejects_unsupported_expression_program_versions() {
    let mut encoded = encode_expression_program(&sample_program()).expect("fixture should encode");
    encoded[4..8].copy_from_slice(&0_u32.to_le_bytes());

    assert!(matches!(
        parse_expression_program(&encoded),
        Err(ExpressionProgramError::Sectioned(
            SectionedError::UnsupportedVersion { found: 0, max: 1 }
        ))
    ));
}

#[test]
fn rejects_missing_expression_sections() {
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
        parse_expression_program(&encoded),
        Err(ExpressionProgramError::MissingExpressionSection)
    ));
}

#[test]
fn rejects_invalid_operation_spans() {
    assert!(matches!(
        parse_expression_program(&invalid_span_file()),
        Err(ExpressionProgramError::OperationSpanOutOfBounds { .. })
    ));
}

#[test]
fn rejects_truncated_expression_sections() {
    assert!(matches!(
        parse_expression_program(&section_truncated_payload_file()),
        Err(ExpressionProgramError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_entry_count_that_exceeds_remaining_entry_records() {
    let bytes = wrap_expression_section(counted_section(0, 0, 0, 1));

    assert!(matches!(
        parse_expression_program(&bytes),
        Err(ExpressionProgramError::LengthOverflow)
    ));
}

#[test]
fn rejects_args_count_that_exceeds_remaining_args() {
    let bytes = wrap_expression_section(counted_section(0, 1, 0, 0));

    assert!(matches!(
        parse_expression_program(&bytes),
        Err(ExpressionProgramError::LengthOverflow)
    ));
}

#[test]
fn rejects_numbers_count_that_exceeds_remaining_numbers() {
    let bytes = wrap_expression_section(counted_section(0, 0, 1, 0));

    assert!(matches!(
        parse_expression_program(&bytes),
        Err(ExpressionProgramError::LengthOverflow)
    ));
}

#[test]
fn rejects_non_canonical_expression_numbers() {
    let mut program = sample_program();
    program.numbers[1] = NON_CANONICAL_FIELD;

    assert!(matches!(
        encode_expression_program(&program),
        Err(ExpressionProgramError::NumberNonCanonical {
            index: 1,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_non_canonical_expression_numbers_when_parsing() {
    let mut section = counted_section(0, 0, 1, 0);
    push_u64(&mut section, NON_CANONICAL_FIELD);
    let bytes = wrap_expression_section(section);

    assert!(matches!(
        parse_expression_program(&bytes),
        Err(ExpressionProgramError::NumberNonCanonical {
            index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn reads_expression_programs_from_a_file_path() {
    let path = temp_file_path("program.bin");
    fs::write(
        &path,
        encode_expression_program(&sample_program()).expect("fixture should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_expression_program_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_program());
}

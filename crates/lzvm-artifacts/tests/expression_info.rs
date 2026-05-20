use lzvm_artifacts::expression_info::{
    encode_expression_info, parse_expression_info, read_expression_info_binary_file,
    read_expression_info_file, ExpressionInfoError,
};
use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use std::fs;
use std::path::PathBuf;

mod fixtures;

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-expression-info-{}-{name}",
        std::process::id()
    ))
}

fn expression_info_file(section: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"xinf",
        version: 8,
        sections: vec![SectionedSection {
            id: 1,
            data: section,
        }],
    })
    .expect("sectioned fixture should encode")
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    push_u32(
        out,
        u32::try_from(value.len()).expect("fixture string fits u32"),
    );
    out.extend_from_slice(value.as_bytes());
}

fn empty_hint_sections() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 0);
    section
}

fn empty_hint_and_expression_sections() -> Vec<u8> {
    let mut section = empty_hint_sections();
    push_u32(&mut section, 0);
    section
}

fn minimal_expression_prefix(operation_count: u32) -> Vec<u8> {
    let mut section = empty_hint_sections();
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    section.push(0);
    push_u32(&mut section, operation_count);
    section
}

fn minimal_operation_with_source_count(source_count: u32) -> Vec<u8> {
    let mut section = minimal_expression_prefix(1);
    section.push(1);
    section.push(1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, source_count);
    section
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

    assert_eq!(version, 8);
}

#[test]
fn rejects_stale_expression_info_format_headers() {
    let info = fixtures::sample_expression_info_fixture();
    let mut bytes = encode_expression_info(&info).expect("fixture should encode");
    bytes[4..8].copy_from_slice(&7_u32.to_le_bytes());

    let error = parse_expression_info(&bytes).expect_err("stale format should be rejected");

    assert!(matches!(
        error,
        ExpressionInfoError::UnsupportedVersion { found: 7, max: 8 }
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

#[test]
fn rejects_hint_count_that_exceeds_remaining_hint_records() {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    let bytes = expression_info_file(section);

    assert!(matches!(
        parse_expression_info(&bytes),
        Err(ExpressionInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_hint_field_count_that_exceeds_remaining_field_records() {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    let bytes = expression_info_file(section);

    assert!(matches!(
        parse_expression_info(&bytes),
        Err(ExpressionInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_hint_value_count_that_exceeds_remaining_value_records() {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    let bytes = expression_info_file(section);

    assert!(matches!(
        parse_expression_info(&bytes),
        Err(ExpressionInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_hint_position_count_that_exceeds_remaining_positions() {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    section.push(1);
    push_u64(&mut section, 7);
    push_u32(&mut section, 1);
    let bytes = expression_info_file(section);

    assert!(matches!(
        parse_expression_info(&bytes),
        Err(ExpressionInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_expression_count_that_exceeds_remaining_expression_records() {
    let mut section = empty_hint_sections();
    push_u32(&mut section, 1);
    let bytes = expression_info_file(section);

    assert!(matches!(
        parse_expression_info(&bytes),
        Err(ExpressionInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_constraint_count_that_exceeds_remaining_constraint_records() {
    let mut section = empty_hint_and_expression_sections();
    push_u32(&mut section, 1);
    let bytes = expression_info_file(section);

    assert!(matches!(
        parse_expression_info(&bytes),
        Err(ExpressionInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_operation_count_that_exceeds_remaining_operation_records() {
    let bytes = expression_info_file(minimal_expression_prefix(1));

    assert!(matches!(
        parse_expression_info(&bytes),
        Err(ExpressionInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_source_count_that_exceeds_remaining_source_operands() {
    let bytes = expression_info_file(minimal_operation_with_source_count(1));

    assert!(matches!(
        parse_expression_info(&bytes),
        Err(ExpressionInfoError::LengthOverflow)
    ));
}

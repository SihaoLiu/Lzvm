use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use lzvm_artifacts::verifier_info::{
    encode_verifier_info, parse_verifier_info, read_verifier_info_binary_file,
    read_verifier_info_file, VerifierInfoError, VerifierOperand,
};
use lzvm_field::FieldError;
use std::fs;
use std::path::PathBuf;

mod fixtures;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-verifier-info-{}-{name}", std::process::id()))
}

fn verifier_info_file(section: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"vinf",
        version: 2,
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

fn verifier_code_prefix(operation_count: u32) -> Vec<u8> {
    let mut section = Vec::new();
    section.push(0);
    section.push(0);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    push_u32(&mut section, operation_count);
    section
}

fn verifier_operation_with_source_count(source_count: u32) -> Vec<u8> {
    let mut section = verifier_code_prefix(1);
    section.push(1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, source_count);
    section
}

fn verifier_code_with_number(value: u64) -> Vec<u8> {
    let mut section = verifier_code_prefix(1);
    section.push(4);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    section.push(2);
    push_u64(&mut section, value);
    push_u32(&mut section, 1);
    section
}

fn verifier_info_with_number(value: u64) -> Vec<u8> {
    let mut section = verifier_code_with_number(value);
    section.extend_from_slice(&verifier_code_with_number(1));
    verifier_info_file(section)
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
fn rejects_non_canonical_verifier_numbers() {
    let mut info = fixtures::sample_verifier_info_fixture();
    info.quotient.operations[0].sources[0] = VerifierOperand::number(NON_CANONICAL_FIELD, 1);

    assert!(matches!(
        encode_verifier_info(&info),
        Err(VerifierInfoError::NumberNonCanonical {
            source_index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_non_canonical_verifier_numbers_when_parsing() {
    let bytes = verifier_info_with_number(NON_CANONICAL_FIELD);

    assert!(matches!(
        parse_verifier_info(&bytes),
        Err(VerifierInfoError::NumberNonCanonical {
            source_index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
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

#[test]
fn rejects_operation_count_that_exceeds_remaining_operation_records() {
    let bytes = verifier_info_file(verifier_code_prefix(1));

    assert!(matches!(
        parse_verifier_info(&bytes),
        Err(VerifierInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_source_count_that_exceeds_remaining_source_operands() {
    let bytes = verifier_info_file(verifier_operation_with_source_count(1));

    assert!(matches!(
        parse_verifier_info(&bytes),
        Err(VerifierInfoError::LengthOverflow)
    ));
}

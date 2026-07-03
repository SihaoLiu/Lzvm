use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use lzvm_artifacts::verifier_info::{
    encode_verifier_info, parse_verifier_info, read_verifier_info_binary_file,
    read_verifier_info_file, VerifierDestination, VerifierInfoError, VerifierOperand,
    VerifierOperation, VerifierOperationKind,
};
use lzvm_field::FieldError;
use std::fs;
use std::path::PathBuf;

mod fixtures;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const REFERENCE_BODY_BYTES: usize = 4 + 4;
const OPERATION_COUNT_END: usize = 1 + 1 + 4 + 4 + 4;
const OPERATION_MIN_BYTES: usize = 1 + REFERENCE_BODY_BYTES + 4;
const SOURCE_COUNT_END: usize = OPERATION_COUNT_END + 1 + REFERENCE_BODY_BYTES + 4;
const OPERAND_MIN_BYTES: usize = 1 + REFERENCE_BODY_BYTES;

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-verifier-info-{}-{name}", std::process::id()));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
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

fn verifier_code_with_destination_dimension(dimension: u32) -> Vec<u8> {
    let mut section = verifier_code_prefix(1);
    section.push(4);
    push_u32(&mut section, 0);
    push_u32(&mut section, dimension);
    push_u32(&mut section, 1);
    section.push(2);
    push_u64(&mut section, 1);
    push_u32(&mut section, 1);
    section
}

fn verifier_code_with_number_dimension(dimension: u32) -> Vec<u8> {
    let mut section = verifier_code_prefix(1);
    section.push(4);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    section.push(2);
    push_u64(&mut section, 1);
    push_u32(&mut section, dimension);
    section
}

fn verifier_code_with_temporary_read_before_write() -> Vec<u8> {
    let mut section = Vec::new();
    section.push(0);
    section.push(0);
    push_string(&mut section, "");
    push_u32(&mut section, 2);
    push_u32(&mut section, 2);

    section.push(4);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    section.push(2);
    push_u64(&mut section, 1);
    push_u32(&mut section, 1);

    section.push(1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 2);
    section.push(1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    section.push(1);
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);

    section
}

fn verifier_info_with_first_code(code: Vec<u8>) -> Vec<u8> {
    let mut section = code;
    section.extend_from_slice(&verifier_code_with_number(1));
    verifier_info_file(section)
}

fn verifier_info_with_number(value: u64) -> Vec<u8> {
    verifier_info_with_first_code(verifier_code_with_number(value))
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
fn rejects_zero_verifier_destination_dimensions() {
    let mut info = fixtures::sample_verifier_info_fixture();
    info.quotient.operations[0].destination = VerifierDestination::temporary(0, 0);

    assert!(matches!(
        encode_verifier_info(&info),
        Err(VerifierInfoError::ZeroDestinationDimension { temporary_id: 0 })
    ));
}

#[test]
fn rejects_zero_verifier_destination_dimensions_when_parsing() {
    let bytes = verifier_info_with_first_code(verifier_code_with_destination_dimension(0));

    assert!(matches!(
        parse_verifier_info(&bytes),
        Err(VerifierInfoError::ZeroDestinationDimension { temporary_id: 0 })
    ));
}

#[test]
fn rejects_zero_verifier_operand_dimensions() {
    let mut info = fixtures::sample_verifier_info_fixture();
    info.quotient.operations[0].sources[0] = VerifierOperand::number(1, 0);

    assert!(matches!(
        encode_verifier_info(&info),
        Err(VerifierInfoError::ZeroOperandDimension { source_index: 0 })
    ));
}

#[test]
fn rejects_zero_verifier_operand_dimensions_when_parsing() {
    let bytes = verifier_info_with_first_code(verifier_code_with_number_dimension(0));

    assert!(matches!(
        parse_verifier_info(&bytes),
        Err(VerifierInfoError::ZeroOperandDimension { source_index: 0 })
    ));
}

#[test]
fn rejects_temporary_sources_before_definition() {
    let mut info = fixtures::sample_verifier_info_fixture();
    info.quotient.temporary_count = 2;
    info.quotient.operations = vec![
        VerifierOperation {
            op: VerifierOperationKind::Copy,
            destination: VerifierDestination::temporary(0, 1),
            sources: vec![VerifierOperand::number(1, 1)],
        },
        VerifierOperation {
            op: VerifierOperationKind::Add,
            destination: VerifierDestination::temporary(0, 1),
            sources: vec![
                VerifierOperand::temporary(0, 1),
                VerifierOperand::temporary(1, 1),
            ],
        },
    ];

    assert!(matches!(
        encode_verifier_info(&info),
        Err(VerifierInfoError::TemporaryReadBeforeWrite {
            temporary_id: 1,
            dimension: 1,
            operation_index: 1
        })
    ));
}

#[test]
fn rejects_temporary_sources_before_definition_when_parsing() {
    let bytes = verifier_info_with_first_code(verifier_code_with_temporary_read_before_write());

    assert!(matches!(
        parse_verifier_info(&bytes),
        Err(VerifierInfoError::TemporaryReadBeforeWrite {
            temporary_id: 1,
            dimension: 1,
            operation_index: 1
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
        Err(VerifierInfoError::UnexpectedEof {
            offset: OPERATION_COUNT_END,
            needed: OPERATION_MIN_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_source_count_that_exceeds_remaining_source_operands() {
    let bytes = verifier_info_file(verifier_operation_with_source_count(1));

    assert!(matches!(
        parse_verifier_info(&bytes),
        Err(VerifierInfoError::UnexpectedEof {
            offset: SOURCE_COUNT_END,
            needed: OPERAND_MIN_BYTES,
            available: 0
        })
    ));
}

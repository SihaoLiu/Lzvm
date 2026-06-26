use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::public_values::{
    encode_public_values, parse_public_values, public_values_digest,
    read_public_values_binary_file, read_public_values_file, PublicValueEntry, PublicValues,
    PublicValuesError,
};
use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use lzvm_field::FieldError;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const SECTION_HEADER_BYTES: usize = 4 + 32 + 4;
const VALUE_ENTRY_HEADER_BYTES: usize = 4 + 4;
const ONE_BYTE_NAME_ELEMENT_COUNT_END: usize = SECTION_HEADER_BYTES + 4 + 1 + 4;
const ELEMENT_BYTES: usize = 8;

fn sample_hash(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-public-values-{}-{name}", std::process::id()));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

fn sample_public_values() -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x11),
        values: vec![
            PublicValueEntry {
                name: "block_number".to_owned(),
                elements: vec![12_345],
            },
            PublicValueEntry {
                name: "state_root_words".to_owned(),
                elements: vec![1, 2, 3, 4],
            },
        ],
    }
}

fn public_values_file(section: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"pval",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: section,
        }],
    })
    .expect("sectioned fixture should encode")
}

fn section_header(value_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 1);
    bytes.extend_from_slice(&sample_hash(0x11));
    push_u32(&mut bytes, value_count);
    bytes
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

#[test]
fn encodes_and_parses_public_values_binary() {
    let encoded = encode_public_values(&sample_public_values()).expect("fixture should encode");

    let parsed = parse_public_values(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_public_values());
    assert_eq!(
        public_values_digest(&parsed).expect("parsed fixture should digest"),
        public_values_digest(&sample_public_values()).expect("sample fixture should digest")
    );
}

#[test]
fn rejects_unsupported_public_values_file_versions() {
    let section = encode_public_values(&sample_public_values()).expect("fixture should encode");
    let parsed = lzvm_artifacts::sectioned::parse_sectioned_file(&section, *b"pval", 1)
        .expect("sectioned fixture should parse");
    let encoded = encode_sectioned_file(&SectionedFile {
        kind: *b"pval",
        version: 0,
        sections: parsed.sections,
    })
    .expect("sectioned fixture should encode");

    assert!(matches!(
        parse_public_values(&encoded),
        Err(PublicValuesError::UnsupportedVersion { found: 0, max: 1 })
    ));
}

#[test]
fn hashes_public_values_deterministically() {
    assert_eq!(
        public_values_digest(&sample_public_values()).expect("fixture should digest"),
        [
            0x60, 0xc9, 0xc6, 0x21, 0x03, 0x25, 0xca, 0xec, 0xe4, 0x9f, 0x23, 0x3d, 0xf3, 0xaf,
            0x82, 0x9a, 0x81, 0x04, 0x7c, 0xf4, 0x04, 0xca, 0x04, 0xd3, 0xf8, 0x29, 0x5d, 0x89,
            0xe8, 0x42, 0x43, 0x88,
        ]
    );
}

#[test]
fn reads_public_values_from_a_file_path() {
    let path = temp_file_path("values.pval");
    let encoded = encode_public_values(&sample_public_values()).expect("fixture should encode");
    fs::write(&path, encoded).expect("fixture should be written");

    let parsed = read_public_values_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_public_values());
}

#[test]
fn rejects_text_public_values_from_a_file_path() {
    let path = temp_file_path("values.json");
    fs::write(&path, "not a binary file").expect("fixture should be written");

    let error = read_public_values_file(&path).expect_err("text fixture should be rejected");
    fs::remove_file(&path).expect("fixture should be removed");

    assert!(matches!(error, PublicValuesError::InvalidMagic));
}

#[test]
fn reads_public_values_binary_from_a_file_path() {
    let path = temp_file_path("values.bin");
    let encoded = encode_public_values(&sample_public_values()).expect("fixture should encode");
    fs::write(&path, encoded).expect("fixture should be written");

    let direct = read_public_values_binary_file(&path).expect("binary fixture should parse");
    let parsed = read_public_values_file(&path).expect("binary fixture should dispatch");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(direct, sample_public_values());
    assert_eq!(parsed, sample_public_values());
}

#[test]
fn rejects_public_values_with_duplicate_names() {
    let mut value = sample_public_values();
    value.values.push(PublicValueEntry {
        name: "block_number".to_owned(),
        elements: vec![9],
    });

    assert!(matches!(
        encode_public_values(&value),
        Err(PublicValuesError::DuplicateName { .. })
    ));
}

#[test]
fn rejects_non_canonical_public_values() {
    let mut value = sample_public_values();
    value.values[1].elements[2] = NON_CANONICAL_FIELD;

    assert!(matches!(
        encode_public_values(&value),
        Err(PublicValuesError::ElementNonCanonical {
            name,
            element_index: 2,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        }) if name == "state_root_words"
    ));
}

#[test]
fn rejects_non_canonical_public_values_when_parsing() {
    let mut section = section_header(1);
    push_string(&mut section, "bad_value");
    push_u32(&mut section, 1);
    push_u64(&mut section, NON_CANONICAL_FIELD);
    let bytes = public_values_file(section);

    assert!(matches!(
        parse_public_values(&bytes),
        Err(PublicValuesError::ElementNonCanonical {
            name,
            element_index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        }) if name == "bad_value"
    ));
}

#[test]
fn rejects_unsupported_public_values_schema_version() {
    let mut value = sample_public_values();
    value.schema_version = 2;

    assert!(matches!(
        encode_public_values(&value),
        Err(PublicValuesError::UnsupportedSchemaVersion {
            found: 2,
            expected: 1
        })
    ));

    let mut section = section_header(1);
    section[0] = 2;
    push_string(&mut section, "block_number");
    push_u32(&mut section, 1);
    push_u64(&mut section, 12_345);
    let bytes = public_values_file(section);

    assert!(matches!(
        parse_public_values(&bytes),
        Err(PublicValuesError::UnsupportedSchemaVersion {
            found: 2,
            expected: 1
        })
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_entry_headers() {
    let bytes = public_values_file(section_header(1));

    assert!(matches!(
        parse_public_values(&bytes),
        Err(PublicValuesError::UnexpectedEof {
            offset: SECTION_HEADER_BYTES,
            needed: VALUE_ENTRY_HEADER_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_element_count_that_exceeds_remaining_elements() {
    let mut section = section_header(1);
    push_string(&mut section, "x");
    push_u32(&mut section, 1);
    let bytes = public_values_file(section);

    assert!(matches!(
        parse_public_values(&bytes),
        Err(PublicValuesError::UnexpectedEof {
            offset: ONE_BYTE_NAME_ELEMENT_COUNT_END,
            needed: ELEMENT_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_truncated_public_value_elements() {
    let mut section = section_header(1);
    push_string(&mut section, "x");
    push_u32(&mut section, 2);
    push_u64(&mut section, 7);
    let bytes = public_values_file(section);

    assert!(matches!(
        parse_public_values(&bytes),
        Err(PublicValuesError::UnexpectedEof {
            offset: ONE_BYTE_NAME_ELEMENT_COUNT_END,
            needed,
            available: ELEMENT_BYTES
        }) if needed == ELEMENT_BYTES * 2
    ));
}

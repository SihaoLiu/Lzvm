use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::public_values::{
    encode_public_values, parse_public_values, public_values_digest,
    read_public_values_binary_file, read_public_values_file, PublicValueEntry, PublicValues,
    PublicValuesError,
};

fn sample_hash(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-public-values-{}-{name}", std::process::id()))
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

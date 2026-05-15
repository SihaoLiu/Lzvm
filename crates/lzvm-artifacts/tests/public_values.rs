use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::public_values::{
    encode_public_values, encode_public_values_json, parse_public_values, parse_public_values_json,
    public_values_digest, read_public_values_binary_file, read_public_values_file,
    PublicValueEntry, PublicValues, PublicValuesError,
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
fn parses_public_values_json() {
    let input = r#"{
        "schema_version": 1,
        "setup_hash": "1111111111111111111111111111111111111111111111111111111111111111",
        "values": [
            {"name": "block_number", "elements": ["12345"]},
            {"name": "state_root_words", "elements": ["1", "2", "3", "4"]}
        ]
    }"#;

    let parsed = parse_public_values_json(input).expect("fixture should parse");

    assert_eq!(parsed, sample_public_values());
}

#[test]
fn encodes_public_values_json_canonically() {
    let encoded =
        encode_public_values_json(&sample_public_values()).expect("fixture should encode");

    assert_eq!(
        encoded,
        r#"{"schema_version":1,"setup_hash":"1111111111111111111111111111111111111111111111111111111111111111","values":[{"name":"block_number","elements":["12345"]},{"name":"state_root_words","elements":["1","2","3","4"]}]}"#
    );
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
    let encoded =
        encode_public_values_json(&sample_public_values()).expect("fixture should encode");
    fs::write(&path, encoded).expect("fixture should be written");

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
        encode_public_values_json(&value),
        Err(PublicValuesError::DuplicateName { .. })
    ));
}

#[test]
fn rejects_invalid_setup_hashes() {
    let input = r#"{
        "schema_version": 1,
        "setup_hash": "abcd",
        "values": []
    }"#;

    assert!(matches!(
        parse_public_values_json(input),
        Err(PublicValuesError::InvalidHash { .. })
    ));
}

#[test]
fn rejects_empty_public_value_entries() {
    let input = r#"{
        "schema_version": 1,
        "setup_hash": "1111111111111111111111111111111111111111111111111111111111111111",
        "values": [
            {"name": "empty", "elements": []}
        ]
    }"#;

    assert!(matches!(
        parse_public_values_json(input),
        Err(PublicValuesError::EmptyValue { .. })
    ));
}

use lzvm_artifacts::verification_key::{
    encode_verification_key_binary, encode_verification_key_json, parse_verification_key_binary,
    parse_verification_key_json, read_verification_key_binary_file,
    read_verification_key_json_file, VerificationKeyError, VerificationKeyRoot,
};
use std::fs;
use std::path::PathBuf;

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-verification-key-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn parses_field_root_json_arrays() {
    let parsed = parse_verification_key_json("[1,2,3,4]").expect("fixture should parse");

    assert_eq!(parsed, VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]));
}

#[test]
fn parses_decimal_scalar_json_strings() {
    let parsed = parse_verification_key_json("\"123456789\"").expect("fixture should parse");

    assert_eq!(
        parsed,
        VerificationKeyRoot::DecimalScalar("123456789".to_owned())
    );
}

#[test]
fn encodes_field_root_json_arrays() {
    let encoded = encode_verification_key_json(&VerificationKeyRoot::FieldElements(vec![
        17,
        18,
        u64::MAX,
        20,
    ]))
    .expect("fixture should encode");

    assert_eq!(encoded, "[17,18,18446744073709551615,20]");
}

#[test]
fn encodes_field_root_binary_little_endian() {
    let encoded = encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![
        1,
        0x1122_3344_5566_7788,
        u64::MAX,
        4,
    ]))
    .expect("fixture should encode");

    let mut expected = Vec::new();
    for value in [1_u64, 0x1122_3344_5566_7788, u64::MAX, 4] {
        expected.extend_from_slice(&value.to_le_bytes());
    }
    assert_eq!(encoded, expected);
}

#[test]
fn parses_field_root_binary_little_endian() {
    let mut bytes = Vec::new();
    for value in [9_u64, 10, 11, 12] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    assert_eq!(
        parse_verification_key_binary(&bytes).expect("fixture should parse"),
        VerificationKeyRoot::FieldElements(vec![9, 10, 11, 12])
    );
}

#[test]
fn rejects_binary_roots_with_the_wrong_size() {
    assert!(matches!(
        parse_verification_key_binary(&[1, 2, 3]),
        Err(VerificationKeyError::InvalidBinaryLength { .. })
    ));
}

#[test]
fn reads_verification_key_json_from_a_file_path() {
    let path = temp_file_path("root.json");
    fs::write(&path, "[5,6,7,8]").expect("fixture should be written");

    let root = read_verification_key_json_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(root, VerificationKeyRoot::FieldElements(vec![5, 6, 7, 8]));
}

#[test]
fn reads_verification_key_binary_from_a_file_path() {
    let path = temp_file_path("root.bin");
    let bytes =
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![13, 14, 15, 16]))
            .expect("fixture should encode");
    fs::write(&path, bytes).expect("fixture should be written");

    let root = read_verification_key_binary_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(
        root,
        VerificationKeyRoot::FieldElements(vec![13, 14, 15, 16])
    );
}

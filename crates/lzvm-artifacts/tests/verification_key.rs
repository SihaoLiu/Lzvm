use lzvm_artifacts::verification_key::{
    encode_verification_key_binary, parse_verification_key_binary,
    read_verification_key_binary_file, VerificationKeyError, VerificationKeyRoot,
};
use std::fs;
use std::path::PathBuf;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-verification-key-{}-{name}",
            std::process::id()
        ));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

#[test]
fn encodes_field_root_binary_little_endian() {
    let encoded = encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![
        1,
        0x1122_3344_5566_7788,
        NON_CANONICAL_FIELD - 1,
        4,
    ]))
    .expect("fixture should encode");

    let mut expected = Vec::new();
    for value in [1_u64, 0x1122_3344_5566_7788, NON_CANONICAL_FIELD - 1, 4] {
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
fn rejects_non_canonical_binary_root_words() {
    let mut bytes = Vec::new();
    for value in [9_u64, NON_CANONICAL_FIELD, 11, 12] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    let error = parse_verification_key_binary(&bytes)
        .expect_err("non-canonical binary root word should reject");

    assert_eq!(
        error.to_string(),
        "verification-key field element 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_encoding_non_canonical_root_words() {
    let error = encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![
        1,
        2,
        NON_CANONICAL_FIELD,
        4,
    ]))
    .expect_err("non-canonical root word should reject");

    assert_eq!(
        error.to_string(),
        "verification-key field element 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
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

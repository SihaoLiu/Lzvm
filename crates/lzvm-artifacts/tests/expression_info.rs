use lzvm_artifacts::expression_info::{
    encode_expression_info, parse_expression_info, read_expression_info_binary_file,
    read_expression_info_file, ExpressionInfoError,
};
use std::fs;
use std::path::PathBuf;

mod fixtures;

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-expression-info-{}-{name}",
        std::process::id()
    ))
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

    assert_eq!(version, 5);
}

#[test]
fn rejects_stale_expression_info_format_headers() {
    let info = fixtures::sample_expression_info_fixture();
    let mut bytes = encode_expression_info(&info).expect("fixture should encode");
    bytes[4..8].copy_from_slice(&4_u32.to_le_bytes());

    let error = parse_expression_info(&bytes).expect_err("stale format should be rejected");

    assert!(matches!(
        error,
        ExpressionInfoError::UnsupportedVersion { found: 4, max: 5 }
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

use lzvm_artifacts::global_info::{
    encode_global_info, parse_global_info, read_global_info_binary_file, read_global_info_file,
    GlobalInfoError,
};
use std::fs;
use std::path::PathBuf;

mod fixtures;

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-global-info-{}-{name}", std::process::id()))
}

#[test]
fn encodes_and_parses_global_info_binary() {
    let info = fixtures::sample_global_info_fixture();
    let bytes = encode_global_info(&info).expect("fixture should encode");

    let parsed = parse_global_info(&bytes).expect("binary fixture should parse");
    assert_eq!(parsed, info);

    let path = temp_file_path("global.generic.bin");
    fs::write(&path, &bytes).expect("binary fixture should be written");
    let from_file = read_global_info_binary_file(&path).expect("binary fixture should read");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(from_file, info);
}

#[test]
fn reads_global_info_from_a_file_path() {
    let info = fixtures::sample_global_info_fixture();
    let bytes = encode_global_info(&info).expect("fixture should encode");
    let path = temp_file_path("global.bin");
    fs::write(&path, bytes).expect("fixture should be written");

    let info = read_global_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(info.total_air_count(), 3);
}

#[test]
fn rejects_text_global_info_from_a_file_path() {
    let path = temp_file_path("global.json");
    fs::write(&path, "not a binary file").expect("fixture should be written");

    let error = read_global_info_file(&path).expect_err("text metadata should be rejected");
    fs::remove_file(&path).expect("fixture should be removed");

    assert!(matches!(error, GlobalInfoError::InvalidMagic));
}

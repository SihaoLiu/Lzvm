use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{
    encode_fixed_columns, parse_fixed_columns, read_fixed_columns_file, FixedColumnError,
};

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(value.as_bytes());
    out.push(0);
}

fn sample_file() -> Vec<u8> {
    let mut section = Vec::new();
    push_string(&mut section, "group-a");
    push_string(&mut section, "unit-a");
    push_u64(&mut section, 3);
    push_u32(&mut section, 2);

    push_string(&mut section, "main.flag");
    push_u32(&mut section, 0);
    for value in [1_u64, 2, 3] {
        push_u64(&mut section, value);
    }

    push_string(&mut section, "main.value");
    push_u32(&mut section, 2);
    push_u32(&mut section, 4);
    push_u32(&mut section, 7);
    for value in [10_u64, 11, 12] {
        push_u64(&mut section, value);
    }

    let mut file = Vec::new();
    file.extend_from_slice(b"cnst");
    push_u32(&mut file, 1);
    push_u32(&mut file, 1);
    push_u32(&mut file, 1);
    push_u64(&mut file, section.len() as u64);
    file.extend_from_slice(&section);
    file
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-artifacts-{}-{name}", std::process::id()))
}

#[test]
fn parses_fixed_columns_with_names_dimensions_and_values() {
    let parsed = parse_fixed_columns(&sample_file()).expect("fixture should parse");

    assert_eq!(parsed.group_name, "group-a");
    assert_eq!(parsed.unit_name, "unit-a");
    assert_eq!(parsed.row_count, 3);
    assert_eq!(parsed.columns.len(), 2);

    assert_eq!(parsed.columns[0].name, "main.flag");
    assert!(parsed.columns[0].dimensions.is_empty());
    assert_eq!(parsed.columns[0].values, [1, 2, 3]);

    assert_eq!(parsed.columns[1].name, "main.value");
    assert_eq!(parsed.columns[1].dimensions, [4, 7]);
    assert_eq!(parsed.columns[1].values, [10, 11, 12]);
}

#[test]
fn rejects_an_invalid_magic_header() {
    let mut bytes = sample_file();
    bytes[0] = b'x';

    assert!(matches!(
        parse_fixed_columns(&bytes),
        Err(FixedColumnError::InvalidMagic)
    ));
}

#[test]
fn rejects_truncated_column_values() {
    let mut bytes = sample_file();
    bytes.truncate(bytes.len() - 1);

    assert!(matches!(
        parse_fixed_columns(&bytes),
        Err(FixedColumnError::UnexpectedEof { .. })
    ));
}

#[test]
fn reads_fixed_columns_from_a_file_path() {
    let path = temp_file_path("fixed-columns.bin");
    fs::write(&path, sample_file()).expect("fixture should be written");

    let parsed = read_fixed_columns_file(&path).expect("fixture should parse from path");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed.group_name, "group-a");
    assert_eq!(parsed.unit_name, "unit-a");
    assert_eq!(parsed.row_count, 3);
}

#[test]
fn encodes_fixed_columns_to_the_canonical_binary_form() {
    let parsed = parse_fixed_columns(&sample_file()).expect("fixture should parse");
    let encoded = encode_fixed_columns(&parsed).expect("fixture should encode");

    assert_eq!(encoded, sample_file());
}

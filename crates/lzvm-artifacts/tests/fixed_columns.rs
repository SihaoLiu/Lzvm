use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{
    encode_fixed_columns, expected_raw_fixed_column_byte_count, parse_fixed_columns,
    parse_raw_fixed_columns, read_fixed_columns_file, read_fixed_columns_file_for_setup,
    FixedColumnError,
};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;

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

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 1,
        "nConstants": 2,
        "qDeg": 3,
        "openingPoints": [0],
        "mapSectionsN": {
            "const": 2,
            "cm1": 1,
            "cm2": 1
        },
        "constPolsMap": [
            {"stage": 0, "name": "main.left", "dim": 1, "polsMapId": 0, "stageId": 0},
            {"stage": 0, "name": "main.right", "dim": 1, "polsMapId": 1, "stageId": 1, "lengths": [2]}
        ],
        "challengesMap": [],
        "evMap": [],
        "boundaries": [],
        "starkStruct": {
            "nBits": 2,
            "nBitsExt": 3,
            "nQueries": 2,
            "steps": [
                {"nBits": 3},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 0,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_raw_file() -> Vec<u8> {
    let mut bytes = Vec::new();
    for (left, right) in [(1_u64, 10_u64), (2, 20), (3, 30), (4, 40)] {
        push_u64(&mut bytes, left);
        push_u64(&mut bytes, right);
    }
    bytes
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

#[test]
fn parses_raw_fixed_columns_using_setup_column_map() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let parsed = parse_raw_fixed_columns(&sample_raw_file(), &setup, "group-a", "unit-a")
        .expect("fixture should parse");

    assert_eq!(expected_raw_fixed_column_byte_count(&setup).unwrap(), 64);
    assert_eq!(parsed.group_name, "group-a");
    assert_eq!(parsed.unit_name, "unit-a");
    assert_eq!(parsed.row_count, 4);
    assert_eq!(parsed.columns.len(), 2);
    assert_eq!(parsed.columns[0].name, "main.left");
    assert_eq!(parsed.columns[0].values, [1, 2, 3, 4]);
    assert_eq!(parsed.columns[1].name, "main.right");
    assert_eq!(parsed.columns[1].dimensions, [2]);
    assert_eq!(parsed.columns[1].values, [10, 20, 30, 40]);
}

#[test]
fn rejects_raw_fixed_columns_with_wrong_size() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let mut bytes = sample_raw_file();
    bytes.pop();

    assert!(matches!(
        parse_raw_fixed_columns(&bytes, &setup, "group-a", "unit-a"),
        Err(FixedColumnError::InvalidRawByteLength {
            expected: 64,
            found: 63
        })
    ));
}

#[test]
fn reads_raw_fixed_columns_from_a_file_path_with_setup() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let path = temp_file_path("raw-fixed-columns.bin");
    fs::write(&path, sample_raw_file()).expect("fixture should be written");

    let parsed = read_fixed_columns_file_for_setup(&path, &setup, "group-a", "unit-a")
        .expect("fixture should parse from path");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed.row_count, 4);
    assert_eq!(parsed.columns[0].values, [1, 2, 3, 4]);
}

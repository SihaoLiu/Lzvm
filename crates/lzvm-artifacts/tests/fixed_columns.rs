use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{
    encode_fixed_columns, encode_raw_fixed_columns, expected_raw_fixed_column_byte_count,
    parse_fixed_columns, parse_raw_fixed_columns, raw_fixed_column_layout, read_fixed_columns_file,
    read_fixed_columns_file_for_setup, read_raw_fixed_column_file,
    read_raw_fixed_column_layout_file, read_raw_fixed_row_file, write_raw_fixed_columns_file,
    FixedColumn, FixedColumnError, FixedColumns,
};
use lzvm_field::FieldError;
mod fixtures;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const SAMPLE_FILE_MAIN_VALUE_ROW_1_OFFSET: usize = 24 + 88 + 8;
const SAMPLE_RAW_MAIN_RIGHT_ROW_2_OFFSET: usize = (2 * 2 + 1) * 8;

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

fn fixed_columns_file(section: Vec<u8>) -> Vec<u8> {
    let mut file = Vec::new();
    file.extend_from_slice(b"cnst");
    push_u32(&mut file, 1);
    push_u32(&mut file, 1);
    push_u32(&mut file, 1);
    push_u64(&mut file, section.len() as u64);
    file.extend_from_slice(&section);
    file
}

fn fixed_columns_prefix(row_count: u64, column_count: u32) -> Vec<u8> {
    let mut section = Vec::new();
    push_string(&mut section, "");
    push_string(&mut section, "");
    push_u64(&mut section, row_count);
    push_u32(&mut section, column_count);
    section
}

fn sample_raw_file() -> Vec<u8> {
    let mut bytes = Vec::new();
    for (left, right) in [(1_u64, 10_u64), (2, 20), (3, 30), (4, 40)] {
        push_u64(&mut bytes, left);
        push_u64(&mut bytes, right);
    }
    bytes
}

fn duplicate_name_raw_file() -> Vec<u8> {
    let mut bytes = Vec::new();
    for (left, right) in [(7_u64, 70_u64), (8, 80)] {
        push_u64(&mut bytes, left);
        push_u64(&mut bytes, right);
    }
    bytes
}

fn sample_raw_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 4,
        columns: vec![
            FixedColumn {
                name: "main.right".to_owned(),
                dimensions: vec![2],
                values: vec![10, 20, 30, 40],
            },
            FixedColumn {
                name: "main.left".to_owned(),
                dimensions: vec![1],
                values: vec![1, 2, 3, 4],
            },
        ],
    }
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
fn rejects_unsupported_fixed_column_file_versions() {
    let mut bytes = sample_file();
    bytes[4..8].copy_from_slice(&0_u32.to_le_bytes());

    assert!(matches!(
        parse_fixed_columns(&bytes),
        Err(FixedColumnError::UnsupportedVersion { found: 0, max: 1 })
    ));
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
fn rejects_non_canonical_fixed_column_values() {
    let mut parsed = parse_fixed_columns(&sample_file()).expect("fixture should parse");
    parsed.columns[1].values[1] = NON_CANONICAL_FIELD;

    assert!(matches!(
        encode_fixed_columns(&parsed),
        Err(FixedColumnError::ValueNonCanonical {
            column,
            row: 1,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        }) if column == "main.value"
    ));
}

#[test]
fn rejects_non_canonical_fixed_column_values_when_parsing() {
    let mut bytes = sample_file();
    bytes[SAMPLE_FILE_MAIN_VALUE_ROW_1_OFFSET..SAMPLE_FILE_MAIN_VALUE_ROW_1_OFFSET + 8]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    assert!(matches!(
        parse_fixed_columns(&bytes),
        Err(FixedColumnError::ValueNonCanonical {
            column,
            row: 1,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        }) if column == "main.value"
    ));
}

#[test]
fn parses_raw_fixed_columns_using_setup_column_map() {
    let setup = fixtures::sample_fixed_columns_setup_info();
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
fn encodes_raw_fixed_columns_using_setup_column_map() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let encoded =
        encode_raw_fixed_columns(&sample_raw_columns(), &setup).expect("fixture should encode");

    assert_eq!(encoded, sample_raw_file());
    let parsed = parse_raw_fixed_columns(&encoded, &setup, "group-a", "unit-a")
        .expect("encoded fixture should parse");
    assert_eq!(parsed.columns[0].values, [1, 2, 3, 4]);
    assert_eq!(parsed.columns[1].values, [10, 20, 30, 40]);
}

#[test]
fn rejects_non_canonical_raw_fixed_column_values() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let mut columns = sample_raw_columns();
    columns.columns[0].values[2] = NON_CANONICAL_FIELD;

    assert!(matches!(
        encode_raw_fixed_columns(&columns, &setup),
        Err(FixedColumnError::ValueNonCanonical {
            column,
            row: 2,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        }) if column == "main.right"
    ));
}

#[test]
fn rejects_non_canonical_raw_fixed_column_values_when_parsing() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let mut bytes = sample_raw_file();
    bytes[SAMPLE_RAW_MAIN_RIGHT_ROW_2_OFFSET..SAMPLE_RAW_MAIN_RIGHT_ROW_2_OFFSET + 8]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    assert!(matches!(
        parse_raw_fixed_columns(&bytes, &setup, "group-a", "unit-a"),
        Err(FixedColumnError::ValueNonCanonical {
            column,
            row: 2,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        }) if column == "main.right"
    ));
}

#[test]
fn disambiguates_duplicate_raw_fixed_column_names_by_physical_index() {
    let setup = fixtures::sample_duplicate_fixed_columns_setup_info();

    let parsed = parse_raw_fixed_columns(&duplicate_name_raw_file(), &setup, "group-a", "unit-a")
        .expect("raw columns should parse");
    let encoded = encode_raw_fixed_columns(&parsed, &setup).expect("raw columns should encode");

    assert_eq!(parsed.columns.len(), 2);
    assert_eq!(parsed.columns[0].name, "main.value[0]");
    assert_eq!(parsed.columns[0].values, [7, 8]);
    assert_eq!(parsed.columns[1].name, "main.value[1]");
    assert_eq!(parsed.columns[1].values, [70, 80]);
    assert_eq!(encoded, duplicate_name_raw_file());
}

#[test]
fn writes_raw_fixed_columns_to_a_file_path() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let path = temp_file_path("raw-fixed-columns-write.bin");
    write_raw_fixed_columns_file(&path, &sample_raw_columns(), &setup)
        .expect("fixture should write");

    let column = read_raw_fixed_column_file(&path, &setup, "group-a", "unit-a", 0)
        .expect("column should read");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(column, [1, 2, 3, 4]);
}

#[test]
fn rejects_raw_fixed_encoding_when_a_setup_column_is_missing() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let mut columns = sample_raw_columns();
    columns.columns.retain(|column| column.name != "main.right");

    assert!(matches!(
        encode_raw_fixed_columns(&columns, &setup),
        Err(FixedColumnError::MissingRawColumn { column })
            if column == "main.right"
    ));
}

#[test]
fn rejects_raw_fixed_encoding_when_dimensions_do_not_match_setup() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let mut columns = sample_raw_columns();
    columns.columns[0].dimensions = vec![3];

    assert!(matches!(
        encode_raw_fixed_columns(&columns, &setup),
        Err(FixedColumnError::RawColumnDimensionMismatch {
            column,
            expected,
            found
        }) if column == "main.right" && expected == vec![2] && found == vec![3]
    ));
}

#[test]
fn derives_raw_fixed_column_layout_from_setup() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let layout =
        raw_fixed_column_layout(&setup, "group-a", "unit-a").expect("layout should derive");

    assert_eq!(layout.group_name, "group-a");
    assert_eq!(layout.unit_name, "unit-a");
    assert_eq!(layout.row_count, 4);
    assert_eq!(layout.column_count, 2);
    assert_eq!(layout.byte_count, 64);
    assert_eq!(layout.columns.len(), 2);
    assert_eq!(layout.columns[0].name, "main.left");
    assert_eq!(layout.columns[0].index, 0);
    assert_eq!(layout.columns[0].dimensions, [1]);
    assert_eq!(layout.columns[1].name, "main.right");
    assert_eq!(layout.columns[1].index, 1);
    assert_eq!(layout.columns[1].dimensions, [2]);
}

#[test]
fn reads_raw_fixed_rows_without_full_parse() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let path = temp_file_path("raw-fixed-row.bin");
    fs::write(&path, sample_raw_file()).expect("fixture should be written");

    let layout = read_raw_fixed_column_layout_file(&path, &setup, "group-a", "unit-a")
        .expect("layout should validate against file");
    let row =
        read_raw_fixed_row_file(&path, &setup, "group-a", "unit-a", 2).expect("row should read");
    let out_of_bounds = read_raw_fixed_row_file(&path, &setup, "group-a", "unit-a", 4);
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(layout.row_count, 4);
    assert_eq!(layout.column_count, 2);
    assert_eq!(row, [3, 30]);
    assert!(matches!(
        out_of_bounds,
        Err(FixedColumnError::RawRowIndexOutOfBounds {
            row: 4,
            row_count: 4
        })
    ));
}

#[test]
fn reads_raw_fixed_columns_without_full_parse() {
    let setup = fixtures::sample_fixed_columns_setup_info();
    let path = temp_file_path("raw-fixed-column.bin");
    fs::write(&path, sample_raw_file()).expect("fixture should be written");

    let column = read_raw_fixed_column_file(&path, &setup, "group-a", "unit-a", 1)
        .expect("column should read");
    let out_of_bounds = read_raw_fixed_column_file(&path, &setup, "group-a", "unit-a", 2);
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(column, [10, 20, 30, 40]);
    assert!(matches!(
        out_of_bounds,
        Err(FixedColumnError::RawColumnIndexOutOfBounds {
            index: 2,
            width: 2,
            ..
        })
    ));
}

#[test]
fn rejects_raw_fixed_columns_with_wrong_size() {
    let setup = fixtures::sample_fixed_columns_setup_info();
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
    let setup = fixtures::sample_fixed_columns_setup_info();
    let path = temp_file_path("raw-fixed-columns.bin");
    fs::write(&path, sample_raw_file()).expect("fixture should be written");

    let parsed = read_fixed_columns_file_for_setup(&path, &setup, "group-a", "unit-a")
        .expect("fixture should parse from path");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed.row_count, 4);
    assert_eq!(parsed.columns[0].values, [1, 2, 3, 4]);
}

#[test]
fn rejects_column_count_that_exceeds_remaining_column_records() {
    let bytes = fixed_columns_file(fixed_columns_prefix(0, 1));

    assert!(matches!(
        parse_fixed_columns(&bytes),
        Err(FixedColumnError::LengthOverflow)
    ));
}

#[test]
fn rejects_dimension_count_that_exceeds_remaining_dimensions() {
    let mut section = fixed_columns_prefix(0, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    let bytes = fixed_columns_file(section);

    assert!(matches!(
        parse_fixed_columns(&bytes),
        Err(FixedColumnError::LengthOverflow)
    ));
}

#[test]
fn rejects_column_count_that_exceeds_remaining_column_values() {
    let mut section = fixed_columns_prefix(1, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 0);
    let bytes = fixed_columns_file(section);

    assert!(matches!(
        parse_fixed_columns(&bytes),
        Err(FixedColumnError::LengthOverflow)
    ));
}

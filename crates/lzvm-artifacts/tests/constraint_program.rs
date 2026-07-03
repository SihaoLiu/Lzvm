use lzvm_artifacts::constraint_program::{
    encode_global_constraint_program, encode_regular_constraint_program,
    parse_global_constraint_program, parse_regular_constraint_program,
    read_global_constraint_program_file, read_regular_constraint_program_file, ConstraintEntry,
    ConstraintProgram, ConstraintProgramError, GlobalConstraintEntry, GlobalConstraintProgram,
};
use lzvm_artifacts::sectioned::{
    encode_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};
use lzvm_field::FieldError;
use std::fs;
use std::path::PathBuf;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const SECTION_HEADER_BYTES: usize = 4 * 4;
const REGULAR_ENTRY_MIN_BYTES: usize = 12 * 4 + 1;
const GLOBAL_ENTRY_MIN_BYTES: usize = 8 * 4 + 1;
const ARG_BYTES: usize = 2;
const NUMBER_BYTES: usize = 8;

fn sample_regular_program() -> ConstraintProgram {
    ConstraintProgram {
        entries: vec![
            ConstraintEntry {
                stage: 1,
                destination_dimension: 3,
                destination_id: 5,
                first_row: 7,
                last_row: 11,
                temp1_count: 13,
                temp3_count: 17,
                ops_count: 2,
                ops_offset: 0,
                args_count: 1,
                args_offset: 0,
                intermediate: true,
                source_line: "regular-a".to_owned(),
            },
            ConstraintEntry {
                stage: 2,
                destination_dimension: 1,
                destination_id: 6,
                first_row: 8,
                last_row: 12,
                temp1_count: 14,
                temp3_count: 18,
                ops_count: 1,
                ops_offset: 2,
                args_count: 2,
                args_offset: 1,
                intermediate: false,
                source_line: "regular-b".to_owned(),
            },
        ],
        ops: vec![1, 2, 3],
        args: vec![10, 11, 12],
        numbers: vec![20, 21],
    }
}

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-constraint-program-{}-{name}",
            std::process::id()
        ));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

fn sample_global_program() -> GlobalConstraintProgram {
    GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 3,
            destination_id: 5,
            temp1_count: 13,
            temp3_count: 17,
            ops_count: 2,
            ops_offset: 0,
            args_count: 2,
            args_offset: 0,
            source_line: "global-a".to_owned(),
        }],
        ops: vec![4, 5],
        args: vec![30, 31],
        numbers: vec![40],
    }
}

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

fn wrap_regular_section(data: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection { id: 2, data }],
    })
    .expect("fixture should encode")
}

fn wrap_global_section(data: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection { id: 1, data }],
    })
    .expect("fixture should encode")
}

fn counted_section(ops_len: u32, args_len: u32, numbers_len: u32, entry_count: u32) -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, ops_len);
    push_u32(&mut section, args_len);
    push_u32(&mut section, numbers_len);
    push_u32(&mut section, entry_count);
    section
}

fn invalid_regular_span_file() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 3);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);

    push_u32(&mut section, 1);
    push_u32(&mut section, 3);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 3);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_string(&mut section, "bad span");

    section.extend_from_slice(&[1, 2, 3]);

    wrap_regular_section(section)
}

fn regular_zero_destination_dimension_file() -> Vec<u8> {
    let mut section = counted_section(0, 0, 0, 1);

    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_string(&mut section, "zero dimension");

    wrap_regular_section(section)
}

fn global_zero_destination_dimension_file() -> Vec<u8> {
    let mut section = counted_section(0, 0, 0, 1);

    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_string(&mut section, "zero dimension");

    wrap_global_section(section)
}

fn truncated_regular_payload_file() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);

    wrap_regular_section(section)
}

#[test]
fn parses_regular_constraint_sections() {
    let encoded = encode_regular_constraint_program(&sample_regular_program())
        .expect("fixture should encode");
    let parsed = parse_regular_constraint_program(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_regular_program());
}

#[test]
fn rejects_unsupported_constraint_program_versions() {
    let mut encoded = encode_regular_constraint_program(&sample_regular_program())
        .expect("fixture should encode");
    encoded[4..8].copy_from_slice(&0_u32.to_le_bytes());

    assert!(matches!(
        parse_regular_constraint_program(&encoded),
        Err(ConstraintProgramError::Sectioned(
            SectionedError::UnsupportedVersion { found: 0, max: 1 }
        ))
    ));
}

#[test]
fn parses_global_constraint_sections() {
    let encoded =
        encode_global_constraint_program(&sample_global_program()).expect("fixture should encode");
    let parsed = parse_global_constraint_program(&encoded).expect("fixture should parse");

    assert_eq!(parsed, sample_global_program());
}

#[test]
fn rejects_missing_regular_constraint_sections() {
    let file = SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: vec![0, 1, 2],
        }],
    };
    let encoded = encode_sectioned_file(&file).expect("fixture should encode");

    assert!(matches!(
        parse_regular_constraint_program(&encoded),
        Err(ConstraintProgramError::MissingConstraintSection { section_id: 2 })
    ));
}

#[test]
fn rejects_invalid_regular_operation_spans() {
    assert!(matches!(
        parse_regular_constraint_program(&invalid_regular_span_file()),
        Err(ConstraintProgramError::OperationSpanOutOfBounds { .. })
    ));
}

#[test]
fn rejects_zero_regular_destination_dimensions_when_encoding() {
    let mut program = sample_regular_program();
    program.entries[1].destination_dimension = 0;

    assert!(matches!(
        encode_regular_constraint_program(&program),
        Err(ConstraintProgramError::ZeroDestinationDimension {
            constraint_index: 1
        })
    ));
}

#[test]
fn rejects_zero_regular_destination_dimensions_when_parsing() {
    assert!(matches!(
        parse_regular_constraint_program(&regular_zero_destination_dimension_file()),
        Err(ConstraintProgramError::ZeroDestinationDimension {
            constraint_index: 0
        })
    ));
}

#[test]
fn rejects_zero_global_destination_dimensions_when_encoding() {
    let mut program = sample_global_program();
    program.entries[0].destination_dimension = 0;

    assert!(matches!(
        encode_global_constraint_program(&program),
        Err(ConstraintProgramError::ZeroDestinationDimension {
            constraint_index: 0
        })
    ));
}

#[test]
fn rejects_zero_global_destination_dimensions_when_parsing() {
    assert!(matches!(
        parse_global_constraint_program(&global_zero_destination_dimension_file()),
        Err(ConstraintProgramError::ZeroDestinationDimension {
            constraint_index: 0
        })
    ));
}

#[test]
fn rejects_truncated_constraint_sections() {
    assert!(matches!(
        parse_regular_constraint_program(&truncated_regular_payload_file()),
        Err(ConstraintProgramError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_regular_entry_count_that_exceeds_remaining_entry_records() {
    let bytes = wrap_regular_section(counted_section(0, 0, 0, 1));

    assert!(matches!(
        parse_regular_constraint_program(&bytes),
        Err(ConstraintProgramError::UnexpectedEof {
            offset: SECTION_HEADER_BYTES,
            needed: REGULAR_ENTRY_MIN_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_global_entry_count_that_exceeds_remaining_entry_records() {
    let bytes = wrap_global_section(counted_section(0, 0, 0, 1));

    assert!(matches!(
        parse_global_constraint_program(&bytes),
        Err(ConstraintProgramError::UnexpectedEof {
            offset: SECTION_HEADER_BYTES,
            needed: GLOBAL_ENTRY_MIN_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_args_count_that_exceeds_remaining_args() {
    let bytes = wrap_regular_section(counted_section(0, 1, 0, 0));

    assert!(matches!(
        parse_regular_constraint_program(&bytes),
        Err(ConstraintProgramError::UnexpectedEof {
            offset: SECTION_HEADER_BYTES,
            needed: ARG_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_numbers_count_that_exceeds_remaining_numbers() {
    let bytes = wrap_regular_section(counted_section(0, 0, 1, 0));

    assert!(matches!(
        parse_regular_constraint_program(&bytes),
        Err(ConstraintProgramError::UnexpectedEof {
            offset: SECTION_HEADER_BYTES,
            needed: NUMBER_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_non_canonical_regular_constraint_numbers() {
    let mut program = sample_regular_program();
    program.numbers[1] = NON_CANONICAL_FIELD;

    assert!(matches!(
        encode_regular_constraint_program(&program),
        Err(ConstraintProgramError::NumberNonCanonical {
            index: 1,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_non_canonical_regular_constraint_numbers_when_parsing() {
    let mut section = counted_section(0, 0, 1, 0);
    push_u64(&mut section, NON_CANONICAL_FIELD);
    let bytes = wrap_regular_section(section);

    assert!(matches!(
        parse_regular_constraint_program(&bytes),
        Err(ConstraintProgramError::NumberNonCanonical {
            index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_non_canonical_global_constraint_numbers() {
    let mut program = sample_global_program();
    program.numbers[0] = NON_CANONICAL_FIELD;

    assert!(matches!(
        encode_global_constraint_program(&program),
        Err(ConstraintProgramError::NumberNonCanonical {
            index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn rejects_non_canonical_global_constraint_numbers_when_parsing() {
    let mut section = counted_section(0, 0, 1, 0);
    push_u64(&mut section, NON_CANONICAL_FIELD);
    let bytes = wrap_global_section(section);

    assert!(matches!(
        parse_global_constraint_program(&bytes),
        Err(ConstraintProgramError::NumberNonCanonical {
            index: 0,
            source: FieldError::NonCanonical {
                value: NON_CANONICAL_FIELD
            },
        })
    ));
}

#[test]
fn reads_regular_constraint_programs_from_a_file_path() {
    let path = temp_file_path("regular.bin");
    fs::write(
        &path,
        encode_regular_constraint_program(&sample_regular_program())
            .expect("fixture should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_regular_constraint_program_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_regular_program());
}

#[test]
fn reads_global_constraint_programs_from_a_file_path() {
    let path = temp_file_path("global.bin");
    fs::write(
        &path,
        encode_global_constraint_program(&sample_global_program()).expect("fixture should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_global_constraint_program_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_global_program());
}

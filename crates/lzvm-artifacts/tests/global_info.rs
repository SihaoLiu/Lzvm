use lzvm_artifacts::global_info::{
    encode_global_info, parse_global_info, read_global_info_binary_file, read_global_info_file,
    GlobalInfoError,
};
use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use std::fs;
use std::path::PathBuf;

mod fixtures;

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-global-info-{}-{name}", std::process::id()))
}

fn global_info_file(section: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"ginf",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: section,
        }],
    })
    .expect("sectioned fixture should encode")
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

fn section_prefix() -> Vec<u8> {
    let mut section = Vec::new();
    push_string(&mut section, "");
    section.push(0);
    section.push(0);
    push_u64(&mut section, 1);
    push_u64(&mut section, 0);
    section
}

fn push_valid_air_layout(out: &mut Vec<u8>) {
    push_u32(out, 1);
    push_string(out, "");
    push_u32(out, 1);
    push_u32(out, 1);
    push_string(out, "");
    push_u64(out, 1);
    out.push(0);
}

fn push_empty_aggregation(out: &mut Vec<u8>) {
    push_u32(out, 1);
    push_u32(out, 0);
}

fn section_after_aggregation() -> Vec<u8> {
    let mut section = section_prefix();
    push_valid_air_layout(&mut section);
    push_empty_aggregation(&mut section);
    section
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
fn rejects_unsupported_global_info_file_versions() {
    let info = fixtures::sample_global_info_fixture();
    let bytes = encode_global_info(&info).expect("fixture should encode");
    let parsed = lzvm_artifacts::sectioned::parse_sectioned_file(&bytes, *b"ginf", 1)
        .expect("sectioned global info should parse");
    let bytes = encode_sectioned_file(&SectionedFile {
        kind: *b"ginf",
        version: 0,
        sections: parsed.sections,
    })
    .expect("sectioned fixture should encode");

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::UnsupportedVersion { found: 0, max: 1 })
    ));
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

#[test]
fn rejects_air_group_count_that_exceeds_remaining_group_names() {
    let mut section = section_prefix();
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_air_section_count_that_exceeds_remaining_sections() {
    let mut section = section_prefix();
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_unit_records() {
    let mut section = section_prefix();
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_aggregation_group_count_that_exceeds_remaining_groups() {
    let mut section = section_prefix();
    push_valid_air_layout(&mut section);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_aggregation_entry_count_that_exceeds_remaining_entries() {
    let mut section = section_prefix();
    push_valid_air_layout(&mut section);
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_challenge_count_that_exceeds_remaining_values() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_proof_value_count_that_exceeds_remaining_values() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_named_stage_value_count_that_exceeds_remaining_records() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_named_stage_value_length_count_that_exceeds_remaining_lengths() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u64(&mut section, 1);
    section.push(0);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_public_value_count_that_exceeds_remaining_records() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_public_value_length_count_that_exceeds_remaining_lengths() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u64(&mut section, 1);
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::LengthOverflow)
    ));
}

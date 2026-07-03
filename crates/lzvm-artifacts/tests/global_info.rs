use lzvm_artifacts::global_info::{
    encode_global_info, parse_global_info, read_global_info_binary_file, read_global_info_file,
    GlobalInfoError,
};
use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use std::fs;
use std::path::PathBuf;

mod fixtures;

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-global-info-{}-{name}", std::process::id()));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
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

fn section_prefix_with_public_count(n_publics: u64) -> Vec<u8> {
    let mut section = Vec::new();
    push_string(&mut section, "");
    section.push(0);
    section.push(0);
    push_u64(&mut section, 1);
    push_u64(&mut section, n_publics);
    section
}

fn section_prefix() -> Vec<u8> {
    section_prefix_with_public_count(0)
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

fn section_after_aggregation_with_public_count(n_publics: u64) -> Vec<u8> {
    let mut section = section_prefix_with_public_count(n_publics);
    push_valid_air_layout(&mut section);
    push_empty_aggregation(&mut section);
    section
}

fn push_u64_vec(out: &mut Vec<u8>, values: &[u64]) {
    push_u32(
        out,
        values.len().try_into().expect("fixture count should fit"),
    );
    for value in values {
        push_u64(out, *value);
    }
}

fn push_named_stage_value(out: &mut Vec<u8>, name: &str, stage: u64, lengths: &[u64]) {
    push_string(out, name);
    push_u64(out, stage);
    out.push(0);
    push_u32(
        out,
        lengths.len().try_into().expect("fixture count should fit"),
    );
    for length in lengths {
        push_u64(out, *length);
    }
}

fn push_public_value(out: &mut Vec<u8>, name: &str, stage: u64, lengths: &[u64]) {
    push_string(out, name);
    push_u64(out, stage);
    push_u32(
        out,
        lengths.len().try_into().expect("fixture count should fit"),
    );
    for length in lengths {
        push_u64(out, *length);
    }
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
fn encodes_public_value_counts_with_array_lengths() {
    let mut info = fixtures::sample_global_info_fixture();
    info.n_publics = 7;

    let bytes = encode_global_info(&info).expect("array public counts should encode");
    let parsed = parse_global_info(&bytes).expect("array public counts should parse");

    assert_eq!(parsed.n_publics, 7);
}

#[test]
fn rejects_invalid_transcript_arities_when_encoding() {
    let mut info = fixtures::sample_global_info_fixture();
    info.transcript_arity = 3;

    assert_eq!(
        encode_global_info(&info),
        Err(GlobalInfoError::InvalidTranscriptArity)
    );
}

#[test]
fn rejects_out_of_u32_transcript_arities_when_encoding_and_parsing() {
    let invalid_arity = u64::from(u32::MAX) + 1;
    let mut info = fixtures::sample_global_info_fixture();
    info.transcript_arity = invalid_arity;

    assert_eq!(
        encode_global_info(&info),
        Err(GlobalInfoError::InvalidTranscriptArity)
    );

    let mut section = Vec::new();
    push_string(&mut section, "");
    section.push(0);
    section.push(0);
    push_u64(&mut section, invalid_arity);
    push_u64(&mut section, 0);
    push_valid_air_layout(&mut section);
    push_empty_aggregation(&mut section);
    push_u64_vec(&mut section, &[]);
    push_u64_vec(&mut section, &[]);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::InvalidTranscriptArity)
    );
}

#[test]
fn rejects_duplicate_proof_value_names_when_encoding() {
    let mut info = fixtures::sample_global_info_fixture();
    info.proof_values_map[1].name = info.proof_values_map[0].name.clone();

    assert_eq!(
        encode_global_info(&info),
        Err(GlobalInfoError::DuplicateValueName {
            field: "proofValuesMap",
            name: "proof-a".to_owned(),
        })
    );
}

#[test]
fn rejects_duplicate_public_value_names_when_encoding() {
    let mut info = fixtures::sample_global_info_fixture();
    info.publics_map[1].name = info.publics_map[0].name.clone();

    assert_eq!(
        encode_global_info(&info),
        Err(GlobalInfoError::DuplicateValueName {
            field: "publicsMap",
            name: "public-a".to_owned(),
        })
    );
}

#[test]
fn rejects_proof_and_public_value_name_collisions_when_encoding() {
    let mut info = fixtures::sample_global_info_fixture();
    info.publics_map[0].name = info.proof_values_map[0].name.clone();

    assert_eq!(
        encode_global_info(&info),
        Err(GlobalInfoError::DuplicateValueName {
            field: "globalValues",
            name: "proof-a".to_owned(),
        })
    );
}

#[test]
fn rejects_duplicate_air_group_names_when_encoding() {
    let mut info = fixtures::sample_global_info_fixture();
    info.air_groups[1] = info.air_groups[0].clone();

    assert_eq!(
        encode_global_info(&info),
        Err(GlobalInfoError::DuplicateValueName {
            field: "airGroups",
            name: "group-a".to_owned(),
        })
    );
}

#[test]
fn rejects_duplicate_air_unit_names_within_group_when_encoding() {
    let mut info = fixtures::sample_global_info_fixture();
    info.airs[0][1].name = info.airs[0][0].name.clone();

    assert_eq!(
        encode_global_info(&info),
        Err(GlobalInfoError::DuplicateValueName {
            field: "airs",
            name: "unit-a".to_owned(),
        })
    );
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
        Err(GlobalInfoError::UnexpectedEof { .. })
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
        Err(GlobalInfoError::UnexpectedEof { .. })
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
        Err(GlobalInfoError::UnexpectedEof { .. })
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
        Err(GlobalInfoError::UnexpectedEof { .. })
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
        Err(GlobalInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_duplicate_air_group_names_when_parsing() {
    let mut section = section_prefix();
    push_u32(&mut section, 2);
    push_string(&mut section, "duplicate-group");
    push_string(&mut section, "duplicate-group");
    push_u32(&mut section, 2);
    for name in ["unit-a", "unit-b"] {
        push_u32(&mut section, 1);
        push_string(&mut section, name);
        push_u64(&mut section, 1);
        section.push(0);
    }
    push_u32(&mut section, 2);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u64_vec(&mut section, &[]);
    push_u64_vec(&mut section, &[]);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::DuplicateValueName {
            field: "airGroups",
            name: "duplicate-group".to_owned(),
        })
    );
}

#[test]
fn rejects_duplicate_air_unit_names_within_group_when_parsing() {
    let mut section = section_prefix();
    push_u32(&mut section, 1);
    push_string(&mut section, "group");
    push_u32(&mut section, 1);
    push_u32(&mut section, 2);
    for _ in 0..2 {
        push_string(&mut section, "duplicate-unit");
        push_u64(&mut section, 1);
        section.push(0);
    }
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u64_vec(&mut section, &[]);
    push_u64_vec(&mut section, &[]);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::DuplicateValueName {
            field: "airs",
            name: "duplicate-unit".to_owned(),
        })
    );
}

#[test]
fn rejects_challenge_count_that_exceeds_remaining_values() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 1);
    let bytes = global_info_file(section);

    assert!(matches!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::UnexpectedEof { .. })
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
        Err(GlobalInfoError::UnexpectedEof { .. })
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
        Err(GlobalInfoError::UnexpectedEof { .. })
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
        Err(GlobalInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_zero_named_stage_value_lengths() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_string(&mut section, "proof-array");
    push_u64(&mut section, 1);
    section.push(0);
    push_u32(&mut section, 1);
    push_u64(&mut section, 0);
    push_u32(&mut section, 0);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::InvalidLength {
            field: "proofValuesMap",
            index: 0,
        })
    );
}

#[test]
fn rejects_duplicate_proof_value_names_when_parsing() {
    let mut section = section_after_aggregation();
    push_u64_vec(&mut section, &[]);
    push_u64_vec(&mut section, &[]);
    push_u32(&mut section, 2);
    push_named_stage_value(&mut section, "duplicate-proof", 1, &[]);
    push_named_stage_value(&mut section, "duplicate-proof", 1, &[]);
    push_u32(&mut section, 0);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::DuplicateValueName {
            field: "proofValuesMap",
            name: "duplicate-proof".to_owned(),
        })
    );
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
        Err(GlobalInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_zero_public_value_lengths() {
    let mut section = section_after_aggregation();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_string(&mut section, "public-array");
    push_u64(&mut section, 1);
    push_u32(&mut section, 1);
    push_u64(&mut section, 0);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::InvalidLength {
            field: "publicsMap",
            index: 0,
        })
    );
}

#[test]
fn rejects_duplicate_public_value_names_when_parsing() {
    let mut section = section_after_aggregation_with_public_count(2);
    push_u64_vec(&mut section, &[]);
    push_u64_vec(&mut section, &[]);
    push_u32(&mut section, 0);
    push_u32(&mut section, 2);
    push_public_value(&mut section, "duplicate-public", 1, &[]);
    push_public_value(&mut section, "duplicate-public", 1, &[]);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::DuplicateValueName {
            field: "publicsMap",
            name: "duplicate-public".to_owned(),
        })
    );
}

#[test]
fn rejects_proof_and_public_value_name_collisions_when_parsing() {
    let mut section = section_after_aggregation_with_public_count(1);
    push_u64_vec(&mut section, &[]);
    push_u64_vec(&mut section, &[]);
    push_u32(&mut section, 1);
    push_named_stage_value(&mut section, "shared-value", 1, &[]);
    push_u32(&mut section, 1);
    push_public_value(&mut section, "shared-value", 1, &[]);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::DuplicateValueName {
            field: "globalValues",
            name: "shared-value".to_owned(),
        })
    );
}

#[test]
fn rejects_invalid_transcript_arities_when_parsing() {
    let mut section = Vec::new();
    push_string(&mut section, "");
    section.push(0);
    section.push(0);
    push_u64(&mut section, 3);
    push_u64(&mut section, 0);
    push_valid_air_layout(&mut section);
    push_empty_aggregation(&mut section);
    push_u64_vec(&mut section, &[]);
    push_u64_vec(&mut section, &[]);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    let bytes = global_info_file(section);

    assert_eq!(
        parse_global_info(&bytes),
        Err(GlobalInfoError::InvalidTranscriptArity)
    );
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
        Err(GlobalInfoError::UnexpectedEof { .. })
    ));
}

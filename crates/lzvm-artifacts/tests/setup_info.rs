use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, parse_unit_setup_info, read_unit_setup_info_binary_file,
    read_unit_setup_info_file, EvaluationMapEntry, EvaluationMapKind, SetupInfoError,
};
use std::fs;
use std::path::PathBuf;

mod fixtures;

fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn setup_info_file(section: Vec<u8>, version: u32) -> Vec<u8> {
    let mut file = Vec::new();
    file.extend_from_slice(b"uinf");
    push_u32(&mut file, version);
    push_u32(&mut file, 1);
    push_u32(&mut file, 1);
    file.extend_from_slice(&(section.len() as u64).to_le_bytes());
    file.extend_from_slice(&section);
    file
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(value.as_bytes());
    out.push(0);
}

fn push_optional_u32(out: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            push_u8(out, 1);
            push_u32(out, value);
        }
        None => push_u8(out, 0),
    }
}

fn push_optional_i64(out: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            push_u8(out, 1);
            push_i64(out, value);
        }
        None => push_u8(out, 0),
    }
}

fn push_optional_string(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            push_u8(out, 1);
            push_string(out, value);
        }
        None => push_u8(out, 0),
    }
}

fn push_optional_bool(out: &mut Vec<u8>, value: Option<bool>) {
    match value {
        Some(value) => {
            push_u8(out, 1);
            push_u8(out, u8::from(value));
        }
        None => push_u8(out, 0),
    }
}

fn minimal_header_prefix() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 2);
    push_u32(&mut section, 0);
    push_optional_u32(&mut section, None);
    push_optional_u32(&mut section, None);
    push_u32(&mut section, 7);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    section
}

fn push_minimal_section_widths(section: &mut Vec<u8>) {
    push_u32(section, 3);
    push_string(section, "cm1");
    push_u32(section, 1);
    push_string(section, "cm2");
    push_u32(section, 1);
    push_string(section, "cm3");
    push_u32(section, 1);
}

fn required_prefix_through_boundaries() -> Vec<u8> {
    let mut section = minimal_header_prefix();
    push_u32(&mut section, 0);
    push_minimal_section_widths(&mut section);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    section
}

fn push_minimal_stark(section: &mut Vec<u8>) {
    push_u32(section, 10);
    push_u32(section, 13);
    push_u32(section, 1);
    push_u32(section, 1);
    push_u32(section, 13);
    push_u8(section, 0);
    push_u32(section, 0);
    push_u32(section, 0);
    push_u32(section, 2);
    push_optional_string(section, None);
    push_optional_u32(section, None);
    push_optional_bool(section, None);
}

fn minimal_required_section() -> Vec<u8> {
    let mut section = required_prefix_through_boundaries();
    push_minimal_stark(&mut section);
    section
}

fn sample_setup_info_binary() -> Vec<u8> {
    let mut section = Vec::new();
    push_u32(&mut section, 2);
    push_u32(&mut section, 5);
    push_optional_u32(&mut section, Some(3));
    push_optional_u32(&mut section, Some(8));
    push_u32(&mut section, 7);
    push_u32(&mut section, 2);
    push_u32(&mut section, 3);

    push_u32(&mut section, 3);
    push_i64(&mut section, 0);
    push_i64(&mut section, 1);
    push_i64(&mut section, -1);

    push_u32(&mut section, 4);
    push_string(&mut section, "cm1");
    push_u32(&mut section, 2);
    push_string(&mut section, "cm2");
    push_u32(&mut section, 3);
    push_string(&mut section, "cm3");
    push_u32(&mut section, 1);
    push_string(&mut section, "const");
    push_u32(&mut section, 5);

    push_u32(&mut section, 5);
    for (name, pols_map_id, lengths) in [
        ("main.a", 0_u32, &[][..]),
        ("main.b", 1, &[][..]),
        ("main.c", 2, &[][..]),
        ("main.d", 3, &[][..]),
        ("main.e", 4, &[5_u32][..]),
    ] {
        push_string(&mut section, name);
        push_u32(&mut section, 0);
        push_u32(&mut section, 1);
        push_u32(&mut section, pols_map_id);
        push_u32(&mut section, pols_map_id);
        push_u32(&mut section, lengths.len() as u32);
        for length in lengths {
            push_u32(&mut section, *length);
        }
    }

    push_u32(&mut section, 2);
    push_optional_string(&mut section, Some("first"));
    push_optional_i64(&mut section, Some(0));
    push_optional_i64(&mut section, Some(3));
    push_optional_string(&mut section, None);
    push_optional_i64(&mut section, Some(-1));
    push_optional_i64(&mut section, None);

    push_u32(&mut section, 10);
    push_u32(&mut section, 13);
    push_u32(&mut section, 4);
    push_u32(&mut section, 3);
    push_u32(&mut section, 13);
    push_u32(&mut section, 9);
    push_u32(&mut section, 5);
    push_u8(&mut section, 1);
    push_u32(&mut section, 2);
    push_u32(&mut section, 20);
    push_u32(&mut section, 4);
    push_optional_string(&mut section, Some("GL"));
    push_optional_u32(&mut section, Some(4));
    push_optional_bool(&mut section, Some(true));

    push_u32(&mut section, 2);
    for (name, stage, dimension, pols_map_id, stage_id, stage_position, intermediate, lengths) in [
        ("trace.a", 1_u32, 1_u32, 0_u32, 0_u32, 0_u32, false, &[][..]),
        ("aux.a", 2, 3, 1, 0, 0, false, &[][..]),
    ] {
        push_string(&mut section, name);
        push_u32(&mut section, stage);
        push_u32(&mut section, dimension);
        push_u32(&mut section, pols_map_id);
        push_u32(&mut section, stage_id);
        push_u32(&mut section, stage_position);
        push_u8(&mut section, u8::from(intermediate));
        push_u32(&mut section, lengths.len() as u32);
        for length in lengths {
            push_u32(&mut section, *length);
        }
    }

    push_u32(&mut section, 2);
    push_string(&mut section, "unit.alpha");
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    push_u32(&mut section, 2);
    push_string(&mut section, "unit.beta");
    push_u32(&mut section, 2);
    push_u32(&mut section, 0);

    push_u32(&mut section, 1);
    push_string(&mut section, "group.alpha");
    push_u32(&mut section, 2);
    push_u32(&mut section, 0);

    let mut file = Vec::new();
    file.extend_from_slice(b"uinf");
    push_u32(&mut file, 2);
    push_u32(&mut file, 1);
    push_u32(&mut file, 1);
    file.extend_from_slice(&(section.len() as u64).to_le_bytes());
    file.extend_from_slice(&section);
    file
}

fn sample_setup_info_binary_with_evaluation_map() -> Vec<u8> {
    let mut file = sample_setup_info_binary();
    file[4..8].copy_from_slice(&3_u32.to_le_bytes());
    let mut section = file[24..].to_vec();
    push_u32(&mut section, 3);
    for (kind, id, prime, opening_pos, commit_id) in [
        (0_u8, 2_u32, 0_i64, 0_u32, None),
        (1_u8, 1_u32, 1_i64, 1_u32, None),
        (2_u8, 7_u32, -1_i64, 2_u32, Some(3_u32)),
    ] {
        push_u8(&mut section, kind);
        push_u32(&mut section, id);
        push_i64(&mut section, prime);
        push_u32(&mut section, opening_pos);
        push_optional_u32(&mut section, commit_id);
    }
    let section_len = section.len() as u64;
    file.truncate(24);
    file[16..24].copy_from_slice(&section_len.to_le_bytes());
    file.extend_from_slice(&section);
    file
}

fn sample_setup_info_binary_without_commitment_columns() -> Vec<u8> {
    const OPTIONAL_TAIL_BYTES: usize = 137;
    let mut file = sample_setup_info_binary();
    let section_len_offset = 16;
    let mut section_len_bytes = [0_u8; 8];
    section_len_bytes.copy_from_slice(&file[section_len_offset..section_len_offset + 8]);
    let section_len = u64::from_le_bytes(section_len_bytes);
    let adjusted_len = section_len - OPTIONAL_TAIL_BYTES as u64;
    file[section_len_offset..section_len_offset + 8].copy_from_slice(&adjusted_len.to_le_bytes());
    file.truncate(file.len() - OPTIONAL_TAIL_BYTES);
    file
}

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-setup-info-{}-{name}", std::process::id()));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

#[test]
fn parses_evaluation_map_entries_from_binary() {
    let expected = vec![
        EvaluationMapEntry {
            kind: EvaluationMapKind::Constant,
            id: 2,
            prime: 0,
            opening_position: 0,
            commit_id: None,
        },
        EvaluationMapEntry {
            kind: EvaluationMapKind::Commitment,
            id: 1,
            prime: 1,
            opening_position: 1,
            commit_id: None,
        },
        EvaluationMapEntry {
            kind: EvaluationMapKind::Custom,
            id: 7,
            prime: -1,
            opening_position: 2,
            commit_id: Some(3),
        },
    ];
    let parsed = parse_unit_setup_info(&sample_setup_info_binary_with_evaluation_map())
        .expect("fixture should parse");
    assert_eq!(parsed.evaluation_map, expected);
}

#[test]
fn reads_unit_setup_info_from_a_file_path() {
    let path = temp_file_path("unit.generic.setup.bin");
    fs::write(&path, sample_setup_info_binary()).expect("fixture should be written");

    let info = read_unit_setup_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(
        info.stage_commit_widths().expect("widths should exist"),
        vec![2, 3, 1]
    );
}

#[test]
fn rejects_text_unit_setup_info_from_a_file_path() {
    let path = temp_file_path("unit.json");
    fs::write(&path, "not a binary file").expect("fixture should be written");

    let error = read_unit_setup_info_file(&path).expect_err("text metadata should be rejected");
    fs::remove_file(&path).expect("fixture should be removed");

    assert!(matches!(error, SetupInfoError::InvalidMagic));
}

#[test]
fn parses_unit_setup_info_binary() {
    let info = parse_unit_setup_info(&sample_setup_info_binary()).expect("fixture should parse");

    assert_eq!(info.n_stages, 2);
    assert_eq!(info.n_constants, 5);
    assert_eq!(info.constant_columns.len(), 5);
    assert_eq!(info.constant_columns[4].name, "main.e");
    assert_eq!(info.constant_columns[4].lengths, [5]);
    assert_eq!(info.commitment_columns.len(), 2);
    assert_eq!(info.commitment_columns[1].stage, 2);
    assert_eq!(info.commitment_columns[1].stage_position, 0);
    assert_eq!(info.commitment_columns[1].dimension, 3);
    assert_eq!(info.unit_value_map.len(), 2);
    assert_eq!(info.unit_value_map[0].name, "unit.alpha");
    assert_eq!(info.unit_value_map[0].lengths, [2]);
    assert_eq!(info.group_value_map.len(), 1);
    assert_eq!(info.group_value_map[0].name, "group.alpha");
    assert_eq!(info.n_publics, Some(3));
    assert_eq!(info.n_constraints, Some(8));
    assert_eq!(info.q_degree, 7);
    assert_eq!(info.opening_points, vec![0, 1, -1]);
    assert_eq!(info.challenge_count, 2);
    assert_eq!(info.eval_count, 3);
    assert_eq!(
        info.stage_commit_widths().expect("widths should exist"),
        vec![2, 3, 1]
    );
    assert_eq!(info.boundaries.len(), 2);
    assert_eq!(info.boundaries[0].name.as_deref(), Some("first"));
    assert_eq!(info.boundaries[1].offset_min, Some(-1));
    assert_eq!(info.stark.n_bits, 10);
    assert_eq!(info.stark.n_bits_ext, 13);
    assert_eq!(info.stark.steps.len(), 3);
    assert_eq!(info.stark.verification_hash_type.as_deref(), Some("GL"));
}

#[test]
fn rejects_zero_constant_column_lengths() {
    let mut info = fixtures::sample_setup_info_fixture();
    info.constant_columns[0].lengths = vec![0];

    assert_eq!(
        encode_unit_setup_info(&info),
        Err(SetupInfoError::InvalidConstantColumn { index: 0 })
    );
}

#[test]
fn rejects_zero_commitment_column_lengths() {
    let mut info = fixtures::sample_setup_info_fixture();
    info.commitment_columns[0].lengths = vec![0];

    assert_eq!(
        encode_unit_setup_info(&info),
        Err(SetupInfoError::InvalidCommitmentColumn { index: 0 })
    );
}

#[test]
fn rejects_overlapping_commitment_columns() {
    let mut info = fixtures::sample_setup_info_fixture();
    let mut overlapping = info.commitment_columns[0].clone();
    overlapping.name = "trace.overlap".to_owned();
    info.commitment_columns.push(overlapping);

    assert_eq!(
        encode_unit_setup_info(&info),
        Err(SetupInfoError::InvalidCommitmentColumn { index: 2 })
    );
}

#[test]
fn rejects_overlapping_commitment_columns_when_parsing() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 2);
    for (name, pols_map_id) in [("trace.a", 0_u32), ("trace.overlap", 1_u32)] {
        push_string(&mut section, name);
        push_u32(&mut section, 1);
        push_u32(&mut section, 1);
        push_u32(&mut section, pols_map_id);
        push_u32(&mut section, pols_map_id);
        push_u32(&mut section, 0);
        push_u8(&mut section, 0);
        push_u32(&mut section, 0);
    }
    let bytes = setup_info_file(section, 3);

    assert_eq!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::InvalidCommitmentColumn { index: 1 })
    );
}

#[test]
fn rejects_zero_commitment_stage_widths() {
    let mut info = fixtures::sample_setup_info_fixture();
    info.section_widths.insert("cm1".to_owned(), 0);

    assert_eq!(
        encode_unit_setup_info(&info),
        Err(SetupInfoError::InvalidSectionWidth {
            name: "cm1".to_owned()
        })
    );
}

#[test]
fn rejects_zero_commitment_stage_widths_when_parsing() {
    let mut section = minimal_header_prefix();
    push_u32(&mut section, 0);
    push_u32(&mut section, 3);
    push_string(&mut section, "cm1");
    push_u32(&mut section, 0);
    push_string(&mut section, "cm2");
    push_u32(&mut section, 1);
    push_string(&mut section, "cm3");
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_minimal_stark(&mut section);
    let bytes = setup_info_file(section, 3);

    assert_eq!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::InvalidSectionWidth {
            name: "cm1".to_owned()
        })
    );
}

#[test]
fn rejects_zero_version_unit_setup_info_binary() {
    let mut bytes = sample_setup_info_binary();
    bytes[4..8].copy_from_slice(&0_u32.to_le_bytes());

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnsupportedVersion { found: 0, max: 3 })
    ));
}

#[test]
fn parses_unit_setup_info_binary_without_commitment_columns() {
    let info = parse_unit_setup_info(&sample_setup_info_binary_without_commitment_columns())
        .expect("fixture should parse");

    assert!(info.commitment_columns.is_empty());
    assert_eq!(
        info.stage_commit_widths().expect("widths should exist"),
        vec![2, 3, 1]
    );
}

#[test]
fn encodes_unit_setup_info_to_the_canonical_binary_form() {
    let info = fixtures::sample_setup_info_fixture_with_evaluation_map();
    let encoded = encode_unit_setup_info(&info).expect("fixture should encode");

    assert_eq!(encoded, sample_setup_info_binary_with_evaluation_map());
}

#[test]
fn reads_unit_setup_info_binary_from_a_file_path() {
    let path = temp_file_path("unit.setup.bin");
    fs::write(&path, sample_setup_info_binary()).expect("fixture should be written");

    let info = read_unit_setup_info_binary_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(
        info.stage_commit_widths().expect("widths should exist"),
        vec![2, 3, 1]
    );
}

#[test]
fn rejects_invalid_binary_setup_info_magic() {
    let mut bytes = sample_setup_info_binary();
    bytes[0] = b'x';

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::InvalidMagic)
    ));
}

#[test]
fn rejects_stage_count_overflow() {
    let mut info = fixtures::sample_setup_info_fixture();
    info.n_stages = u32::MAX;

    assert!(matches!(
        encode_unit_setup_info(&info),
        Err(SetupInfoError::LengthOverflow)
    ));
}

#[test]
fn rejects_opening_point_count_that_exceeds_remaining_points() {
    let mut section = minimal_header_prefix();
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_constant_column_count_that_exceeds_remaining_records() {
    let mut section = minimal_header_prefix();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_constant_column_length_count_that_exceeds_remaining_lengths() {
    let mut section = minimal_header_prefix();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_boundary_count_that_exceeds_remaining_boundary_records() {
    let mut section = minimal_header_prefix();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_fri_step_count_that_exceeds_remaining_steps() {
    let mut section = required_prefix_through_boundaries();
    push_u32(&mut section, 10);
    push_u32(&mut section, 13);
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_commitment_column_count_that_exceeds_remaining_records() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_commitment_column_length_count_that_exceeds_remaining_lengths() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u8(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_unit_value_count_that_exceeds_remaining_records() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_stage_value_length_count_that_exceeds_remaining_lengths() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_string(&mut section, "");
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_zero_unit_value_lengths() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_string(&mut section, "unit.empty");
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    let bytes = setup_info_file(section, 3);

    assert_eq!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::InvalidStageValue {
            field: "unit-value-map",
            index: 0,
        })
    );
}

#[test]
fn rejects_group_value_count_that_exceeds_remaining_records() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_zero_group_value_lengths() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    push_string(&mut section, "group.empty");
    push_u32(&mut section, 1);
    push_u32(&mut section, 1);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    let bytes = setup_info_file(section, 3);

    assert_eq!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::InvalidStageValue {
            field: "group-value-map",
            index: 0,
        })
    );
}

#[test]
fn rejects_evaluation_map_count_that_exceeds_remaining_records() {
    let mut section = minimal_required_section();
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = setup_info_file(section, 3);

    assert!(matches!(
        parse_unit_setup_info(&bytes),
        Err(SetupInfoError::UnexpectedEof { .. })
    ));
}

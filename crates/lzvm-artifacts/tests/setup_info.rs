use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, parse_unit_setup_info, parse_unit_setup_info_json,
    read_unit_setup_info_binary_file, read_unit_setup_info_file, SetupInfoError,
};
use std::fs;
use std::path::PathBuf;

fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
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

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 2,
        "nConstants": 5,
        "nPublics": 3,
        "nConstraints": 8,
        "qDeg": 7,
        "openingPoints": [0, 1, -1],
        "mapSectionsN": {
            "const": 5,
            "cm1": 2,
            "cm2": 3,
            "cm3": 1
        },
        "constPolsMap": [
            {"stage": 0, "name": "main.a", "dim": 1, "polsMapId": 0, "stageId": 0},
            {"stage": 0, "name": "main.b", "dim": 1, "polsMapId": 1, "stageId": 1},
            {"stage": 0, "name": "main.c", "dim": 1, "polsMapId": 2, "stageId": 2},
            {"stage": 0, "name": "main.d", "dim": 1, "polsMapId": 3, "stageId": 3},
            {"stage": 0, "name": "main.e", "dim": 1, "polsMapId": 4, "stageId": 4, "lengths": [5]}
        ],
        "cmPolsMap": [
            {"stage": 1, "name": "trace.a", "dim": 1, "polsMapId": 0, "stageId": 0, "stagePos": 0},
            {"stage": 2, "name": "aux.a", "dim": 3, "polsMapId": 1, "stageId": 0, "stagePos": 0}
        ],
        "airValuesMap": [
            {"stage": 1, "name": "unit.alpha", "lengths": [2]},
            {"stage": 2, "name": "unit.beta"}
        ],
        "airgroupValuesMap": [
            {"stage": 2, "name": "group.alpha"}
        ],
        "challengesMap": [{}, {}],
        "evMap": [{}, {}, {}],
        "boundaries": [
            {"name": "first", "offsetMin": 0, "offsetMax": 3},
            {"offsetMin": -1}
        ],
        "starkStruct": {
            "nBits": 10,
            "nBitsExt": 13,
            "nQueries": 4,
            "steps": [
                {"nBits": 13},
                {"nBits": 9},
                {"nBits": 5}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 20,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
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
    std::env::temp_dir().join(format!("lzvm-setup-info-{}-{name}", std::process::id()))
}

#[test]
fn parses_unit_setup_info_json() {
    let info = parse_unit_setup_info_json(sample_setup_info_json()).expect("fixture should parse");

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
    assert_eq!(info.unit_value_map[0].stage, 1);
    assert_eq!(info.unit_value_map[0].lengths, [2]);
    assert_eq!(info.unit_value_map[1].name, "unit.beta");
    assert_eq!(info.unit_value_map[1].stage, 2);
    assert_eq!(info.group_value_map.len(), 1);
    assert_eq!(info.group_value_map[0].name, "group.alpha");
    assert_eq!(info.group_value_map[0].stage, 2);
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
fn rejects_missing_required_setup_fields() {
    assert!(matches!(
        parse_unit_setup_info_json("{}"),
        Err(SetupInfoError::MissingField { field: "nStages" })
    ));
}

#[test]
fn rejects_mismatched_domain_bits() {
    let json = sample_setup_info_json().replace("\"nBitsExt\": 13", "\"nBitsExt\": 9");

    assert!(matches!(
        parse_unit_setup_info_json(&json),
        Err(SetupInfoError::InvalidDomainBits {
            n_bits: 10,
            n_bits_ext: 9
        })
    ));
}

#[test]
fn rejects_fri_steps_that_do_not_start_at_extended_domain() {
    let json = sample_setup_info_json().replace("\"nBits\": 13", "\"nBits\": 12");

    assert!(matches!(
        parse_unit_setup_info_json(&json),
        Err(SetupInfoError::InvalidFriSteps)
    ));
}

#[test]
fn rejects_missing_stage_widths() {
    let json = sample_setup_info_json().replace("\"cm3\": 1", "\"other\": 1");

    assert!(matches!(
        parse_unit_setup_info_json(&json),
        Err(SetupInfoError::MissingSectionWidth { name }) if name == "cm3"
    ));
}

#[test]
fn reads_unit_setup_info_from_a_file_path() {
    let path = temp_file_path("unit.json");
    fs::write(&path, sample_setup_info_json()).expect("fixture should be written");

    let info = read_unit_setup_info_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(
        info.stage_commit_widths().expect("widths should exist"),
        vec![2, 3, 1]
    );
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
    let info = parse_unit_setup_info_json(sample_setup_info_json()).expect("fixture should parse");
    let encoded = encode_unit_setup_info(&info).expect("fixture should encode");

    assert_eq!(encoded, sample_setup_info_binary());
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

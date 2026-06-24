use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::pcs_plan::{
    derive_pcs_setup_plan, encode_pcs_setup_plan, parse_pcs_setup_plan, read_pcs_setup_plan_file,
    PcsPlanError,
};
use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};

mod fixtures;

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-pcs-plan-{}-{name}", std::process::id()));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

fn pcs_plan_file(section: Vec<u8>) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind: *b"pcsp",
        version: 2,
        sections: vec![SectionedSection {
            id: 1,
            data: section,
        }],
    })
    .expect("sectioned fixture should encode")
}

fn section_prefix() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 10);
    push_u32(&mut bytes, 13);
    push_u64(&mut bytes, 1024);
    push_u64(&mut bytes, 8192);
    push_u64(&mut bytes, 8);
    push_u32(&mut bytes, 4);
    push_u32(&mut bytes, 20);
    push_u32(&mut bytes, 4);
    bytes.push(0);
    bytes.push(0);
    push_u32(&mut bytes, 5);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn derives_pcs_setup_plan_from_unit_setup_metadata() {
    let setup = fixtures::sample_pcs_plan_setup_info();

    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");

    assert_eq!(plan.base_domain_bits, 10);
    assert_eq!(plan.extended_domain_bits, 13);
    assert_eq!(plan.base_domain_size, 1024);
    assert_eq!(plan.extended_domain_size, 8192);
    assert_eq!(plan.blowup_factor, 8);
    assert_eq!(plan.query_count, 4);
    assert_eq!(plan.proof_of_work_bits, 20);
    assert_eq!(plan.merkle_tree_arity, 4);
    assert_eq!(plan.transcript_arity, Some(4));
    assert!(plan.hash_commits);
    assert_eq!(plan.constant_width, 5);
    assert_eq!(plan.stage_commit_widths, vec![2, 3, 1]);
    assert_eq!(plan.opening_points, vec![0, 1, -1]);
    assert_eq!(plan.final_layer_bits, 5);
    assert_eq!(plan.fri_layers.len(), 2);
    assert_eq!(plan.fri_layers[0].input_bits, 13);
    assert_eq!(plan.fri_layers[0].output_bits, 9);
    assert_eq!(plan.fri_layers[0].folding_factor, 16);
    assert_eq!(plan.fri_layers[1].input_bits, 9);
    assert_eq!(plan.fri_layers[1].output_bits, 5);
    assert_eq!(plan.fri_layers[1].folding_factor, 16);
}

#[test]
fn encodes_and_parses_pcs_setup_plans() {
    let setup = fixtures::sample_pcs_plan_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");

    let encoded = encode_pcs_setup_plan(&plan).expect("plan should encode");
    let parsed = parse_pcs_setup_plan(&encoded).expect("plan should parse");

    assert_eq!(&encoded[0..4], b"pcsp");
    assert_eq!(parsed, plan);
}

#[test]
fn encodes_the_current_pcs_setup_plan_format_version() {
    let setup = fixtures::sample_pcs_plan_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");

    let encoded = encode_pcs_setup_plan(&plan).expect("plan should encode");

    assert_eq!(u32::from_le_bytes(encoded[4..8].try_into().unwrap()), 2);
}

#[test]
fn rejects_stale_pcs_setup_plan_format_headers() {
    let setup = fixtures::sample_pcs_plan_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let mut encoded = encode_pcs_setup_plan(&plan).expect("plan should encode");
    encoded[4..8].copy_from_slice(&1_u32.to_le_bytes());

    assert!(parse_pcs_setup_plan(&encoded).is_err());
}

#[test]
fn pcs_setup_plan_encoding_depends_on_commit_hash_mode() {
    let setup = fixtures::sample_pcs_plan_setup_info();
    let mut direct_setup = setup.clone();
    direct_setup.stark.hash_commits = false;

    let compressed_plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let direct_plan = derive_pcs_setup_plan(&direct_setup).expect("plan should derive");

    assert_ne!(compressed_plan, direct_plan);
    assert_ne!(
        encode_pcs_setup_plan(&compressed_plan).expect("plan should encode"),
        encode_pcs_setup_plan(&direct_plan).expect("plan should encode")
    );
}

#[test]
fn reads_pcs_setup_plans_from_a_file_path() {
    let setup = fixtures::sample_pcs_plan_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let path = temp_file_path("plan.bin");
    fs::write(
        &path,
        encode_pcs_setup_plan(&plan).expect("plan should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_pcs_setup_plan_file(&path).expect("plan should read");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, plan);
}

#[test]
fn rejects_invalid_pcs_folding_schedule() {
    let mut setup = fixtures::sample_pcs_plan_setup_info();
    setup.stark.steps[1].n_bits = 13;

    assert!(matches!(
        derive_pcs_setup_plan(&setup),
        Err(PcsPlanError::InvalidFriLayer {
            input_bits: 13,
            output_bits: 13
        })
    ));
}

#[test]
fn rejects_stage_commit_width_count_that_exceeds_remaining_widths() {
    let mut section = section_prefix();
    push_u32(&mut section, 1);
    let bytes = pcs_plan_file(section);

    assert!(matches!(
        parse_pcs_setup_plan(&bytes),
        Err(PcsPlanError::LengthOverflow)
    ));
}

#[test]
fn rejects_opening_point_count_that_exceeds_remaining_points() {
    let mut section = section_prefix();
    push_u32(&mut section, 1);
    push_u32(&mut section, 2);
    push_u32(&mut section, 1);
    let bytes = pcs_plan_file(section);

    assert!(matches!(
        parse_pcs_setup_plan(&bytes),
        Err(PcsPlanError::LengthOverflow)
    ));
}

#[test]
fn rejects_fri_layer_count_that_exceeds_remaining_layers() {
    let mut section = section_prefix();
    push_u32(&mut section, 1);
    push_u32(&mut section, 2);
    push_u32(&mut section, 0);
    push_u32(&mut section, 1);
    let bytes = pcs_plan_file(section);

    assert!(matches!(
        parse_pcs_setup_plan(&bytes),
        Err(PcsPlanError::LengthOverflow)
    ));
}

#[test]
fn rejects_pcs_domains_that_do_not_fit_u64() {
    let mut setup = fixtures::sample_pcs_plan_setup_info();
    setup.stark.n_bits_ext = 64;
    setup.stark.steps[0].n_bits = 64;

    assert!(matches!(
        derive_pcs_setup_plan(&setup),
        Err(PcsPlanError::DomainTooLarge { bits: 64 })
    ));
}

#[test]
fn rejects_pcs_domains_that_shrink_before_extension() {
    let mut setup = fixtures::sample_pcs_plan_setup_info();
    setup.stark.n_bits_ext = 9;
    setup.stark.steps[0].n_bits = 9;

    assert!(matches!(
        derive_pcs_setup_plan(&setup),
        Err(PcsPlanError::InvalidDomainBits {
            base_bits: 10,
            extended_bits: 9
        })
    ));
}

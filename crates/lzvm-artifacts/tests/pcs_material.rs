use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::constant_tree::parse_constant_tree_bytes;
use lzvm_artifacts::pcs_material::{
    build_pcs_setup_material, encode_pcs_setup_material, parse_pcs_setup_material,
    read_pcs_setup_material_file, PcsSetupMaterialError,
};
use lzvm_artifacts::pcs_plan::{derive_pcs_setup_plan, encode_pcs_setup_plan};
use sha2::{Digest, Sha256};

mod fixtures;

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const FILE_PAYLOAD_OFFSET: usize = 24;
const MATERIAL_ROOT_OFFSET: usize = FILE_PAYLOAD_OFFSET + 32 * 3;

fn sample_tree_bytes() -> Vec<u8> {
    let mut bytes = vec![7_u8; 224];
    for (index, value) in [1_u64, 2, 3, 4].iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-pcs-material-{}-{name}", std::process::id()))
}

fn sample_material() -> lzvm_artifacts::pcs_material::PcsSetupMaterial {
    let setup = fixtures::sample_pcs_material_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let tree = parse_constant_tree_bytes(sample_tree_bytes(), &setup).expect("tree should parse");

    build_pcs_setup_material(&plan, &[1_u8; 32], &tree).expect("material should build")
}

#[test]
fn builds_pcs_setup_material_from_plan_and_static_artifacts() {
    let setup = fixtures::sample_pcs_material_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let fixed = [1_u8; 32];
    let tree = parse_constant_tree_bytes(sample_tree_bytes(), &setup).expect("tree should parse");

    let material = build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");
    let expected_plan_digest: [u8; 32] =
        Sha256::digest(encode_pcs_setup_plan(&plan).expect("plan should encode")).into();
    let expected_fixed_digest: [u8; 32] = Sha256::digest(fixed).into();
    let expected_tree_digest: [u8; 32] = Sha256::digest(&tree.bytes).into();

    assert_eq!(material.plan_digest, expected_plan_digest);
    assert_eq!(material.fixed_column_digest, expected_fixed_digest);
    assert_eq!(material.constant_tree_digest, expected_tree_digest);
    assert_eq!(material.constant_tree_root, [1, 2, 3, 4]);
    assert_eq!(material.fixed_byte_count, 32);
    assert_eq!(material.constant_tree_byte_count, 224);
    assert_eq!(material.leaf_byte_count, 64);
    assert_eq!(material.node_byte_count, 160);
}

#[test]
fn encodes_and_parses_pcs_setup_material() {
    let setup = fixtures::sample_pcs_material_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let fixed = [1_u8; 32];
    let tree = parse_constant_tree_bytes(sample_tree_bytes(), &setup).expect("tree should parse");
    let material = build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");

    let encoded = encode_pcs_setup_material(&material).expect("material should encode");
    let parsed = parse_pcs_setup_material(&encoded).expect("material should parse");

    assert_eq!(&encoded[0..4], b"pcsm");
    assert_eq!(parsed, material);
}

#[test]
fn rejects_non_canonical_pcs_setup_material_roots() {
    let mut material = sample_material();
    material.constant_tree_root[1] = NON_CANONICAL_FIELD;

    let err = encode_pcs_setup_material(&material).expect_err("material root should be canonical");

    assert_eq!(
        err.to_string(),
        "PCS setup material constant tree root word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_setup_material_roots_when_parsing() {
    let material = sample_material();
    let mut encoded = encode_pcs_setup_material(&material).expect("material should encode");
    encoded[MATERIAL_ROOT_OFFSET + 16..MATERIAL_ROOT_OFFSET + 24]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_pcs_setup_material(&encoded).expect_err("material root should be canonical");

    assert_eq!(
        err.to_string(),
        "PCS setup material constant tree root word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_unsupported_pcs_setup_material_versions() {
    let setup = fixtures::sample_pcs_material_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let tree = parse_constant_tree_bytes(sample_tree_bytes(), &setup).expect("tree should parse");
    let material =
        build_pcs_setup_material(&plan, &[1_u8; 32], &tree).expect("material should build");
    let encoded = encode_pcs_setup_material(&material).expect("material should encode");
    let parsed = lzvm_artifacts::sectioned::parse_sectioned_file(&encoded, *b"pcsm", 1)
        .expect("sectioned material should parse");
    let encoded = lzvm_artifacts::sectioned::encode_sectioned_file(
        &lzvm_artifacts::sectioned::SectionedFile {
            kind: *b"pcsm",
            version: 0,
            sections: parsed.sections,
        },
    )
    .expect("sectioned material should encode");

    assert_eq!(
        parse_pcs_setup_material(&encoded).expect_err("unsupported material version should reject"),
        PcsSetupMaterialError::UnsupportedVersion {
            found: 0,
            expected: 1,
        }
    );
}

#[test]
fn reads_pcs_setup_material_from_a_file_path() {
    let setup = fixtures::sample_pcs_material_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let tree = parse_constant_tree_bytes(sample_tree_bytes(), &setup).expect("tree should parse");
    let material =
        build_pcs_setup_material(&plan, &[1_u8; 32], &tree).expect("material should build");
    let path = temp_file_path("material.bin");
    fs::write(
        &path,
        encode_pcs_setup_material(&material).expect("material should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_pcs_setup_material_file(&path).expect("material should read");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, material);
}

use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::setup_manifest::{
    encode_setup_directory_manifest, parse_setup_directory_manifest,
    read_setup_directory_manifest_file, SetupDirectoryManifest, SetupDirectoryManifestError,
};

fn sample_manifest() -> SetupDirectoryManifest {
    SetupDirectoryManifest {
        unit_count: 4,
        global_constraint_count: 3,
        fixed_byte_count: 128,
        pcs_material_unit_count: 4,
        pcs_material_byte_count: 512,
        catalog_digest: [0x55; 32],
    }
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-setup-manifest-{}-{name}", std::process::id()))
}

#[test]
fn encodes_and_parses_setup_directory_manifest() {
    let encoded =
        encode_setup_directory_manifest(&sample_manifest()).expect("manifest should encode");
    let parsed = parse_setup_directory_manifest(&encoded).expect("manifest should parse");

    assert_eq!(&encoded[0..4], b"sdmf");
    assert_eq!(parsed, sample_manifest());
}

#[test]
fn reads_setup_directory_manifest_from_a_file_path() {
    let path = temp_file_path("manifest.bin");
    fs::write(
        &path,
        encode_setup_directory_manifest(&sample_manifest()).expect("manifest should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_setup_directory_manifest_file(&path).expect("manifest should read");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_manifest());
}

#[test]
fn rejects_invalid_setup_directory_manifest_counts() {
    let mut manifest = sample_manifest();
    manifest.unit_count = 0;
    assert!(matches!(
        encode_setup_directory_manifest(&manifest),
        Err(SetupDirectoryManifestError::EmptyUnits)
    ));

    let mut manifest = sample_manifest();
    manifest.pcs_material_unit_count = 5;
    assert!(matches!(
        encode_setup_directory_manifest(&manifest),
        Err(SetupDirectoryManifestError::InvalidMaterialUnitCount {
            unit_count: 4,
            pcs_material_unit_count: 5
        })
    ));
}

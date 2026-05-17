use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use lzvm_artifacts::setup_manifest::{
    encode_setup_directory_manifest, parse_setup_directory_manifest,
    read_setup_directory_manifest_file, validate_setup_directory_manifest_file,
    SetupDirectoryManifest, SetupDirectoryManifestError,
};

fn sample_manifest() -> SetupDirectoryManifest {
    SetupDirectoryManifest {
        unit_count: 4,
        global_constraint_count: 3,
        fixed_byte_count: 128,
        pcs_material_unit_count: 4,
        pcs_material_byte_count: 512,
        source_fixed_file_manifest_present: true,
        source_fixed_file_manifest_entry_count: 7,
        source_fixed_file_manifest_byte_count: 256,
        source_program_archive_present: true,
        source_program_archive_source_count: 3,
        source_program_archive_edge_count: 2,
        source_program_archive_byte_count: 1024,
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
fn parses_legacy_setup_directory_manifests_without_source_fixed_file_counts() {
    let mut payload = Vec::new();
    for value in [4_u64, 3, 128, 4, 512] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload.extend_from_slice(&[0x55; 32]);
    let bytes = encode_sectioned_file(&SectionedFile {
        kind: *b"sdmf",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: payload,
        }],
    })
    .expect("legacy manifest should encode");

    let parsed = parse_setup_directory_manifest(&bytes).expect("legacy manifest should parse");

    assert!(!parsed.source_fixed_file_manifest_present);
    assert_eq!(parsed.source_fixed_file_manifest_entry_count, 0);
    assert_eq!(parsed.source_fixed_file_manifest_byte_count, 0);
    assert!(!parsed.source_program_archive_present);
    assert_eq!(parsed.source_program_archive_source_count, 0);
    assert_eq!(parsed.source_program_archive_edge_count, 0);
    assert_eq!(parsed.source_program_archive_byte_count, 0);
    assert_eq!(parsed.unit_count, 4);
    assert_eq!(parsed.catalog_digest, [0x55; 32]);
}

#[test]
fn rejects_zero_version_setup_directory_manifests() {
    let mut payload = Vec::new();
    for value in [4_u64, 3, 128, 4, 512] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload.extend_from_slice(&[0x55; 32]);
    let bytes = encode_sectioned_file(&SectionedFile {
        kind: *b"sdmf",
        version: 0,
        sections: vec![SectionedSection {
            id: 1,
            data: payload,
        }],
    })
    .expect("zero version manifest should encode");

    assert_eq!(
        parse_setup_directory_manifest(&bytes).expect_err("zero version manifest should reject"),
        SetupDirectoryManifestError::UnsupportedVersion { found: 0, max: 4 }
    );
}

#[test]
fn parses_legacy_setup_directory_manifests_without_source_program_archive_counts() {
    let mut payload = Vec::new();
    for value in [4_u64, 3, 128, 4, 512, 1, 7] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload.extend_from_slice(&[0x55; 32]);
    let bytes = encode_sectioned_file(&SectionedFile {
        kind: *b"sdmf",
        version: 2,
        sections: vec![SectionedSection {
            id: 1,
            data: payload,
        }],
    })
    .expect("legacy manifest should encode");

    let parsed = parse_setup_directory_manifest(&bytes).expect("legacy manifest should parse");

    assert!(parsed.source_fixed_file_manifest_present);
    assert_eq!(parsed.source_fixed_file_manifest_entry_count, 7);
    assert_eq!(parsed.source_fixed_file_manifest_byte_count, 0);
    assert!(!parsed.source_program_archive_present);
    assert_eq!(parsed.source_program_archive_source_count, 0);
    assert_eq!(parsed.source_program_archive_edge_count, 0);
    assert_eq!(parsed.source_program_archive_byte_count, 0);
    assert_eq!(parsed.unit_count, 4);
    assert_eq!(parsed.catalog_digest, [0x55; 32]);
}

#[test]
fn parses_legacy_setup_directory_manifests_without_source_companion_byte_counts() {
    let mut payload = Vec::new();
    for value in [4_u64, 3, 128, 4, 512, 1, 7, 1, 3, 2] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload.extend_from_slice(&[0x55; 32]);
    let bytes = encode_sectioned_file(&SectionedFile {
        kind: *b"sdmf",
        version: 3,
        sections: vec![SectionedSection {
            id: 1,
            data: payload,
        }],
    })
    .expect("legacy manifest should encode");

    let parsed = parse_setup_directory_manifest(&bytes).expect("legacy manifest should parse");

    assert!(parsed.source_fixed_file_manifest_present);
    assert_eq!(parsed.source_fixed_file_manifest_entry_count, 7);
    assert_eq!(parsed.source_fixed_file_manifest_byte_count, 0);
    assert!(parsed.source_program_archive_present);
    assert_eq!(parsed.source_program_archive_source_count, 3);
    assert_eq!(parsed.source_program_archive_edge_count, 2);
    assert_eq!(parsed.source_program_archive_byte_count, 0);
    assert_eq!(parsed.unit_count, 4);
    assert_eq!(parsed.catalog_digest, [0x55; 32]);
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
fn rejects_setup_directory_manifest_directory_paths() {
    let path = temp_file_path("manifest-directory");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("fixture directory should be created");

    let error = validate_setup_directory_manifest_file(&path, &sample_manifest())
        .expect_err("manifest directory should be rejected");

    assert!(matches!(error, SetupDirectoryManifestError::Io { .. }));
    fs::remove_dir_all(&path).expect("fixture directory should be removed");
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

    let mut manifest = sample_manifest();
    manifest.source_fixed_file_manifest_present = false;
    assert!(matches!(
        encode_setup_directory_manifest(&manifest),
        Err(
            SetupDirectoryManifestError::InvalidSourceFixedFileManifestCounts {
                present: false,
                entry_count: 7,
                byte_count: 256
            }
        )
    ));

    let mut manifest = sample_manifest();
    manifest.source_program_archive_present = false;
    assert!(matches!(
        encode_setup_directory_manifest(&manifest),
        Err(
            SetupDirectoryManifestError::InvalidSourceProgramArchiveCounts {
                present: false,
                source_count: 3,
                edge_count: 2,
                byte_count: 1024
            }
        )
    ));

    let mut manifest = sample_manifest();
    manifest.source_program_archive_source_count = 0;
    assert!(matches!(
        encode_setup_directory_manifest(&manifest),
        Err(
            SetupDirectoryManifestError::InvalidSourceProgramArchiveCounts {
                present: true,
                source_count: 0,
                edge_count: 2,
                byte_count: 1024
            }
        )
    ));
}

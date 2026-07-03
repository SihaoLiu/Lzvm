use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use lzvm_artifacts::setup_manifest::{
    encode_setup_directory_manifest, parse_setup_directory_manifest,
    read_setup_directory_manifest_file, validate_required_setup_directory_manifest_file,
    validate_setup_directory_manifest_file, SetupDirectoryManifest, SetupDirectoryManifestError,
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
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-setup-manifest-{}-{name}", std::process::id()));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

fn encode_legacy_manifest(version: u32, values: &[u64]) -> Vec<u8> {
    let mut payload = Vec::new();
    for value in values {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload.extend_from_slice(&[0x55; 32]);
    encode_sectioned_file(&SectionedFile {
        kind: *b"sdmf",
        version,
        sections: vec![SectionedSection {
            id: 1,
            data: payload,
        }],
    })
    .expect("legacy manifest should encode")
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
    let bytes = encode_legacy_manifest(1, &[4, 3, 128, 4, 512]);

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
    let bytes = encode_legacy_manifest(0, &[4, 3, 128, 4, 512]);

    assert_eq!(
        parse_setup_directory_manifest(&bytes).expect_err("zero version manifest should reject"),
        SetupDirectoryManifestError::UnsupportedVersion { found: 0, max: 4 }
    );
}

#[test]
fn rejects_future_setup_directory_manifest_versions() {
    let bytes = encode_legacy_manifest(5, &[4, 3, 128, 4, 512]);

    assert_eq!(
        parse_setup_directory_manifest(&bytes).expect_err("future version manifest should reject"),
        SetupDirectoryManifestError::UnsupportedVersion { found: 5, max: 4 }
    );
}

#[test]
fn parses_legacy_setup_directory_manifests_without_source_program_archive_counts() {
    let bytes = encode_legacy_manifest(2, &[4, 3, 128, 4, 512, 1, 7]);

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
    let bytes = encode_legacy_manifest(3, &[4, 3, 128, 4, 512, 1, 7, 1, 3, 2]);

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
fn validates_legacy_setup_directory_manifests_without_unencoded_byte_counts() {
    for (name, version, values) in [
        ("v2", 2, vec![4, 3, 128, 4, 512, 1, 7]),
        ("v3", 3, vec![4, 3, 128, 4, 512, 1, 7, 1, 3, 2]),
    ] {
        let path = temp_file_path(name);
        fs::write(&path, encode_legacy_manifest(version, &values))
            .expect("fixture should be written");

        validate_setup_directory_manifest_file(&path, &sample_manifest())
            .expect("legacy manifest should validate");

        fs::remove_file(&path).expect("fixture should be removed");
    }
}

#[test]
fn validates_v1_setup_directory_manifests_without_unencoded_source_fields() {
    let path = temp_file_path("v1");
    fs::write(&path, encode_legacy_manifest(1, &[4, 3, 128, 4, 512]))
        .expect("fixture should be written");

    validate_setup_directory_manifest_file(&path, &sample_manifest())
        .expect("v1 manifest should validate without source fields");

    fs::remove_file(&path).expect("fixture should be removed");
}

#[test]
fn validates_required_legacy_setup_directory_manifests_with_version_projection() {
    for (name, version, values) in [
        ("required-v1", 1, vec![4, 3, 128, 4, 512]),
        ("required-v2", 2, vec![4, 3, 128, 4, 512, 1, 7]),
        ("required-v3", 3, vec![4, 3, 128, 4, 512, 1, 7, 1, 3, 2]),
    ] {
        let path = temp_file_path(name);
        fs::write(&path, encode_legacy_manifest(version, &values))
            .expect("fixture should be written");

        validate_required_setup_directory_manifest_file(&path, &sample_manifest())
            .expect("required legacy manifest should validate");

        fs::remove_file(&path).expect("fixture should be removed");
    }
}

#[test]
fn rejects_legacy_setup_directory_manifests_with_encoded_mismatches() {
    let path = temp_file_path("legacy-mismatch");
    fs::write(&path, encode_legacy_manifest(2, &[4, 3, 128, 4, 512, 1, 8]))
        .expect("fixture should be written");

    let error = validate_setup_directory_manifest_file(&path, &sample_manifest())
        .expect_err("encoded count mismatch should reject");

    assert!(matches!(
        error,
        SetupDirectoryManifestError::Mismatch { .. }
    ));
    fs::remove_file(&path).expect("fixture should be removed");
}

#[test]
fn rejects_legacy_setup_directory_manifests_with_source_archive_mismatches() {
    let path = temp_file_path("legacy-archive-mismatch");
    fs::write(
        &path,
        encode_legacy_manifest(3, &[4, 3, 128, 4, 512, 1, 7, 1, 4, 2]),
    )
    .expect("fixture should be written");

    let error = validate_setup_directory_manifest_file(&path, &sample_manifest())
        .expect_err("encoded source archive mismatch should reject");

    assert!(matches!(
        error,
        SetupDirectoryManifestError::Mismatch { .. }
    ));
    fs::remove_file(&path).expect("fixture should be removed");
}

#[test]
fn rejects_current_setup_directory_manifests_with_source_companion_byte_mismatches() {
    let cases: [(&str, fn(&mut SetupDirectoryManifest)); 2] = [
        (
            "fixed-source-bytes",
            |manifest: &mut SetupDirectoryManifest| {
                manifest.source_fixed_file_manifest_byte_count += 1;
            },
        ),
        (
            "archive-source-bytes",
            |manifest: &mut SetupDirectoryManifest| {
                manifest.source_program_archive_byte_count += 1;
            },
        ),
    ];
    for (name, mutate) in cases {
        let path = temp_file_path(name);
        let mut manifest = sample_manifest();
        mutate(&mut manifest);
        fs::write(
            &path,
            encode_setup_directory_manifest(&manifest).expect("fixture should encode"),
        )
        .expect("fixture should be written");

        let error = validate_setup_directory_manifest_file(&path, &sample_manifest())
            .expect_err("encoded byte-count mismatch should reject");

        assert!(matches!(
            error,
            SetupDirectoryManifestError::Mismatch { .. }
        ));
        fs::remove_file(&path).expect("fixture should be removed");
    }
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
fn rejects_missing_required_setup_directory_manifest_paths() {
    let path = temp_file_path("required-missing");
    let _ = fs::remove_file(&path);

    let error = validate_required_setup_directory_manifest_file(&path, &sample_manifest())
        .expect_err("missing required manifest should reject");

    assert!(matches!(error, SetupDirectoryManifestError::Io { .. }));
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

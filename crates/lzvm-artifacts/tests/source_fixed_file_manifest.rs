use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::sectioned::{encode_sectioned_file, SectionedFile, SectionedSection};
use lzvm_artifacts::source_fixed_file_manifest::{
    encode_source_fixed_file_manifest, parse_source_fixed_file_manifest,
    read_source_fixed_file_manifest_file, SourceFixedFileManifest, SourceFixedFileManifestEntry,
    SourceFixedFileManifestError, SourceFixedFileManifestKind,
};

fn temp_file(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-artifacts-source-fixed-file-manifest-{}-{name}",
            std::process::id()
        ));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

fn sample_manifest() -> SourceFixedFileManifest {
    SourceFixedFileManifest {
        entries: vec![
            SourceFixedFileManifestEntry {
                source_name: "main.pil".to_owned(),
                kind: SourceFixedFileManifestKind::OutputFixedFile,
                path: Some("Main-0.fixed".to_owned()),
                column: None,
                group_name: "Main".to_owned(),
                group_id: 0,
                unit_id: 0,
                unit_name: "Main".to_owned(),
                template_name: "Main".to_owned(),
                virtual_instance: false,
                start: 11,
                end: 61,
            },
            SourceFixedFileManifestEntry {
                source_name: "main.pil".to_owned(),
                kind: SourceFixedFileManifestKind::FixedLoad,
                path: Some("Main-1.fixed".to_owned()),
                column: Some(2),
                group_name: "Main".to_owned(),
                group_id: 0,
                unit_id: 10_000,
                unit_name: "VirtualMain".to_owned(),
                template_name: "Main".to_owned(),
                virtual_instance: true,
                start: 70,
                end: 121,
            },
        ],
    }
}

#[test]
fn encodes_and_parses_source_fixed_file_manifests() {
    let encoded =
        encode_source_fixed_file_manifest(&sample_manifest()).expect("manifest should encode");
    let parsed = parse_source_fixed_file_manifest(&encoded).expect("manifest should parse");

    assert_eq!(&encoded[0..4], b"sffm");
    assert_eq!(parsed, sample_manifest());
}

#[test]
fn rejects_unsupported_source_fixed_file_manifest_versions() {
    let encoded =
        encode_source_fixed_file_manifest(&sample_manifest()).expect("manifest should encode");
    let parsed = lzvm_artifacts::sectioned::parse_sectioned_file(&encoded, *b"sffm", 1)
        .expect("sectioned manifest should parse");

    for version in [0, 2] {
        let encoded = encode_sectioned_file(&SectionedFile {
            kind: *b"sffm",
            version,
            sections: parsed.sections.clone(),
        })
        .expect("sectioned manifest should encode");

        assert_eq!(
            parse_source_fixed_file_manifest(&encoded)
                .expect_err("unsupported manifest version should reject"),
            SourceFixedFileManifestError::UnsupportedVersion {
                found: version,
                expected: 1,
            }
        );
    }
}

#[test]
fn reads_source_fixed_file_manifests_from_file_paths() {
    let path = temp_file("manifest.bin");
    let _ = fs::remove_file(&path);
    fs::write(
        &path,
        encode_source_fixed_file_manifest(&sample_manifest()).expect("manifest should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_source_fixed_file_manifest_file(&path).expect("manifest should read");

    assert_eq!(parsed, sample_manifest());
    fs::remove_file(&path).expect("fixture should be removed");
}

#[test]
fn rejects_invalid_source_fixed_file_manifest_entries() {
    let mut manifest = sample_manifest();
    manifest.entries[0].source_name.clear();
    assert!(matches!(
        encode_source_fixed_file_manifest(&manifest),
        Err(SourceFixedFileManifestError::EmptySourceName { entry_index: 0 })
    ));

    let mut manifest = sample_manifest();
    manifest.entries[0].start = 62;
    assert!(matches!(
        encode_source_fixed_file_manifest(&manifest),
        Err(SourceFixedFileManifestError::InvalidSpan {
            entry_index: 0,
            start: 62,
            end: 61
        })
    ));

    let mut manifest = sample_manifest();
    manifest.entries[0].path = None;
    assert!(matches!(
        encode_source_fixed_file_manifest(&manifest),
        Err(SourceFixedFileManifestError::MissingPath { entry_index: 0 })
    ));

    let mut manifest = sample_manifest();
    manifest.entries[0].kind = SourceFixedFileManifestKind::FixedExternal;
    assert!(matches!(
        encode_source_fixed_file_manifest(&manifest),
        Err(SourceFixedFileManifestError::UnexpectedPath { entry_index: 0 })
    ));

    let mut manifest = sample_manifest();
    manifest.entries[0].column = Some(3);
    assert!(matches!(
        encode_source_fixed_file_manifest(&manifest),
        Err(SourceFixedFileManifestError::UnexpectedColumn { entry_index: 0 })
    ));

    let mut manifest = sample_manifest();
    manifest.entries[1].column = None;
    assert!(matches!(
        encode_source_fixed_file_manifest(&manifest),
        Err(SourceFixedFileManifestError::MissingColumn { entry_index: 1 })
    ));
}

#[test]
fn rejects_source_fixed_file_manifest_entry_counts_larger_than_payload() {
    let bytes = encode_sectioned_file(&SectionedFile {
        kind: *b"sffm",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: 1_u64.to_le_bytes().to_vec(),
        }],
    })
    .expect("sectioned file should encode");

    assert!(matches!(
        parse_source_fixed_file_manifest(&bytes),
        Err(SourceFixedFileManifestError::UnexpectedPayloadEof {
            offset: 8,
            needed: 68,
            available: 0
        })
    ));
}

#[test]
fn rejects_source_fixed_file_manifest_entry_count_span_overflow() {
    let bytes = encode_sectioned_file(&SectionedFile {
        kind: *b"sffm",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: u64::MAX.to_le_bytes().to_vec(),
        }],
    })
    .expect("sectioned file should encode");

    assert!(matches!(
        parse_source_fixed_file_manifest(&bytes),
        Err(SourceFixedFileManifestError::LengthOverflow)
    ));
}

#[test]
fn rejects_truncated_source_fixed_file_manifest_payload() {
    let bytes = encode_sectioned_file(&SectionedFile {
        kind: *b"sffm",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: vec![1],
        }],
    })
    .expect("sectioned file should encode");

    assert!(matches!(
        parse_source_fixed_file_manifest(&bytes),
        Err(SourceFixedFileManifestError::UnexpectedPayloadEof {
            offset: 0,
            needed: 8,
            available: 1
        })
    ));
}

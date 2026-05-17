use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::source_fixed_file_manifest::{
    encode_source_fixed_file_manifest, parse_source_fixed_file_manifest,
    read_source_fixed_file_manifest_file, SourceFixedFileManifest, SourceFixedFileManifestEntry,
    SourceFixedFileManifestError, SourceFixedFileManifestKind,
};

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-artifacts-source-fixed-file-manifest-{}-{name}",
        std::process::id()
    ))
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
}

use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::source_program::{
    encode_source_program_archive, parse_source_program_archive, read_source_program_archive_file,
    SourceProgramArchive, SourceProgramArchiveEdge, SourceProgramArchiveError,
    SourceProgramArchiveIncludeKind, SourceProgramArchiveIncludeVisibility,
    SourceProgramArchiveSource,
};

const HEADER_BYTES: usize = 4 + 4 + 4 + 4;
const MIN_SOURCE_RECORD_BYTES: usize = 8 + 8;
const MIN_EDGE_RECORD_BYTES: usize = 4 + 4 + 8 + 1 + 1;

fn temp_file(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-artifacts-source-program-{}-{name}",
            std::process::id()
        ));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

fn archive_header(source_count: u32, edge_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"spg0");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&source_count.to_le_bytes());
    bytes.extend_from_slice(&edge_count.to_le_bytes());
    bytes
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn encodes_and_parses_source_program_archives() {
    let path = temp_file("archive.bin");
    let _ = fs::remove_file(&path);
    let archive = SourceProgramArchive {
        sources: vec![
            SourceProgramArchiveSource {
                source_name: "main.pil".to_owned(),
                contents: "include \"shared.pil\";".to_owned(),
            },
            SourceProgramArchiveSource {
                source_name: "shared.pil".to_owned(),
                contents: "constant X = 1;".to_owned(),
            },
        ],
        edges: vec![SourceProgramArchiveEdge {
            from_index: 0,
            to_index: 1,
            request: "shared.pil".to_owned(),
            kind: SourceProgramArchiveIncludeKind::Include,
            visibility: SourceProgramArchiveIncludeVisibility::Public,
        }],
    };

    let bytes = encode_source_program_archive(&archive).expect("archive should encode");
    fs::write(&path, &bytes).expect("archive should be written");
    let parsed = read_source_program_archive_file(&path).expect("archive should parse");

    assert_eq!(parsed, archive);
    assert_eq!(parsed.source_count(), 2);

    fs::remove_file(&path).expect("archive should be removed");
}

#[test]
fn rejects_source_program_archives_with_no_sources() {
    assert!(matches!(
        parse_source_program_archive(&archive_header(0, 0)),
        Err(SourceProgramArchiveError::EmptySources)
    ));
}

#[test]
fn rejects_zero_version_source_program_archives() {
    let mut bytes = archive_header(0, 0);
    bytes[4..8].copy_from_slice(&0_u32.to_le_bytes());

    assert_eq!(
        parse_source_program_archive(&bytes),
        Err(SourceProgramArchiveError::UnsupportedVersion { found: 0, max: 1 })
    );
}

#[test]
fn rejects_future_source_program_archive_versions() {
    let mut bytes = archive_header(0, 0);
    bytes[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert_eq!(
        parse_source_program_archive(&bytes),
        Err(SourceProgramArchiveError::UnsupportedVersion { found: 2, max: 1 })
    );
}

#[test]
fn rejects_source_program_archive_counts_larger_than_payload() {
    assert!(matches!(
        parse_source_program_archive(&archive_header(1, 0)),
        Err(SourceProgramArchiveError::UnexpectedEof {
            offset: HEADER_BYTES,
            needed: MIN_SOURCE_RECORD_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_source_program_archive_edge_counts_larger_than_payload() {
    let mut bytes = archive_header(1, 1);
    push_u64(&mut bytes, 1);
    bytes.extend_from_slice(b"a");
    push_u64(&mut bytes, 0);

    assert!(matches!(
        parse_source_program_archive(&bytes),
        Err(SourceProgramArchiveError::UnexpectedEof {
            offset,
            needed: MIN_EDGE_RECORD_BYTES,
            available: 0
        }) if offset == HEADER_BYTES + MIN_SOURCE_RECORD_BYTES + 1
    ));
}

#[test]
fn rejects_short_source_program_archive_magic() {
    assert!(matches!(
        parse_source_program_archive(b"s"),
        Err(SourceProgramArchiveError::UnexpectedEof {
            offset: 0,
            needed: 4,
            available: 1
        })
    ));
}

#[test]
fn rejects_short_source_program_archive_version() {
    assert!(matches!(
        parse_source_program_archive(b"spg0\x01\0"),
        Err(SourceProgramArchiveError::UnexpectedEof {
            offset: 4,
            needed: 4,
            available: 2
        })
    ));
}

#[test]
fn rejects_short_source_program_archive_source_count() {
    assert!(matches!(
        parse_source_program_archive(b"spg0\x01\0\0\0"),
        Err(SourceProgramArchiveError::UnexpectedEof {
            offset: 8,
            needed: 4,
            available: 0
        })
    ));
}

#[test]
fn rejects_short_source_program_archive_edge_count() {
    assert!(matches!(
        parse_source_program_archive(b"spg0\x01\0\0\0\x01\0\0\0"),
        Err(SourceProgramArchiveError::UnexpectedEof {
            offset: 12,
            needed: 4,
            available: 0
        })
    ));
}

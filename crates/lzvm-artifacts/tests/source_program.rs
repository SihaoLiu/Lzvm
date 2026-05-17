use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::source_program::{
    encode_source_program_archive, parse_source_program_archive, read_source_program_archive_file,
    SourceProgramArchive, SourceProgramArchiveEdge, SourceProgramArchiveError,
    SourceProgramArchiveIncludeKind, SourceProgramArchiveIncludeVisibility,
    SourceProgramArchiveSource,
};

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-artifacts-source-program-{}-{name}",
        std::process::id()
    ))
}

fn archive_header(source_count: u32, edge_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"spg0");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&source_count.to_le_bytes());
    bytes.extend_from_slice(&edge_count.to_le_bytes());
    bytes
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
fn rejects_source_program_archive_counts_larger_than_payload() {
    assert!(matches!(
        parse_source_program_archive(&archive_header(1, 0)),
        Err(SourceProgramArchiveError::LengthOverflow)
    ));
}

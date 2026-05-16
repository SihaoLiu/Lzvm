use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::source_program::{
    encode_source_program_archive, SourceProgramArchive, SourceProgramArchiveEdge,
    SourceProgramArchiveIncludeKind, SourceProgramArchiveIncludeVisibility,
    SourceProgramArchiveSource,
};
use lzvm_pil::SourceProgramArchiveLoader;

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-pil-source-program-archive-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn loads_source_programs_from_archives() {
    let path = temp_file("archive.bin");
    let _ = fs::remove_file(&path);
    let archive = SourceProgramArchive {
        sources: vec![
            SourceProgramArchiveSource {
                source_name: "main.pil".to_owned(),
                contents: "include \"shared.pil\";\ncontainer air.main;".to_owned(),
            },
            SourceProgramArchiveSource {
                source_name: "shared.pil".to_owned(),
                contents: "col fixed shared = [1, 2];".to_owned(),
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

    let program = SourceProgramArchiveLoader::load(&path).expect("archive should load");

    assert_eq!(program.graph.sources.len(), 2);
    assert_eq!(program.graph.edges.len(), 1);
    assert_eq!(program.modules.len(), 2);
    assert_eq!(program.modules[0].source_name, "main.pil");
    assert!(program.modules[0]
        .source
        .contents
        .contains("container air.main"));
    assert_eq!(program.modules[1].source_name, "shared.pil");

    fs::remove_file(&path).expect("archive should be removed");
}

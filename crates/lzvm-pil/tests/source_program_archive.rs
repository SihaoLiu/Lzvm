use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_program::{
    encode_source_program_archive, SourceProgramArchive, SourceProgramArchiveEdge,
    SourceProgramArchiveIncludeKind, SourceProgramArchiveIncludeVisibility,
    SourceProgramArchiveSource,
};
use lzvm_pil::{
    build_source_program_archive, SourceLoaderConfig, SourceProgramArchiveLoader,
    SourceProgramLoader,
};

fn temp_file(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-pil-source-program-archive-{}-{name}",
            std::process::id()
        ));
    fs::create_dir_all(path.parent().expect("temp file should have a parent"))
        .expect("fixture directory should be created");
    path
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
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

#[test]
fn builds_source_program_archives_from_loaded_sources() {
    let dir = temp_file("build-dir");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let child_path = dir.join("shared.pil");
    write_file(
        &main_path,
        "include \"shared.pil\";\ncontainer air.main;\ncol witness main.trace;",
    );
    write_file(&child_path, "col fixed shared = [1, 2];");

    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
    });
    let program = loader
        .load_main(&main_path)
        .expect("source program should load");

    let archive = build_source_program_archive(&program).expect("archive should build");

    assert_eq!(archive.sources.len(), 2);
    assert_eq!(archive.edges.len(), 1);
    assert_eq!(archive.sources[0].source_name, "main.pil");
    assert_eq!(archive.sources[1].source_name, "shared.pil");
    assert!(archive.sources[0].contents.contains("container air.main"));
    assert_eq!(archive.edges[0].from_index, 0);
    assert_eq!(archive.edges[0].to_index, 1);
    assert_eq!(archive.edges[0].request, "shared.pil");
    assert_eq!(
        archive.edges[0].kind,
        SourceProgramArchiveIncludeKind::Include
    );
    assert_eq!(
        archive.edges[0].visibility,
        SourceProgramArchiveIncludeVisibility::Public
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn builds_source_program_archives_with_duplicate_requested_include_names() {
    let dir = temp_file("duplicate-requested-name-dir");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let lib = dir.join("lib");
    write_file(
        &main_path,
        "include \"opids.pil\";\ninclude \"nested/uses_lib.pil\";",
    );
    write_file(&dir.join("nested/uses_lib.pil"), "include \"opids.pil\";");
    write_file(&dir.join("opids.pil"), "constant LOCAL = 1;");
    write_file(&lib.join("opids.pil"), "constant LIB = 2;");

    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: dir.clone(),
        include_paths: vec![lib],
        include_path_first: false,
    });
    let program = loader
        .load_main(&main_path)
        .expect("source program should load");

    let archive = build_source_program_archive(&program).expect("archive should build");

    encode_source_program_archive(&archive).expect("archive should encode");
    let source_names = archive
        .sources
        .iter()
        .map(|source| source.source_name.as_str())
        .collect::<Vec<_>>();
    assert!(source_names.contains(&"opids.pil"));
    assert!(source_names.contains(&"lib/opids.pil"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

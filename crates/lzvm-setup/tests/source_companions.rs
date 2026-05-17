use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_fixed_file_manifest::{
    read_source_fixed_file_manifest_file, SOURCE_FIXED_FILE_MANIFEST_FILE,
};
use lzvm_artifacts::source_program::{
    read_source_program_archive_file, SOURCE_PROGRAM_ARCHIVE_FILE,
};
use lzvm_setup::{write_source_companions, SourceCompanionWriteRequest};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-setup-source-companions-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_source_companions_to_setup_directory_defaults() {
    let dir = temp_dir("write");
    let _ = fs::remove_dir_all(&dir);
    let setup_dir = dir.join("setup");
    let main_path = dir.join("main.pil");
    let child_path = dir.join("shared.pil");
    write_file(
        &main_path,
        "include \"shared.pil\";\n\
         airtemplate Main() {\n\
             #pragma output_fixed_file `${AIR_NAME}.fixed`\n\
             #pragma fixed_load `values/${AIR_ID}.bin` 3\n\
         }\n\
         airgroup Main { Main(); }\n\
         col witness main.trace;",
    );
    write_file(&child_path, "col fixed shared = [1, 2];");

    let report = write_source_companions(&SourceCompanionWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_dir: setup_dir.clone(),
    })
    .expect("source companions should be written");

    let archive_path = setup_dir.join(SOURCE_PROGRAM_ARCHIVE_FILE);
    let manifest_path = setup_dir.join(SOURCE_FIXED_FILE_MANIFEST_FILE);
    let archive = read_source_program_archive_file(&archive_path)
        .expect("source program archive should read");
    let manifest = read_source_fixed_file_manifest_file(&manifest_path)
        .expect("source fixed-file manifest should read");

    assert_eq!(report.setup_dir, setup_dir);
    assert_eq!(report.source_program_archive.output_path, archive_path);
    assert_eq!(report.source_fixed_file_manifest.output_path, manifest_path);
    assert_eq!(report.source_program_archive.source_count, 2);
    assert_eq!(report.source_program_archive.edge_count, 1);
    assert_eq!(report.source_fixed_file_manifest.entry_count, 2);
    assert_eq!(archive.sources.len(), 2);
    assert_eq!(archive.edges.len(), 1);
    assert_eq!(manifest.entries.len(), 2);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

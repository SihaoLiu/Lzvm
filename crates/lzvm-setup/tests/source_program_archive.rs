use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_program::read_source_program_archive_file;
use lzvm_setup::{write_source_program_archive, SourceProgramArchiveWriteRequest};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-setup-source-program-archive-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_source_program_archives_from_setup_requests() {
    let dir = temp_dir("write");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let child_path = dir.join("shared.pil");
    let output_path = dir.join("source-program.bin");
    write_file(
        &main_path,
        "include \"shared.pil\";\n\
         container air.main;\n\
         airtemplate Main() {\n\
             #pragma output_fixed_file `${AIR_NAME}.fixed`\n\
         }\n\
         airgroup Main { Main(); Main() alias Second; }\n\
         col witness main.trace;",
    );
    write_file(&child_path, "col fixed shared = [1, 2];");

    let report = write_source_program_archive(&SourceProgramArchiveWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path.clone(),
        output_path: output_path.clone(),
    })
    .expect("archive should be written");

    let archive = read_source_program_archive_file(&output_path)
        .expect("source program archive should parse");
    let bytes_written = fs::metadata(&output_path)
        .expect("output should exist")
        .len();

    assert_eq!(report.output_path, output_path);
    assert_eq!(report.bytes_written, bytes_written);
    assert_eq!(report.source_count, 2);
    assert_eq!(report.edge_count, 1);
    assert_eq!(report.module_count, 2);
    assert_eq!(report.fixed_file_pragma_count, 1);
    assert_eq!(report.air_template_fixed_file_pragma_count, 1);
    assert_eq!(report.air_unit_count, 2);
    assert_eq!(archive.sources.len(), 2);
    assert_eq!(archive.edges.len(), 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

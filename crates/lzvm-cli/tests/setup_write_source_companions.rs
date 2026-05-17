use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_fixed_file_manifest::{
    read_source_fixed_file_manifest_file, SOURCE_FIXED_FILE_MANIFEST_FILE,
};
use lzvm_artifacts::source_program::{
    read_source_program_archive_file, SOURCE_PROGRAM_ARCHIVE_FILE,
};
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-source-companions-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_source_companions_through_setup_namespace() {
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

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-source-companions",
            main_path.to_str().expect("main path should be utf-8"),
            setup_dir.to_str().expect("setup path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    let archive_path = setup_dir.join(SOURCE_PROGRAM_ARCHIVE_FILE);
    let manifest_path = setup_dir.join(SOURCE_FIXED_FILE_MANIFEST_FILE);
    let archive = read_source_program_archive_file(&archive_path)
        .expect("source program archive should read");
    let manifest = read_source_fixed_file_manifest_file(&manifest_path)
        .expect("source fixed-file manifest should read");
    let archive_bytes = fs::metadata(&archive_path)
        .expect("archive should exist")
        .len();
    let manifest_bytes = fs::metadata(&manifest_path)
        .expect("manifest should exist")
        .len();

    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nsource_program_archive_bytes={archive_bytes}\nsource_program_archive_sources=2\nsource_program_archive_edges=1\nsource_fixed_file_manifest_bytes={manifest_bytes}\nsource_fixed_file_manifest_entries=2\nsetup_directory_manifest_refreshed=false\nsetup_dir={}\nsource_program_archive={}\nsource_fixed_file_manifest={}\n",
            setup_dir.display(),
            archive_path.display(),
            manifest_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(archive.sources.len(), 2);
    assert_eq!(archive.edges.len(), 1);
    assert_eq!(manifest.entries.len(), 2);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

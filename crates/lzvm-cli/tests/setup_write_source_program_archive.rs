use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_program::read_source_program_archive_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-source-program-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_source_program_archive_through_setup_namespace() {
    let dir = temp_dir("archive");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let child_path = dir.join("shared.pil");
    let output_path = dir.join("source-program.bin");
    write_file(
        &main_path,
        "include \"shared.pil\";\ncontainer air.main;\ncol witness main.trace;",
    );
    write_file(&child_path, "col fixed shared = [1, 2];");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-source-program-archive",
            main_path.to_str().expect("main path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    let archive = read_source_program_archive_file(&output_path)
        .expect("source program archive should parse");
    let bytes_written = fs::metadata(&output_path)
        .expect("output should exist")
        .len();
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written={}\noutput={}\n",
            bytes_written,
            output_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(archive.sources.len(), 2);
    assert_eq!(archive.edges.len(), 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

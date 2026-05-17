use std::fs;
use std::path::{Path, PathBuf};

use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-pil-archive-summary-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn summarizes_source_program_archives() {
    let dir = temp_dir("summary");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let child_path = dir.join("shared.pil");
    let archive_path = dir.join("source-program.bin");
    write_file(
        &main_path,
        "include \"shared.pil\";\n\
         container air.main;\n\
         const int ROWS = 2**16;\n\
         function fold(int value): int { int tmp = value; return tmp; }\n\
         col witness main.trace;",
    );
    write_file(&child_path, "col fixed shared = [1, 2];");

    let mut archive_stdout = Vec::new();
    let mut archive_stderr = Vec::new();
    let archive_code = run_cli(
        &[
            "pil",
            "archive",
            main_path.to_str().expect("main path should be utf-8"),
            archive_path.to_str().expect("archive path should be utf-8"),
        ],
        &mut archive_stdout,
        &mut archive_stderr,
    );
    assert_eq!(archive_code, 0);
    assert!(archive_stderr.is_empty());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "pil",
            "archive-summary",
            archive_path.to_str().expect("archive path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nsources=2\nedges=1\nmodules=2\nfixed_file_pragmas=0\nair_template_fixed_file_pragmas=0\nair_units=0\nincludes=1\nuses=0\ncontainers=1\nfunctions=1\nconstants=1\nvariables=1\ncolumns=2\nvalues=0\nair_group_values=0\ncommits=0\npublics=0\npublic_tables=0\n"
    );
    assert!(stderr.is_empty());
}

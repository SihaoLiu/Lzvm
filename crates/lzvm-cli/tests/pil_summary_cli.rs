use std::fs;
use std::path::{Path, PathBuf};

use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-pil-summary-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn summarizes_source_program_declarations() {
    let dir = temp_dir("summary");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let child_path = dir.join("shared.pil");
    write_file(
        &main_path,
        "include \"shared.pil\";\n\
         use lib.shared;\n\
         container air.main;\n\
         function fold(int value): int { return value; }\n\
         col witness main.trace[2];\n\
         challenge stage(3) alpha;\n\
         commit stage(2) public(main.trace) main_commit;\n\
         public output = main.trace[0];\n\
         publictable aggregate(sum, fold) table[cols][rows];",
    );
    write_file(
        &child_path,
        "col fixed shared = [1, 2];\n\
         proofval proof.value;\n\
         airgroupval aggregate(sum) group.total;",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "pil",
            "summary",
            main_path.to_str().expect("main path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nsources=2\nedges=1\nmodules=2\nincludes=1\nuses=1\ncontainers=1\nfunctions=1\ncolumns=2\nvalues=2\nair_group_values=1\ncommits=1\npublics=1\npublic_tables=1\n"
    );
    assert!(stderr.is_empty());
}

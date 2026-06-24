use std::fs;
use std::path::{Path, PathBuf};

use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-cli-pil-graph-{}-{name}", std::process::id()))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_source_graph_for_static_includes() {
    let dir = temp_dir("graph");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let child_path = dir.join("shared.pil");
    write_file(&main_path, "include \"shared.pil\";");
    write_file(&child_path, "constant X = 1;");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "pil",
            "graph",
            main_path.to_str().expect("main path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nsources=2\nedges=1\nsource[0]=main.pil\nsource[1]=shared.pil\nedge[0]=main.pil|shared.pil|shared.pil|include|public\n"
    );
    assert!(stderr.is_empty());
}

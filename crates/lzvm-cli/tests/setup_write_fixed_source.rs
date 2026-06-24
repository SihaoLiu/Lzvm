use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::fixed::{read_fixed_columns_file, FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::encode_unit_setup_info;
use lzvm_cli::run_cli;

mod fixtures;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-write-fixed-source-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_fixed_columns_from_binary_setup_and_source_literals() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    let setup_path = dir.join("unit.setup.bin");
    let main_path = dir.join("main.pil");
    let out_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&fixtures::sample_setup_info()).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "col fixed main.left = [11, 13];\n\
         col fixed main.right = [0x11, 0x13];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-fixed-source",
            setup_path.to_str().expect("setup path should be utf-8"),
            main_path.to_str().expect("main path should be utf-8"),
            "group-a",
            "unit-a",
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let columns = read_fixed_columns_file(&out_path).expect("fixed columns should parse");
    let bytes_written = fs::metadata(&out_path).expect("output should exist").len();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written={bytes_written}\ncolumns=2\nrows=2\noutput={}\n",
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(
        columns,
        FixedColumns {
            group_name: "group-a".to_owned(),
            unit_name: "unit-a".to_owned(),
            row_count: 2,
            columns: vec![
                FixedColumn {
                    name: "main.left".to_owned(),
                    dimensions: vec![1],
                    values: vec![11, 13],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![17, 19],
                },
            ],
        }
    );
}

#[test]
fn reports_usage_for_missing_source_fixed_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-fixed-source",
            "unit.setup.bin",
            "main.pil",
            "group-a",
            "unit-a",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-fixed-source [--include-path <dir>] [--include-path-first] <setup-info-bin> <main-file> <group-name> <unit-name> <out-columns-bin>\n"
    );
}

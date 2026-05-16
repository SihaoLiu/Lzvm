use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{
    encode_fixed_columns, read_raw_fixed_column_file, FixedColumn, FixedColumns,
};
use lzvm_artifacts::setup_info::encode_unit_setup_info;
use lzvm_cli::run_cli;

mod fixtures;

fn sample_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 4,
        columns: vec![
            FixedColumn {
                name: "main.left".to_owned(),
                dimensions: vec![1],
                values: vec![1, 2, 3, 4],
            },
            FixedColumn {
                name: "main.right".to_owned(),
                dimensions: vec![1],
                values: vec![10, 20, 30, 40],
            },
        ],
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-write-fixed-native-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_fixed_columns_from_binary_setup_and_binary_columns() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.fixed.bin");
    let out_path = dir.join("unit.const");
    let setup = fixtures::sample_setup_info_with_wide_fixed();
    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(
        &columns_path,
        encode_fixed_columns(&sample_columns()).expect("columns should encode"),
    )
    .expect("columns fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-fixed-native",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let left = read_raw_fixed_column_file(&out_path, &setup, "group-a", "unit-a", 0)
        .expect("left column should read");
    let right = read_raw_fixed_column_file(&out_path, &setup, "group-a", "unit-a", 1)
        .expect("right column should read");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written=64\noutput={}\n",
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(left, [1, 2, 3, 4]);
    assert_eq!(right, [10, 20, 30, 40]);
}

#[test]
fn reports_usage_for_missing_native_fixed_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-fixed-native",
            "unit.setup.bin",
            "unit.fixed.bin",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-fixed-native <setup-info-bin> <columns-bin> <out-const>\n"
    );
}

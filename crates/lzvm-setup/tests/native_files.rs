use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::{parse_constant_tree_bytes, read_constant_tree_file};
use lzvm_artifacts::fixed::{
    encode_fixed_columns, read_raw_fixed_column_file, FixedColumn, FixedColumns,
};
use lzvm_artifacts::setup_info::encode_unit_setup_info;
mod fixtures;

use fixtures::sample_two_column_setup_info;
use lzvm_setup::{
    build_constant_tree_from_fixed_columns, write_base_native_files,
    write_fixed_columns_native_file, BaseNativeWriteReport, FixedExtensionBackend,
};

fn sample_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 2,
        columns: vec![
            FixedColumn {
                name: "main.left".to_owned(),
                dimensions: vec![1],
                values: vec![5, 1],
            },
            FixedColumn {
                name: "main.right".to_owned(),
                dimensions: vec![1],
                values: vec![9, 9],
            },
        ],
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-setup-native-files-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
}

#[test]
fn writes_fixed_and_base_native_files() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.columns.bin");
    let fixed_path = dir.join("unit.const");
    let base_fixed_path = dir.join("base.const");
    let tree_path = dir.join("base.consttree");

    let setup = sample_two_column_setup_info(1, 2, 2, 4);
    let columns = sample_columns();
    write_bytes(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_bytes(
        &columns_path,
        encode_fixed_columns(&columns).expect("columns should encode"),
    );

    let fixed_report = write_fixed_columns_native_file(&setup_path, &columns_path, &fixed_path)
        .expect("fixed file should write");
    let left = read_raw_fixed_column_file(&fixed_path, &setup, "group-a", "unit-a", 0)
        .expect("left column should parse");
    let right = read_raw_fixed_column_file(&fixed_path, &setup, "group-a", "unit-a", 1)
        .expect("right column should parse");
    assert_eq!(fixed_report.path, fixed_path);
    assert_eq!(fixed_report.bytes_written, 32);
    assert_eq!(left, [5, 1]);
    assert_eq!(right, [9, 9]);

    let base_report = write_base_native_files(
        &setup_path,
        &columns_path,
        &base_fixed_path,
        &tree_path,
        FixedExtensionBackend::Cpu,
    )
    .expect("base files should write");
    let expected_tree =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let expected_root = parse_constant_tree_bytes(expected_tree, &setup)
        .expect("expected tree should parse")
        .root()
        .expect("expected root should derive");
    let tree = read_constant_tree_file(&tree_path, &setup).expect("tree should parse");
    let fixed_len = fs::metadata(&base_fixed_path)
        .expect("base fixed output should exist")
        .len();
    let tree_len = fs::metadata(&tree_path)
        .expect("tree output should exist")
        .len();

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(tree.root().expect("tree root should derive"), expected_root);
    assert_eq!(
        base_report,
        BaseNativeWriteReport {
            fixed: lzvm_setup::FixedColumnWriteReport {
                path: base_fixed_path,
                bytes_written: fixed_len
            },
            tree: lzvm_setup::ConstantTreeWriteReport {
                path: tree_path,
                bytes_written: tree_len,
                root: expected_root
            }
        }
    );
}

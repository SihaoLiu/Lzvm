use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::fixed::{read_raw_fixed_column_file, FixedColumn, FixedColumns};
mod fixtures;

use fixtures::sample_two_column_setup_info;
use lzvm_setup::write_base_fixed_columns;

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
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-setup-{}-{name}", std::process::id()))
}

fn staging_entries(parent: &Path) -> Vec<PathBuf> {
    fs::read_dir(parent)
        .expect("directory should be readable")
        .map(|entry| entry.expect("directory entry should exist").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".staging."))
        })
        .collect()
}

#[test]
fn writes_base_fixed_columns_through_validated_staging() {
    let dir = temp_dir("write-fixed");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.const");
    let setup = sample_two_column_setup_info(2, 3, 2, 4);

    let report =
        write_base_fixed_columns(&path, &sample_columns(), &setup).expect("write should succeed");
    let left = read_raw_fixed_column_file(&path, &setup, "group-a", "unit-a", 0)
        .expect("left should read");
    let right = read_raw_fixed_column_file(&path, &setup, "group-a", "unit-a", 1)
        .expect("right should read");
    let staging = staging_entries(path.parent().expect("path should have a parent"));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(report.path, path);
    assert_eq!(report.bytes_written, 64);
    assert_eq!(left, [1, 2, 3, 4]);
    assert_eq!(right, [10, 20, 30, 40]);
    assert!(staging.is_empty());
}

#[test]
fn preserves_existing_output_when_generation_fails() {
    let dir = temp_dir("preserve-fixed");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.const");
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(&path, b"stable-output").expect("stable fixture should be written");
    let setup = sample_two_column_setup_info(2, 3, 2, 4);
    let mut columns = sample_columns();
    columns.columns.pop();

    let result = write_base_fixed_columns(&path, &columns, &setup);
    let stable = fs::read(&path).expect("stable output should still exist");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(result.is_err());
    assert_eq!(stable, b"stable-output");
}

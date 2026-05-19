use std::fs;
use std::path::{Path, PathBuf};

mod fixtures;

use lzvm_artifacts::fixed::{read_fixed_columns_file, FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::encode_unit_setup_info;
use lzvm_setup::{write_fixed_columns_from_source_file, SourceFixedColumnsWriteRequest};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-setup-source-fixed-columns-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_fixed_column_source_artifacts_from_literal_sequences() {
    let dir = temp_dir("literal-sequences");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_base_setup_info();
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "col fixed main.left = [5, 1];\n\
         col fixed main.right = [0x9, 9];",
    );

    let report = write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path: output_path.clone(),
    })
    .expect("fixed columns should be written");

    let columns = read_fixed_columns_file(&output_path).expect("fixed columns should parse");
    let bytes_written = fs::metadata(&output_path)
        .expect("output should exist")
        .len();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(report.output_path, output_path);
    assert_eq!(report.bytes_written, bytes_written);
    assert_eq!(report.column_count, 2);
    assert_eq!(report.row_count, 2);
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
                    values: vec![5, 1],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![9, 9],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_static_expressions() {
    let dir = temp_dir("static-expressions");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_base_setup_info();
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "col fixed main.left = [2**3, (1 + 4) * 3];\n\
         col fixed main.right = [0x10 >> 1, 7 % 4];",
    );

    write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path: output_path.clone(),
    })
    .expect("fixed columns should be written");

    let columns = read_fixed_columns_file(&output_path).expect("fixed columns should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

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
                    values: vec![8, 15],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![8, 3],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_fill_suffix_sequences() {
    let dir = temp_dir("fill-suffix");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_two_column_setup_info(2, 3, 1, 4);
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "col fixed main.left = [1, 0...];\n\
         col fixed main.right = [2**3, 4...];",
    );

    write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path: output_path.clone(),
    })
    .expect("fixed columns should be written");

    let columns = read_fixed_columns_file(&output_path).expect("fixed columns should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        columns,
        FixedColumns {
            group_name: "group-a".to_owned(),
            unit_name: "unit-a".to_owned(),
            row_count: 4,
            columns: vec![
                FixedColumn {
                    name: "main.left".to_owned(),
                    dimensions: vec![1],
                    values: vec![1, 0, 0, 0],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![8, 4, 4, 4],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_static_ranges() {
    let dir = temp_dir("static-ranges");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_two_column_setup_info(2, 3, 1, 4);
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "col fixed main.left = [0..3];\n\
         col fixed main.right = [3..0];",
    );

    write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path: output_path.clone(),
    })
    .expect("fixed columns should be written");

    let columns = read_fixed_columns_file(&output_path).expect("fixed columns should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        columns,
        FixedColumns {
            group_name: "group-a".to_owned(),
            unit_name: "unit-a".to_owned(),
            row_count: 4,
            columns: vec![
                FixedColumn {
                    name: "main.left".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 1, 2, 3],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![3, 2, 1, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_repeat_segments() {
    let dir = temp_dir("repeat-segments");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_two_column_setup_info(2, 3, 1, 4);
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "col fixed main.left = [1:(1 + 2), 2];\n\
         col fixed main.right = [3, 4:3];",
    );

    write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path: output_path.clone(),
    })
    .expect("fixed columns should be written");

    let columns = read_fixed_columns_file(&output_path).expect("fixed columns should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        columns,
        FixedColumns {
            group_name: "group-a".to_owned(),
            unit_name: "unit-a".to_owned(),
            row_count: 4,
            columns: vec![
                FixedColumn {
                    name: "main.left".to_owned(),
                    dimensions: vec![1],
                    values: vec![1, 1, 1, 2],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![3, 4, 4, 4],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_repeating_sequence_patterns() {
    let dir = temp_dir("repeating-patterns");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_two_column_setup_info(3, 4, 1, 4);
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "col fixed main.left = [1, 0]...;\n\
         col fixed main.right = [0..3]...;",
    );

    write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path: output_path.clone(),
    })
    .expect("fixed columns should be written");

    let columns = read_fixed_columns_file(&output_path).expect("fixed columns should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        columns,
        FixedColumns {
            group_name: "group-a".to_owned(),
            unit_name: "unit-a".to_owned(),
            row_count: 8,
            columns: vec![
                FixedColumn {
                    name: "main.left".to_owned(),
                    dimensions: vec![1],
                    values: vec![1, 0, 1, 0, 1, 0, 1, 0],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 1, 2, 3, 0, 1, 2, 3],
                },
            ],
        }
    );
}

#[test]
fn writes_only_setup_fixed_columns_from_larger_source_program() {
    let dir = temp_dir("filters-setup-columns");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_base_setup_info();
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "col fixed other.left = [99, 99];\n\
         col fixed main.left = [5, 1];\n\
         col fixed other.right = [88, 88];\n\
         col fixed main.right = [0x9, 9];",
    );

    write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path: output_path.clone(),
    })
    .expect("fixed columns should be written");

    let columns = read_fixed_columns_file(&output_path).expect("fixed columns should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

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
                    values: vec![5, 1],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![9, 9],
                },
            ],
        }
    );
}

#[test]
fn rejects_fixed_column_sequence_trailing_comma() {
    let dir = temp_dir("trailing-comma");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_base_setup_info();
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(&main_path, "col fixed main.left = [1,];");

    let error = write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path,
    })
    .expect_err("trailing comma should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(error.to_string().contains("sequence token ]"), "{error}");
}

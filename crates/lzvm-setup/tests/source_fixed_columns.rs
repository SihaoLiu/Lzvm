use std::fs;
use std::path::{Path, PathBuf};

mod fixtures;

use lzvm_artifacts::fixed::{read_fixed_columns_file, FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::encode_unit_setup_info;
use lzvm_field::MODULUS;
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
fn writes_fixed_column_source_artifacts_from_multi_item_declarations() {
    let dir = temp_dir("multi-item-declarations");
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
        "airtemplate UnitA() {\n\
             col fixed main.left, main.right;\n\
             main.left = [5, 1];\n\
             main.right = [9, 8];\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
                    values: vec![9, 8],
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
fn writes_fixed_column_source_artifacts_from_signed_field_values() {
    let dir = temp_dir("signed-field-values");
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
        "col fixed main.left = [-2, -1, 0, 1];\n\
         col fixed main.right = [-3..0];",
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
                    values: vec![MODULUS - 2, MODULUS - 1, 0, 1],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![MODULUS - 3, MODULUS - 2, MODULUS - 1, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_column_references() {
    let dir = temp_dir("column-references");
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
         col fixed main.right = main.left;",
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
                    values: vec![0, 1, 2, 3],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_forward_column_references() {
    let dir = temp_dir("forward-column-references");
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
        "col fixed main.right = main.left;\n\
         col fixed main.left = [0..3];",
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
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 1, 2, 3],
                },
                FixedColumn {
                    name: "main.left".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 1, 2, 3],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_offset_expressions() {
    let dir = temp_dir("row-offset-expressions");
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
         col fixed main.right = main.left' + 1;",
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
                    values: vec![2, 3, 4, 1],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_power_expressions() {
    let dir = temp_dir("row-varying-power-expressions");
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
        "col fixed main.left = [2, 3, 4, 5];\n\
         col fixed main.right = main.left ** 2;",
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
                    values: vec![2, 3, 4, 5],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![4, 9, 16, 25],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_division_expressions() {
    let dir = temp_dir("row-varying-division-expressions");
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
        "col fixed main.left = [3, 5, 7, 9];\n\
         col fixed main.right = main.left / 2;",
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
                    values: vec![3, 5, 7, 9],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![
                        (MODULUS + 3) / 2,
                        (MODULUS + 5) / 2,
                        (MODULUS + 7) / 2,
                        (MODULUS + 9) / 2
                    ],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_backslash_expressions() {
    let dir = temp_dir("row-varying-backslash-expressions");
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
        "col fixed main.left = [3, 5, 7, 9];\n\
         col fixed main.right = main.left \\ 2;",
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
                    values: vec![3, 5, 7, 9],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![
                        (MODULUS + 3) / 2,
                        (MODULUS + 5) / 2,
                        (MODULUS + 7) / 2,
                        (MODULUS + 9) / 2
                    ],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_modulo_expressions() {
    let dir = temp_dir("row-varying-modulo-expressions");
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
        "col fixed main.left = [3, 5, 7, 9];\n\
         col fixed main.right = main.left % 4;",
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
                    values: vec![3, 5, 7, 9],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![3, 1, 3, 1],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_shift_expressions() {
    let dir = temp_dir("row-varying-shift-expressions");
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
        "col fixed main.left = [1, 2, 4, 8];\n\
         col fixed main.right = (main.left << 1) + (main.left >> 1);",
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
                    values: vec![1, 2, 4, 8],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![2, 5, 10, 20],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_bitwise_expressions() {
    let dir = temp_dir("row-varying-bitwise-expressions");
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
        "col fixed main.left = [0, 1, 2, 3];\n\
         col fixed main.right = ((main.left & 1) | 4) ^ 2;",
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
                    values: vec![6, 7, 6, 7],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_comparison_expressions() {
    let dir = temp_dir("row-varying-comparison-expressions");
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
        "col fixed main.left = [0, 1, 2, 3];\n\
         col fixed main.right = (main.left < 2) + ((main.left >= 2) * 2) + \
             ((main.left == 1) * 4) + (main.left != 3) + \
             ((main.left <= 0) * 8) + ((main.left > 2) * 16) + (main.left === 3);",
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
                    values: vec![10, 6, 3, 19],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_logical_expressions() {
    let dir = temp_dir("row-varying-logical-expressions");
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
        "col fixed main.left = [0, 1, 2, 0];\n\
         col fixed main.right = (!main.left) + (main.left && 4) + (main.left || 5);",
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
                    values: vec![0, 1, 2, 0],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![6, 5, 6, 6],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_array_constant_indices() {
    let dir = temp_dir("array-constant-indices");
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
        "const int GEN[4] = [1, 3, 9, 27];\n\
         col fixed main.left = [1, GEN[2]..*..];\n\
         col fixed main.right = [0...];",
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
                    values: vec![1, 9, 81, 729],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 0, 0, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_array_constant_index_expressions() {
    let dir = temp_dir("array-constant-index-expressions");
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
        "const int GEN[2] = [2, 3];\n\
         const int SCALE = 5;\n\
         col fixed main.left = [SCALE * GEN[1]...];\n\
         col fixed main.right = [0...];",
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
                    values: vec![15, 15, 15, 15],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 0, 0, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_array_derived_static_operators() {
    let dir = temp_dir("array-derived-static-operators");
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
        "const int GEN[2] = [2, 3];\n\
         col fixed main.left = [GEN[0] << GEN[1], GEN[1] & 1, GEN[0] < GEN[1], !(GEN[1] - 3)];\n\
         col fixed main.right = [GEN[1] >= GEN[0], GEN[0] != GEN[1], GEN[0] || 5, 0 && GEN[1]];",
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
                    values: vec![16, 1, 1, 1],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![1, 1, 2, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_row_varying_array_indices() {
    let dir = temp_dir("row-varying-array-indices");
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
        "const int GEN[4] = [10, 20, 30, 40];\n\
         col fixed main.left = [0, 1, 2, 3];\n\
         col fixed main.right = GEN[main.left];",
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
                    values: vec![10, 20, 30, 40],
                },
            ],
        }
    );
}

#[test]
fn ignores_non_static_source_constant_arrays_for_fixed_columns() {
    let dir = temp_dir("non-static-constant-arrays");
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
        "col witness selector;\n\
         const expr selectors[1] = [selector];\n\
         col fixed main.left = [7...];\n\
         col fixed main.right = [0...];",
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
                    values: vec![7, 7, 7, 7],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 0, 0, 0],
                },
            ],
        }
    );
}

#[test]
fn ignores_non_literal_source_constant_arrays_for_fixed_columns() {
    let dir = temp_dir("non-literal-constant-arrays");
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
        "function make_values(): expr { return 1; }\n\
         const expr values[1] = make_values();\n\
         col fixed main.left = [3...];\n\
         col fixed main.right = [0...];",
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
                    values: vec![3, 3, 3, 3],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 0, 0, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_template_static_variables() {
    let dir = temp_dir("template-static-variables");
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
        "airtemplate UnitA(const int N = 4) {\n\
             const int count;\n\
             if (N == 4) { count = 2; } else { count = 1; }\n\
             col fixed main.left = [1:count, 0...];\n\
             col fixed main.right = [0...];\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
                    values: vec![1, 1, 0, 0],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 0, 0, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_first_duplicate_name() {
    let dir = temp_dir("first-duplicate-name");
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
        "col fixed main.left = [1...];\n\
         col fixed main.left = [2...];\n\
         col fixed main.right = [0...];",
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
                    values: vec![1, 1, 1, 1],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 0, 0, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_domain_constants() {
    let dir = temp_dir("domain-constants");
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
        "const int GEN[4] = [1, 3, 9, 27];\n\
         col fixed main.left = [0..N-1];\n\
         col fixed main.right = [1, GEN[BITS]..*..];",
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
                    values: vec![0, 1, 2, 3, 4, 5, 6, 7],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![
                        1,
                        27,
                        729,
                        19_683,
                        531_441,
                        14_348_907,
                        387_420_489,
                        10_460_353_203,
                    ],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_omega_domain_constant() {
    let dir = temp_dir("omega-domain-constant");
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
        "col fixed main.left = [1, omega, ..*.., omega**(N-1)];\n\
         col fixed main.right = [omega...];",
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
                    values: vec![
                        1,
                        281_474_976_710_656,
                        MODULUS - 1,
                        MODULUS - 281_474_976_710_656
                    ],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![281_474_976_710_656; 4],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_integer_progression_after_omega_value() {
    let dir = temp_dir("omega-integer-progression");
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
        "col fixed main.left = [omega, -1, -2, ..*.., -64];\n\
         col fixed main.right = [0..7];",
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
                    values: vec![
                        16_777_216,
                        MODULUS - 1,
                        MODULUS - 2,
                        MODULUS - 4,
                        MODULUS - 8,
                        MODULUS - 16,
                        MODULUS - 32,
                        MODULUS - 64,
                    ],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 1, 2, 3, 4, 5, 6, 7],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_source_constants() {
    let dir = temp_dir("source-constants");
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
        "const int ROWS = 4;\n\
         const int HALF = ROWS / 2;\n\
         const int STOP = ROWS - 1;\n\
         col fixed main.left = [0..STOP];\n\
         col fixed main.right = [HALF:2, ROWS:2];",
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
                    values: vec![2, 2, 4, 4],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_included_source_constants() {
    let dir = temp_dir("included-source-constants");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_two_column_setup_info(2, 3, 1, 4);
    let setup_path = dir.join("unit.starkinfo.bin");
    let constants_path = dir.join("constants.pil");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &constants_path,
        "const int ROWS = 4;\n\
         const int HALF = ROWS / 2;\n\
         const int STOP = ROWS - 1;",
    );
    write_file(
        &main_path,
        "include \"constants.pil\";\n\
         col fixed main.left = [0..STOP];\n\
         col fixed main.right = [HALF:2, ROWS:2];",
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
                    values: vec![2, 2, 4, 4],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_with_array_dimensions() {
    let dir = temp_dir("array-dimensions");
    let _ = fs::remove_dir_all(&dir);
    let mut setup = fixtures::sample_two_column_setup_info(2, 3, 1, 4);
    setup.constant_columns[1].dimension = 2;
    setup.constant_columns[1].lengths = vec![2];
    setup.n_constants = 3;
    setup.section_widths.insert("const".to_owned(), 3);
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        "const int WIDTH = 2;\n\
         airtemplate UnitA() {\n\
             col fixed main.left = [0..3];\n\
             col fixed main.right[WIDTH];\n\
             main.right[0] = [9, 8, 7, 6];\n\
             main.right[1] = [6, 7, 8, 9];\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
                    name: "main.right[0]".to_owned(),
                    dimensions: vec![1],
                    values: vec![9, 8, 7, 6],
                },
                FixedColumn {
                    name: "main.right[1]".to_owned(),
                    dimensions: vec![1],
                    values: vec![6, 7, 8, 9],
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
fn writes_fixed_column_source_artifacts_from_whole_sequence_repeat_suffixes() {
    let dir = temp_dir("whole-sequence-repeat-suffixes");
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
        "col fixed main.left = [1, 0]:3...;\n\
         col fixed main.right = [1, 2..*..8]:2;",
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
                    values: vec![1, 2, 4, 8, 1, 2, 4, 8],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_range_repeat_segments() {
    let dir = temp_dir("range-repeat-segments");
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
        "col fixed main.left = [0:2..3:2];\n\
         col fixed main.right = [3:2..0:2];",
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
                    values: vec![0, 0, 1, 1, 2, 2, 3, 3],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![3, 3, 2, 2, 1, 1, 0, 0],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_nested_sequence_repeats() {
    let dir = temp_dir("nested-sequence-repeats");
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
        "col fixed main.left = [[1, 0]:2, 7:4];\n\
         col fixed main.right = [[0..1]:2, 9:4];",
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
                    values: vec![1, 0, 1, 0, 7, 7, 7, 7],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![0, 1, 0, 1, 9, 9, 9, 9],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_nested_open_progression_tails() {
    let dir = temp_dir("nested-open-progression-tails");
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
        "col fixed main.left = [[1, 2]:3, [1, 2..*..]];\n\
         col fixed main.right = [[1, 3]:3, [1, 3..+..]];",
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
                    values: vec![1, 2, 1, 2, 1, 2, 1, 2],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![1, 3, 1, 3, 1, 3, 1, 3],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_progression_segments() {
    let dir = temp_dir("progression-segments");
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
        "col fixed main.left = [1, 3..+..7];\n\
         col fixed main.right = [1, 2..*..8];",
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
                    values: vec![1, 3, 5, 7],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![1, 2, 4, 8],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_repeated_progression_endpoints() {
    let dir = temp_dir("repeated-progression-endpoints");
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
        "col fixed main.left = [1:2, 3:2..+..7:2];\n\
         col fixed main.right = [16:2, 8:2..*..2:2];",
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
                    values: vec![1, 1, 3, 3, 5, 5, 7, 7],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![16, 16, 8, 8, 4, 4, 2, 2],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_open_progressions() {
    let dir = temp_dir("open-progressions");
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
        "col fixed main.left = [1, 3..+..];\n\
         col fixed main.right = [1, 2..*..];",
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
                    values: vec![1, 3, 5, 7],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![1, 2, 4, 8],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_comma_delimited_progressions() {
    let dir = temp_dir("comma-delimited-progressions");
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
        "col fixed main.left = [1, 3, ..+.., 7];\n\
         col fixed main.right = [1, 2, ..*.., 8];",
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
                    values: vec![1, 3, 5, 7],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![1, 2, 4, 8],
                },
            ],
        }
    );
}

#[test]
fn writes_fixed_column_source_artifacts_from_reverse_comma_progressions() {
    let dir = temp_dir("reverse-comma-progressions");
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
        "col fixed main.left = [7, 5, ..+.., 1];\n\
         col fixed main.right = [16, 8, ..*.., 2];",
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
                    values: vec![7, 5, 3, 1],
                },
                FixedColumn {
                    name: "main.right".to_owned(),
                    dimensions: vec![1],
                    values: vec![16, 8, 4, 2],
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

use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::fixed::parse_raw_fixed_columns;
use lzvm_artifacts::global_info::read_global_info_binary_file;
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-setup-generate-source-descending-ranges-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_infers_rows_from_descending_source_ranges() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [3..0];\n\
         col fixed main.right = [0..3];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [3, 2, 1, 0]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [0, 1, 2, 3]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_source_range_repeats() {
    let dir = temp_dir("range-repeats");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0:2..3:2];\n\
         col fixed main.right = [3:2..0:2];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 8);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 8);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [0, 0, 1, 1, 2, 2, 3, 3]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [3, 3, 2, 2, 1, 1, 0, 0]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_source_repeat_segments() {
    let dir = temp_dir("repeat-segments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [1:3, 2:3, 3:2];\n\
         col fixed main.right = [0..7];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 8);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 8);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [1, 1, 1, 2, 2, 2, 3, 3]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_boolean_repeat_segments() {
    let dir = temp_dir("boolean-repeat-segments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int SELECTED = 1;\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [1:(SELECTED == 1), 2:(SELECTED == 1)];\n\
         col fixed main.right = [0..1];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 2);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 2);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [1, 2]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [0, 1]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_boolean_repeat_segments_without_length_peer() {
    let dir = temp_dir("boolean-repeat-segments-only");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int SELECTED = 1;\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [1:(SELECTED == 1), 2:(SELECTED == 1)];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 2);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 2);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [1, 2]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_source_constant_ranges() {
    let dir = temp_dir("constant-ranges");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int ROWS = 4;\n\
         const int STOP = ROWS - 1;\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0..STOP];\n\
         col fixed main.right = [ROWS:2, STOP:2];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [0, 1, 2, 3]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [4, 4, 3, 3]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_static_function_air_parameters() {
    let dir = temp_dir("static-function-air-rows");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function row_count(): int {\n\
             int rows = 2;\n\
             while (rows < 4) {\n\
                 rows *= 2;\n\
             }\n\
             return rows;\n\
         }\n\
         airtemplate UnitA(const int N = row_count()) { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5...];\n\
         col fixed main.right = [1, 0]...;",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [5, 5, 5, 5]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [1, 0, 1, 0]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_fixed_columns_from_static_function_constants() {
    let dir = temp_dir("static-function-constant-columns");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function last_row(): int {\n\
             return 3;\n\
         }\n\
         const int LAST = last_row();\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0..LAST];\n\
         col fixed main.right = [LAST:2, 1:2];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [0, 1, 2, 3]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [3, 3, 1, 1]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_calls_in_source_sequences() {
    let dir = temp_dir("static-function-sequences");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function last_row(): int {\n\
             return 3;\n\
         }\n\
         function repeat_count(): int {\n\
             return 2;\n\
         }\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0..last_row()];\n\
         col fixed main.right = [last_row():repeat_count(), 1:repeat_count()];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [0, 1, 2, 3]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [3, 3, 1, 1]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_calls_in_fixed_expressions() {
    let dir = temp_dir("static-function-fixed-expressions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function base_value(): int {\n\
             return 7;\n\
         }\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0..3];\n\
         col fixed main.right = base_value();",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [0, 1, 2, 3]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [7, 7, 7, 7]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_calls_in_array_constant_indices() {
    let dir = temp_dir("static-function-array-index");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function selected_index(): int {\n\
             return 2;\n\
         }\n\
         const int VALUES[4] = [5, 6, 7, 8];\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0..3];\n\
         col fixed main.right = VALUES[selected_index()];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [0, 1, 2, 3]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [7, 7, 7, 7]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_dependent_source_template_row_count_defaults() {
    let dir = temp_dir("template-row-count-dependent-default");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int BASE, const int N = BASE * 2) { }\n\
         airgroup GroupA { UnitA(2); }\n\
         col fixed main.left = [1, 0...];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.stark.n_bits, 2);
    let fixed = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(fixed.columns[0].values.len(), 4);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_boolean_array_constant_indices() {
    let dir = temp_dir("boolean-array-index");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int SELECTED = 1;\n\
         const int VALUES[2] = [5, 6];\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0..3];\n\
         col fixed main.right = VALUES[SELECTED == 1];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [0, 1, 2, 3]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [6, 6, 6, 6]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_nested_source_sequence_repeats() {
    let dir = temp_dir("nested-repeats");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [[1, 0]:2, 7:4];\n\
         col fixed main.right = [[0..1]:2, 9:4];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 8);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 8);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [1, 0, 1, 0, 7, 7, 7, 7]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [0, 1, 0, 1, 9, 9, 9, 9]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_after_source_fill_sequences() {
    let dir = temp_dir("fill-before-range");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [1, 0]...;\n\
         col fixed main.right = [0..3];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [1, 0, 1, 0]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [0, 1, 2, 3]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_source_progression_segments() {
    let dir = temp_dir("progression-segments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [1, 3..+..7];\n\
         col fixed main.right = [1, 2..*..8];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [1, 3, 5, 7]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [1, 2, 4, 8]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_infers_rows_from_comma_delimited_source_progressions() {
    let dir = temp_dir("comma-progressions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [1, 3, ..+.., 7];\n\
         col fixed main.right = [1, 2, ..*.., 8];",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        let setup_path = unit
            .setup_info_binary()
            .expect("setup metadata path should derive");
        let setup =
            read_unit_setup_info_binary_file(&setup_path).expect("setup metadata should parse");
        let columns = parse_raw_fixed_columns(
            &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
            &setup,
            unit.group_name.as_deref().unwrap_or("raw"),
            unit.unit_name.as_deref().unwrap_or("unit"),
        )
        .expect("fixed columns should parse");
        assert_eq!(columns.row_count, 4);
        assert_eq!(columns.columns[0].name, "main.left");
        assert_eq!(columns.columns[0].values, [1, 3, 5, 7]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [1, 2, 4, 8]);
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

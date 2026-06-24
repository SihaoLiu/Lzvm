use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lzvm_artifacts::fixed::parse_raw_fixed_columns;
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
            "lzvm-cli-setup-generate-source-fixed-assignments-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

fn large_static_array_fixed_assignment_source(
    array_count: usize,
    assignment_count: usize,
) -> String {
    let mut source = format!(
        "const int lookup[{array_count}] = [0...];\n\
         airtemplate UnitA(const int N = {assignment_count}) {{\n\
             col fixed table.value;\n"
    );
    source.push_str(&format!(
        "    for (int index = 0; index < {assignment_count}; ++index) {{\n\
                 table.value[index] = index;\n\
             }}\n\
         }}\n\
         airgroup GroupA {{ UnitA(); }}"
    ));
    source
}

fn many_scalar_fixed_assignment_source(
    scalar_count: usize,
    iteration_count: usize,
    assignments_per_iteration: usize,
) -> String {
    let row_count = iteration_count * assignments_per_iteration;
    let mut source = String::new();
    for index in 0..scalar_count {
        source.push_str(&format!("const int scalar{index} = {index};\n"));
    }
    source.push_str(&format!(
        "airtemplate UnitA(const int N = {row_count}) {{\n\
             col fixed table.value;\n\
             for (int outer = 0; outer < {iteration_count}; ++outer) {{\n"
    ));
    for offset in 0..assignments_per_iteration {
        source.push_str(&format!(
            "        table.value[outer * {assignments_per_iteration} + {offset}] = outer + {offset};\n"
        ));
    }
    source.push_str(
        "    }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
    );
    source
}

fn many_static_if_fixed_assignment_source(statement_count: usize) -> String {
    let row_count = statement_count.next_power_of_two();
    let mut source = format!(
        "airtemplate UnitA(const int N = {row_count}) {{\n\
             col fixed table.value;\n"
    );
    for index in 0..statement_count {
        source.push_str(&format!(
            "    if (1) {{\n\
                 table.value[{index}] = {index};\n\
             }}\n"
        ));
    }
    source.push_str(
        "}\n\
         airgroup GroupA { UnitA(); }",
    );
    source
}

fn many_scalar_static_if_fixed_assignment_source(
    scalar_count: usize,
    iteration_count: usize,
    assignments_per_iteration: usize,
) -> String {
    let row_count = iteration_count * assignments_per_iteration;
    let mut source = String::new();
    for index in 0..scalar_count {
        source.push_str(&format!("const int scalar{index} = {index};\n"));
    }
    source.push_str(&format!(
        "airtemplate UnitA(const int N = {row_count}) {{\n\
             col fixed table.value;\n\
             for (int outer = 0; outer < {iteration_count}; ++outer) {{\n"
    ));
    for offset in 0..assignments_per_iteration {
        source.push_str(&format!(
            "        if (1) {{\n\
                     table.value[outer * {assignments_per_iteration} + {offset}] = outer + {offset};\n\
                 }}\n"
        ));
    }
    source.push_str(
        "    }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
    );
    source
}

#[test]
fn generate_key_lowers_source_template_fixed_index_assignments() {
    let dir = temp_dir("template-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col fixed table.value;\n\
             table.value[0] = 7;\n\
             table.value[1] = 9;\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 2);
    assert_eq!(columns.columns[0].name, "table.value");
    assert_eq!(columns.columns[0].values, [7, 9]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_ignores_inactive_template_fixed_index_assignments() {
    let dir = temp_dir("inactive-template-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate Unused() {\n\
             table.value[0] = 7;\n\
             table.value[1] = 9;\n\
         }\n\
         airtemplate UnitA() {\n\
             col fixed table.value;\n\
         }\n\
         airgroup GroupA {\n\
             virtual Unused();\n\
             UnitA();\n\
         }",
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

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("unsupported fixed-column initializer for table.value"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generate_key_lowers_static_source_for_loop_fixed_index_assignments() {
    let dir = temp_dir("template-static-for-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col fixed table.value;\n\
             for (int index = 0; index < 2; ++index) {\n\
                 table.value[index] = index + 7;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 2);
    assert_eq!(columns.columns[0].name, "table.value");
    assert_eq!(columns.columns[0].values, [7, 8]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_source_while_loop_fixed_index_assignments() {
    let dir = temp_dir("template-static-while-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col fixed table.value;\n\
             int index = 0;\n\
             while (index < 2) {\n\
                 table.value[index] = index + 7;\n\
                 ++index;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 2);
    assert_eq!(columns.columns[0].name, "table.value");
    assert_eq!(columns.columns[0].values, [7, 8]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_source_do_while_loop_fixed_index_assignments() {
    let dir = temp_dir("template-static-do-while-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col fixed table.value;\n\
             int index = 0;\n\
             do {\n\
                 table.value[index] = index + 7;\n\
                 ++index;\n\
             } while (index < 2);\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 2);
    assert_eq!(columns.columns[0].name, "table.value");
    assert_eq!(columns.columns[0].values, [7, 8]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_source_switch_fixed_index_assignments() {
    let dir = temp_dir("template-static-switch-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int SELECTED = 1) {\n\
             col fixed table.value;\n\
             switch (SELECTED) {\n\
                 case 1:\n\
                     table.value[0] = 5;\n\
                     table.value[1] = 7;\n\
                 case 2:\n\
                     table.value[0] = 11;\n\
                     table.value[1] = 13;\n\
                 default:\n\
                     table.value[0] = 17;\n\
                     table.value[1] = 19;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(SELECTED: 2); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 2);
    assert_eq!(columns.columns[0].name, "table.value");
    assert_eq!(columns.columns[0].values, [11, 13]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_template_parameter_static_if_fixed_assignments() {
    let dir = temp_dir("template-param-static-if-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int N = 2, const int ENABLED = 1) {\n\
             col fixed table.value;\n\
             if (ENABLED) {\n\
                 table.value[0] = 5;\n\
                 table.value[1] = 7;\n\
             } else {\n\
                 table.value[0] = 11;\n\
                 table.value[1] = 13;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(ENABLED: 0); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 2);
    assert_eq!(columns.columns[0].name, "table.value");
    assert_eq!(columns.columns[0].values, [11, 13]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_boolean_source_for_loop_fixed_index_assignments() {
    let dir = temp_dir("template-boolean-for-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col fixed table.value;\n\
             for (int index = 0; index < 2; ++index) {\n\
                 table.value[index == 1] = index == 1;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 2);
    assert_eq!(columns.columns[0].name, "table.value");
    assert_eq!(columns.columns[0].values, [0, 1]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_keeps_large_array_fixed_assignments_responsive() {
    let dir = temp_dir("large-array-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        large_static_array_fixed_assignment_source(262_144, 4096),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(8),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 4096);
    assert_eq!(columns.columns[0].name, "table.value");
    assert_eq!(columns.columns[0].values[0], 0);
    assert_eq!(columns.columns[0].values[4095], 4095);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_large_row_mapped_fixed_assignments() {
    let dir = temp_dir("large-row-mapped-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 16384) {\n\
             col fixed table.left = [0..3]...;\n\
             col fixed table.right = [0:4..3:4]...;\n\
             col fixed table.out;\n\
             for (int index = 0; index < N; ++index) {\n\
                 table.out[index] = table.left[index] + table.right[index] + index;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 16_384);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out")
        .expect("output column should exist");
    assert_eq!(output.values[0], 0);
    assert_eq!(output.values[1], 2);
    assert_eq!(output.values[4], 5);
    assert_eq!(output.values[16_383], 16_389);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_local_scalars() {
    let dir = temp_dir("row-mapped-fixed-assignments-local-scalars");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.left = [0..3]...;\n\
             col fixed table.out;\n\
             for (int index = 0; index < N; ++index) {\n\
                 int base = table.left[index] + index;\n\
                 int doubled = base * 2;\n\
                 doubled = doubled + 1;\n\
                 table.out[index] = doubled;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out")
        .expect("output column should exist");
    assert_eq!(output.values[0], 1);
    assert_eq!(output.values[1], 5);
    assert_eq!(output.values[4], 9);
    assert_eq!(output.values[1023], 2053);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_local_destructuring() {
    let dir = temp_dir("row-mapped-fixed-assignments-local-destructuring");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.left = [0..3]...;\n\
             col fixed table.right = [5..8]...;\n\
             col fixed table.out;\n\
             for (int index = 0; index < N; ++index) {\n\
                 int [left, right, offset] = [table.left[index], table.right[index], 7];\n\
                 table.out[index] = left + right + offset;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out")
        .expect("output column should exist");
    assert_eq!(output.values[0], 12);
    assert_eq!(output.values[1], 14);
    assert_eq!(output.values[2], 16);
    assert_eq!(output.values[1023], 18);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_local_branches() {
    let dir = temp_dir("row-mapped-fixed-assignments-local-branches");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.left = [0..3]...;\n\
             col fixed table.out;\n\
             for (int index = 0; index < N; ++index) {\n\
                 int value = table.left[index];\n\
                 if (value == 0) {\n\
                     value = index + 10;\n\
                 } else if (value == 1) {\n\
                     value = index + 20;\n\
                 } else {\n\
                     value = index + 30;\n\
                 }\n\
                 table.out[index] = value;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out")
        .expect("output column should exist");
    assert_eq!(output.values[0], 10);
    assert_eq!(output.values[1], 21);
    assert_eq!(output.values[2], 32);
    assert_eq!(output.values[4], 14);
    assert_eq!(output.values[1023], 1053);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_local_switches() {
    let dir = temp_dir("row-mapped-fixed-assignments-local-switches");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.op = [0..3]...;\n\
             col fixed table.out;\n\
             for (int index = 0; index < N; ++index) {\n\
                 int op = table.op[index];\n\
                 int value = 0;\n\
                 switch (op) {\n\
                     case 0,1:\n\
                         value = index + 10;\n\
                     case 2:\n\
                         value = index + 20;\n\
                     default:\n\
                         value = index + 30;\n\
                 }\n\
                 table.out[index] = value;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out")
        .expect("output column should exist");
    assert_eq!(output.values[0], 10);
    assert_eq!(output.values[1], 11);
    assert_eq!(output.values[2], 22);
    assert_eq!(output.values[3], 33);
    assert_eq!(output.values[4], 14);
    assert_eq!(output.values[1023], 1053);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_unbraced_switch_branches() {
    let dir = temp_dir("row-mapped-fixed-assignments-unbraced-switch-branches");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.op = [0..3]...;\n\
             col fixed table.out;\n\
             for (int index = 0; index < N; ++index) {\n\
                 int op = table.op[index];\n\
                 int value = 0;\n\
                 switch (op) {\n\
                     case 0:\n\
                         if (index == 0) value = 7;\n\
                         else value = index + 10;\n\
                     case 1:\n\
                         value = index + 20;\n\
                     default:\n\
                         value = index + 30;\n\
                 }\n\
                 table.out[index] = value;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out")
        .expect("output column should exist");
    assert_eq!(output.values[0], 7);
    assert_eq!(output.values[1], 21);
    assert_eq!(output.values[2], 32);
    assert_eq!(output.values[3], 33);
    assert_eq!(output.values[4], 14);
    assert_eq!(output.values[1023], 1053);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_conditional_array_outputs() {
    let dir = temp_dir("row-mapped-fixed-assignments-conditional-array-outputs");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.base = [0..3]...;\n\
             col fixed table.out[2];\n\
             for (int index = 0; index < N; ++index) {\n\
                 int value = table.base[index];\n\
                 if (index % 2 == 0) {\n\
                     table.out[0][index] = value + 10;\n\
                     table.out[1][index] = value + 20;\n\
                 } else {\n\
                     table.out[0][index] = value + 30;\n\
                     table.out[1][index] = value + 40;\n\
                 }\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let first = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[0]")
        .expect("first output column should exist");
    let second = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[1]")
        .expect("second output column should exist");
    assert_eq!(first.values[0], 10);
    assert_eq!(second.values[0], 20);
    assert_eq!(first.values[1], 31);
    assert_eq!(second.values[1], 41);
    assert_eq!(first.values[1022], 12);
    assert_eq!(second.values[1023], 43);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_conditional_inner_loops() {
    let dir = temp_dir("row-mapped-fixed-assignments-conditional-inner-loops");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.base = [0..3]...;\n\
             col fixed table.out[2];\n\
             for (int index = 0; index < N; ++index) {\n\
                 int value = table.base[index];\n\
                 if (index % 2 == 0) {\n\
                     for (int slot = 0; slot < 2; slot++) {\n\
                         table.out[slot][index] = value + slot * 10;\n\
                     }\n\
                 } else {\n\
                     table.out[0][index] = value + 30;\n\
                     table.out[1][index] = value + 40;\n\
                 }\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let first = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[0]")
        .expect("first output column should exist");
    let second = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[1]")
        .expect("second output column should exist");
    assert_eq!(first.values[0], 0);
    assert_eq!(second.values[0], 10);
    assert_eq!(first.values[1], 31);
    assert_eq!(second.values[1], 41);
    assert_eq!(first.values[1022], 2);
    assert_eq!(second.values[1023], 43);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_local_target_indices() {
    let dir = temp_dir("row-mapped-fixed-assignments-local-target-indices");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.base = [0..3]...;\n\
             col fixed table.out[2];\n\
             const int first_index = 0;\n\
             int second_index = 1;\n\
             for (int index = 0; index < N; ++index) {\n\
                 int value = table.base[index];\n\
                 if (index % 2 == 0) {\n\
                     for (int slot = 0; slot < 2; slot++) {\n\
                         table.out[slot][index] = value + slot * 10;\n\
                     }\n\
                 } else {\n\
                     table.out[first_index][index] = value + 30;\n\
                     table.out[second_index][index] = value + 40;\n\
                 }\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let first = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[0]")
        .expect("first output column should exist");
    let second = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[1]")
        .expect("second output column should exist");
    assert_eq!(first.values[0], 0);
    assert_eq!(second.values[0], 10);
    assert_eq!(first.values[1], 31);
    assert_eq!(second.values[1], 41);
    assert_eq!(first.values[1022], 2);
    assert_eq!(second.values[1023], 43);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_from_indexed_fixed_arrays() {
    let dir = temp_dir("row-mapped-fixed-assignments-indexed-fixed-arrays");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.base[2];\n\
             table.base[0] = [0..3]...;\n\
             table.base[1] = [10..13]...;\n\
             col fixed table.out;\n\
             for (int index = 0; index < N; ++index) {\n\
                 table.out[index] = table.base[0][index] + table.base[1][index];\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out")
        .expect("output column should exist");
    assert_eq!(output.values[0], 10);
    assert_eq!(output.values[1], 12);
    assert_eq!(output.values[2], 14);
    assert_eq!(output.values[3], 16);
    assert_eq!(output.values[1023], 16);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_from_nested_sequence_columns() {
    let dir = temp_dir("row-mapped-fixed-assignments-nested-sequence-columns");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int R = 2;\n\
         airtemplate UnitA(int N = 1024) {\n\
             col fixed source = [[0:R, 1:R]:2, [2:R, 3:R]:2]...;\n\
             col fixed out;\n\
             for (int index = 0; index < N; ++index) {\n\
                 out[index] = source[index];\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "out")
        .expect("output column should exist");
    assert_eq!(
        &output.values[0..16],
        &[0, 0, 1, 1, 0, 0, 1, 1, 2, 2, 3, 3, 2, 2, 3, 3]
    );
    assert_eq!(output.values[1023], 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_through_local_ternary_outputs() {
    let dir = temp_dir("row-mapped-fixed-assignments-local-ternary-outputs");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.base = [0..3]...;\n\
             col fixed table.out[2];\n\
             for (int index = 0; index < N; ++index) {\n\
                 int value = table.base[index];\n\
                 for (int slot = 0; slot < 2; slot++) {\n\
                     table.out[slot][index] = slot == 0 ? value : value + 10;\n\
                 }\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let first = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[0]")
        .expect("first output column should exist");
    let second = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[1]")
        .expect("second output column should exist");
    assert_eq!(first.values[0], 0);
    assert_eq!(second.values[0], 10);
    assert_eq!(first.values[1], 1);
    assert_eq!(second.values[1], 11);
    assert_eq!(first.values[1023], 3);
    assert_eq!(second.values[1023], 13);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_mapped_fixed_assignments_before_ignored_local_effects() {
    let dir = temp_dir("row-mapped-fixed-assignments-before-ignored-local-effects");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 1024) {\n\
             col fixed table.base = [0..3]...;\n\
             col fixed table.out[2];\n\
             col fixed table.flag;\n\
             int count = 0;\n\
             for (int index = 0; index < N; ++index) {\n\
                 int value = table.base[index];\n\
                 if (index % 2 == 0) {\n\
                     for (int slot = 0; slot < 2; slot++) {\n\
                         table.out[slot][index] = value + slot * 10;\n\
                     }\n\
                 } else {\n\
                     table.out[0][index] = value + 30;\n\
                     table.out[1][index] = value + 40;\n\
                 }\n\
                 table.flag[index] = value + 50;\n\
                 ++count;\n\
                 println(\"{}\", count);\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 1024);
    let output = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[0]")
        .expect("first output column should exist");
    let second = columns
        .columns
        .iter()
        .find(|column| column.name == "table.out[1]")
        .expect("second output column should exist");
    let flag = columns
        .columns
        .iter()
        .find(|column| column.name == "table.flag")
        .expect("flag column should exist");
    assert_eq!(output.values[0], 0);
    assert_eq!(second.values[0], 10);
    assert_eq!(output.values[1], 31);
    assert_eq!(second.values[1], 41);
    assert_eq!(output.values[1022], 2);
    assert_eq!(second.values[1023], 43);
    assert_eq!(flag.values[0], 50);
    assert_eq!(flag.values[1], 51);
    assert_eq!(flag.values[1023], 53);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_reuses_fixed_assignment_scalar_values() {
    let dir = temp_dir("many-scalar-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        many_scalar_fixed_assignment_source(2000, 512, 8),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(8),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 4096);
    assert_eq!(columns.columns[0].values[0], 0);
    assert_eq!(columns.columns[0].values[4095], 518);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_keeps_many_static_if_fixed_assignments_responsive() {
    let dir = temp_dir("many-static-if-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(&source_path, many_static_if_fixed_assignment_source(2048));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(8),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 2048);
    assert_eq!(columns.columns[0].values[0], 0);
    assert_eq!(columns.columns[0].values[2047], 2047);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_reuses_fixed_control_scalar_maps() {
    let dir = temp_dir("many-scalar-static-if-fixed-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        many_scalar_static_if_fixed_assignment_source(4000, 512, 8),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(8),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 4096);
    assert_eq!(columns.columns[0].values[0], 0);
    assert_eq!(columns.columns[0].values[4095], 518);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_avoids_fixed_control_scope_maps() {
    let dir = temp_dir("many-scalar-static-if-fixed-control-scope");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        many_scalar_static_if_fixed_assignment_source(8000, 1024, 8),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(12),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 8192);
    assert_eq!(columns.columns[0].values[0], 0);
    assert_eq!(columns.columns[0].values[8191], 1030);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_fixed_array_element_repeating_progressions() {
    let dir = temp_dir("array-element-repeating-progressions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int N = 16) {\n\
             col fixed lane[2];\n\
             lane[0] = [0..3]...;\n\
             lane[1] = [0:4..3:4]...;\n\
         }\n\
         airgroup GroupA { UnitA(); }",
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");

    assert_eq!(columns.row_count, 16);
    assert_eq!(columns.columns[0].name, "lane[0]");
    assert_eq!(
        columns.columns[0].values,
        [0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3]
    );
    assert_eq!(columns.columns[1].name, "lane[1]");
    assert_eq!(
        columns.columns[1].values,
        [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3]
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lzvm_artifacts::fixed::parse_raw_fixed_columns;
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
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

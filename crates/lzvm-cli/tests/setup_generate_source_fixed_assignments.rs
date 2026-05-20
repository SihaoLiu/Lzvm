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

use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::fixed::parse_raw_fixed_columns;
use lzvm_artifacts::global_info::read_global_info_binary_file;
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
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

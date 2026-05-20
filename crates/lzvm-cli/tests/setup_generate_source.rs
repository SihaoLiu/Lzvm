use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::fixed::parse_raw_fixed_columns;
use lzvm_artifacts::global_info::read_global_info_binary_file;
use lzvm_artifacts::key_directory::{read_key_directory_catalog, read_key_directory_layout};
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_artifacts::setup_manifest::SETUP_DIRECTORY_MANIFEST_FILE;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_bootstraps_empty_directory_from_source() {
    let dir = temp_dir("empty-dir");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];\n\
         col fixed main.right = [9, 8];",
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
    assert_eq!(global.air_groups, ["GroupA"]);
    assert_eq!(global.airs[0][0].name, "UnitA");
    assert_eq!(global.airs[0][0].num_rows, 2);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit_count = layout.units.len();
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
        assert_eq!(columns.columns[0].values, [5, 1]);
        assert_eq!(columns.columns[1].name, "main.right");
        assert_eq!(columns.columns[1].values, [9, 8]);
        assert!(unit.constant_tree.is_file());
        assert!(unit.verification_key_binary().is_file());
        assert!(unit
            .pcs_setup_plan()
            .expect("PCS plan path should derive")
            .is_file());
        assert!(unit
            .pcs_setup_material()
            .expect("PCS material path should derive")
            .is_file());
    }

    let manifest_path = dir.join(SETUP_DIRECTORY_MANIFEST_FILE);
    assert!(manifest_path.is_file());
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    assert!(catalog.source_program_archive.is_some());
    assert!(catalog.source_fixed_file_manifest.is_some());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout.starts_with(&format!("status=ok\nsource_fixed_units={unit_count}\n")));
    assert!(stdout.contains("source_program_archive="));
    assert!(stdout.contains("source_fixed_file_manifest="));
    assert!(stdout.contains(&format!("units={unit_count}\n")));
    assert!(stdout.contains("setup_hash="));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_unused_helper_functions_in_source_metadata() {
    let dir = temp_dir("unused-helper");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function helper() { return 1; }\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
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
    assert!(dir.join(SETUP_DIRECTORY_MANIFEST_FILE).is_file());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_local_variables_inside_unused_helpers() {
    let dir = temp_dir("helper-locals");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function helper() { expr packed = 0; return packed; }\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
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
    assert!(dir.join(SETUP_DIRECTORY_MANIFEST_FILE).is_file());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_rejects_source_metadata_that_requires_lowering() {
    let dir = temp_dir("needs-lowering");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { main.left = 1; }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
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
    assert!(stderr.contains("air template statements need constraint lowering support"));
    assert!(!dir.join("pilout.globalInfo.bin").exists());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

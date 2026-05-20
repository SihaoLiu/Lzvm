use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::read_expression_info_binary_file;
use lzvm_artifacts::fixed::parse_raw_fixed_columns;
use lzvm_artifacts::global_info::read_global_info_binary_file;
use lzvm_artifacts::global_program::read_global_program_file;
use lzvm_artifacts::key_directory::{
    read_key_directory_catalog, read_key_directory_layout, KeyUnitKind,
};
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_artifacts::setup_manifest::SETUP_DIRECTORY_MANIFEST_FILE;
use lzvm_cli::run_cli;
use lzvm_field::{Ext3, Felt};
use lzvm_prover::global_constraints::{evaluate_global_constraints, GlobalConstraintInputs};

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
fn generate_key_skips_source_global_variable_declarations() {
    let dir = temp_dir("global-variable");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int MODE_DEFAULT = 0;\n\
         int MODE = MODE_DEFAULT;\n\
         function set_mode(const int mode) { MODE = mode; }\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 2);
    assert!(dir.join(SETUP_DIRECTORY_MANIFEST_FILE).is_file());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_source_top_level_initializer_blocks() {
    let dir = temp_dir("initializer-blocks");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int FIELD_A = 0;\n\
         int ACTIVE_FIELD = FIELD_A;\n\
         int TABLE[2];\n\
         for (int i = 0; i < length(TABLE); i++) {\n\
             TABLE[i] = 0;\n\
         }\n\
         switch (ACTIVE_FIELD) {\n\
             case FIELD_A:\n\
                 assert(length(TABLE) == 2);\n\
                 for (int i = 0; i < length(TABLE); i++) {\n\
                     TABLE[i] = i;\n\
                 }\n\
             default:\n\
                 TABLE[0] = 1;\n\
         }\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].num_rows, 2);
    assert!(dir.join(SETUP_DIRECTORY_MANIFEST_FILE).is_file());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_top_level_statements_from_include_fragments() {
    let dir = temp_dir("include-fragment-statement");
    let _ = fs::remove_dir_all(&dir);
    let source_dir = dir.join("source");
    let source_path = source_dir.join("main.pil");
    write_file(
        &source_dir.join("fragment.pil"),
        "chunk[0] = x + y;\n\
         chunk[1] = x - y;",
    );
    write_file(
        &source_path,
        "include \"fragment.pil\";\n\
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
fn generate_key_skips_virtual_only_template_columns() {
    let dir = temp_dir("virtual-template-columns");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate VirtualUnit(const int width) {\n\
             int dynamic_width = width;\n\
             col fixed table[dynamic_width];\n\
             col witness trace[dynamic_width];\n\
         }\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA {\n\
             virtual VirtualUnit(2);\n\
             UnitA();\n\
         }\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.airs[0][0].name, "UnitA");
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(
        setup
            .constant_columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        ["main.left"]
    );
    assert!(setup.commitment_columns.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_columns_inside_unused_helper_functions() {
    let dir = temp_dir("helper-function-columns");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function helper(const int id) {\n\
             col fixed `HELPER_${id}`[2];\n\
             col witness helper_trace[2];\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(
        setup
            .constant_columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        ["main.left"]
    );
    assert!(setup.commitment_columns.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_template_defaults_for_column_dimensions() {
    let dir = temp_dir("template-default-column-dimensions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int WIDTH = 2) {\n\
             col witness trace[WIDTH];\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.commitment_columns.len(), 1);
    assert_eq!(setup.commitment_columns[0].name, "trace");
    assert_eq!(setup.commitment_columns[0].lengths, [2]);
    assert_eq!(setup.commitment_columns[0].dimension, 2);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_template_local_constants_for_column_dimensions() {
    let dir = temp_dir("template-local-constant-dimensions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int BASE = 2) {\n\
             const int WIDTH = BASE + 1;\n\
             col witness trace[WIDTH];\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.commitment_columns.len(), 1);
    assert_eq!(setup.commitment_columns[0].name, "trace");
    assert_eq!(setup.commitment_columns[0].lengths, [3]);
    assert_eq!(setup.commitment_columns[0].dimension, 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_constants_for_column_dimensions() {
    let dir = temp_dir("static-function-dimensions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function chunk_size(): int {\n\
             int chunks = 1;\n\
             while (2 ** (chunks + 1) < 20) {\n\
                 chunks += 1;\n\
             }\n\
             return chunks;\n\
         }\n\
         const int WIDTH = chunk_size();\n\
         airtemplate UnitA() {\n\
             col witness trace[WIDTH];\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.commitment_columns.len(), 1);
    assert_eq!(setup.commitment_columns[0].name, "trace");
    assert_eq!(setup.commitment_columns[0].lengths, [4]);
    assert_eq!(setup.commitment_columns[0].dimension, 4);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_template_static_assignments_for_column_dimensions() {
    let dir = temp_dir("template-static-assignment-dimensions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int FLAG_A = 0x01;\n\
         airtemplate UnitA(const int enable = FLAG_A, const int base = 3) {\n\
             const int enabled = (enable & FLAG_A) ? 1 : 0;\n\
             const int width;\n\
             width = enabled * base * 4;\n\
             const int half = width / 2;\n\
             col witness trace[half];\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.commitment_columns.len(), 1);
    assert_eq!(setup.commitment_columns[0].name, "trace");
    assert_eq!(setup.commitment_columns[0].lengths, [6]);
    assert_eq!(setup.commitment_columns[0].dimension, 6);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_known_source_metadata_directives() {
    let dir = temp_dir("metadata-directive");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "enable_range_stats();\n\
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
fn generate_key_writes_source_proof_values_to_global_metadata() {
    let dir = temp_dir("proof-value-metadata");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval enable_flag;\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.num_proof_values, [1]);
    assert_eq!(global.proof_values_map.len(), 1);
    assert_eq!(global.proof_values_map[0].name, "enable_flag");
    assert_eq!(global.proof_values_map[0].stage, 1);
    assert_eq!(global.proof_values_map[0].id, None);
    assert!(global.proof_values_map[0].lengths.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_source_challenge_counts_to_metadata() {
    let dir = temp_dir("challenge-metadata");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "challenge stage(1) alpha;\n\
         challenge stage(2) beta;\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.num_challenges, [1, 1]);
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.challenge_count, 2);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_merges_guarded_source_challenge_declarations() {
    let dir = temp_dir("guarded-challenge-metadata");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function first() {\n\
             if (!defined(alpha)) { challenge stage(2) alpha; }\n\
         }\n\
         function second() {\n\
             if (!defined(alpha)) { challenge stage(2) alpha; }\n\
         }\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.num_challenges, [0, 1]);
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.challenge_count, 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_source_air_values_to_unit_metadata() {
    let dir = temp_dir("air-value-metadata");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int WIDTH = 2;\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];\n\
         airval stage(1) air.flag;\n\
         airval stage(2) air.acc[WIDTH];",
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
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.unit_value_map.len(), 2);
    assert_eq!(setup.unit_value_map[0].name, "air.flag");
    assert_eq!(setup.unit_value_map[0].stage, 1);
    assert!(setup.unit_value_map[0].lengths.is_empty());
    assert_eq!(setup.unit_value_map[1].name, "air.acc");
    assert_eq!(setup.unit_value_map[1].stage, 2);
    assert_eq!(setup.unit_value_map[1].lengths, [2]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_source_air_group_values_to_metadata() {
    let dir = temp_dir("air-group-value-metadata");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int WIDTH = 2;\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];\n\
         airgroupval stage(2) aggregate(sum) group.total;\n\
         airgroupval stage(2) aggregate(prod) group.product[WIDTH];",
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
    assert_eq!(global.aggregation_types.len(), 1);
    assert_eq!(global.aggregation_types[0].len(), 2);
    assert_eq!(global.aggregation_types[0][0].aggregation_type, 0);
    assert_eq!(global.aggregation_types[0][1].aggregation_type, 1);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.group_value_map.len(), 2);
    assert_eq!(setup.group_value_map[0].name, "group.total");
    assert_eq!(setup.group_value_map[0].stage, 2);
    assert!(setup.group_value_map[0].lengths.is_empty());
    assert_eq!(setup.group_value_map[1].name, "group.product");
    assert_eq!(setup.group_value_map[1].stage, 2);
    assert_eq!(setup.group_value_map[1].lengths, [2]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_identity_source_air_group_value_defaults() {
    let dir = temp_dir("air-group-value-identity-defaults");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];\n\
         airgroupval aggregate(sum) default(0) group.total;\n\
         airgroupval aggregate(prod) default(1) group.product;",
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
    assert_eq!(global.aggregation_types[0][0].aggregation_type, 0);
    assert_eq!(global.aggregation_types[0][1].aggregation_type, 1);
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.group_value_map.len(), 2);
    assert_eq!(setup.group_value_map[0].name, "group.total");
    assert_eq!(setup.group_value_map[1].name, "group.product");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_later_stage_source_proof_values_to_global_metadata() {
    let dir = temp_dir("later-proof-value-metadata");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval stage(2) extension_flag;\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.num_proof_values, [0, 1]);
    assert_eq!(global.proof_values_map.len(), 1);
    assert_eq!(global.proof_values_map[0].name, "extension_flag");
    assert_eq!(global.proof_values_map[0].stage, 2);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_proof_value_boolean_constraints() {
    let dir = temp_dir("proof-value-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval enable_flag;\n\
         enable_flag * (1 - enable_flag);\n\
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
    let program = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(program.constraints.entries.len(), 1);

    let satisfied_zero = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::from_u64(0)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("zero proof value should evaluate");
    assert_eq!(satisfied_zero, [Ext3::ZERO]);

    let satisfied_one = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::from_u64(1)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("one proof value should evaluate");
    assert_eq!(satisfied_one, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("non-boolean proof value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_source_public_values_to_global_metadata() {
    let dir = temp_dir("public-value-metadata");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public inputs[4];\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.n_publics, 1);
    assert_eq!(global.publics_map.len(), 1);
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].stage, 1);
    assert_eq!(global.publics_map[0].lengths, [4]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_source_witness_layout_to_unit_metadata() {
    let dir = temp_dir("witness-layout-metadata");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int WIDTH = 2;\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col witness bits(1) main.trace[WIDTH], main.flag;\n\
         col witness stage(2) aux.acc[3];\n\
         col derived stage(2) aux.helper[2];\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.n_stages, 1);
    assert_eq!(setup.section_widths.get("cm1"), Some(&3));
    assert_eq!(setup.section_widths.get("cm2"), Some(&5));
    assert_eq!(setup.commitment_columns.len(), 4);
    assert_eq!(setup.commitment_columns[0].name, "main.trace");
    assert_eq!(setup.commitment_columns[0].stage, 1);
    assert_eq!(setup.commitment_columns[0].dimension, 2);
    assert_eq!(setup.commitment_columns[0].pols_map_id, 0);
    assert_eq!(setup.commitment_columns[0].stage_id, 0);
    assert_eq!(setup.commitment_columns[0].stage_position, 0);
    assert!(!setup.commitment_columns[0].intermediate);
    assert_eq!(setup.commitment_columns[0].lengths, [2]);
    assert_eq!(setup.commitment_columns[1].name, "main.flag");
    assert_eq!(setup.commitment_columns[1].stage, 1);
    assert_eq!(setup.commitment_columns[1].dimension, 1);
    assert_eq!(setup.commitment_columns[1].stage_position, 2);
    assert_eq!(setup.commitment_columns[2].name, "aux.acc");
    assert_eq!(setup.commitment_columns[2].stage, 2);
    assert_eq!(setup.commitment_columns[2].dimension, 3);
    assert_eq!(setup.commitment_columns[2].stage_id, 0);
    assert_eq!(setup.commitment_columns[2].stage_position, 0);
    assert!(!setup.commitment_columns[2].intermediate);
    assert_eq!(setup.commitment_columns[3].name, "aux.helper");
    assert_eq!(setup.commitment_columns[3].stage, 2);
    assert_eq!(setup.commitment_columns[3].dimension, 2);
    assert_eq!(setup.commitment_columns[3].stage_id, 1);
    assert_eq!(setup.commitment_columns[3].stage_position, 3);
    assert!(setup.commitment_columns[3].intermediate);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_source_template_column_declarations() {
    let dir = temp_dir("template-column-declarations");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness local.trace;\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.commitment_columns.len(), 1);
    assert_eq!(setup.commitment_columns[0].name, "local.trace");
    assert_eq!(setup.commitment_columns[0].stage, 1);
    assert_eq!(setup.commitment_columns[0].stage_position, 0);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_source_template_air_value_declarations() {
    let dir = temp_dir("template-air-value-declarations");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int WIDTH = 2;\n\
         airtemplate UnitA() {\n\
             airval local.flag;\n\
             airval stage(2) local.acc[WIDTH];\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.unit_value_map.len(), 2);
    assert_eq!(setup.unit_value_map[0].name, "local.flag");
    assert_eq!(setup.unit_value_map[0].stage, 1);
    assert!(setup.unit_value_map[0].lengths.is_empty());
    assert_eq!(setup.unit_value_map[1].name, "local.acc");
    assert_eq!(setup.unit_value_map[1].stage, 2);
    assert_eq!(setup.unit_value_map[1].lengths, [2]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_statically_inactive_template_air_values() {
    let dir = temp_dir("inactive-template-air-values");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int count = 0;\n\
             if (count > 0) {\n\
                 airval stage(2) skipped[UNKNOWN_WIDTH];\n\
             }\n\
             airval active.flag;\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.unit_value_map.len(), 1);
    assert_eq!(setup.unit_value_map[0].name, "active.flag");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_air_values_inside_helper_functions() {
    let dir = temp_dir("helper-function-air-values");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function declare_helper_values() {\n\
             airval stage(2) skipped[UNKNOWN_WIDTH];\n\
         }\n\
         airtemplate UnitA() {\n\
             airval active.flag;\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup_path = layout.units[0]
        .setup_info_binary()
        .expect("setup metadata path should derive");
    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup metadata should parse");
    assert_eq!(setup.unit_value_map.len(), 1);
    assert_eq!(setup.unit_value_map[0].name, "active.flag");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_template_witness_boolean_constraints() {
    let dir = temp_dir("template-witness-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness flag;\n\
             flag * (1 - flag) === 0;\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let expression_path = unit
        .expression_info_binary()
        .expect("expression metadata path should derive");
    let expressions = read_expression_info_binary_file(expression_path)
        .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert_eq!(expressions.constraints[0].stage, 1);
    assert_eq!(expressions.constraints[0].temporary_count, 2);
    assert_eq!(expressions.constraints[0].operations.len(), 2);

    let regular_path = unit
        .expression_program()
        .expect("regular program path should derive");
    let regular = read_regular_program_file(regular_path).expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
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
fn generate_key_uses_source_template_row_count_for_fill_sequences() {
    let dir = temp_dir("template-row-count-fill");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int N = 4) { }\n\
         airgroup GroupA { UnitA(); }\n\
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
    let columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    assert_eq!(columns.row_count, 4);
    assert_eq!(columns.columns[0].values, [1, 0, 0, 0]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_source_template_row_count_for_repeated_sequences() {
    let dir = temp_dir("template-row-count-repeat");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int N = 8) { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [1:3, 2:3, 3:2];",
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
    assert_eq!(columns.row_count, 8);
    assert_eq!(columns.columns[0].values, [1, 1, 1, 2, 2, 2, 3, 3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_writes_per_unit_source_row_counts() {
    let dir = temp_dir("per-unit-row-counts");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int N = 2) { }\n\
         airtemplate UnitB(const int N = 4) { }\n\
         airgroup GroupA { UnitA(); UnitB(); }",
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
    assert_eq!(global.airs[0][1].num_rows, 4);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let basic_units = layout
        .units
        .iter()
        .filter(|unit| unit.kind == KeyUnitKind::Basic)
        .collect::<Vec<_>>();
    let first_setup = read_unit_setup_info_binary_file(
        basic_units[0]
            .setup_info_binary()
            .expect("first setup metadata path should derive"),
    )
    .expect("first setup metadata should parse");
    let second_setup = read_unit_setup_info_binary_file(
        basic_units[1]
            .setup_info_binary()
            .expect("second setup metadata path should derive"),
    )
    .expect("second setup metadata should parse");
    assert_eq!(first_setup.stark.n_bits, 1);
    assert_eq!(second_setup.stark.n_bits, 2);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_source_custom_commit_declarations() {
    let dir = temp_dir("custom-commit");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public root[4];\n\
         airtemplate UnitA() {\n\
             commit stage(0) public(root) table;\n\
             col table word;\n\
         }\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.n_publics, 1);
    assert_eq!(global.publics_map[0].name, "root");
    assert_eq!(global.publics_map[0].lengths, [4]);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let setup = read_unit_setup_info_binary_file(
        layout.units[0]
            .setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.commitment_columns.len(), 1);
    assert_eq!(setup.commitment_columns[0].name, "word");
    assert!(setup.commitment_columns[0].intermediate);
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

#[test]
fn generate_key_rejects_top_level_expression_statements() {
    let dir = temp_dir("top-level-expression");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "unknown_setup_directive();\n\
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

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("top-level statements need global constraint lowering support"));
    assert!(!dir.join("pilout.globalInfo.bin").exists());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::global_info::read_global_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-setup-generate-source-static-template-values-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_uses_static_template_for_values_in_public_dimensions() {
    let dir = temp_dir("static-template-for-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int width = 0;\n\
             for (int index = 0; index < 3; ++index) {\n\
                 width += 1;\n\
             }\n\
             public inputs[width];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_instance_arguments_in_static_template_for_values() {
    let dir = temp_dir("static-template-for-instance-argument");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int COUNT = 2) {\n\
             int width = 0;\n\
             for (int index = 0; index < COUNT; ++index) {\n\
                 width += 1;\n\
             }\n\
             public inputs[width];\n\
         }\n\
         airgroup GroupA { UnitA(COUNT: 4); }\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [4]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_while_values_in_public_dimensions() {
    let dir = temp_dir("static-template-while-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int width = 0;\n\
             while (width < 3) {\n\
                 width += 1;\n\
             }\n\
             public inputs[width];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_do_while_values_in_public_dimensions() {
    let dir = temp_dir("static-template-do-while-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int width = 1;\n\
             do {\n\
                 width += 1;\n\
             } while (width < 1);\n\
             public inputs[width];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [2]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_do_while_values_in_public_dimensions() {
    let dir = temp_dir("static-function-do-while-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function input_width(): int {\n\
             int width = 0;\n\
             do {\n\
                 width += 1;\n\
             } while (width < 3);\n\
             return width;\n\
         }\n\
         const int WIDTH = input_width();\n\
         airtemplate UnitA() {\n\
             public inputs[WIDTH];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_switch_values_in_public_dimensions() {
    let dir = temp_dir("static-function-switch-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function input_width(int selected): int {\n\
             switch (selected) {\n\
                 case 1:\n\
                     return 3;\n\
                 case 2:\n\
                     return 5;\n\
                 default:\n\
                     return 7;\n\
             }\n\
         }\n\
         const int WIDTH = input_width(2);\n\
         airtemplate UnitA() {\n\
             public inputs[WIDTH];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [5]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_loop_control_in_public_dimensions() {
    let dir = temp_dir("static-function-loop-control-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function input_width(): int {\n\
             int width = 0;\n\
             for (int index = 0; index < 6; ++index) {\n\
                 if (index == 1) {\n\
                     continue;\n\
                 }\n\
                 if (index == 4) {\n\
                     break;\n\
                 }\n\
                 width += 1;\n\
             }\n\
             return width;\n\
         }\n\
         const int WIDTH = input_width();\n\
         airtemplate UnitA() {\n\
             public inputs[WIDTH];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_unbraced_if_else_in_public_dimensions() {
    let dir = temp_dir("static-function-unbraced-if-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function input_width(int selected): int {\n\
             int width = 1;\n\
             if (selected == 1)\n\
                 width += 2;\n\
             else\n\
                 width += 4;\n\
             return width;\n\
         }\n\
         const int WIDTH = input_width(1);\n\
         airtemplate UnitA() {\n\
             public inputs[WIDTH];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_function_unbraced_loop_body_in_public_dimensions() {
    let dir = temp_dir("static-function-unbraced-loop-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function input_width(): int {\n\
             int width = 0;\n\
             for (int index = 0; index < 3; ++index)\n\
                 width += 1;\n\
             return width;\n\
         }\n\
         const int WIDTH = input_width();\n\
         airtemplate UnitA() {\n\
             public inputs[WIDTH];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_break_in_public_dimensions() {
    let dir = temp_dir("static-template-break-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int width = 0;\n\
             for (int index = 0; index < 4; ++index) {\n\
                 width += 1;\n\
                 if (width == 2) {\n\
                     break;\n\
                 }\n\
             }\n\
             public inputs[width];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [2]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_continue_in_public_dimensions() {
    let dir = temp_dir("static-template-continue-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int width = 0;\n\
             for (int index = 0; index < 3; ++index) {\n\
                 width += 1;\n\
                 continue;\n\
                 width += 10;\n\
             }\n\
             public inputs[width];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_nested_static_template_continue_for_assertions() {
    let dir = temp_dir("nested-static-template-continue-assertion");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int count = 0;\n\
             for (int i = 0; i < 4; ++i) {\n\
                 for (int j = 0; j < 4; ++j) {\n\
                     if (i > 1 && i == j) {\n\
                         continue;\n\
                     }\n\
                     ++count;\n\
                 }\n\
             }\n\
             assert(count == 14);\n\
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
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn generate_key_uses_static_template_switch_values_in_public_dimensions() {
    let dir = temp_dir("static-template-switch-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int SELECTED = 2) {\n\
             int width = 0;\n\
             switch (SELECTED) {\n\
                 case 1:\n\
                     width = 2;\n\
                     break;\n\
                 case 2:\n\
                     width = 4;\n\
                     break;\n\
                 default:\n\
                     width = 1;\n\
             }\n\
             public inputs[width];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [4]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_unbroken_static_template_switch_branch_values() {
    let dir = temp_dir("static-template-switch-branch-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int SELECTED = 1) {\n\
             int width = 0;\n\
             switch (SELECTED) {\n\
                 case 1:\n\
                     width = 3;\n\
                 case 2:\n\
                     width = 5;\n\
                 default:\n\
                     width = 7;\n\
             }\n\
             public inputs[width];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_switch_multi_case_branch_values() {
    let dir = temp_dir("static-template-switch-multi-case-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int SELECTED = 2) {\n\
             int width = 0;\n\
             switch (SELECTED) {\n\
                 case 1,2:\n\
                     width = 4;\n\
                 default:\n\
                     width = 7;\n\
             }\n\
             public inputs[width];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [4]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_array_assignments_in_public_dimensions() {
    let dir = temp_dir("static-template-array-assignment-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int widths[2] = [1, 1];\n\
             widths[1] = 4;\n\
             public inputs[widths[1]];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [4]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_uninitialized_array_assignments_in_public_dimensions() {
    let dir = temp_dir("static-template-uninitialized-array-assignment-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int widths[2];\n\
             widths[0] = 1;\n\
             widths[1] = 3;\n\
             public inputs[widths[1]];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [3]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_array_increment_in_public_dimensions() {
    let dir = temp_dir("static-template-array-increment-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int widths[2] = [1, 1];\n\
             ++widths[1];\n\
             public inputs[widths[1]];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [2]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_array_postfix_increment_in_public_dimensions() {
    let dir = temp_dir("static-template-array-postfix-increment-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int widths[2] = [1, 1];\n\
             widths[1]++;\n\
             public inputs[widths[1]];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [2]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_uses_static_template_array_postfix_decrement_in_public_dimensions() {
    let dir = temp_dir("static-template-array-postfix-decrement-public-dimension");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int widths[2] = [1, 3];\n\
             widths[1]--;\n\
             public inputs[widths[1]];\n\
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
    assert_eq!(global.publics_map[0].name, "inputs");
    assert_eq!(global.publics_map[0].lengths, [2]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

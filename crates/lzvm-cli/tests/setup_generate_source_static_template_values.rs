use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::global_info::read_global_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
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

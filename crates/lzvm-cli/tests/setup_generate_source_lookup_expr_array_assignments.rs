use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::hint_program::{HintOperand, SOURCE_LOOKUP_PROVES_HINT};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-lookup-expr-array-assignments-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_expands_local_expr_array_element_assignments_in_lookup_values() {
    let dir = temp_dir("source-lookup-local-expr-array-element-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             expr items[2];\n\
             items[0] = value;\n\
             items[1] = value';\n\
             lookup_proves(7, [...items]);\n\
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
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_expands_function_expr_array_element_assignments_in_lookup_values() {
    let dir = temp_dir("source-lookup-function-expr-array-element-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function emit_lookup(expr value) {\n\
             expr items[2];\n\
             items[0] = value;\n\
             items[1] = value';\n\
             lookup_proves(7, [...items]);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             emit_lookup(value);\n\
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
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_expands_returned_expr_array_assignments_in_lookup_values() {
    let dir = temp_dir("source-lookup-returned-expr-array-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function pair(expr input): expr[] {\n\
             expr first = input;\n\
             expr second = input';\n\
             expr items[2];\n\
             items[0] = first;\n\
             items[1] = second;\n\
             return items;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             const expr items[] = pair(value);\n\
             lookup_proves(7, [...items]);\n\
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
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_expands_returned_expr_array_loop_assignments_in_lookup_values() {
    let dir = temp_dir("source-lookup-returned-expr-array-loop-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function copy_items(const expr input[]): expr[] {\n\
             const int len = length(input);\n\
             expr items[len];\n\
             for (int i = 0; i < len; i++) {\n\
                 items[i] = input[i];\n\
             }\n\
             return items;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             const expr items[] = copy_items([value, value']);\n\
             lookup_proves(7, [...items]);\n\
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
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

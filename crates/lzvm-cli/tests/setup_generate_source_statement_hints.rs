use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{read_expression_info_binary_file, HintPayload};
use lzvm_artifacts::hint_program::{HintOperand, SOURCE_UNSUPPORTED_STATEMENT_HINT};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-statement-hints-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_records_unsupported_source_control_statements_as_regular_hints() {
    let dir = temp_dir("unsupported-source-statement");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             for (int index = 0; index < value; ++index) {\n\
                 value * (1 - value) === 0;\n\
             }\n\
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
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_UNSUPPORTED_STATEMENT_HINT);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(
        regular.hints.hints[0].name,
        SOURCE_UNSUPPORTED_STATEMENT_HINT
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_ignores_source_include_and_require_blocks_as_regular_hints() {
    let dir = temp_dir("source-include-require-block");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let include_path = dir.join("source").join("extra.pil");
    write_file(&include_path, "");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             include \"extra.pil\"\n\
             require \"extra.pil\"\n\
             col witness value;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_ignores_private_source_require_blocks_as_regular_hints() {
    let dir = temp_dir("private-source-require-block");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let include_path = dir.join("source").join("extra.pil");
    write_file(&include_path, "");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             private require \"extra.pil\"\n\
             col witness value;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_ignores_source_use_directives_as_regular_hints() {
    let dir = temp_dir("source-use-directive");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             use proof.main;\n\
             col witness value;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_annotation_objects_as_regular_hints() {
    let dir = temp_dir("source-annotation-object");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness other;\n\
             col witness stage(2) mid;\n\
             col witness stage(3) ext;\n\
             expr total = value + other;\n\
             @record {target: value, expression: total + 2, pair: [value, other], literal: 7}\n\
             @record {value}\n\
             @record {extension: ext + 2}\n\
             @record 11;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 4);
    assert_eq!(expressions.hints[0].name, "record");
    assert_eq!(expressions.hints[0].fields.len(), 4);
    assert_eq!(expressions.hints[0].fields[0].name, "target");
    assert!(matches!(
        expressions.hints[0].fields[0].values[0].payload,
        HintPayload::Commitment { .. }
    ));
    assert_eq!(expressions.hints[0].fields[1].name, "expression");
    assert_eq!(
        expressions.hints[0].fields[1]
            .values
            .last()
            .expect("expression field should contain an operator")
            .payload,
        HintPayload::String {
            value: "add".to_owned()
        }
    );
    assert_eq!(expressions.hints[0].fields[2].name, "pair");
    assert_eq!(expressions.hints[0].fields[2].values.len(), 2);
    assert_eq!(
        expressions.hints[0].fields[3].values[0].payload,
        HintPayload::Number { value: 7 }
    );
    assert_eq!(expressions.hints[1].fields[0].name, "value");
    assert!(matches!(
        expressions.hints[1].fields[0].values[0].payload,
        HintPayload::Commitment { .. }
    ));
    assert_eq!(expressions.hints[2].fields[0].name, "extension");
    assert!(matches!(
        expressions.hints[2].fields[0].values[0].payload,
        HintPayload::Commitment { .. }
    ));
    assert_eq!(expressions.hints[3].fields[0].name, "value");
    assert_eq!(
        expressions.hints[3].fields[0].values[0].payload,
        HintPayload::Number { value: 11 }
    );

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 4);
    assert_eq!(regular.hints.hints[0].name, "record");
    assert!(matches!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Commitment { .. }
    ));
    assert_eq!(
        regular.hints.hints[0].fields[3].values[0].operand,
        HintOperand::Number(7)
    );
    assert_eq!(
        regular.hints.hints[3].fields[0].values[0].operand,
        HintOperand::Number(11)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_annotation_hints_without_stage_one_commitments() {
    let dir = temp_dir("annotation-without-stage-one");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness stage(2) mid;\n\
             col witness stage(3) ext;\n\
             @record {extension: ext + mid}\n\
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
    assert_eq!(setup.n_stages, 2);
    assert_eq!(setup.section_widths.get("cm1"), Some(&1));
    assert_eq!(setup.section_widths.get("cm2"), Some(&1));
    assert_eq!(setup.section_widths.get("cm3"), Some(&1));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, "record");
    assert_eq!(regular.hints.hints[0].fields[0].name, "extension");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

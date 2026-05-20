use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{read_expression_info_binary_file, HintPayload};
use lzvm_artifacts::hint_program::{
    HintOperand, SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT,
};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-lookup-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_records_source_lookup_calls_as_structured_regular_hints() {
    let dir = temp_dir("source-lookup-hints");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness multiplicity;\n\
             col witness value;\n\
             lookup_proves(7, [value, value', main.left], mul: multiplicity);\n\
             lookup_assumes(9, [value, value', main.left], sel: multiplicity);\n\
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
    let expression_path = unit
        .expression_info_binary()
        .expect("expression metadata path should derive");
    let expressions = read_expression_info_binary_file(expression_path)
        .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 2);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 3);
    assert_eq!(expressions.hints[0].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[0].fields[1].name, "values");
    assert_eq!(expressions.hints[0].fields[2].name, "multiplicity");
    assert_eq!(expressions.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[1].fields.len(), 3);
    assert_eq!(expressions.hints[1].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[1].fields[1].name, "values");
    assert_eq!(expressions.hints[1].fields[2].name, "selector");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(7)
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 1
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[2].operand,
        HintOperand::Constant {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(
        regular.hints.hints[1].fields[0].values[0].operand,
        HintOperand::Number(9)
    );
    assert_eq!(
        regular.hints.hints[1].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[1].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 1
        }
    );
    assert_eq!(
        regular.hints.hints[1].fields[1].values[2].operand,
        HintOperand::Constant {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[1].fields[2].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_resolves_static_source_lookup_bus_ids() {
    let dir = temp_dir("source-lookup-static-bus-ids");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int BUS_BASE = 7;\n\
         airtemplate UnitA(const int BUS_OFFSET = 4) {\n\
             col witness multiplicity;\n\
             col witness value;\n\
             const int LOCAL_BUS = BUS_BASE + BUS_OFFSET;\n\
             lookup_proves(LOCAL_BUS, [value], mul: multiplicity);\n\
             lookup_assumes(BUS_OFFSET + 6, [value], sel: multiplicity);\n\
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
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(11)
    );
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].fields[0].name, "bus_id");
    assert_eq!(
        regular.hints.hints[1].fields[0].values[0].operand,
        HintOperand::Number(10)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_named_source_lookup_arguments() {
    let dir = temp_dir("source-lookup-named-arguments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness mul;\n\
             col witness sel;\n\
             col witness value;\n\
             const int table_id = 5;\n\
             lookup_proves(7, expressions: [value], table_id:, mul:);\n\
             lookup_assumes(9, expressions: [value], sel:);\n\
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
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 4);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[2].name, "table_id");
    assert_eq!(regular.hints.hints[0].fields[3].name, "multiplicity");
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 2,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(5)
    );
    assert_eq!(
        regular.hints.hints[0].fields[3].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].fields.len(), 3);
    assert_eq!(regular.hints.hints[1].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[1].fields[1].name, "values");
    assert_eq!(regular.hints.hints[1].fields[2].name, "selector");
    assert_eq!(
        regular.hints.hints[1].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 2,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[1].fields[2].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_lookup_static_label_argument() {
    let dir = temp_dir("source-lookup-static-label");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness multiplicity;\n\
             col witness value;\n\
             const int LOOKUP_LABEL = 11;\n\
             lookup_proves(7, expressions: [value], table_id: 3, mul: multiplicity, surname: LOOKUP_LABEL);\n\
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
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 5);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[2].name, "table_id");
    assert_eq!(regular.hints.hints[0].fields[3].name, "multiplicity");
    assert_eq!(regular.hints.hints[0].fields[4].name, "surname");
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(3)
    );
    assert_eq!(
        regular.hints.hints[0].fields[3].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[4].values[0].operand,
        HintOperand::Number(11)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_bus_call_aliases() {
    let dir = temp_dir("source-bus-call-aliases");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness multiplicity;\n\
             col witness selector;\n\
             col witness value;\n\
             const int LABEL = 17;\n\
             permutation_proves(7, expressions: [value], sel: selector, name: LABEL);\n\
             direct_update_assumes(8, [value], sel: selector, surname: LABEL);\n\
             direct_global_update_proves(9, [value], surname: LABEL);\n\
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
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 3);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[2].name, "selector");
    assert_eq!(regular.hints.hints[0].fields[3].name, "name");
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(7)
    );
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[3].values[0].operand,
        HintOperand::Number(17)
    );

    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[1].fields[2].name, "selector");
    assert_eq!(regular.hints.hints[1].fields[3].name, "surname");
    assert_eq!(
        regular.hints.hints[1].fields[0].values[0].operand,
        HintOperand::Number(8)
    );
    assert_eq!(
        regular.hints.hints[1].fields[2].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[1].fields[3].values[0].operand,
        HintOperand::Number(17)
    );

    assert_eq!(regular.hints.hints[2].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[2].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[2].fields[2].name, "surname");
    assert_eq!(
        regular.hints.hints[2].fields[0].values[0].operand,
        HintOperand::Number(9)
    );
    assert_eq!(
        regular.hints.hints[2].fields[2].values[0].operand,
        HintOperand::Number(17)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_expands_source_lookup_spread_values() {
    let dir = temp_dir("source-lookup-spread-values");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness multiplicity;\n\
             col witness pair[2];\n\
             col witness value;\n\
             lookup_proves(7, [...pair, value], mul: multiplicity);\n\
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
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 3);
    assert_eq!(expressions.hints[0].fields[1].name, "values");
    assert_eq!(expressions.hints[0].fields[1].values.len(), 3);
    assert!(matches!(
        expressions.hints[0].fields[1].values[0].payload,
        HintPayload::CommitmentElement {
            id: 1,
            element: 0,
            row_offset_index: Some(0),
            row_offset: Some(0),
            dimension: Some(1),
            ..
        }
    ));
    assert!(matches!(
        expressions.hints[0].fields[1].values[1].payload,
        HintPayload::CommitmentElement {
            id: 1,
            element: 1,
            row_offset_index: Some(0),
            row_offset: Some(0),
            dimension: Some(1),
            ..
        }
    ));
    assert!(matches!(
        expressions.hints[0].fields[1].values[2].payload,
        HintPayload::Commitment {
            id: 2,
            row_offset_index: Some(0),
            row_offset: Some(0),
            dimension: Some(1),
            ..
        }
    ));

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 3);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::CommitmentElement {
            id: 1,
            element: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::CommitmentElement {
            id: 1,
            element: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[2].operand,
        HintOperand::Commitment {
            id: 2,
            row_offset_index: 0
        }
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_lookup_inside_scalar_helper_calls() {
    let dir = temp_dir("source-lookup-helper-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function emit_lookup(const expr op = 3, const expr mul = 2) {\n\
             lookup_proves(7, [op], mul:);\n\
         }\n\
         airtemplate UnitA() {\n\
             emit_lookup();\n\
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
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 3);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[2].name, "multiplicity");
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(7)
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Number(3)
    );
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(2)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_lookup_inside_expr_helper_calls() {
    let dir = temp_dir("source-lookup-expr-helper-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function emit_lookup(expr item, expr gate = 1) {\n\
             lookup_proves(7, [item], mul: gate);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness multiplicity;\n\
             col witness value;\n\
             emit_lookup(value, gate: multiplicity);\n\
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
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 3);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[2].name, "multiplicity");
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(
        regular.hints.hints[1].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[1].fields[2].values[0].operand,
        HintOperand::Number(1)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_collects_opening_points_inside_expr_helper_calls() {
    let dir = temp_dir("source-lookup-expr-helper-opening-points");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function emit_lookup(expr item) {\n\
             lookup_proves(7, [item']);\n\
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
fn generate_key_lowers_source_lookup_inside_expr_array_helper_calls() {
    let dir = temp_dir("source-lookup-expr-array-helper-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function emit_lookup(expr items[]) {\n\
             lookup_proves(7, [...items]);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness pair[2];\n\
             col witness value;\n\
             emit_lookup(pair);\n\
             emit_lookup([value, value']);\n\
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

    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 2);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::CommitmentElement {
            id: 0,
            element: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::CommitmentElement {
            id: 0,
            element: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[1].fields.len(), 2);
    assert_eq!(
        regular.hints.hints[1].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[1].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 1,
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
fn generate_key_expands_default_array_lookup_helper_values() {
    let dir = temp_dir("source-lookup-helper-array-defaults");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function emit_lookup(const expr pair[] = [3, 4], const expr extra[] = [5]) {\n\
             assert(length(extra) == 1);\n\
             lookup_proves(7, [...pair, ...extra], mul: 2);\n\
         }\n\
         airtemplate UnitA() {\n\
             emit_lookup();\n\
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
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 3);
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(
        regular.hints.hints[0].fields[1]
            .values
            .iter()
            .map(|value| &value.operand)
            .collect::<Vec<_>>(),
        vec![
            &HintOperand::Number(3),
            &HintOperand::Number(4),
            &HintOperand::Number(5),
        ]
    );
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(2)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_expands_array_argument_lookup_helper_values() {
    let dir = temp_dir("source-lookup-helper-array-arguments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function emit_lookup(const expr pair[] = [0, 0], const expr extra[] = [0]) {\n\
             assert(length(extra) == 1);\n\
             lookup_proves(7, [...pair, ...extra], mul: 2);\n\
         }\n\
         airtemplate UnitA() {\n\
             emit_lookup(pair: [3, 4], extra: [5]);\n\
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
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 3);
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(
        regular.hints.hints[0].fields[1]
            .values
            .iter()
            .map(|value| &value.operand)
            .collect::<Vec<_>>(),
        vec![
            &HintOperand::Number(3),
            &HintOperand::Number(4),
            &HintOperand::Number(5),
        ]
    );
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(2)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

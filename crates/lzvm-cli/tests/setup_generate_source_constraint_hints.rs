use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{
    read_expression_info_binary_file, CodeOperand, OperationKind,
};
use lzvm_artifacts::key_directory::{read_key_directory_layout, KeyUnitKind};
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;
use lzvm_field::Felt;
use lzvm_prover::regular_constraints::{
    evaluate_regular_constraints, RegularConstraintInputs, RegularStageColumns,
};

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-setup-generate-source-constraint-hints-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_records_unsupported_source_constraints_as_regular_hints() {
    let dir = temp_dir("unsupported-source-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value / (value + 1) === 0;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 0);
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, "source.constraint.unsupported");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 0);
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, "source.constraint.unsupported");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_nonzero_equality_constraints() {
    let dir = temp_dir("source-equality");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int N = 32) {\n\
             col witness value;\n\
             col witness next;\n\
             next === value + 1;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[1].op, OperationKind::Sub);
    assert!(matches!(
        operations[1].sources[0],
        CodeOperand::Commitment {
            id: 1,
            dimension: 1,
            ..
        }
    ));
    assert!(matches!(
        operations[1].sources[1],
        CodeOperand::Temporary {
            id: 0,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constraints_using_deferred_expr_alias_assignments() {
    let dir = temp_dir("source-deferred-expr-alias-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness left;\n\
             col witness right;\n\
             const expr diff;\n\
             diff = left + right;\n\
             selector * diff === 0;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.hints.len(), 0);
    assert_eq!(expressions.constraints.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constraints_using_scoped_deferred_expr_alias_assignments() {
    let dir = temp_dir("source-scoped-deferred-expr-alias-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness left;\n\
             col witness right;\n\
             const expr air.diff;\n\
             diff = left + right;\n\
             selector * diff === 0;\n\
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
    assert_eq!(expressions.hints.len(), 0);
    assert_eq!(expressions.constraints.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constraints_using_scoped_deferred_expr_array_assignments() {
    let dir = temp_dir("source-scoped-deferred-expr-array-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness left;\n\
             col witness right;\n\
             const expr air.values[2];\n\
             values[0] = left;\n\
             values[1] = right;\n\
             selector * (values[0] + values[1]) === 0;\n\
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
    assert_eq!(expressions.hints.len(), 0);
    assert_eq!(expressions.constraints.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constraints_using_indexed_row_offset_aliases() {
    let dir = temp_dir("source-indexed-row-offset-alias-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant RC = 2;\n\
         constant SEGMENT_L1 = 1;\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness a[RC];\n\
             col witness c[RC];\n\
             airval segment_previous_c[RC];\n\
             for (int index = 0; index < RC; ++index) {\n\
                 const expr previous_c = SEGMENT_L1 * (segment_previous_c[index] - 'c[index]) + 'c[index];\n\
                 selector * (a[index] - previous_c) === 0;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constraints_using_nested_column_array_indices() {
    let dir = temp_dir("source-nested-expr-array-index-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant RC = 2;\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness op[RC];\n\
             col witness low[RC][2];\n\
             col witness high[RC][2];\n\
             const expr value[RC][2];\n\
             const expr fill = selector + 1;\n\
             for (int i = 0; i < RC; ++i) {\n\
                 value[i][0] = low[i][0] + high[i][0] * 256;\n\
                 value[i][1] = low[i][1] + high[i][1] * 256;\n\
                 op[i] * selector * (value[i][0] - fill) === 0;\n\
                 op[i] * selector * (value[i][1] - fill) === 0;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 0);
    assert_eq!(expressions.constraints.len(), 4);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert!(regular.hints.hints.is_empty());
    assert_eq!(regular.constraints.entries.len(), 4);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_does_not_project_source_constraints_onto_recursive_metadata() {
    let dir = temp_dir("source-recursive-metadata-constraints");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value * (1 - value) === 0;\n\
         }\n\
         airtemplate UnitB() {\n\
             col witness other;\n\
             other === 0;\n\
         }\n\
         airgroup GroupA { UnitA(); UnitB(); }\n\
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
    let recursive = layout
        .units
        .iter()
        .find(|unit| unit.kind == KeyUnitKind::RecursiveSecond)
        .expect("recursive unit should be present");
    let expressions = read_expression_info_binary_file(
        recursive
            .expression_info_binary()
            .expect("recursive expression metadata path should derive"),
    )
    .expect("recursive expression metadata should parse");
    assert!(expressions.constraints.is_empty());
    assert!(expressions.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constrained_assignments_to_scoped_columns() {
    let dir = temp_dir("source-scoped-column-constrained-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness input;\n\
             col witness selector;\n\
             if (0) {\n\
                 col witness bits(1) air.sel_memset;\n\
                 col witness bits(8) air.fill_byte;\n\
             } else {\n\
                 const int air.sel_memset = 0;\n\
                 const int air.fill_byte = 0;\n\
             }\n\
             const expr loop_count = input + selector;\n\
             const expr loop_src = input + 1;\n\
             expr _loop_b0 = loop_src * selector + loop_count * (1 - selector + sel_memset);\n\
             expr _loop_extended_arg = loop_count * selector + fill_byte * sel_memset;\n\
             if (degree(_loop_b0) > 1) {\n\
                 col witness bits(32) air.loop_b0;\n\
                 loop_b0 <== _loop_b0;\n\
             } else {\n\
                 const expr air.loop_b0 = _loop_b0;\n\
             }\n\
             if (degree(_loop_extended_arg) > 1) {\n\
                 col witness bits(32) air.loop_extended_arg;\n\
                 loop_extended_arg <== _loop_extended_arg;\n\
             } else {\n\
                 const expr air.loop_extended_arg = _loop_extended_arg;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_executes_static_container_declarations() {
    let dir = temp_dir("source-static-container-declarations");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(int cont_subid = 0) {\n\
             container proof.unit_a {\n\
                 int cont_subids_count = 0;\n\
                 int cont_subids[4];\n\
             }\n\
             use proof.unit_a;\n\
             if (cont_subid == 0) {\n\
                 cont_subid = cont_subids_count + 1;\n\
                 for (int i = 0; i < cont_subids_count; ++i) {\n\
                     if (cont_subids[i] >= cont_subid) {\n\
                         cont_subid = cont_subids[i] + 1;\n\
                     }\n\
                 }\n\
             }\n\
             cont_subids[cont_subids_count] = cont_subid;\n\
             cont_subids_count += 1;\n\
             col witness value;\n\
             value === cont_subid;\n\
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_get_l1_constrained_assignments() {
    let dir = temp_dir("source-get-l1-constrained-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function get_L1(): expr {\n\
             if (!defined(air.__L1__)) {\n\
                 col fixed air.__L1__ = [1,0...];\n\
             }\n\
             return air.__L1__;\n\
         }\n\
         airtemplate UnitA() {\n\
             const expr L1 = get_L1();\n\
             airval segment_previous_seq_end;\n\
             col witness bits(1) seq_end;\n\
             col witness bits(1) previous_seq_end;\n\
             previous_seq_end <== L1 * (segment_previous_seq_end - 'seq_end) + 'seq_end;\n\
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
    assert!(setup
        .constant_columns
        .iter()
        .any(|column| column.name == "air.__L1__"));
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_degree_static_if_after_expr_alias_updates() {
    let dir = temp_dir("source-degree-after-expr-alias-updates");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             const int enable_memcpy = 1;\n\
             const int enable_inputcpy = 1;\n\
             const int enable_memset = 0;\n\
             const int op_x_row = 2;\n\
             const int bus_count = 1 + op_x_row + (enable_memcpy ? op_x_row : 0) + (enable_inputcpy ? 3 * op_x_row : 0);\n\
             const int max_bus_op_degree = (bus_count & 0x01) == 0 ? 2 : 1;\n\
             const int has_src = enable_memcpy;\n\
             const int b0_could_be_count = enable_inputcpy || enable_memset;\n\
             col witness src64;\n\
             col witness count64;\n\
             col witness sel_memcpy;\n\
             col witness sel_inputcpy;\n\
             col witness sel_memset;\n\
             expr _b0 = 0;\n\
             if (has_src && !b0_could_be_count) {\n\
                 _b0 = src64 * 8;\n\
             } else if (!has_src && b0_could_be_count) {\n\
                 _b0 = count64 * 8;\n\
             } else {\n\
                 _b0 = count64 * 8 * (sel_inputcpy + sel_memset) + src64 * 8 * sel_memcpy;\n\
             }\n\
             if (degree(_b0) > max_bus_op_degree) {\n\
                 col witness bits(32) air.b0;\n\
                 b0 <== _b0;\n\
             } else {\n\
                 const expr air.b0 = _b0;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_executes_uninitialized_const_int_array_assignments() {
    let dir = temp_dir("source-uninitialized-const-int-array-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int RC = 2) {\n\
             const int zeros[air.RC];\n\
             for (int index = 0; index < length(zeros); ++index) {\n\
                 zeros[index] = 0;\n\
             }\n\
             col witness value;\n\
             value === zeros[0] + zeros[1];\n\
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_for_over_column_array_length() {
    let dir = temp_dir("source-static-for-column-array-length");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int RC = 2) {\n\
             col witness air.value[air.RC];\n\
             for (int index = 0; index < length(value); index++) {\n\
                 value[index] === index;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_for_over_expr_array_length() {
    let dir = temp_dir("source-static-for-expr-array-length");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int RC = 2) {\n\
             col witness input;\n\
             const expr air.value[air.RC];\n\
             for (int index = 0; index < RC; index++) {\n\
                 value[index] = input + index;\n\
             }\n\
             for (int index = 0; index < length(value); index++) {\n\
                 value[index] === input + index;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_expr_array_element_plus_assignments() {
    let dir = temp_dir("source-expr-array-element-plus-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int RC = 2) {\n\
             const int CHUNKS_BY_RC = 2;\n\
             col witness reg[RC * CHUNKS_BY_RC];\n\
             expr accum[RC];\n\
             for (int rc_index = 0; rc_index < RC; rc_index++) {\n\
                 accum[rc_index] = 0;\n\
                 int base = 1;\n\
                 for (int offset = 0; offset < CHUNKS_BY_RC; offset++) {\n\
                     accum[rc_index] += reg[offset + rc_index * CHUNKS_BY_RC] * base;\n\
                     base *= 256;\n\
                 }\n\
                 accum[rc_index] === reg[rc_index * CHUNKS_BY_RC] + reg[rc_index * CHUNKS_BY_RC + 1] * 256;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_inline_returned_expr_function_constraints() {
    let dir = temp_dir("source-inline-returned-expr-function-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
         "airtemplate UnitA() {\n\
             const int ADDR_OP = 1;\n\
             const int ADDR_X1 = ADDR_OP + 1;\n\
             const int ADDR_Y3 = ADDR_X1 + 5;\n\
             const int ADDR_IND_0 = ADDR_Y3 + 1;\n\
             col witness CLK_0;\n\
             const expr CLK[2];\n\
             CLK[0] = CLK_0;\n\
             CLK[1] = CLK_0';\n\
             col witness sel_op[5];\n\
             col witness step_addr;\n\
             const expr sel_any = sel_op[0] + sel_op[1];\n\
             const expr sel_extra = sel_op[2] + sel_op[3] + sel_op[4];\n\
             (sel_any + sel_extra) * clock_eq(step_addr, ADDR_X1, ADDR_IND_0, 32) === 0;\n\
             function clock_eq(const expr mvcol, int pos1, int pos2, int pos2_offset = 0): const expr {\n\
                 return air.CLK[0] * (mvcol'(pos1) - mvcol'(pos2) - pos2_offset);\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_returned_expr_function_static_while_constraints() {
    let dir = temp_dir("source-returned-expr-function-static-while-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness active;\n\
             col witness values[2];\n\
             active * (sum_values(values) - values[0] - values[1]) === 0;\n\
             function sum_values(const expr items[]): const expr {\n\
                 expr total = 0;\n\
                 int index = 0;\n\
                 while (index < length(items)) {\n\
                     total += items[index];\n\
                     ++index;\n\
                 }\n\
                 return total;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [1, 1, 2, 4, 3, 5].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 3, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_returned_expr_function_static_do_while_constraints() {
    let dir = temp_dir("source-returned-expr-function-static-do-while-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness active;\n\
             col witness values[2];\n\
             active * (sum_values(values) - values[0] - values[1]) === 0;\n\
             function sum_values(const expr items[]): const expr {\n\
                 expr total = 0;\n\
                 int index = 0;\n\
                 do {\n\
                     total += items[index];\n\
                     ++index;\n\
                 } while (index < length(items));\n\
                 return total;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [1, 1, 2, 4, 3, 5].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 3, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results.iter().all(|result| result.invalid_rows.is_empty()));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_returned_expr_function_returning_from_static_do_while() {
    let dir = temp_dir("source-returned-expr-function-do-while-return");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness active;\n\
             col witness values[2];\n\
             active * (first_value(values) - values[0]) === 0;\n\
             function first_value(const expr items[]): const expr {\n\
                 int index = 0;\n\
                 do {\n\
                     return items[index];\n\
                 } while (index < length(items));\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [1, 1, 2, 4, 3, 5].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 3, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results.iter().all(|result| result.invalid_rows.is_empty()));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_collects_returned_expr_static_while_opening_points() {
    let dir = temp_dir("source-returned-expr-static-while-opening-points");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness active;\n\
             col witness values[2];\n\
             active * sum_next(values) === 0;\n\
             function sum_next(const expr items[]): const expr {\n\
                 expr total = 0;\n\
                 int index = 0;\n\
                 while (index < length(items)) {\n\
                     total += items[index]';\n\
                     ++index;\n\
                 }\n\
                 return total;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    assert_eq!(setup.opening_points, [0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [1, 0, 3, 0, 0, 0].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 3, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_collects_returned_expr_static_do_while_opening_points() {
    let dir = temp_dir("source-returned-expr-static-do-while-opening-points");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness active;\n\
             col witness values[2];\n\
             active * sum_next(values) === 0;\n\
             function sum_next(const expr items[]): const expr {\n\
                 expr total = 0;\n\
                 int index = 0;\n\
                 do {\n\
                     total += items[index]';\n\
                     ++index;\n\
                 } while (index < length(items));\n\
                 return total;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    assert_eq!(setup.opening_points, [0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [1, 0, 3, 0, 0, 0].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 3, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    let invalid_stage_values = [1, 3, 5, 0, 4, 6].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns::from_host_values(
        1,
        3,
        &invalid_stage_values,
    )];
    let invalid_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &invalid_stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate invalid input");
    assert_eq!(invalid_results[0].invalid_rows.len(), 1);
    assert_eq!(invalid_results[0].invalid_rows[0].row, 0);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_returned_expr_array_function_static_while_constraints() {
    let dir = temp_dir("source-returned-expr-array-function-static-while-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness active;\n\
             col witness values[2];\n\
             const expr copied[2] = copy_values(values);\n\
             active * (copied[0] - values[0]) === 0;\n\
             active * (copied[1] - values[1]) === 0;\n\
             function copy_values(const expr input[]): expr[] {\n\
                 expr result[2];\n\
                 int index = 0;\n\
                 while (index < 2) {\n\
                     result[index] = input[index];\n\
                     ++index;\n\
                 }\n\
                 return result;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [1, 1, 2, 4, 3, 5].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 3, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results.iter().all(|result| result.invalid_rows.is_empty()));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_returned_expr_array_function_static_do_while_constraints() {
    let dir = temp_dir("source-returned-expr-array-function-static-do-while-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness active;\n\
             col witness values[2];\n\
             const expr copied[2] = copy_values(values);\n\
             active * (copied[0] - values[0]) === 0;\n\
             active * (copied[1] - values[1]) === 0;\n\
             function copy_values(const expr input[]): expr[] {\n\
                 expr result[2];\n\
                 int index = 0;\n\
                 do {\n\
                     result[index] = input[index];\n\
                     ++index;\n\
                 } while (index < length(input));\n\
                 return result;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [1, 1, 2, 4, 3, 5].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 3, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results.iter().all(|result| result.invalid_rows.is_empty()));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_offsets_on_returned_expr_aliases() {
    let dir = temp_dir("source-returned-expr-row-offset-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant P2_4 = 16;\n\
         airtemplate UnitA() {\n\
             const int WRITE = 2;\n\
             const int CLOCKS = 4;\n\
             col witness flag;\n\
             col witness carry;\n\
             col witness value[2][4];\n\
             const expr packed = mix_and_pack(value);\n\
             packed + carry * P2_4\n\
                 - flag * ((WRITE)'packed + (CLOCKS - WRITE)'packed) === 0;\n\
             function mix_and_pack(const expr groups[][]): expr {\n\
                 expr [left, right] = [groups[0], groups[1]];\n\
                 return pack(left) + pack(right);\n\
             }\n\
             function pack(const expr items[]): expr {\n\
                 const int len = length(items);\n\
                 expr packed = 0;\n\
                 for (int j = 0; j < len; j++) {\n\
                     packed += items[j] * 2**j;\n\
                 }\n\
                 return packed;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_row_offsets_on_returned_expr_array_aliases() {
    let dir = temp_dir("source-returned-expr-array-row-offset-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant P2_4 = 16;\n\
         airtemplate UnitA() {\n\
             const int WRITE = 2;\n\
             const int CLOCKS = 4;\n\
             col witness flag;\n\
             col witness carry;\n\
             col witness a[4];\n\
             col witness e[4];\n\
             col witness w[4];\n\
             expr old_a[4][4];\n\
             expr old_e[4][4];\n\
             for (int i = 0; i < 4; i++) {\n\
                 old_a[0][i] = 'a[i];\n\
                 old_a[1][i] = 2'a[i];\n\
                 old_a[2][i] = 3'a[i];\n\
                 old_a[3][i] = 4'a[i];\n\
                 old_e[0][i] = 'e[i];\n\
                 old_e[1][i] = 2'e[i];\n\
                 old_e[2][i] = 3'e[i];\n\
                 old_e[3][i] = 4'e[i];\n\
             }\n\
             const expr new_ae[2] = compute_next(old_a, old_e, w);\n\
             const expr packed = new_ae[0];\n\
             packed + carry * P2_4\n\
                 - flag * ((WRITE)'packed + (CLOCKS - WRITE)'packed) === 0;\n\
             function compute_next(const expr old_a[][], const expr old_e[][], const expr w[]): expr[] {\n\
                 expr [a, b, c, d] = [old_a[0], old_a[1], old_a[2], old_a[3]];\n\
                 expr [e, f, g, h] = [old_e[0], old_e[1], old_e[2], old_e[3]];\n\
                 expr t1 = pack(h) + pack(w);\n\
                 expr t2 = pack(a);\n\
                 expr new_a = t1 + t2;\n\
                 expr new_e = pack(d) + t1;\n\
                 expr result[2] = [new_a, new_e];\n\
                 return result;\n\
             }\n\
             function pack(const expr items[]): expr {\n\
                 const int len = length(items);\n\
                 expr packed = 0;\n\
                 for (int j = 0; j < len; j++) {\n\
                     packed += items[j] * 2**j;\n\
                 }\n\
                 return packed;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_freezes_static_for_indices_in_expr_array_assignments() {
    let dir = temp_dir("source-static-for-expr-array-indices");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant RC = 2;\n\
         constant P2_8 = 256;\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness bytes[8];\n\
             col witness value;\n\
             const expr packed[RC];\n\
             for (int i = 0; i < RC; ++i) {\n\
                 packed[i] = bytes[i * 4] + P2_8 * bytes[i * 4 + 1];\n\
             }\n\
             for (int i = 0; i < 4; ++i) {\n\
                 selector * (1 - selector) === 0;\n\
             }\n\
             selector * (packed[0] - value) === 0;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 5);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 5);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_expr_array_initializers_from_const_expr_returns() {
    let dir = temp_dir("source-const-expr-returned-array-initializer");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             const int WIDTH = 2;\n\
             col witness bits[2];\n\
             const expr values[WIDTH] = build_values(copy_values(bits));\n\
             values[0] - bits[0] === 0;\n\
             function copy_values(const expr input[]): const expr[] {\n\
                 const expr result[air.WIDTH];\n\
                 result[0] = input[0];\n\
                 result[1] = input[1];\n\
                 return result;\n\
             }\n\
             function build_values(const expr input[]): const expr {\n\
                 const expr result[air.WIDTH];\n\
                 result[0] = input[0];\n\
                 result[1] = input[1];\n\
                 return result;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_returned_expr_arrays_with_static_if_elements_in_constraints() {
    let dir = temp_dir("source-returned-expr-array-static-if-elements-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant P2_32 = 4294967296;\n\
         airtemplate UnitA() {\n\
             const int CLOCKS_LOAD_STATE = 4;\n\
             const int CLOCKS_LOAD_INPUT = 16;\n\
             const int CLOCKS_MIXING = 48;\n\
             const int CLOCKS_WRITE_STATE = 4;\n\
             const int CLOCKS = CLOCKS_LOAD_STATE + CLOCKS_LOAD_INPUT + CLOCKS_MIXING + CLOCKS_WRITE_STATE;\n\
             col witness CLK_0;\n\
             expr CLK[CLOCKS];\n\
             for (int i = 0; i < CLOCKS; i++) {\n\
                 CLK[i] = (i)'CLK_0;\n\
             }\n\
             const expr is_loading_input = clock_set(start: 0, end: CLOCKS_LOAD_INPUT - 1, offset: CLOCKS_LOAD_STATE);\n\
             const expr is_mixing = clock_set(start: 0, end: CLOCKS_MIXING - 1, offset: CLOCKS_LOAD_STATE + CLOCKS_LOAD_INPUT);\n\
             col witness flag;\n\
             col witness bits[32];\n\
             col witness bits(4) carry;\n\
             col witness value;\n\
             expr old_bits[4][32];\n\
             for (int i = 0; i < 32; i++) {\n\
                 old_bits[0][i] = 2'bits[i];\n\
                 old_bits[1][i] = 7'bits[i];\n\
                 old_bits[2][i] = 15'bits[i];\n\
                 old_bits[3][i] = 16'bits[i];\n\
             }\n\
             const expr packed = pack(bits);\n\
             const expr next_packed = compute(old_bits);\n\
             packed + carry * P2_32\n\
                 - is_loading_input * packed\n\
                 - is_mixing * next_packed === 0;\n\
             function clock_set(const expr mvcol = 1, int start = 0, int end = -1, int offset = 0): const expr {\n\
                 if (end == -1) {\n\
                     end = start;\n\
                 }\n\
                 start += offset;\n\
                 end += offset;\n\
                 expr res = 0;\n\
                 for (int index = start; index <= end; index++) {\n\
                     res += air.CLK[index];\n\
                 }\n\
                 return res * mvcol;\n\
             }\n\
             function compute(const expr old_bits[][]): expr {\n\
                 expr [old_0, old_1, old_2, old_3] = [old_bits[0], old_bits[1], old_bits[2], old_bits[3]];\n\
                 expr s0 = xor(rotate(old_2, 1), xor(rotate(old_2, 2), shift(old_2, 1)));\n\
                 expr s1 = xor(rotate(old_0, 1), xor(rotate(old_0, 2), shift(old_0, 1)));\n\
                 expr new_value = pack(s1) + pack(old_1) + pack(s0) + pack(old_3);\n\
                 return new_value;\n\
             }\n\
             function xor(const expr left[], const expr right[]): expr[] {\n\
                 const int len = length(left);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     result[i] = left[i] + right[i] - 2 * left[i] * right[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function rotate(const expr state[], const int shift): expr[] {\n\
                 const int len = length(state);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     if (i + shift < len) {\n\
                         result[i] = state[i + shift];\n\
                     } else {\n\
                         result[i] = state[i + shift - len];\n\
                     }\n\
                 }\n\
                 return result;\n\
             }\n\
             function shift(const expr state[], const int shift): expr[] {\n\
                 const int len = length(state);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     if (i + shift < len) {\n\
                         result[i] = state[i + shift];\n\
                     } else {\n\
                         result[i] = 0;\n\
                     }\n\
                 }\n\
                 return result;\n\
             }\n\
             function pack(const expr items[]): expr {\n\
                 const int len = length(items);\n\
                 expr packed = 0;\n\
                 for (int j = 0; j < len; j++) {\n\
                     packed += items[j] * 2**j;\n\
                 }\n\
                 return packed;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_returned_expr_array_elements_with_destructured_array_inputs_in_constraints()
{
    let dir = temp_dir("source-returned-expr-array-destructured-inputs-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness a[32];\n\
             col witness e[32];\n\
             col witness w[32];\n\
             col witness expected;\n\
             expr old_a[4][32];\n\
             expr old_e[4][32];\n\
             for (int i = 0; i < 32; i++) {\n\
                 old_a[0][i] = 'a[i];\n\
                 old_a[1][i] = 2'a[i];\n\
                 old_a[2][i] = 3'a[i];\n\
                 old_a[3][i] = 4'a[i];\n\
                 old_e[0][i] = 'e[i];\n\
                 old_e[1][i] = 2'e[i];\n\
                 old_e[2][i] = 3'e[i];\n\
                 old_e[3][i] = 4'e[i];\n\
             }\n\
             const expr round_key = expected + 1;\n\
             const expr next_values[2] = compute(old_a, old_e, w, round_key);\n\
             const expr next_a_packed = next_values[0];\n\
             next_a_packed - expected === 0;\n\
             function compute(const expr old_a[][], const expr old_e[][], const expr w[], const expr k): expr[] {\n\
                 expr [a, b, c, d] = [old_a[0], old_a[1], old_a[2], old_a[3]];\n\
                 expr [e, f, g, h] = [old_e[0], old_e[1], old_e[2], old_e[3]];\n\
                 expr s0 = xor(rotate(a, 2), xor(rotate(a, 13), rotate(a, 22)));\n\
                 expr s1 = xor(rotate(e, 6), xor(rotate(e, 11), rotate(e, 25)));\n\
                 expr t1 = pack(h) + pack(s1) + pack(ch(e, f, g)) + k + pack(w);\n\
                 expr t2 = pack(s0) + pack(maj(a, b, c));\n\
                 expr next_a = t1 + t2;\n\
                 expr next_e = pack(d) + t1;\n\
                 expr result[2] = [next_a, next_e];\n\
                 return result;\n\
             }\n\
             function xor(const expr left[], const expr right[]): expr[] {\n\
                 const int len = length(left);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     result[i] = left[i] + right[i] - 2 * left[i] * right[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function rotate(const expr state[], const int shift): expr[] {\n\
                 const int len = length(state);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     if (i + shift < len) {\n\
                         result[i] = state[i + shift];\n\
                     } else {\n\
                         result[i] = state[i + shift - len];\n\
                     }\n\
                 }\n\
                 return result;\n\
             }\n\
             function maj(const expr a[], const expr b[], const expr c[]): expr[] {\n\
                 const int len = length(a);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     result[i] = a[i] * b[i] * (1 - c[i]) +\n\
                         a[i] * (1 - b[i]) * c[i] +\n\
                         (1 - a[i]) * b[i] * c[i] +\n\
                         a[i] * b[i] * c[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function ch(const expr a[], const expr b[], const expr c[]): expr[] {\n\
                 const int len = length(a);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     result[i] = a[i] * b[i] + (1 - a[i]) * c[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function pack(const expr items[]): expr {\n\
                 const int len = length(items);\n\
                 expr packed = 0;\n\
                 for (int j = 0; j < len; j++) {\n\
                     packed += items[j] * 2**j;\n\
                 }\n\
                 return packed;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_packed_returned_expr_array_constraints_with_row_offsets() {
    let dir = temp_dir("source-packed-returned-expr-array-row-offset-constraints");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant WIDTH = 4;\n\
         constant P2_WIDTH = 16;\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness is_loading_state;\n\
             col witness is_loading_input;\n\
             col witness is_mixing;\n\
             col witness is_writing_state;\n\
             col witness new_a_carry_bits;\n\
             col witness new_e_carry_bits;\n\
             col witness new_w_carry_bits;\n\
             col witness a[WIDTH];\n\
             col witness e[WIDTH];\n\
             col witness w[WIDTH];\n\
             expr old_a[4][WIDTH];\n\
             expr old_e[4][WIDTH];\n\
             expr old_w[4][WIDTH];\n\
             for (int i = 0; i < WIDTH; i++) {\n\
                 old_a[0][i] = 'a[i];\n\
                 old_a[1][i] = 2'a[i];\n\
                 old_a[2][i] = 3'a[i];\n\
                 old_a[3][i] = 4'a[i];\n\
                 old_e[0][i] = 'e[i];\n\
                 old_e[1][i] = 2'e[i];\n\
                 old_e[2][i] = 3'e[i];\n\
                 old_e[3][i] = 4'e[i];\n\
                 old_w[0][i] = 2'w[i];\n\
                 old_w[1][i] = 3'w[i];\n\
                 old_w[2][i] = 4'w[i];\n\
                 old_w[3][i] = 5'w[i];\n\
             }\n\
             const expr k = selector + 1;\n\
             const expr new_ae[2] = compute_ae(old_a, old_e, w, k);\n\
             const expr new_a_packed = new_ae[0];\n\
             const expr new_e_packed = new_ae[1];\n\
             const expr new_w_packed = compute_w(old_w);\n\
             const expr a_packed = pack(a);\n\
             const expr e_packed = pack(e);\n\
             const expr w_packed = pack(w);\n\
             a_packed + new_a_carry_bits * P2_WIDTH\n\
                 - is_loading_state * a_packed\n\
                 - (is_loading_input + is_mixing) * new_a_packed\n\
                 - is_writing_state * (2'a_packed + 3'a_packed) === 0;\n\
             e_packed + new_e_carry_bits * P2_WIDTH\n\
                 - is_loading_state * e_packed\n\
                 - (is_loading_input + is_mixing) * new_e_packed\n\
                 - is_writing_state * (2'e_packed + 3'e_packed) === 0;\n\
             w_packed + new_w_carry_bits * P2_WIDTH\n\
                 - is_loading_input * w_packed\n\
                 - is_mixing * new_w_packed === 0;\n\
             function compute_w(const expr old_w[][]): expr {\n\
                 expr [old_w2, old_w7, old_w15, old_w16] = [old_w[0], old_w[1], old_w[2], old_w[3]];\n\
                 expr s0 = xor(rotate(old_w15, 1), rotate(old_w15, 2));\n\
                 expr s1 = xor(rotate(old_w2, 1), rotate(old_w2, 3));\n\
                 expr new_w = pack(s1) + pack(old_w7) + pack(s0) + pack(old_w16);\n\
                 return new_w;\n\
             }\n\
             function compute_ae(const expr old_a[][], const expr old_e[][], const expr w[], const expr k): expr[] {\n\
                 expr [a, b, c, d] = [old_a[0], old_a[1], old_a[2], old_a[3]];\n\
                 expr [e, f, g, h] = [old_e[0], old_e[1], old_e[2], old_e[3]];\n\
                 expr s0 = xor(rotate(a, 1), xor(rotate(a, 2), rotate(a, 3)));\n\
                 expr s1 = xor(rotate(e, 1), xor(rotate(e, 2), rotate(e, 3)));\n\
                 expr t1 = pack(h) + pack(s1) + pack(ch(e, f, g)) + k + pack(w);\n\
                 expr t2 = pack(s0) + pack(maj(a, b, c));\n\
                 expr new_a = t1 + t2;\n\
                 expr new_e = pack(d) + t1;\n\
                 expr result[2] = [new_a, new_e];\n\
                 return result;\n\
             }\n\
             function xor(const expr left[], const expr right[]): expr[] {\n\
                 const int len = length(left);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     result[i] = left[i] + right[i] - 2 * left[i] * right[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function rotate(const expr state[], const int shift): expr[] {\n\
                 const int len = length(state);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     if (i + shift < len) {\n\
                         result[i] = state[i + shift];\n\
                     } else {\n\
                         result[i] = state[i + shift - len];\n\
                     }\n\
                 }\n\
                 return result;\n\
             }\n\
             function maj(const expr a[], const expr b[], const expr c[]): expr[] {\n\
                 const int len = length(a);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     result[i] = a[i] * b[i] * (1 - c[i]) +\n\
                         a[i] * (1 - b[i]) * c[i] +\n\
                         (1 - a[i]) * b[i] * c[i] +\n\
                         a[i] * b[i] * c[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function ch(const expr a[], const expr b[], const expr c[]): expr[] {\n\
                 const int len = length(a);\n\
                 expr result[len];\n\
                 for (int i = 0; i < len; i++) {\n\
                     result[i] = a[i] * b[i] + (1 - a[i]) * c[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function pack(const expr items[]): expr {\n\
                 const int len = length(items);\n\
                 expr packed = 0;\n\
                 for (int j = 0; j < len; j++) {\n\
                     packed += items[j] * 2**j;\n\
                 }\n\
                 return packed;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(layout.units[0].kind, KeyUnitKind::Basic);
    let unit = &layout.units[0];
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 0);
    assert_eq!(expressions.constraints.len(), 3);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 3);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_returned_array_constraints_with_call_scoped_aliases() {
    let dir = temp_dir("source-returned-array-constraint-call-scoped-aliases");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness left[2];\n\
             col witness right[2];\n\
             col witness sink_left;\n\
             col witness sink_right;\n\
             check(left, right, sink_left, sink_right);\n\
             function check(const expr xs[], const expr ys[], const expr left_sink, const expr right_sink) {\n\
                 const expr projected_left = pick(xs);\n\
                 const expr projected_right = pick(ys);\n\
                 left_sink - projected_left === 0;\n\
                 right_sink - projected_right === 0;\n\
             }\n\
             function pick(const expr input[]): expr {\n\
                 return pair(input)[0];\n\
             }\n\
             function pair(const expr input[]): expr[] {\n\
                 expr result[1] = [input[0] + 2 * input[1]];\n\
                 return result;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 2);
    assert!(
        expressions.constraints[1]
            .operations
            .iter()
            .any(|operation| {
                operation.sources.iter().any(|source| {
                    matches!(
                        source,
                        CodeOperand::CommitmentElement {
                            id: 1,
                            element: 0,
                            ..
                        }
                    )
                })
            }),
        "second constraint should read right[0], operations={:?}",
        expressions.constraints[1].operations
    );
    assert!(
        expressions.constraints[1]
            .operations
            .iter()
            .any(|operation| {
                operation.sources.iter().any(|source| {
                    matches!(
                        source,
                        CodeOperand::CommitmentElement {
                            id: 1,
                            element: 1,
                            ..
                        }
                    )
                })
            }),
        "second constraint should read right[1], operations={:?}",
        expressions.constraints[1].operations
    );
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_top_level_returned_array_constraints() {
    let dir = temp_dir("source-top-level-returned-array-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[2];\n\
             col witness sink;\n\
             const expr projected = pick(values);\n\
             sink - projected === 0;\n\
             function pick(const expr input[]): expr {\n\
                 return pair(input)[0];\n\
             }\n\
             function pair(const expr input[]): expr[] {\n\
                 expr result[1] = [input[0] + 2 * input[1]];\n\
                 return result;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(
        expressions.constraints[0]
            .operations
            .iter()
            .any(|operation| {
                operation.sources.iter().any(|source| {
                    matches!(
                        source,
                        CodeOperand::CommitmentElement {
                            id: 0,
                            element: 0,
                            ..
                        }
                    )
                })
            }),
        "constraint should read values[0], operations={:?}",
        expressions.constraints[0].operations
    );
    assert!(
        expressions.constraints[0]
            .operations
            .iter()
            .any(|operation| {
                operation.sources.iter().any(|source| {
                    matches!(
                        source,
                        CodeOperand::CommitmentElement {
                            id: 0,
                            element: 1,
                            ..
                        }
                    )
                })
            }),
        "constraint should read values[1], operations={:?}",
        expressions.constraints[0].operations
    );
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constraints_using_multiple_deferred_expr_alias_assignments() {
    let dir = temp_dir("source-multiple-deferred-expr-alias-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness flag;\n\
             col witness left;\n\
             col witness right;\n\
             const expr flag_alias;\n\
             const expr diff;\n\
             flag_alias = flag;\n\
             diff = left + right;\n\
             (1 - flag_alias) * diff === 0;\n\
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
    assert_eq!(expressions.hints.len(), 0);
    assert_eq!(expressions.constraints.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constraints_using_composite_deferred_expr_alias_assignments() {
    let dir = temp_dir("source-composite-deferred-expr-alias-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int SCALE = 16;\n\
         airtemplate UnitA() {\n\
             col witness a;\n\
             col witness b;\n\
             col witness c;\n\
             col witness d;\n\
             const expr flag_alias;\n\
             const expr diff;\n\
             flag_alias = a + b;\n\
             diff = c + d * SCALE;\n\
             (1 - flag_alias) * diff === 0;\n\
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
    assert_eq!(expressions.hints.len(), 0);
    assert_eq!(expressions.constraints.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constraints_using_deferred_expr_aliases_assigned_in_static_if() {
    let dir = temp_dir("source-static-if-deferred-expr-alias-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int ENABLED = 1;\n\
         const int SCALE = 16;\n\
         airtemplate UnitA() {\n\
             col witness a;\n\
             col witness b;\n\
             col witness c;\n\
             col witness d;\n\
             const expr flag_alias;\n\
             const expr diff;\n\
             if (ENABLED) {\n\
                 flag_alias = a + b;\n\
                 diff = c + d * SCALE;\n\
             } else {\n\
                 flag_alias = 0;\n\
                 diff = 0;\n\
             }\n\
             (1 - flag_alias) * diff === 0;\n\
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
    assert_eq!(expressions.hints.len(), 0);
    assert_eq!(expressions.constraints.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_air_scoped_witness_boolean_constraints() {
    let dir = temp_dir("source-air-scoped-witness-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness bits(1) air.flag;\n\
             expr _sel = 0;\n\
             _sel += flag;\n\
             const expr sel = _sel;\n\
             sel * (1 - sel) === 0;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    assert!(expressions.constraints[0]
        .operations
        .iter()
        .any(|operation| {
            operation.sources.iter().any(|source| {
                matches!(
                    source,
                    CodeOperand::Commitment {
                        id: 0,
                        dimension: 1,
                        ..
                    }
                )
            })
        }));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    let stage_values = [0, 1].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results.iter().all(|result| result.invalid_rows.is_empty()));

    let invalid_stage_values = [2, 1].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns::from_host_values(
        1,
        1,
        &invalid_stage_values,
    )];
    let invalid_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &invalid_stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate invalid input");
    assert_eq!(invalid_results[0].invalid_rows.len(), 1);
    assert_eq!(invalid_results[0].invalid_rows[0].row, 0);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_backslash_division_constraints() {
    let dir = temp_dir("source-static-backslash-division");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
              col witness value;\n\
              col witness half;\n\
             const expr divisor = 2;\n\
             half === value \\ divisor;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].op, OperationKind::Mul);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            dimension: 1,
            ..
        }
    ));
    let half_inverse = Felt::from_u64(2)
        .inverse()
        .expect("nonzero divisor should invert")
        .to_u64();
    assert!(matches!(
        operations[0].sources[1],
        CodeOperand::Number {
            value,
            dimension: 1,
        } if value == half_inverse
    ));
    assert_eq!(operations[1].op, OperationKind::Sub);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [4, 2, 8, 4].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    let invalid_stage_values = [4, 2, 8, 5].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns::from_host_values(
        1,
        2,
        &invalid_stage_values,
    )];
    let invalid_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &invalid_stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate invalid input");
    assert_eq!(invalid_results[0].invalid_rows.len(), 1);
    assert_eq!(invalid_results[0].invalid_rows[0].row, 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_alias_exponent_constraints() {
    let dir = temp_dir("source-static-alias-exponent");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness cube;\n\
             const expr exponent = 3;\n\
             cube === value ** exponent;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [2, 8, 3, 27].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    let invalid_stage_values = [2, 8, 3, 28].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns::from_host_values(
        1,
        2,
        &invalid_stage_values,
    )];
    let invalid_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &invalid_stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate invalid input");
    assert_eq!(invalid_results[0].invalid_rows.len(), 1);
    assert_eq!(invalid_results[0].invalid_rows[0].row, 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_array_element_indexes_in_constraints() {
    let dir = temp_dir("source-static-array-index-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int selected[] = [1];\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[selected[0]] === 7;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 1,
            prime: None,
            dimension: 1,
        }
    ));
    assert!(matches!(
        operations[0].sources[1],
        CodeOperand::Number {
            value: 7,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [3, 7, 4, 7].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    let invalid_stage_values = [3, 7, 4, 8].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns::from_host_values(
        1,
        2,
        &invalid_stage_values,
    )];
    let invalid_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &invalid_stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate invalid input");
    assert_eq!(invalid_results[0].invalid_rows.len(), 1);
    assert_eq!(invalid_results[0].invalid_rows[0].row, 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_array_row_assignments_in_constraints() {
    let dir = temp_dir("source-static-array-row-assignment-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             const int TABLE[2][3];\n\
             TABLE[0] = [1, 2, 3];\n\
             TABLE[1] = [4, 5, 6];\n\
             col witness value;\n\
             value === TABLE[1][2];\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert!(matches!(
        operations[0].sources[1],
        CodeOperand::Number {
            value: 6,
            dimension: 1
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constrained_assignments_from_static_array_row_accumulators() {
    let dir = temp_dir("source-static-array-row-accumulator-constrained-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             const int TABLE[2][3];\n\
             TABLE[0] = [1, 2, 3];\n\
             TABLE[1] = [4, 5, 6];\n\
             col witness selector[2];\n\
             col witness value;\n\
             expr value_expr = 0;\n\
             for (int row = 0; row < 2; row++) {\n\
                 expr row_sum = 0;\n\
                 for (int idx = 0; idx < 3; idx++) {\n\
                     row_sum += TABLE[row][idx];\n\
                 }\n\
                 value_expr += selector[row] * row_sum;\n\
             }\n\
             value <== value_expr;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(
        expressions.constraints[0]
            .operations
            .iter()
            .any(|operation| {
                operation.sources.iter().any(|source| {
                    matches!(
                        source,
                        CodeOperand::CommitmentElement {
                            id: 0,
                            element: 0,
                            ..
                        }
                    )
                })
            }),
        "constraint should read selector[0], operations={:?}",
        expressions.constraints[0].operations
    );
    assert!(
        expressions.constraints[0]
            .operations
            .iter()
            .any(|operation| {
                operation.sources.iter().any(|source| {
                    matches!(
                        source,
                        CodeOperand::CommitmentElement {
                            id: 0,
                            element: 1,
                            ..
                        }
                    )
                })
            }),
        "constraint should read selector[1], operations={:?}",
        expressions.constraints[0].operations
    );
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_constrained_assignments_from_static_row_offsets() {
    let dir = temp_dir("source-static-row-offset-accumulator-constrained-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int N = 32) {\n\
             const int CLOCKS_PER_G = 3;\n\
             const int NUM_G_FUNCTIONS = 8;\n\
             const int CLOCKS = CLOCKS_PER_G * NUM_G_FUNCTIONS;\n\
             const int SIGMA[10][16];\n\
             SIGMA[0] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];\n\
             SIGMA[1] = [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3];\n\
             SIGMA[2] = [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4];\n\
             SIGMA[3] = [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8];\n\
             SIGMA[4] = [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13];\n\
             SIGMA[5] = [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9];\n\
             SIGMA[6] = [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11];\n\
             SIGMA[7] = [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10];\n\
             SIGMA[8] = [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5];\n\
             SIGMA[9] = [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0];\n\
             col fixed CLK_0 = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]...;\n\
             const expr CLK[CLOCKS];\n\
             for (int i = 0; i < CLOCKS; i++) {\n\
                 CLK[i] = (i)'CLK_0;\n\
             }\n\
             col witness bits(1) round_idx_sel[10];\n\
             col witness bits(4) sigma_idx;\n\
             expr sigma_idx_expr = 0;\n\
             for (int j = 0; j < 10; j++) {\n\
                 expr sigma_sum = 0;\n\
                 int msg_pos = 0;\n\
                 for (int k = 0; k < CLOCKS; k++) {\n\
                     if (k > 0 && k % CLOCKS_PER_G == (CLOCKS_PER_G - 1)) {\n\
                         continue;\n\
                     }\n\
                     sigma_sum += CLK[k] * SIGMA[j][msg_pos];\n\
                     msg_pos++;\n\
                 }\n\
                 sigma_idx_expr += round_idx_sel[j] * sigma_sum;\n\
             }\n\
             sigma_idx <== sigma_idx_expr;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert!(
        expressions
            .hints
            .iter()
            .all(|hint| hint.name != "source.assignment.unsupported"),
        "unsupported assignment hints: {:?}",
        expressions
            .hints
            .iter()
            .filter(|hint| hint.name == "source.assignment.unsupported")
            .collect::<Vec<_>>()
    );
    assert_eq!(expressions.constraints.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_statement_calls_to_constraint_functions_with_return_types() {
    let dir = temp_dir("source-return-typed-constraint-function-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant P2_32 = 4294967296;\n\
         constant P2_64 = 18446744073709551616;\n\
         airtemplate UnitA() {\n\
             col witness active;\n\
             col witness a[2];\n\
             col witness b[2];\n\
             col witness d[2];\n\
             col witness bits[2][4];\n\
             const expr packed[2] = [pack_bits(bits[0]), pack_bits(bits[1])];\n\
             add3_check(a, b, packed, d);\n\
             function pack_bits(const expr items[]): expr {\n\
                 const int len = length(items);\n\
                 expr packed = 0;\n\
                 for (int i = 0; i < len; i++) {\n\
                     packed += items[i] * 2**i;\n\
                 }\n\
                 return packed;\n\
             }\n\
             function add3_check(const expr out[], const expr left[], const expr right[], const expr extra[]): expr[] {\n\
                 assert(length(out) == 2 && length(left) == 2 && length(right) == 2 && length(extra) == 2);\n\
                 expr sum_0 = out[0] - left[0] - right[0] - extra[0];\n\
                 expr sum_1 = out[1] - left[1] - right[1] - extra[1];\n\
                 expr sum = sum_0 + P2_32 * sum_1;\n\
                 active * (sum_0 * (sum_0 + P2_32) * (sum_0 + 2 * P2_32)) === 0;\n\
                 active * (sum * (sum + P2_64) * (sum + 2 * P2_64)) === 0;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1, 0, 0, 0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 2);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_array_element_arithmetic_indexes_in_constraints() {
    let dir = temp_dir("source-static-array-arithmetic-index-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int selected[] = [0];\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[selected[0] + 1] === 7;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 1,
            prime: None,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [3, 7, 4, 7].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_array_derived_static_function_indexes_in_constraints() {
    let dir = temp_dir("source-static-function-array-index-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function selected_index(): int {\n\
             const int selected[] = [0, 1];\n\
             return selected[0] + selected[1];\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[selected_index()] === 7;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 1,
            prime: None,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [3, 7, 4, 7].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_function_conditional_indexes_in_constraints() {
    let dir = temp_dir("source-static-function-conditional-index-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function selected_index(): int {\n\
             int flag = 0;\n\
             if (flag) {\n\
                 return 0;\n\
             } else {\n\
                 return 1;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[selected_index()] === 7;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 1,
            prime: None,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [3, 7, 4, 7].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_function_for_loop_indexes_in_constraints() {
    let dir = temp_dir("source-static-function-loop-index-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function selected_index(): int {\n\
             int total = 0;\n\
             for (int index = 0; index < 2; ++index) {\n\
                 total += index;\n\
             }\n\
             return total;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[selected_index()] === 7;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 1,
            prime: None,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [3, 7, 4, 7].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_parameterized_static_function_indexes_in_constraints() {
    let dir = temp_dir("source-static-function-parameter-index-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function selected_index(const int base, int delta = 1): int {\n\
             return base + delta;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[selected_index(0)] === 7;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 1,
            prime: None,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [3, 7, 4, 7].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_function_dependent_default_indexes_in_constraints() {
    let dir = temp_dir("source-static-function-dependent-default-index-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function selected_index(const int base, int delta = base + 1): int {\n\
             return base + delta;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[selected_index(0)] === 7;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let operations = &expressions.constraints[0].operations;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 1,
            prime: None,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [3, 7, 4, 7].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 2, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

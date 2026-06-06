use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{
    read_expression_info_binary_file, CodeOperand, OperationKind,
};
use lzvm_artifacts::hint_program::{
    HintOperand, SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT,
    SOURCE_UNSUPPORTED_CALL_HINT,
};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;
use lzvm_field::Felt;
use lzvm_prover::regular_constraints::{
    evaluate_regular_constraints, RegularConstraintInputs, RegularStageColumns,
};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-call-hints-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_records_unsupported_source_calls_as_regular_hints() {
    let dir = temp_dir("unsupported-source-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness selector;\n\
             source_protocol_call(sel: selector, value: selector);\n\
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
    assert_eq!(expressions.hints[0].name, SOURCE_UNSUPPORTED_CALL_HINT);
    assert_eq!(expressions.hints[0].fields[0].name, "name");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_UNSUPPORTED_CALL_HINT);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_range_check_calls_as_lookup_assumes_hints() {
    let dir = temp_dir("source-range-check-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int MAX_LIMIT = 0xFFFF;\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness value;\n\
             range_check(expression: value, min: 0, max: MAX_LIMIT, sel: selector);\n\
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
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 3);
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(102)
    );
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_multi_range_check_calls_as_lookup_assumes_hints() {
    let dir = temp_dir("source-multi-range-check-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness range_selector;\n\
             col witness value;\n\
             multi_range_check(min1: 0, max1: 3, min2: 4, max2: 7,\n\
                               range_sel: range_selector, expression: value, sel: selector);\n\
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
    assert_eq!(expressions.hints.len(), 2);
    assert!(expressions
        .hints
        .iter()
        .all(|hint| hint.name == SOURCE_LOOKUP_ASSUMES_HINT));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(102)
    );
    assert_eq!(
        regular.hints.hints[1].fields[0].values[0].operand,
        HintOperand::Number(103)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_dual_byte_range_calls_as_lookup_assumes_hints() {
    let dir = temp_dir("source-dual-byte-range-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int DUAL_RANGE_BYTE_ID = 88;\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness byte_a;\n\
             col witness byte_b;\n\
             range_dual_byte(byte_a, byte_b, selector);\n\
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
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[0].fields[0].name, "bus_id");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(88)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_arith_helper_calls_as_lookup_hints() {
    let dir = temp_dir("source-arith-helper-calls");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int ARITH_TABLE_ID = 331;\n\
         const int ARITH_RANGE_TABLE_ID = 330;\n\
         const int ARITH_RANGE_CARRY = 100;\n\
         airtemplate UnitA() {\n\
             col witness op;\n\
             col witness flag;\n\
             col witness range_ab;\n\
             col witness range_cd;\n\
             col witness value;\n\
             arith_table_assumes(op, flag, flag, flag, flag, flag, flag, flag,\n\
                                 flag, flag, flag, flag, flag, range_ab, range_cd);\n\
             arith_range_table_assumes(ARITH_RANGE_CARRY, value, flag);\n\
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
    assert_eq!(expressions.hints.len(), 2);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[0].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[1].fields[0].name, "bus_id");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(331)
    );
    assert_eq!(
        regular.hints.hints[1].fields[0].values[0].operand,
        HintOperand::Number(330)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_memory_helper_calls_as_lookup_hints() {
    let dir = temp_dir("source-memory-helper-calls");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int MEMORY_ID = 10;\n\
         const int MEMORY_LOAD_OP = 1;\n\
         const int MEMORY_STORE_OP = 2;\n\
         const int MEMORY_REG_OP = 3;\n\
         const int RESERVED_MEM_STEPS = 1;\n\
         const int MAX_MEM_STEPS_PER_MAIN_STEP = 4;\n\
         airtemplate UnitA(int RC = 2, int stack_enabled = 0) {\n\
             col witness selector;\n\
             col witness op;\n\
             col witness addr;\n\
             col witness step;\n\
             const expr a_src_mem = selector;\n\
             const expr a_src_reg = selector;\n\
             const expr a_mem_step = step;\n\
             col witness a[RC];\n\
             col witness value[RC];\n\
             const expr air.addr0;\n\
             expr src_mem = selector;\n\
             expr src_reg = selector;\n\
             if (stack_enabled) {\n\
                 const expr air.addr0;\n\
                 addr0 = step;\n\
             } else {\n\
                 const expr air.addr0;\n\
                 addr0 = addr;\n\
             }\n\
             reg_pre_load(sel: selector, prev_mem_step: step, addr: addr, value: value);\n\
             mem_op(sel: selector,\n\
                    op: MEMORY_LOAD_OP * selector + op,\n\
                    mem_step: step,\n\
                    bytes: 8,\n\
                    addr: addr,\n\
                    value: value);\n\
             mem_op(sel: src_mem + src_reg,\n\
                    op: MEMORY_LOAD_OP * src_mem + MEMORY_REG_OP * src_reg,\n\
                    mem_step: step,\n\
                    addr: addr0,\n\
                    value: value);\n\
             mem_op(sel: a_src_mem + a_src_reg,\n\
                    op: MEMORY_LOAD_OP * a_src_mem + MEMORY_REG_OP * a_src_reg,\n\
                    mem_step: a_mem_step,\n\
                    addr: addr0,\n\
                    value: a);\n\
             mem_op(op: MEMORY_REG_OP,\n\
                    mem_step: step,\n\
                    addr: addr,\n\
                    value: value[0],\n\
                    sel: selector);\n\
             global_init_mem(sel: selector, addr: addr, value: value);\n\
             precompiled_mem_load(addr: addr, main_step: step, value: value, sel: selector);\n\
             precompiled_mem_store(addr: addr, main_step: step, value: [selector, 0], sel: selector);\n\
             precompiled_mem_op(addr: addr, main_step: step, value: value, sel: selector, is_write: selector);\n\
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
    assert_eq!(expressions.hints.len(), 9);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(expressions.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[2].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[3].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[4].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[5].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[6].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[7].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[8].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[4].fields[0].name, "line");
    assert_eq!(expressions.hints[5].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[6].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[7].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[8].fields[0].name, "bus_id");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 9);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[2].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[3].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[4].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[5].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[6].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[7].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[8].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[4].fields[0].name, "line");
    assert_eq!(regular.hints.hints[5].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[6].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[7].fields[0].name, "bus_id");
    assert_eq!(regular.hints.hints[8].fields[0].name, "bus_id");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_operation_helper_calls_as_lookup_hints() {
    let dir = temp_dir("source-operation-helper-calls");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int OPERATION_BUS_ID = 5000;\n\
         const int OP_ADD = 10;\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness op;\n\
             col witness flag;\n\
             col witness step;\n\
             col witness a[2];\n\
             col witness b[2];\n\
             col witness c[2];\n\
             assumes_operation(op:,\n\
                               a: [a[0], a[1]],\n\
                               b:,\n\
                               c:,\n\
                               flag:,\n\
                               main_step: step,\n\
                               extended_arg: selector,\n\
                               sel: selector);\n\
             proves_operation(op: OP_ADD, a:, b:, c:, flag: flag, mul: selector);\n\
             assumes_padding_operation(op: OP_ADD, padding_size: selector);\n\
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
    assert_eq!(expressions.hints.len(), 3);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[1].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(expressions.hints[2].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[0].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[1].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[2].fields[0].name, "bus_id");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 3);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[2].name, SOURCE_LOOKUP_ASSUMES_HINT);
    for hint in &regular.hints.hints {
        assert_eq!(hint.fields[0].name, "bus_id");
        assert_eq!(hint.fields[0].values[0].operand, HintOperand::Number(5000));
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_signed_permutation_calls_as_lookup_hints() {
    let dir = temp_dir("source-signed-permutation-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int MEMORY_ID = 10;\n\
         airtemplate UnitA() {\n\
             col witness sel_prove;\n\
             col witness sel_assume;\n\
             col witness op;\n\
             col witness value[2];\n\
             permutation(MEMORY_ID, expressions: [op, ...value], sel: sel_prove - sel_assume);\n\
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
    assert_eq!(expressions.hints[0].fields[0].name, "bus_id");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(10)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_satisfied_source_static_assert_calls() {
    let dir = temp_dir("satisfied-source-static-assert");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant N = 2;\n\
         airtemplate UnitA() {\n\
             assert(N == 2);\n\
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
fn generate_key_skips_satisfied_source_static_assert_calls_with_messages() {
    let dir = temp_dir("satisfied-source-static-assert-message");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant N = 2;\n\
         airtemplate UnitA() {\n\
             assert(N == 2, `N is ${N}`);\n\
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
fn generate_key_rejects_failed_source_static_assert_calls() {
    let dir = temp_dir("failed-source-static-assert");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant N = 2;\n\
         airtemplate UnitA() {\n\
             assert(N == 3, `N is ${N}`);\n\
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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup key generation failed: source static assertion failed: assert(N == 3, `N is ${N}`)\n"
    );
}

#[test]
fn generate_key_runs_final_air_function_static_assertions() {
    let dir = temp_dir("final-air-static-assert");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function finish(const int expected) {\n\
             assert(expected == 7);\n\
         }\n\
         airtemplate UnitA() {\n\
             on final air finish(7);\n\
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
    assert!(expressions.constraints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_rejects_failed_final_air_function_static_assertions() {
    let dir = temp_dir("failed-final-air-static-assert");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function finish(const int expected) {\n\
             assert(expected == 7);\n\
         }\n\
         airtemplate UnitA() {\n\
             on final air finish(3);\n\
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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup key generation failed: source static assertion failed: assert(expected == 7)\n"
    );
}

#[test]
fn generate_key_runs_final_air_function_assert_eq_static_updates() {
    let dir = temp_dir("final-air-assert-eq-static-updates");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "int counter = 0;\n\
         int delta = 2;\n\
         function finish() {\n\
             assert_eq(counter, 0);\n\
             counter += delta;\n\
             assert_eq(counter, 2);\n\
         }\n\
         airtemplate UnitA() {\n\
             on final air finish();\n\
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_rejects_failed_final_air_function_assert_eq() {
    let dir = temp_dir("failed-final-air-assert-eq");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "int counter = 1;\n\
         function finish() {\n\
             assert_eq(counter, 0);\n\
         }\n\
         airtemplate UnitA() {\n\
             on final air finish();\n\
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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup key generation failed: source static assertion failed: assert_eq(counter, 0)\n"
    );
}

#[test]
fn generate_key_allows_final_air_print_calls_without_lowered_work() {
    let dir = temp_dir("final-air-print-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function finish() {\n\
             println(\"final air\");\n\
         }\n\
         airtemplate UnitA() {\n\
             on final air finish();\n\
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
    assert!(expressions.constraints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_runs_final_air_functions_with_shared_static_updates() {
    let dir = temp_dir("final-air-shared-static-updates");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "int counter = 0;\n\
         function first() {\n\
             assert_eq(counter, 0);\n\
             counter += 1;\n\
         }\n\
         function second() {\n\
             assert_eq(counter, 1);\n\
             counter += 1;\n\
         }\n\
         airtemplate UnitA() {\n\
             on final air first();\n\
             on final air second();\n\
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_runs_final_air_function_with_airgroup_static_assignment() {
    let dir = temp_dir("final-air-airgroup-static-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "int counter = 0;\n\
         function check() {\n\
             assert_eq(counter, 2);\n\
         }\n\
         airtemplate UnitA() {\n\
             on final air check();\n\
         }\n\
         airgroup GroupA {\n\
             counter = 2;\n\
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_final_airgroup_hooks_without_lowered_work() {
    let dir = temp_dir("final-airgroup-hook");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function finish() {\n\
             println(\"final group\");\n\
         }\n\
         airtemplate UnitA() {\n\
             on final airgroup finish();\n\
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
    assert!(expressions.constraints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_allows_top_level_final_airgroup_hooks_without_lowered_work() {
    let dir = temp_dir("top-level-final-airgroup-hook");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function finish() {\n\
             println(\"final group\");\n\
         }\n\
         on final airgroup finish();\n\
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_runs_final_air_functions_by_descending_priority() {
    let dir = temp_dir("final-air-descending-priority");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "int counter = 0;\n\
         function first() {\n\
             assert_eq(counter, 0);\n\
             counter += 1;\n\
         }\n\
         function second() {\n\
             assert_eq(counter, 1);\n\
             counter += 1;\n\
         }\n\
         function third() {\n\
             assert_eq(counter, 2);\n\
             counter += 1;\n\
         }\n\
         airtemplate UnitA() {\n\
             on final(1) air third();\n\
             on final(3) air first();\n\
             on final(2) air second();\n\
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_runs_reentrant_final_air_function_at_lower_priority() {
    let dir = temp_dir("reentrant-final-air-lower-priority");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "int counter = 0;\n\
         function first() {\n\
             assert_eq(counter, 0);\n\
             counter += 1;\n\
             on final(2) air third();\n\
         }\n\
         function second() {\n\
             assert_eq(counter, 1);\n\
             counter += 1;\n\
         }\n\
         function third() {\n\
             assert_eq(counter, 2);\n\
             counter += 1;\n\
         }\n\
         airtemplate UnitA() {\n\
             on final(5) air first();\n\
             on final(3) air second();\n\
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_reentrant_final_air_function_above_current_priority() {
    let dir = temp_dir("reentrant-final-air-higher-priority");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "int counter = 0;\n\
         function first() {\n\
             assert_eq(counter, 0);\n\
             counter += 1;\n\
             on final(6) air earlier();\n\
         }\n\
         function earlier() {\n\
             assert_eq(counter, 0);\n\
         }\n\
         function second() {\n\
             assert_eq(counter, 1);\n\
             counter += 1;\n\
         }\n\
         airtemplate UnitA() {\n\
             on final(5) air first();\n\
             on final(3) air second();\n\
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_static_if_bodies() {
    let dir = temp_dir("source-function-static-if");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_flag(expr value, const int enabled) {\n\
             if (enabled) {\n\
                 value * (1 - value) === 0;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness flag;\n\
             constrain_flag(flag, 1);\n\
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
fn generate_key_lowers_source_function_params_shadowing_columns() {
    let dir = temp_dir("source-function-shadowed-parameter");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_value(expr value) {\n\
             value * (1 - value) === 0;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             constrain_value(value);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0, 0];",
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
    assert!(results[0].invalid_rows.is_empty());

    let invalid_stage_values = [0, 2].map(Felt::from_u64);
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
    assert_eq!(invalid_results[0].invalid_rows[0].row, 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_ignores_source_directives_in_lowered_function_bodies() {
    let dir = temp_dir("source-function-directive");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let include_path = dir.join("source").join("extra.pil");
    write_file(&include_path, "");
    write_file(
        &source_path,
        "function constrain_flag(expr value) {\n\
             private require \"extra.pil\"\n\
             value * (1 - value) === 0;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness flag;\n\
             constrain_flag(flag);\n\
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
fn generate_key_lowers_source_function_calls_with_static_for_bodies() {
    let dir = temp_dir("source-function-static-for");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function repeat_check(expr value, const int count) {\n\
             for (int index = 0; index < count; ++index) {\n\
                 value * (1 - value) === 0;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness flag;\n\
             repeat_check(flag, 2);\n\
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
fn generate_key_lowers_source_function_calls_with_static_for_accumulators() {
    let dir = temp_dir("source-function-static-for-accumulator");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "constant BASE = 2;\n\
         function require_packed(expr expected, const expr values[], const int count) {\n\
             expr acc = 0;\n\
             for (int index = 0; index < count; ++index) {\n\
                 acc += values[index] * (BASE ** index);\n\
             }\n\
             expected === acc;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness bits[3];\n\
             const expr values[3] = build_values(bits);\n\
             const expr packed = bits[0] + BASE * bits[1] + (BASE ** 2) * bits[2];\n\
             require_packed(packed, values, 3);\n\
             function build_values(const expr input[]): const expr {\n\
                 const expr result[3];\n\
                 for (int index = 0; index < 3; ++index) {\n\
                     result[index] = input[index];\n\
                 }\n\
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
fn generate_key_lowers_source_function_calls_with_mixed_argument_binding() {
    let dir = temp_dir("source-function-mixed-argument-binding");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function require_equal(expr left, expr right) {\n\
             left === right;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             require_equal(left: values[0], values[1]);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0, 0];",
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
            element: 0,
            prime: None,
            dimension: 1,
        }
    ));
    assert!(matches!(
        operations[0].sources[1],
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

    let stage_values = [3, 3, 4, 4].map(Felt::from_u64);
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
fn generate_key_lowers_nested_source_function_calls() {
    let dir = temp_dir("source-function-nested-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function require_equal(expr left, expr right) {\n\
             left === right;\n\
         }\n\
         function require_increment(expr base, expr target) {\n\
             require_equal(target, base + 1);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             col witness next;\n\
             require_increment(value, next);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0, 0];",
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
    assert_eq!(operations[0].op, OperationKind::Add);
    assert_eq!(operations[1].op, OperationKind::Sub);

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [3, 4, 6, 7].map(Felt::from_u64);
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

    let invalid_stage_values = [3, 4, 6, 8].map(Felt::from_u64);
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
fn generate_key_lowers_source_function_calls_with_dependent_default_arguments() {
    let dir = temp_dir("source-function-dependent-default-argument");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function require_offset(expr values[], const int base, const int index = base + 1) {\n\
             values[index] === values[0] + index;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             require_offset(values, 0);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0, 0];",
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
    assert_eq!(operations[0].op, OperationKind::Add);
    assert!(matches!(
        operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 0,
            prime: None,
            dimension: 1,
        }
    ));
    assert_eq!(operations[1].op, OperationKind::Sub);
    assert!(matches!(
        operations[1].sources[0],
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

    let stage_values = [3, 4, 6, 7].map(Felt::from_u64);
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
fn generate_key_lowers_source_function_calls_with_local_expr_arrays() {
    let dir = temp_dir("source-function-local-expr-arrays");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function apply_round(const int n, const expr input[], const expr output[], const expr sel) {\n\
             const expr mat[n];\n\
             for (int i = 0; i < n; i++) {\n\
                 mat[i] = input[i] + 1;\n\
             }\n\
             for (int i = 0; i < n; i++) {\n\
                 sel * (output[i] - mat[i]) === 0;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness input[2];\n\
             col witness output[2];\n\
             apply_round(2, input, output, selector);\n\
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
    assert_eq!(expressions.constraints.len(), 2);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 0);
    assert_eq!(regular.constraints.entries.len(), 2);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_local_lookup_arrays() {
    let dir = temp_dir("source-function-local-lookup-arrays");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
         "const int TABLE_ID = 77;\n\
         const int CHUNK_SIZE = 3;\n\
         const int BASE = 2;\n\
         function mix(const expr left, const expr right): const expr {\n\
             return left + right;\n\
         }\n\
         function rotate_one(const expr input[]): const expr[] {\n\
             const expr result[4];\n\
             for (int i = 0; i < 4; i++) {\n\
                 result[i] = input[(i + 1) % 4];\n\
             }\n\
             return result;\n\
         }\n\
         function add_one(const expr input[]): const expr {\n\
             const expr result[4];\n\
             for (int i = 0; i < 4; i++) {\n\
                 result[i] = mix(input[i], 1);\n\
             }\n\
             return result;\n\
         }\n\
         function pack_lookup_chunk(const int chunk, const int num_bits, const expr acc,\n\
                                    const expr next_bits[], const expr values[], const expr sel) {\n\
             const int bit_offset = chunk * CHUNK_SIZE;\n\
             expr packed = 0;\n\
             for (int j = 0; j < num_bits; j++) {\n\
                 packed += values[bit_offset + j] * (BASE ** j);\n\
             }\n\
             acc === packed;\n\
             const expr lookup_values[CHUNK_SIZE + 1];\n\
             lookup_values[0] = acc;\n\
             for (int j = 0; j < num_bits; j++) {\n\
                 lookup_values[j + 1] = next_bits[bit_offset + j]';\n\
             }\n\
             for (int j = num_bits + 1; j < CHUNK_SIZE + 1; j++) {\n\
                 lookup_values[j] = 0;\n\
             }\n\
             lookup_assumes(TABLE_ID, lookup_values, sel: sel);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness bits(22) accs[2];\n\
             col witness current[4];\n\
             col witness next[4];\n\
             const expr round[4] = add_one(rotate_one(current));\n\
             pack_lookup_chunk(chunk: 0, num_bits: 3, acc: accs[0],\n\
                               next_bits: next, values: round, sel: selector);\n\
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
    assert_eq!(
        expressions
            .hints
            .iter()
            .filter(|hint| hint.name == SOURCE_UNSUPPORTED_CALL_HINT)
            .count(),
        0
    );
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.constraints.len(), 1);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.constraints.entries.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_lookup_arrays_with_air_qualified_columns() {
    let dir = temp_dir("source-function-air-qualified-lookup-arrays");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int TABLE_ID = 77;\n\
         const int CHUNK_SIZE = 3;\n\
         const int BASE = 2;\n\
         function pack_lookup_chunk(const int chunk, const int num_bits, const expr acc,\n\
                                    const expr values[], const expr sel) {\n\
             const int bit_offset = chunk * CHUNK_SIZE;\n\
             expr packed = 0;\n\
             for (int j = 0; j < num_bits; j++) {\n\
                 packed += values[bit_offset + j] * (BASE ** j);\n\
             }\n\
             acc === packed;\n\
             const expr lookup_values[CHUNK_SIZE + 1];\n\
             lookup_values[0] = acc;\n\
             for (int j = 0; j < num_bits; j++) {\n\
                 lookup_values[j + 1] = air.state[bit_offset + j]';\n\
             }\n\
             for (int j = num_bits + 1; j < CHUNK_SIZE + 1; j++) {\n\
                 lookup_values[j] = 0;\n\
             }\n\
             lookup_assumes(TABLE_ID, lookup_values, sel: sel);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness accs[2];\n\
             col witness state[4];\n\
             const expr round[4] = [state[0], state[1], state[2], state[3]];\n\
             pack_lookup_chunk(chunk: 0, num_bits: CHUNK_SIZE, acc: accs[0],\n\
                               values: round, sel: selector);\n\
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
    assert_eq!(
        expressions
            .hints
            .iter()
            .filter(|hint| hint.name == SOURCE_UNSUPPORTED_CALL_HINT)
            .count(),
        0
    );
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.constraints.len(), 1);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[0].fields.len(), 3);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(77)
    );
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    let values = &regular.hints.hints[0].fields[1].values;
    assert_eq!(values.len(), 4);
    assert_eq!(
        values[0].operand,
        HintOperand::CommitmentElement {
            id: 1,
            element: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        values[1].operand,
        HintOperand::CommitmentElement {
            id: 2,
            element: 0,
            row_offset_index: 1
        }
    );
    assert_eq!(
        values[2].operand,
        HintOperand::CommitmentElement {
            id: 2,
            element: 1,
            row_offset_index: 1
        }
    );
    assert_eq!(
        values[3].operand,
        HintOperand::CommitmentElement {
            id: 2,
            element: 2,
            row_offset_index: 1
        }
    );
    assert_eq!(regular.hints.hints[0].fields[2].name, "selector");
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(regular.constraints.entries.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_nested_returned_scalars() {
    let dir = temp_dir("source-function-nested-returned-scalars");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             const int WIDTH = 8;\n\
             const int CHUNK_SIZE = 3;\n\
             const int TABLE_ID = 77;\n\
             const int BASE = 2;\n\
             col witness selector;\n\
             col witness accs[3];\n\
             col witness state[WIDTH];\n\
             function add(const expr items[]): const expr {\n\
                 const int len = length(items);\n\
                 expr result = 0;\n\
                 for (int i = 0; i < len; i++) {\n\
                     result += items[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function xor(const expr left, const expr right): const expr {\n\
                 return add([left, right]);\n\
             }\n\
             function xor3(const expr a, const expr b, const expr c): const expr {\n\
                 return add([a, b, c]);\n\
             }\n\
             function theta(const expr input[]): const expr[] {\n\
                 const expr result[air.WIDTH];\n\
                 const expr c[2][2];\n\
                 for (int x = 0; x < 2; x++) {\n\
                     for (int z = 0; z < 2; z++) {\n\
                         c[x][z] = xor3(input[x * 4 + z], input[x * 4 + z + 2], input[(x * 4 + z + 4) % air.WIDTH]);\n\
                     }\n\
                 }\n\
                 const expr d[2][2];\n\
                 for (int x = 0; x < 2; x++) {\n\
                     for (int z = 0; z < 2; z++) {\n\
                         d[x][z] = xor(c[(x + 1) % 2][z], c[x][(z + 1) % 2]);\n\
                     }\n\
                 }\n\
                 for (int x = 0; x < 2; x++) {\n\
                     for (int y = 0; y < 2; y++) {\n\
                         for (int z = 0; z < 2; z++) {\n\
                             const int pos = x * 4 + y * 2 + z;\n\
                             result[pos] = xor(input[pos], d[x][z]);\n\
                         }\n\
                     }\n\
                 }\n\
                 return result;\n\
             }\n\
             function chi(const expr input[]): const expr[] {\n\
                 const expr result[air.WIDTH];\n\
                 for (int i = 0; i < air.WIDTH; i++) {\n\
                     result[i] = xor(input[i], (1 + input[(i + 1) % air.WIDTH]) * input[(i + 2) % air.WIDTH]);\n\
                 }\n\
                 return result;\n\
             }\n\
             function final_round(const expr input[]): const expr {\n\
                 const expr result[air.WIDTH];\n\
                 result[0] = xor(input[0], 1);\n\
                 for (int i = 1; i < air.WIDTH; i++) {\n\
                     result[i] = input[i];\n\
                 }\n\
                 return result;\n\
             }\n\
             function pack_lookup_chunk(const int chunk, const int num_bits, const expr acc,\n\
                                        const expr values[], const expr sel) {\n\
                 const int bit_offset = chunk * air.CHUNK_SIZE;\n\
                 expr packed = 0;\n\
                 for (int j = 0; j < num_bits; j++) {\n\
                     packed += values[bit_offset + j] * (air.BASE ** j);\n\
                 }\n\
                 acc === packed;\n\
                 const expr lookup_values[air.CHUNK_SIZE + 1];\n\
                 lookup_values[0] = acc;\n\
                 for (int j = 0; j < num_bits; j++) {\n\
                     lookup_values[j + 1] = state[bit_offset + j]';\n\
                 }\n\
                 for (int j = num_bits + 1; j < air.CHUNK_SIZE + 1; j++) {\n\
                     lookup_values[j] = 0;\n\
                 }\n\
                 lookup_assumes(TABLE_ID, lookup_values, sel: sel);\n\
             }\n\
             const expr round[WIDTH] = final_round(chi(theta(state)));\n\
             pack_lookup_chunk(chunk: 0, num_bits: CHUNK_SIZE, acc: accs[0],\n\
                               values: round, sel: selector);\n\
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
    assert_eq!(
        expressions
            .hints
            .iter()
            .filter(|hint| hint.name == SOURCE_UNSUPPORTED_CALL_HINT)
            .count(),
        0
    );
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.constraints.len(), 1);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.constraints.entries.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_records_recursive_returned_scalar_calls_as_unsupported() {
    let dir = temp_dir("recursive-returned-scalar-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             const int TABLE_ID = 77;\n\
             col witness selector;\n\
             col witness acc;\n\
             col witness current;\n\
             function recurse(const expr input): const expr {\n\
                 return recurse(input);\n\
             }\n\
             function pack_lookup(const expr acc, const expr value, const expr sel) {\n\
                 acc === recurse(value);\n\
                 lookup_assumes(TABLE_ID, [acc, current'], sel: sel);\n\
             }\n\
             pack_lookup(acc: acc, value: current, sel: selector);\n\
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
    assert_eq!(expressions.hints[0].name, SOURCE_UNSUPPORTED_CALL_HINT);
    assert_eq!(expressions.constraints.len(), 0);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_UNSUPPORTED_CALL_HINT);
    assert_eq!(regular.constraints.entries.len(), 0);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_passthrough_returned_lookup_arrays() {
    let dir = temp_dir("source-function-passthrough-returned-lookup-arrays");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int TABLE_ID = 77;\n\
         const int CHUNK_SIZE = 3;\n\
         const int BASE = 2;\n\
         function rotate_one(const expr input[]): const expr[] {\n\
             const expr result[4];\n\
             for (int i = 0; i < 4; i++) {\n\
                 result[i] = input[(i + 1) % 4];\n\
             }\n\
             return result;\n\
         }\n\
         function pass_array(const expr input[]): const expr[] {\n\
             const expr result[4] = input;\n\
             return result;\n\
         }\n\
         function pack_lookup_chunk(const int chunk, const int num_bits, const expr acc,\n\
                                    const expr values[], const expr sel) {\n\
             const int bit_offset = chunk * CHUNK_SIZE;\n\
             expr packed = 0;\n\
             for (int j = 0; j < num_bits; j++) {\n\
                 packed += values[bit_offset + j] * (BASE ** j);\n\
             }\n\
             acc === packed;\n\
             const expr lookup_values[CHUNK_SIZE + 1];\n\
             lookup_values[0] = acc;\n\
             for (int j = 0; j < num_bits; j++) {\n\
                 lookup_values[j + 1] = values[bit_offset + j];\n\
             }\n\
             for (int j = num_bits + 1; j < CHUNK_SIZE + 1; j++) {\n\
                 lookup_values[j] = 0;\n\
             }\n\
             lookup_assumes(TABLE_ID, lookup_values, sel: sel);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness accs[2];\n\
             col witness current[4];\n\
             pack_lookup_chunk(chunk: 0, num_bits: 3, acc: accs[0],\n\
                               values: pass_array(rotate_one(current)), sel: selector);\n\
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
    assert_eq!(
        expressions
            .hints
            .iter()
            .filter(|hint| hint.name == SOURCE_UNSUPPORTED_CALL_HINT)
            .count(),
        0
    );
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.constraints.len(), 1);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.constraints.entries.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_scalar_typed_returned_lookup_arrays() {
    let dir = temp_dir("source-function-scalar-typed-returned-lookup-arrays");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int TABLE_ID = 77;\n\
         const int CHUNK_SIZE = 3;\n\
         const int BASE = 2;\n\
         function rotate_one(const expr input[]): const expr[] {\n\
             const expr result[4];\n\
             for (int i = 0; i < 4; i++) {\n\
                 result[i] = input[(i + 1) % 4];\n\
             }\n\
             return result;\n\
         }\n\
         function pass_array(const expr input[]): const expr {\n\
             const expr result[4] = input;\n\
             return result;\n\
         }\n\
         function pack_lookup_chunk(const int chunk, const int num_bits, const expr acc,\n\
                                    const expr values[], const expr sel) {\n\
             const int bit_offset = chunk * CHUNK_SIZE;\n\
             expr packed = 0;\n\
             for (int j = 0; j < num_bits; j++) {\n\
                 packed += values[bit_offset + j] * (BASE ** j);\n\
             }\n\
             acc === packed;\n\
             const expr lookup_values[CHUNK_SIZE + 1];\n\
             lookup_values[0] = acc;\n\
             for (int j = 0; j < num_bits; j++) {\n\
                 lookup_values[j + 1] = values[bit_offset + j];\n\
             }\n\
             for (int j = num_bits + 1; j < CHUNK_SIZE + 1; j++) {\n\
                 lookup_values[j] = 0;\n\
             }\n\
             lookup_assumes(TABLE_ID, lookup_values, sel: sel);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness accs[2];\n\
             col witness current[4];\n\
             pack_lookup_chunk(chunk: 0, num_bits: 3, acc: accs[0],\n\
                               values: pass_array(rotate_one(current)), sel: selector);\n\
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
    assert_eq!(
        expressions
            .hints
            .iter()
            .filter(|hint| hint.name == SOURCE_UNSUPPORTED_CALL_HINT)
            .count(),
        0
    );
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.constraints.len(), 1);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.constraints.entries.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_returned_expr_array_assignments() {
    let dir = temp_dir("source-function-returned-expr-array-assignments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function partial_sum(const int n, const expr input[], const expr value) : const expr {\n\
             const expr res[n - 1];\n\
             res[0] = value + input[1];\n\
             for (int i = 2; i < n; i++) {\n\
                 res[i - 1] = res[i - 2] + input[i];\n\
             }\n\
             return res[n - 2];\n\
         }\n\
         function apply_round(const int n, const expr input[], const expr output[], const expr values[], const expr sel) {\n\
             const expr state[2][n];\n\
             const expr sums[1];\n\
             for (int i = 0; i < n; ++i) {\n\
                 state[0][i] = input[i];\n\
             }\n\
             sums[0] = partial_sum(n, state[0], values[0]);\n\
             state[1][0] = values[0] + sums[0];\n\
             for (int i = 1; i < n; i++) {\n\
                 state[1][i] = state[0][i] + sums[0];\n\
             }\n\
             for (int i = 0; i < n; i++) {\n\
                 sel * (output[i] - state[1][i]) === 0;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness input[3];\n\
             col witness output[3];\n\
             col witness values[1];\n\
             apply_round(3, input, output, values, selector);\n\
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
    assert_eq!(expressions.constraints.len(), 3);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 0);
    assert_eq!(regular.constraints.entries.len(), 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_shadowed_expr_array_slice_arguments() {
    let dir = temp_dir("source-function-shadowed-expr-array-slice-arguments");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function fold_sum(const int n, const expr state[], const expr value) : const expr {\n\
             const expr res[n - 1];\n\
             res[0] = value + state[1];\n\
             for (int i = 2; i < n; i++) {\n\
                 res[i - 1] = res[i - 2] + state[i];\n\
             }\n\
             return res[n - 2];\n\
         }\n\
         function apply_round(const int n, const expr input[], const expr output[], const expr values[], const expr sel) {\n\
             const expr state[2][n];\n\
             const expr sums[1];\n\
             for (int i = 0; i < n; i++) {\n\
                 state[0][i] = input[i];\n\
             }\n\
             sums[0] = fold_sum(n, state[0], values[0]);\n\
             for (int i = 0; i < n; i++) {\n\
                 state[1][i] = state[0][i] + sums[0];\n\
             }\n\
             for (int i = 0; i < n; i++) {\n\
                 sel * (output[i] - state[1][i]) === 0;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness input[3];\n\
             col witness output[3];\n\
             col witness values[1];\n\
             apply_round(3, input, output, values, selector);\n\
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
    assert_eq!(expressions.constraints.len(), 3);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 0);
    assert_eq!(regular.constraints.entries.len(), 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_reused_expr_array_slice_results() {
    let dir = temp_dir("source-function-reused-expr-array-slice-results");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function fold_sum(const int n, const expr input[], const expr value) : const expr {\n\
             const expr res[n - 1];\n\
             res[0] = value + input[1];\n\
             for (int i = 2; i < n; i++) {\n\
                 res[i - 1] = res[i - 2] + input[i];\n\
             }\n\
             return res[n - 2];\n\
         }\n\
         function apply_rounds(const int n, const int rounds, const expr input[], const expr output[],\n\
                               const expr initial[], const expr values[], const int weights[], const expr sel) {\n\
             const expr state[rounds + 1][n];\n\
             const expr sums[rounds];\n\
             for (int i = 0; i < n; ++i) {\n\
                 state[0][i] = input[i];\n\
             }\n\
             for (int i = 0; i < rounds; i++) {\n\
                 sel * (initial[i] - state[i][0]) === 0;\n\
                 sums[i] = fold_sum(n, state[i], values[i]);\n\
                 state[i + 1][0] = values[i] * weights[0] + sums[i];\n\
                 for (int j = 1; j < n; j++) {\n\
                     state[i + 1][j] = state[i][j] * weights[j] + sums[i];\n\
                 }\n\
             }\n\
             for (int i = 0; i < n; i++) {\n\
                 sel * (output[i] - state[rounds][i]) === 0;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             const int weights[3] = [3, 5, 7];\n\
             col witness selector;\n\
             col witness input[3];\n\
             col witness output[3];\n\
             col witness initial[2];\n\
             col witness values[2];\n\
             apply_rounds(3, 2, input, output, initial, values, weights, selector);\n\
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
    assert_eq!(expressions.constraints.len(), 5);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 0);
    assert_eq!(regular.constraints.entries.len(), 5);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_shadowed_recursive_slice_results() {
    let dir = temp_dir("source-function-shadowed-recursive-slice-results");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function fold_sum(const int n, const expr state[], const expr value) : const expr {\n\
             const expr res[n - 1];\n\
             res[0] = value + state[1];\n\
             for (int i = 2; i < n; i++) {\n\
                 res[i - 1] = res[i - 2] + state[i];\n\
             }\n\
             return res[n - 2];\n\
         }\n\
         function apply_rounds(const int n, const int count, const expr input[], const expr output[],\n\
                               const expr values[], const expr sel) {\n\
             const expr state[count + 1][n];\n\
             const expr sums[count];\n\
             for (int i = 0; i < n; i++) {\n\
                 state[0][i] = input[i];\n\
             }\n\
             for (int i = 0; i < count; i++) {\n\
                 sums[i] = fold_sum(n, state[i], values[i]);\n\
                 for (int j = 0; j < n; j++) {\n\
                     state[i + 1][j] = state[i][j] + sums[i];\n\
                 }\n\
             }\n\
             for (int i = 0; i < n; i++) {\n\
                 sel * (output[i] - state[count][i]) === 0;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness input[3];\n\
             col witness output[3];\n\
             col witness values[2];\n\
             apply_rounds(3, 2, input, output, values, selector);\n\
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
    assert_eq!(expressions.constraints.len(), 3);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 0);
    assert_eq!(regular.constraints.entries.len(), 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_function_calls_with_nested_expr_array_accumulators() {
    let dir = temp_dir("source-function-nested-expr-array-accumulators");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function mix_round(const int n, const expr input[], const expr output[], const expr sel) {\n\
             const expr mat[n];\n\
             const expr t0[n / 4], t1[n / 4], t2[n / 4], t3[n / 4];\n\
             for (int i = 0; i < n / 4; i++) {\n\
                 t0[i] = input[4 * i] + input[4 * i + 1];\n\
                 t1[i] = input[4 * i + 2] + input[4 * i + 3];\n\
                 t2[i] = 2 * input[4 * i + 1] + t1[i];\n\
                 t3[i] = 2 * input[4 * i + 3] + t0[i];\n\
                 mat[4 * i + 3] = 4 * t1[i] + t3[i];\n\
                 mat[4 * i + 1] = 4 * t0[i] + t2[i];\n\
                 mat[4 * i] = t3[i] + mat[4 * i + 1];\n\
                 mat[4 * i + 2] = t2[i] + mat[4 * i + 3];\n\
             }\n\
             expr stored[n / 4];\n\
             for (int i = 0; i < n / 4; i++) {\n\
                 stored[i] = 0;\n\
             }\n\
             for (int i = 0; i < n / 4; i++) {\n\
                 for (int j = 0; j < 4; j++) {\n\
                     stored[j] += mat[4 * i + j];\n\
                 }\n\
             }\n\
             for (int i = 0; i < n; i++) {\n\
                 sel * (output[i] - (mat[i] + stored[i % 4])) === 0;\n\
             }\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             col witness input[16];\n\
             col witness output[16];\n\
             mix_round(16, input, output, selector);\n\
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
    assert_eq!(expressions.constraints.len(), 16);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 0);
    assert_eq!(regular.constraints.entries.len(), 16);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_source_static_array_length_assert_calls() {
    let dir = temp_dir("source-static-array-length-assert");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             const int expected[] = [3, 5];\n\
             assert(length(expected) == 2);\n\
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
fn generate_key_skips_source_function_local_static_array_length_assert_calls() {
    let dir = temp_dir("source-function-local-static-array-length-assert");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function check_expected() {\n\
             const int expected[] = [3, 5];\n\
             assert(length(expected) == 2);\n\
         }\n\
         airtemplate UnitA() {\n\
             check_expected();\n\
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
    assert!(expressions.constraints.is_empty());
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert!(regular.constraints.entries.is_empty());
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_applies_expr_array_parameter_outer_length_assertions() {
    let dir = temp_dir("source-expr-array-parameter-outer-length-assert");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function check_rows(const expr rows[][]) {\n\
             assert(length(rows) == 2);\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             const expr rows[2][3];\n\
             for (int i = 0; i < 2; i++) {\n\
                 for (int j = 0; j < 3; j++) {\n\
                     rows[i][j] = value;\n\
                 }\n\
             }\n\
             check_rows(rows);\n\
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
    assert!(expressions.constraints.is_empty());
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert!(regular.constraints.entries.is_empty());
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_source_function_local_static_array_element_assert_calls() {
    let dir = temp_dir("source-function-local-static-array-element-assert");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function check_expected() {\n\
             const int expected[] = [3, 5];\n\
             assert(expected[0] == 3);\n\
         }\n\
         airtemplate UnitA() {\n\
             check_expected();\n\
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
    assert!(expressions.constraints.is_empty());
    assert!(expressions.hints.is_empty());
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert!(regular.constraints.entries.is_empty());
    assert!(regular.hints.hints.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lzvm_artifacts::expression_info::{
    read_expression_info_binary_file, BoundaryKind, CodeOperand,
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
        "lzvm-cli-setup-generate-source-for-loop-constraints-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

fn static_value_table_source(value_count: usize, loop_count: usize) -> String {
    let mut source = String::from("airtemplate UnitA() {\n    col witness values[1];\n");
    for index in 0..value_count {
        source.push_str(&format!("    int v{index} = {index};\n"));
    }
    source.push_str(&format!(
        "    for (int index = 0; index < {loop_count}; ++index) {{\n\
                 values[0] === 0;\n\
             }}\n\
         }}\n\
         airgroup GroupA {{ UnitA(); }}\n\
         col fixed main.left = [0, 0];"
    ));
    source
}

fn many_static_for_loops_source(loop_count: usize) -> String {
    let mut source = String::from("airtemplate UnitA() {\n    col witness values[1];\n");
    for index in 0..loop_count {
        source.push_str(&format!(
            "    for (int index{index} = 0; index{index} < 1; ++index{index}) {{\n\
                 values[0] === 0;\n\
             }}\n"
        ));
    }
    source.push_str("}\nairgroup GroupA { UnitA(); }\ncol fixed main.left = [0, 0];");
    source
}

fn many_static_if_statements_source(statement_count: usize) -> String {
    let mut source = String::from("airtemplate UnitA() {\n    col witness values[1];\n");
    for _ in 0..statement_count {
        source.push_str(
            "    if (1) {\n\
                 values[0] === 0;\n\
             }\n",
        );
    }
    source.push_str("}\nairgroup GroupA { UnitA(); }\ncol fixed main.left = [0, 0];");
    source
}

fn nested_zero_static_for_loop_source(outer_count: usize, body_statement_count: usize) -> String {
    let mut source = String::from("airtemplate UnitA() {\n    col witness values[1];\n");
    source.push_str(&format!(
        "    for (int outer = 0; outer < {outer_count}; ++outer) {{\n\
             for (int inner = 0; inner < 0; ++inner) {{\n"
    ));
    for _ in 0..body_statement_count {
        source.push_str("            values[0] === 0;\n");
    }
    source.push_str(
        "        }\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0, 0];",
    );
    source
}

#[test]
fn generate_key_unrolls_static_source_for_loop_constraints() {
    let dir = temp_dir("static-for-loop-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[2];\n\
             for (int index = 0; index < 2; ++index) {\n\
                 values[index] * (1 - values[index]) === 0;\n\
             }\n\
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
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    assert!(constraint_uses_commitment_element(
        &expressions.constraints[0],
        0
    ));
    assert!(constraint_uses_commitment_element(
        &expressions.constraints[1],
        1
    ));

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [0, 1, 1, 0].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &stage_values,
    }];
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

    let invalid_stage_values = [0, 1, 1, 2].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &invalid_stage_values,
    }];
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
    assert!(invalid_results[0].invalid_rows.is_empty());
    assert_eq!(invalid_results[1].invalid_rows.len(), 1);
    assert_eq!(invalid_results[1].invalid_rows[0].row, 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_unrolls_static_source_if_constraints() {
    let dir = temp_dir("static-if-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[2];\n\
             for (int index = 0; index < 2; ++index) {\n\
                 if (index == 1) {\n\
                     values[index] * (1 - values[index]) === 0;\n\
                 }\n\
             }\n\
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
    assert!(constraint_uses_commitment_element(
        &expressions.constraints[0],
        1
    ));

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [2, 0, 3, 1].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &stage_values,
    }];
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

    let invalid_stage_values = [0, 1, 1, 2].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &invalid_stage_values,
    }];
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
fn generate_key_updates_static_source_postfix_values_in_for_loop_constraints() {
    let dir = temp_dir("static-for-loop-postfix-state");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[4];\n\
             int byte = 0;\n\
             int chunk = 0;\n\
             for (int index = 0; index < 4; ++index) {\n\
                 values[index] === byte + chunk * 10;\n\
                 byte++;\n\
                 if (byte == 2) {\n\
                     byte = 0;\n\
                     chunk++;\n\
                 }\n\
             }\n\
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
    assert_eq!(expressions.constraints.len(), 4);
    assert!(expressions.hints.is_empty());
    for (element, constraint) in expressions.constraints.iter().enumerate() {
        assert!(constraint_uses_commitment_element(
            constraint,
            u32::try_from(element).expect("element should fit")
        ));
    }

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 4);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [0, 1, 10, 11, 0, 1, 10, 11].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 4,
        values: &stage_values,
    }];
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

    let invalid_stage_values = [0, 1, 10, 12, 0, 1, 10, 11].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 4,
        values: &invalid_stage_values,
    }];
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
    assert!(invalid_results[..3]
        .iter()
        .all(|result| result.invalid_rows.is_empty()));
    assert_eq!(invalid_results[3].invalid_rows.len(), 1);
    assert_eq!(invalid_results[3].invalid_rows[0].row, 0);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_keeps_large_static_value_tables_responsive() {
    let dir = temp_dir("large-static-value-table");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(&source_path, static_value_table_source(1500, 128));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(8),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 128);
    assert!(expressions.hints.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_keeps_many_static_for_loops_responsive() {
    let dir = temp_dir("many-static-for-loops");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(&source_path, many_static_for_loops_source(300));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(8),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 300);
    assert!(expressions.hints.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_keeps_many_static_if_statements_responsive() {
    let dir = temp_dir("many-static-if-statements");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(&source_path, many_static_if_statements_source(300));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(6),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 300);
    assert!(expressions.hints.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_reuses_nested_static_for_loop_bodies() {
    let dir = temp_dir("nested-static-for-loop-bodies");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(&source_path, nested_zero_static_for_loop_source(350, 500));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
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
    let elapsed = started.elapsed();

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(
        elapsed < Duration::from_secs(8),
        "source setup took {elapsed:?}"
    );
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert!(expressions.constraints.is_empty());
    assert!(expressions.hints.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_source_else_constraints() {
    let dir = temp_dir("static-else-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[2];\n\
             if (0) {\n\
                 values[0] * (1 - values[0]) === 0;\n\
             } else {\n\
                 values[1] * (1 - values[1]) === 0;\n\
             }\n\
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
    assert!(constraint_uses_commitment_element(
        &expressions.constraints[0],
        1
    ));

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [2, 0, 3, 1].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &stage_values,
    }];
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

    let invalid_stage_values = [0, 1, 1, 2].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &invalid_stage_values,
    }];
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
fn generate_key_unrolls_static_source_for_loop_row_offsets() {
    let dir = temp_dir("static-for-loop-row-offset");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[2];\n\
             for (int index = 0; index < 2; ++index) {\n\
                 values[index]' === values[index];\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [0, 0, 0, 0];",
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 2);
    assert!(expressions.hints.is_empty());
    for (element, constraint) in expressions.constraints.iter().enumerate() {
        assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
        assert_eq!(constraint.offset_min, Some(0));
        assert_eq!(constraint.offset_max, Some(1));
        assert!(constraint_uses_next_commitment_element(
            constraint,
            u32::try_from(element).expect("element should fit")
        ));
    }

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 2);
    assert!(regular.hints.hints.is_empty());

    let stage_values = [5, 9, 5, 9, 5, 9, 5, 9].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &stage_values,
    }];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
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

fn constraint_uses_commitment_element(
    constraint: &lzvm_artifacts::expression_info::ConstraintCode,
    element: u32,
) -> bool {
    constraint
        .operations
        .iter()
        .flat_map(|operation| operation.sources.iter())
        .any(|source| {
            matches!(
                source,
                CodeOperand::CommitmentElement {
                    id: 0,
                    element: found,
                    prime: None,
                    dimension: 1,
                } if *found == element
            )
        })
}

fn constraint_uses_next_commitment_element(
    constraint: &lzvm_artifacts::expression_info::ConstraintCode,
    element: u32,
) -> bool {
    constraint
        .operations
        .iter()
        .flat_map(|operation| operation.sources.iter())
        .any(|source| {
            matches!(
                source,
                CodeOperand::CommitmentElement {
                    id: 0,
                    element: found,
                    prime: Some(1),
                    dimension: 1,
                } if *found == element
            )
        })
}

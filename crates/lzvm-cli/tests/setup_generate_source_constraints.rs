use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{
    read_expression_info_binary_file, BoundaryKind, CodeOperand, OperationKind,
};
use lzvm_artifacts::fixed::parse_raw_fixed_columns;
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;
use lzvm_field::{Felt, MODULUS};
use lzvm_prover::regular_constraints::{
    evaluate_regular_constraints, RegularColumnMatrix, RegularConstraintInputs, RegularStageColumns,
};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-constraints-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_source_air_value_boolean_constraints() {
    let dir = temp_dir("air-value-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             airval flag;\n\
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
    assert!(matches!(
        expressions.constraints[0].operations[0].sources[1],
        CodeOperand::AirValue {
            id: 0,
            dimension: 1,
            ..
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_bare_expression_constraints() {
    let dir = temp_dir("bare-expression-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness flag;\n\
             flag * (1 - flag);\n\
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
    assert_eq!(operations[0].op, OperationKind::Sub);
    assert_eq!(operations[1].op, OperationKind::Mul);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());

    let valid_stage_values = [0, 1].map(Felt::from_u64);
    let valid_stage_columns = [RegularStageColumns::from_host_values(
        1,
        1,
        &valid_stage_values,
    )];
    let valid_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &valid_stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(valid_results[0].invalid_rows.is_empty());

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
    .expect("regular constraints should evaluate");
    assert_eq!(invalid_results[0].invalid_rows.len(), 1);
    assert_eq!(invalid_results[0].invalid_rows[0].row, 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_fixed_column_boolean_constraints() {
    let dir = temp_dir("fixed-column-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value * gate === 0;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed gate = [1, 0];",
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
    assert!(matches!(
        expressions.constraints[0].operations[0].sources[1],
        CodeOperand::Constant {
            id: 0,
            dimension: 1
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_public_value_constraints() {
    let dir = temp_dir("public-value-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public input;\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value === input;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed rows = [0, 0];",
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
    assert_eq!(setup.n_publics, Some(1));
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: None,
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Public {
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
    let stage_values = [7, 8].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let publics = [Felt::from_u64(7)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            publics: &publics,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_public_array_element_constraints() {
    let dir = temp_dir("public-array-element-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public inputs[4];\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value === inputs[2];\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed rows = [0, 0];",
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
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Public {
            id: 2,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let stage_values = [7, 8].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let publics = [3, 5, 7, 11].map(Felt::from_u64);
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            publics: &publics,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_public_scalar_after_array_constraints() {
    let dir = temp_dir("public-scalar-after-array-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public prefix[2];\n\
         public expected;\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value === expected;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed rows = [0, 0];",
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
    assert_eq!(setup.n_publics, Some(3));
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Public {
            id: 2,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let stage_values = [7, 8].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let publics = [3, 5, 7].map(Felt::from_u64);
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            publics: &publics,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_signed_literal_constraints() {
    let dir = temp_dir("signed-literal-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value === -1;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [0, 0];",
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
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: None,
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Number {
            value,
            dimension: 1,
        } if value == MODULUS - 1
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let stage_values = [MODULUS - 1, MODULUS - 1].map(Felt::from_u64);
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_scalar_zero_constraints() {
    let dir = temp_dir("scalar-zero-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value === 0;\n\
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
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Copy);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
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
    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_static_divisor_constraints() {
    let dir = temp_dir("static-divisor-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value / 2 === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [3, 5];",
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
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 2);
    assert_eq!(constraint.operations[0].op, OperationKind::Mul);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: None,
            dimension: 1,
        }
    ));
    let inv_two = Felt::from_u64(2)
        .inverse()
        .expect("nonzero divisor should invert")
        .to_u64();
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Number {
            value,
            dimension: 1,
        } if value == inv_two
    ));
    assert_eq!(constraint.operations[1].op, OperationKind::Sub);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    let fixed_values = [3, 5].map(Felt::from_u64);
    let stage_values = [6, 10].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_source_static_exponent_constraints() {
    let dir = temp_dir("static-exponent-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value ** 3 === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [8, 27];",
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
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 3);
    assert_eq!(constraint.operations[0].op, OperationKind::Mul);
    assert_eq!(constraint.operations[1].op, OperationKind::Mul);
    assert_eq!(constraint.operations[2].op, OperationKind::Sub);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    let fixed_values = [8, 27].map(Felt::from_u64);
    let stage_values = [2, 3].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_source_constant_scalar_constraints() {
    let dir = temp_dir("constant-scalar-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int LIMIT = 6;\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value === LIMIT + 1;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [0, 0];",
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
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: None,
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
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
    let stage_values = [7, 7].map(Felt::from_u64);
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_template_constant_scalar_constraints() {
    let dir = temp_dir("template-constant-scalar-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int LIMIT = 6) {\n\
             col witness value;\n\
             value === LIMIT + 1;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [0, 0];",
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
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[1],
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
    let stage_values = [7, 7].map(Felt::from_u64);
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_template_parameter_static_if_constraints() {
    let dir = temp_dir("template-param-static-if-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int ENABLED = 1) {\n\
             col witness value;\n\
             if (ENABLED) {\n\
                 value === 5;\n\
             } else {\n\
                 value === 11;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(ENABLED: 0); }\n\
         col fixed check = [0, 0];",
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
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Number {
            value: 11,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let stage_values = [11, 11].map(Felt::from_u64);
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_template_parameter_static_if_opening_points() {
    let dir = temp_dir("template-param-static-if-opening-point");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int ENABLED = 1) {\n\
             col witness value;\n\
             if (ENABLED) {\n\
                 value === 5;\n\
             } else {\n\
                 value' === 11;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(ENABLED: 0); }\n\
         col fixed check = [0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.offset_min, Some(0));
    assert_eq!(constraint.offset_max, Some(1));
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(1),
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Number {
            value: 11,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let stage_values = [11, 11, 11, 11].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
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
    assert!(results[0].invalid_rows.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_constraint_expression_aliases() {
    let dir = temp_dir("constraint-expression-alias");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             const expr delayed = value';\n\
             value * (value - delayed) === 0;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [0, 0, 0, 0];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
    assert_eq!(constraint.offset_min, Some(0));
    assert_eq!(constraint.offset_max, Some(1));
    assert_eq!(constraint.operations.len(), 2);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(1),
            dimension: 1,
        }
    ));
    assert_eq!(constraint.operations[1].op, OperationKind::Mul);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert_eq!(regular.constraints.entries[0].first_row, 0);
    assert_eq!(regular.constraints.entries[0].last_row, 3);
    let stage_values = [2, 2, 2, 2].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
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
    assert!(results[0].invalid_rows.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_constraint_expression_array_aliases() {
    let dir = temp_dir("constraint-expression-array-alias");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             const expr items[] = [value, value'];\n\
             items[1] === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [2, 3, 4, 8];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
    assert_eq!(constraint.offset_min, Some(0));
    assert_eq!(constraint.offset_max, Some(1));
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(1),
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Constant {
            id: 0,
            dimension: 1
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert_eq!(regular.constraints.entries[0].first_row, 0);
    assert_eq!(regular.constraints.entries[0].last_row, 3);
    let fixed_columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    let fixed_values = fixed_columns.columns[0]
        .values
        .iter()
        .copied()
        .map(Felt::from_u64)
        .collect::<Vec<_>>();
    let stage_values = [9, 2, 3, 4].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_source_row_offset_expression_alias_targets() {
    let dir = temp_dir("row-offset-expression-alias-target");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             const expr current = value;\n\
             current' === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [2, 3, 4, 8];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
    assert_eq!(constraint.offset_min, Some(0));
    assert_eq!(constraint.offset_max, Some(1));
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(1),
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Constant {
            id: 0,
            dimension: 1
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert_eq!(regular.constraints.entries[0].first_row, 0);
    assert_eq!(regular.constraints.entries[0].last_row, 3);
    let fixed_columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    let fixed_values = fixed_columns.columns[0]
        .values
        .iter()
        .copied()
        .map(Felt::from_u64)
        .collect::<Vec<_>>();
    let stage_values = [9, 2, 3, 4].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_source_row_offset_expression_targets() {
    let dir = temp_dir("row-offset-expression-target");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             const expr shifted = value + 1;\n\
             shifted' === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [3, 4, 5, 8];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
    assert_eq!(constraint.offset_min, Some(0));
    assert_eq!(constraint.offset_max, Some(1));
    assert_eq!(constraint.operations.len(), 2);
    assert_eq!(constraint.operations[0].op, OperationKind::Add);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(1),
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Number {
            value: 1,
            dimension: 1
        }
    ));
    assert_eq!(constraint.operations[1].op, OperationKind::Sub);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert_eq!(regular.constraints.entries[0].first_row, 0);
    assert_eq!(regular.constraints.entries[0].last_row, 3);
    let fixed_columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    let fixed_values = fixed_columns.columns[0]
        .values
        .iter()
        .copied()
        .map(Felt::from_u64)
        .collect::<Vec<_>>();
    let stage_values = [9, 2, 3, 4].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_source_next_row_constraints() {
    let dir = temp_dir("next-row-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value' === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [2, 3, 4, 8];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
    assert_eq!(constraint.offset_min, Some(0));
    assert_eq!(constraint.offset_max, Some(1));
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(1),
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Constant {
            id: 0,
            dimension: 1
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert_eq!(regular.constraints.entries[0].first_row, 0);
    assert_eq!(regular.constraints.entries[0].last_row, 3);
    let fixed_columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    let fixed_values = fixed_columns.columns[0]
        .values
        .iter()
        .copied()
        .map(Felt::from_u64)
        .collect::<Vec<_>>();
    let stage_values = [9, 2, 3, 4].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: setup.n_stages as u16,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_static_function_row_offset_constraints() {
    let dir = temp_dir("function-row-offset-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function advance(): int {\n\
             return 1;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value'(advance()) === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [2, 3, 4, 8];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
    assert_eq!(constraint.offset_min, Some(0));
    assert_eq!(constraint.offset_max, Some(1));
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(1),
            dimension: 1,
        }
    ));

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let fixed_values = [2, 3, 4, 8].map(Felt::from_u64);
    let stage_values = [1, 2, 3, 4].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_boolean_expression_row_offset_constraints() {
    let dir = temp_dir("boolean-expression-row-offset-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int SELECTED = 1;\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value'(SELECTED == 1) === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [2, 3, 4, 8];",
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
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
    assert_eq!(constraint.offset_min, Some(0));
    assert_eq!(constraint.offset_max, Some(1));
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(1),
            dimension: 1,
        }
    ));

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let fixed_values = [2, 3, 4, 8].map(Felt::from_u64);
    let stage_values = [1, 2, 3, 4].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_source_prior_row_constraints() {
    let dir = temp_dir("prior-row-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             'value === check;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [8, 9, 2, 3];",
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
    assert_eq!(setup.opening_points, [0, -1]);
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.boundary, BoundaryKind::EveryFrame);
    assert_eq!(constraint.offset_min, Some(1));
    assert_eq!(constraint.offset_max, Some(0));
    assert_eq!(constraint.operations.len(), 1);
    assert_eq!(constraint.operations[0].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: Some(-1),
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Constant {
            id: 0,
            dimension: 1
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert_eq!(regular.constraints.entries[0].first_row, 1);
    assert_eq!(regular.constraints.entries[0].last_row, 4);
    let fixed_columns = parse_raw_fixed_columns(
        &fs::read(&unit.fixed_columns).expect("fixed columns should read"),
        &setup,
        unit.group_name.as_deref().unwrap_or("raw"),
        unit.unit_name.as_deref().unwrap_or("unit"),
    )
    .expect("fixed columns should parse");
    let fixed_values = fixed_columns.columns[0]
        .values
        .iter()
        .copied()
        .map(Felt::from_u64)
        .collect::<Vec<_>>();
    let stage_values = [9, 2, 3, 4].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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

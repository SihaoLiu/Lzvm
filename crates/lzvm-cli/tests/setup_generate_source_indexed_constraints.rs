use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{
    read_expression_info_binary_file, CodeOperand, OperationKind,
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
        "lzvm-cli-setup-generate-source-indexed-constraints-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_source_indexed_witness_constraints() {
    let dir = temp_dir("indexed-witness-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[1] === values[0] + 1;\n\
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
    assert_eq!(constraint.operations.len(), 2);
    assert_eq!(constraint.operations[0].op, OperationKind::Add);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::CommitmentElement {
            id: 0,
            element: 0,
            prime: None,
            dimension: 1,
        }
    ));
    assert_eq!(constraint.operations[1].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[1].sources[0],
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

    let stage_values = [4, 5, 6, 7].map(Felt::from_u64);
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

    let invalid_stage_values = [4, 5, 6, 8].map(Felt::from_u64);
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
fn generate_key_lowers_source_boolean_index_witness_constraints() {
    let dir = temp_dir("boolean-indexed-witness-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int SELECTED = 1;\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
             values[SELECTED == 1] === values[0] + 1;\n\
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
    assert_eq!(constraint.operations.len(), 2);
    assert_eq!(constraint.operations[0].op, OperationKind::Add);
    assert_eq!(constraint.operations[1].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[1].sources[0],
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

    let stage_values = [4, 5, 6, 7].map(Felt::from_u64);
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

    let invalid_stage_values = [4, 5, 6, 8].map(Felt::from_u64);
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

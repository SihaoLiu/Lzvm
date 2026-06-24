use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{
    read_expression_info_binary_file, CodeDestination, CodeOperand, OperationKind,
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
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-setup-generate-source-proof-value-constraints-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_source_stage_one_proof_value_constraints() {
    let dir = temp_dir("stage-one-proof-value");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval expected;\n\
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
        constraint.operations[0].destination,
        CodeDestination::Temporary {
            id: 0,
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::ProofValue {
            id: 0,
            stage: Some(1),
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
    let proof_values = [Felt::from_u64(7)];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            proof_values: &proof_values,
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
fn generate_key_lowers_source_later_stage_proof_value_constraints() {
    let dir = temp_dir("later-stage-proof-value");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval prefix;\n\
         proofval stage(2) expected;\n\
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
        constraint.operations[0].destination,
        CodeDestination::Temporary {
            id: 0,
            dimension: 3,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::ProofValue {
            id: 1,
            stage: Some(2),
            dimension: 3,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let stage_values = [7, 8].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let proof_values = [3, 7, 0, 0].map(Felt::from_u64);
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            proof_values: &proof_values,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 1);

    let extension_proof_values = [3, 7, 1, 0].map(Felt::from_u64);
    let extension_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            proof_values: &extension_proof_values,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate extension values");
    assert_eq!(extension_results[0].invalid_rows.len(), 2);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_multidimensional_source_proof_value_constraints() {
    let dir = temp_dir("multidimensional-proof-value");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval stage(2) expected[2][2];\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value === expected[1][0];\n\
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
        CodeOperand::ProofValue {
            id: 6,
            stage: Some(2),
            dimension: 3,
        }
    ));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

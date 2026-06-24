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
            "lzvm-cli-setup-generate-source-air-value-constraints-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_source_later_stage_air_value_constraints() {
    let dir = temp_dir("later-stage-air-value");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airval stage(2) expected;\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value === expected;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col witness stage(2) aux.later;\n\
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
    assert_eq!(setup.unit_value_map.len(), 1);
    assert_eq!(setup.unit_value_map[0].name, "expected");
    assert_eq!(setup.unit_value_map[0].stage, 2);
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
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: None,
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::AirValue {
            id: 0,
            stage: Some(2),
            air_group_id: None,
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
    let unit_values = [7, 0, 0].map(Felt::from_u64);
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            unit_values: &unit_values,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 1);

    let extension_unit_values = [7, 1, 0].map(Felt::from_u64);
    let extension_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            unit_values: &extension_unit_values,
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
fn generate_key_lowers_indexed_source_air_value_constraints() {
    let dir = temp_dir("indexed-air-value");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airval stage(2) expected[2];\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value === expected[1];\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col witness stage(2) aux.later;\n\
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
    assert_eq!(setup.unit_value_map.len(), 1);
    assert_eq!(setup.unit_value_map[0].name, "expected");
    assert_eq!(setup.unit_value_map[0].stage, 2);
    assert_eq!(setup.unit_value_map[0].lengths, [2]);
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(matches!(
        expressions.constraints[0].operations[0].sources[1],
        CodeOperand::AirValue {
            id: 1,
            stage: Some(2),
            air_group_id: None,
            dimension: 3,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    let stage_values = [22, 23].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 1, &stage_values)];
    let unit_values = [11, 0, 0, 22, 0, 0].map(Felt::from_u64);
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            unit_values: &unit_values,
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
fn generate_key_lowers_multidimensional_source_air_value_constraints() {
    let dir = temp_dir("multidimensional-air-value");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airval stage(2) expected[2][2];\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             value === expected[1][0];\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col witness stage(2) aux.later;\n\
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
    assert_eq!(setup.unit_value_map.len(), 1);
    assert_eq!(setup.unit_value_map[0].name, "expected");
    assert_eq!(setup.unit_value_map[0].stage, 2);
    assert_eq!(setup.unit_value_map[0].lengths, [2, 2]);
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(matches!(
        expressions.constraints[0].operations[0].sources[1],
        CodeOperand::AirValue {
            id: 2,
            stage: Some(2),
            air_group_id: None,
            dimension: 3,
        }
    ));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_template_parameter_static_if_air_values() {
    let dir = temp_dir("template-param-static-if-air-value");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int ENABLED = 1) {\n\
             col witness value;\n\
             if (ENABLED) {\n\
                 airval stage(2) unused;\n\
                 value === 0;\n\
             } else {\n\
                 airval stage(2) expected;\n\
                 value === expected;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(ENABLED: 0); }\n\
         col witness stage(2) aux.later;\n\
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
    assert_eq!(setup.unit_value_map.len(), 1);
    assert_eq!(setup.unit_value_map[0].name, "expected");
    assert_eq!(setup.unit_value_map[0].stage, 2);
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
        CodeOperand::AirValue {
            id: 0,
            stage: Some(2),
            air_group_id: None,
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
    let unit_values = [7, 0, 0].map(Felt::from_u64);
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            unit_values: &unit_values,
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

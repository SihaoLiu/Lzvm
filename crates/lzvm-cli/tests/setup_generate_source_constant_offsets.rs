use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{read_expression_info_binary_file, BoundaryKind};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;
use lzvm_field::Felt;
use lzvm_prover::regular_constraints::{
    evaluate_regular_constraints, RegularColumnMatrix, RegularConstraintInputs, RegularStageColumns,
};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-constant-offsets-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_source_fixed_row_offset_constraints() {
    let dir = temp_dir("fixed-row-offset-constraint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             value === check';\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed check = [5, 7, 11, 13];",
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

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert_eq!(regular.constraints.entries[0].first_row, 0);
    assert_eq!(regular.constraints.entries[0].last_row, 3);

    let fixed_values = [5, 7, 11, 13].map(Felt::from_u64);
    let stage_values = [7, 11, 13, 99].map(Felt::from_u64);
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

    let invalid_stage_values = [7, 12, 13, 99].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns::from_host_values(
        1,
        1,
        &invalid_stage_values,
    )];
    let invalid_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed_values,
            },
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
fn generate_key_lowers_source_fixed_prior_expression_offset() {
    let dir = temp_dir("fixed-prior-expression-offset");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA(const int N = 32) {\n\
             const int CLOCKS = 20;\n\
             col witness first;\n\
             col fixed first_check = [0...];\n\
             first === first;\n\
         }\n\
         airtemplate UnitB(const int N = 32) {\n\
             const int CLOCKS = 24;\n\
             col witness value;\n\
             col fixed check = [[1, 0:(CLOCKS-1)]:1, 0...];\n\
             const expr last = (CLOCKS-1)'check;\n\
             value * last === 0;\n\
         }\n\
         airgroup GroupA { UnitA(); UnitB(); }",
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
    let unit = layout
        .units
        .iter()
        .find(|unit| unit.unit_name.as_deref() == Some("UnitB"))
        .expect("UnitB should be present");
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert!(setup.opening_points.contains(&-23));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

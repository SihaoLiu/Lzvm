use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{read_expression_info_binary_file, CodeOperand};
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
            "lzvm-cli-setup-generate-source-predeclared-for-loop-constraints-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_preserves_predeclared_index_after_static_for_loop_constraints() {
    let dir = temp_dir("static-for-loop-predeclared-index-after-loop");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[4];\n\
             int index = 0;\n\
             for (index = 0; index < 3; ++index) {\n\
                 values[index] === index;\n\
             }\n\
             values[index] === 3;\n\
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

    let stage_values = [0, 1, 2, 3, 0, 1, 2, 3].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns::from_host_values(1, 4, &stage_values)];
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

    let invalid_stage_values = [0, 1, 2, 4, 0, 1, 2, 3].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns::from_host_values(
        1,
        4,
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

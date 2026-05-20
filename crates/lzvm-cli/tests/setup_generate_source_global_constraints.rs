use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::global_program::read_global_program_file;
use lzvm_cli::run_cli;
use lzvm_field::{Ext3, Felt};
use lzvm_prover::global_constraints::{evaluate_global_constraints, GlobalConstraintInputs};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-global-constraints-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_later_stage_proof_value_boolean_global_constraints() {
    let dir = temp_dir("later-stage-proof-value-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval stage(2) extension_flag;\n\
         extension_flag * (1 - extension_flag);\n\
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
    let program = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(program.constraints.entries.len(), 1);
    assert_eq!(program.constraints.entries[0].destination_dimension, 3);

    let satisfied_zero = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::ZERO, Felt::ZERO, Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("zero proof value should evaluate");
    assert_eq!(satisfied_zero, [Ext3::ZERO]);

    let satisfied_one = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::ONE, Felt::ZERO, Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("one proof value should evaluate");
    assert_eq!(satisfied_one, [Ext3::ZERO]);

    let unsatisfied_base = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::from_u64(2), Felt::ZERO, Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("non-boolean base component should evaluate");
    assert_ne!(unsatisfied_base, [Ext3::ZERO]);

    let unsatisfied_extension = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::ONE, Felt::from_u64(3), Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("nonzero extension component should evaluate");
    assert_ne!(unsatisfied_extension, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_public_value_boolean_global_constraints() {
    let dir = temp_dir("public-value-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public enabled;\n\
         enabled * (1 - enabled);\n\
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
    let program = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(program.constraints.entries.len(), 1);
    assert_eq!(program.constraints.entries[0].destination_dimension, 1);

    let satisfied_zero = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("zero public value should evaluate");
    assert_eq!(satisfied_zero, [Ext3::ZERO]);

    let satisfied_one = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("one public value should evaluate");
    assert_eq!(satisfied_one, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("non-boolean public value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_public_array_element_boolean_global_constraints() {
    let dir = temp_dir("public-array-element-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flags[2];\n\
         flags[1] * (1 - flags[1]);\n\
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
    let program = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(program.constraints.entries.len(), 1);
    assert_eq!(program.constraints.entries[0].destination_dimension, 1);

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(7), Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected public element should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ONE, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected non-boolean public element should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_public_static_index_boolean_global_constraints() {
    let dir = temp_dir("public-static-index-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flags[2];\n\
         const int SELECTED = 1;\n\
         flags[SELECTED] * (1 - flags[SELECTED]);\n\
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
    let program = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(program.constraints.entries.len(), 1);
    assert_eq!(program.constraints.entries[0].destination_dimension, 1);

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(7), Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected public element should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ONE, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected non-boolean public element should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_public_value_alias_boolean_global_constraints() {
    let dir = temp_dir("public-value-alias-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flags[2];\n\
         const expr selected = flags[1];\n\
         selected * (1 - selected);\n\
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
    let program = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(program.constraints.entries.len(), 1);
    assert_eq!(program.constraints.entries[0].destination_dimension, 1);

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(7), Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected public alias should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ONE, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected non-boolean public alias should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_public_expr_array_alias_boolean_global_constraints() {
    let dir = temp_dir("public-expr-array-alias-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flags[2];\n\
         const expr selected[] = [flags[0], flags[1]];\n\
         selected[1] * (1 - selected[1]);\n\
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
    let program = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(program.constraints.entries.len(), 1);
    assert_eq!(program.constraints.entries[0].destination_dimension, 1);

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(7), Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected public alias should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ONE, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected non-boolean public alias should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_rejects_unindexed_public_array_boolean_global_constraints() {
    let dir = temp_dir("unindexed-public-array-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flags[2];\n\
         flags * (1 - flags);\n\
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

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("top-level public value constraints require scalar values"));
    assert!(!dir.join("pilout.globalInfo.bin").exists());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

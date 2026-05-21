use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::global_program::read_global_program_file;
use lzvm_cli::run_cli;
use lzvm_field::{Ext3, Felt};
use lzvm_prover::global_constraints::{evaluate_global_constraints, GlobalConstraintInputs};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-global-function-calls-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_source_function_expr_array_global_constraints() {
    let dir = temp_dir("source-function-expr-array");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_flags(expr values[], const int count) {\n\
             for (int index = 0; index < count; ++index) {\n\
                 values[index] * (1 - values[index]);\n\
             }\n\
         }\n\
         public flags[2];\n\
         constrain_flags(flags, 2);\n\
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
    assert_eq!(program.constraints.entries.len(), 2);
    assert!(program
        .constraints
        .entries
        .iter()
        .all(|entry| entry.destination_dimension == 1));

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("public flags should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO, Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("non-boolean public flag should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO, Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_nested_source_function_global_constraints() {
    let dir = temp_dir("nested-source-function");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function require_boolean(expr value) {\n\
             value * (1 - value);\n\
         }\n\
         function constrain_flag(expr value) {\n\
             require_boolean(value);\n\
         }\n\
         public flag;\n\
         constrain_flag(flag);\n\
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
            publics: &[Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("boolean public flag should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("non-boolean public flag should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_applies_source_function_static_updates_global_constraints() {
    let dir = temp_dir("source-function-static-updates");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_flags(expr values[]) {\n\
             int index = 0;\n\
             values[index] * (1 - values[index]);\n\
             index += 1;\n\
             values[index] * (1 - values[index]);\n\
         }\n\
         public flags[2];\n\
         constrain_flags(flags);\n\
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
    assert_eq!(program.constraints.entries.len(), 2);
    assert!(program
        .constraints
        .entries
        .iter()
        .all(|entry| entry.destination_dimension == 1));

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("public flags should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO, Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("non-boolean public flag should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO, Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_applies_source_function_static_assertions_global_constraints() {
    let dir = temp_dir("source-function-static-assertions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_flags(expr values[], const int count) {\n\
             assert(count == 2);\n\
             for (int index = 0; index < count; ++index) {\n\
                 values[index] * (1 - values[index]);\n\
             }\n\
         }\n\
         public flags[2];\n\
         constrain_flags(flags, 2);\n\
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
    assert_eq!(program.constraints.entries.len(), 2);
    assert!(program
        .constraints
        .entries
        .iter()
        .all(|entry| entry.destination_dimension == 1));

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("public flags should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO, Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("non-boolean public flag should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO, Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_applies_source_function_expr_array_length_assertions_global_constraints() {
    let dir = temp_dir("source-function-expr-array-length-assertions");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_flags(expr values[], const int count) {\n\
             assert(length(values) == count);\n\
             for (int index = 0; index < count; ++index) {\n\
                 values[index] * (1 - values[index]);\n\
             }\n\
         }\n\
         public flags[2];\n\
         constrain_flags(flags, 2);\n\
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
    assert_eq!(program.constraints.entries.len(), 2);
    assert!(program
        .constraints
        .entries
        .iter()
        .all(|entry| entry.destination_dimension == 1));

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("public flags should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO, Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("non-boolean public flag should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO, Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_rejects_source_function_expr_array_length_assertion_mismatch() {
    let dir = temp_dir("source-function-expr-array-length-assertion-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_flags(expr values[], const int count) {\n\
             assert(length(values) == count);\n\
         }\n\
         public flags[2];\n\
         constrain_flags(flags, 3);\n\
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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup key generation failed: source static assertion failed: assert(length(values) == count)\n"
    );
}

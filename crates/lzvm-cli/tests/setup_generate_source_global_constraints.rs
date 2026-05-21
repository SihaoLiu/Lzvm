use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::global_info::read_global_info_binary_file;
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
fn generate_key_lowers_indexed_proof_value_residual_global_constraints() {
    let dir = temp_dir("indexed-proof-value-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval stage(2) expected[2];\n\
         expected[1] - 3;\n\
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

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[
                Felt::from_u64(99),
                Felt::ZERO,
                Felt::ZERO,
                Felt::from_u64(3),
                Felt::ZERO,
                Felt::ZERO,
            ],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching indexed proof value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied_base = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[
                Felt::from_u64(3),
                Felt::ZERO,
                Felt::ZERO,
                Felt::from_u64(4),
                Felt::ZERO,
                Felt::ZERO,
            ],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched indexed proof value base component should evaluate");
    assert_ne!(unsatisfied_base, [Ext3::ZERO]);

    let unsatisfied_extension = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[
                Felt::from_u64(3),
                Felt::ZERO,
                Felt::ZERO,
                Felt::from_u64(3),
                Felt::ONE,
                Felt::ZERO,
            ],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("nonzero indexed proof value extension component should evaluate");
    assert_ne!(unsatisfied_extension, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_proof_value_static_backslash_global_constraints() {
    let dir = temp_dir("proof-value-static-backslash-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval stage(2) expected;\n\
         expected \\ 7 - 1;\n\
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

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::from_u64(7), Felt::ZERO, Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching proof value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::from_u64(8), Felt::ZERO, Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched proof value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_proof_value_static_exponent_global_constraints() {
    let dir = temp_dir("proof-value-static-exponent-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "proofval stage(2) expected;\n\
         expected ** 3 - 27;\n\
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

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::from_u64(3), Felt::ZERO, Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching proof value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            proof_values: &[Felt::from_u64(4), Felt::ZERO, Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched proof value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

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
fn generate_key_lowers_public_value_scalar_residual_global_constraints() {
    let dir = temp_dir("public-value-scalar-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public expected;\n\
         expected - 7;\n\
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
            publics: &[Felt::from_u64(7)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching public value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(8)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched public value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_public_value_static_divisor_global_constraints() {
    let dir = temp_dir("public-value-static-divisor-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public expected;\n\
         expected / 7 - 1;\n\
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
            publics: &[Felt::from_u64(7)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching public value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(8)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched public value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_public_value_static_backslash_global_constraints() {
    let dir = temp_dir("public-value-static-backslash-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public expected;\n\
         expected \\ 7 - 1;\n\
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
            publics: &[Felt::from_u64(7)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching public value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(8)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched public value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_public_value_static_exponent_global_constraints() {
    let dir = temp_dir("public-value-static-exponent-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public expected;\n\
         expected ** 3 - 27;\n\
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
            publics: &[Felt::from_u64(3)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching public value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(4)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched public value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_challenge_residual_global_constraints() {
    let dir = temp_dir("challenge-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "challenge stage(1) alpha;\n\
         alpha - 3;\n\
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

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            challenges: &[Ext3::new(Felt::from_u64(3), Felt::ZERO, Felt::ZERO)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching challenge value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied_base = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            challenges: &[Ext3::new(Felt::from_u64(4), Felt::ZERO, Felt::ZERO)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched challenge base component should evaluate");
    assert_ne!(unsatisfied_base, [Ext3::ZERO]);

    let unsatisfied_extension = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            challenges: &[Ext3::new(Felt::from_u64(3), Felt::ONE, Felt::ZERO)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("nonzero challenge extension component should evaluate");
    assert_ne!(unsatisfied_extension, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_group_value_residual_global_constraints() {
    let dir = temp_dir("group-value-residual");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airgroupval stage(2) aggregate(sum) group.total;\n\
         group.total - 3;\n\
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

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            group_values: &[Ext3::new(Felt::from_u64(3), Felt::ZERO, Felt::ZERO)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching group value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied_base = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            group_values: &[Ext3::new(Felt::from_u64(4), Felt::ZERO, Felt::ZERO)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched group value base component should evaluate");
    assert_ne!(unsatisfied_base, [Ext3::ZERO]);

    let unsatisfied_extension = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            group_values: &[Ext3::new(Felt::from_u64(3), Felt::ONE, Felt::ZERO)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("nonzero group value extension component should evaluate");
    assert_ne!(unsatisfied_extension, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_public_initializers_as_global_constraints() {
    let dir = temp_dir("static-public-initializer");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public expected = 6 + 1;\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.n_publics, 1);
    assert_eq!(global.publics_map.len(), 1);
    assert_eq!(global.publics_map[0].name, "expected");
    assert_eq!(global.publics_map[0].stage, 1);
    assert!(global.publics_map[0].lengths.is_empty());

    let program = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(program.constraints.entries.len(), 1);
    assert_eq!(program.constraints.entries[0].destination_dimension, 1);

    let satisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(7)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching public value should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(8)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched public value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_public_array_initializers_as_global_constraints() {
    let dir = temp_dir("static-public-array-initializer");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public expected[2] = [6 + 1, 9];\n\
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
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("source global metadata should parse");
    assert_eq!(global.n_publics, 2);
    assert_eq!(global.publics_map.len(), 1);
    assert_eq!(global.publics_map[0].name, "expected");
    assert_eq!(global.publics_map[0].lengths, [2]);

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
            publics: &[Felt::from_u64(7), Felt::from_u64(9)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("matching public values should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO, Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::from_u64(7), Felt::from_u64(10)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched public value should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO, Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_rejects_non_static_public_initializers() {
    let dir = temp_dir("non-static-public-initializer");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public expected = main.left[0];\n\
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

    assert_ne!(code, 0, "stdout={}", String::from_utf8_lossy(&stdout));
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("source public initializers must be static field values"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
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
fn generate_key_unrolls_static_for_loop_public_array_boolean_global_constraints() {
    let dir = temp_dir("static-for-public-array-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flags[2];\n\
         for (int i = 0; i < 2; ++i) {\n\
             flags[i] * (1 - flags[i]);\n\
         }\n\
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
    .expect("matching public flags should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO, Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ZERO, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("mismatched public flag should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO, Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_skips_static_for_loop_with_non_global_constraint_body() {
    let dir = temp_dir("static-for-non-global-constraint-body");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flags[2];\n\
         for (int i = 0; i < 2; ++i) {\n\
             flags[i] * (1 - flags[i]);\n\
             unknown(flags[i]);\n\
         }\n\
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
    assert_eq!(program.constraints.entries.len(), 0);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_static_if_public_value_boolean_global_constraints() {
    let dir = temp_dir("static-if-public-value-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int ENABLED = 1;\n\
         public flag;\n\
         if (ENABLED) {\n\
             flag * (1 - flag);\n\
         }\n\
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
fn generate_key_lowers_static_else_public_value_boolean_global_constraints() {
    let dir = temp_dir("static-else-public-value-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flag;\n\
         if (0) {\n\
             flag;\n\
         } else {\n\
             flag * (1 - flag);\n\
         }\n\
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
            publics: &[Felt::ZERO],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("zero public flag should evaluate");
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
fn generate_key_lowers_source_function_public_value_boolean_global_constraints() {
    let dir = temp_dir("source-function-public-value-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_flag(expr value) {\n\
             value * (1 - value);\n\
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
fn generate_key_lowers_public_static_one_boolean_global_constraints() {
    let dir = temp_dir("public-static-one-boolean");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public flags[2];\n\
         const int SELECTED = 1;\n\
         const int UNITY = 1;\n\
         flags[SELECTED] * (UNITY - flags[SELECTED]);\n\
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

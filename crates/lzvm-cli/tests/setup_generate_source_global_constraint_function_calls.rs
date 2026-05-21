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

use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::global_program::read_global_program_file;
use lzvm_cli::run_cli;
use lzvm_field::{Ext3, Felt};
use lzvm_prover::global_constraints::{evaluate_global_constraints, GlobalConstraintInputs};

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-setup-generate-source-global-for-bodies-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_static_for_body_static_if_source_function_global_constraints() {
    let dir = temp_dir("static-for-body-static-if-source-function");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "function constrain_flag(expr value) {\n\
             value * (1 - value);\n\
         }\n\
         public flags[2];\n\
         for (int index = 0; index < 2; ++index) {\n\
             if (index == 1) {\n\
                 constrain_flag(flags[index]);\n\
             }\n\
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
            publics: &[Felt::from_u64(3), Felt::ONE],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected public flag should evaluate");
    assert_eq!(satisfied, [Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program.constraints,
        GlobalConstraintInputs {
            publics: &[Felt::ONE, Felt::from_u64(2)],
            ..GlobalConstraintInputs::default()
        },
    )
    .expect("selected non-boolean public flag should evaluate");
    assert_ne!(unsatisfied, [Ext3::ZERO]);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

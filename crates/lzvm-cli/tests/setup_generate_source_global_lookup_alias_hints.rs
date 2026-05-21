use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::global_program::read_global_program_file;
use lzvm_artifacts::hint_program::{HintOperand, SOURCE_LOOKUP_PROVES_HINT};
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-global-lookup-alias-hints-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_expands_airgroup_expression_array_lookup_aliases() {
    let dir = temp_dir("expression-array");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public inputs[4];\n\
         const int BUS_ID = 9;\n\
         const int LABEL = 17;\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA {\n\
             UnitA();\n\
             for (int i = 0; i < 2; ++i) {\n\
                 const expr tuple[] = [i, inputs[i * 2], inputs[i * 2 + 1]];\n\
                 direct_global_update_proves(BUS_ID, [...tuple], surname: LABEL);\n\
             }\n\
         }\n\
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
    let global = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(global.hints.hints.len(), 2);

    assert_eq!(global.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(global.hints.hints[0].fields[1].name, "values");
    assert_eq!(
        global.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Number(0)
    );
    assert_eq!(
        global.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Public { id: 0 }
    );
    assert_eq!(
        global.hints.hints[0].fields[1].values[2].operand,
        HintOperand::Public { id: 1 }
    );
    assert_eq!(global.hints.hints[0].fields[2].name, "surname");
    assert_eq!(
        global.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(17)
    );

    assert_eq!(
        global.hints.hints[1].fields[1].values[0].operand,
        HintOperand::Number(1)
    );
    assert_eq!(
        global.hints.hints[1].fields[1].values[1].operand,
        HintOperand::Public { id: 2 }
    );
    assert_eq!(
        global.hints.hints[1].fields[1].values[2].operand,
        HintOperand::Public { id: 3 }
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_expands_airgroup_lookup_helper_calls() {
    let dir = temp_dir("helper-call");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public inputs[2];\n\
         const int BUS_ID = 9;\n\
         const int LABEL = 17;\n\
         function emit_update(expr tuple[]) {\n\
             direct_global_update_proves(BUS_ID, [...tuple], surname: LABEL);\n\
         }\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA {\n\
             UnitA();\n\
             emit_update([inputs[0], inputs[1]]);\n\
         }\n\
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
    let global = read_global_program_file(dir.join("pilout.globalConstraints.bin"))
        .expect("source global program should parse");
    assert_eq!(global.hints.hints.len(), 1);

    assert_eq!(global.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(global.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(
        global.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(9)
    );
    assert_eq!(global.hints.hints[0].fields[1].name, "values");
    assert_eq!(
        global.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Public { id: 0 }
    );
    assert_eq!(
        global.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Public { id: 1 }
    );
    assert_eq!(global.hints.hints[0].fields[2].name, "surname");
    assert_eq!(
        global.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(17)
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_rejects_airgroup_lookup_helper_static_assertion_mismatch() {
    let dir = temp_dir("helper-static-assertion");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public inputs[1];\n\
         const int BUS_ID = 9;\n\
         const int LABEL = 17;\n\
         function emit_update(expr tuple[]) {\n\
             assert(length(tuple) == 2);\n\
             direct_global_update_proves(BUS_ID, [...tuple], surname: LABEL);\n\
         }\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA {\n\
             UnitA();\n\
             emit_update([inputs[0]]);\n\
         }\n\
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
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup key generation failed: source static assertion failed: assert(length(tuple) == 2)\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

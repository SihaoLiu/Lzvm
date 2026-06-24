use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::global_program::read_global_program_file;
use lzvm_artifacts::hint_program::HintOperand;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-setup-generate-source-global-static-while-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_lowers_static_while_airgroup_annotations_to_global_hints() {
    let dir = temp_dir("airgroup-annotation-static-while");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "public inputs[3];\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA {\n\
             UnitA();\n\
             int index = 0;\n\
             while (index < 3) {\n\
                 @record {left: inputs[index], literal: index}\n\
                 index++;\n\
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
    assert_eq!(global.hints.hints.len(), 3);
    for (index, hint) in global.hints.hints.iter().enumerate() {
        assert_eq!(hint.name, "record");
        assert_eq!(hint.fields[0].name, "left");
        assert_eq!(
            hint.fields[0].values[0].operand,
            HintOperand::Public { id: index as u32 }
        );
        assert_eq!(hint.fields[1].name, "literal");
        assert_eq!(
            hint.fields[1].values[0].operand,
            HintOperand::Number(index as u64)
        );
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

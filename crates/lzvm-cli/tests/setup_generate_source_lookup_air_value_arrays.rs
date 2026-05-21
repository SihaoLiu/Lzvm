use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::hint_program::{HintOperand, SOURCE_LOOKUP_PROVES_HINT};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-lookup-air-value-arrays-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_expands_air_value_array_source_lookup_values() {
    let dir = temp_dir("air-value-array-lookup");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airval stage(2) expected[2];\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             lookup_proves(9, [...expected], mul: selector);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col witness stage(2) aux.later;\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);

    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::AirValue { id: 0 }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::AirValue { id: 1 }
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_expands_air_group_value_array_source_lookup_values() {
    let dir = temp_dir("air-group-value-array-lookup");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airgroupval stage(2) aggregate(sum) group.expected[2];\n\
         airtemplate UnitA() {\n\
             col witness selector;\n\
             lookup_proves(9, [...group.expected], mul: selector);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col witness stage(2) aux.later;\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);

    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::AirGroupValue { id: 0 }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::AirGroupValue { id: 1 }
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

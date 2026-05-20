use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::read_expression_info_binary_file;
use lzvm_artifacts::hint_program::{
    HintOperand, SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT,
};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-lookup-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn generate_key_records_source_lookup_calls_as_structured_regular_hints() {
    let dir = temp_dir("source-lookup-hints");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness multiplicity;\n\
             col witness value;\n\
             lookup_proves(7, [value], mul: multiplicity);\n\
             lookup_assumes(9, [value], sel: multiplicity);\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let expression_path = unit
        .expression_info_binary()
        .expect("expression metadata path should derive");
    let expressions = read_expression_info_binary_file(expression_path)
        .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 2);
    assert_eq!(expressions.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 3);
    assert_eq!(expressions.hints[0].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[0].fields[1].name, "values");
    assert_eq!(expressions.hints[0].fields[2].name, "multiplicity");
    assert_eq!(expressions.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(expressions.hints[1].fields.len(), 3);
    assert_eq!(expressions.hints[1].fields[0].name, "bus_id");
    assert_eq!(expressions.hints[1].fields[1].name, "values");
    assert_eq!(expressions.hints[1].fields[2].name, "selector");
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(7)
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::String("value".to_owned())
    );
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::String("multiplicity".to_owned())
    );
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(
        regular.hints.hints[1].fields[0].values[0].operand,
        HintOperand::Number(9)
    );
    assert_eq!(
        regular.hints.hints[1].fields[1].values[0].operand,
        HintOperand::String("value".to_owned())
    );
    assert_eq!(
        regular.hints.hints[1].fields[2].values[0].operand,
        HintOperand::String("multiplicity".to_owned())
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_resolves_static_source_lookup_bus_ids() {
    let dir = temp_dir("source-lookup-static-bus-ids");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "const int BUS_BASE = 7;\n\
         airtemplate UnitA(const int BUS_OFFSET = 4) {\n\
             col witness multiplicity;\n\
             col witness value;\n\
             const int LOCAL_BUS = BUS_BASE + BUS_OFFSET;\n\
             lookup_proves(LOCAL_BUS, [value], mul: multiplicity);\n\
             lookup_assumes(BUS_OFFSET + 6, [value], sel: multiplicity);\n\
         }\n\
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
    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[0].name, "bus_id");
    assert_eq!(
        regular.hints.hints[0].fields[0].values[0].operand,
        HintOperand::Number(11)
    );
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].fields[0].name, "bus_id");
    assert_eq!(
        regular.hints.hints[1].fields[0].values[0].operand,
        HintOperand::Number(10)
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

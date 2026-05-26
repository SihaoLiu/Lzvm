use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::hint_program::{
    HintOperand, SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT,
};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-lookup-value-expression-hints-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

fn sample_guest_image() -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x8000_0000_u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_trace_bytes(values: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn value_expression_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness base;\n\
         col witness delta;\n\
         col witness expected;\n\
         col witness weight;\n\
         lookup_proves(7, [base + delta, expected - delta], mul: weight);\n\
         lookup_assumes(7, [expected, base], sel: weight);\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn generate_key(source: &str, dir: &Path) -> (i32, Vec<u8>, Vec<u8>) {
    let source_path = dir.join("source").join("main.pil");
    write_file(&source_path, source);

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
    (code, stdout, stderr)
}

fn assert_generate_key_succeeds(dir: &Path, source: &str) {
    let (code, stdout, stderr) = generate_key(source, dir);
    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

fn run_witness(dir: &Path, trace_values: &[u64]) -> (i32, Vec<u8>, Vec<u8>) {
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(trace_values));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    (code, stdout, stderr)
}

#[test]
fn generate_key_lowers_source_lookup_value_expressions() {
    let dir = temp_dir("source-lookup-value-expression");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, value_expression_source());

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert!(
        regular.hints.hints[0].fields.len() >= 4,
        "expected structured lookup fields, got {:?}",
        regular.hints.hints[0].fields
    );
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 6);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[2].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[5].operand,
        HintOperand::String("sub".to_owned())
    );
    assert_eq!(regular.hints.hints[0].fields[2].name, "value_lengths");
    assert_eq!(
        regular.hints.hints[0].fields[2]
            .values
            .iter()
            .map(|value| &value.operand)
            .collect::<Vec<_>>(),
        vec![&HintOperand::Number(3), &HintOperand::Number(3)]
    );
    assert_eq!(regular.hints.hints[0].fields[3].name, "multiplicity");

    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].fields[1].name, "values");
    assert_eq!(regular.hints.hints[1].fields[1].values.len(), 2);
    assert!(regular.hints.hints[1]
        .fields
        .iter()
        .all(|field| field.name != "value_lengths"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn generate_key_lowers_accumulated_source_lookup_value_expression() {
    let dir = temp_dir("source-lookup-accumulated-value-expression");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(
        &dir,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             expr packed = 0;\n\
             packed += value;\n\
             packed += value';\n\
             lookup_proves(7, [packed]);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert!(
        regular.hints.hints[0].fields.len() >= 3,
        "expected structured lookup fields, got {:?}",
        regular.hints.hints[0].fields
    );
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 5);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Number(0)
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[2].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[3].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[4].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(regular.hints.hints[0].fields[2].name, "value_lengths");
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(5)
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn generate_key_lowers_static_while_lookup_value_expression_opening_points() {
    let dir = temp_dir("source-lookup-static-while-opening-points");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(
        &dir,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             int index = 0;\n\
             while (index < 1) {\n\
                 lookup_proves(7, [value']);\n\
                 ++index;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 1);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn generate_key_lowers_static_while_postfix_update_lookup_value_expression_opening_points() {
    let dir = temp_dir("source-lookup-static-while-postfix-opening-points");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(
        &dir,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             int index = 0;\n\
             while (index < 1) {\n\
                 lookup_proves(7, [value']);\n\
                 index++;\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 1);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn generate_key_lowers_static_switch_lookup_value_expression_opening_points() {
    let dir = temp_dir("source-lookup-static-switch-opening-points");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(
        &dir,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             const int port = 1;\n\
             switch (port) {\n\
                 case 0:\n\
                     lookup_proves(7, [value]);\n\
                     break;\n\
                 case 1:\n\
                     lookup_proves(7, [value']);\n\
                     break;\n\
                 default:\n\
                     lookup_proves(7, [value]);\n\
             }\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 1);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn generate_key_lowers_returned_source_lookup_value_expression() {
    let dir = temp_dir("source-lookup-returned-value-expression");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(
        &dir,
        "function pack(expr input): expr {\n\
             expr packed = 0;\n\
             packed += input;\n\
             packed += input';\n\
             return packed;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             const expr packed = pack(value);\n\
             lookup_proves(7, [packed]);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert!(
        regular.hints.hints[0].fields.len() >= 3,
        "expected structured lookup fields, got {:?}",
        regular.hints.hints[0].fields
    );
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 5);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Number(0)
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[2].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[3].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[4].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(regular.hints.hints[0].fields[2].name, "value_lengths");
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(5)
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn generate_key_lowers_returned_source_lookup_value_expression_array_loop() {
    let dir = temp_dir("source-lookup-returned-value-expression-array-loop");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(
        &dir,
        "function pack(const expr input[]): expr {\n\
             const int len = length(input);\n\
             expr packed = 0;\n\
             for (int j = 0; j < len; j++) {\n\
                 packed += input[j] * (j + 1);\n\
             }\n\
             return packed;\n\
         }\n\
         airtemplate UnitA() {\n\
             col witness value;\n\
             const expr packed = pack([value, value']);\n\
             lookup_proves(7, [packed]);\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    assert_eq!(setup.opening_points, vec![0, 1]);
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert!(
        regular.hints.hints[0].fields.len() >= 3,
        "expected structured lookup fields, got {:?}",
        regular.hints.hints[0].fields
    );
    assert_eq!(regular.hints.hints[0].fields[1].name, "values");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 9);
    assert_eq!(
        regular.hints.hints[0].fields[1].values[0].operand,
        HintOperand::Number(0)
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[1].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[2].operand,
        HintOperand::Number(1)
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[3].operand,
        HintOperand::String("mul".to_owned())
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[4].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[5].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 1
        }
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[6].operand,
        HintOperand::Number(2)
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[7].operand,
        HintOperand::String("mul".to_owned())
    );
    assert_eq!(
        regular.hints.hints[0].fields[1].values[8].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(regular.hints.hints[0].fields[2].name, "value_lengths");
    assert_eq!(
        regular.hints.hints[0].fields[2].values[0].operand,
        HintOperand::Number(9)
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_lookup_value_expressions() {
    let dir = temp_dir("source-lookup-value-expression-witness");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, value_expression_source());

    let (code, stdout, stderr) = run_witness(&dir, &[10, 1, 11, 3, 20, 2, 22, 4]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_source_lookup_value_expression_mismatch() {
    let dir = temp_dir("source-lookup-value-expression-mismatch");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, value_expression_source());

    let (code, stdout, stderr) = run_witness(&dir, &[10, 1, 12, 3, 20, 2, 22, 4]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("unbalanced lookup bus 7 tuple 11,11 has net weight 3"));
}

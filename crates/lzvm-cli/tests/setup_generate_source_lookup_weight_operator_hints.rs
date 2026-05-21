use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::hint_program::{
    HintOperand, SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT,
};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-lookup-weight-operator-hints-{}-{name}",
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

fn modulo_weight_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness value;\n\
         col witness left;\n\
         col witness right;\n\
         col witness selector;\n\
         lookup_proves(7, [value], mul: left % right);\n\
         lookup_assumes(7, [value], sel: selector);\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn comparison_weight_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness value;\n\
         col witness left;\n\
         col witness right;\n\
         col witness selector[6];\n\
         lookup_proves(7, [value], mul: left < right);\n\
         lookup_assumes(7, [value], sel: selector[0]);\n\
         lookup_proves(7, [value], mul: left <= right);\n\
         lookup_assumes(7, [value], sel: selector[1]);\n\
         lookup_proves(7, [value], mul: left > right);\n\
         lookup_assumes(7, [value], sel: selector[2]);\n\
         lookup_proves(7, [value], mul: left >= right);\n\
         lookup_assumes(7, [value], sel: selector[3]);\n\
         lookup_proves(7, [value], mul: left == right);\n\
         lookup_assumes(7, [value], sel: selector[4]);\n\
         lookup_proves(7, [value], mul: left != right);\n\
         lookup_assumes(7, [value], sel: selector[5]);\n\
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

fn assert_lookup_proves_weight_ops(dir: &Path, expected_ops: &[&str]) {
    let layout = read_key_directory_layout(dir).expect("layout should derive");
    let unit = &layout.units[0];
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    let ops = regular
        .hints
        .hints
        .iter()
        .filter(|hint| hint.name == SOURCE_LOOKUP_PROVES_HINT)
        .map(|hint| {
            let field = hint
                .fields
                .iter()
                .find(|field| field.name == "multiplicity")
                .expect("lookup proves should carry a multiplicity");
            match &field
                .values
                .last()
                .expect("multiplicity field should have a value")
                .operand
            {
                HintOperand::String(value) => value.as_str(),
                other => panic!("unexpected multiplicity tail operand: {other:?}"),
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(ops, expected_ops);
}

#[test]
fn generate_key_lowers_source_lookup_modulo_weight_expression() {
    let dir = temp_dir("source-lookup-modulo-weight");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, modulo_weight_source());

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");

    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[0].fields[2].name, "multiplicity");
    assert_eq!(regular.hints.hints[0].fields[2].values.len(), 3);
    assert_eq!(
        regular.hints.hints[0].fields[2].values[2].operand,
        HintOperand::String("mod".to_owned())
    );
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    assert_eq!(regular.hints.hints[1].fields[2].name, "selector");

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_lookup_modulo_weight_expression() {
    let dir = temp_dir("source-lookup-modulo-weight-witness");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, modulo_weight_source());

    let (code, stdout, stderr) = run_witness(&dir, &[11, 5, 3, 2, 12, 4, 2, 0]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_source_lookup_modulo_weight_mismatch() {
    let dir = temp_dir("source-lookup-modulo-weight-mismatch");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, modulo_weight_source());

    let (code, stdout, stderr) = run_witness(&dir, &[11, 5, 3, 1, 12, 4, 2, 0]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("unbalanced lookup bus 7 tuple 11 has net weight 1"));
}

#[test]
fn generate_key_lowers_source_lookup_comparison_weight_expressions() {
    let dir = temp_dir("source-lookup-comparison-weight");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, comparison_weight_source());
    assert_lookup_proves_weight_ops(&dir, &["lt", "le", "gt", "ge", "eq", "ne"]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_lookup_comparison_weight_expressions() {
    let dir = temp_dir("source-lookup-comparison-weight-witness");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, comparison_weight_source());

    let trace_values = &[11, 3, 5, 1, 1, 0, 0, 0, 1, 12, 4, 4, 0, 1, 0, 1, 1, 0];
    let (code, stdout, stderr) = run_witness(&dir, trace_values);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_source_lookup_comparison_weight_mismatch() {
    let dir = temp_dir("source-lookup-comparison-weight-mismatch");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, comparison_weight_source());

    let trace_values = &[11, 3, 5, 1, 1, 0, 0, 0, 0, 12, 4, 4, 0, 1, 0, 1, 1, 0];
    let (code, stdout, stderr) = run_witness(&dir, trace_values);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("unbalanced lookup bus 7 tuple 11 has net weight 1"));
}

use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{read_expression_info_binary_file, HintPayload};
use lzvm_artifacts::hint_program::{HintOperand, SOURCE_ASSIGNMENT_CHECK_HINT};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-setup-generate-source-assignment-operator-hints-{}-{name}",
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

fn assert_generated_assignment_ops(dir: &Path, expected_ops: &[&str]) {
    let layout = read_key_directory_layout(dir).expect("layout should derive");
    let unit = &layout.units[0];
    let expression_path = unit
        .expression_info_binary()
        .expect("expression metadata path should derive");
    let expressions = read_expression_info_binary_file(expression_path)
        .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), expected_ops.len());
    for (hint, op) in expressions.hints.iter().zip(expected_ops) {
        assert_eq!(hint.name, SOURCE_ASSIGNMENT_CHECK_HINT);
        assert_eq!(hint.fields.len(), 2);
        assert_eq!(hint.fields[0].name, "target");
        assert_eq!(hint.fields[1].name, "expression");
        assert_eq!(
            hint.fields[1]
                .values
                .last()
                .expect("expression field should have an operator")
                .payload,
            HintPayload::String {
                value: (*op).to_owned()
            }
        );
    }

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), expected_ops.len());
    for (hint, op) in regular.hints.hints.iter().zip(expected_ops) {
        assert_eq!(hint.name, SOURCE_ASSIGNMENT_CHECK_HINT);
        assert_eq!(hint.fields[1].name, "expression");
        assert_eq!(
            hint.fields[1]
                .values
                .last()
                .expect("expression field should have an operator")
                .operand,
            HintOperand::String((*op).to_owned())
        );
    }
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

fn assert_generate_key_succeeds(dir: &Path, source: &str) {
    let (code, stdout, stderr) = generate_key(source, dir);
    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

fn assert_witness_accepts(source_name: &str, source: &str, trace_values: &[u64]) {
    let dir = temp_dir(source_name);
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, source);

    let (code, stdout, stderr) = run_witness(&dir, trace_values);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

fn assert_witness_rejects(source_name: &str, source: &str, trace_values: &[u64]) {
    let dir = temp_dir(source_name);
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, source);

    let (code, stdout, stderr) = run_witness(&dir, trace_values);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("source assignment validation failed"));
}

fn modulo_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness value;\n\
         col witness out[1];\n\
         out[0] = value % 2;\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn comparison_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness value;\n\
         col witness limit;\n\
         col witness out[6];\n\
         out[0] = value < limit;\n\
         out[1] = value <= limit;\n\
         out[2] = value > limit;\n\
         out[3] = value >= limit;\n\
         out[4] = value == limit;\n\
         out[5] = value != limit;\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn bitwise_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness left;\n\
         col witness right;\n\
         col witness out[3];\n\
         out[0] = left & right;\n\
         out[1] = left ^ right;\n\
         out[2] = left | right;\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn shift_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness value;\n\
         col witness amount;\n\
         col witness out[2];\n\
         out[0] = value << amount;\n\
         out[1] = value >> amount;\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn logical_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness left;\n\
         col witness right;\n\
         col witness out[3];\n\
         out[0] = !left;\n\
         out[1] = left && right;\n\
         out[2] = left || right;\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn dynamic_power_source() -> &'static str {
    "airtemplate UnitA() {\n\
         col witness base;\n\
         col witness exponent;\n\
         col witness out[1];\n\
         out[0] = base ** exponent;\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

#[test]
fn generate_key_lowers_source_modulo_assignments_as_regular_hints() {
    let dir = temp_dir("source-modulo-assignment");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, modulo_source());
    assert_generated_assignment_ops(&dir, &["mod"]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_modulo_assignment_hints_with_trace_bytes() {
    assert_witness_accepts(
        "source-modulo-assignment-witness",
        modulo_source(),
        &[4, 0, 7, 1],
    );
}

#[test]
fn prove_witness_rejects_source_modulo_assignment_mismatch_with_trace_bytes() {
    assert_witness_rejects(
        "source-modulo-assignment-mismatch",
        modulo_source(),
        &[4, 1, 7, 0],
    );
}

#[test]
fn generate_key_lowers_source_comparison_assignments_as_regular_hints() {
    let dir = temp_dir("source-comparison-assignment");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, comparison_source());
    assert_generated_assignment_ops(&dir, &["lt", "le", "gt", "ge", "eq", "ne"]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_comparison_assignment_hints_with_trace_bytes() {
    assert_witness_accepts(
        "source-comparison-assignment-witness",
        comparison_source(),
        &[4, 4, 0, 1, 0, 1, 1, 0, 8, 3, 0, 0, 1, 1, 0, 1],
    );
}

#[test]
fn prove_witness_rejects_source_comparison_assignment_mismatch_with_trace_bytes() {
    assert_witness_rejects(
        "source-comparison-assignment-mismatch",
        comparison_source(),
        &[4, 4, 0, 1, 0, 1, 0, 0, 8, 3, 0, 0, 1, 1, 0, 1],
    );
}

#[test]
fn generate_key_lowers_source_bitwise_assignments_as_regular_hints() {
    let dir = temp_dir("source-bitwise-assignment");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, bitwise_source());
    assert_generated_assignment_ops(&dir, &["bitand", "bitxor", "bitor"]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_bitwise_assignment_hints_with_trace_bytes() {
    assert_witness_accepts(
        "source-bitwise-assignment-witness",
        bitwise_source(),
        &[6, 3, 2, 5, 7, 10, 12, 8, 6, 14],
    );
}

#[test]
fn prove_witness_rejects_source_bitwise_assignment_mismatch_with_trace_bytes() {
    assert_witness_rejects(
        "source-bitwise-assignment-mismatch",
        bitwise_source(),
        &[6, 3, 2, 5, 0, 10, 12, 8, 6, 14],
    );
}

#[test]
fn generate_key_lowers_source_shift_assignments_as_regular_hints() {
    let dir = temp_dir("source-shift-assignment");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, shift_source());
    assert_generated_assignment_ops(&dir, &["shl", "shr"]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_shift_assignment_hints_with_trace_bytes() {
    assert_witness_accepts(
        "source-shift-assignment-witness",
        shift_source(),
        &[3, 2, 12, 0, 32, 3, 256, 4],
    );
}

#[test]
fn prove_witness_rejects_source_shift_assignment_mismatch_with_trace_bytes() {
    assert_witness_rejects(
        "source-shift-assignment-mismatch",
        shift_source(),
        &[3, 2, 12, 1, 32, 3, 256, 4],
    );
}

#[test]
fn generate_key_lowers_source_logical_assignments_as_regular_hints() {
    let dir = temp_dir("source-logical-assignment");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, logical_source());
    assert_generated_assignment_ops(&dir, &["not", "and", "or"]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_logical_assignment_hints_with_trace_bytes() {
    assert_witness_accepts(
        "source-logical-assignment-witness",
        logical_source(),
        &[0, 9, 1, 0, 9, 4, 7, 0, 7, 4],
    );
}

#[test]
fn prove_witness_rejects_source_logical_assignment_mismatch_with_trace_bytes() {
    assert_witness_rejects(
        "source-logical-assignment-mismatch",
        logical_source(),
        &[0, 9, 1, 0, 8, 4, 7, 0, 7, 4],
    );
}

#[test]
fn generate_key_lowers_source_dynamic_power_assignments_as_regular_hints() {
    let dir = temp_dir("source-dynamic-power-assignment");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, dynamic_power_source());
    assert_generated_assignment_ops(&dir, &["pow"]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_accepts_source_dynamic_power_assignment_hints_with_trace_bytes() {
    assert_witness_accepts(
        "source-dynamic-power-assignment-witness",
        dynamic_power_source(),
        &[2, 3, 8, 5, 0, 1],
    );
}

#[test]
fn prove_witness_rejects_source_dynamic_power_assignment_mismatch_with_trace_bytes() {
    assert_witness_rejects(
        "source-dynamic-power-assignment-mismatch",
        dynamic_power_source(),
        &[2, 3, 7, 5, 0, 1],
    );
}

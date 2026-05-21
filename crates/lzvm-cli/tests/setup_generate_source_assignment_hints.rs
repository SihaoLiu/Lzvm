use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{
    read_expression_info_binary_file, CodeOperand, OperationKind,
};
use lzvm_artifacts::hint_program::{
    SOURCE_ASSIGNMENT_CHECK_HINT, SOURCE_UNSUPPORTED_ASSIGNMENT_HINT,
};
use lzvm_artifacts::key_directory::read_key_directory_layout;
use lzvm_artifacts::regular_program::read_regular_program_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_cli::run_cli;
use lzvm_field::Felt;
use lzvm_prover::regular_constraints::{
    evaluate_regular_constraints, RegularConstraintInputs, RegularStageColumns,
};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-setup-generate-source-assignment-hints-{}-{name}",
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

#[test]
fn generate_key_lowers_source_copy_assignments_as_regular_hints() {
    let dir = temp_dir("source-copy-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 2);
    assert_eq!(expressions.hints[0].fields[0].name, "target");
    assert_eq!(expressions.hints[0].fields[1].name, "value");

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_accepts_source_copy_assignment_hints_with_trace_bytes() {
    let dir = temp_dir("source-copy-assignment-witness");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[4, 4, 6, 6]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_arithmetic_assignments_as_regular_hints() {
    let dir = temp_dir("source-arithmetic-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value + 1;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 2);
    assert_eq!(expressions.hints[0].fields[0].name, "target");
    assert_eq!(expressions.hints[0].fields[1].name, "expression");
    assert_eq!(expressions.hints[0].fields[1].values.len(), 3);

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "expression");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_accepts_source_arithmetic_assignment_hints_with_trace_bytes() {
    let dir = temp_dir("source-arithmetic-assignment-witness");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value + 1;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[4, 5, 6, 7]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_source_arithmetic_assignment_mismatch_with_trace_bytes() {
    let dir = temp_dir("source-arithmetic-assignment-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value + 1;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[4, 4, 6, 6]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("source assignment validation failed"));
}

#[test]
fn generate_key_lowers_source_division_assignments_as_regular_hints() {
    let dir = temp_dir("source-division-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value / 2;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 2);
    assert_eq!(expressions.hints[0].fields[0].name, "target");
    assert_eq!(expressions.hints[0].fields[1].name, "expression");
    assert_eq!(expressions.hints[0].fields[1].values.len(), 3);

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "expression");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_accepts_source_division_assignment_hints_with_trace_bytes() {
    let dir = temp_dir("source-division-assignment-witness");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value / 2;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[8, 4, 6, 3]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_source_division_assignment_mismatch_with_trace_bytes() {
    let dir = temp_dir("source-division-assignment-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value / 2;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[8, 5, 6, 3]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("source assignment validation failed"));
}

#[test]
fn prove_witness_rejects_source_division_assignment_zero_divisor_with_trace_bytes() {
    let dir = temp_dir("source-division-assignment-zero-divisor");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value / 0;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[8, 0, 6, 0]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("operator div has zero divisor"));
}

#[test]
fn generate_key_lowers_source_backslash_division_assignments_as_regular_hints() {
    let dir = temp_dir("source-backslash-division-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value \\ 2;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 2);
    assert_eq!(expressions.hints[0].fields[0].name, "target");
    assert_eq!(expressions.hints[0].fields[1].name, "expression");
    assert_eq!(expressions.hints[0].fields[1].values.len(), 3);

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "expression");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_accepts_source_backslash_division_assignment_hints_with_trace_bytes() {
    let dir = temp_dir("source-backslash-division-assignment-witness");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value \\ 2;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[8, 4, 6, 3]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_source_backslash_division_assignment_mismatch_with_trace_bytes() {
    let dir = temp_dir("source-backslash-division-assignment-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value \\ 2;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[8, 5, 6, 3]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("source assignment validation failed"));
}

#[test]
fn generate_key_lowers_source_negated_assignments_as_regular_hints() {
    let dir = temp_dir("source-negated-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = -value;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 2);
    assert_eq!(expressions.hints[0].fields[0].name, "target");
    assert_eq!(expressions.hints[0].fields[1].name, "expression");
    assert_eq!(expressions.hints[0].fields[1].values.len(), 3);

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "expression");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_accepts_source_negated_assignment_hints_with_trace_bytes() {
    let dir = temp_dir("source-negated-assignment-witness");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = -value;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    let neg4 = (Felt::ZERO - Felt::from_u64(4)).to_u64();
    let neg6 = (Felt::ZERO - Felt::from_u64(6)).to_u64();
    write_file(&trace_path, sample_trace_bytes(&[4, neg4, 6, neg6]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_source_negated_assignment_mismatch_with_trace_bytes() {
    let dir = temp_dir("source-negated-assignment-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = -value;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    let neg6 = (Felt::ZERO - Felt::from_u64(6)).to_u64();
    write_file(&trace_path, sample_trace_bytes(&[4, 4, 6, neg6]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("source assignment validation failed"));
}

#[test]
fn generate_key_lowers_source_power_assignments_as_regular_hints() {
    let dir = temp_dir("source-power-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value ** 2;\n\
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
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(expressions.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(expressions.hints[0].fields.len(), 2);
    assert_eq!(expressions.hints[0].fields[0].name, "target");
    assert_eq!(expressions.hints[0].fields[1].name, "expression");
    assert_eq!(expressions.hints[0].fields[1].values.len(), 3);

    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(regular.hints.hints[0].name, SOURCE_ASSIGNMENT_CHECK_HINT);
    assert_eq!(regular.hints.hints[0].fields[1].name, "expression");
    assert_eq!(regular.hints.hints[0].fields[1].values.len(), 3);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_accepts_source_power_assignment_hints_with_trace_bytes() {
    let dir = temp_dir("source-power-assignment-witness");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value ** 2;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[4, 16, 6, 36]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_source_power_assignment_mismatch_with_trace_bytes() {
    let dir = temp_dir("source-power-assignment-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let output_dir = dir.join("proof-out");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] = value ** 2;\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    write_file(&guest_image, sample_guest_image());
    write_file(&trace_path, sample_trace_bytes(&[4, 15, 6, 36]));

    let mut setup_stdout = Vec::new();
    let mut setup_stderr = Vec::new();
    let setup_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut setup_stdout,
        &mut setup_stderr,
    );
    assert_eq!(
        setup_code,
        0,
        "stderr={}",
        String::from_utf8_lossy(&setup_stderr)
    );

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

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("source assignment validation failed"));
}

#[test]
fn generate_key_records_assignments_to_statically_inactive_fixed_columns_as_hints() {
    let dir = temp_dir("inactive-fixed-assignment-hint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int count = 0;\n\
             if (count > 0) {\n\
                 col fixed inactive.value;\n\
             }\n\
             inactive.value[0] = 7;\n\
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
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(
        expressions.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(
        regular.hints.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_records_assignments_to_statically_inactive_variables_as_hints() {
    let dir = temp_dir("inactive-variable-assignment-hint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             int count = 0;\n\
             if (count > 0) {\n\
                 int inactive = 0;\n\
             }\n\
             inactive = 7;\n\
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
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(
        expressions.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(
        regular.hints.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_records_assignments_to_inactive_template_variables_as_hints() {
    let dir = temp_dir("inactive-template-variable-assignment-hint");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate Unused() {\n\
             int inactive = 0;\n\
         }\n\
         airtemplate UnitA() {\n\
             inactive = 7;\n\
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
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(
        expressions.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(
        regular.hints.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_records_unsupported_source_compound_assignments_as_regular_hints() {
    let dir = temp_dir("unsupported-source-compound-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out[1];\n\
             out[0] += value;\n\
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
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(
        expressions.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(
        regular.hints.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_lowers_source_constrained_assignments() {
    let dir = temp_dir("source-constrained-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             col witness out;\n\
             out <== value + 1;\n\
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
    let setup = read_unit_setup_info_binary_file(
        unit.setup_info_binary()
            .expect("setup metadata path should derive"),
    )
    .expect("setup metadata should parse");
    let expressions = read_expression_info_binary_file(
        unit.expression_info_binary()
            .expect("expression metadata path should derive"),
    )
    .expect("expression metadata should parse");
    assert_eq!(expressions.constraints.len(), 1);
    assert!(expressions.hints.is_empty());
    let constraint = &expressions.constraints[0];
    assert_eq!(constraint.operations.len(), 2);
    assert_eq!(constraint.operations[0].op, OperationKind::Add);
    assert!(matches!(
        constraint.operations[0].sources[0],
        CodeOperand::Commitment {
            id: 0,
            prime: None,
            dimension: 1,
        }
    ));
    assert!(matches!(
        constraint.operations[0].sources[1],
        CodeOperand::Number {
            value: 1,
            dimension: 1,
        }
    ));
    assert_eq!(constraint.operations[1].op, OperationKind::Sub);
    assert!(matches!(
        constraint.operations[1].sources[0],
        CodeOperand::Commitment {
            id: 1,
            prime: None,
            dimension: 1,
        }
    ));
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.constraints.entries.len(), 1);
    assert!(regular.hints.hints.is_empty());
    let stage_values = [4, 5, 6, 7].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &stage_values,
    }];
    let results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate");
    assert!(results[0].invalid_rows.is_empty());

    let invalid_stage_values = [4, 5, 6, 8].map(Felt::from_u64);
    let invalid_stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &invalid_stage_values,
    }];
    let invalid_results = evaluate_regular_constraints(
        &regular.constraints,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: u16::try_from(setup.n_stages).expect("stage count should fit"),
            stage_columns: &invalid_stage_columns,
            opening_point_offsets: &setup.opening_points,
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraints should evaluate invalid input");
    assert_eq!(invalid_results[0].invalid_rows.len(), 1);
    assert_eq!(invalid_results[0].invalid_rows[0].row, 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn generate_key_records_unsupported_source_array_assignments_as_regular_hints() {
    let dir = temp_dir("unsupported-source-array-assignment");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_file(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness value;\n\
             table[0] = [0, 1];\n\
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
    assert_eq!(expressions.hints.len(), 1);
    assert_eq!(
        expressions.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    assert_eq!(regular.hints.hints.len(), 1);
    assert_eq!(
        regular.hints.hints[0].name,
        SOURCE_UNSUPPORTED_ASSIGNMENT_HINT
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

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
        "lzvm-cli-setup-generate-precompiled-mem-helper-hints-{}-{name}",
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

fn precompiled_mem_source() -> &'static str {
    "const int MEMORY_ID = 10;\n\
     const int MEMORY_LOAD_OP = 1;\n\
     const int MEMORY_STORE_OP = 2;\n\
     const int RESERVED_MEM_STEPS = 1;\n\
     const int MAX_MEM_STEPS_PER_MAIN_STEP = 4;\n\
     airtemplate UnitA() {\n\
         col witness main_step;\n\
         col witness is_write;\n\
         col witness assumed_is_write;\n\
         col witness addr;\n\
         col witness value[2];\n\
         col witness selector;\n\
         precompiled_mem_proves(addr: addr, main_step: main_step, value: value, sel: selector, is_write: is_write);\n\
         precompiled_mem_op(addr: addr, main_step: main_step, value: value, sel: selector, is_write: assumed_is_write);\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn positional_precompiled_mem_source() -> &'static str {
    "const int MEMORY_ID = 10;\n\
     const int MEMORY_LOAD_OP = 1;\n\
     const int MEMORY_STORE_OP = 2;\n\
     const int RESERVED_MEM_STEPS = 1;\n\
     const int MAX_MEM_STEPS_PER_MAIN_STEP = 4;\n\
     airtemplate UnitA() {\n\
         col witness main_step;\n\
         col witness is_write;\n\
         col witness addr;\n\
         col witness value[2];\n\
         col witness selector;\n\
         precompiled_mem_proves(MEMORY_ID, addr, main_step, value, selector, is_write);\n\
         precompiled_mem_op(addr: addr, main_step: main_step, value: value, sel: selector, is_write: is_write);\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.left = [5, 1];"
}

fn positional_precompiled_mem_op_source() -> &'static str {
    "const int MEMORY_ID = 10;\n\
     const int MEMORY_LOAD_OP = 1;\n\
     const int MEMORY_STORE_OP = 2;\n\
     const int RESERVED_MEM_STEPS = 1;\n\
     const int MAX_MEM_STEPS_PER_MAIN_STEP = 4;\n\
     airtemplate UnitA() {\n\
         col witness main_step;\n\
         col witness is_write;\n\
         col witness addr;\n\
         col witness value[2];\n\
         col witness selector;\n\
         precompiled_mem_proves(addr: addr, main_step: main_step, value: value, sel: selector, is_write: is_write);\n\
         precompiled_mem_op(MEMORY_ID, addr, main_step, value, selector, is_write);\n\
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
fn generate_key_lowers_precompiled_mem_proves_helper() {
    let dir = temp_dir("precompiled-mem-proves-helper");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, precompiled_mem_source());

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit = &layout.units[0];
    let regular = read_regular_program_file(
        unit.expression_program()
            .expect("regular program path should derive"),
    )
    .expect("regular program should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(regular.hints.hints.len(), 2);
    assert_eq!(regular.hints.hints[0].name, SOURCE_LOOKUP_PROVES_HINT);
    assert_eq!(regular.hints.hints[1].name, SOURCE_LOOKUP_ASSUMES_HINT);
    let proves = &regular.hints.hints[0];
    assert_eq!(proves.fields[0].name, "bus_id");
    assert_eq!(proves.fields[0].values[0].operand, HintOperand::Number(10));
    assert_eq!(proves.fields[1].name, "values");
    assert_eq!(
        proves.fields[1].values[0].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(proves.fields[1].values[1].operand, HintOperand::Number(2));
    assert_eq!(proves.fields[1].values[2].operand, HintOperand::Number(1));
    assert_eq!(
        proves.fields[1].values[3].operand,
        HintOperand::String("sub".to_owned())
    );
    assert_eq!(
        proves.fields[1].values[4].operand,
        HintOperand::String("mul".to_owned())
    );
    assert_eq!(proves.fields[1].values[5].operand, HintOperand::Number(1));
    assert_eq!(
        proves.fields[1].values[6].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(
        proves.fields[1].values[7].operand,
        HintOperand::Commitment {
            id: 3,
            row_offset_index: 0
        }
    );
    assert_eq!(proves.fields[1].values[8].operand, HintOperand::Number(1));
    assert_eq!(proves.fields[1].values[9].operand, HintOperand::Number(4));
    assert_eq!(
        proves.fields[1].values[10].operand,
        HintOperand::Commitment {
            id: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        proves.fields[1].values[11].operand,
        HintOperand::String("mul".to_owned())
    );
    assert_eq!(
        proves.fields[1].values[12].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(
        proves.fields[1].values[13].operand,
        HintOperand::Commitment {
            id: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(
        proves.fields[1].values[14].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(proves.fields[1].values[15].operand, HintOperand::Number(2));
    assert_eq!(
        proves.fields[1].values[16].operand,
        HintOperand::String("add".to_owned())
    );
    assert_eq!(proves.fields[1].values[17].operand, HintOperand::Number(8));
    assert_eq!(
        proves.fields[1].values[18].operand,
        HintOperand::CommitmentElement {
            id: 4,
            element: 0,
            row_offset_index: 0
        }
    );
    assert_eq!(
        proves.fields[1].values[19].operand,
        HintOperand::CommitmentElement {
            id: 4,
            element: 1,
            row_offset_index: 0
        }
    );
    assert_eq!(proves.fields[2].name, "value_lengths");
    assert_eq!(
        proves.fields[2]
            .values
            .iter()
            .map(|value| &value.operand)
            .collect::<Vec<_>>(),
        vec![
            &HintOperand::Number(7),
            &HintOperand::Number(1),
            &HintOperand::Number(9),
            &HintOperand::Number(1),
            &HintOperand::Number(1),
            &HintOperand::Number(1),
        ]
    );
    assert_eq!(proves.fields[3].name, "selector");
    assert_eq!(
        proves.fields[3].values[0].operand,
        HintOperand::Commitment {
            id: 5,
            row_offset_index: 0
        }
    );
}

#[test]
fn positional_precompiled_mem_proves_uses_write_flag() {
    let dir = temp_dir("positional-precompiled-mem-proves-helper");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, positional_precompiled_mem_source());

    let trace_values = &[3, 1, 96, 0xffff_ffff, 0, 2, 4, 0, 128, 7, 0, 2];
    let (code, stdout, stderr) = run_witness(&dir, trace_values);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn positional_precompiled_mem_op_uses_write_flag() {
    let dir = temp_dir("positional-precompiled-mem-op-helper");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, positional_precompiled_mem_op_source());

    let trace_values = &[3, 1, 96, 0xffff_ffff, 0, 2, 4, 0, 128, 7, 0, 2];
    let (code, stdout, stderr) = run_witness(&dir, trace_values);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_accepts_balanced_precompiled_mem_helper_rows() {
    let dir = temp_dir("precompiled-mem-helper-witness");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, precompiled_mem_source());

    let trace_values = &[3, 1, 1, 96, 0xffff_ffff, 0, 1, 4, 0, 0, 128, 7, 0, 1];
    let (code, stdout, stderr) = run_witness(&dir, trace_values);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
    assert!(stderr.is_empty());
}

#[test]
fn prove_witness_rejects_mismatched_precompiled_mem_write_flag() {
    let dir = temp_dir("precompiled-mem-helper-mismatch");
    let _ = fs::remove_dir_all(&dir);
    assert_generate_key_succeeds(&dir, precompiled_mem_source());

    let trace_values = &[3, 1, 0, 96, 0xffff_ffff, 0, 1, 4, 0, 0, 128, 7, 0, 1];
    let (code, stdout, stderr) = run_witness(&dir, trace_values);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("unbalanced lookup bus 10"));
}

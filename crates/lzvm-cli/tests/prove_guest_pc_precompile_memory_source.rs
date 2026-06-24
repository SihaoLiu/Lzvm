use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::key_directory::{key_directory_catalog_digest, read_key_directory_catalog};
use lzvm_artifacts::proof::parse_proof_artifact;
use lzvm_artifacts::public_values::{encode_public_values, parse_public_values, PublicValues};
use lzvm_cli::run_cli;

const ENTRY: u64 = 0x8000_0000;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-prove-guest-pc-precompile-memory-source-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

fn precompile_memory_source() -> &'static str {
    "const int MEMORY_ID = 10;\n\
     const int MEMORY_LOAD_OP = 1;\n\
     const int MEMORY_STORE_OP = 2;\n\
     const int RESERVED_MEM_STEPS = 1;\n\
     const int MAX_MEM_STEPS_PER_MAIN_STEP = 4;\n\
     airtemplate UnitA() {\n\
         col witness precompile_mem_main_step;\n\
         col witness precompile_mem_is_write;\n\
         col witness precompile_mem_address;\n\
         col witness precompile_mem_value[2];\n\
         col witness precompile_mem_byte_len;\n\
         col witness precompile_mem_selector;\n\
         precompile_mem_main_step - main.expected_main_step;\n\
         precompile_mem_is_write - main.expected_is_write;\n\
         precompile_mem_address - main.expected_address;\n\
         precompile_mem_value[0] - main.expected_value_lo;\n\
         precompile_mem_value[1] - main.expected_value_hi;\n\
         precompile_mem_byte_len - main.expected_byte_len;\n\
         precompile_mem_selector - main.expected_selector;\n\
         precompile_mem_selector * (1 - precompile_mem_selector);\n\
         precompile_mem_is_write * (1 - precompile_mem_is_write);\n\
         precompile_mem_selector * (precompile_mem_byte_len - 8);\n\
         precompiled_mem_proves(addr: precompile_mem_address, main_step: precompile_mem_main_step, value: precompile_mem_value, sel: precompile_mem_selector, is_write: precompile_mem_is_write);\n\
         precompiled_mem_op(addr: precompile_mem_address, main_step: precompile_mem_main_step, value: precompile_mem_value, sel: precompile_mem_selector, is_write: precompile_mem_is_write);\n\
     }\n\
     airgroup GroupA { UnitA(); }\n\
     col fixed main.expected_main_step = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];\n\
     col fixed main.expected_is_write = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];\n\
     col fixed main.expected_address = [64, 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168, 176, 184, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];\n\
     col fixed main.expected_value_lo = [96, 128, 1, 160, 4294967295, 4294967295, 4294967295, 4294967295, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];\n\
     col fixed main.expected_value_hi = [0, 0, 0, 0, 4294967295, 4294967295, 4294967295, 4294967295, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];\n\
     col fixed main.expected_byte_len = [8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];\n\
     col fixed main.expected_selector = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];"
}

fn generate_key(dir: &Path) {
    let source_path = dir.join("source").join("main.pil");
    write_file(&source_path, precompile_memory_source());

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
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains("status=ok\n"));
    assert!(stderr.is_empty());
}

fn write_public_values(dir: &Path) -> PathBuf {
    let catalog = read_key_directory_catalog(dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_path = dir.join("public_values.bin");
    write_file(
        &public_values_path,
        encode_public_values(&PublicValues {
            schema_version: 1,
            setup_hash,
            values: Vec::new(),
        })
        .expect("public values should encode"),
    );
    public_values_path
}

fn write_zero_trace(dir: &Path) -> PathBuf {
    let trace_path = dir.join("zero_trace.bin");
    write_file(&trace_path, vec![0_u8; 32 * 8 * 8]);
    trace_path
}

fn sample_add256_precompile_image() -> Vec<u8> {
    let data_address = 64_u64;
    let params_address = data_address;
    let a_address = data_address + 32;
    let b_address = a_address + 32;
    let out_address = b_address + 32;
    let code_words = [
        riscv_addi(1, 0, params_address as i16),
        riscv_csrrs(2, 0x0811, 1),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&a_address.to_le_bytes());
    data.extend_from_slice(&b_address.to_le_bytes());
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.extend_from_slice(&out_address.to_le_bytes());
    for _ in 0..4 {
        data.extend_from_slice(&u64::MAX.to_le_bytes());
    }
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.extend_from_slice(&[0; 24]);
    data.extend_from_slice(&[0; 32]);

    let headers = [
        program_header_at(176, ENTRY, code.len() as u64),
        program_header_at(data_offset, data_address, data.len() as u64),
    ];
    let mut image = sample_guest_image_with_program_headers(&headers);
    image.resize(176, 0);
    image.extend_from_slice(&code);
    image.resize(data_offset as usize, 0);
    image.extend_from_slice(&data);
    image
}

fn riscv_addi(rd: u8, rs1: u8, imm: i16) -> u32 {
    ((imm as u16 as u32) << 20) | (u32::from(rs1) << 15) | (u32::from(rd) << 7) | 0x13
}

fn riscv_csrrs(rd: u8, csr: u16, rs1: u8) -> u32 {
    (u32::from(csr) << 20) | (u32::from(rs1) << 15) | (0x2 << 12) | (u32::from(rd) << 7) | 0x73
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
    bytes[24..32].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_guest_image_with_program_headers(program_headers: &[[u8; 56]]) -> Vec<u8> {
    let mut bytes = sample_guest_image();
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(program_headers.len() as u16).to_le_bytes());
    for header in program_headers {
        bytes.extend_from_slice(header);
    }
    bytes
}

fn program_header_at(offset: u64, virtual_address: u64, len: u64) -> [u8; 56] {
    let mut header = [0_u8; 56];
    header[0..4].copy_from_slice(&1_u32.to_le_bytes());
    header[4..8].copy_from_slice(&7_u32.to_le_bytes());
    header[8..16].copy_from_slice(&offset.to_le_bytes());
    header[16..24].copy_from_slice(&virtual_address.to_le_bytes());
    header[24..32].copy_from_slice(&virtual_address.to_le_bytes());
    header[32..40].copy_from_slice(&len.to_le_bytes());
    header[40..48].copy_from_slice(&len.to_le_bytes());
    header[48..56].copy_from_slice(&8_u64.to_le_bytes());
    header
}

#[test]
fn source_generated_key_proves_guest_pc_precompile_memory_trace() {
    let dir = temp_dir("add256");
    let _ = fs::remove_dir_all(&dir);
    generate_key(&dir);
    let public_values_path = write_public_values(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    write_file(&guest_image, sample_add256_precompile_image());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--guest-pc-trace",
            "16",
            dir.to_str().expect("setup path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "stderr={}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("trace_rows=32\n"), "{stdout_text}");
    assert!(stdout_text.contains("trace_columns=8\n"), "{stdout_text}");
    assert!(
        stdout_text.contains("source_program_archive=present\n"),
        "{stdout_text}"
    );

    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let public_values =
        parse_public_values(&fs::read(&public_values_path).expect("public values should read"))
            .expect("public values should parse");
    assert_eq!(proof.setup_hash, public_values.setup_hash);

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    assert!(String::from_utf8(verify_stdout)
        .expect("verify stdout should be utf-8")
        .contains("status=ok\n"));
}

#[test]
fn source_generated_key_rejects_empty_precompile_memory_trace() {
    let dir = temp_dir("empty-trace");
    let _ = fs::remove_dir_all(&dir);
    generate_key(&dir);
    let public_values_path = write_public_values(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = write_zero_trace(&dir);
    write_file(&guest_image, sample_add256_precompile_image());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("setup path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains("regular constraint"));
}

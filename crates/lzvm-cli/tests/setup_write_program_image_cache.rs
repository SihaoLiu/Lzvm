use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageGpuMode,
};
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
use lzvm_cli::run_cli;

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

fn temp_dir(name: &str) -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-program-image-cache-{}-{name}",
            std::process::id()
        ))
}

#[test]
fn writes_program_image_commitment_cache_from_cli_inputs() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let program_path = dir.join("program.bin");
    let guest_image_path = dir.join("guest.elf");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("unit.root.bin");
    let output_path = dir.join("unit.commit.bin");

    fs::write(&program_path, b"packed-program").expect("program should be written");
    fs::write(&guest_image_path, sample_guest_image()).expect("guest image should be written");
    fs::write(&constraint_digest_path, [0x44_u8; 32]).expect("digest should be written");
    fs::write(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    )
    .expect("root should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-program-image-cache",
            "--backend",
            "cuda",
            program_path.to_str().expect("program path should be utf-8"),
            guest_image_path
                .to_str()
                .expect("guest image path should be utf-8"),
            constraint_digest_path
                .to_str()
                .expect("constraint digest path should be utf-8"),
            root_path.to_str().expect("root path should be utf-8"),
            "1024",
            "17",
            "8",
            "4",
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let cache = read_program_image_commitment_cache_file(&output_path)
        .expect("program-image cache should parse");
    let byte_count = fs::metadata(&output_path)
        .expect("program-image cache should exist")
        .len();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(cache.tree_root, [11, 12, 13, 14]);
    assert_eq!(cache.trace_row_count, 1024);
    assert_eq!(cache.trace_column_count, 17);
    assert_eq!(cache.blowup_factor, 8);
    assert_eq!(cache.merkle_tree_arity, 4);
    assert_eq!(cache.gpu_mode, ProgramImageGpuMode::Cuda);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written={byte_count}\noutput={}\nprogram_image_cache={}\nprogram_image_cache_segment_hash=f42614ba128d6a56d9d2df9b73c1c44aa3898eab8a02c5e99078918d4be1545b\nprogram_image_cache_program_digest={}\nprogram_image_cache_source_image_digest={}\nprogram_image_cache_constraint_system_digest={}\nprogram_image_cache_tree_root=11,12,13,14\nprogram_image_cache_trace_rows=1024\nprogram_image_cache_trace_columns=17\nprogram_image_cache_blowup_factor=8\nprogram_image_cache_arity=4\nprogram_image_cache_gpu_mode=cuda\n",
            output_path.display(),
            output_path.display(),
            format_hash(&cache.program_digest),
            format_hash(&cache.source_image_digest),
            format_hash(&cache.constraint_system_digest)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_program_image_cache_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-program-image-cache",
            "program.bin",
            "guest.elf",
            "constraint.digest",
            "root.bin",
            "1024",
            "17",
            "8",
            "4",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-program-image-cache [--backend cpu|cuda] <program-bin> <guest-image> <constraint-digest-bin> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>\n       lzvm setup write-program-image-cache [--backend cpu|cuda] --setup-dir <setup-dir> <program-bin> <guest-image> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>\n"
    );
}

fn format_hash(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

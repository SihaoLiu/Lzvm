use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageGpuMode,
};
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
use lzvm_setup::{
    write_program_image_commitment_cache_file, ProgramImageCommitmentCacheFileRequest,
};

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
    std::env::temp_dir().join(format!(
        "lzvm-setup-program-image-cache-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_program_image_commitment_cache_through_validated_staging() {
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

    let report =
        write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
            program_path: &program_path,
            guest_image_path: &guest_image_path,
            constraint_digest_path: &constraint_digest_path,
            root_path: &root_path,
            trace_row_count: 1024,
            trace_column_count: 17,
            blowup_factor: 8,
            merkle_tree_arity: 4,
            gpu_mode: ProgramImageGpuMode::Cuda,
            output_path: &output_path,
        })
        .expect("program-image cache should write");
    let cache = read_program_image_commitment_cache_file(&output_path)
        .expect("program-image cache should parse");
    let byte_count = fs::metadata(&output_path)
        .expect("program-image cache should exist")
        .len();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(report.path, output_path);
    assert_eq!(report.bytes_written, byte_count);
    assert_eq!(cache.tree_root, [11, 12, 13, 14]);
    assert_eq!(cache.trace_row_count, 1024);
    assert_eq!(cache.trace_column_count, 17);
    assert_eq!(cache.blowup_factor, 8);
    assert_eq!(cache.merkle_tree_arity, 4);
    assert_eq!(cache.gpu_mode, ProgramImageGpuMode::Cuda);
    assert_ne!(cache.program_digest, [0_u8; 32]);
    assert_ne!(cache.source_image_digest, [0_u8; 32]);
    assert_eq!(cache.constraint_system_digest, [0x44_u8; 32]);
}

#[test]
fn rejects_program_image_cache_with_wrong_constraint_digest_length() {
    let dir = temp_dir("bad-digest");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let program_path = dir.join("program.bin");
    let guest_image_path = dir.join("guest.elf");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("unit.root.bin");
    let output_path = dir.join("unit.commit.bin");

    fs::write(&program_path, b"packed-program").expect("program should be written");
    fs::write(&guest_image_path, sample_guest_image()).expect("guest image should be written");
    fs::write(&constraint_digest_path, [0x44_u8; 31]).expect("digest should be written");
    fs::write(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    )
    .expect("root should be written");

    let result =
        write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
            program_path: &program_path,
            guest_image_path: &guest_image_path,
            constraint_digest_path: &constraint_digest_path,
            root_path: &root_path,
            trace_row_count: 1024,
            trace_column_count: 17,
            blowup_factor: 8,
            merkle_tree_arity: 4,
            gpu_mode: ProgramImageGpuMode::Cpu,
            output_path: &output_path,
        });
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(result.is_err());
}

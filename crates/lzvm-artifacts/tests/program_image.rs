use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::program_image::{
    build_program_image_commitment_cache, encode_program_image_commitment_cache,
    parse_program_image_commitment_cache, read_program_image_commitment_cache_file,
    ProgramImageCommitmentCache, ProgramImageCommitmentCacheError, ProgramImageCommitmentInputs,
    ProgramImageGpuMode,
};
use sha2::{Digest, Sha256};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const FILE_PAYLOAD_OFFSET: usize = 24;
const TREE_ROOT_OFFSET: usize = FILE_PAYLOAD_OFFSET + 32 * 3;
const TRACE_ROW_COUNT_OFFSET: usize = TREE_ROOT_OFFSET + 8 * 4;
const BLOWUP_FACTOR_OFFSET: usize = TRACE_ROW_COUNT_OFFSET + 8 + 4;
const MERKLE_TREE_ARITY_OFFSET: usize = BLOWUP_FACTOR_OFFSET + 4;

fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn sample_cache() -> ProgramImageCommitmentCache {
    ProgramImageCommitmentCache {
        program_digest: hash(b"packed-program"),
        source_image_digest: hash(b"source-image"),
        constraint_system_digest: [0x44; 32],
        tree_root: [11, 12, 13, 14],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
    }
}

fn encoded_sample_cache() -> Vec<u8> {
    encode_program_image_commitment_cache(&sample_cache()).expect("cache should encode")
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-program-image-{}-{name}", std::process::id()))
}

#[test]
fn builds_program_image_commitment_cache_from_inputs() {
    let cache = build_program_image_commitment_cache(ProgramImageCommitmentInputs {
        program_bytes: b"packed-program",
        source_image_bytes: b"source-image",
        constraint_system_digest: [0x44; 32],
        tree_root: [11, 12, 13, 14],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
    })
    .expect("cache should build");

    assert_eq!(cache, sample_cache());
}

#[test]
fn encodes_and_parses_program_image_commitment_cache() {
    let encoded = encoded_sample_cache();
    let parsed = parse_program_image_commitment_cache(&encoded).expect("cache should parse");

    assert_eq!(&encoded[0..4], b"pimg");
    assert_eq!(parsed, sample_cache());
}

#[test]
fn rejects_non_canonical_program_image_commitment_cache_tree_roots() {
    let mut cache = sample_cache();
    cache.tree_root[1] = NON_CANONICAL_FIELD;
    let error =
        encode_program_image_commitment_cache(&cache).expect_err("cache root should be canonical");

    assert_eq!(
        error.to_string(),
        "program-image commitment cache tree root word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );

    let error = build_program_image_commitment_cache(ProgramImageCommitmentInputs {
        program_bytes: b"packed-program",
        source_image_bytes: b"source-image",
        constraint_system_digest: [0x44; 32],
        tree_root: [11, NON_CANONICAL_FIELD, 13, 14],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
    })
    .expect_err("cache root should be canonical");

    assert_eq!(
        error.to_string(),
        "program-image commitment cache tree root word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_program_image_commitment_cache_tree_roots_when_parsing() {
    let mut encoded = encoded_sample_cache();
    encoded[TREE_ROOT_OFFSET + 16..TREE_ROOT_OFFSET + 24]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let error =
        parse_program_image_commitment_cache(&encoded).expect_err("cache root should be canonical");

    assert_eq!(
        error.to_string(),
        "program-image commitment cache tree root word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_unsupported_program_image_commitment_cache_versions() {
    let encoded = encoded_sample_cache();
    let parsed = lzvm_artifacts::sectioned::parse_sectioned_file(&encoded, *b"pimg", 1)
        .expect("sectioned cache should parse");
    let encoded = lzvm_artifacts::sectioned::encode_sectioned_file(
        &lzvm_artifacts::sectioned::SectionedFile {
            kind: *b"pimg",
            version: 0,
            sections: parsed.sections,
        },
    )
    .expect("sectioned cache should encode");

    assert_eq!(
        parse_program_image_commitment_cache(&encoded)
            .expect_err("unsupported cache version should reject"),
        ProgramImageCommitmentCacheError::UnsupportedVersion {
            found: 0,
            expected: 1,
        }
    );
}

#[test]
fn reads_program_image_commitment_cache_from_a_file_path() {
    let path = temp_file_path("cache.bin");
    fs::write(&path, encoded_sample_cache()).expect("fixture should be written");

    let parsed = read_program_image_commitment_cache_file(&path).expect("cache should read");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_cache());
}

#[test]
fn rejects_empty_program_image_inputs() {
    assert!(matches!(
        build_program_image_commitment_cache(ProgramImageCommitmentInputs {
            program_bytes: b"",
            source_image_bytes: b"source-image",
            constraint_system_digest: [0x44; 32],
            tree_root: [11, 12, 13, 14],
            trace_row_count: 1024,
            trace_column_count: 17,
            blowup_factor: 8,
            merkle_tree_arity: 4,
            gpu_mode: ProgramImageGpuMode::Cpu,
        }),
        Err(ProgramImageCommitmentCacheError::EmptyProgram)
    ));
    assert!(matches!(
        build_program_image_commitment_cache(ProgramImageCommitmentInputs {
            program_bytes: b"packed-program",
            source_image_bytes: b"",
            constraint_system_digest: [0x44; 32],
            tree_root: [11, 12, 13, 14],
            trace_row_count: 1024,
            trace_column_count: 17,
            blowup_factor: 8,
            merkle_tree_arity: 4,
            gpu_mode: ProgramImageGpuMode::Cpu,
        }),
        Err(ProgramImageCommitmentCacheError::EmptySourceImage)
    ));
}

#[test]
fn rejects_invalid_program_image_geometry() {
    let mut cache = sample_cache();
    cache.trace_row_count = 0;
    assert!(matches!(
        encode_program_image_commitment_cache(&cache),
        Err(ProgramImageCommitmentCacheError::EmptyTraceRows)
    ));

    let mut cache = sample_cache();
    cache.trace_row_count = 1000;
    assert!(matches!(
        encode_program_image_commitment_cache(&cache),
        Err(ProgramImageCommitmentCacheError::InvalidTraceRows { value: 1000 })
    ));

    let mut cache = sample_cache();
    cache.trace_column_count = 0;
    assert!(matches!(
        encode_program_image_commitment_cache(&cache),
        Err(ProgramImageCommitmentCacheError::EmptyTraceColumns)
    ));

    let mut cache = sample_cache();
    cache.blowup_factor = 3;
    assert!(matches!(
        encode_program_image_commitment_cache(&cache),
        Err(ProgramImageCommitmentCacheError::InvalidBlowupFactor { value: 3 })
    ));

    let mut cache = sample_cache();
    cache.trace_row_count = 1_u64 << 63;
    cache.blowup_factor = 4;
    assert!(matches!(
        encode_program_image_commitment_cache(&cache),
        Err(
            ProgramImageCommitmentCacheError::TraceRowExpansionOverflow {
                trace_row_count: 0x8000_0000_0000_0000,
                blowup_factor: 4
            }
        )
    ));

    let mut cache = sample_cache();
    cache.trace_row_count = 1_u64 << 31;
    cache.blowup_factor = 4;
    assert!(matches!(
        encode_program_image_commitment_cache(&cache),
        Err(
            ProgramImageCommitmentCacheError::UnsupportedTraceDomainBits {
                bits: 33,
                max_bits: 32
            }
        )
    ));

    let mut cache = sample_cache();
    cache.merkle_tree_arity = 3;
    assert!(matches!(
        encode_program_image_commitment_cache(&cache),
        Err(ProgramImageCommitmentCacheError::InvalidMerkleTreeArity { value: 3 })
    ));

    let mut cache = sample_cache();
    cache.merkle_tree_arity = 8;
    assert!(matches!(
        encode_program_image_commitment_cache(&cache),
        Err(ProgramImageCommitmentCacheError::InvalidMerkleTreeArity { value: 8 })
    ));
}

#[test]
fn rejects_invalid_program_image_geometry_when_parsing() {
    let mut encoded = encoded_sample_cache();
    encoded[TRACE_ROW_COUNT_OFFSET..TRACE_ROW_COUNT_OFFSET + 8]
        .copy_from_slice(&1000_u64.to_le_bytes());
    assert!(matches!(
        parse_program_image_commitment_cache(&encoded),
        Err(ProgramImageCommitmentCacheError::InvalidTraceRows { value: 1000 })
    ));

    let mut encoded = encoded_sample_cache();
    encoded[TRACE_ROW_COUNT_OFFSET..TRACE_ROW_COUNT_OFFSET + 8]
        .copy_from_slice(&(1_u64 << 63).to_le_bytes());
    encoded[BLOWUP_FACTOR_OFFSET..BLOWUP_FACTOR_OFFSET + 4].copy_from_slice(&4_u32.to_le_bytes());
    assert!(matches!(
        parse_program_image_commitment_cache(&encoded),
        Err(
            ProgramImageCommitmentCacheError::TraceRowExpansionOverflow {
                trace_row_count: 0x8000_0000_0000_0000,
                blowup_factor: 4
            }
        )
    ));

    let mut encoded = encoded_sample_cache();
    encoded[TRACE_ROW_COUNT_OFFSET..TRACE_ROW_COUNT_OFFSET + 8]
        .copy_from_slice(&(1_u64 << 31).to_le_bytes());
    encoded[BLOWUP_FACTOR_OFFSET..BLOWUP_FACTOR_OFFSET + 4].copy_from_slice(&4_u32.to_le_bytes());
    assert!(matches!(
        parse_program_image_commitment_cache(&encoded),
        Err(
            ProgramImageCommitmentCacheError::UnsupportedTraceDomainBits {
                bits: 33,
                max_bits: 32
            }
        )
    ));

    let mut encoded = encoded_sample_cache();
    encoded[MERKLE_TREE_ARITY_OFFSET..MERKLE_TREE_ARITY_OFFSET + 4]
        .copy_from_slice(&8_u32.to_le_bytes());
    assert!(matches!(
        parse_program_image_commitment_cache(&encoded),
        Err(ProgramImageCommitmentCacheError::InvalidMerkleTreeArity { value: 8 })
    ));
}

#[test]
fn rejects_unknown_gpu_mode_in_program_image_cache() {
    let mut encoded = encoded_sample_cache();
    let mode_offset = encoded.len() - 4;
    encoded[mode_offset..].copy_from_slice(&7_u32.to_le_bytes());

    assert!(matches!(
        parse_program_image_commitment_cache(&encoded),
        Err(ProgramImageCommitmentCacheError::UnsupportedGpuMode { value: 7 })
    ));
}

use lzvm_artifacts::program_image::{ProgramImageCommitmentCache, ProgramImageGpuMode};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, parse_program_image_cache_segment,
    program_image_cache_segment_digest, ProgramImageCacheSegmentError,
    PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const SEGMENT_PAYLOAD_OFFSET: usize = 8;
const TREE_ROOT_OFFSET: usize = SEGMENT_PAYLOAD_OFFSET + 32 * 3;

fn sample_cache() -> ProgramImageCommitmentCache {
    ProgramImageCommitmentCache {
        program_digest: [0x11; 32],
        source_image_digest: [0x22; 32],
        constraint_system_digest: [0x33; 32],
        tree_root: [10, 11, 12, 13],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
    }
}

#[test]
fn encodes_and_parses_program_image_cache_segments() {
    let encoded =
        encode_program_image_cache_segment(&sample_cache()).expect("segment should encode");
    let parsed = parse_program_image_cache_segment(&encoded).expect("segment should parse");

    assert_eq!(PROGRAM_IMAGE_CACHE_SEGMENT_ID, 10_010);
    assert_eq!(&encoded[..4], b"pic0");
    assert_eq!(parsed, sample_cache());
}

#[test]
fn rejects_non_canonical_program_image_cache_segment_tree_roots() {
    let mut encoded =
        encode_program_image_cache_segment(&sample_cache()).expect("segment should encode");
    encoded[TREE_ROOT_OFFSET..TREE_ROOT_OFFSET + 8]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let error =
        parse_program_image_cache_segment(&encoded).expect_err("segment root should be canonical");

    assert_eq!(
        error.to_string(),
        "invalid program image cache segment payload: program-image commitment cache tree root word 0 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_unsupported_program_image_cache_segment_versions() {
    let mut encoded =
        encode_program_image_cache_segment(&sample_cache()).expect("segment should encode");
    encoded[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert_eq!(
        parse_program_image_cache_segment(&encoded),
        Err(ProgramImageCacheSegmentError::UnsupportedVersion { version: 2 })
    );
}

#[test]
fn computes_program_image_cache_segment_digest() {
    let encoded =
        encode_program_image_cache_segment(&sample_cache()).expect("segment should encode");

    assert_eq!(
        to_hex(&program_image_cache_segment_digest(&encoded)),
        "5b49502fb9787fe9214bf093e6755c185c73f5e39a7b0bef5841f1f4b107d880"
    );
}

#[test]
fn rejects_program_image_cache_segments_with_bad_magic() {
    let mut encoded =
        encode_program_image_cache_segment(&sample_cache()).expect("segment should encode");
    encoded[..4].copy_from_slice(b"bad0");

    assert_eq!(
        parse_program_image_cache_segment(&encoded),
        Err(ProgramImageCacheSegmentError::InvalidMagic)
    );
}

#[test]
fn rejects_truncated_program_image_cache_segments() {
    assert_eq!(
        parse_program_image_cache_segment(b"pic0"),
        Err(ProgramImageCacheSegmentError::UnexpectedEof {
            needed: 8,
            available: 4
        })
    );
}

fn to_hex(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

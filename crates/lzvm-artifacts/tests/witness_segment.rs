use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, parse_witness_commitment_segment, WitnessCommitmentSegment,
    WitnessCommitmentSegmentError, WitnessCommitmentStageSegment,
};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const FIRST_STAGE_ROOT_OFFSET: usize = 40 + 8;

fn sample_segment() -> WitnessCommitmentSegment {
    WitnessCommitmentSegment {
        unit_index: 0,
        input_byte_count: 64,
        trace_rows: 16,
        trace_columns: 4,
        stages: vec![WitnessCommitmentStageSegment {
            stage_index: 0,
            arity: 4,
            root: [1, 2, 3, 4],
            tree_byte_count: 128,
            tree_digest: [5; 32],
        }],
    }
}

fn segment_header(stage_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"wcs0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 0);
    push_u64(&mut bytes, 64);
    push_u64(&mut bytes, 16);
    push_u64(&mut bytes, 4);
    push_u32(&mut bytes, stage_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn rejects_non_canonical_witness_commitment_stage_roots() {
    let mut segment = sample_segment();
    segment.stages[0].root[2] = NON_CANONICAL_FIELD;

    let err = encode_witness_commitment_segment(&segment).expect_err("stage root should reject");

    assert_eq!(
        err.to_string(),
        "witness commitment unit 0 stage 0 root word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_witness_commitment_stage_roots_when_parsing() {
    let mut encoded =
        encode_witness_commitment_segment(&sample_segment()).expect("segment should encode");
    encoded[FIRST_STAGE_ROOT_OFFSET + 8..FIRST_STAGE_ROOT_OFFSET + 16]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_witness_commitment_segment(&encoded).expect_err("stage root should reject");

    assert_eq!(
        err.to_string(),
        "witness commitment unit 0 stage 0 root word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_truncated_witness_commitment_segments() {
    let result = parse_witness_commitment_segment(b"wcs0\x01\0");

    assert!(matches!(
        result,
        Err(WitnessCommitmentSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_stage_count_that_exceeds_remaining_stage_records() {
    assert!(matches!(
        parse_witness_commitment_segment(&segment_header(1)),
        Err(WitnessCommitmentSegmentError::LengthOverflow)
    ));
}

use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, WitnessCommitmentSegmentError,
};

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

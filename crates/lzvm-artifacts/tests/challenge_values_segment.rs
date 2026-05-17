use lzvm_artifacts::challenge_values_segment::{
    encode_challenge_values_segment, parse_challenge_values_segment, ChallengeValuesSegment,
    ChallengeValuesSegmentError,
};

fn segment_header(value_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"cvs0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, value_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn rejects_unsupported_challenge_values_segment_versions() {
    let segment = ChallengeValuesSegment {
        values: vec![[1, 2, 3]],
    };
    let mut encoded = encode_challenge_values_segment(&segment).expect("segment should encode");
    encoded[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert!(matches!(
        parse_challenge_values_segment(&encoded),
        Err(ChallengeValuesSegmentError::UnsupportedVersion { version: 2 })
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_extensions() {
    assert!(matches!(
        parse_challenge_values_segment(&segment_header(1)),
        Err(ChallengeValuesSegmentError::LengthOverflow)
    ));
}

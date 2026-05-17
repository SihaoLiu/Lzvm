use lzvm_artifacts::group_values_segment::{
    encode_group_values_segment, parse_group_values_segment, GroupValuesSegment,
    GroupValuesSegmentError,
};

fn segment_header(value_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"gvs0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, value_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn round_trips_group_values_segment() {
    let segment = GroupValuesSegment {
        values: vec![[1, 2, 3], [4, 5, 6]],
    };

    let encoded = encode_group_values_segment(&segment).expect("segment should encode");
    let parsed = parse_group_values_segment(&encoded).expect("segment should parse");

    assert_eq!(parsed, segment);
}

#[test]
fn rejects_empty_group_values_segment() {
    let error = encode_group_values_segment(&GroupValuesSegment { values: vec![] })
        .expect_err("empty segment should be rejected");

    assert_eq!(error, GroupValuesSegmentError::EmptyValues);
}

#[test]
fn rejects_value_count_that_exceeds_remaining_extensions() {
    assert!(matches!(
        parse_group_values_segment(&segment_header(1)),
        Err(GroupValuesSegmentError::LengthOverflow)
    ));
}

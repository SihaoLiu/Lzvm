use lzvm_artifacts::group_values_segment::{
    encode_group_values_segment, parse_group_values_segment, GroupValuesSegment,
    GroupValuesSegmentError,
};

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

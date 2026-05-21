use lzvm_artifacts::group_values_segment::{
    encode_group_values_segment, parse_group_values_segment, GroupValuesSegment,
    GroupValuesSegmentError,
};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const FIRST_VALUE_OFFSET: usize = 12;

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
fn rejects_unsupported_group_values_segment_versions() {
    let segment = GroupValuesSegment {
        values: vec![[1, 2, 3]],
    };
    let mut encoded = encode_group_values_segment(&segment).expect("segment should encode");
    encoded[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert!(matches!(
        parse_group_values_segment(&encoded),
        Err(GroupValuesSegmentError::UnsupportedVersion { version: 2 })
    ));
}

#[test]
fn rejects_empty_group_values_segment() {
    let error = encode_group_values_segment(&GroupValuesSegment { values: vec![] })
        .expect_err("empty segment should be rejected");

    assert_eq!(error, GroupValuesSegmentError::EmptyValues);
}

#[test]
fn rejects_non_canonical_group_values() {
    let mut segment = GroupValuesSegment {
        values: vec![[1, 2, 3], [4, 5, 6]],
    };
    segment.values[1][2] = NON_CANONICAL_FIELD;

    let err = encode_group_values_segment(&segment).expect_err("value should be rejected");
    assert_eq!(
        err.to_string(),
        "group values value 1 word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_group_values_when_parsing() {
    let segment = GroupValuesSegment {
        values: vec![[1, 2, 3]],
    };
    let mut encoded = encode_group_values_segment(&segment).expect("segment should encode");
    encoded[FIRST_VALUE_OFFSET + 8..FIRST_VALUE_OFFSET + 16]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_group_values_segment(&encoded).expect_err("value should be rejected");
    assert_eq!(
        err.to_string(),
        "group values value 0 word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_value_count_that_exceeds_remaining_extensions() {
    assert!(matches!(
        parse_group_values_segment(&segment_header(1)),
        Err(GroupValuesSegmentError::LengthOverflow)
    ));
}

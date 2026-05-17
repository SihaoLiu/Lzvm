use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, parse_constant_opening_segment, ConstantOpeningLevelSegment,
    ConstantOpeningQuerySegment, ConstantOpeningSegment, ConstantOpeningSegmentError,
    ConstantOpeningUnitSegment,
};

fn sample_segment() -> ConstantOpeningSegment {
    ConstantOpeningSegment {
        units: vec![ConstantOpeningUnitSegment {
            unit_index: 0,
            queries: vec![ConstantOpeningQuerySegment {
                row_index: 3,
                values: vec![11, 12],
                siblings: vec![
                    ConstantOpeningLevelSegment {
                        siblings: vec![[1, 2, 3, 4]],
                    },
                    ConstantOpeningLevelSegment {
                        siblings: vec![[5, 6, 7, 8]],
                    },
                ],
            }],
        }],
    }
}

fn segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"cos0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, unit_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn encodes_and_parses_constant_opening_segments() {
    let encoded =
        encode_constant_opening_segment(&sample_segment()).expect("opening segment should encode");
    let parsed = parse_constant_opening_segment(&encoded).expect("opening segment should parse");

    assert_eq!(&encoded[0..4], b"cos0");
    assert_eq!(parsed, sample_segment());
}

#[test]
fn rejects_unsupported_constant_opening_segment_versions() {
    let mut encoded =
        encode_constant_opening_segment(&sample_segment()).expect("opening segment should encode");
    encoded[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert!(matches!(
        parse_constant_opening_segment(&encoded),
        Err(ConstantOpeningSegmentError::UnsupportedVersion { version: 2 })
    ));
}

#[test]
fn rejects_empty_constant_opening_segments() {
    let segment = ConstantOpeningSegment { units: Vec::new() };

    assert!(matches!(
        encode_constant_opening_segment(&segment),
        Err(ConstantOpeningSegmentError::EmptyUnits)
    ));
}

#[test]
fn rejects_constant_opening_queries_without_values() {
    let segment = ConstantOpeningSegment {
        units: vec![ConstantOpeningUnitSegment {
            unit_index: 0,
            queries: vec![ConstantOpeningQuerySegment {
                row_index: 3,
                values: Vec::new(),
                siblings: Vec::new(),
            }],
        }],
    };

    assert!(matches!(
        encode_constant_opening_segment(&segment),
        Err(ConstantOpeningSegmentError::EmptyValues {
            unit_index: 0,
            row_index: 3
        })
    ));
}

#[test]
fn encodes_duplicate_constant_opening_rows() {
    let mut segment = sample_segment();
    let query = segment.units[0].queries[0].clone();
    segment.units[0].queries.push(query);

    let encoded = encode_constant_opening_segment(&segment).expect("duplicate rows should encode");
    let parsed = parse_constant_opening_segment(&encoded).expect("opening segment should parse");

    assert_eq!(parsed, segment);
}

#[test]
fn rejects_truncated_constant_opening_segments() {
    assert!(matches!(
        parse_constant_opening_segment(b"cos0\x01\0"),
        Err(ConstantOpeningSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_unit_headers() {
    assert!(matches!(
        parse_constant_opening_segment(&segment_header(1)),
        Err(ConstantOpeningSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_query_count_that_exceeds_remaining_query_headers() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_constant_opening_segment(&bytes),
        Err(ConstantOpeningSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_words() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 3);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 0);

    assert!(matches!(
        parse_constant_opening_segment(&bytes),
        Err(ConstantOpeningSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_level_count_that_exceeds_remaining_level_headers() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 3);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 11);

    assert!(matches!(
        parse_constant_opening_segment(&bytes),
        Err(ConstantOpeningSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_sibling_count_that_exceeds_remaining_digests() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 3);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 11);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_constant_opening_segment(&bytes),
        Err(ConstantOpeningSegmentError::LengthOverflow)
    ));
}

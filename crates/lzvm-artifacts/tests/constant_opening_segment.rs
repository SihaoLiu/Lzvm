use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, parse_constant_opening_segment, ConstantOpeningLevelSegment,
    ConstantOpeningQuerySegment, ConstantOpeningSegment, ConstantOpeningSegmentError,
    ConstantOpeningUnitSegment,
};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const FIRST_VALUE_OFFSET: usize = 12 + 4 + 4 + 8 + 4 + 4;
const FIRST_SIBLING_OFFSET: usize = FIRST_VALUE_OFFSET + 2 * 8 + 4;
const SECOND_SIBLING_OFFSET: usize = FIRST_SIBLING_OFFSET + 4 * 8 + 4;

fn sample_segment() -> ConstantOpeningSegment {
    ConstantOpeningSegment {
        units: vec![ConstantOpeningUnitSegment {
            unit_index: 0,
            trace_instance_index: 0,
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

fn v2_segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"cos0");
    push_u32(&mut bytes, 2);
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
fn encodes_and_parses_trace_instance_constant_opening_segments() {
    let mut segment = sample_segment();
    let mut later = segment.units[0].clone();
    later.trace_instance_index = 1;
    later.queries[0].values[0] = 21;
    segment.units.push(later);

    let encoded = encode_constant_opening_segment(&segment).expect("opening segment should encode");
    let parsed = parse_constant_opening_segment(&encoded).expect("opening segment should parse");

    assert_eq!(&encoded[4..8], &2_u32.to_le_bytes());
    assert_eq!(parsed, segment);
}

#[test]
fn parses_legacy_constant_opening_units_as_base_trace_instances() {
    let encoded =
        encode_constant_opening_segment(&sample_segment()).expect("opening segment should encode");
    let parsed = parse_constant_opening_segment(&encoded).expect("opening segment should parse");

    assert_eq!(&encoded[4..8], &1_u32.to_le_bytes());
    assert_eq!(parsed.units[0].trace_instance_index, 0);
}

#[test]
fn rejects_duplicate_trace_instance_constant_opening_units() {
    let mut segment = sample_segment();
    segment.units[0].trace_instance_index = 1;
    segment.units.push(segment.units[0].clone());

    assert!(matches!(
        encode_constant_opening_segment(&segment),
        Err(ConstantOpeningSegmentError::DuplicateUnitIdentity {
            unit_index: 0,
            trace_instance_index: 1
        })
    ));
}

#[test]
fn rejects_non_canonical_constant_opening_values() {
    let mut segment = sample_segment();
    segment.units[0].queries[0].values[1] = NON_CANONICAL_FIELD;

    let err = encode_constant_opening_segment(&segment).expect_err("value should reject");

    assert_eq!(
        err.to_string(),
        "constant opening unit 0 row 3 value 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_constant_opening_values_when_parsing() {
    let mut encoded =
        encode_constant_opening_segment(&sample_segment()).expect("opening segment should encode");
    encoded[FIRST_VALUE_OFFSET..FIRST_VALUE_OFFSET + 8]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_constant_opening_segment(&encoded).expect_err("value should reject");

    assert_eq!(
        err.to_string(),
        "constant opening unit 0 row 3 value 0 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_constant_opening_sibling_roots() {
    let mut segment = sample_segment();
    segment.units[0].queries[0].siblings[1].siblings[0][2] = NON_CANONICAL_FIELD;

    let err = encode_constant_opening_segment(&segment).expect_err("sibling root should reject");

    assert_eq!(
        err.to_string(),
        "constant opening unit 0 row 3 sibling level 1 root 0 word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_constant_opening_sibling_roots_when_parsing() {
    let mut encoded =
        encode_constant_opening_segment(&sample_segment()).expect("opening segment should encode");
    encoded[SECOND_SIBLING_OFFSET + 16..SECOND_SIBLING_OFFSET + 24]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_constant_opening_segment(&encoded).expect_err("sibling root should reject");

    assert_eq!(
        err.to_string(),
        "constant opening unit 0 row 3 sibling level 1 root 0 word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_unsupported_constant_opening_segment_versions() {
    let mut encoded =
        encode_constant_opening_segment(&sample_segment()).expect("opening segment should encode");
    encoded[4..8].copy_from_slice(&3_u32.to_le_bytes());

    assert!(matches!(
        parse_constant_opening_segment(&encoded),
        Err(ConstantOpeningSegmentError::UnsupportedVersion { version: 3 })
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
            trace_instance_index: 0,
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
        Err(ConstantOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_query_count_that_exceeds_remaining_query_headers() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_constant_opening_segment(&bytes),
        Err(ConstantOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_v2_query_count_that_exceeds_remaining_query_headers() {
    let mut bytes = v2_segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_constant_opening_segment(&bytes),
        Err(ConstantOpeningSegmentError::UnexpectedEof { .. })
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
        Err(ConstantOpeningSegmentError::UnexpectedEof { .. })
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
        Err(ConstantOpeningSegmentError::UnexpectedEof { .. })
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
        Err(ConstantOpeningSegmentError::UnexpectedEof { .. })
    ));
}

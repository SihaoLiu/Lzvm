use lzvm_artifacts::unit_values_segment::{
    encode_unit_values_segment, parse_unit_values_segment, UnitValuesSegment,
    UnitValuesSegmentError, UnitValuesUnitSegment,
};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const FIRST_VALUE_OFFSET: usize = 12 + 4 + 4;

fn sample_segment() -> UnitValuesSegment {
    UnitValuesSegment {
        units: vec![
            UnitValuesUnitSegment {
                unit_index: 0,
                trace_instance_index: 0,
                values: vec![1, 2, 3, 4],
            },
            UnitValuesUnitSegment {
                unit_index: 2,
                trace_instance_index: 0,
                values: vec![5, 6],
            },
        ],
    }
}

fn segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"uvs0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, unit_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn encodes_and_parses_unit_values_segments() {
    let encoded = encode_unit_values_segment(&sample_segment()).expect("segment should encode");
    let parsed = parse_unit_values_segment(&encoded).expect("segment should parse");

    assert_eq!(&encoded[0..4], b"uvs0");
    assert_eq!(u32::from_le_bytes(encoded[4..8].try_into().unwrap()), 1);
    assert_eq!(parsed, sample_segment());
}

#[test]
fn encodes_and_parses_trace_instance_unit_values_segments() {
    let segment = UnitValuesSegment {
        units: vec![
            UnitValuesUnitSegment {
                unit_index: 1,
                trace_instance_index: 0,
                values: vec![11],
            },
            UnitValuesUnitSegment {
                unit_index: 1,
                trace_instance_index: 2,
                values: vec![13],
            },
        ],
    };

    let encoded = encode_unit_values_segment(&segment).expect("segment should encode");
    let parsed = parse_unit_values_segment(&encoded).expect("segment should parse");

    assert_eq!(u32::from_le_bytes(encoded[4..8].try_into().unwrap()), 2);
    assert_eq!(parsed, segment);
}

#[test]
fn parses_legacy_unit_values_segments_as_base_trace_instances() {
    let encoded = encode_unit_values_segment(&sample_segment()).expect("segment should encode");
    let parsed = parse_unit_values_segment(&encoded).expect("segment should parse");

    assert!(parsed
        .units
        .iter()
        .all(|unit| unit.trace_instance_index == 0));
}

#[test]
fn rejects_unsupported_unit_values_segment_versions() {
    let mut encoded = encode_unit_values_segment(&sample_segment()).expect("segment should encode");
    encoded[4..8].copy_from_slice(&3_u32.to_le_bytes());

    assert!(matches!(
        parse_unit_values_segment(&encoded),
        Err(UnitValuesSegmentError::UnsupportedVersion { version: 3 })
    ));
}

#[test]
fn rejects_empty_unit_values_segments() {
    let segment = UnitValuesSegment { units: Vec::new() };

    assert!(matches!(
        encode_unit_values_segment(&segment),
        Err(UnitValuesSegmentError::EmptyUnits)
    ));
}

#[test]
fn rejects_non_canonical_unit_values() {
    let mut segment = sample_segment();
    segment.units[1].values[0] = NON_CANONICAL_FIELD;

    let err = encode_unit_values_segment(&segment).expect_err("value should be rejected");
    assert_eq!(
        err.to_string(),
        "unit values unit 2 value 0 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_unit_values_when_parsing() {
    let mut encoded = encode_unit_values_segment(&sample_segment()).expect("segment should encode");
    encoded[FIRST_VALUE_OFFSET + 8..FIRST_VALUE_OFFSET + 16]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_unit_values_segment(&encoded).expect_err("value should be rejected");
    assert_eq!(
        err.to_string(),
        "unit values unit 0 value 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_unit_values_units_without_values() {
    let segment = UnitValuesSegment {
        units: vec![UnitValuesUnitSegment {
            unit_index: 0,
            trace_instance_index: 0,
            values: Vec::new(),
        }],
    };

    assert!(matches!(
        encode_unit_values_segment(&segment),
        Err(UnitValuesSegmentError::EmptyValues { unit_index: 0 })
    ));
}

#[test]
fn rejects_duplicate_unit_values_units() {
    let segment = UnitValuesSegment {
        units: vec![
            UnitValuesUnitSegment {
                unit_index: 1,
                trace_instance_index: 0,
                values: vec![1],
            },
            UnitValuesUnitSegment {
                unit_index: 1,
                trace_instance_index: 0,
                values: vec![2],
            },
        ],
    };

    assert!(matches!(
        encode_unit_values_segment(&segment),
        Err(UnitValuesSegmentError::DuplicateUnitIndex { unit_index: 1 })
    ));
}

#[test]
fn rejects_duplicate_unit_values_unit_identities() {
    let segment = UnitValuesSegment {
        units: vec![
            UnitValuesUnitSegment {
                unit_index: 1,
                trace_instance_index: 2,
                values: vec![1],
            },
            UnitValuesUnitSegment {
                unit_index: 1,
                trace_instance_index: 2,
                values: vec![2],
            },
        ],
    };

    assert!(matches!(
        encode_unit_values_segment(&segment),
        Err(UnitValuesSegmentError::DuplicateUnitIdentity {
            unit_index: 1,
            trace_instance_index: 2
        })
    ));
}

#[test]
fn rejects_truncated_unit_values_segments() {
    let result = parse_unit_values_segment(b"uvs0\x01\0");

    assert!(matches!(
        result,
        Err(UnitValuesSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_unit_headers() {
    let result = parse_unit_values_segment(&segment_header(1));

    assert!(matches!(
        result,
        Err(UnitValuesSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_words() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_unit_values_segment(&bytes),
        Err(UnitValuesSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_trailing_unit_values_bytes() {
    let mut encoded = encode_unit_values_segment(&sample_segment()).expect("segment should encode");
    encoded.push(0);

    assert!(matches!(
        parse_unit_values_segment(&encoded),
        Err(UnitValuesSegmentError::TrailingBytes { trailing: 1 })
    ));
}

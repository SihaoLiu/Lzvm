use lzvm_artifacts::unit_values_segment::{
    encode_unit_values_segment, parse_unit_values_segment, UnitValuesSegment,
    UnitValuesSegmentError, UnitValuesUnitSegment,
};

fn sample_segment() -> UnitValuesSegment {
    UnitValuesSegment {
        units: vec![
            UnitValuesUnitSegment {
                unit_index: 0,
                values: vec![1, 2, 3, 4],
            },
            UnitValuesUnitSegment {
                unit_index: 2,
                values: vec![5, 6],
            },
        ],
    }
}

#[test]
fn encodes_and_parses_unit_values_segments() {
    let encoded = encode_unit_values_segment(&sample_segment()).expect("segment should encode");
    let parsed = parse_unit_values_segment(&encoded).expect("segment should parse");

    assert_eq!(&encoded[0..4], b"uvs0");
    assert_eq!(parsed, sample_segment());
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
fn rejects_unit_values_units_without_values() {
    let segment = UnitValuesSegment {
        units: vec![UnitValuesUnitSegment {
            unit_index: 0,
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
                values: vec![1],
            },
            UnitValuesUnitSegment {
                unit_index: 1,
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
fn rejects_truncated_unit_values_segments() {
    let result = parse_unit_values_segment(b"uvs0\x01\0\0\0\x01\0\0\0");

    assert!(matches!(
        result,
        Err(UnitValuesSegmentError::UnexpectedEof {
            needed: 16,
            available: 12
        })
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

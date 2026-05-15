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

#[test]
fn encodes_and_parses_constant_opening_segments() {
    let encoded =
        encode_constant_opening_segment(&sample_segment()).expect("opening segment should encode");
    let parsed = parse_constant_opening_segment(&encoded).expect("opening segment should parse");

    assert_eq!(&encoded[0..4], b"cos0");
    assert_eq!(parsed, sample_segment());
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

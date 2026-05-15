use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, parse_witness_opening_segment, WitnessOpeningLevelSegment,
    WitnessOpeningQuerySegment, WitnessOpeningSegment, WitnessOpeningSegmentError,
    WitnessOpeningStageSegment, WitnessOpeningUnitSegment,
};

fn sample_segment() -> WitnessOpeningSegment {
    WitnessOpeningSegment {
        units: vec![WitnessOpeningUnitSegment {
            unit_index: 0,
            queries: vec![WitnessOpeningQuerySegment {
                row_index: 3,
                stages: vec![WitnessOpeningStageSegment {
                    stage_index: 1,
                    values: vec![11, 12],
                    siblings: vec![
                        WitnessOpeningLevelSegment {
                            siblings: vec![[1, 2, 3, 4]],
                        },
                        WitnessOpeningLevelSegment {
                            siblings: vec![[5, 6, 7, 8]],
                        },
                    ],
                }],
            }],
        }],
    }
}

#[test]
fn encodes_and_parses_witness_opening_segments() {
    let encoded =
        encode_witness_opening_segment(&sample_segment()).expect("opening segment should encode");
    let parsed = parse_witness_opening_segment(&encoded).expect("opening segment should parse");

    assert_eq!(&encoded[0..4], b"wos0");
    assert_eq!(parsed, sample_segment());
}

#[test]
fn rejects_empty_witness_opening_segments() {
    let segment = WitnessOpeningSegment { units: Vec::new() };

    assert!(matches!(
        encode_witness_opening_segment(&segment),
        Err(WitnessOpeningSegmentError::EmptyUnits)
    ));
}

#[test]
fn rejects_witness_opening_queries_without_stages() {
    let segment = WitnessOpeningSegment {
        units: vec![WitnessOpeningUnitSegment {
            unit_index: 0,
            queries: vec![WitnessOpeningQuerySegment {
                row_index: 3,
                stages: Vec::new(),
            }],
        }],
    };

    assert!(matches!(
        encode_witness_opening_segment(&segment),
        Err(WitnessOpeningSegmentError::EmptyStages {
            unit_index: 0,
            row_index: 3
        })
    ));
}

#[test]
fn rejects_duplicate_witness_opening_units() {
    let mut segment = sample_segment();
    segment.units.push(segment.units[0].clone());

    assert!(matches!(
        encode_witness_opening_segment(&segment),
        Err(WitnessOpeningSegmentError::DuplicateUnitIndex { unit_index: 0 })
    ));
}

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

fn segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"wos0");
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
fn encodes_and_parses_witness_opening_segments() {
    let encoded =
        encode_witness_opening_segment(&sample_segment()).expect("opening segment should encode");
    let parsed = parse_witness_opening_segment(&encoded).expect("opening segment should parse");

    assert_eq!(&encoded[0..4], b"wos0");
    assert_eq!(parsed, sample_segment());
}

#[test]
fn rejects_unsupported_witness_opening_segment_versions() {
    let mut encoded =
        encode_witness_opening_segment(&sample_segment()).expect("opening segment should encode");
    encoded[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert!(matches!(
        parse_witness_opening_segment(&encoded),
        Err(WitnessOpeningSegmentError::UnsupportedVersion { version: 2 })
    ));
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

#[test]
fn rejects_truncated_witness_opening_segments() {
    assert!(matches!(
        parse_witness_opening_segment(b"wos0\x01\0"),
        Err(WitnessOpeningSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_unit_headers() {
    assert!(matches!(
        parse_witness_opening_segment(&segment_header(1)),
        Err(WitnessOpeningSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_query_count_that_exceeds_remaining_query_headers() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_witness_opening_segment(&bytes),
        Err(WitnessOpeningSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_stage_count_that_exceeds_remaining_stage_headers() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 3);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_witness_opening_segment(&bytes),
        Err(WitnessOpeningSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_words() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 3);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 0);

    assert!(matches!(
        parse_witness_opening_segment(&bytes),
        Err(WitnessOpeningSegmentError::LengthOverflow)
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
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 11);

    assert!(matches!(
        parse_witness_opening_segment(&bytes),
        Err(WitnessOpeningSegmentError::LengthOverflow)
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
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 11);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_witness_opening_segment(&bytes),
        Err(WitnessOpeningSegmentError::LengthOverflow)
    ));
}

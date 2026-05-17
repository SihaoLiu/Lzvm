use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, parse_pcs_evaluation_segment, PcsEvaluationSegment,
    PcsEvaluationSegmentError, PcsEvaluationUnitSegment,
};

fn sample_segment() -> PcsEvaluationSegment {
    PcsEvaluationSegment {
        units: vec![
            PcsEvaluationUnitSegment {
                unit_index: 0,
                values: vec![[1, 2, 3], [4, 5, 6]],
            },
            PcsEvaluationUnitSegment {
                unit_index: 2,
                values: vec![[7, 8, 9]],
            },
        ],
    }
}

fn segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"evs0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, unit_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn encodes_and_parses_pcs_evaluation_segments() {
    let encoded =
        encode_pcs_evaluation_segment(&sample_segment()).expect("evaluation segment should encode");
    let parsed = parse_pcs_evaluation_segment(&encoded).expect("evaluation segment should parse");

    assert_eq!(&encoded[0..4], b"evs0");
    assert_eq!(parsed, sample_segment());
}

#[test]
fn rejects_empty_pcs_evaluation_segments() {
    let segment = PcsEvaluationSegment { units: Vec::new() };

    assert!(matches!(
        encode_pcs_evaluation_segment(&segment),
        Err(PcsEvaluationSegmentError::EmptyUnits)
    ));
}

#[test]
fn rejects_pcs_evaluation_units_without_values() {
    let segment = PcsEvaluationSegment {
        units: vec![PcsEvaluationUnitSegment {
            unit_index: 0,
            values: Vec::new(),
        }],
    };

    assert!(matches!(
        encode_pcs_evaluation_segment(&segment),
        Err(PcsEvaluationSegmentError::EmptyValues { unit_index: 0 })
    ));
}

#[test]
fn rejects_duplicate_pcs_evaluation_units() {
    let segment = PcsEvaluationSegment {
        units: vec![
            PcsEvaluationUnitSegment {
                unit_index: 1,
                values: vec![[1, 2, 3]],
            },
            PcsEvaluationUnitSegment {
                unit_index: 1,
                values: vec![[4, 5, 6]],
            },
        ],
    };

    assert!(matches!(
        encode_pcs_evaluation_segment(&segment),
        Err(PcsEvaluationSegmentError::DuplicateUnitIndex { unit_index: 1 })
    ));
}

#[test]
fn rejects_truncated_pcs_evaluation_segments() {
    let result = parse_pcs_evaluation_segment(b"evs0\x01\0");

    assert!(matches!(
        result,
        Err(PcsEvaluationSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_unit_headers() {
    assert!(matches!(
        parse_pcs_evaluation_segment(&segment_header(1)),
        Err(PcsEvaluationSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_extensions() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_pcs_evaluation_segment(&bytes),
        Err(PcsEvaluationSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_trailing_pcs_evaluation_bytes() {
    let mut encoded =
        encode_pcs_evaluation_segment(&sample_segment()).expect("evaluation segment should encode");
    encoded.push(0);

    assert!(matches!(
        parse_pcs_evaluation_segment(&encoded),
        Err(PcsEvaluationSegmentError::TrailingBytes { trailing: 1 })
    ));
}

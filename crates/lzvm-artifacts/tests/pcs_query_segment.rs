use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, parse_pcs_query_plan_segment, PcsQueryPlanSegment,
    PcsQueryPlanSegmentError, PcsQueryPlanUnit,
};

fn sample_segment() -> PcsQueryPlanSegment {
    PcsQueryPlanSegment {
        units: vec![
            PcsQueryPlanUnit {
                unit_index: 0,
                queries: vec![3, 11, 19],
            },
            PcsQueryPlanUnit {
                unit_index: 2,
                queries: vec![7, 23],
            },
        ],
    }
}

fn segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"pqs0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, unit_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn encodes_and_parses_pcs_query_plan_segments() {
    let encoded =
        encode_pcs_query_plan_segment(&sample_segment()).expect("query segment should encode");
    let parsed = parse_pcs_query_plan_segment(&encoded).expect("query segment should parse");

    assert_eq!(&encoded[0..4], b"pqs0");
    assert_eq!(parsed, sample_segment());
}

#[test]
fn rejects_empty_pcs_query_plan_segments() {
    let segment = PcsQueryPlanSegment { units: Vec::new() };

    assert!(matches!(
        encode_pcs_query_plan_segment(&segment),
        Err(PcsQueryPlanSegmentError::EmptyUnits)
    ));
}

#[test]
fn rejects_pcs_query_plan_units_without_queries() {
    let segment = PcsQueryPlanSegment {
        units: vec![PcsQueryPlanUnit {
            unit_index: 0,
            queries: Vec::new(),
        }],
    };

    assert!(matches!(
        encode_pcs_query_plan_segment(&segment),
        Err(PcsQueryPlanSegmentError::EmptyQueries { unit_index: 0 })
    ));
}

#[test]
fn rejects_duplicate_pcs_query_plan_units() {
    let mut segment = sample_segment();
    segment.units[1].unit_index = 0;

    assert!(matches!(
        encode_pcs_query_plan_segment(&segment),
        Err(PcsQueryPlanSegmentError::DuplicateUnitIndex { unit_index: 0 })
    ));
}

#[test]
fn rejects_truncated_pcs_query_plan_segments() {
    let result = parse_pcs_query_plan_segment(b"pqs0\x01\0");

    assert!(matches!(
        result,
        Err(PcsQueryPlanSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_unit_headers() {
    assert!(matches!(
        parse_pcs_query_plan_segment(&segment_header(1)),
        Err(PcsQueryPlanSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_query_count_that_exceeds_remaining_queries() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_pcs_query_plan_segment(&bytes),
        Err(PcsQueryPlanSegmentError::LengthOverflow)
    ));
}

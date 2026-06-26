use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, parse_pcs_query_plan_segment, PcsQueryPlanSegment,
    PcsQueryPlanSegmentError, PcsQueryPlanUnit,
};

const HEADER_BYTES: usize = 12;
const V1_UNIT_HEADER_BYTES: usize = 4 + 4;
const V2_UNIT_HEADER_BYTES: usize = 4 + 4 + 4;
const QUERY_BYTES: usize = 8;

fn sample_segment() -> PcsQueryPlanSegment {
    PcsQueryPlanSegment {
        units: vec![
            PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 0,
                queries: vec![3, 11, 19],
            },
            PcsQueryPlanUnit {
                unit_index: 2,
                trace_instance_index: 0,
                queries: vec![7, 23],
            },
        ],
    }
}

fn segment_header(unit_count: u32) -> Vec<u8> {
    segment_header_with_version(1, unit_count)
}

fn segment_header_with_version(version: u32, unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"pqs0");
    push_u32(&mut bytes, version);
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
    let expected = v1_sample_segment_bytes();

    assert_eq!(&encoded[0..4], b"pqs0");
    assert_eq!(
        u32::from_le_bytes(encoded[4..8].try_into().expect("version bytes")),
        1
    );
    assert_eq!(encoded, expected);
    assert_eq!(parsed, sample_segment());
}

#[test]
fn encodes_and_parses_trace_instance_pcs_query_plan_segments() {
    let segment = PcsQueryPlanSegment {
        units: vec![
            PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 0,
                queries: vec![3, 11],
            },
            PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 1,
                queries: vec![7, 23],
            },
        ],
    };

    let encoded = encode_pcs_query_plan_segment(&segment).expect("query segment should encode");
    let parsed = parse_pcs_query_plan_segment(&encoded).expect("query segment should parse");

    assert_eq!(
        u32::from_le_bytes(encoded[4..8].try_into().expect("version bytes")),
        2
    );
    assert_eq!(parsed, segment);
}

#[test]
fn rejects_unsupported_pcs_query_plan_segment_versions() {
    let mut encoded =
        encode_pcs_query_plan_segment(&sample_segment()).expect("query segment should encode");
    encoded[4..8].copy_from_slice(&3_u32.to_le_bytes());

    assert!(matches!(
        parse_pcs_query_plan_segment(&encoded),
        Err(PcsQueryPlanSegmentError::UnsupportedVersion { version: 3 })
    ));
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
            trace_instance_index: 0,
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
fn rejects_duplicate_trace_instance_pcs_query_plan_units() {
    let segment = PcsQueryPlanSegment {
        units: vec![
            PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 1,
                queries: vec![3],
            },
            PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 1,
                queries: vec![7],
            },
        ],
    };

    assert!(matches!(
        encode_pcs_query_plan_segment(&segment),
        Err(PcsQueryPlanSegmentError::DuplicateUnitIdentity {
            unit_index: 0,
            trace_instance_index: 1
        })
    ));
    assert!(matches!(
        parse_pcs_query_plan_segment(&duplicate_trace_instance_segment_bytes()),
        Err(PcsQueryPlanSegmentError::DuplicateUnitIdentity {
            unit_index: 0,
            trace_instance_index: 1
        })
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

fn v1_sample_segment_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"pqs0");
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&2_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&3_u64.to_le_bytes());
    out.extend_from_slice(&11_u64.to_le_bytes());
    out.extend_from_slice(&19_u64.to_le_bytes());
    out.extend_from_slice(&2_u32.to_le_bytes());
    out.extend_from_slice(&2_u32.to_le_bytes());
    out.extend_from_slice(&7_u64.to_le_bytes());
    out.extend_from_slice(&23_u64.to_le_bytes());
    out
}

fn duplicate_trace_instance_segment_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"pqs0");
    out.extend_from_slice(&2_u32.to_le_bytes());
    out.extend_from_slice(&2_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&3_u64.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&7_u64.to_le_bytes());
    out
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_unit_headers() {
    assert!(matches!(
        parse_pcs_query_plan_segment(&segment_header(1)),
        Err(PcsQueryPlanSegmentError::UnexpectedEof {
            needed,
            available: HEADER_BYTES
        }) if needed == HEADER_BYTES + V1_UNIT_HEADER_BYTES
    ));
}

#[test]
fn rejects_v2_unit_count_that_exceeds_remaining_unit_headers() {
    assert!(matches!(
        parse_pcs_query_plan_segment(&segment_header_with_version(2, 1)),
        Err(PcsQueryPlanSegmentError::UnexpectedEof {
            needed,
            available: HEADER_BYTES
        }) if needed == HEADER_BYTES + V2_UNIT_HEADER_BYTES
    ));
}

#[test]
fn rejects_query_count_that_exceeds_remaining_queries() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_pcs_query_plan_segment(&bytes),
        Err(PcsQueryPlanSegmentError::UnexpectedEof {
            needed,
            available
        }) if needed == HEADER_BYTES + V1_UNIT_HEADER_BYTES + QUERY_BYTES
            && available == HEADER_BYTES + V1_UNIT_HEADER_BYTES
    ));
}

#[test]
fn rejects_truncated_pcs_query_payload() {
    let mut encoded =
        encode_pcs_query_plan_segment(&sample_segment()).expect("query segment should encode");
    encoded.pop();

    assert!(matches!(
        parse_pcs_query_plan_segment(&encoded),
        Err(PcsQueryPlanSegmentError::UnexpectedEof {
            needed,
            available
        }) if needed == HEADER_BYTES + V1_UNIT_HEADER_BYTES * 2 + QUERY_BYTES * 5
            && available == HEADER_BYTES + V1_UNIT_HEADER_BYTES * 2 + QUERY_BYTES * 5 - 1
    ));
}

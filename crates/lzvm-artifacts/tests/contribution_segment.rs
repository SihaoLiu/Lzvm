use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, parse_contribution_segment, ContributionEntry,
    ContributionSegment, ContributionSegmentError,
};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const FIRST_VALUE_OFFSET: usize = 12 + 4 + 4 + 4 + 4;

fn sample_segment() -> ContributionSegment {
    ContributionSegment {
        entries: vec![
            ContributionEntry {
                worker_index: 0,
                group_id: 0,
                aggregated: false,
                values: vec![1, 2, 3, 4],
            },
            ContributionEntry {
                worker_index: 2,
                group_id: 1,
                aggregated: true,
                values: vec![5, 6],
            },
        ],
    }
}

fn segment_header(entry_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"ctr0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, entry_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn encodes_and_parses_contribution_segments() {
    let encoded = encode_contribution_segment(&sample_segment()).expect("segment should encode");
    let parsed = parse_contribution_segment(&encoded).expect("segment should parse");

    assert_eq!(&encoded[0..4], b"ctr0");
    assert_eq!(parsed, sample_segment());
}

#[test]
fn rejects_unsupported_contribution_segment_versions() {
    let mut encoded =
        encode_contribution_segment(&sample_segment()).expect("segment should encode");
    encoded[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert!(matches!(
        parse_contribution_segment(&encoded),
        Err(ContributionSegmentError::UnsupportedVersion { version: 2 })
    ));
}

#[test]
fn rejects_empty_contribution_segments() {
    let segment = ContributionSegment {
        entries: Vec::new(),
    };

    assert!(matches!(
        encode_contribution_segment(&segment),
        Err(ContributionSegmentError::EmptyEntries)
    ));
}

#[test]
fn rejects_non_canonical_contribution_values() {
    let mut segment = sample_segment();
    segment.entries[1].values[1] = NON_CANONICAL_FIELD;

    let err = encode_contribution_segment(&segment).expect_err("value should be rejected");
    assert_eq!(
        err.to_string(),
        "contribution entry worker 2 group 1 value 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_contribution_values_when_parsing() {
    let mut encoded =
        encode_contribution_segment(&sample_segment()).expect("segment should encode");
    encoded[FIRST_VALUE_OFFSET + 8..FIRST_VALUE_OFFSET + 16]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_contribution_segment(&encoded).expect_err("value should be rejected");
    assert_eq!(
        err.to_string(),
        "contribution entry worker 0 group 0 value 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_contribution_entries_without_values() {
    let segment = ContributionSegment {
        entries: vec![ContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: Vec::new(),
        }],
    };

    assert!(matches!(
        encode_contribution_segment(&segment),
        Err(ContributionSegmentError::EmptyValues {
            worker_index: 0,
            group_id: 0
        })
    ));
}

#[test]
fn rejects_duplicate_contribution_entries() {
    let segment = ContributionSegment {
        entries: vec![
            ContributionEntry {
                worker_index: 1,
                group_id: 2,
                aggregated: false,
                values: vec![1],
            },
            ContributionEntry {
                worker_index: 1,
                group_id: 2,
                aggregated: true,
                values: vec![2],
            },
        ],
    };

    assert!(matches!(
        encode_contribution_segment(&segment),
        Err(ContributionSegmentError::DuplicateEntry {
            worker_index: 1,
            group_id: 2
        })
    ));
}

#[test]
fn rejects_invalid_aggregated_flags() {
    let mut encoded =
        encode_contribution_segment(&sample_segment()).expect("segment should encode");
    encoded[20] = 2;

    assert!(matches!(
        parse_contribution_segment(&encoded),
        Err(ContributionSegmentError::InvalidAggregatedFlag { value: 2 })
    ));
}

#[test]
fn rejects_truncated_contribution_segments() {
    let result = parse_contribution_segment(b"ctr0\x01\0");

    assert!(matches!(
        result,
        Err(ContributionSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_entry_count_that_exceeds_remaining_entry_headers() {
    assert!(matches!(
        parse_contribution_segment(&segment_header(1)),
        Err(ContributionSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_words() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_contribution_segment(&bytes),
        Err(ContributionSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_trailing_contribution_segment_bytes() {
    let mut encoded =
        encode_contribution_segment(&sample_segment()).expect("segment should encode");
    encoded.push(0);

    assert!(matches!(
        parse_contribution_segment(&encoded),
        Err(ContributionSegmentError::TrailingBytes { trailing: 1 })
    ));
}

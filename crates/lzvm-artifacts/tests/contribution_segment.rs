use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, parse_contribution_segment, ContributionEntry,
    ContributionSegment, ContributionSegmentError,
};

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

#[test]
fn encodes_and_parses_contribution_segments() {
    let encoded = encode_contribution_segment(&sample_segment()).expect("segment should encode");
    let parsed = parse_contribution_segment(&encoded).expect("segment should parse");

    assert_eq!(&encoded[0..4], b"ctr0");
    assert_eq!(parsed, sample_segment());
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
fn rejects_trailing_contribution_segment_bytes() {
    let mut encoded =
        encode_contribution_segment(&sample_segment()).expect("segment should encode");
    encoded.push(0);

    assert!(matches!(
        parse_contribution_segment(&encoded),
        Err(ContributionSegmentError::TrailingBytes { trailing: 1 })
    ));
}

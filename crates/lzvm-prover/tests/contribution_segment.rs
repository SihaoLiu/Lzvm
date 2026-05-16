use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, ContributionEntry, ContributionSegment, CONTRIBUTION_SEGMENT_ID,
};
use lzvm_field::{Felt, FieldError};
use lzvm_prover::contribution::{
    build_contribution_segment, load_contribution_segment_from_segments,
    LoadContributionSegmentError, ProveContributionEntry,
};

fn sample_entries() -> Vec<ProveContributionEntry> {
    vec![
        ProveContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(11), Felt::from_u64(22), Felt::from_u64(33)],
        },
        ProveContributionEntry {
            worker_index: 2,
            group_id: 1,
            aggregated: true,
            values: vec![Felt::from_u64(44), Felt::from_u64(55)],
        },
    ]
}

#[test]
fn round_trips_contribution_segments() {
    let entries = sample_entries();
    let segment = build_contribution_segment(&entries)
        .expect("segment should build")
        .expect("segment should exist");

    assert_eq!(segment.id, CONTRIBUTION_SEGMENT_ID);
    assert_eq!(
        load_contribution_segment_from_segments(&[segment]).expect("segment should load"),
        entries
    );
}

#[test]
fn rejects_non_canonical_contribution_values() {
    let bytes = encode_contribution_segment(&ContributionSegment {
        entries: vec![ContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![u64::MAX],
        }],
    })
    .expect("segment should encode");
    let segment = lzvm_artifacts::proof::ProofSegment {
        id: CONTRIBUTION_SEGMENT_ID,
        data: bytes,
    };

    assert!(matches!(
        load_contribution_segment_from_segments(&[segment]),
        Err(LoadContributionSegmentError::NonCanonicalValue {
            entry_index: 0,
            index: 0,
            source: FieldError::NonCanonical { value: u64::MAX },
        })
    ));
}

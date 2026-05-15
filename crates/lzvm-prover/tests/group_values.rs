use lzvm_artifacts::global_info::{AggregationType, CurveKind, GlobalAir, GlobalInfo};
use lzvm_artifacts::group_values_segment::{
    encode_group_values_segment, parse_group_values_segment, GroupValuesSegment,
    GROUP_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, FieldError, MODULUS};
use lzvm_prover::group_values::{
    build_group_values_segment, load_group_values_from_segments, LoadGroupValuesSegmentError,
    ProveGroupValuesSegmentError,
};

#[test]
fn builds_group_values_segment() {
    let segment = build_group_values_segment(&sample_global_info(1), &[ext([7, 8, 9])])
        .expect("segment should build")
        .expect("segment should be present");

    assert_eq!(segment.id, GROUP_VALUES_SEGMENT_ID);
    let parsed = parse_group_values_segment(&segment.data).expect("segment should parse");
    assert_eq!(parsed.values, vec![[7, 8, 9]]);
}

#[test]
fn loads_group_values_from_segments() {
    let segment = build_group_values_segment(&sample_global_info(1), &[ext([7, 8, 9])])
        .expect("segment should build")
        .expect("segment should be present");

    let values = load_group_values_from_segments(&sample_global_info(1), &[segment])
        .expect("segment should load");

    assert_eq!(values, vec![ext([7, 8, 9])]);
}

#[test]
fn rejects_missing_group_values_segment() {
    let error = load_group_values_from_segments(&sample_global_info(1), &[])
        .expect_err("required segment should be present");

    assert_eq!(error, LoadGroupValuesSegmentError::MissingSegment);
}

#[test]
fn rejects_unexpected_group_values_segment() {
    let error = load_group_values_from_segments(
        &sample_global_info(0),
        &[ProofSegment {
            id: GROUP_VALUES_SEGMENT_ID,
            data: vec![1, 2, 3, 4],
        }],
    )
    .expect_err("segment should not be present");

    assert_eq!(error, LoadGroupValuesSegmentError::UnexpectedSegment);
}

#[test]
fn rejects_loaded_group_values_count_mismatch() {
    let segment = group_values_segment([[7, 8, 9]]);

    let error = load_group_values_from_segments(&sample_global_info(2), &[segment])
        .expect_err("segment value count should match metadata");

    assert_eq!(
        error,
        LoadGroupValuesSegmentError::ValueCountMismatch {
            expected: 2,
            found: 1
        }
    );
}

#[test]
fn rejects_loaded_noncanonical_group_values() {
    let segment = group_values_segment([[MODULUS, 8, 9]]);

    let error = load_group_values_from_segments(&sample_global_info(1), &[segment])
        .expect_err("segment values should be canonical field elements");

    assert_eq!(
        error,
        LoadGroupValuesSegmentError::NonCanonicalValue {
            index: 0,
            source: FieldError::NonCanonical { value: MODULUS }
        }
    );
}

#[test]
fn omits_group_values_segment_when_metadata_declares_none() {
    let segment = build_group_values_segment(&sample_global_info(0), &[])
        .expect("empty values should be accepted");

    assert!(segment.is_none());
}

#[test]
fn rejects_unexpected_group_values() {
    let error = build_group_values_segment(&sample_global_info(0), &[ext([1, 2, 3])])
        .expect_err("unexpected values should fail");

    assert_eq!(
        error,
        ProveGroupValuesSegmentError::UnexpectedValues { found: 1 }
    );
}

#[test]
fn rejects_group_values_count_mismatch() {
    let error = build_group_values_segment(&sample_global_info(2), &[ext([1, 2, 3])])
        .expect_err("mismatch should fail");

    assert_eq!(
        error,
        ProveGroupValuesSegmentError::ValueCountMismatch {
            expected: 2,
            found: 1
        }
    );
}

fn ext(words: [u64; 3]) -> Ext3 {
    Ext3::from_u64s(words)
}

fn group_values_segment<const N: usize>(values: [[u64; 3]; N]) -> ProofSegment {
    ProofSegment {
        id: GROUP_VALUES_SEGMENT_ID,
        data: encode_group_values_segment(&GroupValuesSegment {
            values: values.to_vec(),
        })
        .expect("segment should encode"),
    }
}

fn sample_global_info(group_value_count: usize) -> GlobalInfo {
    GlobalInfo {
        name: "sample-program".to_owned(),
        air_groups: vec!["group-a".to_owned()],
        airs: vec![vec![GlobalAir {
            name: "unit-a".to_owned(),
            num_rows: 2,
            has_compressor: false,
        }]],
        curve: CurveKind::None,
        lattice_size: Some(368),
        aggregation_types: vec![(0..group_value_count)
            .map(|index| AggregationType {
                aggregation_type: index as u64,
            })
            .collect()],
        n_publics: 0,
        num_challenges: vec![1],
        num_proof_values: vec![],
        proof_values_map: vec![],
        publics_map: vec![],
        transcript_arity: 4,
    }
}

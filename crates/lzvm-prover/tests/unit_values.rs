use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::unit_values_segment::{parse_unit_values_segment, UNIT_VALUES_SEGMENT_ID};
use lzvm_field::Felt;
use lzvm_prover::unit_values::{
    build_unit_values_segment_from_packed_values, ProveUnitValuesSegmentError,
};

fn stage_value(name: &str, stage: u32) -> StageValue {
    StageValue {
        name: name.to_owned(),
        stage,
        lengths: Vec::new(),
    }
}

fn values(words: &[u64]) -> Vec<Felt> {
    words
        .iter()
        .copied()
        .map(|value| Felt::from_canonical(value).expect("value should be canonical"))
        .collect()
}

#[test]
fn builds_unit_values_segment_from_packed_values() {
    let map = vec![stage_value("unit.alpha", 1), stage_value("unit.beta", 2)];
    let segment = build_unit_values_segment_from_packed_values(3, &map, &values(&[11, 21, 22, 23]))
        .expect("segment should build")
        .expect("segment should be present");

    assert_eq!(segment.id, UNIT_VALUES_SEGMENT_ID);
    let parsed = parse_unit_values_segment(&segment.data).expect("segment should parse");
    assert_eq!(parsed.units.len(), 1);
    assert_eq!(parsed.units[0].unit_index, 3);
    assert_eq!(parsed.units[0].values, vec![11, 21, 22, 23]);
}

#[test]
fn omits_unit_values_segment_when_metadata_declares_none() {
    let segment = build_unit_values_segment_from_packed_values(0, &[], &[])
        .expect("empty values should be accepted");

    assert!(segment.is_none());
}

#[test]
fn rejects_unexpected_unit_values() {
    let error = build_unit_values_segment_from_packed_values(0, &[], &values(&[1]))
        .expect_err("unexpected values should be rejected");

    assert_eq!(
        error,
        ProveUnitValuesSegmentError::UnexpectedValues {
            unit_index: 0,
            found: 1
        }
    );
}

#[test]
fn rejects_unit_value_count_mismatches() {
    let map = vec![stage_value("unit.alpha", 1), stage_value("unit.beta", 2)];
    let error = build_unit_values_segment_from_packed_values(0, &map, &values(&[11, 21, 22]))
        .expect_err("short values should be rejected");

    assert_eq!(
        error,
        ProveUnitValuesSegmentError::ValueCountMismatch {
            unit_index: 0,
            expected: 4,
            found: 3
        }
    );
}

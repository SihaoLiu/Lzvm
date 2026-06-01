use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::unit_values_segment::{
    encode_unit_values_segment, parse_unit_values_segment, UnitValuesSegment,
    UnitValuesSegmentError, UnitValuesUnitSegment, UNIT_VALUES_SEGMENT_ID,
};
use lzvm_field::{Felt, FieldError, MODULUS};
use lzvm_prover::unit_values::{
    build_unit_values_segment_from_packed_values,
    build_unit_values_segment_from_packed_values_batch, expected_packed_unit_value_count,
    load_unit_values_from_segments, LoadUnitValuesSegmentError, ProveUnitValues,
    ProveUnitValuesSegmentError,
};

const FIRST_UNIT_VALUE_OFFSET: usize = 12 + 4 + 4;

fn stage_value(name: &str, stage: u32) -> StageValue {
    StageValue {
        name: name.to_owned(),
        stage,
        lengths: Vec::new(),
    }
}

fn stage_value_with_lengths(name: &str, stage: u32, lengths: &[u32]) -> StageValue {
    StageValue {
        name: name.to_owned(),
        stage,
        lengths: lengths.to_vec(),
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
fn builds_dimensioned_unit_values_segment_from_packed_values() {
    let map = vec![
        stage_value_with_lengths("unit.array", 1, &[2, 3]),
        stage_value_with_lengths("unit.extension", 2, &[2]),
    ];
    let packed_values: Vec<u64> = (1..=12).collect();

    let segment = build_unit_values_segment_from_packed_values(5, &map, &values(&packed_values))
        .expect("dimensioned segment should build")
        .expect("segment should be present");

    let parsed = parse_unit_values_segment(&segment.data).expect("segment should parse");
    assert_eq!(parsed.units.len(), 1);
    assert_eq!(parsed.units[0].unit_index, 5);
    assert_eq!(parsed.units[0].values, packed_values);
}

#[test]
fn builds_unit_values_segment_for_multiple_units() {
    let inputs = vec![
        ProveUnitValues {
            unit_index: 7,
            unit_value_map: vec![stage_value("unit.gamma", 1)],
            packed_values: values(&[31]),
        },
        ProveUnitValues {
            unit_index: 3,
            unit_value_map: vec![stage_value("unit.alpha", 1), stage_value("unit.beta", 2)],
            packed_values: values(&[11, 21, 22, 23]),
        },
    ];

    let segment = build_unit_values_segment_from_packed_values_batch(&inputs)
        .expect("segment should build")
        .expect("segment should be present");

    assert_eq!(segment.id, UNIT_VALUES_SEGMENT_ID);
    let parsed = parse_unit_values_segment(&segment.data).expect("segment should parse");
    assert_eq!(parsed.units.len(), 2);
    assert_eq!(parsed.units[0].unit_index, 3);
    assert_eq!(parsed.units[0].values, vec![11, 21, 22, 23]);
    assert_eq!(parsed.units[1].unit_index, 7);
    assert_eq!(parsed.units[1].values, vec![31]);
}

#[test]
fn loads_unit_values_from_segments() {
    let map = vec![stage_value("unit.alpha", 1), stage_value("unit.beta", 2)];
    let segment = unit_values_segment(3, &[11, 21, 22, 23]);

    let loaded =
        load_unit_values_from_segments(3, &map, &[segment]).expect("unit values should load");

    assert_eq!(loaded, values(&[11, 21, 22, 23]));
}

#[test]
fn loads_dimensioned_unit_values_from_segments() {
    let map = vec![
        stage_value_with_lengths("unit.array", 1, &[2, 3]),
        stage_value_with_lengths("unit.extension", 2, &[2]),
    ];
    let packed_values: Vec<u64> = (1..=12).collect();
    let segment = unit_values_segment(3, &packed_values);

    let loaded =
        load_unit_values_from_segments(3, &map, &[segment]).expect("unit values should load");

    assert_eq!(loaded, values(&packed_values));
}

#[test]
fn rejects_missing_unit_values_segment() {
    let map = vec![stage_value("unit.alpha", 1)];

    let error = load_unit_values_from_segments(0, &map, &[])
        .expect_err("required segment should be present");

    assert_eq!(error, LoadUnitValuesSegmentError::MissingSegment);
}

#[test]
fn rejects_missing_unit_values_unit() {
    let map = vec![stage_value("unit.alpha", 1)];
    let segment = unit_values_segment(2, &[11]);

    let error = load_unit_values_from_segments(0, &map, &[segment])
        .expect_err("required unit should be present");

    assert_eq!(
        error,
        LoadUnitValuesSegmentError::MissingUnit { unit_index: 0 }
    );
}

#[test]
fn rejects_duplicate_unit_values_segments() {
    let map = vec![stage_value("unit.alpha", 1)];
    let segment = unit_values_segment(0, &[11]);

    let error = load_unit_values_from_segments(0, &map, &[segment.clone(), segment])
        .expect_err("duplicate segments should reject");

    assert_eq!(error.to_string(), "duplicate unit values segment");
}

#[test]
fn rejects_unexpected_unit_values_unit() {
    let segment = unit_values_segment(0, &[11]);

    let error = load_unit_values_from_segments(0, &[], &[segment])
        .expect_err("unit values should not be present");

    assert_eq!(
        error,
        LoadUnitValuesSegmentError::UnexpectedUnit { unit_index: 0 }
    );
}

#[test]
fn rejects_loaded_unit_value_count_mismatch() {
    let map = vec![stage_value("unit.alpha", 1), stage_value("unit.beta", 2)];
    let segment = unit_values_segment(0, &[11, 21, 22]);

    let error = load_unit_values_from_segments(0, &map, &[segment])
        .expect_err("segment value count should match metadata");

    assert_eq!(
        error,
        LoadUnitValuesSegmentError::ValueCountMismatch {
            unit_index: 0,
            expected: 4,
            found: 3
        }
    );
}

#[test]
fn rejects_loaded_noncanonical_unit_values() {
    let map = vec![stage_value("unit.alpha", 1)];
    let mut segment = unit_values_segment(0, &[11]);
    segment.data[FIRST_UNIT_VALUE_OFFSET..FIRST_UNIT_VALUE_OFFSET + 8]
        .copy_from_slice(&MODULUS.to_le_bytes());

    let error = load_unit_values_from_segments(0, &map, &[segment])
        .expect_err("unit values should be canonical field elements");

    assert_eq!(
        error,
        LoadUnitValuesSegmentError::Segment(UnitValuesSegmentError::ValueNonCanonical {
            unit_index: 0,
            value_index: 0,
            source: FieldError::NonCanonical { value: MODULUS }
        })
    );
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

fn unit_values_segment(unit_index: u32, values: &[u64]) -> ProofSegment {
    ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: encode_unit_values_segment(&UnitValuesSegment {
            units: vec![UnitValuesUnitSegment {
                unit_index,
                values: values.to_vec(),
            }],
        })
        .expect("segment should encode"),
    }
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

#[test]
fn rejects_zero_length_unit_value_dimensions() {
    let map = vec![stage_value_with_lengths("unit.empty", 1, &[0])];

    assert_eq!(
        expected_packed_unit_value_count(&map),
        Err(ProveUnitValuesSegmentError::LengthOverflow)
    );
    assert_eq!(
        build_unit_values_segment_from_packed_values(0, &map, &[])
            .expect_err("zero dimensions should be rejected"),
        ProveUnitValuesSegmentError::LengthOverflow
    );
    assert_eq!(
        load_unit_values_from_segments(0, &map, &[])
            .expect_err("zero dimensions should be rejected"),
        LoadUnitValuesSegmentError::LengthOverflow
    );
}

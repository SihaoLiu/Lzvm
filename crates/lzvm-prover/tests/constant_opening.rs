use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, ConstantOpeningQuerySegment, ConstantOpeningSegment,
    ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_prover::constant_opening::{
    load_constant_opening_segment_from_segments, load_constant_opening_unit_from_segments,
    LoadConstantOpeningSegmentError, LoadConstantOpeningUnitError,
};

#[test]
fn loads_constant_opening_segment_from_segments() {
    let unit = constant_opening_unit(0);
    let segment = constant_opening_proof_segment(vec![unit.clone()]);

    let loaded =
        load_constant_opening_segment_from_segments(&[segment]).expect("segment should load");

    assert_eq!(loaded, ConstantOpeningSegment { units: vec![unit] });
}

#[test]
fn loads_constant_opening_unit_from_segments() {
    let unit = constant_opening_unit(0);
    let segment = constant_opening_proof_segment(vec![unit.clone()]);

    let loaded = load_constant_opening_unit_from_segments(0, &[segment]).expect("unit should load");

    assert_eq!(loaded, unit);
}

#[test]
fn rejects_missing_constant_opening_segment() {
    let error = load_constant_opening_segment_from_segments(&[]).expect_err("segment should exist");

    assert_eq!(error, LoadConstantOpeningSegmentError::MissingSegment);
}

#[test]
fn rejects_invalid_constant_opening_segment() {
    let error = load_constant_opening_segment_from_segments(&[ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }])
    .expect_err("segment should parse");

    assert!(matches!(error, LoadConstantOpeningSegmentError::Segment(_)));
}

#[test]
fn rejects_missing_constant_opening_unit() {
    let segment = constant_opening_proof_segment(vec![constant_opening_unit(1)]);

    let error =
        load_constant_opening_unit_from_segments(0, &[segment]).expect_err("unit should exist");

    assert_eq!(
        error,
        LoadConstantOpeningUnitError::MissingUnit { unit_index: 0 }
    );
}

fn constant_opening_proof_segment(units: Vec<ConstantOpeningUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: encode_constant_opening_segment(&ConstantOpeningSegment { units })
            .expect("segment should encode"),
    }
}

fn constant_opening_unit(unit_index: u32) -> ConstantOpeningUnitSegment {
    ConstantOpeningUnitSegment {
        unit_index,
        queries: vec![ConstantOpeningQuerySegment {
            row_index: 3,
            values: vec![5],
            siblings: Vec::new(),
        }],
    }
}

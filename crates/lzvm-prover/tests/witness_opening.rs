use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, WitnessOpeningQuerySegment, WitnessOpeningSegment,
    WitnessOpeningStageSegment, WitnessOpeningUnitSegment, WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_prover::witness_opening::{
    load_witness_opening_segment_from_segments, load_witness_opening_unit_from_segments,
    LoadWitnessOpeningSegmentError, LoadWitnessOpeningUnitError,
};

#[test]
fn loads_witness_opening_segment_from_segments() {
    let unit = witness_opening_unit(0);
    let segment = witness_opening_proof_segment(vec![unit.clone()]);

    let loaded =
        load_witness_opening_segment_from_segments(&[segment]).expect("segment should load");

    assert_eq!(loaded, WitnessOpeningSegment { units: vec![unit] });
}

#[test]
fn loads_witness_opening_unit_from_segments() {
    let unit = witness_opening_unit(0);
    let segment = witness_opening_proof_segment(vec![unit.clone()]);

    let loaded = load_witness_opening_unit_from_segments(0, &[segment]).expect("unit should load");

    assert_eq!(loaded, unit);
}

#[test]
fn rejects_missing_witness_opening_segment() {
    let error = load_witness_opening_segment_from_segments(&[]).expect_err("segment should exist");

    assert_eq!(error, LoadWitnessOpeningSegmentError::MissingSegment);
}

#[test]
fn rejects_invalid_witness_opening_segment() {
    let error = load_witness_opening_segment_from_segments(&[ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }])
    .expect_err("segment should parse");

    assert!(matches!(error, LoadWitnessOpeningSegmentError::Segment(_)));
}

#[test]
fn rejects_missing_witness_opening_unit() {
    let segment = witness_opening_proof_segment(vec![witness_opening_unit(1)]);

    let error =
        load_witness_opening_unit_from_segments(0, &[segment]).expect_err("unit should exist");

    assert_eq!(
        error,
        LoadWitnessOpeningUnitError::MissingUnit { unit_index: 0 }
    );
}

fn witness_opening_proof_segment(units: Vec<WitnessOpeningUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: encode_witness_opening_segment(&WitnessOpeningSegment { units })
            .expect("segment should encode"),
    }
}

fn witness_opening_unit(unit_index: u32) -> WitnessOpeningUnitSegment {
    WitnessOpeningUnitSegment {
        unit_index,
        queries: vec![WitnessOpeningQuerySegment {
            row_index: 3,
            stages: vec![WitnessOpeningStageSegment {
                stage_index: 1,
                values: vec![5],
                siblings: Vec::new(),
            }],
        }],
    }
}

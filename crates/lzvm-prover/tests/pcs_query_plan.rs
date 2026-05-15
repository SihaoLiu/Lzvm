use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_prover::pcs_query_plan::{
    load_pcs_query_plan_from_segments, LoadPcsQueryPlanSegmentError,
};

#[test]
fn loads_pcs_query_plan_from_segments() {
    let segment = pcs_query_plan_proof_segment(vec![PcsQueryPlanUnit {
        unit_index: 0,
        queries: vec![1, 3],
    }]);

    let loaded = load_pcs_query_plan_from_segments(&[segment]).expect("query plan should load");

    assert_eq!(
        loaded,
        PcsQueryPlanSegment {
            units: vec![PcsQueryPlanUnit {
                unit_index: 0,
                queries: vec![1, 3]
            }]
        }
    );
}

#[test]
fn rejects_missing_pcs_query_plan_segment() {
    let error = load_pcs_query_plan_from_segments(&[]).expect_err("segment should be present");

    assert_eq!(error, LoadPcsQueryPlanSegmentError::MissingSegment);
}

#[test]
fn rejects_invalid_pcs_query_plan_segment() {
    let error = load_pcs_query_plan_from_segments(&[ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }])
    .expect_err("segment should parse");

    assert!(matches!(error, LoadPcsQueryPlanSegmentError::Segment(_)));
}

fn pcs_query_plan_proof_segment(units: Vec<PcsQueryPlanUnit>) -> ProofSegment {
    ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&PcsQueryPlanSegment { units })
            .expect("segment should encode"),
    }
}

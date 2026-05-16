use std::collections::BTreeSet;

use lzvm_artifacts::challenge_values_segment::CHALLENGE_VALUES_SEGMENT_ID;
use lzvm_artifacts::constant_opening_segment::CONSTANT_OPENING_SEGMENT_ID;
use lzvm_artifacts::contribution_segment::CONTRIBUTION_SEGMENT_ID;
use lzvm_artifacts::group_values_segment::GROUP_VALUES_SEGMENT_ID;
use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
use lzvm_artifacts::pcs_fri_segment::PCS_FRI_OPENING_SEGMENT_ID;
use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
use lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID;
use lzvm_artifacts::program_image_segment::PROGRAM_IMAGE_CACHE_SEGMENT_ID;
use lzvm_artifacts::unit_values_segment::UNIT_VALUES_SEGMENT_ID;
use lzvm_artifacts::witness_opening_segment::WITNESS_OPENING_SEGMENT_ID;

#[test]
fn segment_ids_are_unique() {
    let ids = [
        PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        PCS_QUERY_PLAN_SEGMENT_ID,
        PCS_FRI_OPENING_SEGMENT_ID,
        PCS_QUERY_NONCE_SEGMENT_ID,
        PCS_EVALUATION_SEGMENT_ID,
        PCS_PROOF_VALUES_SEGMENT_ID,
        GROUP_VALUES_SEGMENT_ID,
        CHALLENGE_VALUES_SEGMENT_ID,
        UNIT_VALUES_SEGMENT_ID,
        PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        CONSTANT_OPENING_SEGMENT_ID,
        WITNESS_OPENING_SEGMENT_ID,
        CONTRIBUTION_SEGMENT_ID,
    ];

    let unique = ids.into_iter().collect::<BTreeSet<_>>();

    assert_eq!(unique.len(), ids.len());
}

use lzvm_artifacts::challenge_values_segment::CHALLENGE_VALUES_SEGMENT_ID;
use lzvm_artifacts::constant_opening_segment::CONSTANT_OPENING_SEGMENT_ID;
use lzvm_artifacts::contribution_segment::CONTRIBUTION_SEGMENT_ID;
use lzvm_artifacts::eth_block_input_segment::ETH_BLOCK_INPUT_SEGMENT_ID;
use lzvm_artifacts::group_values_segment::GROUP_VALUES_SEGMENT_ID;
use lzvm_artifacts::guest_input_segment::FRAMED_GUEST_INPUT_SEGMENT_ID;
use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
use lzvm_artifacts::pcs_fri_segment::PCS_FRI_OPENING_SEGMENT_ID;
use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
use lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID;
use lzvm_artifacts::program_image_segment::PROGRAM_IMAGE_CACHE_SEGMENT_ID;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::trace_constraint_segment::TRACE_CONSTRAINT_SEGMENT_ID;
use lzvm_artifacts::unit_values_segment::UNIT_VALUES_SEGMENT_ID;
use lzvm_artifacts::witness_opening_segment::WITNESS_OPENING_SEGMENT_ID;
use lzvm_artifacts::witness_segment::WITNESS_COMMITMENT_SEGMENT_BASE_ID;

pub(crate) fn is_allowed_proof_segment_id(id: u32) -> bool {
    if (WITNESS_COMMITMENT_SEGMENT_BASE_ID..PCS_MATERIAL_MANIFEST_SEGMENT_ID).contains(&id) {
        return true;
    }

    matches!(
        id,
        PCS_MATERIAL_MANIFEST_SEGMENT_ID
            | PCS_QUERY_PLAN_SEGMENT_ID
            | WITNESS_OPENING_SEGMENT_ID
            | CONSTANT_OPENING_SEGMENT_ID
            | PCS_FRI_OPENING_SEGMENT_ID
            | PCS_QUERY_NONCE_SEGMENT_ID
            | PCS_EVALUATION_SEGMENT_ID
            | PCS_PROOF_VALUES_SEGMENT_ID
            | GROUP_VALUES_SEGMENT_ID
            | CHALLENGE_VALUES_SEGMENT_ID
            | UNIT_VALUES_SEGMENT_ID
            | TRACE_CONSTRAINT_SEGMENT_ID
            | PROGRAM_IMAGE_CACHE_SEGMENT_ID
            | CONTRIBUTION_SEGMENT_ID
            | ETH_BLOCK_INPUT_SEGMENT_ID
            | FRAMED_GUEST_INPUT_SEGMENT_ID
    )
}

pub(crate) fn unexpected_proof_segment_id(segments: &[ProofSegment]) -> Option<u32> {
    segments
        .iter()
        .find(|segment| !is_allowed_proof_segment_id(segment.id))
        .map(|segment| segment.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_witness_commitment_range_boundaries() {
        assert!(is_allowed_proof_segment_id(
            WITNESS_COMMITMENT_SEGMENT_BASE_ID
        ));
        assert!(is_allowed_proof_segment_id(
            PCS_MATERIAL_MANIFEST_SEGMENT_ID - 1
        ));
        assert!(is_allowed_proof_segment_id(
            PCS_MATERIAL_MANIFEST_SEGMENT_ID
        ));
    }

    #[test]
    fn allows_all_fixed_segment_ids() {
        for id in [
            PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            PCS_QUERY_PLAN_SEGMENT_ID,
            WITNESS_OPENING_SEGMENT_ID,
            CONSTANT_OPENING_SEGMENT_ID,
            PCS_FRI_OPENING_SEGMENT_ID,
            PCS_QUERY_NONCE_SEGMENT_ID,
            PCS_EVALUATION_SEGMENT_ID,
            PCS_PROOF_VALUES_SEGMENT_ID,
            GROUP_VALUES_SEGMENT_ID,
            CHALLENGE_VALUES_SEGMENT_ID,
            UNIT_VALUES_SEGMENT_ID,
            TRACE_CONSTRAINT_SEGMENT_ID,
            PROGRAM_IMAGE_CACHE_SEGMENT_ID,
            CONTRIBUTION_SEGMENT_ID,
            ETH_BLOCK_INPUT_SEGMENT_ID,
            FRAMED_GUEST_INPUT_SEGMENT_ID,
        ] {
            assert!(is_allowed_proof_segment_id(id), "{id} should be allowed");
        }
    }

    #[test]
    fn rejects_reserved_and_unknown_fixed_segment_ids() {
        assert!(!is_allowed_proof_segment_id(
            WITNESS_COMMITMENT_SEGMENT_BASE_ID - 1
        ));
        assert!(!is_allowed_proof_segment_id(20_000));
    }

    #[test]
    fn reports_first_unexpected_segment_id() {
        let segments = [
            segment(PCS_MATERIAL_MANIFEST_SEGMENT_ID),
            segment(20_000),
            segment(20_001),
        ];

        assert_eq!(unexpected_proof_segment_id(&segments), Some(20_000));
    }

    #[test]
    fn accepts_only_expected_segment_ids() {
        let segments = [
            segment(WITNESS_COMMITMENT_SEGMENT_BASE_ID),
            segment(PCS_MATERIAL_MANIFEST_SEGMENT_ID - 1),
            segment(PCS_MATERIAL_MANIFEST_SEGMENT_ID),
            segment(FRAMED_GUEST_INPUT_SEGMENT_ID),
        ];

        assert_eq!(unexpected_proof_segment_id(&segments), None);
    }

    fn segment(id: u32) -> ProofSegment {
        ProofSegment { id, data: vec![1] }
    }
}

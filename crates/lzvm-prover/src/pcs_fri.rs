mod build;
mod errors;
mod fold;
mod merkle;
mod requests;
mod validation;

pub use build::{build_pcs_fri_opening_unit, build_pcs_fri_transcript_commitments};
pub use errors::*;
pub use fold::{verify_fri_fold, verify_fri_opening_folds, PcsFriFoldError};
use lzvm_artifacts::pcs_fri_segment::{
    parse_pcs_fri_opening_segment, PcsFriOpeningSegment, PcsFriOpeningUnitSegment,
    PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
pub use merkle::{verify_fri_last_level_root, verify_fri_query_path, PcsFriMerkleError};
pub use requests::*;
pub use validation::{
    validate_optional_pcs_fri_opening_proof_segments, validate_pcs_fri_opening_folds_from_units,
    validate_pcs_fri_opening_segments,
};

pub fn load_pcs_fri_opening_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<PcsFriOpeningSegment, LoadPcsFriOpeningSegmentError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .ok_or(LoadPcsFriOpeningSegmentError::MissingSegment)?;
    parse_pcs_fri_opening_segment(&segment.data).map_err(LoadPcsFriOpeningSegmentError::Segment)
}

pub fn load_pcs_fri_opening_unit_from_segments(
    unit_index: usize,
    segments: &[ProofSegment],
) -> Result<PcsFriOpeningUnitSegment, LoadPcsFriOpeningUnitError> {
    let opening = load_pcs_fri_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadPcsFriOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| unit.unit_index == unit_index_u32)
        .ok_or(LoadPcsFriOpeningUnitError::MissingUnit { unit_index })
}

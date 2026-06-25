mod build;
mod errors;
mod fold;
mod merkle;
mod requests;
mod validation;

pub(crate) use build::build_pcs_fri_opening_unit_from_transcript_commitments_with_timing;
pub use build::{
    build_pcs_fri_opening_unit, build_pcs_fri_opening_unit_with_timing,
    build_pcs_fri_transcript_commitments, build_pcs_fri_transcript_commitments_with_timing,
};
pub use errors::*;
pub use fold::{verify_fri_fold, verify_fri_opening_folds, PcsFriFoldError};
use lzvm_artifacts::pcs_fri_segment::{
    parse_pcs_fri_opening_segment, PcsFriOpeningSegment, PcsFriOpeningUnitSegment,
    PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::PcsQueryPlanUnit;
use lzvm_artifacts::proof::ProofSegment;
pub use merkle::{verify_fri_last_level_root, verify_fri_query_path, PcsFriMerkleError};
pub use requests::*;
pub(crate) use validation::validate_optional_pcs_fri_opening_proof_segments_with_transcript_challenges;
pub use validation::{
    validate_optional_pcs_fri_opening_proof_segments, validate_pcs_fri_opening_folds_from_units,
    validate_pcs_fri_opening_segments,
};

pub fn load_pcs_fri_opening_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<PcsFriOpeningSegment, LoadPcsFriOpeningSegmentError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(LoadPcsFriOpeningSegmentError::MissingSegment)?;
    if matching_segments.next().is_some() {
        return Err(LoadPcsFriOpeningSegmentError::DuplicateSegment);
    }
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
        .find(|unit| unit.unit_index == unit_index_u32 && unit.trace_instance_index == 0)
        .ok_or(LoadPcsFriOpeningUnitError::MissingUnit { unit_index })
}

pub(crate) fn validate_pcs_fri_opening_units_match_query_units_from_segment(
    query_units: &[PcsQueryPlanUnit],
    opening: &PcsFriOpeningSegment,
) -> Result<(), LoadPcsFriOpeningUnitError> {
    for unit in &opening.units {
        if !query_units.iter().any(|query_unit| {
            query_unit.unit_index == unit.unit_index
                && query_unit.trace_instance_index == unit.trace_instance_index
        }) {
            let unit_index = usize::try_from(unit.unit_index)
                .map_err(|_| LoadPcsFriOpeningUnitError::UnitIndexOverflow)?;
            return Err(LoadPcsFriOpeningUnitError::UnexpectedUnit { unit_index });
        }
    }
    Ok(())
}

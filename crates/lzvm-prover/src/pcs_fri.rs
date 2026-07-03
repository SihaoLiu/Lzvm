use std::collections::BTreeSet;

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
pub(crate) use validation::validate_optional_pcs_fri_opening_proof_segments_with_preflight_values;
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
    load_pcs_fri_opening_unit_for_identity_from_segments(unit_index, 0, segments)
}

pub fn load_pcs_fri_opening_unit_for_identity_from_segments(
    unit_index: usize,
    trace_instance_index: u32,
    segments: &[ProofSegment],
) -> Result<PcsFriOpeningUnitSegment, LoadPcsFriOpeningUnitError> {
    let opening = load_pcs_fri_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadPcsFriOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| {
            unit.unit_index == unit_index_u32 && unit.trace_instance_index == trace_instance_index
        })
        .ok_or(LoadPcsFriOpeningUnitError::MissingUnit { unit_index })
}

pub(crate) fn validate_pcs_fri_opening_units_match_query_units_from_segment(
    query_units: &[PcsQueryPlanUnit],
    opening: &PcsFriOpeningSegment,
) -> Result<(), LoadPcsFriOpeningUnitError> {
    let query_identities = query_units
        .iter()
        .map(|unit| (unit.unit_index, unit.trace_instance_index))
        .collect::<BTreeSet<_>>();
    let mut opening_identities = BTreeSet::new();
    for unit in &opening.units {
        let identity = (unit.unit_index, unit.trace_instance_index);
        let unit_index = usize::try_from(unit.unit_index)
            .map_err(|_| LoadPcsFriOpeningUnitError::UnitIndexOverflow)?;
        if !query_identities.contains(&identity) || !opening_identities.insert(identity) {
            return Err(LoadPcsFriOpeningUnitError::UnexpectedUnit { unit_index });
        }
    }
    for query_unit in query_units {
        let identity = (query_unit.unit_index, query_unit.trace_instance_index);
        if !opening_identities.contains(&identity) {
            let unit_index = usize::try_from(query_unit.unit_index)
                .map_err(|_| LoadPcsFriOpeningUnitError::UnitIndexOverflow)?;
            return Err(LoadPcsFriOpeningUnitError::MissingUnit { unit_index });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fri_opening_units_match_query_units_rejects_duplicate_in_memory_identity() {
        let query_units = vec![query_unit(0, 1)];
        let opening = PcsFriOpeningSegment {
            units: vec![opening_unit(0, 1), opening_unit(0, 1)],
        };

        let error =
            validate_pcs_fri_opening_units_match_query_units_from_segment(&query_units, &opening)
                .expect_err("duplicate FRI opening identity should reject");

        assert_eq!(
            error,
            LoadPcsFriOpeningUnitError::UnexpectedUnit { unit_index: 0 }
        );
    }

    fn query_unit(unit_index: u32, trace_instance_index: u32) -> PcsQueryPlanUnit {
        PcsQueryPlanUnit {
            unit_index,
            trace_instance_index,
            queries: vec![0],
        }
    }

    fn opening_unit(unit_index: u32, trace_instance_index: u32) -> PcsFriOpeningUnitSegment {
        PcsFriOpeningUnitSegment {
            unit_index,
            trace_instance_index,
            layers: Vec::new(),
            final_polynomial: Vec::new(),
        }
    }
}

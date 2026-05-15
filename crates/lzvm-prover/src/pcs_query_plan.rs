use std::fmt;

use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanSegmentError,
    PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;

use crate::prove_witness::{build_pcs_query_plan_segment, ProvePcsQueryPlanSegmentError};
use crate::witness_commitment::{
    load_witness_commitment_segments, LoadWitnessCommitmentSegmentsError,
};
use crate::ProveSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPcsQueryPlanSegmentError {
    MissingSegment,
    Segment(PcsQueryPlanSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatePcsQueryPlanSegmentsError {
    MissingMaterialSegment,
    QueryPlan(LoadPcsQueryPlanSegmentError),
    Witness(LoadWitnessCommitmentSegmentsError),
    Build(ProvePcsQueryPlanSegmentError),
    QueryPlanMismatch,
}

impl fmt::Display for LoadPcsQueryPlanSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS query plan segment"),
            Self::Segment(error) => write!(f, "invalid PCS query plan segment: {error}"),
        }
    }
}

impl fmt::Display for ValidatePcsQueryPlanSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMaterialSegment => write!(f, "missing PCS material manifest segment"),
            Self::QueryPlan(error) => write!(f, "{error}"),
            Self::Witness(error) => write!(f, "{error}"),
            Self::Build(error) => write!(f, "derive PCS query plan segment failed: {error}"),
            Self::QueryPlanMismatch => write!(f, "PCS query plan segment mismatch"),
        }
    }
}

impl std::error::Error for LoadPcsQueryPlanSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment => None,
        }
    }
}

impl std::error::Error for ValidatePcsQueryPlanSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Witness(error) => Some(error),
            Self::Build(error) => Some(error),
            Self::MissingMaterialSegment | Self::QueryPlanMismatch => None,
        }
    }
}

pub fn load_pcs_query_plan_from_segments(
    segments: &[ProofSegment],
) -> Result<PcsQueryPlanSegment, LoadPcsQueryPlanSegmentError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .ok_or(LoadPcsQueryPlanSegmentError::MissingSegment)?;
    parse_pcs_query_plan_segment(&segment.data).map_err(LoadPcsQueryPlanSegmentError::Segment)
}

pub fn validate_seeded_pcs_query_plan_segments(
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    segments: &[ProofSegment],
) -> Result<(), ValidatePcsQueryPlanSegmentsError> {
    let material_segment = segments
        .iter()
        .find(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID)
        .ok_or(ValidatePcsQueryPlanSegmentsError::MissingMaterialSegment)?;
    load_pcs_query_plan_from_segments(segments)
        .map_err(ValidatePcsQueryPlanSegmentsError::QueryPlan)?;
    let query_segment = segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .ok_or(ValidatePcsQueryPlanSegmentsError::QueryPlan(
            LoadPcsQueryPlanSegmentError::MissingSegment,
        ))?;
    let witness_segments = load_witness_commitment_segments(&schedule.units, segments)
        .map_err(ValidatePcsQueryPlanSegmentsError::Witness)?;
    let expected_segment = build_pcs_query_plan_segment(
        schedule,
        public_values_hash,
        material_segment,
        &witness_segments,
    )
    .map_err(ValidatePcsQueryPlanSegmentsError::Build)?;
    if query_segment.data != expected_segment.data {
        return Err(ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
    }
    Ok(())
}

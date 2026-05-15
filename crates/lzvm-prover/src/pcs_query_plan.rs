use std::fmt;

use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanSegmentError,
    PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPcsQueryPlanSegmentError {
    MissingSegment,
    Segment(PcsQueryPlanSegmentError),
}

impl fmt::Display for LoadPcsQueryPlanSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS query plan segment"),
            Self::Segment(error) => write!(f, "invalid PCS query plan segment: {error}"),
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

pub fn load_pcs_query_plan_from_segments(
    segments: &[ProofSegment],
) -> Result<PcsQueryPlanSegment, LoadPcsQueryPlanSegmentError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .ok_or(LoadPcsQueryPlanSegmentError::MissingSegment)?;
    parse_pcs_query_plan_segment(&segment.data).map_err(LoadPcsQueryPlanSegmentError::Segment)
}

use std::collections::BTreeSet;
use std::fmt;

use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, PcsFriOpeningSegment, PcsFriOpeningSegmentError,
    PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegmentError, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::Ext3;

use crate::pcs_fri::{
    build_pcs_fri_opening_unit, PcsFriOpeningBuildError, PcsFriOpeningBuildRequest,
};
use crate::ProveSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsFriOpeningValues {
    pub unit_index: usize,
    pub challenges: Vec<Ext3>,
    pub polynomial: Vec<Ext3>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsFriOpeningSegmentError {
    InvalidQuerySegmentId {
        segment_id: u32,
    },
    QueryPlan(PcsQueryPlanSegmentError),
    MissingQueryUnit {
        unit_index: usize,
    },
    DuplicateUnitIndex {
        unit_index: usize,
    },
    UnitIndexOverflow {
        unit_index: usize,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    Build {
        unit_index: usize,
        source: PcsFriOpeningBuildError,
    },
    Segment(PcsFriOpeningSegmentError),
}

impl fmt::Display for ProvePcsFriOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuerySegmentId { segment_id } => write!(
                f,
                "prove PCS FRI opening expected query plan segment id {PCS_QUERY_PLAN_SEGMENT_ID}, found {segment_id}"
            ),
            Self::QueryPlan(error) => {
                write!(f, "prove PCS FRI opening query plan parse failed: {error}")
            }
            Self::MissingQueryUnit { unit_index } => {
                write!(f, "prove PCS FRI opening is missing query unit {unit_index}")
            }
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate prove PCS FRI opening unit index: {unit_index}")
            }
            Self::UnitIndexOverflow { unit_index } => write!(
                f,
                "prove PCS FRI opening unit index does not fit u32: {unit_index}"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove PCS FRI opening unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::Build { unit_index, source } => write!(
                f,
                "prove PCS FRI opening build failed for unit {unit_index}: {source}"
            ),
            Self::Segment(error) => {
                write!(f, "prove PCS FRI opening segment encode failed: {error}")
            }
        }
    }
}

impl std::error::Error for ProvePcsFriOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Build { source, .. } => Some(source),
            Self::Segment(error) => Some(error),
            Self::InvalidQuerySegmentId { .. }
            | Self::MissingQueryUnit { .. }
            | Self::DuplicateUnitIndex { .. }
            | Self::UnitIndexOverflow { .. }
            | Self::UnitIndexOutOfRange { .. } => None,
        }
    }
}

impl From<PcsQueryPlanSegmentError> for ProvePcsFriOpeningSegmentError {
    fn from(error: PcsQueryPlanSegmentError) -> Self {
        Self::QueryPlan(error)
    }
}

impl From<PcsFriOpeningSegmentError> for ProvePcsFriOpeningSegmentError {
    fn from(error: PcsFriOpeningSegmentError) -> Self {
        Self::Segment(error)
    }
}

pub fn build_pcs_fri_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriOpeningValues],
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    if query_segment.id != PCS_QUERY_PLAN_SEGMENT_ID {
        return Err(ProvePcsFriOpeningSegmentError::InvalidQuerySegmentId {
            segment_id: query_segment.id,
        });
    }
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let mut seen_units = BTreeSet::new();
    let mut units = Vec::with_capacity(values.len());
    for input in values {
        if !seen_units.insert(input.unit_index) {
            return Err(ProvePcsFriOpeningSegmentError::DuplicateUnitIndex {
                unit_index: input.unit_index,
            });
        }
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriOpeningSegmentError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let unit_index_u32 = u32::try_from(input.unit_index).map_err(|_| {
            ProvePcsFriOpeningSegmentError::UnitIndexOverflow {
                unit_index: input.unit_index,
            }
        })?;
        let query_unit = query_plan
            .units
            .iter()
            .find(|unit| unit.unit_index == unit_index_u32)
            .ok_or(ProvePcsFriOpeningSegmentError::MissingQueryUnit {
                unit_index: input.unit_index,
            })?;
        let opening = build_pcs_fri_opening_unit(
            unit,
            PcsFriOpeningBuildRequest {
                unit_index: unit_index_u32,
                query_rows: &query_unit.queries,
                challenges: &input.challenges,
                polynomial: &input.polynomial,
            },
        )
        .map_err(|source| ProvePcsFriOpeningSegmentError::Build {
            unit_index: input.unit_index,
            source,
        })?;
        units.push(opening);
    }

    let segment = PcsFriOpeningSegment { units };
    Ok(ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: encode_pcs_fri_opening_segment(&segment)?,
    })
}

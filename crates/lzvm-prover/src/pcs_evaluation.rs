use std::fmt;

use lzvm_artifacts::pcs_evaluation_segment::{
    parse_pcs_evaluation_segment, PcsEvaluationSegmentError, PcsEvaluationUnitSegment,
    PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;

use crate::ProveUnitSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPcsEvaluationUnitError {
    MissingSegment,
    MissingUnit {
        unit_index: usize,
    },
    UnitIndexOverflow,
    ValueCountMismatch {
        unit_index: usize,
        expected: usize,
        found: usize,
    },
    Segment(PcsEvaluationSegmentError),
}

impl fmt::Display for LoadPcsEvaluationUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS evaluation segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "PCS evaluation segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "PCS evaluation segment unit index overflow"),
            Self::ValueCountMismatch { unit_index, .. } => write!(
                f,
                "PCS evaluation segment value count mismatch for unit {unit_index}"
            ),
            Self::Segment(error) => write!(f, "invalid PCS evaluation segment: {error}"),
        }
    }
}

impl std::error::Error for LoadPcsEvaluationUnitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment
            | Self::MissingUnit { .. }
            | Self::UnitIndexOverflow
            | Self::ValueCountMismatch { .. } => None,
        }
    }
}

pub fn load_pcs_evaluation_unit_from_segments(
    unit_index: usize,
    unit: &ProveUnitSchedule,
    segments: &[ProofSegment],
) -> Result<PcsEvaluationUnitSegment, LoadPcsEvaluationUnitError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
        .ok_or(LoadPcsEvaluationUnitError::MissingSegment)?;
    let evaluations =
        parse_pcs_evaluation_segment(&segment.data).map_err(LoadPcsEvaluationUnitError::Segment)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadPcsEvaluationUnitError::UnitIndexOverflow)?;
    let evaluation_unit = evaluations
        .units
        .into_iter()
        .find(|unit| unit.unit_index == unit_index_u32)
        .ok_or(LoadPcsEvaluationUnitError::MissingUnit { unit_index })?;

    let expected_value_count = unit.expected_evaluation_value_count();
    if evaluation_unit.values.len() != expected_value_count {
        return Err(LoadPcsEvaluationUnitError::ValueCountMismatch {
            unit_index,
            expected: expected_value_count,
            found: evaluation_unit.values.len(),
        });
    }
    Ok(evaluation_unit)
}

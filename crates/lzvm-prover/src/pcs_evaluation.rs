use std::fmt;

use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, parse_pcs_evaluation_segment, PcsEvaluationSegment,
    PcsEvaluationSegmentError, PcsEvaluationUnitSegment, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::Ext3;

use crate::ProveSchedule;
use crate::ProveUnitSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsEvaluationValues {
    pub unit_index: usize,
    pub values: Vec<Ext3>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsEvaluationSegmentError {
    UnitIndexOverflow {
        unit_index: usize,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    ValueCountMismatch {
        unit_index: usize,
        expected: usize,
        found: usize,
    },
    Segment(PcsEvaluationSegmentError),
}

impl fmt::Display for ProvePcsEvaluationSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOverflow { unit_index } => write!(
                f,
                "prove PCS evaluation segment unit index does not fit u32: {unit_index}"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove PCS evaluation segment unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::ValueCountMismatch {
                unit_index,
                expected,
                found,
            } => write!(
                f,
                "prove PCS evaluation segment unit {unit_index} value count mismatch: expected {expected}, found {found}"
            ),
            Self::Segment(error) => {
                write!(f, "prove PCS evaluation segment encode failed: {error}")
            }
        }
    }
}

impl std::error::Error for ProvePcsEvaluationSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::UnitIndexOverflow { .. }
            | Self::UnitIndexOutOfRange { .. }
            | Self::ValueCountMismatch { .. } => None,
        }
    }
}

impl From<PcsEvaluationSegmentError> for ProvePcsEvaluationSegmentError {
    fn from(error: PcsEvaluationSegmentError) -> Self {
        Self::Segment(error)
    }
}

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

pub fn build_pcs_evaluation_segment(
    schedule: &ProveSchedule,
    values: &[ProvePcsEvaluationValues],
) -> Result<ProofSegment, ProvePcsEvaluationSegmentError> {
    let mut units = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsEvaluationSegmentError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let expected_value_count = unit.expected_evaluation_value_count();
        if input.values.len() != expected_value_count {
            return Err(ProvePcsEvaluationSegmentError::ValueCountMismatch {
                unit_index: input.unit_index,
                expected: expected_value_count,
                found: input.values.len(),
            });
        }
        units.push(PcsEvaluationUnitSegment {
            unit_index: u32::try_from(input.unit_index).map_err(|_| {
                ProvePcsEvaluationSegmentError::UnitIndexOverflow {
                    unit_index: input.unit_index,
                }
            })?,
            values: input.values.iter().copied().map(Ext3::to_u64s).collect(),
        });
    }
    units.sort_by_key(|unit| unit.unit_index);

    let segment = PcsEvaluationSegment { units };
    Ok(ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: encode_pcs_evaluation_segment(&segment)?,
    })
}

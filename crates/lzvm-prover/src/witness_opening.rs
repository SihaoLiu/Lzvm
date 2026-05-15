use std::fmt;

use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    parse_witness_opening_segment, WitnessOpeningSegment, WitnessOpeningSegmentError,
    WitnessOpeningUnitSegment, WITNESS_OPENING_SEGMENT_ID,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWitnessOpeningSegmentError {
    MissingSegment,
    Segment(WitnessOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWitnessOpeningUnitError {
    MissingSegment,
    MissingUnit { unit_index: usize },
    UnitIndexOverflow,
    Segment(WitnessOpeningSegmentError),
}

impl fmt::Display for LoadWitnessOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing witness opening segment"),
            Self::Segment(error) => write!(f, "invalid witness opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadWitnessOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment => None,
        }
    }
}

impl fmt::Display for LoadWitnessOpeningUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing witness opening segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "witness opening segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "witness opening segment unit index overflow"),
            Self::Segment(error) => write!(f, "invalid witness opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadWitnessOpeningUnitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment | Self::MissingUnit { .. } | Self::UnitIndexOverflow => None,
        }
    }
}

impl From<LoadWitnessOpeningSegmentError> for LoadWitnessOpeningUnitError {
    fn from(error: LoadWitnessOpeningSegmentError) -> Self {
        match error {
            LoadWitnessOpeningSegmentError::MissingSegment => Self::MissingSegment,
            LoadWitnessOpeningSegmentError::Segment(error) => Self::Segment(error),
        }
    }
}

pub fn load_witness_opening_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<WitnessOpeningSegment, LoadWitnessOpeningSegmentError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
        .ok_or(LoadWitnessOpeningSegmentError::MissingSegment)?;
    parse_witness_opening_segment(&segment.data).map_err(LoadWitnessOpeningSegmentError::Segment)
}

pub fn load_witness_opening_unit_from_segments(
    unit_index: usize,
    segments: &[ProofSegment],
) -> Result<WitnessOpeningUnitSegment, LoadWitnessOpeningUnitError> {
    let opening = load_witness_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadWitnessOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| unit.unit_index == unit_index_u32)
        .ok_or(LoadWitnessOpeningUnitError::MissingUnit { unit_index })
}

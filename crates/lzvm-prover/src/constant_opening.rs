use std::fmt;

use lzvm_artifacts::constant_opening_segment::{
    parse_constant_opening_segment, ConstantOpeningSegment, ConstantOpeningSegmentError,
    ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadConstantOpeningSegmentError {
    MissingSegment,
    Segment(ConstantOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadConstantOpeningUnitError {
    MissingSegment,
    MissingUnit { unit_index: usize },
    UnitIndexOverflow,
    Segment(ConstantOpeningSegmentError),
}

impl fmt::Display for LoadConstantOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing constant opening segment"),
            Self::Segment(error) => write!(f, "invalid constant opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadConstantOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment => None,
        }
    }
}

impl fmt::Display for LoadConstantOpeningUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing constant opening segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "constant opening segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "constant opening segment unit index overflow"),
            Self::Segment(error) => write!(f, "invalid constant opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadConstantOpeningUnitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment | Self::MissingUnit { .. } | Self::UnitIndexOverflow => None,
        }
    }
}

impl From<LoadConstantOpeningSegmentError> for LoadConstantOpeningUnitError {
    fn from(error: LoadConstantOpeningSegmentError) -> Self {
        match error {
            LoadConstantOpeningSegmentError::MissingSegment => Self::MissingSegment,
            LoadConstantOpeningSegmentError::Segment(error) => Self::Segment(error),
        }
    }
}

pub fn load_constant_opening_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<ConstantOpeningSegment, LoadConstantOpeningSegmentError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID)
        .ok_or(LoadConstantOpeningSegmentError::MissingSegment)?;
    parse_constant_opening_segment(&segment.data).map_err(LoadConstantOpeningSegmentError::Segment)
}

pub fn load_constant_opening_unit_from_segments(
    unit_index: usize,
    segments: &[ProofSegment],
) -> Result<ConstantOpeningUnitSegment, LoadConstantOpeningUnitError> {
    let opening = load_constant_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadConstantOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| unit.unit_index == unit_index_u32)
        .ok_or(LoadConstantOpeningUnitError::MissingUnit { unit_index })
}

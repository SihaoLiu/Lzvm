use std::fmt;

use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::unit_values_segment::{
    encode_unit_values_segment, UnitValuesSegment, UnitValuesSegmentError, UnitValuesUnitSegment,
    UNIT_VALUES_SEGMENT_ID,
};
use lzvm_field::Felt;

const EXTENSION_WORDS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveUnitValuesSegmentError {
    UnitIndexOverflow {
        unit_index: usize,
    },
    UnexpectedValues {
        unit_index: usize,
        found: usize,
    },
    ValueCountMismatch {
        unit_index: usize,
        expected: usize,
        found: usize,
    },
    LengthOverflow,
    Segment(UnitValuesSegmentError),
}

impl fmt::Display for ProveUnitValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOverflow { unit_index } => {
                write!(f, "unit values segment unit index overflow: {unit_index}")
            }
            Self::UnexpectedValues { unit_index, found } => write!(
                f,
                "unit values segment for unit {unit_index} received {found} values but metadata declares none"
            ),
            Self::ValueCountMismatch {
                unit_index,
                expected,
                found,
            } => write!(
                f,
                "unit values segment for unit {unit_index} value count mismatch: expected {expected}, found {found}"
            ),
            Self::LengthOverflow => write!(f, "unit values segment length overflow"),
            Self::Segment(error) => write!(f, "unit values segment encode failed: {error}"),
        }
    }
}

impl std::error::Error for ProveUnitValuesSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            _ => None,
        }
    }
}

impl From<UnitValuesSegmentError> for ProveUnitValuesSegmentError {
    fn from(error: UnitValuesSegmentError) -> Self {
        Self::Segment(error)
    }
}

pub fn build_unit_values_segment_from_packed_values(
    unit_index: usize,
    unit_value_map: &[StageValue],
    packed_values: &[Felt],
) -> Result<Option<ProofSegment>, ProveUnitValuesSegmentError> {
    if unit_value_map.is_empty() {
        if packed_values.is_empty() {
            return Ok(None);
        }
        return Err(ProveUnitValuesSegmentError::UnexpectedValues {
            unit_index,
            found: packed_values.len(),
        });
    }

    let expected = expected_packed_unit_value_count(unit_value_map)?;
    if packed_values.len() != expected {
        return Err(ProveUnitValuesSegmentError::ValueCountMismatch {
            unit_index,
            expected,
            found: packed_values.len(),
        });
    }

    let segment = UnitValuesSegment {
        units: vec![UnitValuesUnitSegment {
            unit_index: u32::try_from(unit_index)
                .map_err(|_| ProveUnitValuesSegmentError::UnitIndexOverflow { unit_index })?,
            values: packed_values.iter().map(|value| value.to_u64()).collect(),
        }],
    };
    Ok(Some(ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: encode_unit_values_segment(&segment)?,
    }))
}

pub fn expected_packed_unit_value_count(
    unit_value_map: &[StageValue],
) -> Result<usize, ProveUnitValuesSegmentError> {
    unit_value_map.iter().try_fold(0_usize, |count, value| {
        count
            .checked_add(if value.stage == 1 { 1 } else { EXTENSION_WORDS })
            .ok_or(ProveUnitValuesSegmentError::LengthOverflow)
    })
}

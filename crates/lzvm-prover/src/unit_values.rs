use std::fmt;

use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::unit_values_segment::{
    encode_unit_values_segment, parse_unit_values_segment, UnitValuesSegment,
    UnitValuesSegmentError, UnitValuesUnitSegment, UNIT_VALUES_SEGMENT_ID,
};
use lzvm_field::{Felt, FieldError};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveUnitValues {
    pub unit_index: usize,
    pub unit_value_map: Vec<StageValue>,
    pub packed_values: Vec<Felt>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadUnitValuesSegmentError {
    UnitIndexOverflow {
        unit_index: usize,
    },
    MissingSegment,
    DuplicateSegment,
    MissingUnit {
        unit_index: usize,
    },
    UnexpectedUnit {
        unit_index: usize,
    },
    ValueCountMismatch {
        unit_index: usize,
        expected: usize,
        found: usize,
    },
    NonCanonicalValue {
        unit_index: usize,
        index: usize,
        source: FieldError,
    },
    Segment(UnitValuesSegmentError),
    LengthOverflow,
}

impl fmt::Display for LoadUnitValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOverflow { unit_index } => {
                write!(f, "unit values segment unit index overflow: {unit_index}")
            }
            Self::MissingSegment => write!(f, "missing unit values segment"),
            Self::DuplicateSegment => write!(f, "duplicate unit values segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "missing unit values segment for unit {unit_index}")
            }
            Self::UnexpectedUnit { unit_index } => {
                write!(f, "unexpected unit values segment for unit {unit_index}")
            }
            Self::ValueCountMismatch {
                unit_index,
                expected,
                found,
            } => write!(
                f,
                "unit values segment count mismatch for unit {unit_index}: expected {expected}, found {found}"
            ),
            Self::NonCanonicalValue {
                unit_index,
                index,
                source,
            } => write!(
                f,
                "invalid unit values segment value {index} for unit {unit_index}: {source}"
            ),
            Self::Segment(error) => write!(f, "invalid unit values segment: {error}"),
            Self::LengthOverflow => write!(f, "unit values segment length overflow"),
        }
    }
}

impl std::error::Error for LoadUnitValuesSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NonCanonicalValue { source, .. } => Some(source),
            Self::Segment(error) => Some(error),
            Self::UnitIndexOverflow { .. }
            | Self::MissingSegment
            | Self::DuplicateSegment
            | Self::MissingUnit { .. }
            | Self::UnexpectedUnit { .. }
            | Self::ValueCountMismatch { .. }
            | Self::LengthOverflow => None,
        }
    }
}

pub fn build_unit_values_segment_from_packed_values(
    unit_index: usize,
    unit_value_map: &[StageValue],
    packed_values: &[Felt],
) -> Result<Option<ProofSegment>, ProveUnitValuesSegmentError> {
    let Some(unit) = build_unit_values_unit_segment(unit_index, unit_value_map, packed_values)?
    else {
        return Ok(None);
    };

    let segment = UnitValuesSegment { units: vec![unit] };
    Ok(Some(ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: encode_unit_values_segment(&segment)?,
    }))
}

pub fn build_unit_values_segment_from_packed_values_batch(
    inputs: &[ProveUnitValues],
) -> Result<Option<ProofSegment>, ProveUnitValuesSegmentError> {
    let mut units = Vec::with_capacity(inputs.len());
    for input in inputs {
        if let Some(unit) = build_unit_values_unit_segment(
            input.unit_index,
            &input.unit_value_map,
            &input.packed_values,
        )? {
            units.push(unit);
        }
    }
    if units.is_empty() {
        return Ok(None);
    }
    units.sort_by_key(|unit| unit.unit_index);

    let segment = UnitValuesSegment { units };
    Ok(Some(ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: encode_unit_values_segment(&segment)?,
    }))
}

fn build_unit_values_unit_segment(
    unit_index: usize,
    unit_value_map: &[StageValue],
    packed_values: &[Felt],
) -> Result<Option<UnitValuesUnitSegment>, ProveUnitValuesSegmentError> {
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

    Ok(Some(UnitValuesUnitSegment {
        unit_index: u32::try_from(unit_index)
            .map_err(|_| ProveUnitValuesSegmentError::UnitIndexOverflow { unit_index })?,
        values: packed_values.iter().map(|value| value.to_u64()).collect(),
    }))
}

pub fn load_unit_values_from_segments(
    unit_index: usize,
    unit_value_map: &[StageValue],
    segments: &[ProofSegment],
) -> Result<Vec<Felt>, LoadUnitValuesSegmentError> {
    let expected_count = expected_packed_unit_value_count(unit_value_map)
        .map_err(|_| LoadUnitValuesSegmentError::LengthOverflow)?;
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == UNIT_VALUES_SEGMENT_ID);
    let segment = matching_segments.next();
    if matching_segments.next().is_some() {
        return Err(LoadUnitValuesSegmentError::DuplicateSegment);
    }
    let parsed = match segment {
        Some(segment) => Some(
            parse_unit_values_segment(&segment.data)
                .map_err(LoadUnitValuesSegmentError::Segment)?,
        ),
        None => None,
    };
    let unit_index_u32 = u32::try_from(unit_index)
        .map_err(|_| LoadUnitValuesSegmentError::UnitIndexOverflow { unit_index })?;
    let unit_values = parsed.as_ref().and_then(|parsed| {
        parsed
            .units
            .iter()
            .find(|unit| unit.unit_index == unit_index_u32)
    });

    if expected_count == 0 {
        if unit_values.is_some() {
            return Err(LoadUnitValuesSegmentError::UnexpectedUnit { unit_index });
        }
        return Ok(Vec::new());
    }

    let unit_values = match (segment, unit_values) {
        (None, _) => return Err(LoadUnitValuesSegmentError::MissingSegment),
        (Some(_), None) => return Err(LoadUnitValuesSegmentError::MissingUnit { unit_index }),
        (Some(_), Some(values)) => values,
    };
    if unit_values.values.len() != expected_count {
        return Err(LoadUnitValuesSegmentError::ValueCountMismatch {
            unit_index,
            expected: expected_count,
            found: unit_values.values.len(),
        });
    }

    unit_values
        .values
        .iter()
        .copied()
        .enumerate()
        .map(|(index, value)| {
            Felt::from_canonical(value).map_err(|source| {
                LoadUnitValuesSegmentError::NonCanonicalValue {
                    unit_index,
                    index,
                    source,
                }
            })
        })
        .collect()
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

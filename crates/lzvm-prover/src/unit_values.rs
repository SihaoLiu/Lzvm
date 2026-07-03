use std::{collections::BTreeSet, fmt};

use lzvm_artifacts::pcs_query_segment::PcsQueryPlanUnit;
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
    pub trace_instance_index: u32,
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
    build_unit_values_segment_from_packed_values_for_identity(
        unit_index,
        0,
        unit_value_map,
        packed_values,
    )
}

pub fn build_unit_values_segment_from_packed_values_for_identity(
    unit_index: usize,
    trace_instance_index: u32,
    unit_value_map: &[StageValue],
    packed_values: &[Felt],
) -> Result<Option<ProofSegment>, ProveUnitValuesSegmentError> {
    let Some(unit) = build_unit_values_unit_segment(
        unit_index,
        trace_instance_index,
        unit_value_map,
        packed_values,
    )?
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
            input.trace_instance_index,
            &input.unit_value_map,
            &input.packed_values,
        )? {
            units.push(unit);
        }
    }
    if units.is_empty() {
        return Ok(None);
    }
    units.sort_by_key(|unit| (unit.unit_index, unit.trace_instance_index));

    let segment = UnitValuesSegment { units };
    Ok(Some(ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: encode_unit_values_segment(&segment)?,
    }))
}

fn build_unit_values_unit_segment(
    unit_index: usize,
    trace_instance_index: u32,
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
        trace_instance_index,
        values: packed_values.iter().map(|value| value.to_u64()).collect(),
    }))
}

pub fn load_unit_values_from_segments(
    unit_index: usize,
    unit_value_map: &[StageValue],
    segments: &[ProofSegment],
) -> Result<Vec<Felt>, LoadUnitValuesSegmentError> {
    load_unit_values_for_identity_from_segments(unit_index, 0, unit_value_map, segments)
}

pub fn load_unit_values_for_identity_from_segments(
    unit_index: usize,
    trace_instance_index: u32,
    unit_value_map: &[StageValue],
    segments: &[ProofSegment],
) -> Result<Vec<Felt>, LoadUnitValuesSegmentError> {
    let expected_count = expected_packed_unit_value_count(unit_value_map)
        .map_err(|_| LoadUnitValuesSegmentError::LengthOverflow)?;
    let parsed = load_unit_values_segment_from_segments(segments)?;
    load_unit_values_for_identity_from_parsed_segment_with_expected_count(
        unit_index,
        trace_instance_index,
        expected_count,
        parsed.as_ref(),
    )
}

pub(crate) fn load_unit_values_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<Option<UnitValuesSegment>, LoadUnitValuesSegmentError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == UNIT_VALUES_SEGMENT_ID);
    let segment = matching_segments.next();
    if matching_segments.next().is_some() {
        return Err(LoadUnitValuesSegmentError::DuplicateSegment);
    }
    segment
        .map(|segment| {
            parse_unit_values_segment(&segment.data).map_err(LoadUnitValuesSegmentError::Segment)
        })
        .transpose()
}

pub(crate) fn load_unit_values_for_identity_from_parsed_segment(
    unit_index: usize,
    trace_instance_index: u32,
    unit_value_map: &[StageValue],
    parsed: Option<&UnitValuesSegment>,
) -> Result<Vec<Felt>, LoadUnitValuesSegmentError> {
    let expected_count = expected_packed_unit_value_count(unit_value_map)
        .map_err(|_| LoadUnitValuesSegmentError::LengthOverflow)?;
    load_unit_values_for_identity_from_parsed_segment_with_expected_count(
        unit_index,
        trace_instance_index,
        expected_count,
        parsed,
    )
}

fn load_unit_values_for_identity_from_parsed_segment_with_expected_count(
    unit_index: usize,
    trace_instance_index: u32,
    expected_count: usize,
    parsed: Option<&UnitValuesSegment>,
) -> Result<Vec<Felt>, LoadUnitValuesSegmentError> {
    let unit_index_u32 = u32::try_from(unit_index)
        .map_err(|_| LoadUnitValuesSegmentError::UnitIndexOverflow { unit_index })?;
    let unit_values = parsed.and_then(|parsed| {
        parsed.units.iter().find(|unit| {
            unit.unit_index == unit_index_u32 && unit.trace_instance_index == trace_instance_index
        })
    });

    if expected_count == 0 {
        if unit_values.is_some() {
            return Err(LoadUnitValuesSegmentError::UnexpectedUnit { unit_index });
        }
        return Ok(Vec::new());
    }

    let unit_values = match (parsed, unit_values) {
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

pub(crate) fn validate_unit_values_units_match_query_units_from_segment(
    query_units: &[PcsQueryPlanUnit],
    parsed: Option<&UnitValuesSegment>,
) -> Result<(), LoadUnitValuesSegmentError> {
    let Some(parsed) = parsed else {
        return Ok(());
    };
    let mut query_identities = BTreeSet::new();
    for unit in query_units {
        let identity = (unit.unit_index, unit.trace_instance_index);
        let unit_index = usize::try_from(unit.unit_index).map_err(|_| {
            LoadUnitValuesSegmentError::UnitIndexOverflow {
                unit_index: usize::MAX,
            }
        })?;
        if !query_identities.insert(identity) {
            return Err(LoadUnitValuesSegmentError::UnexpectedUnit { unit_index });
        }
    }
    let mut unit_value_identities = BTreeSet::new();
    for unit in &parsed.units {
        let identity = (unit.unit_index, unit.trace_instance_index);
        let unit_index = usize::try_from(unit.unit_index).map_err(|_| {
            LoadUnitValuesSegmentError::UnitIndexOverflow {
                unit_index: usize::MAX,
            }
        })?;
        if !query_identities.contains(&identity) || !unit_value_identities.insert(identity) {
            return Err(LoadUnitValuesSegmentError::UnexpectedUnit { unit_index });
        }
    }
    Ok(())
}

pub fn expected_packed_unit_value_count(
    unit_value_map: &[StageValue],
) -> Result<usize, ProveUnitValuesSegmentError> {
    unit_value_map.iter().try_fold(0_usize, |count, value| {
        let dimension = stage_value_dimension(value)?;
        let width = if value.stage == 1 { 1 } else { EXTENSION_WORDS };
        let value_count = dimension
            .checked_mul(width)
            .ok_or(ProveUnitValuesSegmentError::LengthOverflow)?;
        count
            .checked_add(value_count)
            .ok_or(ProveUnitValuesSegmentError::LengthOverflow)
    })
}

fn stage_value_dimension(value: &StageValue) -> Result<usize, ProveUnitValuesSegmentError> {
    value.lengths.iter().try_fold(1_usize, |dimension, length| {
        let length =
            usize::try_from(*length).map_err(|_| ProveUnitValuesSegmentError::LengthOverflow)?;
        if length == 0 {
            return Err(ProveUnitValuesSegmentError::LengthOverflow);
        }
        dimension
            .checked_mul(length)
            .ok_or(ProveUnitValuesSegmentError::LengthOverflow)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_field::MODULUS;

    fn stage_value(name: &str, stage: u32) -> StageValue {
        StageValue {
            name: name.to_owned(),
            stage,
            lengths: Vec::new(),
        }
    }

    #[test]
    fn parsed_unit_values_loader_rejects_noncanonical_in_memory_values() {
        let parsed = UnitValuesSegment {
            units: vec![UnitValuesUnitSegment {
                unit_index: 0,
                trace_instance_index: 0,
                values: vec![MODULUS],
            }],
        };

        let error = load_unit_values_for_identity_from_parsed_segment(
            0,
            0,
            &[stage_value("unit.alpha", 1)],
            Some(&parsed),
        )
        .expect_err("parsed in-memory unit value should still be canonical");

        assert_eq!(
            error,
            LoadUnitValuesSegmentError::NonCanonicalValue {
                unit_index: 0,
                index: 0,
                source: FieldError::NonCanonical { value: MODULUS },
            }
        );
    }

    #[test]
    fn unit_values_match_query_units_rejects_duplicate_in_memory_identity() {
        let query_units = vec![query_unit(0, 1)];
        let parsed = UnitValuesSegment {
            units: vec![unit_values_unit(0, 1), unit_values_unit(0, 1)],
        };

        let error =
            validate_unit_values_units_match_query_units_from_segment(&query_units, Some(&parsed))
                .expect_err("duplicate unit values identity should reject");

        assert_eq!(
            error,
            LoadUnitValuesSegmentError::UnexpectedUnit { unit_index: 0 }
        );
    }

    #[test]
    fn unit_values_match_query_units_rejects_duplicate_query_identity() {
        let query_units = vec![query_unit(0, 1), query_unit(0, 1)];
        let parsed = UnitValuesSegment {
            units: vec![unit_values_unit(0, 1)],
        };

        let error =
            validate_unit_values_units_match_query_units_from_segment(&query_units, Some(&parsed))
                .expect_err("duplicate query identity should reject");

        assert_eq!(
            error,
            LoadUnitValuesSegmentError::UnexpectedUnit { unit_index: 0 }
        );
    }

    fn query_unit(unit_index: u32, trace_instance_index: u32) -> PcsQueryPlanUnit {
        PcsQueryPlanUnit {
            unit_index,
            trace_instance_index,
            queries: vec![0],
        }
    }

    fn unit_values_unit(unit_index: u32, trace_instance_index: u32) -> UnitValuesUnitSegment {
        UnitValuesUnitSegment {
            unit_index,
            trace_instance_index,
            values: Vec::new(),
        }
    }
}

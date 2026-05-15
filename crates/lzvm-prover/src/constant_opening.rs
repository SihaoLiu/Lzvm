use std::fmt;

use lzvm_artifacts::constant_opening_segment::{
    parse_constant_opening_segment, ConstantOpeningSegment, ConstantOpeningSegmentError,
    ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Felt, FieldError};

use crate::constant_tree_opening::{
    constant_tree_merkle_level_count, verify_constant_tree_opening_root, ConstantTreeOpening,
    ConstantTreeOpeningError,
};
use crate::pcs_query_plan::{load_pcs_query_plan_from_segments, LoadPcsQueryPlanSegmentError};
use crate::ProveUnitSchedule;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateConstantOpeningSegmentsError {
    QueryPlan(LoadPcsQueryPlanSegmentError),
    Opening(LoadConstantOpeningSegmentError),
    TreeOpening {
        unit_index: usize,
        source: ConstantTreeOpeningError,
    },
    TreeShape(ConstantTreeOpeningError),
    UnitCountMismatch,
    UnitMismatch {
        unit_index: usize,
    },
    UnitIndexOverflow,
    ArityOverflow,
    WidthOverflow,
    FieldValue(FieldError),
    FieldDigest(FieldError),
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

impl fmt::Display for ValidateConstantOpeningSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryPlan(error) => write!(f, "{error}"),
            Self::Opening(error) => write!(f, "{error}"),
            Self::TreeOpening { unit_index, source } => write!(
                f,
                "invalid constant opening segment for unit {unit_index}: {source}"
            ),
            Self::TreeShape(error) => write!(f, "invalid constant opening segment: {error}"),
            Self::UnitCountMismatch => write!(f, "constant opening segment unit count mismatch"),
            Self::UnitMismatch { unit_index } => {
                write!(f, "constant opening segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "constant opening segment unit index overflow"),
            Self::ArityOverflow => write!(f, "constant opening segment arity overflow"),
            Self::WidthOverflow => write!(f, "constant opening segment width overflow"),
            Self::FieldValue(error) => write!(f, "invalid constant opening segment value: {error}"),
            Self::FieldDigest(error) => {
                write!(f, "invalid constant opening segment digest: {error}")
            }
        }
    }
}

impl std::error::Error for ValidateConstantOpeningSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Opening(error) => Some(error),
            Self::TreeOpening { source, .. } | Self::TreeShape(source) => Some(source),
            Self::FieldValue(error) | Self::FieldDigest(error) => Some(error),
            Self::UnitCountMismatch
            | Self::UnitMismatch { .. }
            | Self::UnitIndexOverflow
            | Self::ArityOverflow
            | Self::WidthOverflow => None,
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

pub fn validate_constant_opening_segments(
    units: &[ProveUnitSchedule],
    segments: &[ProofSegment],
) -> Result<(), ValidateConstantOpeningSegmentsError> {
    let query_plan = load_pcs_query_plan_from_segments(segments)
        .map_err(ValidateConstantOpeningSegmentsError::QueryPlan)?;
    let opening = load_constant_opening_segment_from_segments(segments)
        .map_err(ValidateConstantOpeningSegmentsError::Opening)?;
    if opening.units.len() != query_plan.units.len() {
        return Err(ValidateConstantOpeningSegmentsError::UnitCountMismatch);
    }

    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidateConstantOpeningSegmentsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index })?;
        let opening_unit = opening
            .units
            .iter()
            .find(|unit| unit.unit_index == query_unit.unit_index)
            .ok_or(ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index })?;
        if opening_unit.queries.len() != query_unit.queries.len() {
            return Err(ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index });
        }

        let arity = usize::try_from(unit.merkle_tree_arity)
            .map_err(|_| ValidateConstantOpeningSegmentsError::ArityOverflow)?;
        let expected_level_count =
            constant_tree_merkle_level_count(unit.extended_domain_size, arity)
                .map_err(ValidateConstantOpeningSegmentsError::TreeShape)?;
        let constant_width = usize::try_from(unit.constant_width)
            .map_err(|_| ValidateConstantOpeningSegmentsError::WidthOverflow)?;
        let root = field_digest_from_words(
            unit.pcs_material_constant_tree_root
                .ok_or(ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index })?,
        )?;

        for (query, expected_row) in opening_unit.queries.iter().zip(query_unit.queries.iter()) {
            if query.row_index != *expected_row
                || query.values.len() != constant_width
                || query.siblings.len() != expected_level_count
            {
                return Err(ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index });
            }
            let values = query
                .values
                .iter()
                .map(|value| Felt::from_canonical(*value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(ValidateConstantOpeningSegmentsError::FieldValue)?;
            let siblings = query
                .siblings
                .iter()
                .map(|level| {
                    if level.siblings.len() + 1 != arity {
                        return Err(ValidateConstantOpeningSegmentsError::UnitMismatch {
                            unit_index,
                        });
                    }
                    level
                        .siblings
                        .iter()
                        .map(|digest| field_digest_from_words(*digest))
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            let opening =
                ConstantTreeOpening::new(query.row_index, values, siblings).map_err(|source| {
                    ValidateConstantOpeningSegmentsError::TreeOpening { unit_index, source }
                })?;
            let valid =
                verify_constant_tree_opening_root(root, arity, &opening).map_err(|source| {
                    ValidateConstantOpeningSegmentsError::TreeOpening { unit_index, source }
                })?;
            if !valid {
                return Err(ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index });
            }
        }
    }
    Ok(())
}

fn field_digest_from_words(
    words: [u64; 4],
) -> Result<[Felt; 4], ValidateConstantOpeningSegmentsError> {
    let mut out = [Felt::ZERO; 4];
    for (target, value) in out.iter_mut().zip(words) {
        *target = Felt::from_canonical(value)
            .map_err(ValidateConstantOpeningSegmentsError::FieldDigest)?;
    }
    Ok(out)
}

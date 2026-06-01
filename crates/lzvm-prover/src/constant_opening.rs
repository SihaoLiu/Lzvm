use std::fmt;

use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, parse_constant_opening_segment, ConstantOpeningLevelSegment,
    ConstantOpeningQuerySegment, ConstantOpeningSegment, ConstantOpeningSegmentError,
    ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::{read_constant_tree_file, ConstantTreeError};
use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegmentError, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Felt, FieldError};

use crate::constant_tree_opening::open_constant_tree_row;
use crate::constant_tree_opening::{
    constant_tree_merkle_level_count, verify_constant_tree_opening_root, ConstantTreeOpening,
    ConstantTreeOpeningError,
};
use crate::pcs_query_plan::{
    load_pcs_query_plan_from_segments, unsupported_pcs_query_trace_instance,
    LoadPcsQueryPlanSegmentError,
};
use crate::ProveSchedule;
use crate::ProveUnitSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveConstantOpeningSegmentError {
    QueryPlan(PcsQueryPlanSegmentError),
    UnsupportedTraceInstance {
        unit_index: u32,
        trace_instance_index: u32,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    UnitIndexOverflow {
        unit_index: u32,
    },
    ConstantTree {
        unit_index: usize,
        source: ConstantTreeError,
    },
    Opening(ConstantTreeOpeningError),
    Segment(ConstantOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadConstantOpeningSegmentError {
    MissingSegment,
    DuplicateSegment,
    Segment(ConstantOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadConstantOpeningUnitError {
    MissingSegment,
    DuplicateSegment,
    MissingUnit { unit_index: usize },
    UnexpectedUnit { unit_index: usize },
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

impl fmt::Display for ProveConstantOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryPlan(error) => {
                write!(f, "prove constant opening query plan parse failed: {error}")
            }
            Self::UnsupportedTraceInstance {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "prove constant opening query plan trace instance {trace_instance_index} for unit {unit_index} is unsupported"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove constant opening unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::UnitIndexOverflow { unit_index } => write!(
                f,
                "prove constant opening unit index does not fit usize: {unit_index}"
            ),
            Self::ConstantTree { unit_index, source } => write!(
                f,
                "prove constant opening tree read failed for unit {unit_index}: {source}"
            ),
            Self::Opening(error) => write!(f, "prove constant opening failed: {error}"),
            Self::Segment(error) => {
                write!(f, "prove constant opening segment encode failed: {error}")
            }
        }
    }
}

impl std::error::Error for ProveConstantOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::ConstantTree { source, .. } => Some(source),
            Self::Opening(error) => Some(error),
            Self::Segment(error) => Some(error),
            Self::UnsupportedTraceInstance { .. }
            | Self::UnitIndexOutOfRange { .. }
            | Self::UnitIndexOverflow { .. } => None,
        }
    }
}

impl From<PcsQueryPlanSegmentError> for ProveConstantOpeningSegmentError {
    fn from(error: PcsQueryPlanSegmentError) -> Self {
        Self::QueryPlan(error)
    }
}

impl From<ConstantTreeOpeningError> for ProveConstantOpeningSegmentError {
    fn from(error: ConstantTreeOpeningError) -> Self {
        Self::Opening(error)
    }
}

impl From<ConstantOpeningSegmentError> for ProveConstantOpeningSegmentError {
    fn from(error: ConstantOpeningSegmentError) -> Self {
        Self::Segment(error)
    }
}

impl fmt::Display for LoadConstantOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing constant opening segment"),
            Self::DuplicateSegment => write!(f, "duplicate constant opening segment"),
            Self::Segment(error) => write!(f, "invalid constant opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadConstantOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment | Self::DuplicateSegment => None,
        }
    }
}

impl fmt::Display for LoadConstantOpeningUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing constant opening segment"),
            Self::DuplicateSegment => write!(f, "duplicate constant opening segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "constant opening segment mismatch for unit {unit_index}")
            }
            Self::UnexpectedUnit { unit_index } => {
                write!(f, "unexpected constant opening segment unit {unit_index}")
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
            Self::MissingSegment
            | Self::DuplicateSegment
            | Self::MissingUnit { .. }
            | Self::UnexpectedUnit { .. }
            | Self::UnitIndexOverflow => None,
        }
    }
}

impl From<LoadConstantOpeningSegmentError> for LoadConstantOpeningUnitError {
    fn from(error: LoadConstantOpeningSegmentError) -> Self {
        match error {
            LoadConstantOpeningSegmentError::MissingSegment => Self::MissingSegment,
            LoadConstantOpeningSegmentError::DuplicateSegment => Self::DuplicateSegment,
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
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(LoadConstantOpeningSegmentError::MissingSegment)?;
    if matching_segments.next().is_some() {
        return Err(LoadConstantOpeningSegmentError::DuplicateSegment);
    }
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

pub(crate) fn validate_constant_opening_units_match_query_units(
    query_units: &[PcsQueryPlanUnit],
    segments: &[ProofSegment],
) -> Result<(), LoadConstantOpeningUnitError> {
    let opening = load_constant_opening_segment_from_segments(segments)?;
    for unit in opening.units {
        if !query_units
            .iter()
            .any(|query_unit| query_unit.unit_index == unit.unit_index)
        {
            let unit_index = usize::try_from(unit.unit_index)
                .map_err(|_| LoadConstantOpeningUnitError::UnitIndexOverflow)?;
            return Err(LoadConstantOpeningUnitError::UnexpectedUnit { unit_index });
        }
    }
    Ok(())
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

pub fn build_constant_opening_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
) -> Result<ProofSegment, ProveConstantOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    if let Some(unsupported) = unsupported_pcs_query_trace_instance(&query_plan.units) {
        return Err(ProveConstantOpeningSegmentError::UnsupportedTraceInstance {
            unit_index: unsupported.unit_index,
            trace_instance_index: unsupported.trace_instance_index,
        });
    }
    let mut units = Vec::with_capacity(query_plan.units.len());
    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOverflow {
                unit_index: query_unit.unit_index,
            }
        })?;
        let schedule_unit = schedule.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let catalog_unit = catalog.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: catalog.units.len(),
            },
        )?;
        let tree = read_constant_tree_file(
            &catalog_unit.paths.constant_tree,
            &catalog_unit.metadata.setup,
        )
        .map_err(|source| ProveConstantOpeningSegmentError::ConstantTree { unit_index, source })?;
        let arity = usize::try_from(schedule_unit.merkle_tree_arity).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            }
        })?;
        let mut queries = Vec::with_capacity(query_unit.queries.len());
        for row_index in &query_unit.queries {
            let opening = open_constant_tree_row(&tree, *row_index, arity)?;
            queries.push(ConstantOpeningQuerySegment {
                row_index: *row_index,
                values: opening
                    .values()
                    .iter()
                    .map(|value| value.to_u64())
                    .collect(),
                siblings: opening
                    .siblings()
                    .iter()
                    .map(|level| ConstantOpeningLevelSegment {
                        siblings: level
                            .iter()
                            .map(|digest| digest.map(|value| value.to_u64()))
                            .collect(),
                    })
                    .collect(),
            });
        }
        units.push(ConstantOpeningUnitSegment {
            unit_index: query_unit.unit_index,
            queries,
        });
    }

    let segment = ConstantOpeningSegment { units };
    Ok(ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: encode_constant_opening_segment(&segment)?,
    })
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

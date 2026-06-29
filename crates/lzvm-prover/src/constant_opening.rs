use std::{collections::BTreeSet, fmt};

use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, parse_constant_opening_segment, ConstantOpeningLevelSegment,
    ConstantOpeningQuerySegment, ConstantOpeningSegment, ConstantOpeningSegmentError,
    ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::{
    expected_constant_tree_byte_count, expected_constant_tree_leaf_node_byte_counts,
    summarize_constant_tree_file, ConstantTreeError, ConstantTreeFileSummary,
};
use lzvm_artifacts::key_directory::{KeyDirectoryCatalog, KeyUnitCatalogEntry};
use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegmentError, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_field::{Felt, FieldError};

use crate::constant_tree_opening::open_constant_tree_row_from_file;
use crate::constant_tree_opening::{
    constant_tree_merkle_level_count, verify_constant_tree_opening_root, ConstantTreeOpening,
    ConstantTreeOpeningError,
};
use crate::pcs_query_plan::{load_pcs_query_plan_from_segments, LoadPcsQueryPlanSegmentError};
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
    MissingConstantTreeRoot {
        unit_index: usize,
    },
    MissingConstantTreeMaterial {
        unit_index: usize,
    },
    ConstantTreeMaterialMismatch {
        unit_index: usize,
        field: &'static str,
    },
    Opening(ConstantTreeOpeningError),
    OpeningRootMismatch {
        unit_index: usize,
        row_index: u64,
    },
    NoConstantOpenings,
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
            Self::MissingConstantTreeRoot { unit_index } => write!(
                f,
                "prove constant opening is missing scheduled constant-tree root for unit {unit_index}"
            ),
            Self::MissingConstantTreeMaterial { unit_index } => write!(
                f,
                "prove constant opening is missing scheduled constant-tree material for unit {unit_index}"
            ),
            Self::ConstantTreeMaterialMismatch { unit_index, field } => write!(
                f,
                "prove constant opening constant-tree {field} mismatch for unit {unit_index}"
            ),
            Self::Opening(error) => write!(f, "prove constant opening failed: {error}"),
            Self::OpeningRootMismatch {
                unit_index,
                row_index,
            } => write!(
                f,
                "prove constant opening root mismatch for unit {unit_index} row {row_index}"
            ),
            Self::NoConstantOpenings => write!(
                f,
                "prove constant opening has no constant-width query units"
            ),
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
            | Self::UnitIndexOverflow { .. }
            | Self::MissingConstantTreeRoot { .. }
            | Self::MissingConstantTreeMaterial { .. }
            | Self::ConstantTreeMaterialMismatch { .. }
            | Self::OpeningRootMismatch { .. }
            | Self::NoConstantOpenings => None,
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
            Self::FieldDigest(error) => Some(error),
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
    load_constant_opening_unit_for_identity_from_segments(unit_index, 0, segments)
}

pub fn load_constant_opening_unit_for_identity_from_segments(
    unit_index: usize,
    trace_instance_index: u32,
    segments: &[ProofSegment],
) -> Result<ConstantOpeningUnitSegment, LoadConstantOpeningUnitError> {
    let opening = load_constant_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadConstantOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| {
            unit.unit_index == unit_index_u32 && unit.trace_instance_index == trace_instance_index
        })
        .ok_or(LoadConstantOpeningUnitError::MissingUnit { unit_index })
}

pub(crate) fn validate_constant_opening_units_match_query_units_from_segment(
    query_units: &[PcsQueryPlanUnit],
    opening: &ConstantOpeningSegment,
) -> Result<(), LoadConstantOpeningUnitError> {
    let query_identities = query_units
        .iter()
        .map(|unit| (unit.unit_index, unit.trace_instance_index))
        .collect::<BTreeSet<_>>();
    let mut opening_identities = BTreeSet::new();
    for unit in &opening.units {
        let identity = (unit.unit_index, unit.trace_instance_index);
        let unit_index = usize::try_from(unit.unit_index)
            .map_err(|_| LoadConstantOpeningUnitError::UnitIndexOverflow)?;
        if !query_identities.contains(&identity) || !opening_identities.insert(identity) {
            return Err(LoadConstantOpeningUnitError::UnexpectedUnit { unit_index });
        }
    }
    for query_unit in query_units {
        let identity = (query_unit.unit_index, query_unit.trace_instance_index);
        if !opening_identities.contains(&identity) {
            let unit_index = usize::try_from(query_unit.unit_index)
                .map_err(|_| LoadConstantOpeningUnitError::UnitIndexOverflow)?;
            return Err(LoadConstantOpeningUnitError::MissingUnit { unit_index });
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
    let required_query_units = constant_opening_query_units(units, &query_plan.units)?;
    let opening = match load_constant_opening_segment_from_segments(segments) {
        Ok(opening) => opening,
        Err(LoadConstantOpeningSegmentError::MissingSegment) if required_query_units.is_empty() => {
            return Ok(());
        }
        Err(error) => return Err(ValidateConstantOpeningSegmentsError::Opening(error)),
    };
    if opening.units.len() != required_query_units.len() {
        return Err(ValidateConstantOpeningSegmentsError::UnitCountMismatch);
    }

    let opening_units_by_identity = constant_opening_units_by_identity(&opening);
    for query_unit in required_query_units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidateConstantOpeningSegmentsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index })?;
        let identity = (query_unit.unit_index, query_unit.trace_instance_index);
        let opening_unit = opening_units_by_identity
            .get(&identity)
            .copied()
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
                .copied()
                .map(Felt::from_u64)
                .collect::<Vec<_>>();
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

fn constant_opening_units_by_identity(
    opening: &ConstantOpeningSegment,
) -> std::collections::BTreeMap<(u32, u32), &ConstantOpeningUnitSegment> {
    opening
        .units
        .iter()
        .map(|unit| ((unit.unit_index, unit.trace_instance_index), unit))
        .collect()
}

pub fn build_constant_opening_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
) -> Result<ProofSegment, ProveConstantOpeningSegmentError> {
    build_optional_constant_opening_segment(catalog, schedule, query_segment)
        .and_then(require_constant_opening_segment)
}

pub fn build_constant_opening_segment_with_material_summaries(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    constant_tree_material_summaries: &[Option<ConstantTreeFileSummary>],
) -> Result<ProofSegment, ProveConstantOpeningSegmentError> {
    build_optional_constant_opening_segment_with_material_summaries(
        catalog,
        schedule,
        query_segment,
        constant_tree_material_summaries,
    )
    .and_then(require_constant_opening_segment)
}

pub fn build_constant_opening_segment_with_schedule_material(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
) -> Result<ProofSegment, ProveConstantOpeningSegmentError> {
    build_optional_constant_opening_segment_with_schedule_material(catalog, schedule, query_segment)
        .and_then(require_constant_opening_segment)
}

pub(crate) fn build_optional_constant_opening_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
) -> Result<Option<ProofSegment>, ProveConstantOpeningSegmentError> {
    build_constant_opening_segment_inner(
        catalog,
        schedule,
        query_segment,
        ConstantTreeMaterialSource::SummarizeFile,
    )
}

pub(crate) fn build_optional_constant_opening_segment_with_material_summaries(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    constant_tree_material_summaries: &[Option<ConstantTreeFileSummary>],
) -> Result<Option<ProofSegment>, ProveConstantOpeningSegmentError> {
    build_constant_opening_segment_inner(
        catalog,
        schedule,
        query_segment,
        ConstantTreeMaterialSource::Prevalidated(constant_tree_material_summaries),
    )
}

pub(crate) fn build_optional_constant_opening_segment_with_schedule_material(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
) -> Result<Option<ProofSegment>, ProveConstantOpeningSegmentError> {
    build_constant_opening_segment_inner(
        catalog,
        schedule,
        query_segment,
        ConstantTreeMaterialSource::ScheduleRoot,
    )
}

pub fn validate_constant_opening_materials(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
) -> Result<Vec<Option<ConstantTreeFileSummary>>, ProveConstantOpeningSegmentError> {
    let mut summaries = vec![None; schedule.units.len()];
    for (unit_index, schedule_unit) in schedule.units.iter().enumerate() {
        if !schedule_unit_has_constant_tree_material(schedule_unit) {
            continue;
        }
        let catalog_unit = catalog.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: catalog.units.len(),
            },
        )?;
        let summary = summarize_constant_tree_file(
            &catalog_unit.paths.constant_tree,
            &catalog_unit.metadata.setup,
        )
        .map_err(|source| ProveConstantOpeningSegmentError::ConstantTree { unit_index, source })?;
        validate_constant_tree_material_summary(unit_index, schedule_unit, &summary)?;
        summaries[unit_index] = Some(summary);
    }
    Ok(summaries)
}

fn build_constant_opening_segment_inner(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    material_source: ConstantTreeMaterialSource<'_>,
) -> Result<Option<ProofSegment>, ProveConstantOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
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
        if !constant_opening_required(schedule_unit) {
            continue;
        }
        let catalog_unit = catalog.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: catalog.units.len(),
            },
        )?;
        let arity = usize::try_from(schedule_unit.merkle_tree_arity).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            }
        })?;
        let root = validate_constant_tree_material_binding(
            unit_index,
            schedule_unit,
            catalog_unit,
            material_source,
        )?;
        let mut queries = Vec::with_capacity(query_unit.queries.len());
        for row_index in &query_unit.queries {
            let opening = open_constant_tree_row_from_file(
                &catalog_unit.paths.constant_tree,
                &catalog_unit.metadata.setup,
                *row_index,
                arity,
            )?;
            let valid = verify_constant_tree_opening_root(root, arity, &opening)?;
            if !valid {
                return Err(ProveConstantOpeningSegmentError::OpeningRootMismatch {
                    unit_index,
                    row_index: *row_index,
                });
            }
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
            trace_instance_index: query_unit.trace_instance_index,
            queries,
        });
    }
    if units.is_empty() {
        return Ok(None);
    }

    let segment = ConstantOpeningSegment { units };
    Ok(Some(ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: encode_constant_opening_segment(&segment)?,
    }))
}

fn require_constant_opening_segment(
    segment: Option<ProofSegment>,
) -> Result<ProofSegment, ProveConstantOpeningSegmentError> {
    segment.ok_or(ProveConstantOpeningSegmentError::NoConstantOpenings)
}

#[derive(Clone, Copy)]
enum ConstantTreeMaterialSource<'a> {
    SummarizeFile,
    Prevalidated(&'a [Option<ConstantTreeFileSummary>]),
    ScheduleRoot,
}

fn schedule_unit_has_constant_tree_material(schedule_unit: &ProveUnitSchedule) -> bool {
    schedule_unit.pcs_material_constant_tree_digest.is_some()
        || schedule_unit.pcs_material_constant_tree_root.is_some()
        || schedule_unit
            .pcs_material_constant_tree_byte_count
            .is_some()
        || schedule_unit.pcs_material_leaf_byte_count.is_some()
        || schedule_unit.pcs_material_node_byte_count.is_some()
}

pub(crate) fn constant_opening_required(schedule_unit: &ProveUnitSchedule) -> bool {
    schedule_unit.constant_width > 0
}

fn constant_opening_query_units<'a>(
    units: &[ProveUnitSchedule],
    query_units: &'a [PcsQueryPlanUnit],
) -> Result<Vec<&'a PcsQueryPlanUnit>, ValidateConstantOpeningSegmentsError> {
    let mut required = Vec::new();
    for query_unit in query_units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidateConstantOpeningSegmentsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidateConstantOpeningSegmentsError::UnitMismatch { unit_index })?;
        if constant_opening_required(unit) {
            required.push(query_unit);
        }
    }
    Ok(required)
}

fn validate_constant_tree_material_binding(
    unit_index: usize,
    schedule_unit: &ProveUnitSchedule,
    catalog_unit: &KeyUnitCatalogEntry,
    material_source: ConstantTreeMaterialSource<'_>,
) -> Result<[Felt; 4], ProveConstantOpeningSegmentError> {
    match material_source {
        ConstantTreeMaterialSource::Prevalidated(summaries) => {
            let summary = summaries.get(unit_index).and_then(Option::as_ref).ok_or(
                ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index },
            )?;
            validate_constant_tree_material_summary(unit_index, schedule_unit, summary)
        }
        ConstantTreeMaterialSource::SummarizeFile => {
            let summary = summarize_constant_tree_file(
                &catalog_unit.paths.constant_tree,
                &catalog_unit.metadata.setup,
            )
            .map_err(|source| ProveConstantOpeningSegmentError::ConstantTree {
                unit_index,
                source,
            })?;
            validate_constant_tree_material_summary(unit_index, schedule_unit, &summary)
        }
        ConstantTreeMaterialSource::ScheduleRoot => {
            validate_constant_tree_material_schedule(unit_index, schedule_unit, catalog_unit)
        }
    }
}

fn validate_constant_tree_material_schedule(
    unit_index: usize,
    schedule_unit: &ProveUnitSchedule,
    catalog_unit: &KeyUnitCatalogEntry,
) -> Result<[Felt; 4], ProveConstantOpeningSegmentError> {
    let _expected_digest = schedule_unit
        .pcs_material_constant_tree_digest
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index })?;
    let expected_root_words = schedule_unit
        .pcs_material_constant_tree_root
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeRoot { unit_index })?;
    let expected_byte_count = schedule_unit
        .pcs_material_constant_tree_byte_count
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index })?;
    let expected_leaf_byte_count = schedule_unit
        .pcs_material_leaf_byte_count
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index })?;
    let expected_node_byte_count = schedule_unit
        .pcs_material_node_byte_count
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index })?;
    let setup_byte_count = expected_constant_tree_byte_count(&catalog_unit.metadata.setup)
        .map_err(|source| ProveConstantOpeningSegmentError::ConstantTree { unit_index, source })?;
    let (setup_leaf_byte_count, setup_node_byte_count) =
        expected_constant_tree_leaf_node_byte_counts(&catalog_unit.metadata.setup).map_err(
            |source| ProveConstantOpeningSegmentError::ConstantTree { unit_index, source },
        )?;
    if u64::try_from(setup_byte_count).map_err(|_| {
        ProveConstantOpeningSegmentError::ConstantTree {
            unit_index,
            source: ConstantTreeError::LengthOverflow,
        }
    })? != expected_byte_count
    {
        return Err(
            ProveConstantOpeningSegmentError::ConstantTreeMaterialMismatch {
                unit_index,
                field: "byte count",
            },
        );
    }
    if u64::try_from(setup_leaf_byte_count).map_err(|_| {
        ProveConstantOpeningSegmentError::ConstantTree {
            unit_index,
            source: ConstantTreeError::LengthOverflow,
        }
    })? != expected_leaf_byte_count
    {
        return Err(
            ProveConstantOpeningSegmentError::ConstantTreeMaterialMismatch {
                unit_index,
                field: "leaf byte count",
            },
        );
    }
    if u64::try_from(setup_node_byte_count).map_err(|_| {
        ProveConstantOpeningSegmentError::ConstantTree {
            unit_index,
            source: ConstantTreeError::LengthOverflow,
        }
    })? != expected_node_byte_count
    {
        return Err(
            ProveConstantOpeningSegmentError::ConstantTreeMaterialMismatch {
                unit_index,
                field: "node byte count",
            },
        );
    }

    opening_root_from_words(expected_root_words)
}

fn validate_constant_tree_material_summary(
    unit_index: usize,
    schedule_unit: &ProveUnitSchedule,
    summary: &ConstantTreeFileSummary,
) -> Result<[Felt; 4], ProveConstantOpeningSegmentError> {
    let expected_digest = schedule_unit
        .pcs_material_constant_tree_digest
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index })?;
    let expected_root_words = schedule_unit
        .pcs_material_constant_tree_root
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeRoot { unit_index })?;
    let expected_byte_count = schedule_unit
        .pcs_material_constant_tree_byte_count
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index })?;
    let expected_leaf_byte_count = schedule_unit
        .pcs_material_leaf_byte_count
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index })?;
    let expected_node_byte_count = schedule_unit
        .pcs_material_node_byte_count
        .ok_or(ProveConstantOpeningSegmentError::MissingConstantTreeMaterial { unit_index })?;

    if summary.digest != expected_digest {
        return Err(ProveConstantOpeningSegmentError::ConstantTree {
            unit_index,
            source: ConstantTreeError::DigestMismatch {
                expected: expected_digest,
                found: summary.digest,
            },
        });
    }
    if summary.byte_count != expected_byte_count {
        return Err(
            ProveConstantOpeningSegmentError::ConstantTreeMaterialMismatch {
                unit_index,
                field: "byte count",
            },
        );
    }
    if u64::try_from(summary.leaf_byte_count).map_err(|_| {
        ProveConstantOpeningSegmentError::ConstantTree {
            unit_index,
            source: ConstantTreeError::LengthOverflow,
        }
    })? != expected_leaf_byte_count
    {
        return Err(
            ProveConstantOpeningSegmentError::ConstantTreeMaterialMismatch {
                unit_index,
                field: "leaf byte count",
            },
        );
    }
    if u64::try_from(summary.node_byte_count).map_err(|_| {
        ProveConstantOpeningSegmentError::ConstantTree {
            unit_index,
            source: ConstantTreeError::LengthOverflow,
        }
    })? != expected_node_byte_count
    {
        return Err(
            ProveConstantOpeningSegmentError::ConstantTreeMaterialMismatch {
                unit_index,
                field: "node byte count",
            },
        );
    }
    let VerificationKeyRoot::FieldElements(root_words) = &summary.root;
    if root_words.as_slice() != expected_root_words.as_slice() {
        return Err(
            ProveConstantOpeningSegmentError::ConstantTreeMaterialMismatch {
                unit_index,
                field: "root",
            },
        );
    }

    opening_root_from_words(expected_root_words)
}

fn opening_root_from_words(words: [u64; 4]) -> Result<[Felt; 4], ProveConstantOpeningSegmentError> {
    let mut out = [Felt::ZERO; 4];
    for (target, value) in out.iter_mut().zip(words) {
        *target = Felt::from_canonical(value).map_err(ConstantTreeOpeningError::Field)?;
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_units_match_query_units_rejects_duplicate_in_memory_identity() {
        let query_units = vec![query_unit(0, 1)];
        let opening = ConstantOpeningSegment {
            units: vec![opening_unit(0, 1), opening_unit(0, 1)],
        };

        let error =
            validate_constant_opening_units_match_query_units_from_segment(&query_units, &opening)
                .expect_err("duplicate constant opening identity should reject");

        assert_eq!(
            error,
            LoadConstantOpeningUnitError::UnexpectedUnit { unit_index: 0 }
        );
    }

    fn query_unit(unit_index: u32, trace_instance_index: u32) -> PcsQueryPlanUnit {
        PcsQueryPlanUnit {
            unit_index,
            trace_instance_index,
            queries: vec![0],
        }
    }

    fn opening_unit(unit_index: u32, trace_instance_index: u32) -> ConstantOpeningUnitSegment {
        ConstantOpeningUnitSegment {
            unit_index,
            trace_instance_index,
            queries: Vec::new(),
        }
    }
}

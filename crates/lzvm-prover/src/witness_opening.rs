#[cfg(feature = "cuda")]
use std::sync::Arc;
use std::time::Instant;
use std::{collections::BTreeSet, fmt};

use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanSegmentError, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, parse_witness_opening_segment, WitnessOpeningLevelSegment,
    WitnessOpeningQuerySegment, WitnessOpeningSegment, WitnessOpeningSegmentError,
    WitnessOpeningStageSegment, WitnessOpeningUnitSegment, WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::WitnessCommitmentSegmentError;
use lzvm_field::{Felt, FieldError};

#[cfg(feature = "cuda")]
use crate::guest_pc_trace_backend::{
    build_guest_pc_trace_descriptor_source_from_device_material_timing,
    build_guest_pc_trace_stage_source_devices_from_device_descriptors_timing,
    build_guest_pc_trace_stage_source_devices_from_device_material_timing,
    GuestPcDeviceSourceBuildTiming,
};
use crate::indexing::collect_unique_query_identities;
use crate::pcs_query_plan::{load_pcs_query_plan_from_segments, LoadPcsQueryPlanSegmentError};
#[cfg(feature = "cuda")]
use crate::proof_artifact_timing::WitnessOpeningSourceKind;
use crate::proof_artifact_timing::WitnessProofArtifactTiming;
use crate::witness_commitment::{
    load_witness_commitment_segment_refs_with_shapes, open_witness_stage_commitments,
    verify_witness_stage_opening_root, LoadWitnessCommitmentSegmentsError,
    LoadedWitnessCommitmentSegmentRef, WitnessStageOpening, WitnessStageOpeningError,
};
#[cfg(feature = "cuda")]
use crate::witness_commitment::{
    open_witness_stage_commitment_batches_with_source_devices_timing, WitnessStageCommitment,
    WitnessStageOpeningBatchRequest, WitnessStageOpeningWorkTiming, WitnessStageSourceDevice,
    WitnessStageSourceDeviceView,
};
use crate::witness_execution::{ProveWitnessCommitments, ProveWitnessTraceCommitments};
#[cfg(feature = "cuda")]
use crate::witness_layout::derive_witness_trace_layout;
use crate::ProveSchedule;
use crate::ProveUnitSchedule;

#[cfg(any(feature = "cuda", test))]
const DEFAULT_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE: usize = 2;
#[cfg(feature = "cuda")]
const TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE_ENV: &str =
    "LZVM_WITNESS_OPENING_EXTERNAL_SOURCE_BATCH_SIZE";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveWitnessOpeningSegmentError {
    QueryPlan(PcsQueryPlanSegmentError),
    UnsupportedTraceInstance {
        unit_index: u32,
        trace_instance_index: u32,
    },
    MissingQueryUnit {
        unit_index: usize,
    },
    MissingOutputUnit {
        unit_index: usize,
    },
    DuplicateOutputUnit {
        unit_index: usize,
    },
    UnitIndexOverflow {
        unit_index: usize,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    StageIndexOutOfRange {
        stage_index: usize,
        stage_count: usize,
    },
    StageOpening {
        unit_index: usize,
        trace_instance_index: u32,
        stage_index: usize,
        source_kind: &'static str,
        source: WitnessStageOpeningError,
    },
    Opening(WitnessStageOpeningError),
    ExternalSource {
        message: String,
    },
    Segment(WitnessOpeningSegmentError),
}

impl fmt::Display for ProveWitnessOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryPlan(error) => {
                write!(f, "prove witness opening query plan parse failed: {error}")
            }
            Self::UnsupportedTraceInstance {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "prove witness opening query plan trace instance {trace_instance_index} for unit {unit_index} is unsupported"
            ),
            Self::MissingQueryUnit { unit_index } => {
                write!(f, "prove witness opening is missing query unit {unit_index}")
            }
            Self::MissingOutputUnit { unit_index } => {
                write!(f, "prove witness opening is missing output unit {unit_index}")
            }
            Self::DuplicateOutputUnit { unit_index } => {
                write!(f, "duplicate prove witness opening output unit: {unit_index}")
            }
            Self::UnitIndexOverflow { unit_index } => write!(
                f,
                "prove witness opening unit index does not fit u32: {unit_index}"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove witness opening unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::StageIndexOutOfRange {
                stage_index,
                stage_count,
            } => write!(
                f,
                "prove witness opening stage index {stage_index} is outside stage count {stage_count}"
            ),
            Self::StageOpening {
                unit_index,
                trace_instance_index,
                stage_index,
                source_kind,
                source,
            } => write!(
                f,
                "prove witness opening failed for unit {unit_index} trace {trace_instance_index} stage {stage_index} source {source_kind}: {source}"
            ),
            Self::Opening(error) => write!(f, "prove witness opening failed: {error}"),
            Self::ExternalSource { message } => {
                write!(f, "prove witness opening external source failed: {message}")
            }
            Self::Segment(error) => write!(f, "prove witness opening segment encode failed: {error}"),
        }
    }
}

impl std::error::Error for ProveWitnessOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::StageOpening { source, .. } => Some(source),
            Self::Opening(error) => Some(error),
            Self::Segment(error) => Some(error),
            Self::UnsupportedTraceInstance { .. }
            | Self::MissingQueryUnit { .. }
            | Self::MissingOutputUnit { .. }
            | Self::DuplicateOutputUnit { .. }
            | Self::UnitIndexOverflow { .. }
            | Self::UnitIndexOutOfRange { .. }
            | Self::StageIndexOutOfRange { .. } => None,
            Self::ExternalSource { .. } => None,
        }
    }
}

impl From<PcsQueryPlanSegmentError> for ProveWitnessOpeningSegmentError {
    fn from(error: PcsQueryPlanSegmentError) -> Self {
        Self::QueryPlan(error)
    }
}

impl From<WitnessStageOpeningError> for ProveWitnessOpeningSegmentError {
    fn from(error: WitnessStageOpeningError) -> Self {
        Self::Opening(error)
    }
}

impl From<WitnessOpeningSegmentError> for ProveWitnessOpeningSegmentError {
    fn from(error: WitnessOpeningSegmentError) -> Self {
        Self::Segment(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWitnessOpeningSegmentError {
    MissingSegment,
    DuplicateSegment,
    Segment(WitnessOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWitnessOpeningUnitError {
    MissingSegment,
    DuplicateSegment,
    MissingUnit { unit_index: usize },
    UnexpectedUnit { unit_index: usize },
    UnitIndexOverflow,
    Segment(WitnessOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateWitnessOpeningSegmentsError {
    QueryPlan(LoadPcsQueryPlanSegmentError),
    Opening(LoadWitnessOpeningSegmentError),
    Commitments(LoadWitnessCommitmentSegmentsError),
    CommitmentSegment {
        unit_index: usize,
        source: WitnessCommitmentSegmentError,
    },
    StageOpening {
        unit_index: usize,
        source: WitnessStageOpeningError,
    },
    UnitCountMismatch,
    UnitMismatch {
        unit_index: usize,
    },
    UnitIndexOverflow,
    SegmentIdOverflow,
    StageIndexOverflow,
    ArityOverflow,
    InvalidTreeShape,
    LevelCountOverflow,
    FieldDigest(FieldError),
}

impl fmt::Display for LoadWitnessOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing witness opening segment"),
            Self::DuplicateSegment => write!(f, "duplicate witness opening segment"),
            Self::Segment(error) => write!(f, "invalid witness opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadWitnessOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment | Self::DuplicateSegment => None,
        }
    }
}

impl fmt::Display for LoadWitnessOpeningUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing witness opening segment"),
            Self::DuplicateSegment => write!(f, "duplicate witness opening segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "witness opening segment mismatch for unit {unit_index}")
            }
            Self::UnexpectedUnit { unit_index } => {
                write!(f, "unexpected witness opening segment unit {unit_index}")
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
            Self::MissingSegment
            | Self::DuplicateSegment
            | Self::MissingUnit { .. }
            | Self::UnexpectedUnit { .. }
            | Self::UnitIndexOverflow => None,
        }
    }
}

impl From<LoadWitnessOpeningSegmentError> for LoadWitnessOpeningUnitError {
    fn from(error: LoadWitnessOpeningSegmentError) -> Self {
        match error {
            LoadWitnessOpeningSegmentError::MissingSegment => Self::MissingSegment,
            LoadWitnessOpeningSegmentError::DuplicateSegment => Self::DuplicateSegment,
            LoadWitnessOpeningSegmentError::Segment(error) => Self::Segment(error),
        }
    }
}

impl fmt::Display for ValidateWitnessOpeningSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryPlan(error) => write!(f, "{error}"),
            Self::Opening(error) => write!(f, "{error}"),
            Self::Commitments(error) => write!(f, "{error}"),
            Self::CommitmentSegment { unit_index, source } => write!(
                f,
                "invalid witness commitment segment for unit {unit_index}: {source}"
            ),
            Self::StageOpening { unit_index, source } => write!(
                f,
                "invalid witness opening segment for unit {unit_index}: {source}"
            ),
            Self::UnitCountMismatch => write!(f, "witness opening segment unit count mismatch"),
            Self::UnitMismatch { unit_index } => {
                write!(f, "witness opening segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "witness opening segment unit index overflow"),
            Self::SegmentIdOverflow => write!(f, "witness opening segment id overflow"),
            Self::StageIndexOverflow => write!(f, "witness opening segment stage index overflow"),
            Self::ArityOverflow => write!(f, "witness opening segment arity overflow"),
            Self::InvalidTreeShape => write!(f, "witness opening segment invalid tree shape"),
            Self::LevelCountOverflow => write!(f, "witness opening segment level count overflow"),
            Self::FieldDigest(error) => {
                write!(f, "invalid witness opening segment digest: {error}")
            }
        }
    }
}

impl std::error::Error for ValidateWitnessOpeningSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Opening(error) => Some(error),
            Self::Commitments(error) => Some(error),
            Self::CommitmentSegment { source, .. } => Some(source),
            Self::StageOpening { source, .. } => Some(source),
            Self::FieldDigest(error) => Some(error),
            Self::UnitCountMismatch
            | Self::UnitMismatch { .. }
            | Self::UnitIndexOverflow
            | Self::SegmentIdOverflow
            | Self::StageIndexOverflow
            | Self::ArityOverflow
            | Self::InvalidTreeShape
            | Self::LevelCountOverflow => None,
        }
    }
}

pub fn load_witness_opening_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<WitnessOpeningSegment, LoadWitnessOpeningSegmentError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(LoadWitnessOpeningSegmentError::MissingSegment)?;
    if matching_segments.next().is_some() {
        return Err(LoadWitnessOpeningSegmentError::DuplicateSegment);
    }
    parse_witness_opening_segment(&segment.data).map_err(LoadWitnessOpeningSegmentError::Segment)
}

pub fn load_witness_opening_unit_from_segments(
    unit_index: usize,
    segments: &[ProofSegment],
) -> Result<WitnessOpeningUnitSegment, LoadWitnessOpeningUnitError> {
    load_witness_opening_unit_for_identity_from_segments(unit_index, 0, segments)
}

pub fn load_witness_opening_unit_for_identity_from_segments(
    unit_index: usize,
    trace_instance_index: u32,
    segments: &[ProofSegment],
) -> Result<WitnessOpeningUnitSegment, LoadWitnessOpeningUnitError> {
    let opening = load_witness_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadWitnessOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| {
            unit.unit_index == unit_index_u32 && unit.trace_instance_index == trace_instance_index
        })
        .ok_or(LoadWitnessOpeningUnitError::MissingUnit { unit_index })
}

pub(crate) fn validate_witness_opening_units_match_query_units_from_segment(
    query_units: &[PcsQueryPlanUnit],
    opening: &WitnessOpeningSegment,
) -> Result<(), LoadWitnessOpeningUnitError> {
    let query_identities = collect_unique_query_identities(
        query_units,
        || LoadWitnessOpeningUnitError::UnitIndexOverflow,
        |unit_index| LoadWitnessOpeningUnitError::UnexpectedUnit { unit_index },
    )?;
    let mut opening_identities = BTreeSet::new();
    for unit in &opening.units {
        let identity = (unit.unit_index, unit.trace_instance_index);
        let unit_index = usize::try_from(unit.unit_index)
            .map_err(|_| LoadWitnessOpeningUnitError::UnitIndexOverflow)?;
        if !query_identities.contains(&identity) || !opening_identities.insert(identity) {
            return Err(LoadWitnessOpeningUnitError::UnexpectedUnit { unit_index });
        }
    }
    for query_unit in query_units {
        let identity = (query_unit.unit_index, query_unit.trace_instance_index);
        if !opening_identities.contains(&identity) {
            let unit_index = usize::try_from(query_unit.unit_index)
                .map_err(|_| LoadWitnessOpeningUnitError::UnitIndexOverflow)?;
            return Err(LoadWitnessOpeningUnitError::MissingUnit { unit_index });
        }
    }
    Ok(())
}

pub fn validate_witness_opening_segments(
    units: &[ProveUnitSchedule],
    segments: &[ProofSegment],
) -> Result<(), ValidateWitnessOpeningSegmentsError> {
    let query_plan = load_pcs_query_plan_from_segments(segments)
        .map_err(ValidateWitnessOpeningSegmentsError::QueryPlan)?;
    let opening = load_witness_opening_segment_from_segments(segments)
        .map_err(ValidateWitnessOpeningSegmentsError::Opening)?;
    if opening.units.len() != query_plan.units.len() {
        return Err(ValidateWitnessOpeningSegmentsError::UnitCountMismatch);
    }

    let witness_segments = load_witness_commitment_segment_refs_with_shapes(units, segments)
        .map_err(ValidateWitnessOpeningSegmentsError::Commitments)?;
    let opening_units_by_identity = witness_opening_units_by_identity(&opening);
    let witness_segments_by_identity = witness_commitment_segments_by_identity(&witness_segments);
    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidateWitnessOpeningSegmentsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        let identity = (query_unit.unit_index, query_unit.trace_instance_index);
        let opening_unit = opening_units_by_identity
            .get(&identity)
            .copied()
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        if opening_unit.queries.len() != query_unit.queries.len() {
            return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
        }

        let witness_segment = witness_segments_by_identity
            .get(&identity)
            .copied()
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        let arity = usize::try_from(unit.merkle_tree_arity)
            .map_err(|_| ValidateWitnessOpeningSegmentsError::ArityOverflow)?;
        let expected_level_count = expected_merkle_level_count(unit.extended_domain_size, arity)?;

        for (query, expected_row) in opening_unit.queries.iter().zip(query_unit.queries.iter()) {
            if query.row_index != *expected_row
                || query.stages.len() != witness_segment.witness.stages.len()
            {
                return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
            }
            for stage in &query.stages {
                let stage_index = usize::try_from(stage.stage_index)
                    .map_err(|_| ValidateWitnessOpeningSegmentsError::StageIndexOverflow)?;
                let Some(stage_slot) = stage_index.checked_sub(1) else {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                };
                let Some(width) = unit.stage_commit_widths.get(stage_slot) else {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                };
                if stage.values.len() != *width as usize
                    || stage.siblings.len() != expected_level_count
                {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                }
                let Some(witness_stage) = witness_segment
                    .witness
                    .stages
                    .get(stage_slot)
                    .filter(|witness_stage| witness_stage.stage_index == stage.stage_index)
                else {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                };
                let values = stage
                    .values
                    .iter()
                    .copied()
                    .map(Felt::from_u64)
                    .collect::<Vec<_>>();
                let siblings = stage
                    .siblings
                    .iter()
                    .map(|level| {
                        if level.siblings.len() + 1 != arity {
                            return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch {
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
                let opening = WitnessStageOpening::new(query.row_index, values, siblings).map_err(
                    |source| ValidateWitnessOpeningSegmentsError::StageOpening {
                        unit_index,
                        source,
                    },
                )?;
                let root = field_digest_from_words(witness_stage.root)?;
                let stage_arity = usize::try_from(witness_stage.arity)
                    .map_err(|_| ValidateWitnessOpeningSegmentsError::ArityOverflow)?;
                let valid = verify_witness_stage_opening_root(root, stage_arity, &opening)
                    .map_err(|source| ValidateWitnessOpeningSegmentsError::StageOpening {
                        unit_index,
                        source,
                    })?;
                if !valid {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                }
            }
        }
    }
    Ok(())
}

fn witness_opening_units_by_identity(
    opening: &WitnessOpeningSegment,
) -> std::collections::BTreeMap<(u32, u32), &WitnessOpeningUnitSegment> {
    opening
        .units
        .iter()
        .map(|unit| ((unit.unit_index, unit.trace_instance_index), unit))
        .collect()
}

fn witness_commitment_segments_by_identity<'loaded, 'segment>(
    witness_segments: &'loaded [LoadedWitnessCommitmentSegmentRef<'segment>],
) -> std::collections::BTreeMap<(u32, u32), &'loaded LoadedWitnessCommitmentSegmentRef<'segment>> {
    witness_segments
        .iter()
        .map(|segment| {
            (
                (
                    segment.identity.unit_index,
                    segment.identity.trace_instance_index,
                ),
                segment,
            )
        })
        .collect()
}

pub fn build_witness_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_query_plan_segment(query_segment)?;
    build_witness_opening_segment_from_query_plan(schedule, &query_plan, &[output])
}

pub fn build_witness_opening_segment_batch(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    outputs: &[&ProveWitnessCommitments],
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_query_plan_segment(query_segment)?;
    build_witness_opening_segment_from_query_plan(schedule, &query_plan, outputs)
}

pub(crate) fn build_witness_opening_segment_batch_from_trace_outputs(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    outputs: &[&ProveWitnessTraceCommitments],
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_query_plan_segment(query_segment)?;
    build_witness_opening_segment_from_trace_outputs(schedule, &query_plan, outputs, None)
}

pub(crate) fn build_witness_opening_segment_batch_from_trace_outputs_with_timing(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    outputs: &[&ProveWitnessTraceCommitments],
    timing: &mut WitnessProofArtifactTiming,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_query_plan_segment(query_segment)?;
    build_witness_opening_segment_from_trace_outputs(schedule, &query_plan, outputs, Some(timing))
}

fn parse_query_plan_segment(
    query_segment: &ProofSegment,
) -> Result<PcsQueryPlanSegment, ProveWitnessOpeningSegmentError> {
    Ok(parse_pcs_query_plan_segment(&query_segment.data)?)
}

fn witness_stage_width(
    unit: &ProveUnitSchedule,
    stage_index: usize,
) -> Result<usize, ProveWitnessOpeningSegmentError> {
    let width = unit
        .stage_commit_widths
        .get(stage_index.checked_sub(1).ok_or(
            ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                stage_index,
                stage_count: unit.stage_commit_widths.len(),
            },
        )?)
        .ok_or(ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
            stage_index,
            stage_count: unit.stage_commit_widths.len(),
        })?;
    usize::try_from(*width).map_err(|_| ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
        stage_index,
        stage_count: unit.stage_commit_widths.len(),
    })
}

fn witness_opening_stage_segment(
    stage_index: usize,
    stage_count: usize,
    opening: &WitnessStageOpening,
) -> Result<WitnessOpeningStageSegment, ProveWitnessOpeningSegmentError> {
    Ok(WitnessOpeningStageSegment {
        stage_index: u32::try_from(stage_index).map_err(|_| {
            ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                stage_index,
                stage_count,
            }
        })?,
        values: opening
            .values()
            .iter()
            .map(|value| value.to_u64())
            .collect(),
        siblings: opening
            .siblings()
            .iter()
            .map(|level| WitnessOpeningLevelSegment {
                siblings: level
                    .iter()
                    .map(|digest| digest.map(|value| value.to_u64()))
                    .collect(),
            })
            .collect(),
    })
}

fn build_witness_opening_segment_from_query_plan(
    schedule: &ProveSchedule,
    query_plan: &PcsQueryPlanSegment,
    outputs: &[&ProveWitnessCommitments],
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let mut outputs_by_unit = std::collections::BTreeMap::new();
    for output in outputs {
        let unit_index_u32 = u32::try_from(output.unit_index()).map_err(|_| {
            ProveWitnessOpeningSegmentError::UnitIndexOverflow {
                unit_index: output.unit_index(),
            }
        })?;
        let identity = (unit_index_u32, output.trace_instance_index());
        if outputs_by_unit.insert(identity, *output).is_some() {
            return Err(ProveWitnessOpeningSegmentError::DuplicateOutputUnit {
                unit_index: output.unit_index(),
            });
        }
    }

    let query_units = query_plan
        .units
        .iter()
        .map(|unit| (unit.unit_index, unit.trace_instance_index))
        .collect::<std::collections::BTreeSet<_>>();
    for (unit_index_u32, trace_instance_index) in outputs_by_unit.keys() {
        if !query_units.contains(&(*unit_index_u32, *trace_instance_index)) {
            return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
                unit_index: *unit_index_u32 as usize,
            });
        }
    }

    let mut units = Vec::with_capacity(query_plan.units.len());
    for query_unit in &query_plan.units {
        let unit_index = query_unit.unit_index as usize;
        let output = outputs_by_unit
            .get(&(query_unit.unit_index, query_unit.trace_instance_index))
            .ok_or(ProveWitnessOpeningSegmentError::MissingOutputUnit { unit_index })?;
        units.push(build_witness_opening_unit_segment(
            schedule, query_unit, output,
        )?);
    }

    let segment = WitnessOpeningSegment { units };
    Ok(ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: encode_witness_opening_segment(&segment)?,
    })
}

fn build_witness_opening_segment_from_trace_outputs(
    schedule: &ProveSchedule,
    query_plan: &PcsQueryPlanSegment,
    outputs: &[&ProveWitnessTraceCommitments],
    timing: Option<&mut WitnessProofArtifactTiming>,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let mut outputs_by_unit = std::collections::BTreeMap::new();
    for output in outputs {
        let commitments = output.commitments();
        let unit_index_u32 = u32::try_from(commitments.unit_index()).map_err(|_| {
            ProveWitnessOpeningSegmentError::UnitIndexOverflow {
                unit_index: commitments.unit_index(),
            }
        })?;
        let identity = (unit_index_u32, commitments.trace_instance_index());
        if outputs_by_unit.insert(identity, *output).is_some() {
            return Err(ProveWitnessOpeningSegmentError::DuplicateOutputUnit {
                unit_index: commitments.unit_index(),
            });
        }
    }

    let query_units = query_plan
        .units
        .iter()
        .map(|unit| (unit.unit_index, unit.trace_instance_index))
        .collect::<std::collections::BTreeSet<_>>();
    for (unit_index_u32, trace_instance_index) in outputs_by_unit.keys() {
        if !query_units.contains(&(*unit_index_u32, *trace_instance_index)) {
            return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
                unit_index: *unit_index_u32 as usize,
            });
        }
    }

    #[cfg(feature = "cuda")]
    {
        build_witness_opening_segment_from_trace_outputs_cuda_batched(
            schedule,
            query_plan,
            &outputs_by_unit,
            timing,
        )
    }

    #[cfg(not(feature = "cuda"))]
    {
        let mut timing = timing;
        let mut units = Vec::with_capacity(query_plan.units.len());
        for query_unit in &query_plan.units {
            let unit_index = query_unit.unit_index as usize;
            let output = outputs_by_unit
                .get(&(query_unit.unit_index, query_unit.trace_instance_index))
                .ok_or(ProveWitnessOpeningSegmentError::MissingOutputUnit { unit_index })?;
            units.push(build_witness_opening_unit_segment_from_trace_output(
                schedule,
                query_unit,
                output,
                timing.as_deref_mut(),
            )?);
        }

        let segment = WitnessOpeningSegment { units };
        Ok(ProofSegment {
            id: WITNESS_OPENING_SEGMENT_ID,
            data: encode_witness_opening_segment(&segment)?,
        })
    }
}

#[cfg(feature = "cuda")]
struct TraceOutputOpeningStageWork<'a> {
    commitment: &'a WitnessStageCommitment,
    stage_index: usize,
    width: usize,
    source_kind: WitnessOpeningSourceKind,
    stage_opening_start: Instant,
}

#[cfg(feature = "cuda")]
struct TraceOutputOpeningUnitWork<'a> {
    query_unit: &'a PcsQueryPlanUnit,
    unit: &'a ProveUnitSchedule,
    commitments: &'a ProveWitnessCommitments,
    query_stages: Vec<Vec<WitnessOpeningStageSegment>>,
    stage_entries: Vec<TraceOutputOpeningStageWork<'a>>,
    stage_source_views: Vec<Option<WitnessStageSourceDeviceView>>,
}

#[cfg(feature = "cuda")]
fn build_witness_opening_segment_from_trace_outputs_cuda_batched(
    schedule: &ProveSchedule,
    query_plan: &PcsQueryPlanSegment,
    outputs_by_unit: &std::collections::BTreeMap<(u32, u32), &ProveWitnessTraceCommitments>,
    mut timing: Option<&mut WitnessProofArtifactTiming>,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let mut units = Vec::with_capacity(query_plan.units.len());
    let mut pending_works = Vec::new();
    let mut pending_external_source_works = 0usize;
    let external_source_batch_size = trace_output_external_source_opening_batch_size();
    for query_unit in &query_plan.units {
        let unit_index = query_unit.unit_index as usize;
        let output = outputs_by_unit
            .get(&(query_unit.unit_index, query_unit.trace_instance_index))
            .ok_or(ProveWitnessOpeningSegmentError::MissingOutputUnit { unit_index })?;
        let needs_external_source = trace_output_opening_unit_needs_external_source(output);
        if needs_external_source {
            if pending_external_source_works == 0 && !pending_works.is_empty() {
                append_trace_output_opening_units_from_prepared_cuda_batch(
                    std::mem::take(&mut pending_works),
                    timing.as_deref_mut(),
                    &mut units,
                )?;
            }
            pending_works.push(prepare_trace_output_opening_unit_work(
                schedule,
                query_unit,
                output,
                timing.as_deref_mut(),
            )?);
            pending_external_source_works = pending_external_source_works.checked_add(1).ok_or(
                ProveWitnessOpeningSegmentError::StageOpening {
                    unit_index: 0,
                    trace_instance_index: 0,
                    stage_index: 0,
                    source_kind: "external",
                    source: WitnessStageOpeningError::LengthOverflow,
                },
            )?;
            if pending_external_source_works >= external_source_batch_size {
                append_trace_output_opening_units_from_prepared_cuda_batch(
                    std::mem::take(&mut pending_works),
                    timing.as_deref_mut(),
                    &mut units,
                )?;
                pending_external_source_works = 0;
            }
        } else {
            if pending_external_source_works > 0 {
                append_trace_output_opening_units_from_prepared_cuda_batch(
                    std::mem::take(&mut pending_works),
                    timing.as_deref_mut(),
                    &mut units,
                )?;
                pending_external_source_works = 0;
            }
            pending_works.push(prepare_trace_output_opening_unit_work(
                schedule,
                query_unit,
                output,
                timing.as_deref_mut(),
            )?);
        }
    }
    append_trace_output_opening_units_from_prepared_cuda_batch(pending_works, timing, &mut units)?;

    let segment = WitnessOpeningSegment { units };
    Ok(ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: encode_witness_opening_segment(&segment)?,
    })
}

#[cfg(feature = "cuda")]
fn trace_output_external_source_opening_batch_size() -> usize {
    let value = std::env::var(TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE_ENV).ok();
    parse_trace_output_external_source_opening_batch_size(value.as_deref())
}

#[cfg(any(feature = "cuda", test))]
fn parse_trace_output_external_source_opening_batch_size(value: Option<&str>) -> usize {
    value
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE)
}

#[cfg(feature = "cuda")]
fn trace_output_opening_unit_needs_external_source(output: &ProveWitnessTraceCommitments) -> bool {
    output
        .commitments()
        .stage_commitments()
        .commitments()
        .iter()
        .any(|commitment| {
            commitment.requires_external_source()
                && output
                    .stage_source_device_view(commitment.stage_index())
                    .is_none()
        })
}

#[cfg(feature = "cuda")]
fn append_trace_output_opening_units_from_prepared_cuda_batch(
    mut works: Vec<TraceOutputOpeningUnitWork<'_>>,
    mut timing: Option<&mut WitnessProofArtifactTiming>,
    units: &mut Vec<WitnessOpeningUnitSegment>,
) -> Result<(), ProveWitnessOpeningSegmentError> {
    if works.is_empty() {
        return Ok(());
    }

    let request_count = works
        .iter()
        .try_fold(0_usize, |sum, work| {
            sum.checked_add(work.stage_entries.len())
        })
        .ok_or(ProveWitnessOpeningSegmentError::StageOpening {
            unit_index: 0,
            trace_instance_index: 0,
            stage_index: 0,
            source_kind: "embedded",
            source: WitnessStageOpeningError::LengthOverflow,
        })?;
    let mut requests = Vec::with_capacity(request_count);
    let mut positions = Vec::with_capacity(request_count);
    for (work_index, work) in works.iter().enumerate() {
        for (stage_index, entry) in work.stage_entries.iter().enumerate() {
            requests.push(WitnessStageOpeningBatchRequest {
                commitment: entry.commitment,
                row_indices: &work.query_unit.queries,
                row_count: work.unit.extended_domain_size,
                column_count: entry.width,
                source_device: work.stage_source_views[stage_index].as_ref(),
            });
            positions.push((work_index, stage_index));
        }
    }
    let mut opening_work_timings = vec![WitnessStageOpeningWorkTiming::default(); requests.len()];
    let stage_opening_groups = open_witness_stage_commitment_batches_with_source_devices_timing(
        &requests,
        &mut opening_work_timings,
    )
    .map_err(|source| {
        let (unit_index, trace_instance_index, stage_index, source_kind) = positions
            .first()
            .map(|(work_index, stage_entry_index)| {
                let work = &works[*work_index];
                let entry = &work.stage_entries[*stage_entry_index];
                (
                    work.commitments.unit_index(),
                    work.commitments.trace_instance_index(),
                    entry.stage_index,
                    entry.source_kind,
                )
            })
            .unwrap_or((0, 0, 0, WitnessOpeningSourceKind::Embedded));
        ProveWitnessOpeningSegmentError::StageOpening {
            unit_index,
            trace_instance_index,
            stage_index,
            source_kind: source_kind.as_str(),
            source,
        }
    })?;
    if stage_opening_groups.len() != positions.len() {
        return Err(ProveWitnessOpeningSegmentError::StageOpening {
            unit_index: 0,
            trace_instance_index: 0,
            stage_index: 0,
            source_kind: "embedded",
            source: WitnessStageOpeningError::LengthOverflow,
        });
    }

    for (((work_index, stage_entry_index), opening_work_timing), openings) in positions
        .into_iter()
        .zip(opening_work_timings)
        .zip(stage_opening_groups)
    {
        let work = &mut works[work_index];
        let (stage_index, source_kind, stage_opening_start) = {
            let entry = &work.stage_entries[stage_entry_index];
            (
                entry.stage_index,
                entry.source_kind,
                entry.stage_opening_start,
            )
        };
        if openings.len() != work.query_stages.len() {
            return Err(ProveWitnessOpeningSegmentError::StageOpening {
                unit_index: work.commitments.unit_index(),
                trace_instance_index: work.commitments.trace_instance_index(),
                stage_index,
                source_kind: source_kind.as_str(),
                source: WitnessStageOpeningError::LengthOverflow,
            });
        }
        if let Some(timing) = timing.as_deref_mut() {
            timing.add_witness_stage_opening_setup(stage_index, opening_work_timing.setup);
            timing.add_witness_stage_opening_leaf_extend(
                stage_index,
                opening_work_timing.leaf_extend,
            );
            timing.add_witness_stage_opening_leaf_hash(stage_index, &opening_work_timing);
            timing.add_witness_stage_opening_path(stage_index, opening_work_timing.path);
            timing
                .add_witness_stage_opening_row_values(stage_index, opening_work_timing.row_values);
            timing.add_witness_stage_opening(stage_index, stage_opening_start.elapsed());
        }
        for (stages, opening) in work.query_stages.iter_mut().zip(openings.iter()) {
            stages.push(witness_opening_stage_segment(
                stage_index,
                work.unit.stage_commit_widths.len(),
                opening,
            )?);
        }
    }

    for work in works {
        let queries = work
            .query_unit
            .queries
            .iter()
            .copied()
            .zip(work.query_stages)
            .map(|(row_index, stages)| WitnessOpeningQuerySegment { row_index, stages })
            .collect();
        units.push(WitnessOpeningUnitSegment {
            unit_index: work.query_unit.unit_index,
            trace_instance_index: work.query_unit.trace_instance_index,
            queries,
        });
    }
    Ok(())
}

#[cfg(feature = "cuda")]
fn prepare_trace_output_opening_unit_work<'a>(
    schedule: &'a ProveSchedule,
    query_unit: &'a PcsQueryPlanUnit,
    output: &'a ProveWitnessTraceCommitments,
    mut timing: Option<&mut WitnessProofArtifactTiming>,
) -> Result<TraceOutputOpeningUnitWork<'a>, ProveWitnessOpeningSegmentError> {
    let commitments = output.commitments();
    let unit_index_u32 = u32::try_from(commitments.unit_index()).map_err(|_| {
        ProveWitnessOpeningSegmentError::UnitIndexOverflow {
            unit_index: commitments.unit_index(),
        }
    })?;
    if query_unit.unit_index != unit_index_u32 {
        return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: commitments.unit_index(),
        });
    }
    if query_unit.trace_instance_index != commitments.trace_instance_index() {
        return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: commitments.unit_index(),
        });
    }
    let unit = schedule.units.get(commitments.unit_index()).ok_or(
        ProveWitnessOpeningSegmentError::UnitIndexOutOfRange {
            unit_index: commitments.unit_index(),
            unit_count: schedule.units.len(),
        },
    )?;
    let mut guest_pc_external_stage_sources = None;
    if let Some(timing) = timing.as_deref_mut() {
        timing.add_witness_opening_queries(query_unit.queries.len());
    }
    let stage_count = commitments.stage_commitments().stage_count();
    let query_stages = (0..query_unit.queries.len())
        .map(|_| Vec::with_capacity(stage_count))
        .collect::<Vec<_>>();
    let mut stage_entries = Vec::with_capacity(stage_count);
    let mut stage_source_views = Vec::with_capacity(stage_count);
    for commitment in commitments.stage_commitments().commitments() {
        let stage_index = commitment.stage_index();
        let width = witness_stage_width(unit, stage_index)?;
        let stage_opening_start = Instant::now();
        let retained_source_view = output.stage_source_device_view(stage_index).cloned();
        let external_source_view =
            if retained_source_view.is_none() && commitment.requires_external_source() {
                let external_source_start = Instant::now();
                let mut external_source_timing = GuestPcDeviceSourceBuildTiming::default();
                let collect_external_source_timing = timing.is_some();
                let source_view = ensure_guest_pc_external_stage_sources(
                    &mut guest_pc_external_stage_sources,
                    unit,
                    output,
                    collect_external_source_timing.then_some(&mut external_source_timing),
                )?
                .and_then(|source_devices| {
                    stage_source_device_view_for_stage(source_devices, stage_index)
                });
                if let Some(timing) = timing.as_deref_mut() {
                    let duration = external_source_start.elapsed();
                    timing.add_witness_external_source(duration);
                    timing.add_witness_stage_external_source(stage_index, duration);
                    timing.add_witness_external_source_build_timing(&external_source_timing);
                }
                source_view
            } else {
                None
            };
        let source_kind = if retained_source_view.is_some() {
            WitnessOpeningSourceKind::Retained
        } else if external_source_view.is_some() {
            WitnessOpeningSourceKind::External
        } else if commitment.requires_external_source() {
            WitnessOpeningSourceKind::Missing
        } else {
            WitnessOpeningSourceKind::Embedded
        };
        let has_source_view = retained_source_view.is_some() || external_source_view.is_some();
        if let Some(timing) = timing.as_deref_mut() {
            timing.add_witness_stage_opening_source(stage_index, source_kind);
        }
        if commitment.requires_external_source() && !has_source_view {
            let provider_stage_count = guest_pc_external_stage_sources.as_ref().map_or(0, Vec::len);
            return Err(ProveWitnessOpeningSegmentError::ExternalSource {
                message: format!(
                    "missing provider for unit {} trace {} stage {stage_index}; material={}, provider_stages={provider_stage_count}",
                    commitments.unit_index(),
                    commitments.trace_instance_index(),
                    output.guest_pc_device_segment_material().is_some()
                ),
            });
        }
        stage_entries.push(TraceOutputOpeningStageWork {
            commitment,
            stage_index,
            width,
            source_kind,
            stage_opening_start,
        });
        stage_source_views.push(retained_source_view.or(external_source_view));
    }

    Ok(TraceOutputOpeningUnitWork {
        query_unit,
        unit,
        commitments,
        query_stages,
        stage_entries,
        stage_source_views,
    })
}

fn build_witness_opening_unit_segment(
    schedule: &ProveSchedule,
    query_unit: &PcsQueryPlanUnit,
    output: &ProveWitnessCommitments,
) -> Result<WitnessOpeningUnitSegment, ProveWitnessOpeningSegmentError> {
    let unit_index_u32 = u32::try_from(output.unit_index()).map_err(|_| {
        ProveWitnessOpeningSegmentError::UnitIndexOverflow {
            unit_index: output.unit_index(),
        }
    })?;
    if query_unit.unit_index != unit_index_u32 {
        return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: output.unit_index(),
        });
    }
    if query_unit.trace_instance_index != output.trace_instance_index() {
        return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: output.unit_index(),
        });
    }
    let unit = schedule.units.get(output.unit_index()).ok_or(
        ProveWitnessOpeningSegmentError::UnitIndexOutOfRange {
            unit_index: output.unit_index(),
            unit_count: schedule.units.len(),
        },
    )?;
    let stage_count = output.stage_commitments().stage_count();
    let mut query_stages = (0..query_unit.queries.len())
        .map(|_| Vec::with_capacity(stage_count))
        .collect::<Vec<_>>();
    for commitment in output.stage_commitments().commitments() {
        let stage_index = commitment.stage_index();
        let width = witness_stage_width(unit, stage_index)?;
        let openings = open_witness_stage_commitments(
            commitment,
            &query_unit.queries,
            unit.extended_domain_size,
            width,
        )
        .map_err(|source| ProveWitnessOpeningSegmentError::StageOpening {
            unit_index: output.unit_index(),
            trace_instance_index: output.trace_instance_index(),
            stage_index,
            source_kind: "host",
            source,
        })?;
        for (stages, opening) in query_stages.iter_mut().zip(openings.iter()) {
            stages.push(witness_opening_stage_segment(
                stage_index,
                unit.stage_commit_widths.len(),
                opening,
            )?);
        }
    }

    let queries = query_unit
        .queries
        .iter()
        .copied()
        .zip(query_stages)
        .map(|(row_index, stages)| WitnessOpeningQuerySegment { row_index, stages })
        .collect();

    Ok(WitnessOpeningUnitSegment {
        unit_index: unit_index_u32,
        trace_instance_index: query_unit.trace_instance_index,
        queries,
    })
}

#[cfg(not(feature = "cuda"))]
fn build_witness_opening_unit_segment_from_trace_output(
    schedule: &ProveSchedule,
    query_unit: &PcsQueryPlanUnit,
    output: &ProveWitnessTraceCommitments,
    mut timing: Option<&mut WitnessProofArtifactTiming>,
) -> Result<WitnessOpeningUnitSegment, ProveWitnessOpeningSegmentError> {
    let commitments = output.commitments();
    let unit_index_u32 = u32::try_from(commitments.unit_index()).map_err(|_| {
        ProveWitnessOpeningSegmentError::UnitIndexOverflow {
            unit_index: commitments.unit_index(),
        }
    })?;
    if query_unit.unit_index != unit_index_u32 {
        return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: commitments.unit_index(),
        });
    }
    if query_unit.trace_instance_index != commitments.trace_instance_index() {
        return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: commitments.unit_index(),
        });
    }
    let unit = schedule.units.get(commitments.unit_index()).ok_or(
        ProveWitnessOpeningSegmentError::UnitIndexOutOfRange {
            unit_index: commitments.unit_index(),
            unit_count: schedule.units.len(),
        },
    )?;
    #[cfg(feature = "cuda")]
    let mut guest_pc_external_stage_sources = None;
    if let Some(timing) = timing.as_deref_mut() {
        timing.add_witness_opening_queries(query_unit.queries.len());
    }
    let stage_count = commitments.stage_commitments().stage_count();
    let mut query_stages = (0..query_unit.queries.len())
        .map(|_| Vec::with_capacity(stage_count))
        .collect::<Vec<_>>();
    #[cfg(feature = "cuda")]
    {
        let mut stage_entries = Vec::with_capacity(stage_count);
        let mut stage_source_views = Vec::with_capacity(stage_count);
        for commitment in commitments.stage_commitments().commitments() {
            let stage_index = commitment.stage_index();
            let width = witness_stage_width(unit, stage_index)?;
            let stage_opening_start = Instant::now();
            let retained_source_view = output.stage_source_device_view(stage_index).cloned();
            let external_source_view =
                if retained_source_view.is_none() && commitment.requires_external_source() {
                    let external_source_start = Instant::now();
                    let mut external_source_timing = GuestPcDeviceSourceBuildTiming::default();
                    let collect_external_source_timing = timing.is_some();
                    let source_view = ensure_guest_pc_external_stage_sources(
                        &mut guest_pc_external_stage_sources,
                        unit,
                        output,
                        collect_external_source_timing.then_some(&mut external_source_timing),
                    )?
                    .and_then(|source_devices| {
                        stage_source_device_view_for_stage(source_devices, stage_index)
                    });
                    if let Some(timing) = timing.as_deref_mut() {
                        let duration = external_source_start.elapsed();
                        timing.add_witness_external_source(duration);
                        timing.add_witness_stage_external_source(stage_index, duration);
                        timing.add_witness_external_source_build_timing(&external_source_timing);
                    }
                    source_view
                } else {
                    None
                };
            let source_kind = if retained_source_view.is_some() {
                WitnessOpeningSourceKind::Retained
            } else if external_source_view.is_some() {
                WitnessOpeningSourceKind::External
            } else if commitment.requires_external_source() {
                WitnessOpeningSourceKind::Missing
            } else {
                WitnessOpeningSourceKind::Embedded
            };
            let has_source_view = retained_source_view.is_some() || external_source_view.is_some();
            if let Some(timing) = timing.as_deref_mut() {
                timing.add_witness_stage_opening_source(stage_index, source_kind);
            }
            if commitment.requires_external_source() && !has_source_view {
                let provider_stage_count =
                    guest_pc_external_stage_sources.as_ref().map_or(0, Vec::len);
                return Err(ProveWitnessOpeningSegmentError::ExternalSource {
                    message: format!(
                        "missing provider for unit {} trace {} stage {stage_index}; material={}, provider_stages={provider_stage_count}",
                        commitments.unit_index(),
                        commitments.trace_instance_index(),
                        output.guest_pc_device_segment_material().is_some()
                    ),
                });
            }
            stage_entries.push((
                commitment,
                stage_index,
                width,
                source_kind,
                stage_opening_start,
            ));
            stage_source_views.push(retained_source_view.or(external_source_view));
        }

        let stage_requests = stage_entries
            .iter()
            .zip(stage_source_views.iter())
            .map(
                |((commitment, _stage_index, width, _source_kind, _start), source_view)| {
                    WitnessStageOpeningBatchRequest {
                        commitment,
                        row_indices: &query_unit.queries,
                        row_count: unit.extended_domain_size,
                        column_count: *width,
                        source_device: source_view.as_ref(),
                    }
                },
            )
            .collect::<Vec<_>>();
        let mut opening_work_timings =
            vec![WitnessStageOpeningWorkTiming::default(); stage_requests.len()];
        let stage_opening_groups =
            open_witness_stage_commitment_batches_with_source_devices_timing(
                &stage_requests,
                &mut opening_work_timings,
            )
            .map_err(|source| {
                let (stage_index, source_kind) = stage_entries
                    .first()
                    .map(|(_, stage_index, _, source_kind, _)| (*stage_index, *source_kind))
                    .unwrap_or((0, WitnessOpeningSourceKind::Embedded));
                ProveWitnessOpeningSegmentError::StageOpening {
                    unit_index: commitments.unit_index(),
                    trace_instance_index: commitments.trace_instance_index(),
                    stage_index,
                    source_kind: source_kind.as_str(),
                    source,
                }
            })?;
        if stage_opening_groups.len() != stage_entries.len() {
            let (stage_index, source_kind) = stage_entries
                .first()
                .map(|(_, stage_index, _, source_kind, _)| (*stage_index, *source_kind))
                .unwrap_or((0, WitnessOpeningSourceKind::Embedded));
            return Err(ProveWitnessOpeningSegmentError::StageOpening {
                unit_index: commitments.unit_index(),
                trace_instance_index: commitments.trace_instance_index(),
                stage_index,
                source_kind: source_kind.as_str(),
                source: WitnessStageOpeningError::LengthOverflow,
            });
        }
        for ((stage_entry, opening_work_timing), openings) in stage_entries
            .into_iter()
            .zip(opening_work_timings)
            .zip(stage_opening_groups)
        {
            let (_commitment, stage_index, _width, source_kind, stage_opening_start) = stage_entry;
            if openings.len() != query_stages.len() {
                return Err(ProveWitnessOpeningSegmentError::StageOpening {
                    unit_index: commitments.unit_index(),
                    trace_instance_index: commitments.trace_instance_index(),
                    stage_index,
                    source_kind: source_kind.as_str(),
                    source: WitnessStageOpeningError::LengthOverflow,
                });
            }
            if let Some(timing) = timing.as_deref_mut() {
                timing.add_witness_stage_opening_setup(stage_index, opening_work_timing.setup);
                timing.add_witness_stage_opening_leaf_extend(
                    stage_index,
                    opening_work_timing.leaf_extend,
                );
                timing.add_witness_stage_opening_leaf_hash(stage_index, &opening_work_timing);
                timing.add_witness_stage_opening_path(stage_index, opening_work_timing.path);
                timing.add_witness_stage_opening_row_values(
                    stage_index,
                    opening_work_timing.row_values,
                );
                timing.add_witness_stage_opening(stage_index, stage_opening_start.elapsed());
            }
            for (stages, opening) in query_stages.iter_mut().zip(openings.iter()) {
                stages.push(witness_opening_stage_segment(
                    stage_index,
                    unit.stage_commit_widths.len(),
                    opening,
                )?);
            }
        }
    }
    #[cfg(not(feature = "cuda"))]
    for commitment in commitments.stage_commitments().commitments() {
        let stage_index = commitment.stage_index();
        let width = witness_stage_width(unit, stage_index)?;
        let openings = {
            let stage_opening_start = Instant::now();
            let openings = open_witness_stage_commitments(
                commitment,
                &query_unit.queries,
                unit.extended_domain_size,
                width,
            )
            .map_err(|source| ProveWitnessOpeningSegmentError::StageOpening {
                unit_index: commitments.unit_index(),
                trace_instance_index: commitments.trace_instance_index(),
                stage_index,
                source_kind: "host",
                source,
            })?;
            if let Some(timing) = timing.as_deref_mut() {
                timing.add_witness_stage_opening(stage_index, stage_opening_start.elapsed());
            }
            openings
        };
        for (stages, opening) in query_stages.iter_mut().zip(openings.iter()) {
            stages.push(witness_opening_stage_segment(
                stage_index,
                unit.stage_commit_widths.len(),
                opening,
            )?);
        }
    }

    let queries = query_unit
        .queries
        .iter()
        .copied()
        .zip(query_stages)
        .map(|(row_index, stages)| WitnessOpeningQuerySegment { row_index, stages })
        .collect();

    Ok(WitnessOpeningUnitSegment {
        unit_index: unit_index_u32,
        trace_instance_index: query_unit.trace_instance_index,
        queries,
    })
}

#[cfg(feature = "cuda")]
fn stage_source_device_view_for_stage(
    source_devices: &[WitnessStageSourceDevice],
    stage_index: usize,
) -> Option<WitnessStageSourceDeviceView> {
    if let Some(stage_slot) = stage_index.checked_sub(1) {
        if let Some(source_device) = source_devices
            .get(stage_slot)
            .filter(|source_device| source_device.stage_index() == stage_index)
        {
            if source_devices[..stage_slot]
                .iter()
                .all(|source_device| source_device.stage_index() != stage_index)
            {
                return Some(source_device.source_view());
            }
        }
    }

    source_devices
        .iter()
        .find(|source_device| source_device.stage_index() == stage_index)
        .map(WitnessStageSourceDevice::source_view)
}

#[cfg(feature = "cuda")]
fn ensure_guest_pc_external_stage_sources<'a>(
    cached_sources: &'a mut Option<Vec<WitnessStageSourceDevice>>,
    unit: &ProveUnitSchedule,
    output: &ProveWitnessTraceCommitments,
    timing: Option<&mut GuestPcDeviceSourceBuildTiming>,
) -> Result<Option<&'a [WitnessStageSourceDevice]>, ProveWitnessOpeningSegmentError> {
    if output.guest_pc_device_segment_material().is_none() {
        return Ok(None);
    }
    if cached_sources.is_none() {
        *cached_sources = guest_pc_external_stage_sources(unit, output, timing)?;
    }
    Ok(cached_sources.as_deref())
}

#[cfg(feature = "cuda")]
fn guest_pc_external_stage_sources(
    unit: &ProveUnitSchedule,
    output: &ProveWitnessTraceCommitments,
    mut timing: Option<&mut GuestPcDeviceSourceBuildTiming>,
) -> Result<Option<Vec<WitnessStageSourceDevice>>, ProveWitnessOpeningSegmentError> {
    let Some(material) = output.guest_pc_device_segment_material_arc() else {
        return Ok(None);
    };
    let layout = derive_witness_trace_layout(unit).map_err(|error| {
        ProveWitnessOpeningSegmentError::ExternalSource {
            message: error.to_string(),
        }
    })?;
    if let Some(source) = build_guest_pc_trace_descriptor_source_from_device_material_timing(
        &layout,
        Arc::clone(&material),
        output.guest_pc_device_descriptor_buffer_arc(),
        timing.as_deref_mut(),
    )
    .map_err(|error| ProveWitnessOpeningSegmentError::ExternalSource {
        message: error.to_string(),
    })? {
        return Ok(Some(
            source
                .stages()
                .iter()
                .map(|stage| {
                    WitnessStageSourceDevice::from_main_trace_compact_descriptors(
                        stage.stage_index(),
                        source.row_count(),
                        stage.column_count(),
                        source.row_stride(),
                        stage.column_offset(),
                        stage.is_known_zero(),
                        source.descriptor_count(),
                        source.terminal_pc(),
                        source.layout(),
                        source.pending_upload(),
                        source.material(),
                        source.descriptors(),
                    )
                })
                .collect(),
        ));
    }
    let builder = if let Some(descriptor_buffer) = output.guest_pc_device_descriptor_buffer() {
        build_guest_pc_trace_stage_source_devices_from_device_descriptors_timing(
            &layout,
            material.as_ref(),
            descriptor_buffer,
            timing,
        )
    } else {
        build_guest_pc_trace_stage_source_devices_from_device_material_timing(
            &layout,
            material.as_ref(),
            timing,
        )
    }
    .map_err(|error| ProveWitnessOpeningSegmentError::ExternalSource {
        message: error.to_string(),
    })?;
    let trace = builder.trace();
    Ok(Some(
        builder
            .stages()
            .iter()
            .map(|stage| {
                WitnessStageSourceDevice::from_row_major_column_window_with_known_zero(
                    stage.stage_index(),
                    stage.row_count(),
                    stage.column_count(),
                    stage.row_stride(),
                    stage.column_offset(),
                    stage.is_known_zero(),
                    trace,
                )
            })
            .collect(),
    ))
}

fn expected_merkle_level_count(
    row_count: u64,
    arity: usize,
) -> Result<usize, ValidateWitnessOpeningSegmentsError> {
    if row_count == 0 || arity < 2 {
        return Err(ValidateWitnessOpeningSegmentsError::InvalidTreeShape);
    }
    let arity =
        u64::try_from(arity).map_err(|_| ValidateWitnessOpeningSegmentsError::ArityOverflow)?;
    let mut levels = 0_usize;
    let mut rows = row_count;
    while rows > 1 {
        rows = rows.div_ceil(arity);
        levels = levels
            .checked_add(1)
            .ok_or(ValidateWitnessOpeningSegmentsError::LevelCountOverflow)?;
    }
    Ok(levels)
}

fn field_digest_from_words(
    words: [u64; 4],
) -> Result<[Felt; 4], ValidateWitnessOpeningSegmentsError> {
    let mut out = [Felt::ZERO; 4];
    for (target, value) in out.iter_mut().zip(words) {
        *target = Felt::from_canonical(value)
            .map_err(ValidateWitnessOpeningSegmentsError::FieldDigest)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_units_match_query_units_rejects_duplicate_in_memory_identity() {
        let query_units = vec![query_unit(0, 1)];
        let opening = WitnessOpeningSegment {
            units: vec![opening_unit(0, 1), opening_unit(0, 1)],
        };

        let error =
            validate_witness_opening_units_match_query_units_from_segment(&query_units, &opening)
                .expect_err("duplicate witness opening identity should reject");

        assert_eq!(
            error,
            LoadWitnessOpeningUnitError::UnexpectedUnit { unit_index: 0 }
        );
    }

    #[test]
    fn opening_units_match_query_units_rejects_duplicate_query_identity() {
        let query_units = vec![query_unit(0, 1), query_unit(0, 1)];
        let opening = WitnessOpeningSegment {
            units: vec![opening_unit(0, 1)],
        };

        let error =
            validate_witness_opening_units_match_query_units_from_segment(&query_units, &opening)
                .expect_err("duplicate query identity should reject");

        assert_eq!(
            error,
            LoadWitnessOpeningUnitError::UnexpectedUnit { unit_index: 0 }
        );
    }

    #[test]
    fn external_source_opening_batch_size_parser_uses_positive_values_only() {
        assert_eq!(DEFAULT_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE, 2);
        assert_eq!(
            parse_trace_output_external_source_opening_batch_size(None),
            DEFAULT_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE
        );
        assert_eq!(
            parse_trace_output_external_source_opening_batch_size(Some("0")),
            DEFAULT_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE
        );
        assert_eq!(
            parse_trace_output_external_source_opening_batch_size(Some("invalid")),
            DEFAULT_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE
        );
        assert_eq!(
            parse_trace_output_external_source_opening_batch_size(Some("12")),
            12
        );
    }

    fn query_unit(unit_index: u32, trace_instance_index: u32) -> PcsQueryPlanUnit {
        PcsQueryPlanUnit {
            unit_index,
            trace_instance_index,
            queries: vec![0],
        }
    }

    fn opening_unit(unit_index: u32, trace_instance_index: u32) -> WitnessOpeningUnitSegment {
        WitnessOpeningUnitSegment {
            unit_index,
            trace_instance_index,
            queries: Vec::new(),
        }
    }
}

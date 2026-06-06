use std::fmt;
use std::time::Instant;

use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanSegmentError, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, parse_witness_opening_segment, WitnessOpeningLevelSegment,
    WitnessOpeningQuerySegment, WitnessOpeningSegment, WitnessOpeningSegmentError,
    WitnessOpeningStageSegment, WitnessOpeningUnitSegment, WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, witness_commitment_segment_id, WitnessCommitmentSegmentError,
    WitnessCommitmentSegmentIdentity,
};
use lzvm_field::{Felt, FieldError};

#[cfg(feature = "cuda")]
use crate::guest_pc_trace_backend::{
    build_guest_pc_trace_stage_source_devices_from_device_descriptors,
    build_guest_pc_trace_stage_source_devices_from_device_material,
};
use crate::pcs_query_plan::{load_pcs_query_plan_from_segments, LoadPcsQueryPlanSegmentError};
use crate::proof_artifact_timing::WitnessProofArtifactTiming;
use crate::witness_commitment::{
    load_witness_commitment_segments, open_witness_stage_commitment,
    verify_witness_stage_opening_root, LoadWitnessCommitmentSegmentsError, WitnessStageOpening,
    WitnessStageOpeningError,
};
#[cfg(feature = "cuda")]
use crate::witness_commitment::{
    open_witness_stage_commitment_with_source_device_timing, WitnessStageOpeningWorkTiming,
    WitnessStageSourceDevice,
};
use crate::witness_execution::{ProveWitnessCommitments, ProveWitnessTraceCommitments};
#[cfg(feature = "cuda")]
use crate::witness_layout::derive_witness_trace_layout;
use crate::ProveSchedule;
use crate::ProveUnitSchedule;

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
    FieldValue(FieldError),
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
            Self::FieldValue(error) => write!(f, "invalid witness opening segment value: {error}"),
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
            Self::FieldValue(error) | Self::FieldDigest(error) => Some(error),
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
    let opening = load_witness_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadWitnessOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| unit.unit_index == unit_index_u32 && unit.trace_instance_index == 0)
        .ok_or(LoadWitnessOpeningUnitError::MissingUnit { unit_index })
}

pub(crate) fn validate_witness_opening_units_match_query_units(
    query_units: &[PcsQueryPlanUnit],
    segments: &[ProofSegment],
) -> Result<(), LoadWitnessOpeningUnitError> {
    let opening = load_witness_opening_segment_from_segments(segments)?;
    for unit in opening.units {
        if !query_units.iter().any(|query_unit| {
            query_unit.unit_index == unit.unit_index
                && query_unit.trace_instance_index == unit.trace_instance_index
        }) {
            let unit_index = usize::try_from(unit.unit_index)
                .map_err(|_| LoadWitnessOpeningUnitError::UnitIndexOverflow)?;
            return Err(LoadWitnessOpeningUnitError::UnexpectedUnit { unit_index });
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

    let witness_segments = load_witness_commitment_segments(units, segments)
        .map_err(ValidateWitnessOpeningSegmentsError::Commitments)?;
    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidateWitnessOpeningSegmentsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        let opening_unit = opening
            .units
            .iter()
            .find(|unit| {
                unit.unit_index == query_unit.unit_index
                    && unit.trace_instance_index == query_unit.trace_instance_index
            })
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        if opening_unit.queries.len() != query_unit.queries.len() {
            return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
        }

        let unit_count = u32::try_from(units.len())
            .map_err(|_| ValidateWitnessOpeningSegmentsError::SegmentIdOverflow)?;
        let witness_segment_id = witness_commitment_segment_id(
            unit_count,
            WitnessCommitmentSegmentIdentity {
                unit_index: query_unit.unit_index,
                trace_instance_index: query_unit.trace_instance_index,
            },
        )
        .map_err(|_| ValidateWitnessOpeningSegmentsError::SegmentIdOverflow)?;
        let witness_segment = witness_segments
            .iter()
            .find(|segment| segment.id == witness_segment_id)
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        let witness =
            parse_witness_commitment_segment(&witness_segment.data).map_err(|source| {
                ValidateWitnessOpeningSegmentsError::CommitmentSegment { unit_index, source }
            })?;
        let arity = usize::try_from(unit.merkle_tree_arity)
            .map_err(|_| ValidateWitnessOpeningSegmentsError::ArityOverflow)?;
        let expected_level_count = expected_merkle_level_count(unit.extended_domain_size, arity)?;

        for (query, expected_row) in opening_unit.queries.iter().zip(query_unit.queries.iter()) {
            if query.row_index != *expected_row || query.stages.len() != witness.stages.len() {
                return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
            }
            for stage in &query.stages {
                let stage_index = usize::try_from(stage.stage_index)
                    .map_err(|_| ValidateWitnessOpeningSegmentsError::StageIndexOverflow)?;
                let Some(width) = stage_index
                    .checked_sub(1)
                    .and_then(|index| unit.stage_commit_widths.get(index))
                else {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                };
                if stage.values.len() != *width as usize
                    || stage.siblings.len() != expected_level_count
                {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                }
                let Some(witness_stage) = witness
                    .stages
                    .iter()
                    .find(|witness_stage| witness_stage.stage_index == stage.stage_index)
                else {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                };
                let values = stage
                    .values
                    .iter()
                    .map(|value| Felt::from_canonical(*value))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ValidateWitnessOpeningSegmentsError::FieldValue)?;
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
    mut timing: Option<&mut WitnessProofArtifactTiming>,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let commitments = outputs
        .iter()
        .map(|output| output.commitments())
        .collect::<Vec<_>>();
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
    for output in &commitments {
        let unit_index_u32 = u32::try_from(output.unit_index()).map_err(|_| {
            ProveWitnessOpeningSegmentError::UnitIndexOverflow {
                unit_index: output.unit_index(),
            }
        })?;
        if !query_units.contains(&(unit_index_u32, output.trace_instance_index())) {
            return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
                unit_index: unit_index_u32 as usize,
            });
        }
    }

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
    let mut queries = Vec::with_capacity(query_unit.queries.len());
    for row_index in &query_unit.queries {
        let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
        for commitment in output.stage_commitments().commitments() {
            let stage_index = commitment.stage_index();
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
            let opening = open_witness_stage_commitment(
                commitment,
                *row_index,
                unit.extended_domain_size,
                usize::try_from(*width).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    }
                })?,
            )?;
            stages.push(WitnessOpeningStageSegment {
                stage_index: u32::try_from(stage_index).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
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
            });
        }
        queries.push(WitnessOpeningQuerySegment {
            row_index: *row_index,
            stages,
        });
    }

    Ok(WitnessOpeningUnitSegment {
        unit_index: unit_index_u32,
        trace_instance_index: query_unit.trace_instance_index,
        queries,
    })
}

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
    let mut queries = Vec::with_capacity(query_unit.queries.len());
    for row_index in &query_unit.queries {
        let mut stages = Vec::with_capacity(commitments.stage_commitments().stage_count());
        for commitment in commitments.stage_commitments().commitments() {
            let stage_index = commitment.stage_index();
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
            #[cfg(feature = "cuda")]
            let opening = {
                let width = usize::try_from(*width).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    }
                })?;
                let stage_opening_start = Instant::now();
                let retained_source_view = output.stage_source_device_view(stage_index);
                let external_source_view =
                    if retained_source_view.is_none() && commitment.requires_external_source() {
                        let external_source_start = Instant::now();
                        let source_view = ensure_guest_pc_external_stage_sources(
                            &mut guest_pc_external_stage_sources,
                            unit,
                            output,
                        )?
                        .and_then(|source_devices| {
                            source_devices
                                .iter()
                                .find(|source_device| source_device.stage_index() == stage_index)
                                .map(|source_device| source_device.source_view())
                        });
                        if let Some(timing) = timing.as_deref_mut() {
                            let duration = external_source_start.elapsed();
                            timing.add_witness_external_source(duration);
                            timing.add_witness_stage_external_source(stage_index, duration);
                        }
                        source_view
                    } else {
                        None
                    };
                let source_view = retained_source_view.or(external_source_view.as_ref());
                if commitment.requires_external_source() && source_view.is_none() {
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
                let mut opening_work_timing = WitnessStageOpeningWorkTiming::default();
                let opening = open_witness_stage_commitment_with_source_device_timing(
                    commitment,
                    *row_index,
                    unit.extended_domain_size,
                    width,
                    source_view,
                    &mut opening_work_timing,
                )?;
                if let Some(timing) = timing.as_deref_mut() {
                    timing.add_witness_stage_opening_setup(stage_index, opening_work_timing.setup);
                    timing.add_witness_stage_opening_leaf_extend(
                        stage_index,
                        opening_work_timing.leaf_extend,
                    );
                    timing.add_witness_stage_opening_leaf_hash(
                        stage_index,
                        opening_work_timing.leaf_hash,
                        opening_work_timing.leaf_hash_rows,
                        opening_work_timing.leaf_hash_bytes,
                    );
                    timing.add_witness_stage_opening_path(stage_index, opening_work_timing.path);
                    timing.add_witness_stage_opening_row_values(
                        stage_index,
                        opening_work_timing.row_values,
                    );
                    timing.add_witness_stage_opening(stage_index, stage_opening_start.elapsed());
                }
                opening
            };
            #[cfg(not(feature = "cuda"))]
            let opening = {
                let stage_opening_start = Instant::now();
                let opening = open_witness_stage_commitment(
                    commitment,
                    *row_index,
                    unit.extended_domain_size,
                    usize::try_from(*width).map_err(|_| {
                        ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                            stage_index,
                            stage_count: unit.stage_commit_widths.len(),
                        }
                    })?,
                )?;
                if let Some(timing) = timing.as_deref_mut() {
                    timing.add_witness_stage_opening(stage_index, stage_opening_start.elapsed());
                }
                opening
            };
            stages.push(WitnessOpeningStageSegment {
                stage_index: u32::try_from(stage_index).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
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
            });
        }
        queries.push(WitnessOpeningQuerySegment {
            row_index: *row_index,
            stages,
        });
    }

    Ok(WitnessOpeningUnitSegment {
        unit_index: unit_index_u32,
        trace_instance_index: query_unit.trace_instance_index,
        queries,
    })
}

#[cfg(feature = "cuda")]
fn ensure_guest_pc_external_stage_sources<'a>(
    cached_sources: &'a mut Option<Vec<WitnessStageSourceDevice>>,
    unit: &ProveUnitSchedule,
    output: &ProveWitnessTraceCommitments,
) -> Result<Option<&'a [WitnessStageSourceDevice]>, ProveWitnessOpeningSegmentError> {
    if output.guest_pc_device_segment_material().is_none() {
        return Ok(None);
    }
    if cached_sources.is_none() {
        *cached_sources = guest_pc_external_stage_sources(unit, output)?;
    }
    Ok(cached_sources.as_deref())
}

#[cfg(feature = "cuda")]
fn guest_pc_external_stage_sources(
    unit: &ProveUnitSchedule,
    output: &ProveWitnessTraceCommitments,
) -> Result<Option<Vec<WitnessStageSourceDevice>>, ProveWitnessOpeningSegmentError> {
    let Some(material) = output.guest_pc_device_segment_material() else {
        return Ok(None);
    };
    let layout = derive_witness_trace_layout(unit).map_err(|error| {
        ProveWitnessOpeningSegmentError::ExternalSource {
            message: error.to_string(),
        }
    })?;
    let builder = if let Some(descriptor_buffer) = output.guest_pc_device_descriptor_buffer() {
        build_guest_pc_trace_stage_source_devices_from_device_descriptors(
            &layout,
            material,
            descriptor_buffer,
        )
    } else {
        build_guest_pc_trace_stage_source_devices_from_device_material(&layout, material)
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
                WitnessStageSourceDevice::from_row_major_column_window(
                    stage.stage_index(),
                    stage.row_count(),
                    stage.column_count(),
                    stage.row_stride(),
                    stage.column_offset(),
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

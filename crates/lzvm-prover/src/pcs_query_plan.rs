use std::fmt;

mod build;
mod errors;

pub use build::{
    build_pcs_query_nonce_segment, build_pcs_query_nonce_segment_from_transcript_segments,
    build_pcs_query_nonce_segment_with_streams, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_challenge,
    build_pcs_query_plan_segment_from_transcript_segments,
    build_pcs_query_plan_segment_with_bindings,
};
pub use errors::ProvePcsQueryPlanSegmentError;

use lzvm_artifacts::challenge_values_segment::CHALLENGE_VALUES_SEGMENT_ID;
use lzvm_artifacts::eth_block_input_segment::ETH_BLOCK_INPUT_SEGMENT_ID;
use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PcsMaterialManifestSegment,
    PcsMaterialManifestSegmentError, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_nonce_segment::{
    parse_pcs_query_nonce_segment, PCS_QUERY_NONCE_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanSegmentError, PcsQueryPlanUnit,
    PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::program_image_segment::PROGRAM_IMAGE_CACHE_SEGMENT_ID;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, witness_commitment_segment_id, WitnessCommitmentSegmentError,
    WitnessCommitmentSegmentIdentity,
};
use lzvm_field::{Ext3, Felt};

use crate::pcs_evaluation::{
    load_pcs_evaluation_segment_from_segments,
    load_pcs_evaluation_unit_for_identity_from_parsed_segment,
    validate_pcs_evaluation_units_match_query_units_from_segment, LoadPcsEvaluationUnitError,
};
use crate::pcs_fri::{
    load_pcs_fri_opening_segment_from_segments,
    validate_pcs_fri_opening_units_match_query_units_from_segment, LoadPcsFriOpeningUnitError,
};
use crate::pcs_transcript::{
    aggregate_pcs_final_query_challenges, derive_pcs_final_query_challenge_from_segments,
    PcsTranscriptError, PcsTranscriptSegmentInputs,
};
use crate::unit_values::{
    load_unit_values_for_identity_from_parsed_segment, load_unit_values_segment_from_segments,
    validate_unit_values_units_match_query_units_from_segment, LoadUnitValuesSegmentError,
};
use crate::witness_commitment::{
    load_witness_commitment_segment_refs, LoadWitnessCommitmentSegmentsError,
};
use crate::ProveSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPcsQueryPlanSegmentError {
    MissingSegment,
    DuplicateSegment,
    UnsupportedTraceInstance {
        unit_index: u32,
        trace_instance_index: u32,
    },
    Segment(PcsQueryPlanSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatePcsQueryPlanSegmentsError {
    MissingMaterialSegment,
    DuplicateMaterialSegment,
    MissingNonceSegment,
    DuplicateNonceSegment,
    QueryPlan(LoadPcsQueryPlanSegmentError),
    Witness(LoadWitnessCommitmentSegmentsError),
    Material(PcsMaterialManifestSegmentError),
    WitnessSegment {
        unit_index: usize,
        source: WitnessCommitmentSegmentError,
    },
    Evaluation(LoadPcsEvaluationUnitError),
    Fri(LoadPcsFriOpeningUnitError),
    UnitValues(LoadUnitValuesSegmentError),
    Transcript(PcsTranscriptError),
    Build(ProvePcsQueryPlanSegmentError),
    DuplicateBindingSegment {
        id: u32,
    },
    QueryPlanMismatch,
    TranscriptUnitCountMismatch,
    WitnessSegmentIdOverflow,
    UnitIndexOverflow,
    UnitMismatch {
        unit_index: usize,
    },
}

impl fmt::Display for LoadPcsQueryPlanSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS query plan segment"),
            Self::DuplicateSegment => write!(f, "duplicate PCS query plan segment"),
            Self::UnsupportedTraceInstance {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "unsupported PCS query plan trace instance {trace_instance_index} for unit {unit_index}"
            ),
            Self::Segment(error) => write!(f, "invalid PCS query plan segment: {error}"),
        }
    }
}

impl fmt::Display for ValidatePcsQueryPlanSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMaterialSegment => write!(f, "missing PCS material manifest segment"),
            Self::DuplicateMaterialSegment => {
                write!(f, "duplicate PCS material manifest segment")
            }
            Self::MissingNonceSegment => write!(f, "missing PCS query nonce segment"),
            Self::DuplicateNonceSegment => write!(f, "duplicate PCS query nonce segment"),
            Self::QueryPlan(error) => write!(f, "{error}"),
            Self::Witness(error) => write!(f, "{error}"),
            Self::Material(error) => write!(f, "invalid PCS material manifest segment: {error}"),
            Self::WitnessSegment { unit_index, source } => write!(
                f,
                "invalid witness commitment segment for unit {unit_index}: {source}"
            ),
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Fri(error) => write!(f, "{error}"),
            Self::UnitValues(error) => write!(f, "{error}"),
            Self::Transcript(error) => {
                write!(f, "derive PCS query plan transcript failed: {error}")
            }
            Self::Build(error) => write!(f, "derive PCS query plan segment failed: {error}"),
            Self::DuplicateBindingSegment { id } => {
                write!(f, "duplicate proof binding segment id: {id}")
            }
            Self::QueryPlanMismatch => write!(f, "PCS query plan segment mismatch"),
            Self::TranscriptUnitCountMismatch => {
                write!(f, "PCS transcript query plan unit count mismatch")
            }
            Self::WitnessSegmentIdOverflow => {
                write!(f, "PCS transcript query plan witness segment id overflow")
            }
            Self::UnitIndexOverflow => write!(f, "PCS transcript query plan unit index overflow"),
            Self::UnitMismatch { unit_index } => {
                write!(
                    f,
                    "PCS transcript query plan mismatch for unit {unit_index}"
                )
            }
        }
    }
}

impl std::error::Error for LoadPcsQueryPlanSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment
            | Self::DuplicateSegment
            | Self::UnsupportedTraceInstance { .. } => None,
        }
    }
}

impl std::error::Error for ValidatePcsQueryPlanSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Witness(error) => Some(error),
            Self::Material(error) => Some(error),
            Self::WitnessSegment { source, .. } => Some(source),
            Self::Evaluation(error) => Some(error),
            Self::Fri(error) => Some(error),
            Self::UnitValues(error) => Some(error),
            Self::Transcript(error) => Some(error),
            Self::Build(error) => Some(error),
            Self::MissingMaterialSegment
            | Self::DuplicateMaterialSegment
            | Self::MissingNonceSegment
            | Self::DuplicateNonceSegment
            | Self::DuplicateBindingSegment { .. }
            | Self::QueryPlanMismatch
            | Self::TranscriptUnitCountMismatch
            | Self::WitnessSegmentIdOverflow
            | Self::UnitIndexOverflow
            | Self::UnitMismatch { .. } => None,
        }
    }
}

pub fn load_pcs_query_plan_from_segments(
    segments: &[ProofSegment],
) -> Result<PcsQueryPlanSegment, LoadPcsQueryPlanSegmentError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(LoadPcsQueryPlanSegmentError::MissingSegment)?;
    if matching_segments.next().is_some() {
        return Err(LoadPcsQueryPlanSegmentError::DuplicateSegment);
    }
    parse_pcs_query_plan_segment(&segment.data).map_err(LoadPcsQueryPlanSegmentError::Segment)
}

pub fn uses_transcript_pcs_query_plan_inputs(segments: &[ProofSegment]) -> bool {
    segments
        .iter()
        .any(|segment| segment.id == PCS_QUERY_NONCE_SEGMENT_ID)
        || segments
            .iter()
            .any(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
}

pub fn validate_pcs_query_plan_segments(
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    public_values: &[Felt],
    segments: &[ProofSegment],
) -> Result<(), ValidatePcsQueryPlanSegmentsError> {
    if uses_transcript_pcs_query_plan_inputs(segments) {
        validate_transcript_pcs_query_plan_segments(schedule, public_values, segments)
    } else {
        validate_seeded_pcs_query_plan_segments(schedule, public_values_hash, segments)
    }
}

pub fn validate_seeded_pcs_query_plan_segments(
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    segments: &[ProofSegment],
) -> Result<(), ValidatePcsQueryPlanSegmentsError> {
    let material_segment = single_material_manifest_segment(segments)?;
    load_pcs_query_plan_from_segments(segments)
        .map_err(ValidatePcsQueryPlanSegmentsError::QueryPlan)?;
    let query_segment = segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .ok_or(ValidatePcsQueryPlanSegmentsError::QueryPlan(
            LoadPcsQueryPlanSegmentError::MissingSegment,
        ))?;
    let witness_segments = load_witness_commitment_segment_refs(&schedule.units, segments)
        .map_err(ValidatePcsQueryPlanSegmentsError::Witness)?;
    let binding_segments = checked_proof_binding_segments(segments)
        .map_err(|id| ValidatePcsQueryPlanSegmentsError::DuplicateBindingSegment { id })?;
    let expected_segment = build::build_pcs_query_plan_segment_with_binding_refs(
        schedule,
        public_values_hash,
        material_segment,
        &witness_segments,
        &binding_segments,
    )
    .map_err(ValidatePcsQueryPlanSegmentsError::Build)?;
    if query_segment.data != expected_segment.data {
        return Err(ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
    }
    Ok(())
}

fn validate_transcript_query_plan_unit_inputs(
    schedule: &ProveSchedule,
    query_units: &[PcsQueryPlanUnit],
    material: &PcsMaterialManifestSegment,
    public_values: &[Felt],
    witness_segments: &[&ProofSegment],
    segments: &[ProofSegment],
) -> Result<Vec<Ext3>, ValidatePcsQueryPlanSegmentsError> {
    let binding_segments = checked_proof_binding_segments(segments)
        .map_err(|id| ValidatePcsQueryPlanSegmentsError::DuplicateBindingSegment { id })?;
    let evaluation_segment = load_pcs_evaluation_segment_from_segments(segments)
        .map_err(ValidatePcsQueryPlanSegmentsError::Evaluation)?;
    let fri_segment = load_pcs_fri_opening_segment_from_segments(segments)
        .map_err(|error| ValidatePcsQueryPlanSegmentsError::Fri(error.into()))?;
    let unit_values_segment = load_unit_values_segment_from_segments(segments)
        .map_err(ValidatePcsQueryPlanSegmentsError::UnitValues)?;
    let mut final_query_challenges = Vec::with_capacity(query_units.len());
    for query_unit in query_units {
        let unit_index_u32 = query_unit.unit_index;
        let unit_index = usize::try_from(unit_index_u32)
            .map_err(|_| ValidatePcsQueryPlanSegmentsError::UnitIndexOverflow)?;
        let unit = schedule
            .units
            .get(unit_index)
            .ok_or(ValidatePcsQueryPlanSegmentsError::UnitMismatch { unit_index })?;
        let material_unit = material
            .units
            .iter()
            .find(|unit| unit.unit_index == unit_index_u32)
            .ok_or(ValidatePcsQueryPlanSegmentsError::UnitMismatch { unit_index })?;
        let unit_count = u32::try_from(schedule.units.len())
            .map_err(|_| ValidatePcsQueryPlanSegmentsError::WitnessSegmentIdOverflow)?;
        let witness_segment_id = witness_commitment_segment_id(
            unit_count,
            WitnessCommitmentSegmentIdentity {
                unit_index: unit_index_u32,
                trace_instance_index: query_unit.trace_instance_index,
            },
        )
        .map_err(|_| ValidatePcsQueryPlanSegmentsError::WitnessSegmentIdOverflow)?;
        let witness_segment = witness_segments
            .iter()
            .find(|segment| segment.id == witness_segment_id)
            .ok_or(ValidatePcsQueryPlanSegmentsError::UnitMismatch { unit_index })?;
        let witness =
            parse_witness_commitment_segment(&witness_segment.data).map_err(|source| {
                ValidatePcsQueryPlanSegmentsError::WitnessSegment { unit_index, source }
            })?;
        let evaluations = load_pcs_evaluation_unit_for_identity_from_parsed_segment(
            unit_index,
            query_unit.trace_instance_index,
            unit,
            &evaluation_segment,
        )
        .map_err(ValidatePcsQueryPlanSegmentsError::Evaluation)?;
        let fri = fri_segment
            .units
            .iter()
            .find(|unit| {
                unit.unit_index == query_unit.unit_index
                    && unit.trace_instance_index == query_unit.trace_instance_index
            })
            .ok_or(ValidatePcsQueryPlanSegmentsError::UnitMismatch { unit_index })?;
        let unit_values = load_unit_values_for_identity_from_parsed_segment(
            unit_index,
            query_unit.trace_instance_index,
            &unit.unit_value_map,
            unit_values_segment.as_ref(),
        )
        .map_err(ValidatePcsQueryPlanSegmentsError::UnitValues)?;
        let final_query_challenge =
            derive_pcs_final_query_challenge_from_segments(PcsTranscriptSegmentInputs {
                unit_index,
                unit,
                material: material_unit,
                public_values,
                unit_values: &unit_values,
                witness: &witness,
                evaluations: &evaluations,
                fri: &fri,
                root_challenge_draws: &unit.transcript_root_challenge_draws,
                evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
                binding_segments: &binding_segments,
            })
            .map_err(ValidatePcsQueryPlanSegmentsError::Transcript)?;
        final_query_challenges.push(final_query_challenge);
    }
    validate_pcs_evaluation_units_match_query_units_from_segment(query_units, &evaluation_segment)
        .map_err(ValidatePcsQueryPlanSegmentsError::Evaluation)?;
    validate_pcs_fri_opening_units_match_query_units_from_segment(query_units, &fri_segment)
        .map_err(ValidatePcsQueryPlanSegmentsError::Fri)?;
    validate_unit_values_units_match_query_units_from_segment(
        query_units,
        unit_values_segment.as_ref(),
    )
    .map_err(ValidatePcsQueryPlanSegmentsError::UnitValues)?;
    Ok(final_query_challenges)
}

pub(crate) fn proof_binding_segments(segments: &[ProofSegment]) -> Vec<ProofSegment> {
    segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.id,
                PROGRAM_IMAGE_CACHE_SEGMENT_ID
                    | ETH_BLOCK_INPUT_SEGMENT_ID
                    | CHALLENGE_VALUES_SEGMENT_ID
            )
        })
        .cloned()
        .collect()
}

pub(crate) fn duplicate_proof_binding_segment_id(segments: &[ProofSegment]) -> Option<u32> {
    let mut seen = std::collections::BTreeSet::new();
    for segment in segments
        .iter()
        .filter(|segment| is_proof_binding_id(segment.id))
    {
        if !seen.insert(segment.id) {
            return Some(segment.id);
        }
    }
    None
}

pub(crate) fn checked_proof_binding_segments(
    segments: &[ProofSegment],
) -> Result<Vec<ProofSegment>, u32> {
    if let Some(id) = duplicate_proof_binding_segment_id(segments) {
        return Err(id);
    }
    Ok(proof_binding_segments(segments))
}

fn is_proof_binding_id(id: u32) -> bool {
    matches!(
        id,
        PROGRAM_IMAGE_CACHE_SEGMENT_ID | ETH_BLOCK_INPUT_SEGMENT_ID | CHALLENGE_VALUES_SEGMENT_ID
    )
}

fn single_material_manifest_segment(
    segments: &[ProofSegment],
) -> Result<&ProofSegment, ValidatePcsQueryPlanSegmentsError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(ValidatePcsQueryPlanSegmentsError::MissingMaterialSegment)?;
    if matching_segments.next().is_some() {
        return Err(ValidatePcsQueryPlanSegmentsError::DuplicateMaterialSegment);
    }
    Ok(segment)
}

fn single_query_nonce_segment(
    segments: &[ProofSegment],
) -> Result<&ProofSegment, ValidatePcsQueryPlanSegmentsError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == PCS_QUERY_NONCE_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(ValidatePcsQueryPlanSegmentsError::MissingNonceSegment)?;
    if matching_segments.next().is_some() {
        return Err(ValidatePcsQueryPlanSegmentsError::DuplicateNonceSegment);
    }
    Ok(segment)
}

pub fn validate_transcript_pcs_query_plan_segments(
    schedule: &ProveSchedule,
    public_values: &[Felt],
    segments: &[ProofSegment],
) -> Result<(), ValidatePcsQueryPlanSegmentsError> {
    let material_segment = single_material_manifest_segment(segments)?;
    let nonce_segment = single_query_nonce_segment(segments)?;
    let query_plan = load_pcs_query_plan_from_segments(segments)
        .map_err(ValidatePcsQueryPlanSegmentsError::QueryPlan)?;
    let query_segment = segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .ok_or(ValidatePcsQueryPlanSegmentsError::QueryPlan(
            LoadPcsQueryPlanSegmentError::MissingSegment,
        ))?;
    let witness_segments = load_witness_commitment_segment_refs(&schedule.units, segments)
        .map_err(ValidatePcsQueryPlanSegmentsError::Witness)?;
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .map_err(ValidatePcsQueryPlanSegmentsError::Material)?;
    if query_plan.units.is_empty() {
        return Err(ValidatePcsQueryPlanSegmentsError::TranscriptUnitCountMismatch);
    }
    let final_query_challenges = validate_transcript_query_plan_unit_inputs(
        schedule,
        &query_plan.units,
        &material,
        public_values,
        &witness_segments,
        segments,
    )?;
    let final_query_challenge = aggregate_pcs_final_query_challenges(&final_query_challenges)
        .map_err(ValidatePcsQueryPlanSegmentsError::Transcript)?;
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .map_err(ProvePcsQueryPlanSegmentError::from)
            .map_err(ValidatePcsQueryPlanSegmentsError::Build)?
            .nonce,
    );
    let expected_segment = build::build_pcs_query_plan_segment_from_challenge_refs(
        schedule,
        &witness_segments,
        final_query_challenge,
        nonce,
    )
    .map_err(ValidatePcsQueryPlanSegmentsError::Build)?;
    if query_segment.data != expected_segment.data {
        return Err(ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
    }
    Ok(())
}

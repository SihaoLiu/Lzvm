use std::fmt;

use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PcsMaterialManifestSegmentError,
    PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
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
use crate::pcs_query_plan::checked_proof_binding_segments;
use crate::pcs_query_plan::{load_pcs_query_plan_from_segments, LoadPcsQueryPlanSegmentError};
use crate::pcs_transcript::{
    derive_pcs_transcript_challenges_from_segments, PcsTranscriptError, PcsTranscriptSegmentInputs,
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
pub enum PcsTranscriptProofSegmentsError {
    MissingMaterialSegment,
    DuplicateMaterialSegment,
    QueryPlan(LoadPcsQueryPlanSegmentError),
    Material(PcsMaterialManifestSegmentError),
    Fri(LoadPcsFriOpeningUnitError),
    Witness(LoadWitnessCommitmentSegmentsError),
    WitnessSegment {
        unit_index: usize,
        source: WitnessCommitmentSegmentError,
    },
    Evaluation(LoadPcsEvaluationUnitError),
    UnitValues(LoadUnitValuesSegmentError),
    Transcript(PcsTranscriptError),
    DuplicateBindingSegment {
        id: u32,
    },
    UnitIndexOverflow,
    WitnessSegmentIdOverflow,
    UnitMismatch {
        unit_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsTranscriptUnitChallenges {
    pub unit_index: u32,
    pub trace_instance_index: u32,
    pub challenges: Vec<Ext3>,
}

impl fmt::Display for PcsTranscriptProofSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMaterialSegment => write!(f, "missing PCS material manifest segment"),
            Self::DuplicateMaterialSegment => {
                write!(f, "duplicate PCS material manifest segment")
            }
            Self::QueryPlan(error) => write!(f, "{error}"),
            Self::Material(error) => write!(f, "invalid PCS material manifest segment: {error}"),
            Self::Fri(error) => write!(f, "{error}"),
            Self::Witness(error) => write!(f, "{error}"),
            Self::WitnessSegment { unit_index, source } => write!(
                f,
                "invalid witness commitment segment for unit {unit_index}: {source}"
            ),
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::UnitValues(error) => write!(f, "{error}"),
            Self::Transcript(error) => {
                write!(f, "derive PCS transcript challenges failed: {error}")
            }
            Self::DuplicateBindingSegment { id } => {
                write!(f, "duplicate proof binding segment id: {id}")
            }
            Self::UnitIndexOverflow => write!(f, "PCS transcript challenge unit index overflow"),
            Self::WitnessSegmentIdOverflow => {
                write!(f, "PCS transcript challenge witness id overflow")
            }
            Self::UnitMismatch { unit_index } => {
                write!(f, "PCS transcript challenge mismatch for unit {unit_index}")
            }
        }
    }
}

impl std::error::Error for PcsTranscriptProofSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Material(error) => Some(error),
            Self::Fri(error) => Some(error),
            Self::Witness(error) => Some(error),
            Self::WitnessSegment { source, .. } => Some(source),
            Self::Evaluation(error) => Some(error),
            Self::UnitValues(error) => Some(error),
            Self::Transcript(error) => Some(error),
            Self::MissingMaterialSegment
            | Self::DuplicateMaterialSegment
            | Self::DuplicateBindingSegment { .. }
            | Self::UnitIndexOverflow
            | Self::WitnessSegmentIdOverflow
            | Self::UnitMismatch { .. } => None,
        }
    }
}

pub fn derive_pcs_transcript_challenges_from_proof_segments(
    schedule: &ProveSchedule,
    public_values: &[Felt],
    segments: &[ProofSegment],
) -> Result<Vec<Ext3>, PcsTranscriptProofSegmentsError> {
    let units = derive_pcs_transcript_unit_challenges_from_proof_segments(
        schedule,
        public_values,
        segments,
    )?;
    Ok(units.into_iter().flat_map(|unit| unit.challenges).collect())
}

pub fn derive_pcs_transcript_unit_challenges_from_proof_segments(
    schedule: &ProveSchedule,
    public_values: &[Felt],
    segments: &[ProofSegment],
) -> Result<Vec<PcsTranscriptUnitChallenges>, PcsTranscriptProofSegmentsError> {
    let mut material_segments = segments
        .iter()
        .filter(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID);
    let material_segment = material_segments
        .next()
        .ok_or(PcsTranscriptProofSegmentsError::MissingMaterialSegment)?;
    if material_segments.next().is_some() {
        return Err(PcsTranscriptProofSegmentsError::DuplicateMaterialSegment);
    }
    let query_plan = load_pcs_query_plan_from_segments(segments)
        .map_err(PcsTranscriptProofSegmentsError::QueryPlan)?;
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .map_err(PcsTranscriptProofSegmentsError::Material)?;
    let fri = load_pcs_fri_opening_segment_from_segments(segments)
        .map_err(|error| PcsTranscriptProofSegmentsError::Fri(error.into()))?;
    let witness_segments = load_witness_commitment_segment_refs(&schedule.units, segments)
        .map_err(PcsTranscriptProofSegmentsError::Witness)?;
    let binding_segments = checked_proof_binding_segments(segments)
        .map_err(|id| PcsTranscriptProofSegmentsError::DuplicateBindingSegment { id })?;
    let evaluation_segment = load_pcs_evaluation_segment_from_segments(segments)
        .map_err(PcsTranscriptProofSegmentsError::Evaluation)?;
    let unit_values_segment = load_unit_values_segment_from_segments(segments)
        .map_err(PcsTranscriptProofSegmentsError::UnitValues)?;
    let mut units = Vec::new();

    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| PcsTranscriptProofSegmentsError::UnitIndexOverflow)?;
        let unit = schedule
            .units
            .get(unit_index)
            .ok_or(PcsTranscriptProofSegmentsError::UnitMismatch { unit_index })?;
        let material_unit = material
            .units
            .iter()
            .find(|unit| unit.unit_index == query_unit.unit_index)
            .ok_or(PcsTranscriptProofSegmentsError::UnitMismatch { unit_index })?;
        let unit_count = u32::try_from(schedule.units.len())
            .map_err(|_| PcsTranscriptProofSegmentsError::WitnessSegmentIdOverflow)?;
        let witness_segment_id = witness_commitment_segment_id(
            unit_count,
            WitnessCommitmentSegmentIdentity {
                unit_index: query_unit.unit_index,
                trace_instance_index: query_unit.trace_instance_index,
            },
        )
        .map_err(|_| PcsTranscriptProofSegmentsError::WitnessSegmentIdOverflow)?;
        let witness_segment = witness_segments
            .iter()
            .find(|segment| segment.id == witness_segment_id)
            .ok_or(PcsTranscriptProofSegmentsError::UnitMismatch { unit_index })?;
        let witness =
            parse_witness_commitment_segment(&witness_segment.data).map_err(|source| {
                PcsTranscriptProofSegmentsError::WitnessSegment { unit_index, source }
            })?;
        let evaluation_unit = load_pcs_evaluation_unit_for_identity_from_parsed_segment(
            unit_index,
            query_unit.trace_instance_index,
            unit,
            &evaluation_segment,
        )
        .map_err(PcsTranscriptProofSegmentsError::Evaluation)?;
        let fri_unit = fri
            .units
            .iter()
            .find(|unit| {
                unit.unit_index == query_unit.unit_index
                    && unit.trace_instance_index == query_unit.trace_instance_index
            })
            .ok_or(PcsTranscriptProofSegmentsError::UnitMismatch { unit_index })?;
        let unit_values = load_unit_values_for_identity_from_parsed_segment(
            unit_index,
            query_unit.trace_instance_index,
            &unit.unit_value_map,
            unit_values_segment.as_ref(),
        )
        .map_err(PcsTranscriptProofSegmentsError::UnitValues)?;
        let mut unit_challenges =
            derive_pcs_transcript_challenges_from_segments(PcsTranscriptSegmentInputs {
                unit_index,
                unit,
                material: material_unit,
                public_values,
                unit_values: &unit_values,
                witness: &witness,
                evaluations: evaluation_unit,
                fri: fri_unit,
                root_challenge_draws: &unit.transcript_root_challenge_draws,
                evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
                binding_segments: &binding_segments,
            })
            .map_err(PcsTranscriptProofSegmentsError::Transcript)?;
        unit_challenges.shrink_to_fit();
        units.push(PcsTranscriptUnitChallenges {
            unit_index: query_unit.unit_index,
            trace_instance_index: query_unit.trace_instance_index,
            challenges: unit_challenges,
        });
    }
    validate_pcs_evaluation_units_match_query_units_from_segment(
        &query_plan.units,
        &evaluation_segment,
    )
    .map_err(PcsTranscriptProofSegmentsError::Evaluation)?;
    validate_pcs_fri_opening_units_match_query_units_from_segment(&query_plan.units, &fri)
        .map_err(PcsTranscriptProofSegmentsError::Fri)?;
    validate_unit_values_units_match_query_units_from_segment(
        &query_plan.units,
        unit_values_segment.as_ref(),
    )
    .map_err(PcsTranscriptProofSegmentsError::UnitValues)?;

    Ok(units)
}

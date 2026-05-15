use std::fmt;

use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PcsMaterialManifestSegmentError,
    PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, WitnessCommitmentSegmentError,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt};

use crate::pcs_evaluation::{load_pcs_evaluation_unit_from_segments, LoadPcsEvaluationUnitError};
use crate::pcs_fri::{load_pcs_fri_opening_segment_from_segments, LoadPcsFriOpeningSegmentError};
use crate::pcs_query_plan::{load_pcs_query_plan_from_segments, LoadPcsQueryPlanSegmentError};
use crate::pcs_transcript::{
    derive_pcs_transcript_challenges_from_segments, PcsTranscriptError, PcsTranscriptSegmentInputs,
};
use crate::unit_values::{load_unit_values_from_segments, LoadUnitValuesSegmentError};
use crate::witness_commitment::{
    load_witness_commitment_segments, LoadWitnessCommitmentSegmentsError,
};
use crate::ProveSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsTranscriptProofSegmentsError {
    MissingMaterialSegment,
    QueryPlan(LoadPcsQueryPlanSegmentError),
    Material(PcsMaterialManifestSegmentError),
    Fri(LoadPcsFriOpeningSegmentError),
    Witness(LoadWitnessCommitmentSegmentsError),
    WitnessSegment {
        unit_index: usize,
        source: WitnessCommitmentSegmentError,
    },
    Evaluation(LoadPcsEvaluationUnitError),
    UnitValues(LoadUnitValuesSegmentError),
    Transcript(PcsTranscriptError),
    UnitIndexOverflow,
    WitnessSegmentIdOverflow,
    UnitMismatch {
        unit_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsTranscriptUnitChallenges {
    pub unit_index: u32,
    pub challenges: Vec<Ext3>,
}

impl fmt::Display for PcsTranscriptProofSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMaterialSegment => write!(f, "missing PCS material manifest segment"),
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
    let material_segment = segments
        .iter()
        .find(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID)
        .ok_or(PcsTranscriptProofSegmentsError::MissingMaterialSegment)?;
    let query_plan = load_pcs_query_plan_from_segments(segments)
        .map_err(PcsTranscriptProofSegmentsError::QueryPlan)?;
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .map_err(PcsTranscriptProofSegmentsError::Material)?;
    let fri = load_pcs_fri_opening_segment_from_segments(segments)
        .map_err(PcsTranscriptProofSegmentsError::Fri)?;
    let witness_segments = load_witness_commitment_segments(&schedule.units, segments)
        .map_err(PcsTranscriptProofSegmentsError::Witness)?;
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
        let witness_segment_id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
            .checked_add(query_unit.unit_index)
            .ok_or(PcsTranscriptProofSegmentsError::WitnessSegmentIdOverflow)?;
        let witness_segment = witness_segments
            .iter()
            .find(|segment| segment.id == witness_segment_id)
            .ok_or(PcsTranscriptProofSegmentsError::UnitMismatch { unit_index })?;
        let witness =
            parse_witness_commitment_segment(&witness_segment.data).map_err(|source| {
                PcsTranscriptProofSegmentsError::WitnessSegment { unit_index, source }
            })?;
        let evaluation_unit = load_pcs_evaluation_unit_from_segments(unit_index, unit, segments)
            .map_err(PcsTranscriptProofSegmentsError::Evaluation)?;
        let fri_unit = fri
            .units
            .iter()
            .find(|unit| unit.unit_index == query_unit.unit_index)
            .ok_or(PcsTranscriptProofSegmentsError::UnitMismatch { unit_index })?;
        let unit_values =
            load_unit_values_from_segments(unit_index, &unit.unit_value_map, segments)
                .map_err(PcsTranscriptProofSegmentsError::UnitValues)?;
        let mut unit_challenges =
            derive_pcs_transcript_challenges_from_segments(PcsTranscriptSegmentInputs {
                unit_index,
                unit,
                material: material_unit,
                public_values,
                unit_values: &unit_values,
                witness: &witness,
                evaluations: &evaluation_unit,
                fri: fri_unit,
                root_challenge_draws: &unit.transcript_root_challenge_draws,
                evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
            })
            .map_err(PcsTranscriptProofSegmentsError::Transcript)?;
        unit_challenges.shrink_to_fit();
        units.push(PcsTranscriptUnitChallenges {
            unit_index: query_unit.unit_index,
            challenges: unit_challenges,
        });
    }

    Ok(units)
}

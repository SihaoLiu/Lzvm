use std::collections::BTreeSet;
use std::fmt;

use lzvm_artifacts::pcs_evaluation_segment::{
    parse_pcs_evaluation_segment, PcsEvaluationSegmentError, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, PcsFriOpeningSegment, PcsFriOpeningSegmentError,
    PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PcsMaterialManifestSegmentError,
    PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegmentError, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, WitnessCommitmentSegmentError,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt, FieldError};

use crate::pcs_fri::{
    build_pcs_fri_opening_unit, build_pcs_fri_transcript_commitments, PcsFriOpeningBuildError,
    PcsFriOpeningBuildRequest, PcsFriTranscriptCommitmentError, PcsFriTranscriptCommitmentRequest,
    PcsFriTranscriptCommitments,
};
use crate::pcs_transcript::{
    derive_pcs_transcript_prefix_challenges, PcsTranscriptError, PcsTranscriptPrefixInputs,
};
use crate::prove_fri_polynomial::{build_pcs_fri_polynomial_values, ProvePcsFriPolynomialError};
use crate::witness_trace::WitnessTraceBuffer;
use crate::{ProveExecutionUnitArtifacts, ProveSchedule, ProveWitnessAuxiliaryInputs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsFriOpeningValues {
    pub unit_index: usize,
    pub challenges: Vec<Ext3>,
    pub polynomial: Vec<Ext3>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProvePcsFriOpeningTraceValues<'a> {
    pub unit_index: usize,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    pub challenges: &'a [Ext3],
    pub xi_challenge: Ext3,
}

#[derive(Debug, Clone, Copy)]
pub struct ProvePcsFriTranscriptTraceValues<'a> {
    pub unit_index: usize,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    pub constant_root: [Felt; 4],
    pub witness_roots: &'a [[Felt; 4]],
    pub evaluation_values: &'a [Ext3],
    pub xi_challenge: Ext3,
}

#[derive(Debug, Clone, Copy)]
pub struct ProvePcsFriTranscriptTraceSegmentValues<'a> {
    pub unit_index: usize,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    pub material_segment: &'a ProofSegment,
    pub witness_segment: &'a ProofSegment,
    pub evaluation_segment: &'a ProofSegment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsFriTranscriptValues {
    pub unit_index: usize,
    pub polynomial: Vec<Ext3>,
    pub commitments: PcsFriTranscriptCommitments,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsFriOpeningSegmentError {
    InvalidQuerySegmentId {
        segment_id: u32,
    },
    QueryPlan(PcsQueryPlanSegmentError),
    MissingQueryUnit {
        unit_index: usize,
    },
    DuplicateUnitIndex {
        unit_index: usize,
    },
    UnitIndexOverflow {
        unit_index: usize,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    Build {
        unit_index: usize,
        source: PcsFriOpeningBuildError,
    },
    Segment(PcsFriOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsFriOpeningTraceSegmentError {
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    Polynomial {
        unit_index: usize,
        source: Box<ProvePcsFriPolynomialError>,
    },
    Opening {
        source: Box<ProvePcsFriOpeningSegmentError>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsFriTranscriptTraceValuesError {
    UnitIndexOverflow {
        unit_index: usize,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    MissingTranscriptArity {
        unit_index: usize,
    },
    InvalidMaterialSegmentId {
        segment_id: u32,
    },
    InvalidWitnessSegmentId {
        unit_index: usize,
        expected: u32,
        found: u32,
    },
    InvalidEvaluationSegmentId {
        segment_id: u32,
    },
    MaterialSegment(PcsMaterialManifestSegmentError),
    WitnessSegment {
        unit_index: usize,
        source: WitnessCommitmentSegmentError,
    },
    EvaluationSegment(PcsEvaluationSegmentError),
    MissingMaterialUnit {
        unit_index: usize,
    },
    MissingEvaluationUnit {
        unit_index: usize,
    },
    SegmentUnitIndexMismatch {
        segment: &'static str,
        expected: u32,
        found: u32,
    },
    Field {
        unit_index: usize,
        source: FieldError,
    },
    PrefixTranscript {
        unit_index: usize,
        source: Box<PcsTranscriptError>,
    },
    MissingXiChallenge {
        unit_index: usize,
        challenge_count: usize,
    },
    PrefixChallengeOutOfRange {
        unit_index: usize,
        index: usize,
        len: usize,
    },
    Polynomial {
        unit_index: usize,
        source: Box<ProvePcsFriPolynomialError>,
    },
    Transcript {
        unit_index: usize,
        source: Box<PcsFriTranscriptCommitmentError>,
    },
}

impl fmt::Display for ProvePcsFriOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuerySegmentId { segment_id } => write!(
                f,
                "prove PCS FRI opening expected query plan segment id {PCS_QUERY_PLAN_SEGMENT_ID}, found {segment_id}"
            ),
            Self::QueryPlan(error) => {
                write!(f, "prove PCS FRI opening query plan parse failed: {error}")
            }
            Self::MissingQueryUnit { unit_index } => {
                write!(f, "prove PCS FRI opening is missing query unit {unit_index}")
            }
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate prove PCS FRI opening unit index: {unit_index}")
            }
            Self::UnitIndexOverflow { unit_index } => write!(
                f,
                "prove PCS FRI opening unit index does not fit u32: {unit_index}"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove PCS FRI opening unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::Build { unit_index, source } => write!(
                f,
                "prove PCS FRI opening build failed for unit {unit_index}: {source}"
            ),
            Self::Segment(error) => {
                write!(f, "prove PCS FRI opening segment encode failed: {error}")
            }
        }
    }
}

impl fmt::Display for ProvePcsFriOpeningTraceSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove PCS FRI trace opening unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::Polynomial { unit_index, source } => write!(
                f,
                "prove PCS FRI trace opening polynomial failed for unit {unit_index}: {source}"
            ),
            Self::Opening { source } => {
                write!(f, "prove PCS FRI trace opening segment failed: {source}")
            }
        }
    }
}

impl fmt::Display for ProvePcsFriTranscriptTraceValuesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOverflow { unit_index } => write!(
                f,
                "prove PCS FRI transcript unit index does not fit u32: {unit_index}"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove PCS FRI transcript unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::MissingTranscriptArity { unit_index } => write!(
                f,
                "prove PCS FRI transcript unit {unit_index} is missing transcript arity"
            ),
            Self::InvalidMaterialSegmentId { segment_id } => write!(
                f,
                "prove PCS FRI transcript expected material segment id {PCS_MATERIAL_MANIFEST_SEGMENT_ID}, found {segment_id}"
            ),
            Self::InvalidWitnessSegmentId {
                unit_index,
                expected,
                found,
            } => write!(
                f,
                "prove PCS FRI transcript witness segment id mismatch for unit {unit_index}: expected {expected}, found {found}"
            ),
            Self::InvalidEvaluationSegmentId { segment_id } => write!(
                f,
                "prove PCS FRI transcript expected evaluation segment id {PCS_EVALUATION_SEGMENT_ID}, found {segment_id}"
            ),
            Self::MaterialSegment(error) => {
                write!(f, "prove PCS FRI transcript material segment failed: {error}")
            }
            Self::WitnessSegment { unit_index, source } => write!(
                f,
                "prove PCS FRI transcript witness segment failed for unit {unit_index}: {source}"
            ),
            Self::EvaluationSegment(error) => write!(
                f,
                "prove PCS FRI transcript evaluation segment failed: {error}"
            ),
            Self::MissingMaterialUnit { unit_index } => write!(
                f,
                "prove PCS FRI transcript material segment is missing unit {unit_index}"
            ),
            Self::MissingEvaluationUnit { unit_index } => write!(
                f,
                "prove PCS FRI transcript evaluation segment is missing unit {unit_index}"
            ),
            Self::SegmentUnitIndexMismatch {
                segment,
                expected,
                found,
            } => write!(
                f,
                "prove PCS FRI transcript {segment} unit index mismatch: expected {expected}, found {found}"
            ),
            Self::Field { unit_index, source } => write!(
                f,
                "prove PCS FRI transcript field conversion failed for unit {unit_index}: {source}"
            ),
            Self::PrefixTranscript { unit_index, source } => write!(
                f,
                "prove PCS FRI transcript prefix failed for unit {unit_index}: {source}"
            ),
            Self::MissingXiChallenge {
                unit_index,
                challenge_count,
            } => write!(
                f,
                "prove PCS FRI transcript unit {unit_index} challenge count {challenge_count} cannot locate xi challenge"
            ),
            Self::PrefixChallengeOutOfRange {
                unit_index,
                index,
                len,
            } => write!(
                f,
                "prove PCS FRI transcript unit {unit_index} prefix challenge index {index} is outside challenge count {len}"
            ),
            Self::Polynomial { unit_index, source } => write!(
                f,
                "prove PCS FRI transcript polynomial failed for unit {unit_index}: {source}"
            ),
            Self::Transcript { unit_index, source } => write!(
                f,
                "prove PCS FRI transcript commitments failed for unit {unit_index}: {source}"
            ),
        }
    }
}

impl std::error::Error for ProvePcsFriOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Build { source, .. } => Some(source),
            Self::Segment(error) => Some(error),
            Self::InvalidQuerySegmentId { .. }
            | Self::MissingQueryUnit { .. }
            | Self::DuplicateUnitIndex { .. }
            | Self::UnitIndexOverflow { .. }
            | Self::UnitIndexOutOfRange { .. } => None,
        }
    }
}

impl std::error::Error for ProvePcsFriOpeningTraceSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Polynomial { source, .. } => Some(source.as_ref()),
            Self::Opening { source } => Some(source.as_ref()),
            Self::UnitIndexOutOfRange { .. } => None,
        }
    }
}

impl std::error::Error for ProvePcsFriTranscriptTraceValuesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MaterialSegment(error) => Some(error),
            Self::WitnessSegment { source, .. } => Some(source),
            Self::EvaluationSegment(error) => Some(error),
            Self::Field { source, .. } => Some(source),
            Self::PrefixTranscript { source, .. } => Some(source.as_ref()),
            Self::Polynomial { source, .. } => Some(source.as_ref()),
            Self::Transcript { source, .. } => Some(source.as_ref()),
            Self::UnitIndexOverflow { .. }
            | Self::UnitIndexOutOfRange { .. }
            | Self::MissingTranscriptArity { .. }
            | Self::InvalidMaterialSegmentId { .. }
            | Self::InvalidWitnessSegmentId { .. }
            | Self::InvalidEvaluationSegmentId { .. }
            | Self::MissingMaterialUnit { .. }
            | Self::MissingEvaluationUnit { .. }
            | Self::SegmentUnitIndexMismatch { .. }
            | Self::MissingXiChallenge { .. }
            | Self::PrefixChallengeOutOfRange { .. } => None,
        }
    }
}

impl From<PcsQueryPlanSegmentError> for ProvePcsFriOpeningSegmentError {
    fn from(error: PcsQueryPlanSegmentError) -> Self {
        Self::QueryPlan(error)
    }
}

impl From<PcsFriOpeningSegmentError> for ProvePcsFriOpeningSegmentError {
    fn from(error: PcsFriOpeningSegmentError) -> Self {
        Self::Segment(error)
    }
}

pub fn build_pcs_fri_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriOpeningValues],
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    if query_segment.id != PCS_QUERY_PLAN_SEGMENT_ID {
        return Err(ProvePcsFriOpeningSegmentError::InvalidQuerySegmentId {
            segment_id: query_segment.id,
        });
    }
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let mut seen_units = BTreeSet::new();
    let mut units = Vec::with_capacity(values.len());
    for input in values {
        if !seen_units.insert(input.unit_index) {
            return Err(ProvePcsFriOpeningSegmentError::DuplicateUnitIndex {
                unit_index: input.unit_index,
            });
        }
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriOpeningSegmentError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let unit_index_u32 = u32::try_from(input.unit_index).map_err(|_| {
            ProvePcsFriOpeningSegmentError::UnitIndexOverflow {
                unit_index: input.unit_index,
            }
        })?;
        let query_unit = query_plan
            .units
            .iter()
            .find(|unit| unit.unit_index == unit_index_u32)
            .ok_or(ProvePcsFriOpeningSegmentError::MissingQueryUnit {
                unit_index: input.unit_index,
            })?;
        let opening = build_pcs_fri_opening_unit(
            unit,
            PcsFriOpeningBuildRequest {
                unit_index: unit_index_u32,
                query_rows: &query_unit.queries,
                challenges: &input.challenges,
                polynomial: &input.polynomial,
            },
        )
        .map_err(|source| ProvePcsFriOpeningSegmentError::Build {
            unit_index: input.unit_index,
            source,
        })?;
        units.push(opening);
    }

    let segment = PcsFriOpeningSegment { units };
    Ok(ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: encode_pcs_fri_opening_segment(&segment)?,
    })
}

pub fn build_pcs_fri_transcript_values_from_trace(
    schedule: &ProveSchedule,
    values: &[ProvePcsFriTranscriptTraceValues<'_>],
) -> Result<Vec<ProvePcsFriTranscriptValues>, ProvePcsFriTranscriptTraceValuesError> {
    let mut out = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriTranscriptTraceValuesError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let arity = unit.transcript_arity.ok_or(
            ProvePcsFriTranscriptTraceValuesError::MissingTranscriptArity {
                unit_index: input.unit_index,
            },
        )? as usize;
        let polynomial = build_pcs_fri_polynomial_values(
            input.unit_index,
            unit,
            input.execution_unit,
            input.trace,
            input.publics,
            input.auxiliary_inputs,
            input.xi_challenge,
        )
        .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Polynomial {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        let commitments = build_pcs_fri_transcript_commitments(
            unit,
            PcsFriTranscriptCommitmentRequest {
                arity,
                hash_values: unit.hash_commits,
                constant_root: input.constant_root,
                public_values: input.publics,
                witness_roots: input.witness_roots,
                root_challenge_draws: &unit.transcript_root_challenge_draws,
                unit_value_map: &unit.unit_value_map,
                unit_values: &input.auxiliary_inputs.unit_values,
                evaluation_values: input.evaluation_values,
                evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
                polynomial: &polynomial,
            },
        )
        .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Transcript {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        out.push(ProvePcsFriTranscriptValues {
            unit_index: input.unit_index,
            polynomial,
            commitments,
        });
    }
    Ok(out)
}

pub fn build_pcs_fri_transcript_values_from_trace_segments(
    schedule: &ProveSchedule,
    values: &[ProvePcsFriTranscriptTraceSegmentValues<'_>],
) -> Result<Vec<ProvePcsFriTranscriptValues>, ProvePcsFriTranscriptTraceValuesError> {
    let mut out = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriTranscriptTraceValuesError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let unit_index_u32 = u32::try_from(input.unit_index).map_err(|_| {
            ProvePcsFriTranscriptTraceValuesError::UnitIndexOverflow {
                unit_index: input.unit_index,
            }
        })?;
        let arity = unit.transcript_arity.ok_or(
            ProvePcsFriTranscriptTraceValuesError::MissingTranscriptArity {
                unit_index: input.unit_index,
            },
        )? as usize;
        if input.material_segment.id != PCS_MATERIAL_MANIFEST_SEGMENT_ID {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::InvalidMaterialSegmentId {
                    segment_id: input.material_segment.id,
                },
            );
        }
        let expected_witness_id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
            .checked_add(unit_index_u32)
            .ok_or(ProvePcsFriTranscriptTraceValuesError::UnitIndexOverflow {
                unit_index: input.unit_index,
            })?;
        if input.witness_segment.id != expected_witness_id {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::InvalidWitnessSegmentId {
                    unit_index: input.unit_index,
                    expected: expected_witness_id,
                    found: input.witness_segment.id,
                },
            );
        }
        if input.evaluation_segment.id != PCS_EVALUATION_SEGMENT_ID {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::InvalidEvaluationSegmentId {
                    segment_id: input.evaluation_segment.id,
                },
            );
        }

        let material = parse_pcs_material_manifest_segment(&input.material_segment.data)
            .map_err(ProvePcsFriTranscriptTraceValuesError::MaterialSegment)?
            .units
            .into_iter()
            .find(|unit| unit.unit_index == unit_index_u32)
            .ok_or(ProvePcsFriTranscriptTraceValuesError::MissingMaterialUnit {
                unit_index: input.unit_index,
            })?;
        let witness =
            parse_witness_commitment_segment(&input.witness_segment.data).map_err(|source| {
                ProvePcsFriTranscriptTraceValuesError::WitnessSegment {
                    unit_index: input.unit_index,
                    source,
                }
            })?;
        if witness.unit_index != unit_index_u32 {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::SegmentUnitIndexMismatch {
                    segment: "witness",
                    expected: unit_index_u32,
                    found: witness.unit_index,
                },
            );
        }
        let evaluations = parse_pcs_evaluation_segment(&input.evaluation_segment.data)
            .map_err(ProvePcsFriTranscriptTraceValuesError::EvaluationSegment)?
            .units
            .into_iter()
            .find(|unit| unit.unit_index == unit_index_u32)
            .ok_or(
                ProvePcsFriTranscriptTraceValuesError::MissingEvaluationUnit {
                    unit_index: input.unit_index,
                },
            )?;

        let constant_root = root_from_words(material.constant_tree_root).map_err(|source| {
            ProvePcsFriTranscriptTraceValuesError::Field {
                unit_index: input.unit_index,
                source,
            }
        })?;
        let witness_roots = witness
            .stages
            .iter()
            .map(|stage| root_from_words(stage.root))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Field {
                unit_index: input.unit_index,
                source,
            })?;
        let evaluation_values = evaluations
            .values
            .iter()
            .map(|value| extension_from_words(*value))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Field {
                unit_index: input.unit_index,
                source,
            })?;
        let prefix_challenges =
            derive_pcs_transcript_prefix_challenges(PcsTranscriptPrefixInputs {
                arity,
                hash_values: unit.hash_commits,
                constant_root,
                public_values: input.publics,
                witness_roots: &witness_roots,
                root_challenge_draws: &unit.transcript_root_challenge_draws,
                unit_value_map: &unit.unit_value_map,
                unit_values: &input.auxiliary_inputs.unit_values,
                evaluation_values: &evaluation_values,
                evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
            })
            .map_err(|source| {
                ProvePcsFriTranscriptTraceValuesError::PrefixTranscript {
                    unit_index: input.unit_index,
                    source: Box::new(source),
                }
            })?;
        let xi_index = unit.challenge_count.checked_sub(3).ok_or(
            ProvePcsFriTranscriptTraceValuesError::MissingXiChallenge {
                unit_index: input.unit_index,
                challenge_count: unit.challenge_count,
            },
        )?;
        let xi_challenge = *prefix_challenges.get(xi_index).ok_or(
            ProvePcsFriTranscriptTraceValuesError::PrefixChallengeOutOfRange {
                unit_index: input.unit_index,
                index: xi_index,
                len: prefix_challenges.len(),
            },
        )?;

        let mut built = build_pcs_fri_transcript_values_from_trace(
            schedule,
            &[ProvePcsFriTranscriptTraceValues {
                unit_index: input.unit_index,
                execution_unit: input.execution_unit,
                trace: input.trace,
                publics: input.publics,
                auxiliary_inputs: input.auxiliary_inputs,
                constant_root,
                witness_roots: &witness_roots,
                evaluation_values: &evaluation_values,
                xi_challenge,
            }],
        )?;
        out.append(&mut built);
    }
    Ok(out)
}

pub fn build_pcs_fri_opening_segment_from_transcript_values(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriTranscriptValues],
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    let opening_values = values
        .iter()
        .map(|value| ProvePcsFriOpeningValues {
            unit_index: value.unit_index,
            challenges: value.commitments.challenges.clone(),
            polynomial: value.polynomial.clone(),
        })
        .collect::<Vec<_>>();
    build_pcs_fri_opening_segment(schedule, query_segment, &opening_values)
}

pub fn build_pcs_fri_opening_segment_from_trace(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriOpeningTraceValues<'_>],
) -> Result<ProofSegment, ProvePcsFriOpeningTraceSegmentError> {
    let mut opening_values = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriOpeningTraceSegmentError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let polynomial = build_pcs_fri_polynomial_values(
            input.unit_index,
            unit,
            input.execution_unit,
            input.trace,
            input.publics,
            input.auxiliary_inputs,
            input.xi_challenge,
        )
        .map_err(|source| ProvePcsFriOpeningTraceSegmentError::Polynomial {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        opening_values.push(ProvePcsFriOpeningValues {
            unit_index: input.unit_index,
            challenges: input.challenges.to_vec(),
            polynomial,
        });
    }

    build_pcs_fri_opening_segment(schedule, query_segment, &opening_values).map_err(|source| {
        ProvePcsFriOpeningTraceSegmentError::Opening {
            source: Box::new(source),
        }
    })
}

fn root_from_words(words: [u64; 4]) -> Result<[Felt; 4], FieldError> {
    Ok([
        Felt::from_canonical(words[0])?,
        Felt::from_canonical(words[1])?,
        Felt::from_canonical(words[2])?,
        Felt::from_canonical(words[3])?,
    ])
}

fn extension_from_words(words: [u64; 3]) -> Result<Ext3, FieldError> {
    Ok(Ext3::new(
        Felt::from_canonical(words[0])?,
        Felt::from_canonical(words[1])?,
        Felt::from_canonical(words[2])?,
    ))
}

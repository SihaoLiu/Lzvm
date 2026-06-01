use std::fmt;

use lzvm_artifacts::pcs_evaluation_segment::{
    PcsEvaluationSegmentError, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::PcsFriOpeningSegmentError;
use lzvm_artifacts::pcs_material_segment::{
    PcsMaterialManifestSegmentError, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{PcsQueryPlanSegmentError, PCS_QUERY_PLAN_SEGMENT_ID};
use lzvm_artifacts::witness_segment::WitnessCommitmentSegmentError;
use lzvm_field::FieldError;

use crate::pcs_fri::{PcsFriOpeningBuildError, PcsFriTranscriptCommitmentError};
use crate::pcs_transcript::PcsTranscriptError;
use crate::prove_fri_polynomial::ProvePcsFriPolynomialError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsFriOpeningSegmentError {
    InvalidQuerySegmentId {
        segment_id: u32,
    },
    QueryPlan(PcsQueryPlanSegmentError),
    UnsupportedTraceInstance {
        unit_index: u32,
        trace_instance_index: u32,
    },
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
    TranscriptValues {
        source: Box<ProvePcsFriTranscriptTraceValuesError>,
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
            Self::UnsupportedTraceInstance {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "prove PCS FRI opening query plan trace instance {trace_instance_index} for unit {unit_index} is unsupported"
            ),
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
            Self::TranscriptValues { source } => {
                write!(f, "prove PCS FRI trace opening transcript values failed: {source}")
            }
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
            | Self::UnsupportedTraceInstance { .. }
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
            Self::TranscriptValues { source } => Some(source.as_ref()),
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

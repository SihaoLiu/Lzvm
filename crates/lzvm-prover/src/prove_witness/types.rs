use std::fmt;

use lzvm_artifacts::constant_opening_segment::ConstantOpeningSegmentError;
use lzvm_artifacts::constant_tree::ConstantTreeError;
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_evaluation_segment::PcsEvaluationSegmentError;
use lzvm_artifacts::pcs_material_segment::PcsMaterialManifestSegmentError;
use lzvm_artifacts::pcs_nonce_segment::{PcsQueryNonceSegmentError, PCS_QUERY_NONCE_SEGMENT_ID};
use lzvm_artifacts::pcs_query_segment::PcsQueryPlanSegmentError;
use lzvm_artifacts::witness_opening_segment::WitnessOpeningSegmentError;
use lzvm_artifacts::witness_segment::WitnessCommitmentSegmentError;
use lzvm_field::Ext3;

use crate::constant_tree_opening::ConstantTreeOpeningError;
use crate::pcs_challenge::PcsChallengeError;
use crate::pcs_transcript::PcsTranscriptError;
use crate::witness_commitment::WitnessStageOpeningError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsEvaluationValues {
    pub unit_index: usize,
    pub values: Vec<Ext3>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveWitnessSegmentError {
    LengthOverflow,
    Segment(WitnessCommitmentSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsMaterialSegmentError {
    MissingMaterial {
        unit_index: usize,
        kind: KeyUnitKind,
    },
    UnitIndexOverflow {
        unit_index: usize,
    },
    Segment(PcsMaterialManifestSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsEvaluationSegmentError {
    UnitIndexOverflow {
        unit_index: usize,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    ValueCountMismatch {
        unit_index: usize,
        expected: usize,
        found: usize,
    },
    Segment(PcsEvaluationSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsQueryPlanSegmentError {
    MissingWitnessSegments,
    InvalidWitnessSegment {
        unit_index: usize,
        source: WitnessCommitmentSegmentError,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    WitnessUnitMismatch {
        segment_unit_index: u32,
        payload_unit_index: u32,
    },
    QueryCountExceedsDomain {
        unit_index: usize,
        query_count: u32,
        domain_size: u64,
    },
    MissingTranscriptArity {
        unit_index: usize,
    },
    InvalidNonceSegmentId {
        segment_id: u32,
    },
    QueryNonceMismatch {
        unit_index: usize,
        bits: u32,
    },
    Challenge(PcsChallengeError),
    Transcript(PcsTranscriptError),
    LengthOverflow,
    Segment(PcsQueryPlanSegmentError),
    NonceSegment(PcsQueryNonceSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveWitnessOpeningSegmentError {
    QueryPlan(PcsQueryPlanSegmentError),
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
    Segment(WitnessOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveConstantOpeningSegmentError {
    QueryPlan(PcsQueryPlanSegmentError),
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

impl fmt::Display for ProveWitnessSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthOverflow => write!(f, "prove witness segment length overflow"),
            Self::Segment(error) => write!(f, "prove witness segment encode failed: {error}"),
        }
    }
}

impl fmt::Display for ProvePcsMaterialSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMaterial { unit_index, kind } => write!(
                f,
                "prove PCS material segment is missing material for unit {unit_index} ({kind})"
            ),
            Self::UnitIndexOverflow { unit_index } => {
                write!(
                    f,
                    "prove PCS material segment unit index does not fit u32: {unit_index}"
                )
            }
            Self::Segment(error) => write!(f, "prove PCS material segment encode failed: {error}"),
        }
    }
}

impl fmt::Display for ProvePcsEvaluationSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOverflow { unit_index } => write!(
                f,
                "prove PCS evaluation segment unit index does not fit u32: {unit_index}"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove PCS evaluation segment unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::ValueCountMismatch {
                unit_index,
                expected,
                found,
            } => write!(
                f,
                "prove PCS evaluation segment unit {unit_index} value count mismatch: expected {expected}, found {found}"
            ),
            Self::Segment(error) => {
                write!(f, "prove PCS evaluation segment encode failed: {error}")
            }
        }
    }
}

impl fmt::Display for ProvePcsQueryPlanSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingWitnessSegments => write!(f, "prove PCS query plan has no witness segments"),
            Self::InvalidWitnessSegment { unit_index, source } => write!(
                f,
                "prove PCS query plan witness segment for unit {unit_index} is invalid: {source}"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove PCS query plan unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::WitnessUnitMismatch {
                segment_unit_index,
                payload_unit_index,
            } => write!(
                f,
                "prove PCS query plan witness unit mismatch: segment {segment_unit_index}, payload {payload_unit_index}"
            ),
            Self::QueryCountExceedsDomain {
                unit_index,
                query_count,
                domain_size,
            } => write!(
                f,
                "prove PCS query plan unit {unit_index} query count {query_count} exceeds domain size {domain_size}"
            ),
            Self::MissingTranscriptArity { unit_index } => write!(
                f,
                "prove PCS query plan unit {unit_index} is missing transcript arity"
            ),
            Self::InvalidNonceSegmentId { segment_id } => write!(
                f,
                "prove PCS query plan expected query nonce segment id {PCS_QUERY_NONCE_SEGMENT_ID}, found {segment_id}"
            ),
            Self::QueryNonceMismatch { unit_index, bits } => write!(
                f,
                "prove PCS query plan unit {unit_index} query nonce does not satisfy {bits} work bits"
            ),
            Self::Challenge(error) => write!(f, "prove PCS query plan challenge failed: {error}"),
            Self::Transcript(error) => {
                write!(f, "prove PCS query plan transcript failed: {error}")
            }
            Self::LengthOverflow => write!(f, "prove PCS query plan length overflow"),
            Self::Segment(error) => write!(f, "prove PCS query plan encode failed: {error}"),
            Self::NonceSegment(error) => {
                write!(f, "prove PCS query nonce segment encode failed: {error}")
            }
        }
    }
}

impl fmt::Display for ProveWitnessOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryPlan(error) => {
                write!(f, "prove witness opening query plan parse failed: {error}")
            }
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
            Self::Segment(error) => write!(f, "prove witness opening segment encode failed: {error}"),
        }
    }
}

impl fmt::Display for ProveConstantOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryPlan(error) => {
                write!(f, "prove constant opening query plan parse failed: {error}")
            }
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

impl std::error::Error for ProveWitnessSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::LengthOverflow => None,
        }
    }
}

impl std::error::Error for ProvePcsMaterialSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingMaterial { .. } | Self::UnitIndexOverflow { .. } => None,
        }
    }
}

impl std::error::Error for ProvePcsEvaluationSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::UnitIndexOverflow { .. }
            | Self::UnitIndexOutOfRange { .. }
            | Self::ValueCountMismatch { .. } => None,
        }
    }
}

impl std::error::Error for ProvePcsQueryPlanSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidWitnessSegment { source, .. } => Some(source),
            Self::Challenge(error) => Some(error),
            Self::Transcript(error) => Some(error),
            Self::Segment(error) => Some(error),
            Self::NonceSegment(error) => Some(error),
            Self::MissingWitnessSegments
            | Self::UnitIndexOutOfRange { .. }
            | Self::WitnessUnitMismatch { .. }
            | Self::QueryCountExceedsDomain { .. }
            | Self::MissingTranscriptArity { .. }
            | Self::InvalidNonceSegmentId { .. }
            | Self::QueryNonceMismatch { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl std::error::Error for ProveWitnessOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Opening(error) => Some(error),
            Self::Segment(error) => Some(error),
            Self::MissingQueryUnit { .. }
            | Self::MissingOutputUnit { .. }
            | Self::DuplicateOutputUnit { .. }
            | Self::UnitIndexOverflow { .. }
            | Self::UnitIndexOutOfRange { .. }
            | Self::StageIndexOutOfRange { .. } => None,
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
            Self::UnitIndexOutOfRange { .. } | Self::UnitIndexOverflow { .. } => None,
        }
    }
}

impl From<WitnessCommitmentSegmentError> for ProveWitnessSegmentError {
    fn from(error: WitnessCommitmentSegmentError) -> Self {
        Self::Segment(error)
    }
}

impl From<PcsMaterialManifestSegmentError> for ProvePcsMaterialSegmentError {
    fn from(error: PcsMaterialManifestSegmentError) -> Self {
        Self::Segment(error)
    }
}

impl From<PcsEvaluationSegmentError> for ProvePcsEvaluationSegmentError {
    fn from(error: PcsEvaluationSegmentError) -> Self {
        Self::Segment(error)
    }
}

impl From<PcsQueryPlanSegmentError> for ProvePcsQueryPlanSegmentError {
    fn from(error: PcsQueryPlanSegmentError) -> Self {
        Self::Segment(error)
    }
}

impl From<PcsChallengeError> for ProvePcsQueryPlanSegmentError {
    fn from(error: PcsChallengeError) -> Self {
        Self::Challenge(error)
    }
}

impl From<PcsTranscriptError> for ProvePcsQueryPlanSegmentError {
    fn from(error: PcsTranscriptError) -> Self {
        Self::Transcript(error)
    }
}

impl From<PcsQueryNonceSegmentError> for ProvePcsQueryPlanSegmentError {
    fn from(error: PcsQueryNonceSegmentError) -> Self {
        Self::NonceSegment(error)
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

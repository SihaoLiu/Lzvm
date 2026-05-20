use std::fmt;

use lzvm_artifacts::pcs_nonce_segment::{PcsQueryNonceSegmentError, PCS_QUERY_NONCE_SEGMENT_ID};
use lzvm_artifacts::pcs_query_segment::PcsQueryPlanSegmentError as PcsQueryPlanSegmentEncodeError;
use lzvm_artifacts::witness_segment::WitnessCommitmentSegmentError;

use crate::pcs_challenge::PcsChallengeError;
use crate::pcs_transcript::PcsTranscriptError;

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
    TranscriptWitnessUnitCountMismatch {
        expected: usize,
        found: usize,
    },
    TranscriptWitnessUnitMismatch {
        input_unit_index: u32,
        witness_unit_index: u32,
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
    Segment(PcsQueryPlanSegmentEncodeError),
    NonceSegment(PcsQueryNonceSegmentError),
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
            Self::TranscriptWitnessUnitCountMismatch { expected, found } => write!(
                f,
                "prove PCS transcript query plan expected {expected} witness unit, found {found}"
            ),
            Self::TranscriptWitnessUnitMismatch {
                input_unit_index,
                witness_unit_index,
            } => write!(
                f,
                "prove PCS transcript query plan witness unit {witness_unit_index} does not match input unit {input_unit_index}"
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
            | Self::TranscriptWitnessUnitCountMismatch { .. }
            | Self::TranscriptWitnessUnitMismatch { .. }
            | Self::QueryCountExceedsDomain { .. }
            | Self::MissingTranscriptArity { .. }
            | Self::InvalidNonceSegmentId { .. }
            | Self::QueryNonceMismatch { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<PcsQueryPlanSegmentEncodeError> for ProvePcsQueryPlanSegmentError {
    fn from(error: PcsQueryPlanSegmentEncodeError) -> Self {
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

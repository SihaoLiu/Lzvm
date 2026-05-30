use std::fmt;

use lzvm_artifacts::pcs_fri_segment::PcsFriOpeningSegmentError;
use lzvm_field::FieldError;

use super::fold::PcsFriFoldError;
use super::merkle::PcsFriMerkleError;
use crate::pcs_query_plan::LoadPcsQueryPlanSegmentError;
use crate::pcs_transcript::PcsTranscriptError;
use crate::pcs_transcript_segments::PcsTranscriptProofSegmentsError;
use crate::verifier_query::VerifierFriQueryOutputSegmentsError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPcsFriOpeningSegmentError {
    MissingSegment,
    DuplicateSegment,
    Segment(PcsFriOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPcsFriOpeningUnitError {
    MissingSegment,
    DuplicateSegment,
    MissingUnit { unit_index: usize },
    UnexpectedUnit { unit_index: usize },
    UnitIndexOverflow,
    Segment(PcsFriOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatePcsFriOpeningSegmentsError {
    QueryPlan(LoadPcsQueryPlanSegmentError),
    Opening(LoadPcsFriOpeningSegmentError),
    Merkle {
        unit_index: usize,
        source: PcsFriMerkleError,
    },
    UnitCountMismatch,
    UnitMismatch {
        unit_index: usize,
    },
    UnitIndexOverflow,
    ArityOverflow,
    FinalLayerSizeOverflow,
    LayerSizeOverflow,
    FoldingWidthOverflow,
    LastLevelCountOverflow,
    LevelCountOverflow,
    InvalidTreeShape,
    FieldValue(FieldError),
    FieldDigest(FieldError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatePcsFriOpeningFoldUnitsError {
    Fold {
        unit_index: usize,
        source: PcsFriOpeningFoldError,
    },
    UnitMismatch {
        unit_index: usize,
    },
    UnitIndexOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateOptionalPcsFriOpeningProofSegmentsError {
    Opening(ValidatePcsFriOpeningSegmentsError),
    QueryPlan(LoadPcsQueryPlanSegmentError),
    OpeningSegment(LoadPcsFriOpeningSegmentError),
    UnboundOpeningSegment,
    Transcript(PcsTranscriptProofSegmentsError),
    Fold(ValidatePcsFriOpeningFoldUnitsError),
    VerifierQuery(VerifierFriQueryOutputSegmentsError),
}

impl fmt::Display for LoadPcsFriOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS FRI opening segment"),
            Self::DuplicateSegment => write!(f, "duplicate PCS FRI opening segment"),
            Self::Segment(error) => write!(f, "invalid PCS FRI opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadPcsFriOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment | Self::DuplicateSegment => None,
        }
    }
}

impl fmt::Display for LoadPcsFriOpeningUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS FRI opening segment"),
            Self::DuplicateSegment => write!(f, "duplicate PCS FRI opening segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "PCS FRI opening segment mismatch for unit {unit_index}")
            }
            Self::UnexpectedUnit { unit_index } => {
                write!(f, "unexpected PCS FRI opening segment unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "PCS FRI opening segment unit index overflow"),
            Self::Segment(error) => write!(f, "invalid PCS FRI opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadPcsFriOpeningUnitError {
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

impl From<LoadPcsFriOpeningSegmentError> for LoadPcsFriOpeningUnitError {
    fn from(error: LoadPcsFriOpeningSegmentError) -> Self {
        match error {
            LoadPcsFriOpeningSegmentError::MissingSegment => Self::MissingSegment,
            LoadPcsFriOpeningSegmentError::DuplicateSegment => Self::DuplicateSegment,
            LoadPcsFriOpeningSegmentError::Segment(error) => Self::Segment(error),
        }
    }
}

impl fmt::Display for ValidatePcsFriOpeningSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryPlan(error) => write!(f, "{error}"),
            Self::Opening(error) => write!(f, "{error}"),
            Self::Merkle { unit_index, source } => write!(
                f,
                "invalid PCS FRI opening segment for unit {unit_index}: {source}"
            ),
            Self::UnitCountMismatch => write!(f, "PCS FRI opening segment unit count mismatch"),
            Self::UnitMismatch { unit_index } => {
                write!(f, "PCS FRI opening segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "PCS FRI opening segment unit index overflow"),
            Self::ArityOverflow => write!(f, "PCS FRI opening segment arity overflow"),
            Self::FinalLayerSizeOverflow => {
                write!(f, "PCS FRI opening segment final layer size overflow")
            }
            Self::LayerSizeOverflow => write!(f, "PCS FRI opening segment layer size overflow"),
            Self::FoldingWidthOverflow => {
                write!(f, "PCS FRI opening segment folding width overflow")
            }
            Self::LastLevelCountOverflow => {
                write!(f, "PCS FRI opening segment last-level count overflow")
            }
            Self::LevelCountOverflow => write!(f, "PCS FRI opening segment level count overflow"),
            Self::InvalidTreeShape => write!(f, "PCS FRI opening segment invalid tree shape"),
            Self::FieldValue(error) => {
                write!(f, "invalid PCS FRI opening segment value: {error}")
            }
            Self::FieldDigest(error) => {
                write!(f, "invalid PCS FRI opening segment digest: {error}")
            }
        }
    }
}

impl std::error::Error for ValidatePcsFriOpeningSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Opening(error) => Some(error),
            Self::Merkle { source, .. } => Some(source),
            Self::FieldValue(error) | Self::FieldDigest(error) => Some(error),
            Self::UnitCountMismatch
            | Self::UnitMismatch { .. }
            | Self::UnitIndexOverflow
            | Self::ArityOverflow
            | Self::FinalLayerSizeOverflow
            | Self::LayerSizeOverflow
            | Self::FoldingWidthOverflow
            | Self::LastLevelCountOverflow
            | Self::LevelCountOverflow
            | Self::InvalidTreeShape => None,
        }
    }
}

impl fmt::Display for ValidatePcsFriOpeningFoldUnitsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fold { unit_index, source } => write!(
                f,
                "invalid PCS FRI opening segment for unit {unit_index}: {source}"
            ),
            Self::UnitMismatch { unit_index } => {
                write!(f, "PCS FRI opening segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "PCS FRI opening segment unit index overflow"),
        }
    }
}

impl std::error::Error for ValidatePcsFriOpeningFoldUnitsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fold { source, .. } => Some(source),
            Self::UnitMismatch { .. } | Self::UnitIndexOverflow => None,
        }
    }
}

impl fmt::Display for ValidateOptionalPcsFriOpeningProofSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Opening(error) => write!(f, "{error}"),
            Self::QueryPlan(error) => write!(f, "{error}"),
            Self::OpeningSegment(error) => write!(f, "{error}"),
            Self::UnboundOpeningSegment => {
                write!(
                    f,
                    "PCS FRI opening segment requires transcript query inputs"
                )
            }
            Self::Transcript(error) => write!(f, "{error}"),
            Self::Fold(error) => write!(f, "{error}"),
            Self::VerifierQuery(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ValidateOptionalPcsFriOpeningProofSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Opening(error) => Some(error),
            Self::QueryPlan(error) => Some(error),
            Self::OpeningSegment(error) => Some(error),
            Self::UnboundOpeningSegment => None,
            Self::Transcript(error) => Some(error),
            Self::Fold(error) => Some(error),
            Self::VerifierQuery(error) => Some(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriOpeningBuildError {
    EmptyFriLayers,
    QueryRowCountMismatch {
        expected: usize,
        found: usize,
    },
    InvalidLayerBits {
        layer_index: usize,
        input_bits: u32,
        output_bits: u32,
    },
    LayerInputMismatch {
        layer_index: usize,
        expected: u32,
        found: u32,
    },
    FoldingFactorMismatch {
        layer_index: usize,
        expected: usize,
        found: usize,
    },
    PolynomialLengthMismatch {
        layer_index: usize,
        expected: usize,
        found: usize,
    },
    FinalLayerMismatch {
        expected: u32,
        found: u32,
    },
    MissingChallenge {
        index: usize,
        len: usize,
    },
    UnsupportedDomainBits {
        bits: u32,
    },
    Merkle(PcsFriMerkleError),
    Fold(PcsFriFoldError),
    LengthOverflow,
}

impl fmt::Display for PcsFriOpeningBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFriLayers => write!(f, "PCS FRI opening build has no layers"),
            Self::QueryRowCountMismatch { expected, found } => write!(
                f,
                "PCS FRI opening build expected {expected} query rows, found {found}"
            ),
            Self::InvalidLayerBits {
                layer_index,
                input_bits,
                output_bits,
            } => write!(
                f,
                "PCS FRI opening build layer {layer_index} bits are invalid: input {input_bits}, output {output_bits}"
            ),
            Self::LayerInputMismatch {
                layer_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening build layer {layer_index} input bits {found} do not match expected {expected}"
            ),
            Self::FoldingFactorMismatch {
                layer_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening build layer {layer_index} folding factor {found} does not match expected {expected}"
            ),
            Self::PolynomialLengthMismatch {
                layer_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening build layer {layer_index} expected polynomial length {expected}, found {found}"
            ),
            Self::FinalLayerMismatch { expected, found } => write!(
                f,
                "PCS FRI opening build final layer bits {found} do not match expected {expected}"
            ),
            Self::MissingChallenge { index, len } => write!(
                f,
                "PCS FRI opening build challenge index {index} is outside challenge count {len}"
            ),
            Self::UnsupportedDomainBits { bits } => write!(
                f,
                "PCS FRI opening build domain bits are unsupported: {bits}"
            ),
            Self::Merkle(error) => write!(f, "PCS FRI opening build Merkle error: {error}"),
            Self::Fold(error) => write!(f, "PCS FRI opening build fold error: {error}"),
            Self::LengthOverflow => write!(f, "PCS FRI opening build length overflow"),
        }
    }
}

impl std::error::Error for PcsFriOpeningBuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Merkle(error) => Some(error),
            Self::Fold(error) => Some(error),
            Self::EmptyFriLayers
            | Self::QueryRowCountMismatch { .. }
            | Self::InvalidLayerBits { .. }
            | Self::LayerInputMismatch { .. }
            | Self::FoldingFactorMismatch { .. }
            | Self::PolynomialLengthMismatch { .. }
            | Self::FinalLayerMismatch { .. }
            | Self::MissingChallenge { .. }
            | Self::UnsupportedDomainBits { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<PcsFriMerkleError> for PcsFriOpeningBuildError {
    fn from(error: PcsFriMerkleError) -> Self {
        Self::Merkle(error)
    }
}

impl From<PcsFriFoldError> for PcsFriOpeningBuildError {
    fn from(error: PcsFriFoldError) -> Self {
        Self::Fold(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriTranscriptCommitmentError {
    Transcript(PcsTranscriptError),
    Opening(PcsFriOpeningBuildError),
}

impl fmt::Display for PcsFriTranscriptCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transcript(error) => {
                write!(f, "PCS FRI transcript commitment failed: {error}")
            }
            Self::Opening(error) => write!(f, "PCS FRI transcript opening failed: {error}"),
        }
    }
}

impl std::error::Error for PcsFriTranscriptCommitmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transcript(error) => Some(error),
            Self::Opening(error) => Some(error),
        }
    }
}

impl From<PcsTranscriptError> for PcsFriTranscriptCommitmentError {
    fn from(error: PcsTranscriptError) -> Self {
        Self::Transcript(error)
    }
}

impl From<PcsFriOpeningBuildError> for PcsFriTranscriptCommitmentError {
    fn from(error: PcsFriOpeningBuildError) -> Self {
        Self::Opening(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriOpeningFoldError {
    UnitIndexMismatch {
        expected: u32,
        found: u32,
    },
    QueryRowCountMismatch {
        expected: usize,
        found: usize,
    },
    LayerCountMismatch {
        expected: usize,
        found: usize,
    },
    MissingLayer {
        layer_index: u32,
    },
    LayerQueryCountMismatch {
        layer_index: u32,
        expected: usize,
        found: usize,
    },
    LayerQueryRowMismatch {
        layer_index: u32,
        query_index: usize,
        expected: u64,
        found: u64,
    },
    MissingChallenge {
        index: usize,
        len: usize,
    },
    UnsupportedDomainBits {
        bits: u32,
    },
    LayerValueIndexOutOfRange {
        layer_index: u32,
        query_index: usize,
        value_index: usize,
        len: usize,
    },
    FinalIndexOutOfRange {
        query_index: usize,
        index: usize,
        len: usize,
    },
    NonCanonicalField {
        value: u64,
    },
    Fold(PcsFriFoldError),
    LengthOverflow,
}

impl fmt::Display for PcsFriOpeningFoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexMismatch { expected, found } => write!(
                f,
                "PCS FRI opening fold unit index {found} does not match expected {expected}"
            ),
            Self::QueryRowCountMismatch { expected, found } => write!(
                f,
                "PCS FRI opening fold expected {expected} query rows, found {found}"
            ),
            Self::LayerCountMismatch { expected, found } => write!(
                f,
                "PCS FRI opening fold expected {expected} layers, found {found}"
            ),
            Self::MissingLayer { layer_index } => {
                write!(f, "PCS FRI opening fold is missing layer {layer_index}")
            }
            Self::LayerQueryCountMismatch {
                layer_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening fold layer {layer_index} expected {expected} queries, found {found}"
            ),
            Self::LayerQueryRowMismatch {
                layer_index,
                query_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening fold layer {layer_index} query {query_index} row {found} does not match expected {expected}"
            ),
            Self::MissingChallenge { index, len } => write!(
                f,
                "PCS FRI opening fold challenge index {index} is outside challenge count {len}"
            ),
            Self::UnsupportedDomainBits { bits } => {
                write!(f, "unsupported PCS FRI opening fold domain bits: {bits}")
            }
            Self::LayerValueIndexOutOfRange {
                layer_index,
                query_index,
                value_index,
                len,
            } => write!(
                f,
                "PCS FRI opening fold layer {layer_index} query {query_index} value index {value_index} is outside value count {len}"
            ),
            Self::FinalIndexOutOfRange {
                query_index,
                index,
                len,
            } => write!(
                f,
                "PCS FRI opening fold query {query_index} final index {index} is outside final polynomial length {len}"
            ),
            Self::NonCanonicalField { value } => write!(
                f,
                "PCS FRI opening fold field value is not canonical: {value}"
            ),
            Self::Fold(error) => write!(f, "PCS FRI opening fold evaluation failed: {error}"),
            Self::LengthOverflow => write!(f, "PCS FRI opening fold length overflow"),
        }
    }
}

impl std::error::Error for PcsFriOpeningFoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fold(error) => Some(error),
            Self::UnitIndexMismatch { .. }
            | Self::QueryRowCountMismatch { .. }
            | Self::LayerCountMismatch { .. }
            | Self::MissingLayer { .. }
            | Self::LayerQueryCountMismatch { .. }
            | Self::LayerQueryRowMismatch { .. }
            | Self::MissingChallenge { .. }
            | Self::UnsupportedDomainBits { .. }
            | Self::LayerValueIndexOutOfRange { .. }
            | Self::FinalIndexOutOfRange { .. }
            | Self::NonCanonicalField { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<PcsFriFoldError> for PcsFriOpeningFoldError {
    fn from(error: PcsFriFoldError) -> Self {
        Self::Fold(error)
    }
}

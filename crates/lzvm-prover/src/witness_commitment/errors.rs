use std::fmt;

use lzvm_artifacts::witness_segment::{
    WitnessCommitmentSegmentError, WitnessCommitmentSegmentIdError,
};
use lzvm_field::{DomainError, FieldError};

#[cfg(feature = "cuda")]
use crate::gpu_setup::GpuSetupError;
use crate::merkle_hash::MerkleHashError;
use crate::witness_layout::WitnessTraceLayoutError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveWitnessSegmentError {
    UnsupportedTraceInstance {
        unit_index: usize,
        trace_instance_index: u32,
    },
    LengthOverflow,
    SegmentId(WitnessCommitmentSegmentIdError),
    Segment(WitnessCommitmentSegmentError),
}

impl fmt::Display for ProveWitnessSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTraceInstance {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "prove witness segment trace instance {trace_instance_index} for unit {unit_index} requires schedule unit count"
            ),
            Self::LengthOverflow => write!(f, "prove witness segment length overflow"),
            Self::SegmentId(error) => write!(f, "prove witness segment id failed: {error}"),
            Self::Segment(error) => write!(f, "prove witness segment encode failed: {error}"),
        }
    }
}

impl std::error::Error for ProveWitnessSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SegmentId(error) => Some(error),
            Self::Segment(error) => Some(error),
            Self::UnsupportedTraceInstance { .. } | Self::LengthOverflow => None,
        }
    }
}

impl From<WitnessCommitmentSegmentError> for ProveWitnessSegmentError {
    fn from(error: WitnessCommitmentSegmentError) -> Self {
        Self::Segment(error)
    }
}

impl From<WitnessCommitmentSegmentIdError> for ProveWitnessSegmentError {
    fn from(error: WitnessCommitmentSegmentIdError) -> Self {
        Self::SegmentId(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessStageLeafError {
    Domain(DomainError),
    Field(FieldError),
    #[cfg(feature = "cuda")]
    GpuSetup(GpuSetupError),
    #[cfg(feature = "cuda")]
    Accel(lzvm_accel::AccelError),
    #[cfg(feature = "cuda")]
    NonCanonicalDeviceWord,
    LengthOverflow,
}

impl fmt::Display for WitnessStageLeafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => write!(f, "witness stage leaf domain error: {error}"),
            Self::Field(error) => write!(f, "witness stage leaf field error: {error}"),
            #[cfg(feature = "cuda")]
            Self::GpuSetup(error) => write!(f, "witness stage leaf GPU setup error: {error}"),
            #[cfg(feature = "cuda")]
            Self::Accel(error) => write!(f, "witness stage leaf cuda error: {error}"),
            #[cfg(feature = "cuda")]
            Self::NonCanonicalDeviceWord => {
                write!(f, "witness stage leaf device canonicality check failed")
            }
            Self::LengthOverflow => write!(f, "witness stage leaf length overflow"),
        }
    }
}

impl std::error::Error for WitnessStageLeafError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::Field(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::GpuSetup(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::Accel(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::NonCanonicalDeviceWord => None,
            Self::LengthOverflow => None,
        }
    }
}

impl From<DomainError> for WitnessStageLeafError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<FieldError> for WitnessStageLeafError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

#[cfg(feature = "cuda")]
impl From<GpuSetupError> for WitnessStageLeafError {
    fn from(error: GpuSetupError) -> Self {
        Self::GpuSetup(error)
    }
}

#[cfg(feature = "cuda")]
impl From<lzvm_accel::AccelError> for WitnessStageLeafError {
    fn from(error: lzvm_accel::AccelError) -> Self {
        Self::Accel(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessStageCommitmentError {
    Field(FieldError),
    #[cfg(feature = "cuda")]
    Leaf(WitnessStageLeafError),
    InvalidLeafByteLength {
        expected: usize,
        found: usize,
    },
    InvalidLeafDigestCount {
        expected: usize,
        found: usize,
    },
    UnsupportedArity {
        arity: usize,
    },
    EmptyStage,
    SourceDeviceRetentionUnavailable {
        bytes: usize,
    },
    LengthOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessStageOpeningError {
    Field(FieldError),
    Commitment(WitnessStageCommitmentError),
    RowOutOfRange {
        row_index: u64,
        row_count: u64,
    },
    ZeroRows,
    ZeroColumns,
    EmptyValues,
    ExternalSourceUnavailable,
    InvalidTreeByteLength {
        expected: usize,
        found: usize,
    },
    InvalidSiblingCount {
        expected: usize,
        found: usize,
    },
    Context {
        operation: &'static str,
        source: Box<WitnessStageOpeningError>,
    },
    LengthOverflow,
}

impl fmt::Display for WitnessStageCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Field(error) => write!(f, "witness stage commitment field error: {error}"),
            #[cfg(feature = "cuda")]
            Self::Leaf(error) => write!(f, "witness stage commitment leaf error: {error}"),
            Self::InvalidLeafByteLength { expected, found } => write!(
                f,
                "invalid witness stage leaf byte length: expected {expected}, found {found}"
            ),
            Self::InvalidLeafDigestCount { expected, found } => write!(
                f,
                "invalid witness stage leaf digest count: expected {expected}, found {found}"
            ),
            Self::UnsupportedArity { arity } => {
                write!(f, "unsupported witness stage commitment arity: {arity}")
            }
            Self::EmptyStage => write!(f, "witness stage commitment has no rows"),
            Self::SourceDeviceRetentionUnavailable { bytes } => write!(
                f,
                "witness stage source device retention unavailable for {bytes} bytes"
            ),
            Self::LengthOverflow => write!(f, "witness stage commitment length overflow"),
        }
    }
}

impl fmt::Display for WitnessStageOpeningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Field(error) => write!(f, "witness stage opening field error: {error}"),
            Self::Commitment(error) => write!(f, "witness stage opening commitment error: {error}"),
            Self::RowOutOfRange {
                row_index,
                row_count,
            } => write!(
                f,
                "witness stage opening row index {row_index} is outside row count {row_count}"
            ),
            Self::ZeroRows => write!(f, "witness stage opening has no rows"),
            Self::ZeroColumns => write!(f, "witness stage opening has no columns"),
            Self::EmptyValues => write!(f, "witness stage opening has no values"),
            Self::ExternalSourceUnavailable => write!(
                f,
                "witness stage opening requires an external source provider"
            ),
            Self::InvalidTreeByteLength { expected, found } => write!(
                f,
                "invalid witness stage opening tree byte length: expected {expected}, found {found}"
            ),
            Self::InvalidSiblingCount { expected, found } => write!(
                f,
                "invalid witness stage opening sibling count: expected {expected}, found {found}"
            ),
            Self::Context { operation, source } => {
                write!(f, "witness stage opening {operation} failed: {source}")
            }
            Self::LengthOverflow => write!(f, "witness stage opening length overflow"),
        }
    }
}

impl std::error::Error for WitnessStageCommitmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::Leaf(error) => Some(error),
            Self::InvalidLeafByteLength { .. }
            | Self::InvalidLeafDigestCount { .. }
            | Self::UnsupportedArity { .. }
            | Self::EmptyStage
            | Self::SourceDeviceRetentionUnavailable { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl std::error::Error for WitnessStageOpeningError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field(error) => Some(error),
            Self::Commitment(error) => Some(error),
            Self::Context { source, .. } => Some(source),
            Self::RowOutOfRange { .. }
            | Self::ZeroRows
            | Self::ZeroColumns
            | Self::EmptyValues
            | Self::ExternalSourceUnavailable
            | Self::InvalidTreeByteLength { .. }
            | Self::InvalidSiblingCount { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl WitnessStageOpeningError {
    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn context(operation: &'static str, source: Self) -> Self {
        Self::Context {
            operation,
            source: Box::new(source),
        }
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn is_length_overflow(&self) -> bool {
        match self {
            Self::LengthOverflow => true,
            Self::Context { source, .. } => source.is_length_overflow(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WitnessStageOpeningError;

    #[test]
    fn contexted_length_overflow_keeps_length_overflow_classification() {
        let error = WitnessStageOpeningError::context(
            "outer",
            WitnessStageOpeningError::context("inner", WitnessStageOpeningError::LengthOverflow),
        );

        assert!(error.is_length_overflow());
    }

    #[test]
    fn contexted_non_length_error_is_not_length_overflow() {
        let error = WitnessStageOpeningError::context(
            "outer",
            WitnessStageOpeningError::RowOutOfRange {
                row_index: 4,
                row_count: 3,
            },
        );

        assert!(!error.is_length_overflow());
    }
}

impl From<FieldError> for WitnessStageCommitmentError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

#[cfg(feature = "cuda")]
impl From<WitnessStageLeafError> for WitnessStageCommitmentError {
    fn from(error: WitnessStageLeafError) -> Self {
        Self::Leaf(error)
    }
}

impl From<MerkleHashError> for WitnessStageCommitmentError {
    fn from(error: MerkleHashError) -> Self {
        match error {
            MerkleHashError::UnsupportedArity { arity } => Self::UnsupportedArity { arity },
            MerkleHashError::InvalidChildCount { .. } => Self::LengthOverflow,
            MerkleHashError::Field(error) => Self::Field(error),
            #[cfg(feature = "cuda")]
            MerkleHashError::Accel(error) => Self::Leaf(WitnessStageLeafError::Accel(error)),
            MerkleHashError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

impl From<FieldError> for WitnessStageOpeningError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

impl From<MerkleHashError> for WitnessStageOpeningError {
    fn from(error: MerkleHashError) -> Self {
        match error {
            MerkleHashError::UnsupportedArity { arity } => {
                Self::Commitment(WitnessStageCommitmentError::UnsupportedArity { arity })
            }
            MerkleHashError::InvalidChildCount { expected, found } => {
                Self::InvalidSiblingCount { expected, found }
            }
            MerkleHashError::Field(error) => Self::Field(error),
            #[cfg(feature = "cuda")]
            MerkleHashError::Accel(error) => Self::Commitment(WitnessStageCommitmentError::Leaf(
                WitnessStageLeafError::Accel(error),
            )),
            MerkleHashError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

impl From<WitnessStageCommitmentError> for WitnessStageOpeningError {
    fn from(error: WitnessStageCommitmentError) -> Self {
        Self::Commitment(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWitnessCommitmentSegmentsError {
    UnitCountOverflow,
    SegmentIdOverflow,
    UnitIndexOverflow,
    MissingSegment,
    DuplicateSegment {
        unit_index: usize,
    },
    UnexpectedSegment {
        unit_index: usize,
    },
    Segment {
        unit_index: usize,
        source: WitnessCommitmentSegmentError,
    },
    UnitMismatch {
        unit_index: usize,
    },
    RowCountMismatch {
        unit_index: usize,
    },
    ColumnCountOverflow,
    ColumnCountMismatch {
        unit_index: usize,
    },
    StageCountMismatch {
        unit_index: usize,
    },
    StageIndexOverflow,
    StageIndexMismatch {
        unit_index: usize,
    },
    ArityMismatch {
        unit_index: usize,
    },
    EmptyTree {
        unit_index: usize,
    },
}

impl fmt::Display for LoadWitnessCommitmentSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitCountOverflow => write!(f, "witness commitment segment unit count overflow"),
            Self::SegmentIdOverflow => write!(f, "witness commitment segment id overflow"),
            Self::UnitIndexOverflow => write!(f, "witness commitment segment unit index overflow"),
            Self::MissingSegment => write!(f, "missing witness commitment segment"),
            Self::DuplicateSegment { unit_index } => {
                write!(
                    f,
                    "duplicate witness commitment segment for unit {unit_index}"
                )
            }
            Self::UnexpectedSegment { unit_index } => {
                write!(
                    f,
                    "unexpected witness commitment segment for unit {unit_index}"
                )
            }
            Self::Segment { unit_index, source } => write!(
                f,
                "invalid witness commitment segment for unit {unit_index}: {source}"
            ),
            Self::UnitMismatch { unit_index } => {
                write!(
                    f,
                    "witness commitment segment unit mismatch for unit {unit_index}"
                )
            }
            Self::RowCountMismatch { unit_index } => write!(
                f,
                "witness commitment segment row count mismatch for unit {unit_index}"
            ),
            Self::ColumnCountOverflow => {
                write!(f, "witness commitment segment column count overflow")
            }
            Self::ColumnCountMismatch { unit_index } => write!(
                f,
                "witness commitment segment column count mismatch for unit {unit_index}"
            ),
            Self::StageCountMismatch { unit_index } => write!(
                f,
                "witness commitment segment stage count mismatch for unit {unit_index}"
            ),
            Self::StageIndexOverflow => {
                write!(f, "witness commitment segment stage index overflow")
            }
            Self::StageIndexMismatch { unit_index } => write!(
                f,
                "witness commitment segment stage index mismatch for unit {unit_index}"
            ),
            Self::ArityMismatch { unit_index } => write!(
                f,
                "witness commitment segment arity mismatch for unit {unit_index}"
            ),
            Self::EmptyTree { unit_index } => {
                write!(
                    f,
                    "witness commitment segment empty tree for unit {unit_index}"
                )
            }
        }
    }
}

impl std::error::Error for LoadWitnessCommitmentSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment { source, .. } => Some(source),
            Self::UnitCountOverflow
            | Self::SegmentIdOverflow
            | Self::UnitIndexOverflow
            | Self::MissingSegment
            | Self::DuplicateSegment { .. }
            | Self::UnexpectedSegment { .. }
            | Self::UnitMismatch { .. }
            | Self::RowCountMismatch { .. }
            | Self::ColumnCountOverflow
            | Self::ColumnCountMismatch { .. }
            | Self::StageCountMismatch { .. }
            | Self::StageIndexOverflow
            | Self::StageIndexMismatch { .. }
            | Self::ArityMismatch { .. }
            | Self::EmptyTree { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessTraceCommitmentError {
    Layout(WitnessTraceLayoutError),
    StageLeaf(WitnessStageLeafError),
    StageCommitment(WitnessStageCommitmentError),
    #[cfg(feature = "cuda")]
    KnownZeroSourceMismatch {
        stage_index: usize,
    },
    WorkerPanic,
    LengthOverflow,
}

impl fmt::Display for WitnessTraceCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(error) => write!(f, "witness trace commitment layout error: {error}"),
            Self::StageLeaf(error) => {
                write!(f, "witness trace commitment leaf error: {error}")
            }
            Self::StageCommitment(error) => {
                write!(f, "witness trace commitment tree error: {error}")
            }
            #[cfg(feature = "cuda")]
            Self::KnownZeroSourceMismatch { stage_index } => write!(
                f,
                "witness trace commitment known-zero source mismatch at stage {stage_index}"
            ),
            Self::WorkerPanic => write!(f, "witness trace commitment worker panicked"),
            Self::LengthOverflow => write!(f, "witness trace commitment length overflow"),
        }
    }
}

impl std::error::Error for WitnessTraceCommitmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Layout(error) => Some(error),
            Self::StageLeaf(error) => Some(error),
            Self::StageCommitment(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::KnownZeroSourceMismatch { .. } => None,
            Self::WorkerPanic | Self::LengthOverflow => None,
        }
    }
}

impl From<WitnessTraceLayoutError> for WitnessTraceCommitmentError {
    fn from(error: WitnessTraceLayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<WitnessStageLeafError> for WitnessTraceCommitmentError {
    fn from(error: WitnessStageLeafError) -> Self {
        Self::StageLeaf(error)
    }
}

impl From<WitnessStageCommitmentError> for WitnessTraceCommitmentError {
    fn from(error: WitnessStageCommitmentError) -> Self {
        Self::StageCommitment(error)
    }
}

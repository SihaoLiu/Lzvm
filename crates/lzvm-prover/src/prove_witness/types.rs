use std::fmt;

use lzvm_artifacts::constant_opening_segment::ConstantOpeningSegmentError;
use lzvm_artifacts::constant_tree::ConstantTreeError;
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_evaluation_segment::PcsEvaluationSegmentError;
use lzvm_artifacts::pcs_material_segment::PcsMaterialManifestSegmentError;
use lzvm_artifacts::pcs_query_segment::PcsQueryPlanSegmentError;
use lzvm_artifacts::witness_opening_segment::WitnessOpeningSegmentError;
use lzvm_field::Ext3;

use crate::constant_tree_opening::ConstantTreeOpeningError;
use crate::witness_commitment::WitnessStageOpeningError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsEvaluationValues {
    pub unit_index: usize,
    pub values: Vec<Ext3>,
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

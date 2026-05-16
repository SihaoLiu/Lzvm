use std::fmt;

use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::hint_program::HintProgram;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt};

use crate::group_values::LoadGroupValuesSegmentError;
use crate::pcs_transcript_segments::PcsTranscriptProofSegmentsError;
use crate::proof_values::{LoadPcsProofValuesSegmentError, ProvePcsProofValuesSegmentError};
use crate::ProveSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHintValue {
    pub payload: ResolvedHintPayload,
    pub positions: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHintField {
    pub name: String,
    pub values: Vec<ResolvedHintValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHint {
    pub name: String,
    pub fields: Vec<ResolvedHintField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedHintPayload {
    Scalar(Felt),
    Extension(Ext3),
    Text(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GlobalHintInputRequirements {
    pub publics: bool,
    pub proof_values: bool,
    pub challenges: bool,
    pub group_values: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RegularHintInputRequirements {
    pub fixed_columns: bool,
    pub stage_columns: bool,
    pub publics: bool,
    pub unit_values: bool,
    pub proof_values: bool,
    pub group_values: bool,
    pub challenges: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ResolveGlobalHintProofSegmentsRequest<'a> {
    pub global_info: &'a GlobalInfo,
    pub program: &'a HintProgram,
    pub schedule: &'a ProveSchedule,
    pub public_values: &'a [Felt],
    pub segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintEvalError {
    EmptyDomain,
    MissingHint {
        index: usize,
        count: usize,
    },
    MissingField {
        hint_index: usize,
        name: String,
    },
    UnsupportedOperand {
        operand: &'static str,
    },
    NonCanonicalNumber {
        value: u64,
    },
    SourceIndexOutOfRange {
        source: &'static str,
        index: usize,
        width: usize,
        len: usize,
    },
    GroupIndexOutOfRange {
        group_id: usize,
        group_count: usize,
    },
    RowIndexOutOfRange {
        row: usize,
        domain_size: usize,
    },
    MissingColumn {
        source: &'static str,
        id: u32,
    },
    MissingStageColumns {
        stage_index: u16,
    },
    MatrixLengthMismatch {
        source: &'static str,
        expected: usize,
        found: usize,
    },
    UnsupportedDimension {
        source: &'static str,
        dimension: u32,
    },
    LengthOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveGlobalHintProofSegmentsError {
    ProofValues(LoadPcsProofValuesSegmentError),
    PackedProofValues(ProvePcsProofValuesSegmentError),
    Transcript(PcsTranscriptProofSegmentsError),
    GroupValues(LoadGroupValuesSegmentError),
    Eval(HintEvalError),
}

impl fmt::Display for HintEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDomain => write!(f, "hint evaluation domain is empty"),
            Self::MissingHint { index, count } => {
                write!(f, "hint index {index} is outside hint count {count}")
            }
            Self::MissingField { hint_index, name } => {
                write!(f, "hint {hint_index} has no field {name}")
            }
            Self::UnsupportedOperand { operand } => {
                write!(f, "unsupported hint operand: {operand}")
            }
            Self::NonCanonicalNumber { value } => {
                write!(f, "non-canonical hint number: {value}")
            }
            Self::SourceIndexOutOfRange {
                source,
                index,
                width,
                len,
            } => write!(
                f,
                "hint {source} index {index} with width {width} is outside length {len}"
            ),
            Self::GroupIndexOutOfRange {
                group_id,
                group_count,
            } => write!(
                f,
                "hint group index {group_id} is outside group count {group_count}"
            ),
            Self::RowIndexOutOfRange { row, domain_size } => write!(
                f,
                "hint row index {row} is outside domain size {domain_size}"
            ),
            Self::MissingColumn { source, id } => {
                write!(f, "hint {source} column id {id} is not declared")
            }
            Self::MissingStageColumns { stage_index } => {
                write!(f, "hint stage columns missing for stage {stage_index}")
            }
            Self::MatrixLengthMismatch {
                source,
                expected,
                found,
            } => write!(
                f,
                "hint {source} matrix length mismatch: expected {expected}, found {found}"
            ),
            Self::UnsupportedDimension { source, dimension } => {
                write!(f, "unsupported hint {source} dimension: {dimension}")
            }
            Self::LengthOverflow => write!(f, "hint evaluation length overflow"),
        }
    }
}

impl std::error::Error for HintEvalError {}

impl fmt::Display for ResolveGlobalHintProofSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProofValues(error) => write!(f, "{error}"),
            Self::PackedProofValues(error) => {
                write!(f, "global hint proof values invalid: {error}")
            }
            Self::Transcript(error) => write!(f, "{error}"),
            Self::GroupValues(error) => write!(f, "{error}"),
            Self::Eval(error) => write!(f, "invalid global hint program: {error}"),
        }
    }
}

impl std::error::Error for ResolveGlobalHintProofSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ProofValues(error) => Some(error),
            Self::PackedProofValues(error) => Some(error),
            Self::Transcript(error) => Some(error),
            Self::GroupValues(error) => Some(error),
            Self::Eval(error) => Some(error),
        }
    }
}

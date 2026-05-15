use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, ConstantOpeningLevelSegment, ConstantOpeningQuerySegment,
    ConstantOpeningSegment, ConstantOpeningSegmentError, ConstantOpeningUnitSegment,
    CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::{read_constant_tree_file, ConstantTreeError};
use lzvm_artifacts::key_directory::{KeyDirectoryCatalog, KeyUnitKind};
use lzvm_artifacts::pcs_material_segment::{
    encode_pcs_material_manifest_segment, PcsMaterialManifestSegment,
    PcsMaterialManifestSegmentError, PcsMaterialManifestUnit, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_nonce_segment::{
    encode_pcs_query_nonce_segment, PcsQueryNonceSegment, PcsQueryNonceSegmentError,
    PCS_QUERY_NONCE_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, parse_pcs_query_plan_segment, PcsQueryPlanSegment,
    PcsQueryPlanSegmentError, PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, WitnessOpeningLevelSegment, WitnessOpeningQuerySegment,
    WitnessOpeningSegment, WitnessOpeningSegmentError, WitnessOpeningStageSegment,
    WitnessOpeningUnitSegment, WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, parse_witness_commitment_segment, WitnessCommitmentSegment,
    WitnessCommitmentSegmentError, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::constant_tree_opening::{open_constant_tree_row, ConstantTreeOpeningError};
#[cfg(not(feature = "cuda"))]
use crate::pcs_challenge::find_query_nonce;
#[cfg(feature = "cuda")]
use crate::pcs_challenge::find_query_nonce_cuda;
use crate::pcs_challenge::{derive_fri_queries, verify_query_nonce, PcsChallengeError};
use crate::witness_commitment::{
    commit_witness_trace_stages, open_witness_stage_commitment, WitnessStageOpeningError,
    WitnessTraceCommitmentError, WitnessTraceCommitments,
};
use crate::witness_layout::{derive_witness_trace_layout, WitnessTraceLayoutError};
use crate::witness_loader::{load_witness_library, WitnessLoadError};
use crate::witness_runner::{run_witness_trace, WitnessTraceRunError};
use crate::{ProveExecutionPlan, ProvePassRequest, ProveSchedule};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveWitnessCommitments {
    unit_index: usize,
    input_byte_count: usize,
    trace_rows: usize,
    trace_columns: usize,
    stage_commitments: WitnessTraceCommitments,
}

impl ProveWitnessCommitments {
    pub fn unit_index(&self) -> usize {
        self.unit_index
    }

    pub fn input_byte_count(&self) -> usize {
        self.input_byte_count
    }

    pub fn trace_row_count(&self) -> usize {
        self.trace_rows
    }

    pub fn trace_column_count(&self) -> usize {
        self.trace_columns
    }

    pub fn stage_commitments(&self) -> &WitnessTraceCommitments {
        &self.stage_commitments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveWitnessCommitmentError {
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    InputData {
        path: PathBuf,
        message: String,
    },
    WitnessLoad(WitnessLoadError),
    Layout(WitnessTraceLayoutError),
    WitnessRun(WitnessTraceRunError),
    Commit(WitnessTraceCommitmentError),
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
    QueryNonceMismatch {
        unit_index: usize,
        bits: u32,
    },
    Challenge(PcsChallengeError),
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

impl fmt::Display for ProveWitnessCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove witness commitment unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::InputData { path, message } => write!(
                f,
                "prove witness commitment input-data read failed: {}: {message}",
                path.display()
            ),
            Self::WitnessLoad(error) => {
                write!(f, "prove witness commitment library load failed: {error}")
            }
            Self::Layout(error) => write!(f, "prove witness commitment layout failed: {error}"),
            Self::WitnessRun(error) => write!(f, "prove witness commitment run failed: {error}"),
            Self::Commit(error) => write!(f, "prove witness commitment failed: {error}"),
        }
    }
}

impl std::error::Error for ProveWitnessCommitmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::WitnessLoad(error) => Some(error),
            Self::Layout(error) => Some(error),
            Self::WitnessRun(error) => Some(error),
            Self::Commit(error) => Some(error),
            Self::UnitIndexOutOfRange { .. } | Self::InputData { .. } => None,
        }
    }
}

impl From<WitnessLoadError> for ProveWitnessCommitmentError {
    fn from(error: WitnessLoadError) -> Self {
        Self::WitnessLoad(error)
    }
}

impl From<WitnessTraceLayoutError> for ProveWitnessCommitmentError {
    fn from(error: WitnessTraceLayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<WitnessTraceRunError> for ProveWitnessCommitmentError {
    fn from(error: WitnessTraceRunError) -> Self {
        Self::WitnessRun(error)
    }
}

impl From<WitnessTraceCommitmentError> for ProveWitnessCommitmentError {
    fn from(error: WitnessTraceCommitmentError) -> Self {
        Self::Commit(error)
    }
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
            Self::QueryNonceMismatch { unit_index, bits } => write!(
                f,
                "prove PCS query plan unit {unit_index} query nonce does not satisfy {bits} work bits"
            ),
            Self::Challenge(error) => write!(f, "prove PCS query plan challenge failed: {error}"),
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

impl std::error::Error for ProvePcsQueryPlanSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidWitnessSegment { source, .. } => Some(source),
            Self::Challenge(error) => Some(error),
            Self::Segment(error) => Some(error),
            Self::NonceSegment(error) => Some(error),
            Self::MissingWitnessSegments
            | Self::UnitIndexOutOfRange { .. }
            | Self::WitnessUnitMismatch { .. }
            | Self::QueryCountExceedsDomain { .. }
            | Self::MissingTranscriptArity { .. }
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

pub fn run_prove_witness_commitments(
    plan: &ProveExecutionPlan,
    unit_index: usize,
) -> Result<ProveWitnessCommitments, ProveWitnessCommitmentError> {
    let unit_count = plan.run_plan.schedule.units.len();
    let unit = plan.run_plan.schedule.units.get(unit_index).ok_or(
        ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index,
            unit_count,
        },
    )?;
    let input = read_witness_input(&plan.run_plan.pass)?;
    let input_byte_count = input.len();
    let library = load_witness_library(&plan.inputs.witness_library)?;
    let layout = derive_witness_trace_layout(unit)?;
    let trace = run_witness_trace(&library, layout.request(input))?;
    let trace_rows = trace.row_count();
    let trace_columns = trace.column_count();
    let stage_commitments = commit_witness_trace_stages(&trace, unit)?;

    Ok(ProveWitnessCommitments {
        unit_index,
        input_byte_count,
        trace_rows,
        trace_columns,
        stage_commitments,
    })
}

pub fn build_witness_commitment_segment(
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessSegmentError> {
    let unit_index =
        u32::try_from(output.unit_index()).map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
    let id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
        .checked_add(unit_index)
        .ok_or(ProveWitnessSegmentError::LengthOverflow)?;
    let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
    for commitment in output.stage_commitments().commitments() {
        let stage_index = u32::try_from(commitment.stage_index())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let arity = u32::try_from(commitment.arity())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let tree_byte_count = u64::try_from(commitment.tree_bytes().len())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        stages.push(WitnessCommitmentStageSegment {
            stage_index,
            arity,
            root: commitment.root().map(|value| value.to_u64()),
            tree_byte_count,
            tree_digest: Sha256::digest(commitment.tree_bytes()).into(),
        });
    }

    let segment = WitnessCommitmentSegment {
        unit_index,
        input_byte_count: u64::try_from(output.input_byte_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        trace_rows: u64::try_from(output.trace_row_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        trace_columns: u64::try_from(output.trace_column_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        stages,
    };
    Ok(ProofSegment {
        id,
        data: encode_witness_commitment_segment(&segment)?,
    })
}

pub fn build_pcs_material_manifest_segment(
    schedule: &ProveSchedule,
) -> Result<ProofSegment, ProvePcsMaterialSegmentError> {
    let mut units = Vec::with_capacity(schedule.units.len());
    for (unit_index, unit) in schedule.units.iter().enumerate() {
        let unit_index_u32 = u32::try_from(unit_index)
            .map_err(|_| ProvePcsMaterialSegmentError::UnitIndexOverflow { unit_index })?;
        units.push(PcsMaterialManifestUnit {
            unit_index: unit_index_u32,
            plan_digest: unit.pcs_material_plan_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            fixed_column_digest: unit.pcs_material_fixed_column_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_digest: unit.pcs_material_constant_tree_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_root: unit.pcs_material_constant_tree_root.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            fixed_byte_count: unit.pcs_material_fixed_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_byte_count: unit.pcs_material_constant_tree_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            leaf_byte_count: unit.pcs_material_leaf_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            node_byte_count: unit.pcs_material_node_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
        });
    }
    let manifest = PcsMaterialManifestSegment { units };
    Ok(ProofSegment {
        id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        data: encode_pcs_material_manifest_segment(&manifest)?,
    })
}

pub fn build_pcs_query_plan_segment(
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    material_segment: &ProofSegment,
    witness_segments: &[ProofSegment],
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let witness_segments = sorted_witness_commitment_segments(witness_segments)?;

    let mut hasher = Sha256::new();
    hasher.update(b"lzvm-pcs-query-plan-v1");
    hasher.update(schedule.setup_hash);
    hasher.update(public_values_hash);
    hash_proof_segment(&mut hasher, material_segment)?;
    for segment in &witness_segments {
        hash_proof_segment(&mut hasher, segment)?;
    }
    let seed: [u8; 32] = hasher.finalize().into();

    let query_units = collect_witness_query_units(schedule, &witness_segments)?;
    let mut units = Vec::with_capacity(query_units.len());
    for (unit_index_u32, unit) in query_units {
        units.push(PcsQueryPlanUnit {
            unit_index: unit_index_u32,
            queries: derive_unit_queries(
                &seed,
                unit_index_u32,
                unit.query_count,
                unit.extended_domain_size,
            )?,
        });
    }

    let query_plan = PcsQueryPlanSegment { units };
    Ok(ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&query_plan)?,
    })
}

pub fn build_pcs_query_nonce_segment(
    schedule: &ProveSchedule,
    challenge: Ext3,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let bits = schedule
        .units
        .iter()
        .map(|unit| unit.proof_of_work_bits)
        .max()
        .unwrap_or(0);
    let nonce = find_query_nonce_with_available_backend(challenge, bits)?;
    let segment = PcsQueryNonceSegment {
        nonce: nonce.to_u64(),
    };
    Ok(ProofSegment {
        id: PCS_QUERY_NONCE_SEGMENT_ID,
        data: encode_pcs_query_nonce_segment(&segment)?,
    })
}

#[cfg(feature = "cuda")]
fn find_query_nonce_with_available_backend(
    challenge: Ext3,
    bits: u32,
) -> Result<Felt, ProvePcsQueryPlanSegmentError> {
    Ok(find_query_nonce_cuda(challenge, bits)?)
}

#[cfg(not(feature = "cuda"))]
fn find_query_nonce_with_available_backend(
    challenge: Ext3,
    bits: u32,
) -> Result<Felt, ProvePcsQueryPlanSegmentError> {
    Ok(find_query_nonce(challenge, bits)?)
}

pub fn build_pcs_query_plan_segment_from_challenge(
    schedule: &ProveSchedule,
    witness_segments: &[ProofSegment],
    challenge: Ext3,
    nonce: Felt,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let witness_segments = sorted_witness_commitment_segments(witness_segments)?;
    let query_units = collect_witness_query_units(schedule, &witness_segments)?;
    let mut units = Vec::with_capacity(query_units.len());
    for (unit_index_u32, unit) in query_units {
        let unit_index = usize::try_from(unit_index_u32)
            .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
        if !verify_query_nonce(challenge, nonce, unit.proof_of_work_bits)? {
            return Err(ProvePcsQueryPlanSegmentError::QueryNonceMismatch {
                unit_index,
                bits: unit.proof_of_work_bits,
            });
        }
        let arity = unit
            .transcript_arity
            .ok_or(ProvePcsQueryPlanSegmentError::MissingTranscriptArity { unit_index })?
            as usize;
        units.push(PcsQueryPlanUnit {
            unit_index: unit_index_u32,
            queries: derive_fri_queries(
                arity,
                challenge,
                nonce,
                unit.query_count as usize,
                unit.extended_domain_bits,
            )?,
        });
    }

    let query_plan = PcsQueryPlanSegment { units };
    Ok(ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&query_plan)?,
    })
}

pub fn build_witness_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let unit_index_u32 = u32::try_from(output.unit_index()).map_err(|_| {
        ProveWitnessOpeningSegmentError::UnitIndexOverflow {
            unit_index: output.unit_index(),
        }
    })?;
    let unit = schedule.units.get(output.unit_index()).ok_or(
        ProveWitnessOpeningSegmentError::UnitIndexOutOfRange {
            unit_index: output.unit_index(),
            unit_count: schedule.units.len(),
        },
    )?;
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let query_unit = query_plan
        .units
        .iter()
        .find(|unit| unit.unit_index == unit_index_u32)
        .ok_or(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: output.unit_index(),
        })?;
    let mut queries = Vec::with_capacity(query_unit.queries.len());
    for row_index in &query_unit.queries {
        let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
        for commitment in output.stage_commitments().commitments() {
            let stage_index = commitment.stage_index();
            let width = unit
                .stage_commit_widths
                .get(stage_index.checked_sub(1).ok_or(
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    },
                )?)
                .ok_or(ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                    stage_index,
                    stage_count: unit.stage_commit_widths.len(),
                })?;
            let opening = open_witness_stage_commitment(
                commitment,
                *row_index,
                unit.extended_domain_size,
                usize::try_from(*width).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    }
                })?,
            )?;
            stages.push(WitnessOpeningStageSegment {
                stage_index: u32::try_from(stage_index).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    }
                })?,
                values: opening
                    .values()
                    .iter()
                    .map(|value| value.to_u64())
                    .collect(),
                siblings: opening
                    .siblings()
                    .iter()
                    .map(|level| WitnessOpeningLevelSegment {
                        siblings: level
                            .iter()
                            .map(|digest| digest.map(|value| value.to_u64()))
                            .collect(),
                    })
                    .collect(),
            });
        }
        queries.push(WitnessOpeningQuerySegment {
            row_index: *row_index,
            stages,
        });
    }

    let segment = WitnessOpeningSegment {
        units: vec![WitnessOpeningUnitSegment {
            unit_index: unit_index_u32,
            queries,
        }],
    };
    Ok(ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: encode_witness_opening_segment(&segment)?,
    })
}

pub fn build_constant_opening_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
) -> Result<ProofSegment, ProveConstantOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let mut units = Vec::with_capacity(query_plan.units.len());
    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOverflow {
                unit_index: query_unit.unit_index,
            }
        })?;
        let schedule_unit = schedule.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let catalog_unit = catalog.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: catalog.units.len(),
            },
        )?;
        let tree = read_constant_tree_file(
            &catalog_unit.paths.constant_tree,
            &catalog_unit.metadata.setup,
        )
        .map_err(|source| ProveConstantOpeningSegmentError::ConstantTree { unit_index, source })?;
        let arity = usize::try_from(schedule_unit.merkle_tree_arity).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            }
        })?;
        let mut queries = Vec::with_capacity(query_unit.queries.len());
        for row_index in &query_unit.queries {
            let opening = open_constant_tree_row(&tree, *row_index, arity)?;
            queries.push(ConstantOpeningQuerySegment {
                row_index: *row_index,
                values: opening
                    .values()
                    .iter()
                    .map(|value| value.to_u64())
                    .collect(),
                siblings: opening
                    .siblings()
                    .iter()
                    .map(|level| ConstantOpeningLevelSegment {
                        siblings: level
                            .iter()
                            .map(|digest| digest.map(|value| value.to_u64()))
                            .collect(),
                    })
                    .collect(),
            });
        }
        units.push(ConstantOpeningUnitSegment {
            unit_index: query_unit.unit_index,
            queries,
        });
    }

    let segment = ConstantOpeningSegment { units };
    Ok(ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: encode_constant_opening_segment(&segment)?,
    })
}

fn sorted_witness_commitment_segments(
    witness_segments: &[ProofSegment],
) -> Result<Vec<ProofSegment>, ProvePcsQueryPlanSegmentError> {
    if witness_segments.is_empty() {
        return Err(ProvePcsQueryPlanSegmentError::MissingWitnessSegments);
    }
    let mut out = witness_segments.to_vec();
    out.sort_by_key(|segment| segment.id);
    Ok(out)
}

fn collect_witness_query_units<'a>(
    schedule: &'a ProveSchedule,
    witness_segments: &[ProofSegment],
) -> Result<Vec<(u32, &'a crate::ProveUnitSchedule)>, ProvePcsQueryPlanSegmentError> {
    let mut units = Vec::with_capacity(witness_segments.len());
    let mut seen_units = BTreeSet::new();
    for segment in witness_segments {
        let unit_index_u32 = segment
            .id
            .checked_sub(WITNESS_COMMITMENT_SEGMENT_BASE_ID)
            .ok_or(ProvePcsQueryPlanSegmentError::LengthOverflow)?;
        let unit_index = usize::try_from(unit_index_u32)
            .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
        let unit = schedule.units.get(unit_index).ok_or(
            ProvePcsQueryPlanSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let witness = parse_witness_commitment_segment(&segment.data).map_err(|source| {
            ProvePcsQueryPlanSegmentError::InvalidWitnessSegment { unit_index, source }
        })?;
        if witness.unit_index != unit_index_u32 {
            return Err(ProvePcsQueryPlanSegmentError::WitnessUnitMismatch {
                segment_unit_index: unit_index_u32,
                payload_unit_index: witness.unit_index,
            });
        }
        if !seen_units.insert(unit_index_u32) {
            return Err(ProvePcsQueryPlanSegmentError::Segment(
                PcsQueryPlanSegmentError::DuplicateUnitIndex {
                    unit_index: unit_index_u32,
                },
            ));
        }
        units.push((unit_index_u32, unit));
    }
    Ok(units)
}

fn derive_unit_queries(
    seed: &[u8; 32],
    unit_index: u32,
    query_count: u32,
    domain_size: u64,
) -> Result<Vec<u64>, ProvePcsQueryPlanSegmentError> {
    if u64::from(query_count) > domain_size {
        return Err(ProvePcsQueryPlanSegmentError::QueryCountExceedsDomain {
            unit_index: unit_index as usize,
            query_count,
            domain_size,
        });
    }
    let mut queries = Vec::with_capacity(query_count as usize);
    let mut seen = BTreeSet::new();
    let mask = domain_size
        .checked_sub(1)
        .ok_or(ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    let mut draw = 0_u64;
    while queries.len() < query_count as usize {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(unit_index.to_le_bytes());
        hasher.update(draw.to_le_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        let raw = u64::from_le_bytes(digest[..8].try_into().expect("slice length checked"));
        let query = raw & mask;
        if seen.insert(query) {
            queries.push(query);
        }
        draw = draw
            .checked_add(1)
            .ok_or(ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    }
    Ok(queries)
}

fn hash_proof_segment(
    hasher: &mut Sha256,
    segment: &ProofSegment,
) -> Result<(), ProvePcsQueryPlanSegmentError> {
    hasher.update(segment.id.to_le_bytes());
    let byte_count = u64::try_from(segment.data.len())
        .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    hasher.update(byte_count.to_le_bytes());
    hasher.update(Sha256::digest(&segment.data));
    Ok(())
}

fn read_witness_input(pass: &ProvePassRequest) -> Result<Vec<u8>, ProveWitnessCommitmentError> {
    match witness_input_path(pass) {
        Some(path) => std::fs::read(path).map_err(|error| ProveWitnessCommitmentError::InputData {
            path: path.to_path_buf(),
            message: error.to_string(),
        }),
        None => Ok(Vec::new()),
    }
}

fn witness_input_path(pass: &ProvePassRequest) -> Option<&Path> {
    match pass {
        ProvePassRequest::Contributions(partition) | ProvePassRequest::Full(partition) => {
            partition.input_data.as_deref()
        }
        ProvePassRequest::Internal { .. } => None,
    }
}

use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_material_segment::{
    encode_pcs_material_manifest_segment, PcsMaterialManifestSegment,
    PcsMaterialManifestSegmentError, PcsMaterialManifestUnit, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentSegmentError,
    WitnessCommitmentStageSegment, WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use sha2::{Digest, Sha256};

use crate::witness_commitment::{
    commit_witness_trace_stages, WitnessTraceCommitmentError, WitnessTraceCommitments,
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

use std::fmt;
use std::path::{Path, PathBuf};

use crate::witness_commitment::{
    commit_witness_trace_stages, WitnessTraceCommitmentError, WitnessTraceCommitments,
};
use crate::witness_layout::{derive_witness_trace_layout, WitnessTraceLayoutError};
use crate::witness_loader::{load_witness_library, WitnessLoadError};
use crate::witness_runner::{run_witness_trace, WitnessTraceRunError};
use crate::{ProveExecutionPlan, ProvePassRequest};

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

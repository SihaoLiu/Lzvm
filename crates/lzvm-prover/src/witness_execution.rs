use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::fixed::FixedColumns;
use lzvm_artifacts::hint_program::{source_unimplemented_hint_name, HintProgram};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValues, PublicValuesError};
use lzvm_artifacts::trace_bundle::TraceBundle;
use lzvm_field::{Ext3, Felt, FieldError};

use crate::fixed_material::FixedColumnsMaterialError;
use crate::fri_polynomial::{
    build_fri_domain_points, FriPolynomialError, FriPolynomialZerofierTable,
};
use crate::global_constraints::GlobalConstraintInputs;
use crate::hint_eval::{
    regular_hint_input_requirements, resolve_global_hint_program,
    resolve_regular_hint_program_for_row, HintEvalError,
};
use crate::regular_constraints::{
    evaluate_regular_constraints, RegularColumnMatrix, RegularConstraintEvalError,
    RegularConstraintInputs, RegularStageColumns,
};
use crate::source_assignment_hints::validate_source_assignment_hints;
use crate::source_lookup_hints::{SourceLookupBalance, SourceLookupHintError};
use crate::witness_commitment::{
    commit_witness_trace_stages_with_workers, WitnessTraceCommitmentError, WitnessTraceCommitments,
};
use crate::witness_layout::{
    derive_witness_trace_layout, WitnessTraceLayout, WitnessTraceLayoutError,
};
use crate::witness_loader::{
    load_witness_library, TraceBytesBackend, WitnessBackend, WitnessComputeContext,
    WitnessLoadError,
};
use crate::witness_runner::{run_witness_trace_with_context, WitnessTraceRunError};
use crate::witness_trace::WitnessTraceBuffer;
use crate::{ProveExecutionPlan, ProveExecutionUnitArtifacts, ProvePassRequest};

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
pub struct ProveWitnessTraceCommitments {
    commitments: ProveWitnessCommitments,
    trace: WitnessTraceBuffer,
    publics: Vec<Felt>,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
}

impl ProveWitnessTraceCommitments {
    pub fn commitments(&self) -> &ProveWitnessCommitments {
        &self.commitments
    }

    pub fn trace(&self) -> &WitnessTraceBuffer {
        &self.trace
    }

    pub fn publics(&self) -> &[Felt] {
        &self.publics
    }

    pub fn auxiliary_inputs(&self) -> &ProveWitnessAuxiliaryInputs {
        &self.auxiliary_inputs
    }

    pub fn into_commitments(self) -> ProveWitnessCommitments {
        self.commitments
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProveWitnessAuxiliaryInputs {
    pub unit_values: Vec<Felt>,
    pub proof_values: Vec<Felt>,
    pub group_values: Vec<Ext3>,
    pub challenges: Vec<Ext3>,
    pub evaluations: Vec<Ext3>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WitnessSharedInputs {
    input: Vec<u8>,
    publics: Vec<Felt>,
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
    MissingWitnessLibrary,
    PublicInputs {
        path: PathBuf,
        source: PublicValuesError,
    },
    PublicInputsSetupHashMismatch,
    PublicInputNonCanonical {
        index: usize,
        value: u64,
    },
    WitnessLoad(WitnessLoadError),
    Layout(WitnessTraceLayoutError),
    WitnessRun(WitnessTraceRunError),
    FixedColumns {
        unit_index: usize,
        path: PathBuf,
        source: Box<FixedColumnsMaterialError>,
    },
    FixedRowCountTooLarge {
        unit_index: usize,
        path: PathBuf,
        rows: u64,
    },
    FixedRowCountMismatch {
        unit_index: usize,
        path: PathBuf,
        expected: usize,
        found: usize,
    },
    FixedColumnCountMismatch {
        unit_index: usize,
        path: PathBuf,
        expected: usize,
        found: usize,
    },
    FixedColumnValueCountMismatch {
        unit_index: usize,
        path: PathBuf,
        column: String,
        expected: usize,
        found: usize,
    },
    FixedColumnValueCountOverflow {
        unit_index: usize,
        path: PathBuf,
    },
    FixedColumnNonCanonical {
        unit_index: usize,
        path: PathBuf,
        index: usize,
        value: u64,
    },
    StageIndexTooLarge {
        unit_index: usize,
        stage_index: usize,
    },
    MissingRegularConstraintInput {
        unit_index: usize,
        buffer: &'static str,
    },
    RegularConstraintDomainHelper {
        unit_index: usize,
        source: FriPolynomialError,
    },
    RegularConstraintEval(RegularConstraintEvalError),
    MissingRegularHintInput {
        unit_index: usize,
        source: &'static str,
    },
    RegularHintEval {
        unit_index: usize,
        source: HintEvalError,
    },
    GlobalHintEval {
        source: HintEvalError,
    },
    UnsupportedRegularHint {
        unit_index: usize,
        name: String,
    },
    SourceLookup {
        unit_index: usize,
        message: String,
    },
    SourceAssignment {
        unit_index: usize,
        message: String,
    },
    SourceLookupSet {
        message: String,
    },
    RegularConstraintViolation {
        unit_index: usize,
        constraint_index: usize,
        row: usize,
        value: [u64; 3],
    },
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
            Self::MissingWitnessLibrary => {
                write!(f, "prove witness commitment missing witness library")
            }
            Self::PublicInputs { path, source } => {
                write!(f, "read public inputs failed: {}: {source}", path.display())
            }
            Self::PublicInputsSetupHashMismatch => {
                write!(f, "public inputs setup hash mismatch")
            }
            Self::PublicInputNonCanonical { index, value } => write!(
                f,
                "public input field {index} is non-canonical: {value}"
            ),
            Self::WitnessLoad(error) => {
                write!(f, "prove witness commitment library load failed: {error}")
            }
            Self::Layout(error) => write!(f, "prove witness commitment layout failed: {error}"),
            Self::WitnessRun(error) => write!(f, "prove witness commitment run failed: {error}"),
            Self::FixedColumns {
                unit_index,
                path,
                source,
            } => write!(
                f,
                "prove witness commitment fixed columns failed for unit {unit_index}: {}: {source}",
                path.display()
            ),
            Self::FixedRowCountTooLarge {
                unit_index,
                path,
                rows,
            } => write!(
                f,
                "prove witness commitment fixed-column row count is too large for unit {unit_index}: {}: {rows}",
                path.display()
            ),
            Self::FixedRowCountMismatch {
                unit_index,
                path,
                expected,
                found,
            } => write!(
                f,
                "prove witness commitment fixed-column row count mismatch for unit {unit_index}: {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnCountMismatch {
                unit_index,
                path,
                expected,
                found,
            } => write!(
                f,
                "prove witness commitment fixed-column count mismatch for unit {unit_index}: {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnValueCountMismatch {
                unit_index,
                path,
                column,
                expected,
                found,
            } => write!(
                f,
                "prove witness commitment fixed-column value count mismatch for unit {unit_index}: {}: {column}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnValueCountOverflow { unit_index, path } => write!(
                f,
                "prove witness commitment fixed-column value count overflow for unit {unit_index}: {}",
                path.display()
            ),
            Self::FixedColumnNonCanonical {
                unit_index,
                path,
                index,
                value,
            } => write!(
                f,
                "prove witness commitment fixed-column value is non-canonical for unit {unit_index}: {}: index {index}: {value}",
                path.display()
            ),
            Self::StageIndexTooLarge {
                unit_index,
                stage_index,
            } => write!(
                f,
                "prove witness commitment stage index does not fit u16 for unit {unit_index}: {stage_index}"
            ),
            Self::MissingRegularConstraintInput { unit_index, buffer } => write!(
                f,
                "missing regular constraint {buffer} input for prove witness commitment unit {unit_index}"
            ),
            Self::RegularConstraintDomainHelper { unit_index, source } => write!(
                f,
                "prove witness commitment regular constraint domain helper build failed for unit {unit_index}: {source}"
            ),
            Self::RegularConstraintEval(error) => {
                write!(f, "prove witness commitment regular constraint evaluation failed: {error}")
            }
            Self::MissingRegularHintInput { unit_index, source } => write!(
                f,
                "missing regular hint {source} input for prove witness commitment unit {unit_index}"
            ),
            Self::RegularHintEval { unit_index, source } => write!(
                f,
                "prove witness commitment regular hint evaluation failed for unit {unit_index}: {source}"
            ),
            Self::GlobalHintEval { source } => {
                write!(f, "prove witness commitment global hint evaluation failed: {source}")
            }
            Self::UnsupportedRegularHint { unit_index, name } => write!(
                f,
                "unsupported regular hint {name} for prove witness commitment unit {unit_index}"
            ),
            Self::SourceLookup {
                unit_index,
                message,
            } => write!(
                f,
                "source lookup validation failed for prove witness commitment unit {unit_index}: {message}"
            ),
            Self::SourceAssignment {
                unit_index,
                message,
            } => write!(
                f,
                "source assignment validation failed for prove witness commitment unit {unit_index}: {message}"
            ),
            Self::SourceLookupSet { message } => write!(
                f,
                "source lookup validation failed for prove witness commitment set: {message}"
            ),
            Self::RegularConstraintViolation {
                unit_index,
                constraint_index,
                row,
                value,
            } => write!(
                f,
                "prove witness commitment regular constraint {constraint_index} failed for unit {unit_index} at row {row}: {value:?}"
            ),
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
            Self::PublicInputs { source, .. } => Some(source),
            Self::FixedColumns { source, .. } => Some(source),
            Self::RegularConstraintDomainHelper { source, .. } => Some(source),
            Self::RegularConstraintEval(error) => Some(error),
            Self::RegularHintEval { source, .. } => Some(source),
            Self::GlobalHintEval { source } => Some(source),
            Self::Commit(error) => Some(error),
            Self::UnitIndexOutOfRange { .. }
            | Self::InputData { .. }
            | Self::MissingWitnessLibrary
            | Self::PublicInputsSetupHashMismatch
            | Self::PublicInputNonCanonical { .. }
            | Self::FixedRowCountTooLarge { .. }
            | Self::FixedRowCountMismatch { .. }
            | Self::FixedColumnCountMismatch { .. }
            | Self::FixedColumnValueCountMismatch { .. }
            | Self::FixedColumnValueCountOverflow { .. }
            | Self::FixedColumnNonCanonical { .. }
            | Self::StageIndexTooLarge { .. }
            | Self::MissingRegularConstraintInput { .. }
            | Self::MissingRegularHintInput { .. }
            | Self::UnsupportedRegularHint { .. }
            | Self::SourceLookup { .. }
            | Self::SourceAssignment { .. }
            | Self::SourceLookupSet { .. }
            | Self::RegularConstraintViolation { .. } => None,
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

impl From<RegularConstraintEvalError> for ProveWitnessCommitmentError {
    fn from(error: RegularConstraintEvalError) -> Self {
        Self::RegularConstraintEval(error)
    }
}

impl From<WitnessTraceCommitmentError> for ProveWitnessCommitmentError {
    fn from(error: WitnessTraceCommitmentError) -> Self {
        Self::Commit(error)
    }
}

impl From<SourceLookupHintError> for ProveWitnessCommitmentError {
    fn from(error: SourceLookupHintError) -> Self {
        match error {
            SourceLookupHintError::Unit {
                unit_index,
                message,
            } => Self::SourceLookup {
                unit_index,
                message,
            },
            SourceLookupHintError::Set { message } => Self::SourceLookupSet { message },
        }
    }
}

pub fn run_prove_witness_commitments(
    plan: &ProveExecutionPlan,
    unit_index: usize,
) -> Result<ProveWitnessCommitments, ProveWitnessCommitmentError> {
    run_prove_witness_commitments_with_auxiliary_inputs(
        plan,
        unit_index,
        ProveWitnessAuxiliaryInputs::default(),
    )
}

pub fn run_prove_witness_commitments_with_auxiliary_inputs(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
) -> Result<ProveWitnessCommitments, ProveWitnessCommitmentError> {
    run_prove_witness_commitments_with_trace(plan, unit_index, auxiliary_inputs)
        .map(ProveWitnessTraceCommitments::into_commitments)
}

pub fn run_prove_witness_commitments_with_trace(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let Some(witness_library) = &plan.inputs.witness_library else {
        return Err(ProveWitnessCommitmentError::MissingWitnessLibrary);
    };
    let library = load_witness_library(witness_library)?;
    run_prove_witness_commitments_with_trace_backend(plan, unit_index, auxiliary_inputs, &library)
}

/// Runs witness commitments with a caller-supplied witness backend.
pub fn run_prove_witness_commitments_with_trace_backend<B: WitnessBackend + ?Sized>(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    backend: &B,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let mut source_lookup_balance = SourceLookupBalance::default();
    validate_witness_unit_index(plan, unit_index)?;
    let shared_inputs = load_witness_shared_inputs(plan)?;
    let output = run_prove_witness_commitments_with_trace_backend_inner(
        plan,
        unit_index,
        &shared_inputs,
        auxiliary_inputs.clone(),
        backend,
        Some(&mut source_lookup_balance),
    )?;
    accumulate_witness_global_hints(
        plan,
        &shared_inputs.publics,
        &auxiliary_inputs,
        &mut source_lookup_balance,
    )?;
    source_lookup_balance.validate_all_units()?;
    Ok(output)
}

fn run_prove_witness_commitments_with_trace_backend_inner<B: WitnessBackend + ?Sized>(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    shared_inputs: &WitnessSharedInputs,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    backend: &B,
    source_lookup_balance: Option<&mut SourceLookupBalance>,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let unit_count = plan.run_plan.schedule.units.len();
    let unit = plan.run_plan.schedule.units.get(unit_index).ok_or(
        ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index,
            unit_count,
        },
    )?;
    let input_byte_count = shared_inputs.input.len();
    let layout = derive_witness_trace_layout(unit)?;
    let trace = run_witness_trace_with_context(
        backend,
        WitnessComputeContext {
            guest_image: Some(&plan.inputs.guest_image),
            guest_image_info: Some(&plan.guest_image_info),
        },
        layout.request(&shared_inputs.input[..]),
    )?;
    let execution_unit =
        plan.units
            .get(unit_index)
            .ok_or(ProveWitnessCommitmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: plan.units.len(),
            })?;
    validate_witness_regular_constraints(
        execution_unit,
        unit_index,
        &layout,
        &trace,
        &shared_inputs.publics,
        &auxiliary_inputs,
    )?;
    if let Some(source_lookup_balance) = source_lookup_balance {
        accumulate_witness_regular_hints(
            execution_unit,
            unit_index,
            &layout,
            &trace,
            &shared_inputs.publics,
            &auxiliary_inputs,
            source_lookup_balance,
        )?;
    } else {
        validate_witness_regular_hints(
            execution_unit,
            unit_index,
            &layout,
            &trace,
            &shared_inputs.publics,
            &auxiliary_inputs,
        )?;
    }
    let trace_rows = trace.row_count();
    let trace_columns = trace.column_count();
    let stage_commitments = commit_witness_trace_stages_with_workers(
        &trace,
        unit,
        plan.run_plan.gpu.witness_thread_pools,
    )?;

    let commitments = ProveWitnessCommitments {
        unit_index,
        input_byte_count,
        trace_rows,
        trace_columns,
        stage_commitments,
    };

    Ok(ProveWitnessTraceCommitments {
        commitments,
        trace,
        publics: shared_inputs.publics.clone(),
        auxiliary_inputs,
    })
}

pub fn run_prove_witness_commitments_for_all_units(
    plan: &ProveExecutionPlan,
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    backend: &(impl WitnessBackend + ?Sized),
) -> Result<Vec<ProveWitnessTraceCommitments>, String> {
    let mut outputs = Vec::with_capacity(plan.units.len());
    let mut source_lookup_balance = SourceLookupBalance::default();
    let shared_inputs = load_witness_shared_inputs(plan).map_err(|error| error.to_string())?;
    for unit_index in 0..plan.units.len() {
        let output = run_prove_witness_commitments_with_trace_backend_inner(
            plan,
            unit_index,
            &shared_inputs,
            auxiliary_inputs.clone(),
            backend,
            Some(&mut source_lookup_balance),
        )
        .map_err(|error| {
            format!("run witness commitments failed for unit {unit_index}: {error}")
        })?;
        outputs.push(output);
    }
    accumulate_witness_global_hints(
        plan,
        &shared_inputs.publics,
        auxiliary_inputs,
        &mut source_lookup_balance,
    )
    .map_err(|error| error.to_string())?;
    source_lookup_balance
        .validate_all_units()
        .map_err(|error| error.to_string())?;
    Ok(outputs)
}

pub fn run_prove_witness_commitments_for_all_units_with_trace_bundle(
    plan: &ProveExecutionPlan,
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    bundle: &TraceBundle,
) -> Result<Vec<ProveWitnessTraceCommitments>, String> {
    validate_trace_bundle_unit_set(plan.units.len(), bundle)?;
    let mut outputs = Vec::with_capacity(plan.units.len());
    let mut source_lookup_balance = SourceLookupBalance::default();
    let shared_inputs = load_witness_shared_inputs(plan).map_err(|error| error.to_string())?;
    for unit_index in 0..plan.units.len() {
        let unit_index_u32 = u32::try_from(unit_index)
            .map_err(|_| format!("trace bundle unit index is too large: {unit_index}"))?;
        let trace_bytes = bundle
            .trace_bytes_for_unit(unit_index_u32)
            .ok_or_else(|| format!("trace bundle is missing unit {unit_index}"))?;
        let backend = TraceBytesBackend::borrowed(trace_bytes);
        let output = run_prove_witness_commitments_with_trace_backend_inner(
            plan,
            unit_index,
            &shared_inputs,
            auxiliary_inputs.clone(),
            &backend,
            Some(&mut source_lookup_balance),
        )
        .map_err(|error| {
            format!("run witness commitments failed for unit {unit_index}: {error}")
        })?;
        outputs.push(output);
    }
    accumulate_witness_global_hints(
        plan,
        &shared_inputs.publics,
        auxiliary_inputs,
        &mut source_lookup_balance,
    )
    .map_err(|error| error.to_string())?;
    source_lookup_balance
        .validate_all_units()
        .map_err(|error| error.to_string())?;
    Ok(outputs)
}

fn validate_trace_bundle_unit_set(
    plan_unit_count: usize,
    bundle: &TraceBundle,
) -> Result<(), String> {
    for unit in &bundle.units {
        let unit_index = usize::try_from(unit.unit_index)
            .map_err(|_| format!("trace bundle unit index is too large: {}", unit.unit_index))?;
        if unit_index >= plan_unit_count {
            return Err(format!(
                "trace bundle has unexpected unit {}",
                unit.unit_index
            ));
        }
    }
    Ok(())
}

fn load_witness_shared_inputs(
    plan: &ProveExecutionPlan,
) -> Result<WitnessSharedInputs, ProveWitnessCommitmentError> {
    Ok(WitnessSharedInputs {
        input: read_witness_input(&plan.run_plan.pass)?,
        publics: load_public_inputs(plan)?,
    })
}

fn validate_witness_unit_index(
    plan: &ProveExecutionPlan,
    unit_index: usize,
) -> Result<(), ProveWitnessCommitmentError> {
    let unit_count = plan.run_plan.schedule.units.len();
    if unit_index >= unit_count {
        return Err(ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index,
            unit_count,
        });
    }
    Ok(())
}

fn load_public_inputs(plan: &ProveExecutionPlan) -> Result<Vec<Felt>, ProveWitnessCommitmentError> {
    let Some(path) = &plan.inputs.public_inputs else {
        return Ok(Vec::new());
    };
    let public_values = read_public_values_file(path).map_err(|source| {
        ProveWitnessCommitmentError::PublicInputs {
            path: path.clone(),
            source,
        }
    })?;
    if public_values.setup_hash != plan.run_plan.schedule.setup_hash {
        return Err(ProveWitnessCommitmentError::PublicInputsSetupHashMismatch);
    }
    public_values_to_fields(&public_values)
}

fn public_values_to_fields(
    public_values: &PublicValues,
) -> Result<Vec<Felt>, ProveWitnessCommitmentError> {
    public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied())
        .enumerate()
        .map(|(index, value)| {
            Felt::from_canonical(value).map_err(|error| match error {
                FieldError::NonCanonical { value } => {
                    ProveWitnessCommitmentError::PublicInputNonCanonical { index, value }
                }
            })
        })
        .collect()
}

fn accumulate_witness_global_hints(
    plan: &ProveExecutionPlan,
    publics: &[Felt],
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    source_lookup_balance: &mut SourceLookupBalance,
) -> Result<(), ProveWitnessCommitmentError> {
    if plan.global_hints.hints.is_empty() {
        return Ok(());
    }
    let resolved = resolve_global_hint_program(
        &plan.global_info,
        &plan.global_hints,
        GlobalConstraintInputs {
            publics,
            proof_values: &auxiliary_inputs.proof_values,
            challenges: &auxiliary_inputs.challenges,
            group_values: &auxiliary_inputs.group_values,
        },
    )
    .map_err(|source| ProveWitnessCommitmentError::GlobalHintEval { source })?;
    source_lookup_balance.absorb(0, 0, &resolved)?;
    Ok(())
}

fn validate_witness_regular_constraints(
    plan_unit: &ProveExecutionUnitArtifacts,
    unit_index: usize,
    layout: &WitnessTraceLayout,
    trace: &WitnessTraceBuffer,
    publics: &[Felt],
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
) -> Result<(), ProveWitnessCommitmentError> {
    if plan_unit.regular_constraints.entries.is_empty() {
        return Ok(());
    }

    let material = load_witness_fixed_columns_material(unit_index, plan_unit)?;
    validate_fixed_columns_shape(
        &material.fixed_columns,
        plan_unit.fixed_column_count,
        layout.row_count(),
        unit_index,
        &plan_unit.fixed_columns,
    )?;

    let stage_traces = layout
        .stages()
        .iter()
        .map(|stage| layout.stage_trace(trace, stage.stage_index))
        .collect::<Result<Vec<_>, _>>()?;
    let mut stage_columns = Vec::with_capacity(stage_traces.len());
    for stage in &stage_traces {
        let stage_index = u16::try_from(stage.stage_index()).map_err(|_| {
            ProveWitnessCommitmentError::StageIndexTooLarge {
                unit_index,
                stage_index: stage.stage_index(),
            }
        })?;
        stage_columns.push(RegularStageColumns {
            stage_index,
            column_count: stage.column_count(),
            values: stage.values(),
        });
    }
    let domain_points =
        build_fri_domain_points(plan_unit.setup.stark.n_bits).map_err(|source| {
            ProveWitnessCommitmentError::RegularConstraintDomainHelper { unit_index, source }
        })?;
    let zerofiers = FriPolynomialZerofierTable::build(
        plan_unit.setup.stark.n_bits,
        plan_unit.setup.stark.n_bits,
        &plan_unit.setup.boundaries,
    )
    .map_err(
        |source| ProveWitnessCommitmentError::RegularConstraintDomainHelper { unit_index, source },
    )?;

    let results = evaluate_regular_constraints(
        &plan_unit.regular_constraints,
        RegularConstraintInputs {
            domain_size: layout.row_count(),
            stage_count: plan_unit.stage_count,
            fixed_columns: RegularColumnMatrix {
                column_count: plan_unit.fixed_column_count,
                values: &material.row_major_values,
            },
            stage_columns: &stage_columns,
            custom_fixed_columns: &[],
            opening_point_offsets: &plan_unit.opening_point_offsets,
            domain_points: &domain_points,
            zerofier_values: RegularColumnMatrix {
                column_count: zerofiers.column_count,
                values: &zerofiers.values,
            },
            publics,
            unit_values: &auxiliary_inputs.unit_values,
            proof_values: &auxiliary_inputs.proof_values,
            group_values: &auxiliary_inputs.group_values,
            challenges: &auxiliary_inputs.challenges,
            evaluations: &auxiliary_inputs.evaluations,
        },
    )
    .map_err(|error| map_regular_constraint_eval_error(unit_index, error))?;

    for result in results {
        if let Some(violation) = result.invalid_rows.first() {
            return Err(ProveWitnessCommitmentError::RegularConstraintViolation {
                unit_index,
                constraint_index: result.constraint_index,
                row: violation.row,
                value: violation.value.to_u64s(),
            });
        }
    }
    Ok(())
}

fn map_regular_constraint_eval_error(
    unit_index: usize,
    error: RegularConstraintEvalError,
) -> ProveWitnessCommitmentError {
    match error {
        RegularConstraintEvalError::SourceIndexOutOfRange { buffer, len: 0, .. }
            if is_regular_constraint_input_buffer(buffer) =>
        {
            ProveWitnessCommitmentError::MissingRegularConstraintInput { unit_index, buffer }
        }
        error => ProveWitnessCommitmentError::RegularConstraintEval(error),
    }
}

fn is_regular_constraint_input_buffer(buffer: &str) -> bool {
    matches!(
        buffer,
        "public" | "unit value" | "proof value" | "group value" | "challenge" | "evaluation"
    )
}

fn validate_witness_regular_hints(
    plan_unit: &ProveExecutionUnitArtifacts,
    unit_index: usize,
    layout: &WitnessTraceLayout,
    trace: &WitnessTraceBuffer,
    publics: &[Felt],
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
) -> Result<(), ProveWitnessCommitmentError> {
    let mut source_lookup_balance = SourceLookupBalance::default();
    accumulate_witness_regular_hints(
        plan_unit,
        unit_index,
        layout,
        trace,
        publics,
        auxiliary_inputs,
        &mut source_lookup_balance,
    )?;
    source_lookup_balance.validate(unit_index)?;
    Ok(())
}

fn accumulate_witness_regular_hints(
    plan_unit: &ProveExecutionUnitArtifacts,
    unit_index: usize,
    layout: &WitnessTraceLayout,
    trace: &WitnessTraceBuffer,
    publics: &[Felt],
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    source_lookup_balance: &mut SourceLookupBalance,
) -> Result<(), ProveWitnessCommitmentError> {
    if plan_unit.regular_hints.hints.is_empty() {
        return Ok(());
    }
    reject_unsupported_regular_hints(&plan_unit.regular_hints, unit_index)?;

    let requirements = regular_hint_input_requirements(&plan_unit.regular_hints);

    let fixed_material = if requirements.fixed_columns {
        let material = load_witness_fixed_columns_material(unit_index, plan_unit)?;
        validate_fixed_columns_shape(
            &material.fixed_columns,
            plan_unit.fixed_column_count,
            layout.row_count(),
            unit_index,
            &plan_unit.fixed_columns,
        )?;
        Some(material)
    } else {
        None
    };

    let stage_traces = if requirements.stage_columns {
        layout
            .stages()
            .iter()
            .map(|stage| layout.stage_trace(trace, stage.stage_index))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let mut stage_columns = Vec::with_capacity(stage_traces.len());
    for stage in &stage_traces {
        let stage_index = u16::try_from(stage.stage_index()).map_err(|_| {
            ProveWitnessCommitmentError::StageIndexTooLarge {
                unit_index,
                stage_index: stage.stage_index(),
            }
        })?;
        stage_columns.push(RegularStageColumns {
            stage_index,
            column_count: stage.column_count(),
            values: stage.values(),
        });
    }

    let fixed_columns =
        fixed_material
            .as_ref()
            .map_or_else(RegularColumnMatrix::default, |material| {
                RegularColumnMatrix {
                    column_count: plan_unit.fixed_column_count,
                    values: &material.row_major_values,
                }
            });

    for row in 0..layout.row_count() {
        let resolved = resolve_regular_hint_program_for_row(
            &plan_unit.setup,
            &plan_unit.regular_hints,
            row,
            RegularConstraintInputs {
                domain_size: layout.row_count(),
                stage_count: plan_unit.stage_count,
                fixed_columns,
                stage_columns: &stage_columns,
                custom_fixed_columns: &[],
                opening_point_offsets: &plan_unit.opening_point_offsets,
                domain_points: &[],
                zerofier_values: RegularColumnMatrix::default(),
                publics,
                unit_values: &auxiliary_inputs.unit_values,
                proof_values: &auxiliary_inputs.proof_values,
                group_values: &auxiliary_inputs.group_values,
                challenges: &auxiliary_inputs.challenges,
                evaluations: &auxiliary_inputs.evaluations,
            },
        )
        .map_err(|error| map_regular_hint_eval_error(unit_index, error))?;
        validate_source_assignment_hints(unit_index, row, &resolved)?;
        source_lookup_balance.absorb(unit_index, row, &resolved)?;
    }
    Ok(())
}

fn reject_unsupported_regular_hints(
    program: &HintProgram,
    unit_index: usize,
) -> Result<(), ProveWitnessCommitmentError> {
    if let Some(hint) = program
        .hints
        .iter()
        .find(|hint| source_unimplemented_hint_name(&hint.name))
    {
        return Err(ProveWitnessCommitmentError::UnsupportedRegularHint {
            unit_index,
            name: hint.name.clone(),
        });
    }
    Ok(())
}

fn map_regular_hint_eval_error(
    unit_index: usize,
    error: HintEvalError,
) -> ProveWitnessCommitmentError {
    match error {
        HintEvalError::SourceIndexOutOfRange { source, len: 0, .. }
            if is_regular_hint_input_source(source) =>
        {
            ProveWitnessCommitmentError::MissingRegularHintInput { unit_index, source }
        }
        source => ProveWitnessCommitmentError::RegularHintEval { unit_index, source },
    }
}

fn is_regular_hint_input_source(source: &str) -> bool {
    matches!(
        source,
        "public" | "unit value" | "proof value" | "unit group value" | "challenge" | "evaluation"
    )
}

fn load_witness_fixed_columns_material(
    unit_index: usize,
    plan_unit: &ProveExecutionUnitArtifacts,
) -> Result<crate::FixedColumnsMaterial, ProveWitnessCommitmentError> {
    crate::load_fixed_columns_material(
        &plan_unit.fixed_columns,
        &plan_unit.setup,
        plan_unit.group_name.clone(),
        plan_unit.unit_name.clone(),
    )
    .map_err(|source| ProveWitnessCommitmentError::FixedColumns {
        unit_index,
        path: plan_unit.fixed_columns.clone(),
        source: Box::new(source),
    })
}

fn validate_fixed_columns_shape(
    fixed_columns: &FixedColumns,
    fixed_column_count: usize,
    row_count: usize,
    unit_index: usize,
    path: &Path,
) -> Result<(), ProveWitnessCommitmentError> {
    let found_rows = usize::try_from(fixed_columns.row_count).map_err(|_| {
        ProveWitnessCommitmentError::FixedRowCountTooLarge {
            unit_index,
            path: path.to_path_buf(),
            rows: fixed_columns.row_count,
        }
    })?;
    if found_rows != row_count {
        return Err(ProveWitnessCommitmentError::FixedRowCountMismatch {
            unit_index,
            path: path.to_path_buf(),
            expected: row_count,
            found: found_rows,
        });
    }
    if fixed_columns.columns.len() != fixed_column_count {
        return Err(ProveWitnessCommitmentError::FixedColumnCountMismatch {
            unit_index,
            path: path.to_path_buf(),
            expected: fixed_column_count,
            found: fixed_columns.columns.len(),
        });
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::witness_layout::derive_witness_trace_layout;
    use crate::witness_trace::parse_witness_trace;
    use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
    use lzvm_artifacts::guest_image::{ElfClass, ElfEndian, GuestImageInfo};
    use lzvm_artifacts::hint_program::{
        Hint, HintField, HintOperand, HintValue, SOURCE_ASSIGNMENT_CHECK_HINT,
        SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT, SOURCE_UNSUPPORTED_ASSIGNMENT_HINT,
        SOURCE_UNSUPPORTED_CALL_HINT, SOURCE_UNSUPPORTED_CONSTRAINT_HINT,
        SOURCE_UNSUPPORTED_STATEMENT_HINT,
    };
    use lzvm_artifacts::key_directory::KeyUnitKind;
    use lzvm_artifacts::setup_info::{CommitmentColumn, FriStep, StarkStruct, UnitSetupInfo};
    use lzvm_artifacts::trace_bundle::TraceBundleUnit;

    #[test]
    fn rejects_trace_bundles_with_unexpected_units() {
        let bundle = TraceBundle {
            units: vec![
                TraceBundleUnit {
                    unit_index: 0,
                    trace_bytes: vec![1],
                },
                TraceBundleUnit {
                    unit_index: 2,
                    trace_bytes: vec![2],
                },
            ],
        };

        let error = validate_trace_bundle_unit_set(2, &bundle)
            .expect_err("trace bundle should not carry units outside the plan");

        assert_eq!(error, "trace bundle has unexpected unit 2");
    }

    #[test]
    fn accepts_source_lookup_regular_hints_at_unsupported_gate() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_LOOKUP_PROVES_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("lookup_proves(7, [value])".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        reject_unsupported_regular_hints(&program, 3)
            .expect("source lookup hints should reach semantic validation");
    }

    #[test]
    fn accepts_balanced_source_lookup_regular_hints() {
        let program = HintProgram {
            hints: vec![
                source_lookup_hint(
                    SOURCE_LOOKUP_PROVES_HINT,
                    "multiplicity",
                    HintOperand::Commitment {
                        id: 1,
                        row_offset_index: 0,
                    },
                ),
                source_lookup_hint(
                    SOURCE_LOOKUP_ASSUMES_HINT,
                    "selector",
                    HintOperand::Commitment {
                        id: 1,
                        row_offset_index: 0,
                    },
                ),
            ],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect("balanced lookup hints should validate");
    }

    #[test]
    fn accepts_balanced_source_lookup_weight_expressions() {
        let program = HintProgram {
            hints: vec![
                source_lookup_hint_with_weight_values(
                    SOURCE_LOOKUP_PROVES_HINT,
                    "multiplicity",
                    vec![
                        HintOperand::Commitment {
                            id: 1,
                            row_offset_index: 0,
                        },
                        HintOperand::Commitment {
                            id: 1,
                            row_offset_index: 0,
                        },
                        HintOperand::String("add".to_owned()),
                    ],
                ),
                source_lookup_hint_with_weight_values(
                    SOURCE_LOOKUP_ASSUMES_HINT,
                    "selector",
                    vec![
                        HintOperand::Number(2),
                        HintOperand::Commitment {
                            id: 1,
                            row_offset_index: 0,
                        },
                        HintOperand::String("mul".to_owned()),
                    ],
                ),
            ],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect("balanced lookup weight expressions should validate");
    }

    #[test]
    fn rejects_mismatched_source_assignment_regular_hints() {
        let program = HintProgram {
            hints: vec![source_assignment_hint(
                HintOperand::Commitment {
                    id: 0,
                    row_offset_index: 0,
                },
                HintOperand::Commitment {
                    id: 1,
                    row_offset_index: 0,
                },
            )],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        let error = validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect_err("mismatched assignment hint should reject");

        assert!(error.to_string().contains("source assignment"));
    }

    #[test]
    fn rejects_unbalanced_source_lookup_regular_hints() {
        let program = HintProgram {
            hints: vec![
                source_lookup_hint(
                    SOURCE_LOOKUP_PROVES_HINT,
                    "multiplicity",
                    HintOperand::Commitment {
                        id: 1,
                        row_offset_index: 0,
                    },
                ),
                source_lookup_hint(
                    SOURCE_LOOKUP_ASSUMES_HINT,
                    "selector",
                    HintOperand::Number(1),
                ),
            ],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        let error = validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect_err("unbalanced lookup hints should reject");

        assert!(matches!(
            error,
            ProveWitnessCommitmentError::SourceLookup { unit_index: 0, .. }
        ));
    }

    #[test]
    fn accepts_balanced_source_lookup_global_hints() {
        let plan = source_lookup_global_plan(HintProgram {
            hints: vec![
                source_lookup_global_hint(SOURCE_LOOKUP_PROVES_HINT),
                source_lookup_global_hint(SOURCE_LOOKUP_ASSUMES_HINT),
            ],
        });
        let mut balance = SourceLookupBalance::default();

        accumulate_witness_global_hints(
            &plan,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
            &mut balance,
        )
        .expect("balanced global lookup hints should accumulate");

        balance
            .validate_all_units()
            .expect("balanced global lookup hints should validate");
    }

    #[test]
    fn rejects_unbalanced_source_lookup_global_hints() {
        let plan = source_lookup_global_plan(HintProgram {
            hints: vec![source_lookup_global_hint(SOURCE_LOOKUP_PROVES_HINT)],
        });
        let mut balance = SourceLookupBalance::default();

        accumulate_witness_global_hints(
            &plan,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
            &mut balance,
        )
        .expect("global lookup hint should accumulate");

        let error = ProveWitnessCommitmentError::from(
            balance
                .validate_all_units()
                .expect_err("unbalanced global lookup hints should reject"),
        );

        assert!(matches!(
            error,
            ProveWitnessCommitmentError::SourceLookupSet { .. }
        ));
    }

    #[test]
    fn rejects_unsupported_source_call_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_UNSUPPORTED_CALL_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("source_protocol_call()".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        let error = reject_unsupported_regular_hints(&program, 5)
            .expect_err("unsupported source call hints should be rejected before evaluation");

        assert_eq!(
            error,
            ProveWitnessCommitmentError::UnsupportedRegularHint {
                unit_index: 5,
                name: SOURCE_UNSUPPORTED_CALL_HINT.to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unsupported_source_assignment_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_UNSUPPORTED_ASSIGNMENT_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("out[0] = value + 1".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        let error = reject_unsupported_regular_hints(&program, 7)
            .expect_err("unsupported source assignment hints should reject");

        assert_eq!(
            error,
            ProveWitnessCommitmentError::UnsupportedRegularHint {
                unit_index: 7,
                name: SOURCE_UNSUPPORTED_ASSIGNMENT_HINT.to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unsupported_source_statement_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_UNSUPPORTED_STATEMENT_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("for (...) { }".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        let error = reject_unsupported_regular_hints(&program, 9)
            .expect_err("unsupported source statement hints should be rejected before evaluation");

        assert_eq!(
            error,
            ProveWitnessCommitmentError::UnsupportedRegularHint {
                unit_index: 9,
                name: SOURCE_UNSUPPORTED_STATEMENT_HINT.to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unsupported_source_constraint_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_UNSUPPORTED_CONSTRAINT_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("value * (value - delayed) === 0".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        let error = reject_unsupported_regular_hints(&program, 11)
            .expect_err("unsupported source constraint hints should be rejected before evaluation");

        assert_eq!(
            error,
            ProveWitnessCommitmentError::UnsupportedRegularHint {
                unit_index: 11,
                name: SOURCE_UNSUPPORTED_CONSTRAINT_HINT.to_owned(),
            }
        );
    }

    fn source_lookup_hint(name: &str, weight_field: &str, weight_operand: HintOperand) -> Hint {
        source_lookup_hint_with_weight_values(name, weight_field, vec![weight_operand])
    }

    fn source_lookup_hint_with_weight_values(
        name: &str,
        weight_field: &str,
        weight_operands: Vec<HintOperand>,
    ) -> Hint {
        Hint {
            name: name.to_owned(),
            fields: vec![
                HintField {
                    name: "bus_id".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Number(7),
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: "values".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Commitment {
                            id: 0,
                            row_offset_index: 0,
                        },
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: weight_field.to_owned(),
                    values: weight_operands
                        .into_iter()
                        .map(|operand| HintValue {
                            operand,
                            positions: Vec::new(),
                        })
                        .collect(),
                },
            ],
        }
    }

    fn source_assignment_hint(target: HintOperand, value: HintOperand) -> Hint {
        Hint {
            name: SOURCE_ASSIGNMENT_CHECK_HINT.to_owned(),
            fields: vec![
                HintField {
                    name: "target".to_owned(),
                    values: vec![HintValue {
                        operand: target,
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: "value".to_owned(),
                    values: vec![HintValue {
                        operand: value,
                        positions: Vec::new(),
                    }],
                },
            ],
        }
    }

    fn source_lookup_global_hint(name: &str) -> Hint {
        Hint {
            name: name.to_owned(),
            fields: vec![
                HintField {
                    name: "bus_id".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Number(7),
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: "values".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Number(11),
                        positions: Vec::new(),
                    }],
                },
            ],
        }
    }

    fn source_lookup_global_plan(global_hints: HintProgram) -> ProveExecutionPlan {
        ProveExecutionPlan {
            run_plan: crate::ProveRunPlan {
                schedule: crate::ProveSchedule {
                    setup_hash: [0; 32],
                    unit_count: 1,
                    total_fixed_bytes: 0,
                    total_pcs_material_bytes: 0,
                    pcs_material_unit_count: 0,
                    total_query_count: 0,
                    max_extended_domain_bits: 0,
                    units: vec![source_lookup_schedule()],
                },
                pass: ProvePassRequest::Full(crate::ProvePartitionPlan::single()),
                options: crate::ProveRunOptions::default_for_output(PathBuf::from("out")),
                gpu: crate::GpuRunOptions::default(),
            },
            inputs: crate::ProveExecutionInputArtifacts {
                witness_library: None,
                guest_image: PathBuf::from("guest.elf"),
                public_inputs: None,
            },
            global_info: GlobalInfo {
                name: "program".to_owned(),
                air_groups: Vec::new(),
                airs: Vec::new(),
                curve: CurveKind::None,
                lattice_size: None,
                aggregation_types: Vec::new(),
                n_publics: 0,
                num_challenges: Vec::new(),
                num_proof_values: Vec::new(),
                proof_values_map: Vec::new(),
                publics_map: Vec::new(),
                transcript_arity: 4,
            },
            global_hints,
            witness_library_info: None,
            guest_image_info: GuestImageInfo {
                byte_len: 0,
                digest: [0; 32],
                elf_class: ElfClass::Elf64,
                endian: ElfEndian::Little,
                machine: 0,
                entry: 0,
                load_segments: Vec::new(),
            },
            program_image_cache: None,
            units: vec![source_lookup_plan_unit(HintProgram { hints: Vec::new() })],
        }
    }

    fn source_lookup_plan_unit(program: HintProgram) -> ProveExecutionUnitArtifacts {
        ProveExecutionUnitArtifacts {
            fixed_columns: PathBuf::from("fixed.bin"),
            expression_program: lzvm_artifacts::expression_program::ExpressionProgram {
                max_tmp1: 0,
                max_tmp3: 0,
                max_args: 0,
                max_ops: 0,
                entries: Vec::new(),
                ops: Vec::new(),
                args: Vec::new(),
                numbers: Vec::new(),
            },
            fri_expression_id: None,
            regular_constraints: lzvm_artifacts::constraint_program::ConstraintProgram {
                entries: Vec::new(),
                ops: Vec::new(),
                args: Vec::new(),
                numbers: Vec::new(),
            },
            regular_hints: program,
            setup: source_lookup_setup(),
            fixed_column_count: 0,
            stage_count: 1,
            opening_point_offsets: vec![0],
            group_name: "group".to_owned(),
            unit_name: "unit".to_owned(),
        }
    }

    fn source_lookup_setup() -> UnitSetupInfo {
        UnitSetupInfo {
            n_stages: 1,
            n_constants: 0,
            constant_columns: Vec::new(),
            n_publics: Some(0),
            n_constraints: Some(0),
            q_degree: 3,
            opening_points: vec![0],
            section_widths: std::collections::BTreeMap::new(),
            challenge_count: 0,
            eval_count: 0,
            evaluation_map: Vec::new(),
            boundaries: Vec::new(),
            commitment_columns: vec![
                CommitmentColumn {
                    name: "value".to_owned(),
                    stage: 1,
                    dimension: 1,
                    pols_map_id: 0,
                    stage_id: 0,
                    stage_position: 0,
                    intermediate: false,
                    lengths: Vec::new(),
                },
                CommitmentColumn {
                    name: "weight".to_owned(),
                    stage: 1,
                    dimension: 1,
                    pols_map_id: 1,
                    stage_id: 1,
                    stage_position: 1,
                    intermediate: false,
                    lengths: Vec::new(),
                },
            ],
            unit_value_map: Vec::new(),
            group_value_map: Vec::new(),
            stark: StarkStruct {
                n_bits: 1,
                n_bits_ext: 2,
                n_queries: 1,
                steps: vec![FriStep { n_bits: 2 }],
                hash_commits: true,
                last_level_verification: 1,
                pow_bits: 0,
                merkle_tree_arity: 4,
                verification_hash_type: Some("GL".to_owned()),
                transcript_arity: Some(4),
                merkle_tree_custom: Some(true),
            },
        }
    }

    fn source_lookup_schedule() -> crate::ProveUnitSchedule {
        crate::ProveUnitSchedule {
            kind: KeyUnitKind::Basic,
            group_id: Some(0),
            unit_id: Some(0),
            group_name: Some("group".to_owned()),
            unit_name: Some("unit".to_owned()),
            base_domain_bits: 1,
            extended_domain_bits: 2,
            base_domain_size: 2,
            extended_domain_size: 4,
            blowup_factor: 2,
            query_count: 1,
            proof_of_work_bits: 0,
            merkle_tree_arity: 4,
            last_level_verification: 1,
            transcript_arity: Some(4),
            hash_commits: true,
            transcript_root_challenge_draws: vec![2],
            challenge_count: 0,
            evaluation_value_count: 0,
            evaluation_map: Vec::new(),
            transcript_evaluation_challenge_draws: 0,
            constant_width: 0,
            stage_commit_widths: vec![2],
            commitment_columns: source_lookup_setup().commitment_columns,
            unit_value_map: Vec::new(),
            group_value_map: Vec::new(),
            opening_points: vec![0],
            fri_layers: Vec::new(),
            final_layer_bits: 0,
            fixed_bytes: 0,
            constant_tree_root: None,
            pcs_material_bytes: None,
            pcs_material_plan_digest: None,
            pcs_material_fixed_column_digest: None,
            pcs_material_constant_tree_digest: None,
            pcs_material_constant_tree_root: None,
            pcs_material_fixed_byte_count: None,
            pcs_material_constant_tree_byte_count: None,
            pcs_material_leaf_byte_count: None,
            pcs_material_node_byte_count: None,
        }
    }

    fn source_lookup_trace(values: &[u64]) -> WitnessTraceBuffer {
        let bytes = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        parse_witness_trace(&bytes, 2, 2).expect("trace should parse")
    }
}

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constraint_program::ConstraintProgram;
use lzvm_artifacts::expression_program::ExpressionProgram;
use lzvm_artifacts::guest_image::{read_guest_image_file, GuestImageError, GuestImageInfo};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, KeyDirectoryCatalog, KeyDirectoryError, KeyUnitKind,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::{CommitmentColumn, StageValue, UnitSetupInfo};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_artifacts::witness_library::{
    read_witness_library_file, WitnessLibraryError, WitnessLibraryInfo,
};

pub mod constant_tree_opening;
pub mod fri_polynomial;
pub mod global_constraints;
pub mod group_values;
pub mod hint_eval;
mod merkle_hash;
pub mod pcs_challenge;
pub mod pcs_fri;
pub mod pcs_transcript;
pub mod proof_values;
mod prove_fri_opening;
mod prove_fri_polynomial;
mod prove_witness;
pub mod regular_constraints;
pub mod unit_values;
pub mod verifier_eval;
pub mod verifier_query;
pub mod witness_commitment;
pub mod witness_layout;
pub mod witness_loader;
pub mod witness_runner;
pub mod witness_trace;

pub use prove_fri_opening::{
    build_pcs_fri_opening_segment, build_pcs_fri_opening_segment_from_trace,
    build_pcs_fri_opening_segment_from_transcript_values,
    build_pcs_fri_transcript_values_from_trace,
    build_pcs_fri_transcript_values_from_trace_segments, ProvePcsFriOpeningSegmentError,
    ProvePcsFriOpeningTraceSegmentError, ProvePcsFriOpeningTraceValues, ProvePcsFriOpeningValues,
    ProvePcsFriTranscriptTraceSegmentValues, ProvePcsFriTranscriptTraceValues,
    ProvePcsFriTranscriptTraceValuesError, ProvePcsFriTranscriptValues,
};
pub use prove_fri_polynomial::{build_pcs_fri_polynomial_values, ProvePcsFriPolynomialError};
pub use prove_witness::{
    build_constant_opening_segment, build_pcs_evaluation_segment,
    build_pcs_material_manifest_segment, build_pcs_query_nonce_segment,
    build_pcs_query_nonce_segment_from_transcript_segments, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_challenge,
    build_pcs_query_plan_segment_from_transcript_segments, build_witness_commitment_segment,
    build_witness_opening_segment, run_prove_witness_commitments,
    run_prove_witness_commitments_with_auxiliary_inputs, run_prove_witness_commitments_with_trace,
    ProveConstantOpeningSegmentError, ProvePcsEvaluationSegmentError, ProvePcsEvaluationValues,
    ProvePcsMaterialSegmentError, ProvePcsQueryPlanSegmentError, ProveWitnessAuxiliaryInputs,
    ProveWitnessCommitmentError, ProveWitnessCommitments, ProveWitnessOpeningSegmentError,
    ProveWitnessSegmentError, ProveWitnessTraceCommitments,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveSchedule {
    pub setup_hash: [u8; 32],
    pub unit_count: usize,
    pub total_fixed_bytes: u64,
    pub total_pcs_material_bytes: u64,
    pub pcs_material_unit_count: usize,
    pub total_query_count: u64,
    pub max_extended_domain_bits: u32,
    pub units: Vec<ProveUnitSchedule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveUnitSchedule {
    pub kind: KeyUnitKind,
    pub group_id: Option<usize>,
    pub unit_id: Option<usize>,
    pub group_name: Option<String>,
    pub unit_name: Option<String>,
    pub base_domain_bits: u32,
    pub extended_domain_bits: u32,
    pub base_domain_size: u64,
    pub extended_domain_size: u64,
    pub blowup_factor: u64,
    pub query_count: u32,
    pub proof_of_work_bits: u32,
    pub merkle_tree_arity: u32,
    pub last_level_verification: u32,
    pub transcript_arity: Option<u32>,
    pub hash_commits: bool,
    pub transcript_root_challenge_draws: Vec<usize>,
    pub challenge_count: usize,
    pub evaluation_value_count: usize,
    pub transcript_evaluation_challenge_draws: usize,
    pub constant_width: u32,
    pub stage_commit_widths: Vec<u32>,
    pub commitment_columns: Vec<CommitmentColumn>,
    pub unit_value_map: Vec<StageValue>,
    pub group_value_map: Vec<StageValue>,
    pub opening_points: Vec<i64>,
    pub fri_layers: Vec<PcsFriLayer>,
    pub final_layer_bits: u32,
    pub fixed_bytes: u64,
    pub constant_tree_root: Option<VerificationKeyRoot>,
    pub pcs_material_bytes: Option<u64>,
    pub pcs_material_plan_digest: Option<[u8; 32]>,
    pub pcs_material_fixed_column_digest: Option<[u8; 32]>,
    pub pcs_material_constant_tree_digest: Option<[u8; 32]>,
    pub pcs_material_constant_tree_root: Option<[u64; 4]>,
    pub pcs_material_fixed_byte_count: Option<u64>,
    pub pcs_material_constant_tree_byte_count: Option<u64>,
    pub pcs_material_leaf_byte_count: Option<u64>,
    pub pcs_material_node_byte_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveScheduleError {
    EmptyCatalog,
    LengthOverflow,
    KeyDirectory(KeyDirectoryError),
}

impl fmt::Display for ProveScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCatalog => write!(f, "prove schedule catalog is empty"),
            Self::LengthOverflow => write!(f, "prove schedule length overflow"),
            Self::KeyDirectory(error) => write!(f, "prove schedule catalog error: {error}"),
        }
    }
}

impl std::error::Error for ProveScheduleError {}

impl From<KeyDirectoryError> for ProveScheduleError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::KeyDirectory(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveRunRequest {
    pub pass: ProvePassRequest,
    pub options: ProveRunOptions,
    pub gpu: GpuRunOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveRunPlan {
    pub schedule: ProveSchedule,
    pub pass: ProvePassRequest,
    pub options: ProveRunOptions,
    pub gpu: GpuRunOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePassRequest {
    Contributions(ProvePartitionPlan),
    Internal { contribution_count: usize },
    Full(ProvePartitionPlan),
}

impl ProvePassRequest {
    pub fn kind(&self) -> ProvePassKind {
        match self {
            Self::Contributions(_) => ProvePassKind::Contributions,
            Self::Internal { .. } => ProvePassKind::Internal,
            Self::Full(_) => ProvePassKind::Full,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvePassKind {
    Contributions,
    Internal,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePartitionPlan {
    pub input_data: Option<PathBuf>,
    pub partition_count: usize,
    pub partition_ids: Vec<u32>,
    pub worker_index: usize,
}

impl ProvePartitionPlan {
    pub fn single() -> Self {
        Self {
            input_data: None,
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveRunOptions {
    pub aggregate: bool,
    pub remote_aggregation: bool,
    pub final_wrap: bool,
    pub verify_outputs: bool,
    pub save_outputs: bool,
    pub minimal_memory: bool,
    pub output_dir: PathBuf,
}

impl ProveRunOptions {
    pub fn default_for_output(output_dir: PathBuf) -> Self {
        Self {
            aggregate: false,
            remote_aggregation: false,
            final_wrap: false,
            verify_outputs: true,
            save_outputs: false,
            minimal_memory: false,
            output_dir,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRunOptions {
    pub preallocate: bool,
    pub max_streams: usize,
    pub witness_thread_pools: usize,
    pub max_stored_witnesses: usize,
    pub pack_trace: bool,
}

impl Default for GpuRunOptions {
    fn default() -> Self {
        Self {
            preallocate: false,
            max_streams: 20,
            witness_thread_pools: 4,
            max_stored_witnesses: 4,
            pack_trace: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveRunPlanError {
    Schedule(ProveScheduleError),
    PartitionCountZero,
    EmptyPartitionSet,
    PartitionOutOfRange {
        partition_id: u32,
        partition_count: usize,
    },
    WorkerOutOfRange {
        worker_index: usize,
        partition_count: usize,
    },
    EmptyContributionSet,
    EmptyOutputDirectory,
    AggregationRequired {
        option: &'static str,
    },
    InvalidGpuStreams,
    InvalidWitnessThreadPools,
    InvalidStoredWitnesses,
}

impl fmt::Display for ProveRunPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schedule(error) => write!(f, "prove run plan schedule error: {error}"),
            Self::PartitionCountZero => write!(f, "prove run plan partition count is zero"),
            Self::EmptyPartitionSet => write!(f, "prove run plan partition set is empty"),
            Self::PartitionOutOfRange {
                partition_id,
                partition_count,
            } => write!(
                f,
                "prove run plan partition id {partition_id} is outside partition count {partition_count}"
            ),
            Self::WorkerOutOfRange {
                worker_index,
                partition_count,
            } => write!(
                f,
                "prove run plan worker index {worker_index} is outside partition count {partition_count}"
            ),
            Self::EmptyContributionSet => write!(f, "prove run plan contribution set is empty"),
            Self::EmptyOutputDirectory => write!(f, "prove run plan output directory is empty"),
            Self::AggregationRequired { option } => {
                write!(f, "prove run plan option {option} requires aggregation")
            }
            Self::InvalidGpuStreams => write!(f, "prove run plan GPU stream count is invalid"),
            Self::InvalidWitnessThreadPools => {
                write!(f, "prove run plan witness thread-pool count is invalid")
            }
            Self::InvalidStoredWitnesses => {
                write!(f, "prove run plan stored witness count is invalid")
            }
        }
    }
}

impl std::error::Error for ProveRunPlanError {}

impl From<ProveScheduleError> for ProveRunPlanError {
    fn from(error: ProveScheduleError) -> Self {
        Self::Schedule(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveExecutionInputArtifacts {
    pub witness_library: PathBuf,
    pub guest_image: PathBuf,
    pub public_inputs: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveExecutionPlan {
    pub run_plan: ProveRunPlan,
    pub inputs: ProveExecutionInputArtifacts,
    pub witness_library_info: WitnessLibraryInfo,
    pub guest_image_info: GuestImageInfo,
    pub units: Vec<ProveExecutionUnitArtifacts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveExecutionUnitArtifacts {
    pub fixed_columns: PathBuf,
    pub expression_program: ExpressionProgram,
    pub fri_expression_id: Option<u32>,
    pub regular_constraints: ConstraintProgram,
    pub setup: UnitSetupInfo,
    pub fixed_column_count: usize,
    pub stage_count: u16,
    pub opening_point_offsets: Vec<i64>,
    pub group_name: String,
    pub unit_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveExecutionPlanError {
    RunPlan(ProveRunPlanError),
    MissingPcsMaterial {
        unit_index: usize,
        kind: KeyUnitKind,
    },
    MissingWitnessLibrary {
        path: PathBuf,
    },
    WitnessLibraryIsNotFile {
        path: PathBuf,
    },
    InvalidWitnessLibrary {
        path: PathBuf,
        source: WitnessLibraryError,
    },
    MissingGuestImage {
        path: PathBuf,
    },
    GuestImageIsNotFile {
        path: PathBuf,
    },
    InvalidGuestImage {
        path: PathBuf,
        source: GuestImageError,
    },
    MissingPublicInputs {
        path: PathBuf,
    },
    PublicInputsIsNotFile {
        path: PathBuf,
    },
    FixedColumnCountTooLarge {
        unit_index: usize,
        fixed_column_count: u32,
    },
    StageCountTooLarge {
        unit_index: usize,
        stage_count: u32,
    },
}

impl fmt::Display for ProveExecutionPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RunPlan(error) => write!(f, "prove execution plan run-plan error: {error}"),
            Self::MissingPcsMaterial { unit_index, kind } => write!(
                f,
                "prove execution plan PCS setup material is missing for unit {unit_index} ({kind})"
            ),
            Self::MissingWitnessLibrary { path } => {
                write!(
                    f,
                    "prove execution plan witness library is missing: {}",
                    path.display()
                )
            }
            Self::WitnessLibraryIsNotFile { path } => write!(
                f,
                "prove execution plan witness library is not a file: {}",
                path.display()
            ),
            Self::InvalidWitnessLibrary { path, source } => write!(
                f,
                "prove execution plan witness library is invalid: {}: {source}",
                path.display()
            ),
            Self::MissingGuestImage { path } => {
                write!(
                    f,
                    "prove execution plan guest image is missing: {}",
                    path.display()
                )
            }
            Self::GuestImageIsNotFile { path } => write!(
                f,
                "prove execution plan guest image is not a file: {}",
                path.display()
            ),
            Self::InvalidGuestImage { path, source } => write!(
                f,
                "prove execution plan guest image is invalid: {}: {source}",
                path.display()
            ),
            Self::MissingPublicInputs { path } => {
                write!(
                    f,
                    "prove execution plan public inputs are missing: {}",
                    path.display()
                )
            }
            Self::PublicInputsIsNotFile { path } => write!(
                f,
                "prove execution plan public inputs are not a file: {}",
                path.display()
            ),
            Self::FixedColumnCountTooLarge {
                unit_index,
                fixed_column_count,
            } => write!(
                f,
                "prove execution plan unit {unit_index} fixed column count does not fit usize: {fixed_column_count}"
            ),
            Self::StageCountTooLarge {
                unit_index,
                stage_count,
            } => write!(
                f,
                "prove execution plan unit {unit_index} stage count does not fit u16: {stage_count}"
            ),
        }
    }
}

impl std::error::Error for ProveExecutionPlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidWitnessLibrary { source, .. } => Some(source),
            Self::InvalidGuestImage { source, .. } => Some(source),
            Self::RunPlan(error) => Some(error),
            Self::MissingPcsMaterial { .. }
            | Self::FixedColumnCountTooLarge { .. }
            | Self::StageCountTooLarge { .. } => None,
            _ => None,
        }
    }
}

impl From<ProveRunPlanError> for ProveExecutionPlanError {
    fn from(error: ProveRunPlanError) -> Self {
        Self::RunPlan(error)
    }
}

pub fn derive_prove_schedule(
    catalog: &KeyDirectoryCatalog,
) -> Result<ProveSchedule, ProveScheduleError> {
    if catalog.units.is_empty() {
        return Err(ProveScheduleError::EmptyCatalog);
    }

    let setup_hash = key_directory_catalog_digest(catalog)?;
    let mut total_fixed_bytes = 0_u64;
    let mut total_pcs_material_bytes = 0_u64;
    let mut pcs_material_unit_count = 0_usize;
    let mut total_query_count = 0_u64;
    let mut max_extended_domain_bits = 0_u32;
    let mut units = Vec::with_capacity(catalog.units.len());
    for unit in &catalog.units {
        total_fixed_bytes = total_fixed_bytes
            .checked_add(unit.actual_fixed_bytes)
            .ok_or(ProveScheduleError::LengthOverflow)?;
        total_query_count = total_query_count
            .checked_add(u64::from(unit.pcs_plan.query_count))
            .ok_or(ProveScheduleError::LengthOverflow)?;
        if unit.pcs_material_present {
            pcs_material_unit_count += 1;
        }
        if let Some(bytes) = unit.pcs_material_bytes {
            total_pcs_material_bytes = total_pcs_material_bytes
                .checked_add(bytes)
                .ok_or(ProveScheduleError::LengthOverflow)?;
        }
        max_extended_domain_bits = max_extended_domain_bits.max(unit.pcs_plan.extended_domain_bits);
        let material = unit.pcs_material.as_ref();

        units.push(ProveUnitSchedule {
            kind: unit.paths.kind,
            group_id: unit.paths.group_id,
            unit_id: unit.paths.unit_id,
            group_name: unit.paths.group_name.clone(),
            unit_name: unit.paths.unit_name.clone(),
            base_domain_bits: unit.pcs_plan.base_domain_bits,
            extended_domain_bits: unit.pcs_plan.extended_domain_bits,
            base_domain_size: unit.pcs_plan.base_domain_size,
            extended_domain_size: unit.pcs_plan.extended_domain_size,
            blowup_factor: unit.pcs_plan.blowup_factor,
            query_count: unit.pcs_plan.query_count,
            proof_of_work_bits: unit.pcs_plan.proof_of_work_bits,
            merkle_tree_arity: unit.pcs_plan.merkle_tree_arity,
            last_level_verification: unit.metadata.setup.stark.last_level_verification,
            transcript_arity: unit.pcs_plan.transcript_arity,
            hash_commits: unit.pcs_plan.hash_commits,
            transcript_root_challenge_draws: derive_transcript_root_challenge_draws(
                unit.pcs_plan.stage_commit_widths.len(),
            ),
            challenge_count: unit.metadata.setup.challenge_count,
            evaluation_value_count: unit.metadata.setup.eval_count,
            transcript_evaluation_challenge_draws: 2,
            constant_width: unit.pcs_plan.constant_width,
            stage_commit_widths: unit.pcs_plan.stage_commit_widths.clone(),
            commitment_columns: unit.metadata.setup.commitment_columns.clone(),
            unit_value_map: unit.metadata.setup.unit_value_map.clone(),
            group_value_map: unit.metadata.setup.group_value_map.clone(),
            opening_points: unit.pcs_plan.opening_points.clone(),
            fri_layers: unit.pcs_plan.fri_layers.clone(),
            final_layer_bits: unit.pcs_plan.final_layer_bits,
            fixed_bytes: unit.actual_fixed_bytes,
            constant_tree_root: unit.constant_tree_root.clone(),
            pcs_material_bytes: unit.pcs_material_bytes,
            pcs_material_plan_digest: material.map(|value| value.plan_digest),
            pcs_material_fixed_column_digest: material.map(|value| value.fixed_column_digest),
            pcs_material_constant_tree_digest: material.map(|value| value.constant_tree_digest),
            pcs_material_constant_tree_root: material.map(|value| value.constant_tree_root),
            pcs_material_fixed_byte_count: material.map(|value| value.fixed_byte_count),
            pcs_material_constant_tree_byte_count: material
                .map(|value| value.constant_tree_byte_count),
            pcs_material_leaf_byte_count: material.map(|value| value.leaf_byte_count),
            pcs_material_node_byte_count: material.map(|value| value.node_byte_count),
        });
    }

    Ok(ProveSchedule {
        setup_hash,
        unit_count: units.len(),
        total_fixed_bytes,
        total_pcs_material_bytes,
        pcs_material_unit_count,
        total_query_count,
        max_extended_domain_bits,
        units,
    })
}

fn derive_transcript_root_challenge_draws(root_count: usize) -> Vec<usize> {
    let mut draws = vec![1; root_count];
    if let Some(first) = draws.first_mut() {
        *first = 2;
    }
    draws
}

pub fn derive_prove_execution_plan(
    catalog: &KeyDirectoryCatalog,
    request: ProveRunRequest,
    inputs: ProveExecutionInputArtifacts,
) -> Result<ProveExecutionPlan, ProveExecutionPlanError> {
    let run_plan = derive_prove_run_plan(catalog, request)?;
    validate_execution_pcs_material(&run_plan.schedule)?;
    validate_regular_file(
        &inputs.witness_library,
        |path| ProveExecutionPlanError::MissingWitnessLibrary { path },
        |path| ProveExecutionPlanError::WitnessLibraryIsNotFile { path },
    )?;
    let witness_library_info =
        read_witness_library_file(&inputs.witness_library).map_err(|source| {
            ProveExecutionPlanError::InvalidWitnessLibrary {
                path: inputs.witness_library.clone(),
                source,
            }
        })?;
    validate_regular_file(
        &inputs.guest_image,
        |path| ProveExecutionPlanError::MissingGuestImage { path },
        |path| ProveExecutionPlanError::GuestImageIsNotFile { path },
    )?;
    let guest_image_info = read_guest_image_file(&inputs.guest_image).map_err(|source| {
        ProveExecutionPlanError::InvalidGuestImage {
            path: inputs.guest_image.clone(),
            source,
        }
    })?;
    if let Some(public_inputs) = &inputs.public_inputs {
        validate_regular_file(
            public_inputs,
            |path| ProveExecutionPlanError::MissingPublicInputs { path },
            |path| ProveExecutionPlanError::PublicInputsIsNotFile { path },
        )?;
    }

    let units = derive_prove_execution_units(catalog)?;

    Ok(ProveExecutionPlan {
        run_plan,
        inputs,
        witness_library_info,
        guest_image_info,
        units,
    })
}

fn derive_prove_execution_units(
    catalog: &KeyDirectoryCatalog,
) -> Result<Vec<ProveExecutionUnitArtifacts>, ProveExecutionPlanError> {
    let mut units = Vec::with_capacity(catalog.units.len());
    for (unit_index, unit) in catalog.units.iter().enumerate() {
        let fixed_column_count =
            usize::try_from(unit.metadata.setup.n_constants).map_err(|_| {
                ProveExecutionPlanError::FixedColumnCountTooLarge {
                    unit_index,
                    fixed_column_count: unit.metadata.setup.n_constants,
                }
            })?;
        let stage_count = u16::try_from(unit.metadata.setup.n_stages).map_err(|_| {
            ProveExecutionPlanError::StageCountTooLarge {
                unit_index,
                stage_count: unit.metadata.setup.n_stages,
            }
        })?;
        units.push(ProveExecutionUnitArtifacts {
            fixed_columns: unit.paths.fixed_columns.clone(),
            expression_program: unit.expression_program.clone(),
            fri_expression_id: unit.metadata.verifier.quotient.expression_id,
            regular_constraints: unit.regular_constraints.clone(),
            setup: unit.metadata.setup.clone(),
            fixed_column_count,
            stage_count,
            opening_point_offsets: unit.metadata.setup.opening_points.clone(),
            group_name: unit
                .paths
                .group_name
                .clone()
                .unwrap_or_else(|| "global".to_owned()),
            unit_name: unit
                .paths
                .unit_name
                .clone()
                .unwrap_or_else(|| "main".to_owned()),
        });
    }
    Ok(units)
}

fn validate_execution_pcs_material(
    schedule: &ProveSchedule,
) -> Result<(), ProveExecutionPlanError> {
    for (unit_index, unit) in schedule.units.iter().enumerate() {
        if unit.pcs_material_bytes.is_none()
            || unit.pcs_material_plan_digest.is_none()
            || unit.pcs_material_fixed_column_digest.is_none()
            || unit.pcs_material_constant_tree_digest.is_none()
            || unit.pcs_material_constant_tree_root.is_none()
            || unit.pcs_material_fixed_byte_count.is_none()
            || unit.pcs_material_constant_tree_byte_count.is_none()
            || unit.pcs_material_leaf_byte_count.is_none()
            || unit.pcs_material_node_byte_count.is_none()
        {
            return Err(ProveExecutionPlanError::MissingPcsMaterial {
                unit_index,
                kind: unit.kind,
            });
        }
    }
    Ok(())
}

pub fn derive_prove_run_plan(
    catalog: &KeyDirectoryCatalog,
    request: ProveRunRequest,
) -> Result<ProveRunPlan, ProveRunPlanError> {
    let schedule = derive_prove_schedule(catalog)?;
    validate_pass(&request.pass)?;
    validate_run_options(&request.options)?;
    validate_gpu_options(&request.gpu)?;

    Ok(ProveRunPlan {
        schedule,
        pass: request.pass,
        options: request.options,
        gpu: request.gpu,
    })
}

fn validate_pass(pass: &ProvePassRequest) -> Result<(), ProveRunPlanError> {
    match pass {
        ProvePassRequest::Contributions(partitions) | ProvePassRequest::Full(partitions) => {
            validate_partition_plan(partitions)
        }
        ProvePassRequest::Internal { contribution_count } => {
            if *contribution_count == 0 {
                return Err(ProveRunPlanError::EmptyContributionSet);
            }
            Ok(())
        }
    }
}

fn validate_partition_plan(partitions: &ProvePartitionPlan) -> Result<(), ProveRunPlanError> {
    if partitions.partition_count == 0 {
        return Err(ProveRunPlanError::PartitionCountZero);
    }
    if partitions.partition_ids.is_empty() {
        return Err(ProveRunPlanError::EmptyPartitionSet);
    }
    if partitions.worker_index >= partitions.partition_count {
        return Err(ProveRunPlanError::WorkerOutOfRange {
            worker_index: partitions.worker_index,
            partition_count: partitions.partition_count,
        });
    }
    for partition_id in &partitions.partition_ids {
        if *partition_id as usize >= partitions.partition_count {
            return Err(ProveRunPlanError::PartitionOutOfRange {
                partition_id: *partition_id,
                partition_count: partitions.partition_count,
            });
        }
    }
    Ok(())
}

fn validate_regular_file(
    path: &Path,
    missing: fn(PathBuf) -> ProveExecutionPlanError,
    not_file: fn(PathBuf) -> ProveExecutionPlanError,
) -> Result<(), ProveExecutionPlanError> {
    let metadata = fs::metadata(path).map_err(|_| missing(path.to_path_buf()))?;
    if !metadata.is_file() {
        return Err(not_file(path.to_path_buf()));
    }
    Ok(())
}

fn validate_run_options(options: &ProveRunOptions) -> Result<(), ProveRunPlanError> {
    if options.output_dir.as_os_str().is_empty() {
        return Err(ProveRunPlanError::EmptyOutputDirectory);
    }
    if options.remote_aggregation && !options.aggregate {
        return Err(ProveRunPlanError::AggregationRequired {
            option: "remote_aggregation",
        });
    }
    if options.final_wrap && !options.aggregate {
        return Err(ProveRunPlanError::AggregationRequired {
            option: "final_wrap",
        });
    }
    Ok(())
}

fn validate_gpu_options(options: &GpuRunOptions) -> Result<(), ProveRunPlanError> {
    if options.max_streams == 0 {
        return Err(ProveRunPlanError::InvalidGpuStreams);
    }
    if options.witness_thread_pools == 0 {
        return Err(ProveRunPlanError::InvalidWitnessThreadPools);
    }
    if options.max_stored_witnesses == 0 {
        return Err(ProveRunPlanError::InvalidStoredWitnesses);
    }
    Ok(())
}

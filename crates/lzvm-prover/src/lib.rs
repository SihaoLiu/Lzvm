use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constraint_program::ConstraintProgram;
use lzvm_artifacts::eth_block_public_values::{
    validate_program_image_cache_public_values, EthBlockPublicValuesError,
};
use lzvm_artifacts::expression_program::ExpressionProgram;
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::guest_image::{read_guest_image_file, GuestImageError, GuestImageInfo};
use lzvm_artifacts::hint_program::{source_unimplemented_hint_name, HintProgram};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, read_key_directory_catalog, KeyDirectoryCatalog,
    KeyDirectoryError, KeyUnitKind,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageCommitmentCache,
    ProgramImageCommitmentCacheError,
};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValuesError};
use lzvm_artifacts::setup_info::{CommitmentColumn, EvaluationMapEntry, StageValue, UnitSetupInfo};
use lzvm_artifacts::setup_manifest::SetupDirectoryManifestError;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_artifacts::witness_library::{
    read_witness_library_file, WitnessLibraryError, WitnessLibraryInfo,
};
use lzvm_field::{Felt, FieldError};

use crate::proof_preflight::{public_values_as_fields, PublicValueFieldError};
use crate::setup_preflight::{validate_public_values_metadata, SetupPreflightError};

pub mod constant_opening;
pub mod constant_tree_opening;
pub mod contribution;
mod contribution_eth_block;
mod fixed_material;
pub mod fri_polynomial;
pub mod global_constraints;
pub mod gpu_setup;
pub mod group_values;
pub mod guest_instruction;
pub mod guest_machine;
pub mod guest_memory;
pub mod hint_eval;
mod merkle_hash;
pub mod pcs_challenge;
pub mod pcs_evaluation;
pub mod pcs_fri;
pub mod pcs_material_manifest;
pub mod pcs_query_plan;
pub mod pcs_transcript;
pub mod pcs_transcript_segments;
mod proof_artifact;
pub mod proof_preflight;
pub mod proof_values;
mod prove_fri_opening;
mod prove_fri_polynomial;
mod prove_witness;
pub mod regular_constraints;
pub mod setup_preflight;
mod source_assignment_hints;
mod source_lookup_hints;
pub mod unit_values;
pub mod verifier_eval;
pub mod verifier_query;
pub mod witness_commitment;
mod witness_execution;
pub mod witness_layout;
pub mod witness_loader;
pub mod witness_opening;
pub mod witness_runner;
pub mod witness_trace;

pub use fixed_material::{
    load_fixed_columns_material, FixedColumnsMaterial, FixedColumnsMaterialError,
};
pub use gpu_setup::{gpu_setup_available, prepare_gpu_setup, GpuSetupError};
pub use proof_artifact::{
    build_witness_contribution_proof_artifact_for_all_units,
    build_witness_contribution_proof_artifact_for_unit, build_witness_proof_artifact,
    build_witness_proof_artifact_for_all_units, build_witness_proof_artifact_for_unit,
    build_witness_proof_artifact_with_bindings, build_witness_proof_core_artifact,
    ProofArtifactInputs, WitnessAllUnitsProofRequest, WitnessProofRequest,
};
pub use prove_fri_opening::{
    build_pcs_fri_opening_segment, build_pcs_fri_opening_segment_from_trace,
    build_pcs_fri_opening_segment_from_trace_segments,
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
    build_pcs_query_nonce_segment_from_transcript_segments,
    build_pcs_query_nonce_segment_with_streams, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_challenge,
    build_pcs_query_plan_segment_from_transcript_segments,
    build_pcs_query_plan_segment_with_bindings, build_witness_commitment_segment,
    build_witness_opening_segment, build_witness_opening_segment_batch,
    ProveConstantOpeningSegmentError, ProvePcsEvaluationSegmentError, ProvePcsEvaluationValues,
    ProvePcsMaterialSegmentError, ProvePcsQueryPlanSegmentError, ProveWitnessOpeningSegmentError,
    ProveWitnessSegmentError,
};
pub use witness_execution::{
    run_prove_witness_commitments, run_prove_witness_commitments_for_all_units,
    run_prove_witness_commitments_for_all_units_with_trace_bundle,
    run_prove_witness_commitments_with_auxiliary_inputs, run_prove_witness_commitments_with_trace,
    run_prove_witness_commitments_with_trace_backend, ProveWitnessAuxiliaryInputs,
    ProveWitnessCommitmentError, ProveWitnessCommitments, ProveWitnessTraceCommitments,
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
    pub evaluation_map: Vec<EvaluationMapEntry>,
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

impl ProveUnitSchedule {
    pub fn expected_evaluation_value_count(&self) -> usize {
        if self.evaluation_map.is_empty() {
            self.evaluation_value_count
        } else {
            self.evaluation_map.len()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveScheduleError {
    EmptyCatalog,
    LengthOverflow,
    KeyDirectory(KeyDirectoryError),
    SetupDirectoryManifest(SetupDirectoryManifestError),
    VerificationKeyRootNonCanonical {
        unit_index: usize,
        word_index: usize,
        source: FieldError,
    },
    ConstantTreeRootNonCanonical {
        unit_index: usize,
        word_index: usize,
        source: FieldError,
    },
    PcsMaterialConstantTreeRootNonCanonical {
        unit_index: usize,
        word_index: usize,
        source: FieldError,
    },
    UnsupportedGlobalHint {
        name: String,
    },
    UnsupportedRegularHint {
        unit_index: usize,
        name: String,
    },
}

impl fmt::Display for ProveScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCatalog => write!(f, "prove schedule catalog is empty"),
            Self::LengthOverflow => write!(f, "prove schedule length overflow"),
            Self::KeyDirectory(error) => write!(f, "prove schedule catalog error: {error}"),
            Self::SetupDirectoryManifest(error) => write!(f, "{error}"),
            Self::VerificationKeyRootNonCanonical {
                unit_index,
                word_index,
                source,
            } => write!(
                f,
                "prove schedule verification key root word {word_index} is non-canonical for unit {unit_index}: {source}"
            ),
            Self::ConstantTreeRootNonCanonical {
                unit_index,
                word_index,
                source,
            } => write!(
                f,
                "prove schedule constant tree root word {word_index} is non-canonical for unit {unit_index}: {source}"
            ),
            Self::PcsMaterialConstantTreeRootNonCanonical {
                unit_index,
                word_index,
                source,
            } => write!(
                f,
                "prove schedule PCS material constant tree root word {word_index} is non-canonical for unit {unit_index}: {source}"
            ),
            Self::UnsupportedGlobalHint { name } => {
                write!(f, "prove schedule unsupported global hint {name}")
            }
            Self::UnsupportedRegularHint { unit_index, name } => {
                write!(
                    f,
                    "prove schedule unsupported regular hint {name} for unit {unit_index}"
                )
            }
        }
    }
}

impl std::error::Error for ProveScheduleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeyDirectory(error) => Some(error),
            Self::SetupDirectoryManifest(error) => Some(error),
            Self::VerificationKeyRootNonCanonical { source, .. } => Some(source),
            Self::ConstantTreeRootNonCanonical { source, .. } => Some(source),
            Self::PcsMaterialConstantTreeRootNonCanonical { source, .. } => Some(source),
            Self::EmptyCatalog
            | Self::LengthOverflow
            | Self::UnsupportedGlobalHint { .. }
            | Self::UnsupportedRegularHint { .. } => None,
        }
    }
}

impl From<KeyDirectoryError> for ProveScheduleError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::KeyDirectory(error)
    }
}

impl From<SetupDirectoryManifestError> for ProveScheduleError {
    fn from(error: SetupDirectoryManifestError) -> Self {
        Self::SetupDirectoryManifest(error)
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
    DuplicatePartitionId {
        partition_id: u32,
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
    FinalWrapRequiresFullPass,
    FinalWrapRemoteAggregation,
    FinalWrapRequiresSingleCompletePartition,
    FinalWrapRequiresFinalAggregation,
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
            Self::DuplicatePartitionId { partition_id } => {
                write!(f, "prove run plan partition id {partition_id} is duplicated")
            }
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
            Self::FinalWrapRequiresFullPass => {
                write!(f, "prove run plan final wrap requires full pass")
            }
            Self::FinalWrapRemoteAggregation => {
                write!(f, "prove run plan final wrap cannot use remote aggregation")
            }
            Self::FinalWrapRequiresSingleCompletePartition => write!(
                f,
                "prove run plan final wrap requires a single complete partition"
            ),
            Self::FinalWrapRequiresFinalAggregation => {
                write!(
                    f,
                    "prove run plan final wrap requires final aggregation unit"
                )
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
    pub witness_library: Option<PathBuf>,
    pub guest_image: PathBuf,
    pub public_inputs: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveProgramImageCache {
    pub path: PathBuf,
    pub cache: ProgramImageCommitmentCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveExecutionPlan {
    pub run_plan: ProveRunPlan,
    pub inputs: ProveExecutionInputArtifacts,
    pub global_info: GlobalInfo,
    pub global_hints: HintProgram,
    pub witness_library_info: Option<WitnessLibraryInfo>,
    pub guest_image_info: GuestImageInfo,
    pub program_image_cache: Option<ProveProgramImageCache>,
    pub units: Vec<ProveExecutionUnitArtifacts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveExecutionUnitArtifacts {
    pub fixed_columns: PathBuf,
    pub expression_program: ExpressionProgram,
    pub fri_expression_id: Option<u32>,
    pub regular_constraints: ConstraintProgram,
    pub regular_hints: HintProgram,
    pub setup: UnitSetupInfo,
    pub fixed_column_count: usize,
    pub stage_count: u16,
    pub opening_point_offsets: Vec<i64>,
    pub group_name: String,
    pub unit_name: String,
}

impl ProveExecutionUnitArtifacts {
    pub fn expected_evaluation_value_count(&self) -> usize {
        if self.setup.evaluation_map.is_empty() {
            self.setup.eval_count
        } else {
            self.setup.evaluation_map.len()
        }
    }
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
    MissingProgramImageCache {
        path: PathBuf,
    },
    ProgramImageCacheIsNotFile {
        path: PathBuf,
    },
    InvalidProgramImageCache {
        path: PathBuf,
        source: ProgramImageCommitmentCacheError,
    },
    ProgramImageCacheTreeRootNonCanonical {
        path: PathBuf,
        word_index: usize,
        source: FieldError,
    },
    ProgramImageCacheGuestImageDigestMismatch {
        path: PathBuf,
    },
    ProgramImageCacheSetupHashMismatch {
        path: PathBuf,
    },
    MissingPublicInputs {
        path: PathBuf,
    },
    FinalWrapRequiresPublicInputs,
    PublicInputsIsNotFile {
        path: PathBuf,
    },
    InvalidPublicInputs {
        path: PathBuf,
        source: PublicValuesError,
    },
    PublicInputsFieldConversion {
        path: PathBuf,
        source: PublicValueFieldError,
    },
    PublicInputsMetadata {
        path: PathBuf,
        source: SetupPreflightError,
    },
    PublicInputsSetupHashMismatch {
        path: PathBuf,
    },
    ProgramImageCachePublicInputs {
        path: PathBuf,
        source: EthBlockPublicValuesError,
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
            Self::MissingProgramImageCache { path } => {
                write!(f, "program image cache is missing: {}", path.display())
            }
            Self::ProgramImageCacheIsNotFile { path } => {
                write!(f, "program image cache is not a file: {}", path.display())
            }
            Self::InvalidProgramImageCache { path, source } => {
                write!(f, "program image cache failed at {}: {source}", path.display())
            }
            Self::ProgramImageCacheTreeRootNonCanonical {
                path,
                word_index,
                source,
            } => write!(
                f,
                "program image cache tree root word {word_index} is non-canonical at {}: {source}",
                path.display()
            ),
            Self::ProgramImageCacheGuestImageDigestMismatch { path } => write!(
                f,
                "program image cache guest image digest mismatch at {}",
                path.display()
            ),
            Self::ProgramImageCacheSetupHashMismatch { path } => write!(
                f,
                "program image cache setup hash mismatch at {}",
                path.display()
            ),
            Self::MissingPublicInputs { path } => {
                write!(
                    f,
                    "prove execution plan public inputs are missing: {}",
                    path.display()
                )
            }
            Self::FinalWrapRequiresPublicInputs => {
                write!(f, "prove execution plan final wrap requires public inputs")
            }
            Self::PublicInputsIsNotFile { path } => write!(
                f,
                "prove execution plan public inputs are not a file: {}",
                path.display()
            ),
            Self::InvalidPublicInputs { path, source } => write!(
                f,
                "prove execution plan public inputs are invalid: {}: {source}",
                path.display()
            ),
            Self::PublicInputsFieldConversion { path, source } => write!(
                f,
                "prove execution plan public inputs field conversion failed: {}: {source}",
                path.display()
            ),
            Self::PublicInputsMetadata { path, source } => write!(
                f,
                "prove execution plan public inputs metadata mismatch: {}: {source}",
                path.display()
            ),
            Self::PublicInputsSetupHashMismatch { path } => write!(
                f,
                "prove execution plan public inputs setup hash mismatch: {}",
                path.display()
            ),
            Self::ProgramImageCachePublicInputs { path, source } => write!(
                f,
                "prove execution plan program image cache public inputs mismatch: {}: {source}",
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
            Self::InvalidProgramImageCache { source, .. } => Some(source),
            Self::ProgramImageCacheTreeRootNonCanonical { source, .. } => Some(source),
            Self::InvalidPublicInputs { source, .. } => Some(source),
            Self::PublicInputsFieldConversion { source, .. } => Some(source),
            Self::PublicInputsMetadata { source, .. } => Some(source),
            Self::ProgramImageCachePublicInputs { source, .. } => Some(source),
            Self::RunPlan(error) => Some(error),
            Self::MissingPcsMaterial { .. }
            | Self::MissingProgramImageCache { .. }
            | Self::ProgramImageCacheIsNotFile { .. }
            | Self::ProgramImageCacheGuestImageDigestMismatch { .. }
            | Self::ProgramImageCacheSetupHashMismatch { .. }
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

    validate_schedulable_global_hints(&catalog.global_hints)?;
    let setup_hash = key_directory_catalog_digest(catalog)?;
    let mut total_fixed_bytes = 0_u64;
    let mut total_pcs_material_bytes = 0_u64;
    let mut pcs_material_unit_count = 0_usize;
    let mut total_query_count = 0_u64;
    let mut max_extended_domain_bits = 0_u32;
    let mut units = Vec::with_capacity(catalog.units.len());
    for (unit_index, unit) in catalog.units.iter().enumerate() {
        validate_schedulable_regular_hints(&unit.regular_hints, unit_index)?;
        validate_verification_key_root(unit_index, &unit.verification_key)?;
        if let Some(root) = &unit.constant_tree_root {
            validate_constant_tree_root(unit_index, root)?;
        }
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
        if let Some(material) = material {
            validate_pcs_material_constant_tree_root(unit_index, material.constant_tree_root)?;
        }

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
            evaluation_map: unit.metadata.setup.evaluation_map.clone(),
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

fn validate_verification_key_root(
    unit_index: usize,
    root: &VerificationKeyRoot,
) -> Result<(), ProveScheduleError> {
    let VerificationKeyRoot::FieldElements(words) = root;
    for (word_index, word) in words.iter().copied().enumerate() {
        Felt::from_canonical(word).map_err(|source| {
            ProveScheduleError::VerificationKeyRootNonCanonical {
                unit_index,
                word_index,
                source,
            }
        })?;
    }
    Ok(())
}

fn validate_constant_tree_root(
    unit_index: usize,
    root: &VerificationKeyRoot,
) -> Result<(), ProveScheduleError> {
    let VerificationKeyRoot::FieldElements(words) = root;
    for (word_index, word) in words.iter().copied().enumerate() {
        Felt::from_canonical(word).map_err(|source| {
            ProveScheduleError::ConstantTreeRootNonCanonical {
                unit_index,
                word_index,
                source,
            }
        })?;
    }
    Ok(())
}

fn validate_pcs_material_constant_tree_root(
    unit_index: usize,
    root: [u64; 4],
) -> Result<(), ProveScheduleError> {
    for (word_index, word) in root.into_iter().enumerate() {
        Felt::from_canonical(word).map_err(|source| {
            ProveScheduleError::PcsMaterialConstantTreeRootNonCanonical {
                unit_index,
                word_index,
                source,
            }
        })?;
    }
    Ok(())
}

fn validate_schedulable_global_hints(program: &HintProgram) -> Result<(), ProveScheduleError> {
    if let Some(hint) = program
        .hints
        .iter()
        .find(|hint| source_unimplemented_hint_name(&hint.name))
    {
        return Err(ProveScheduleError::UnsupportedGlobalHint {
            name: hint.name.clone(),
        });
    }
    Ok(())
}

fn validate_schedulable_regular_hints(
    program: &HintProgram,
    unit_index: usize,
) -> Result<(), ProveScheduleError> {
    if let Some(hint) = program
        .hints
        .iter()
        .find(|hint| source_unimplemented_hint_name(&hint.name))
    {
        return Err(ProveScheduleError::UnsupportedRegularHint {
            unit_index,
            name: hint.name.clone(),
        });
    }
    Ok(())
}

pub fn derive_prove_schedule_from_directory(
    root: impl AsRef<Path>,
) -> Result<ProveSchedule, ProveScheduleError> {
    let root = root.as_ref();
    let catalog = read_key_directory_catalog(root).map_err(ProveScheduleError::from)?;
    crate::setup_preflight::validate_setup_directory_manifest_if_present(root, &catalog)
        .map_err(ProveScheduleError::from)?;
    derive_prove_schedule(&catalog)
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
    derive_prove_execution_plan_with_program_image_cache(catalog, request, inputs, None)
}

pub fn derive_prove_execution_plan_with_program_image_cache(
    catalog: &KeyDirectoryCatalog,
    request: ProveRunRequest,
    inputs: ProveExecutionInputArtifacts,
    program_image_cache: Option<PathBuf>,
) -> Result<ProveExecutionPlan, ProveExecutionPlanError> {
    let run_plan = derive_prove_run_plan(catalog, request)?;
    if run_plan.options.final_wrap && inputs.public_inputs.is_none() {
        return Err(ProveExecutionPlanError::FinalWrapRequiresPublicInputs);
    }
    validate_execution_pcs_material(&run_plan.schedule)?;
    let witness_library_info = match &inputs.witness_library {
        Some(path) => {
            validate_regular_file(
                path,
                |path| ProveExecutionPlanError::MissingWitnessLibrary { path },
                |path| ProveExecutionPlanError::WitnessLibraryIsNotFile { path },
            )?;
            Some(read_witness_library_file(path).map_err(|source| {
                ProveExecutionPlanError::InvalidWitnessLibrary {
                    path: path.clone(),
                    source,
                }
            })?)
        }
        None => None,
    };
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
    let public_values = if let Some(public_inputs) = &inputs.public_inputs {
        validate_regular_file(
            public_inputs,
            |path| ProveExecutionPlanError::MissingPublicInputs { path },
            |path| ProveExecutionPlanError::PublicInputsIsNotFile { path },
        )?;
        let public_values = read_public_values_file(public_inputs).map_err(|source| {
            ProveExecutionPlanError::InvalidPublicInputs {
                path: public_inputs.clone(),
                source,
            }
        })?;
        if public_values.setup_hash != run_plan.schedule.setup_hash {
            return Err(ProveExecutionPlanError::PublicInputsSetupHashMismatch {
                path: public_inputs.clone(),
            });
        }
        public_values_as_fields(&public_values).map_err(|source| {
            ProveExecutionPlanError::PublicInputsFieldConversion {
                path: public_inputs.clone(),
                source,
            }
        })?;
        validate_public_values_metadata(&catalog.layout.global_info, &public_values).map_err(
            |source| ProveExecutionPlanError::PublicInputsMetadata {
                path: public_inputs.clone(),
                source,
            },
        )?;
        Some((public_inputs.clone(), public_values))
    } else {
        None
    };
    let program_image_cache = match program_image_cache {
        Some(path) => Some(load_program_image_cache(
            &path,
            &guest_image_info,
            &run_plan.schedule.setup_hash,
        )?),
        None => None,
    };
    if let Some((path, public_values)) = &public_values {
        validate_program_image_cache_public_values(
            public_values,
            program_image_cache
                .as_ref()
                .map(|program_image_cache| &program_image_cache.cache),
        )
        .map_err(
            |source| ProveExecutionPlanError::ProgramImageCachePublicInputs {
                path: path.clone(),
                source,
            },
        )?;
    }

    let units = derive_prove_execution_units(catalog)?;

    Ok(ProveExecutionPlan {
        run_plan,
        inputs,
        global_info: catalog.layout.global_info.clone(),
        global_hints: catalog.global_hints.clone(),
        witness_library_info,
        guest_image_info,
        program_image_cache,
        units,
    })
}

fn load_program_image_cache(
    path: &Path,
    guest_image_info: &GuestImageInfo,
    setup_hash: &[u8; 32],
) -> Result<ProveProgramImageCache, ProveExecutionPlanError> {
    validate_regular_file(
        path,
        |path| ProveExecutionPlanError::MissingProgramImageCache { path },
        |path| ProveExecutionPlanError::ProgramImageCacheIsNotFile { path },
    )?;
    let cache = read_program_image_commitment_cache_file(path).map_err(|source| match source {
        ProgramImageCommitmentCacheError::TreeRootNonCanonical { word_index, source } => {
            ProveExecutionPlanError::ProgramImageCacheTreeRootNonCanonical {
                path: path.to_path_buf(),
                word_index,
                source,
            }
        }
        source => ProveExecutionPlanError::InvalidProgramImageCache {
            path: path.to_path_buf(),
            source,
        },
    })?;
    validate_program_image_cache_tree_root(path, &cache)?;
    if cache.source_image_digest != guest_image_info.digest {
        return Err(
            ProveExecutionPlanError::ProgramImageCacheGuestImageDigestMismatch {
                path: path.to_path_buf(),
            },
        );
    }
    if &cache.constraint_system_digest != setup_hash {
        return Err(
            ProveExecutionPlanError::ProgramImageCacheSetupHashMismatch {
                path: path.to_path_buf(),
            },
        );
    }
    Ok(ProveProgramImageCache {
        path: path.to_path_buf(),
        cache,
    })
}

fn validate_program_image_cache_tree_root(
    path: &Path,
    cache: &ProgramImageCommitmentCache,
) -> Result<(), ProveExecutionPlanError> {
    for (word_index, word) in cache.tree_root.iter().copied().enumerate() {
        Felt::from_canonical(word).map_err(|source| {
            ProveExecutionPlanError::ProgramImageCacheTreeRootNonCanonical {
                path: path.to_path_buf(),
                word_index,
                source,
            }
        })?;
    }
    Ok(())
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
            regular_hints: unit.regular_hints.clone(),
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
    validate_final_wrap_options(&schedule, &request.pass, &request.options)?;

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
    let mut seen_partition_ids = BTreeSet::new();
    for partition_id in &partitions.partition_ids {
        if *partition_id as usize >= partitions.partition_count {
            return Err(ProveRunPlanError::PartitionOutOfRange {
                partition_id: *partition_id,
                partition_count: partitions.partition_count,
            });
        }
        if !seen_partition_ids.insert(*partition_id) {
            return Err(ProveRunPlanError::DuplicatePartitionId {
                partition_id: *partition_id,
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

fn validate_final_wrap_options(
    schedule: &ProveSchedule,
    pass: &ProvePassRequest,
    options: &ProveRunOptions,
) -> Result<(), ProveRunPlanError> {
    if !options.final_wrap {
        return Ok(());
    }
    let ProvePassRequest::Full(partitions) = pass else {
        return Err(ProveRunPlanError::FinalWrapRequiresFullPass);
    };
    if options.remote_aggregation {
        return Err(ProveRunPlanError::FinalWrapRemoteAggregation);
    }
    if partitions.partition_count != 1
        || partitions.partition_ids.as_slice() != [0]
        || partitions.worker_index != 0
    {
        return Err(ProveRunPlanError::FinalWrapRequiresSingleCompletePartition);
    }
    if !schedule
        .units
        .iter()
        .any(|unit| unit.kind == KeyUnitKind::FinalAggregation)
    {
        return Err(ProveRunPlanError::FinalWrapRequiresFinalAggregation);
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

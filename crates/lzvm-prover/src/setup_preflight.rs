use std::fmt;
use std::path::Path;

use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, read_key_directory_catalog, KeyDirectoryCatalog,
    KeyDirectoryError,
};
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::proof::{read_proof_artifact_file, ProofArtifact, ProofArtifactError};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValues, PublicValuesError};
use lzvm_artifacts::setup_manifest::{
    build_setup_directory_manifest, validate_setup_directory_manifest_file,
    SetupDirectoryManifestError, SETUP_DIRECTORY_MANIFEST_FILE,
};

use crate::constant_opening::{
    validate_constant_opening_segments, ValidateConstantOpeningSegmentsError,
};
use crate::global_constraints::{
    validate_global_constraints_from_proof_segments, ValidateGlobalConstraintProofSegmentsError,
    ValidateGlobalConstraintProofSegmentsRequest,
};
use crate::hint_eval::{
    resolve_global_hint_program_from_proof_segments, ResolveGlobalHintProofSegmentsError,
    ResolveGlobalHintProofSegmentsRequest,
};
use crate::pcs_fri::{
    validate_optional_pcs_fri_opening_proof_segments,
    ValidateOptionalPcsFriOpeningProofSegmentsError,
    ValidateOptionalPcsFriOpeningProofSegmentsRequest,
};
use crate::pcs_material_manifest::{
    validate_pcs_material_manifest_segments, ValidatePcsMaterialManifestSegmentsError,
};
use crate::pcs_query_plan::{
    uses_transcript_pcs_query_plan_inputs, validate_pcs_query_plan_segments,
    ValidatePcsQueryPlanSegmentsError,
};
use crate::proof_preflight::{
    public_values_as_fields, validate_proof_public_values, ProofPreflightError,
    ProofPreflightReport, PublicValueFieldError,
};
use crate::witness_commitment::{
    load_witness_commitment_segments, LoadWitnessCommitmentSegmentsError,
};
use crate::witness_opening::{
    validate_witness_opening_segments, ValidateWitnessOpeningSegmentsError,
};
use crate::{derive_prove_schedule, ProveScheduleError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupPreflightReport {
    pub unit_count: usize,
    pub segment_count: usize,
    pub public_value_count: usize,
    pub public_values_hash: [u8; 32],
    pub public_value_field_count: usize,
    pub program_image_cache_count: usize,
    pub program_image_caches: Vec<ProgramImageCommitmentCache>,
    pub program_image_cache_hashes: Vec<[u8; 32]>,
    pub eth_block_input_count: usize,
    pub eth_block_input_hashes: Vec<[u8; 32]>,
    pub eth_block_input_block_rlp_byte_counts: Vec<usize>,
    pub eth_block_input_block_hashes: Vec<[u8; 32]>,
    pub eth_block_input_parent_hashes: Vec<[u8; 32]>,
    pub eth_block_input_ommers_hashes: Vec<[u8; 32]>,
    pub eth_block_input_beneficiaries: Vec<[u8; 20]>,
    pub eth_block_input_state_roots: Vec<[u8; 32]>,
    pub eth_block_input_receipt_roots: Vec<[u8; 32]>,
    pub eth_block_input_logs_blooms: Vec<[u8; 256]>,
    pub eth_block_input_difficulties: Vec<[u8; 32]>,
    pub eth_block_input_block_numbers: Vec<u64>,
    pub eth_block_input_timestamps: Vec<u64>,
    pub eth_block_input_extra_data: Vec<Vec<u8>>,
    pub eth_block_input_gas_limits: Vec<u64>,
    pub eth_block_input_gas_used_values: Vec<u64>,
    pub eth_block_input_base_fees_per_gas: Vec<Option<[u8; 32]>>,
    pub eth_block_input_mix_hashes: Vec<[u8; 32]>,
    pub eth_block_input_nonces: Vec<[u8; 8]>,
    pub eth_block_input_transaction_roots: Vec<[u8; 32]>,
    pub eth_block_input_transaction_preimage_counts: Vec<usize>,
    pub eth_block_input_legacy_transaction_counts: Vec<usize>,
    pub eth_block_input_typed_transaction_counts: Vec<usize>,
    pub eth_block_input_receipt_preimage_counts: Vec<Option<usize>>,
    pub eth_block_input_legacy_receipt_counts: Vec<Option<usize>>,
    pub eth_block_input_typed_receipt_counts: Vec<Option<usize>>,
    pub eth_block_input_withdrawal_roots: Vec<Option<[u8; 32]>>,
    pub eth_block_input_withdrawal_counts: Vec<Option<usize>>,
    pub eth_block_input_withdrawal_preimage_counts: Vec<Option<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupPreflightError {
    Catalog(KeyDirectoryError),
    Proof(ProofPreflightError),
    CatalogHashMismatch,
    SetupDirectoryManifest(SetupDirectoryManifestError),
    Schedule(ProveScheduleError),
    PublicValues(PublicValueFieldError),
    PcsMaterial(ValidatePcsMaterialManifestSegmentsError),
    WitnessCommitment(LoadWitnessCommitmentSegmentsError),
    PcsQueryPlan(ValidatePcsQueryPlanSegmentsError),
    ConstantOpening(ValidateConstantOpeningSegmentsError),
    WitnessOpening(ValidateWitnessOpeningSegmentsError),
    GlobalConstraints(ValidateGlobalConstraintProofSegmentsError),
    GlobalHints(ResolveGlobalHintProofSegmentsError),
    PcsFri(ValidateOptionalPcsFriOpeningProofSegmentsError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupPreflightFileError {
    Catalog(KeyDirectoryError),
    Proof(ProofArtifactError),
    PublicValues(PublicValuesError),
    SetupPreflight(SetupPreflightError),
}

impl fmt::Display for SetupPreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "{error}"),
            Self::Proof(error) => write!(f, "{error}"),
            Self::CatalogHashMismatch => write!(f, "setup catalog fingerprint mismatch"),
            Self::SetupDirectoryManifest(error) => write!(f, "{error}"),
            Self::Schedule(error) => write!(f, "{error}"),
            Self::PublicValues(error) => write!(f, "{error}"),
            Self::PcsMaterial(error) => write!(f, "{error}"),
            Self::WitnessCommitment(error) => write!(f, "{error}"),
            Self::PcsQueryPlan(error) => write!(f, "{error}"),
            Self::ConstantOpening(error) => write!(f, "{error}"),
            Self::WitnessOpening(error) => write!(f, "{error}"),
            Self::GlobalConstraints(error) => write!(f, "{error}"),
            Self::GlobalHints(error) => write!(f, "{error}"),
            Self::PcsFri(error) => write!(f, "{error}"),
        }
    }
}

impl fmt::Display for SetupPreflightFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "{error}"),
            Self::Proof(error) => write!(f, "{error}"),
            Self::PublicValues(error) => write!(f, "{error}"),
            Self::SetupPreflight(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SetupPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Proof(error) => Some(error),
            Self::SetupDirectoryManifest(error) => Some(error),
            Self::Schedule(error) => Some(error),
            Self::PublicValues(error) => Some(error),
            Self::PcsMaterial(error) => Some(error),
            Self::WitnessCommitment(error) => Some(error),
            Self::PcsQueryPlan(error) => Some(error),
            Self::ConstantOpening(error) => Some(error),
            Self::WitnessOpening(error) => Some(error),
            Self::GlobalConstraints(error) => Some(error),
            Self::GlobalHints(error) => Some(error),
            Self::PcsFri(error) => Some(error),
            Self::CatalogHashMismatch => None,
        }
    }
}

impl std::error::Error for SetupPreflightFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Proof(error) => Some(error),
            Self::PublicValues(error) => Some(error),
            Self::SetupPreflight(error) => Some(error),
        }
    }
}

impl From<KeyDirectoryError> for SetupPreflightFileError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::Catalog(error)
    }
}

impl From<ProofArtifactError> for SetupPreflightFileError {
    fn from(error: ProofArtifactError) -> Self {
        Self::Proof(error)
    }
}

impl From<PublicValuesError> for SetupPreflightFileError {
    fn from(error: PublicValuesError) -> Self {
        Self::PublicValues(error)
    }
}

impl From<SetupPreflightError> for SetupPreflightFileError {
    fn from(error: SetupPreflightError) -> Self {
        Self::SetupPreflight(error)
    }
}

pub fn validate_setup_directory_manifest_if_present(
    root: &Path,
    catalog: &KeyDirectoryCatalog,
) -> Result<(), SetupDirectoryManifestError> {
    let expected = build_setup_directory_manifest(catalog)?;
    let path = root.join(SETUP_DIRECTORY_MANIFEST_FILE);
    validate_setup_directory_manifest_file(&path, &expected)
}

pub fn validate_setup_preflight_hashes(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<SetupPreflightReport, SetupPreflightError> {
    if proof.setup_hash != public_values.setup_hash {
        return Err(SetupPreflightError::Proof(
            ProofPreflightError::SetupHashMismatch,
        ));
    }

    let catalog_hash =
        key_directory_catalog_digest(catalog).map_err(SetupPreflightError::Catalog)?;
    if proof.setup_hash != catalog_hash {
        return Err(SetupPreflightError::CatalogHashMismatch);
    }

    let ProofPreflightReport {
        segment_count,
        public_value_count,
        public_values_hash,
        public_value_field_count,
        program_image_cache_count,
        program_image_caches,
        program_image_cache_hashes,
        eth_block_input_count,
        eth_block_input_hashes,
        eth_block_input_block_rlp_byte_counts,
        eth_block_input_block_hashes,
        eth_block_input_parent_hashes,
        eth_block_input_ommers_hashes,
        eth_block_input_beneficiaries,
        eth_block_input_state_roots,
        eth_block_input_receipt_roots,
        eth_block_input_logs_blooms,
        eth_block_input_difficulties,
        eth_block_input_block_numbers,
        eth_block_input_timestamps,
        eth_block_input_extra_data,
        eth_block_input_gas_limits,
        eth_block_input_gas_used_values,
        eth_block_input_base_fees_per_gas,
        eth_block_input_mix_hashes,
        eth_block_input_nonces,
        eth_block_input_transaction_roots,
        eth_block_input_transaction_preimage_counts,
        eth_block_input_legacy_transaction_counts,
        eth_block_input_typed_transaction_counts,
        eth_block_input_receipt_preimage_counts,
        eth_block_input_legacy_receipt_counts,
        eth_block_input_typed_receipt_counts,
        eth_block_input_withdrawal_roots,
        eth_block_input_withdrawal_counts,
        eth_block_input_withdrawal_preimage_counts,
    } = validate_proof_public_values(proof, public_values).map_err(SetupPreflightError::Proof)?;

    Ok(SetupPreflightReport {
        unit_count: catalog.units.len(),
        segment_count,
        public_value_count,
        public_values_hash,
        public_value_field_count,
        program_image_cache_count,
        program_image_caches,
        program_image_cache_hashes,
        eth_block_input_count,
        eth_block_input_hashes,
        eth_block_input_block_rlp_byte_counts,
        eth_block_input_block_hashes,
        eth_block_input_parent_hashes,
        eth_block_input_ommers_hashes,
        eth_block_input_beneficiaries,
        eth_block_input_state_roots,
        eth_block_input_receipt_roots,
        eth_block_input_logs_blooms,
        eth_block_input_difficulties,
        eth_block_input_block_numbers,
        eth_block_input_timestamps,
        eth_block_input_extra_data,
        eth_block_input_gas_limits,
        eth_block_input_gas_used_values,
        eth_block_input_base_fees_per_gas,
        eth_block_input_mix_hashes,
        eth_block_input_nonces,
        eth_block_input_transaction_roots,
        eth_block_input_transaction_preimage_counts,
        eth_block_input_legacy_transaction_counts,
        eth_block_input_typed_transaction_counts,
        eth_block_input_receipt_preimage_counts,
        eth_block_input_legacy_receipt_counts,
        eth_block_input_typed_receipt_counts,
        eth_block_input_withdrawal_roots,
        eth_block_input_withdrawal_counts,
        eth_block_input_withdrawal_preimage_counts,
    })
}

pub fn validate_setup_preflight(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<SetupPreflightReport, SetupPreflightError> {
    let report = validate_setup_preflight_hashes(catalog, proof, public_values)?;
    let schedule = derive_prove_schedule(catalog).map_err(SetupPreflightError::Schedule)?;
    let uses_transcript_inputs = uses_transcript_pcs_query_plan_inputs(&proof.segments);
    let needs_public_fields = uses_transcript_inputs
        || !catalog.global_constraints.entries.is_empty()
        || !catalog.global_hints.hints.is_empty();
    let public_fields = if needs_public_fields {
        Some(public_values_as_fields(public_values).map_err(SetupPreflightError::PublicValues)?)
    } else {
        None
    };
    let transcript_public_fields = if uses_transcript_inputs {
        public_fields.as_deref().unwrap_or(&[])
    } else {
        &[]
    };

    validate_pcs_material_manifest_segments(&schedule, &proof.segments)
        .map_err(SetupPreflightError::PcsMaterial)?;
    load_witness_commitment_segments(&schedule.units, &proof.segments)
        .map(|_| ())
        .map_err(SetupPreflightError::WitnessCommitment)?;
    validate_pcs_query_plan_segments(
        &schedule,
        proof.public_values_hash,
        transcript_public_fields,
        &proof.segments,
    )
    .map_err(SetupPreflightError::PcsQueryPlan)?;
    validate_constant_opening_segments(&schedule.units, &proof.segments)
        .map_err(SetupPreflightError::ConstantOpening)?;
    validate_witness_opening_segments(&schedule.units, &proof.segments)
        .map_err(SetupPreflightError::WitnessOpening)?;

    if !catalog.global_constraints.entries.is_empty() {
        validate_global_constraints_from_proof_segments(
            ValidateGlobalConstraintProofSegmentsRequest {
                program: &catalog.global_constraints,
                global_info: &catalog.layout.global_info,
                schedule: &schedule,
                public_values: public_fields.as_deref().unwrap_or(&[]),
                segments: &proof.segments,
            },
        )
        .map_err(SetupPreflightError::GlobalConstraints)?;
    }

    if !catalog.global_hints.hints.is_empty() {
        resolve_global_hint_program_from_proof_segments(ResolveGlobalHintProofSegmentsRequest {
            global_info: &catalog.layout.global_info,
            program: &catalog.global_hints,
            schedule: &schedule,
            public_values: public_fields.as_deref().unwrap_or(&[]),
            segments: &proof.segments,
        })
        .map(|_| ())
        .map_err(SetupPreflightError::GlobalHints)?;
    }

    let verifier_codes = catalog
        .units
        .iter()
        .map(|unit| &unit.metadata.verifier.query)
        .collect::<Vec<_>>();
    validate_optional_pcs_fri_opening_proof_segments(
        ValidateOptionalPcsFriOpeningProofSegmentsRequest {
            schedule: &schedule,
            verifier_codes: &verifier_codes,
            global_info: &catalog.layout.global_info,
            public_values: transcript_public_fields,
            segments: &proof.segments,
        },
    )
    .map_err(SetupPreflightError::PcsFri)?;

    Ok(report)
}

pub fn validate_setup_preflight_from_files(
    setup_dir: impl AsRef<Path>,
    proof_path: impl AsRef<Path>,
    public_values_path: impl AsRef<Path>,
) -> Result<SetupPreflightReport, SetupPreflightFileError> {
    let setup_dir = setup_dir.as_ref();
    let catalog = read_key_directory_catalog(setup_dir)?;
    validate_setup_directory_manifest_if_present(setup_dir, &catalog)
        .map_err(SetupPreflightError::SetupDirectoryManifest)?;
    let proof = read_proof_artifact_file(proof_path)?;
    let public_values = read_public_values_file(public_values_path)?;
    validate_setup_preflight(&catalog, &proof, &public_values).map_err(Into::into)
}

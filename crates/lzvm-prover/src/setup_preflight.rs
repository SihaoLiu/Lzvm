use std::fmt;
use std::path::Path;

use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, read_key_directory_catalog, KeyDirectoryCatalog,
    KeyDirectoryError,
};
use lzvm_artifacts::proof::{read_proof_artifact_file, ProofArtifact, ProofArtifactError};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValues, PublicValuesError};

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupPreflightError {
    Catalog(KeyDirectoryError),
    Proof(ProofPreflightError),
    CatalogHashMismatch,
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
    } = validate_proof_public_values(proof, public_values).map_err(SetupPreflightError::Proof)?;

    Ok(SetupPreflightReport {
        unit_count: catalog.units.len(),
        segment_count,
        public_value_count,
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
    let catalog = read_key_directory_catalog(setup_dir)?;
    let proof = read_proof_artifact_file(proof_path)?;
    let public_values = read_public_values_file(public_values_path)?;
    validate_setup_preflight(&catalog, &proof, &public_values).map_err(Into::into)
}

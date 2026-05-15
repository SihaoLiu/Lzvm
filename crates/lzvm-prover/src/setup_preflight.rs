use std::fmt;

use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, KeyDirectoryCatalog, KeyDirectoryError,
};
use lzvm_artifacts::proof::ProofArtifact;
use lzvm_artifacts::public_values::PublicValues;

use crate::proof_preflight::{
    validate_proof_public_values, ProofPreflightError, ProofPreflightReport,
};

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
}

impl fmt::Display for SetupPreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "{error}"),
            Self::Proof(error) => write!(f, "{error}"),
            Self::CatalogHashMismatch => write!(f, "setup catalog fingerprint mismatch"),
        }
    }
}

impl std::error::Error for SetupPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Proof(error) => Some(error),
            Self::CatalogHashMismatch => None,
        }
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

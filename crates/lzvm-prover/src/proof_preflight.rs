use std::fmt;

use lzvm_artifacts::proof::ProofArtifact;
use lzvm_artifacts::public_values::{public_values_digest, PublicValues, PublicValuesError};
use lzvm_field::{Felt, FieldError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofPreflightReport {
    pub segment_count: usize,
    pub public_value_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofPreflightError {
    SetupHashMismatch,
    PublicValuesDigest(PublicValuesError),
    PublicValuesHashMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicValueFieldError {
    Field(FieldError),
}

impl fmt::Display for ProofPreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupHashMismatch => write!(f, "setup hash mismatch"),
            Self::PublicValuesDigest(error) => write!(f, "{error}"),
            Self::PublicValuesHashMismatch => write!(f, "public-values hash mismatch"),
        }
    }
}

impl fmt::Display for PublicValueFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Field(error) => write!(f, "invalid PCS transcript public value: {error}"),
        }
    }
}

impl std::error::Error for ProofPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PublicValuesDigest(error) => Some(error),
            Self::SetupHashMismatch | Self::PublicValuesHashMismatch => None,
        }
    }
}

impl std::error::Error for PublicValueFieldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field(error) => Some(error),
        }
    }
}

pub fn validate_proof_public_values(
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<ProofPreflightReport, ProofPreflightError> {
    if proof.setup_hash != public_values.setup_hash {
        return Err(ProofPreflightError::SetupHashMismatch);
    }

    let digest =
        public_values_digest(public_values).map_err(ProofPreflightError::PublicValuesDigest)?;
    if proof.public_values_hash != digest {
        return Err(ProofPreflightError::PublicValuesHashMismatch);
    }

    Ok(ProofPreflightReport {
        segment_count: proof.segments.len(),
        public_value_count: public_values.values.len(),
    })
}

pub fn public_values_as_fields(
    public_values: &PublicValues,
) -> Result<Vec<Felt>, PublicValueFieldError> {
    public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied())
        .map(Felt::from_canonical)
        .collect::<Result<Vec<_>, _>>()
        .map_err(PublicValueFieldError::Field)
}

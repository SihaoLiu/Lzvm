use std::fmt;

use lzvm_artifacts::proof::ProofArtifact;
use lzvm_artifacts::public_values::{public_values_digest, PublicValues, PublicValuesError};

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

impl fmt::Display for ProofPreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupHashMismatch => write!(f, "setup hash mismatch"),
            Self::PublicValuesDigest(error) => write!(f, "{error}"),
            Self::PublicValuesHashMismatch => write!(f, "public-values hash mismatch"),
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

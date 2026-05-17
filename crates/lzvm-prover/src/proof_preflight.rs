use std::fmt;
use std::path::Path;

use lzvm_artifacts::eth_block_input::{eth_block_input_bytes_digest, EthBlockInputError};
use lzvm_artifacts::eth_block_input_segment::{
    parse_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::{
    validate_eth_block_public_values, EthBlockPublicValuesError,
};
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::program_image_segment::{
    parse_program_image_cache_segment, ProgramImageCacheSegmentError,
    PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::{
    read_proof_artifact_file, validate_proof_artifact, ProofArtifact, ProofArtifactError,
};
use lzvm_artifacts::public_values::{
    public_values_digest, read_public_values_file, PublicValues, PublicValuesError,
};
use lzvm_field::{Felt, FieldError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofPreflightReport {
    pub segment_count: usize,
    pub public_value_count: usize,
    pub public_values_hash: [u8; 32],
    pub public_value_field_count: usize,
    pub program_image_cache_count: usize,
    pub program_image_caches: Vec<ProgramImageCommitmentCache>,
    pub eth_block_input_count: usize,
    pub eth_block_input_hashes: Vec<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofPreflightError {
    SetupHashMismatch,
    PublicValuesDigest(PublicValuesError),
    PublicValuesHashMismatch,
    PublicValuesField(PublicValueFieldError),
    ProgramImageCache(ProgramImageCacheSegmentError),
    EthBlockInput(EthBlockInputError),
    EthBlockPublicValues(EthBlockPublicValuesError),
    MissingEthBlockInput,
    ProofArtifact(ProofArtifactError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicValueFieldError {
    Field(FieldError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofPreflightFileError {
    Proof(ProofArtifactError),
    PublicValues(PublicValuesError),
    ProofPreflight(ProofPreflightError),
}

impl fmt::Display for ProofPreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupHashMismatch => write!(f, "setup hash mismatch"),
            Self::PublicValuesDigest(error) => write!(f, "{error}"),
            Self::PublicValuesHashMismatch => write!(f, "public-values hash mismatch"),
            Self::PublicValuesField(error) => write!(f, "{error}"),
            Self::ProgramImageCache(error) => write!(f, "{error}"),
            Self::EthBlockInput(error) => write!(f, "{error}"),
            Self::EthBlockPublicValues(error) => write!(f, "{error}"),
            Self::MissingEthBlockInput => write!(f, "missing ETH block input proof segment"),
            Self::ProofArtifact(error) => write!(f, "{error}"),
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

impl fmt::Display for ProofPreflightFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proof(error) => write!(f, "{error}"),
            Self::PublicValues(error) => write!(f, "{error}"),
            Self::ProofPreflight(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ProofPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PublicValuesDigest(error) => Some(error),
            Self::PublicValuesField(error) => Some(error),
            Self::ProgramImageCache(error) => Some(error),
            Self::EthBlockInput(error) => Some(error),
            Self::EthBlockPublicValues(error) => Some(error),
            Self::ProofArtifact(error) => Some(error),
            Self::SetupHashMismatch
            | Self::PublicValuesHashMismatch
            | Self::MissingEthBlockInput => None,
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

impl std::error::Error for ProofPreflightFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Proof(error) => Some(error),
            Self::PublicValues(error) => Some(error),
            Self::ProofPreflight(error) => Some(error),
        }
    }
}

impl From<ProofArtifactError> for ProofPreflightFileError {
    fn from(error: ProofArtifactError) -> Self {
        Self::Proof(error)
    }
}

impl From<PublicValuesError> for ProofPreflightFileError {
    fn from(error: PublicValuesError) -> Self {
        Self::PublicValues(error)
    }
}

impl From<ProofPreflightError> for ProofPreflightFileError {
    fn from(error: ProofPreflightError) -> Self {
        Self::ProofPreflight(error)
    }
}

pub fn validate_proof_public_values(
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<ProofPreflightReport, ProofPreflightError> {
    validate_proof_artifact(proof).map_err(ProofPreflightError::ProofArtifact)?;

    if proof.setup_hash != public_values.setup_hash {
        return Err(ProofPreflightError::SetupHashMismatch);
    }

    let digest =
        public_values_digest(public_values).map_err(ProofPreflightError::PublicValuesDigest)?;
    if proof.public_values_hash != digest {
        return Err(ProofPreflightError::PublicValuesHashMismatch);
    }
    let public_value_fields =
        public_values_as_fields(public_values).map_err(ProofPreflightError::PublicValuesField)?;
    let mut program_image_caches = Vec::new();
    for segment in proof
        .segments
        .iter()
        .filter(|segment| segment.id == PROGRAM_IMAGE_CACHE_SEGMENT_ID)
    {
        let cache = parse_program_image_cache_segment(&segment.data)
            .map_err(ProofPreflightError::ProgramImageCache)?;
        program_image_caches.push(cache);
    }
    let program_image_cache_count = program_image_caches.len();
    let mut eth_block_input_hashes = Vec::new();
    let eth_block_input_count = proof
        .segments
        .iter()
        .filter(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .count();
    if eth_block_input_count == 0 && contains_eth_block_public_values(public_values) {
        return Err(ProofPreflightError::MissingEthBlockInput);
    }
    for segment in proof
        .segments
        .iter()
        .filter(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
    {
        eth_block_input_hashes.push(eth_block_input_bytes_digest(&segment.data));
        let input = parse_eth_block_input_segment(&segment.data)
            .map_err(ProofPreflightError::EthBlockInput)?;
        validate_eth_block_public_values(&input, public_values)
            .map_err(ProofPreflightError::EthBlockPublicValues)?;
    }

    Ok(ProofPreflightReport {
        segment_count: proof.segments.len(),
        public_value_count: public_values.values.len(),
        public_values_hash: digest,
        public_value_field_count: public_value_fields.len(),
        program_image_cache_count,
        program_image_caches,
        eth_block_input_count,
        eth_block_input_hashes,
    })
}

fn contains_eth_block_public_values(public_values: &PublicValues) -> bool {
    public_values
        .values
        .iter()
        .any(|entry| is_eth_block_public_value_name(&entry.name))
}

fn is_eth_block_public_value_name(name: &str) -> bool {
    matches!(
        name,
        "eth_block_hash_u32_be"
            | "eth_block_number_u32_le"
            | "eth_block_timestamp_u32_le"
            | "eth_ommers_hash_u32_be"
            | "eth_transactions_root_u32_be"
            | "eth_withdrawals_root_present"
            | "eth_withdrawals_root_u32_be"
    )
}

pub fn validate_proof_public_values_from_files(
    proof_path: impl AsRef<Path>,
    public_values_path: impl AsRef<Path>,
) -> Result<ProofPreflightReport, ProofPreflightFileError> {
    let proof = read_proof_artifact_file(proof_path)?;
    let public_values = read_public_values_file(public_values_path)?;
    validate_proof_public_values(&proof, &public_values).map_err(Into::into)
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

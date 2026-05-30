use std::fmt;
use std::path::Path;

use lzvm_artifacts::challenge_values_segment::{
    parse_challenge_values_segment, ChallengeValuesSegmentError, CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_input::{
    eth_block_input_bytes_digest, eth_block_input_extra_field_counts,
    eth_block_input_receipt_kind_counts, eth_block_input_transaction_kind_counts,
    eth_block_input_withdrawal_count, EthBlockInputError,
};
use lzvm_artifacts::eth_block_input_segment::{
    parse_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::{
    validate_eth_block_public_values, EthBlockPublicValuesError,
};
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::program_image_segment::{
    parse_program_image_cache_segment, program_image_cache_segment_digest,
    ProgramImageCacheSegmentError, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
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
    pub program_image_cache_hashes: Vec<[u8; 32]>,
    pub challenge_values_segment_count: usize,
    pub challenge_values_segment_byte_counts: Vec<usize>,
    pub challenge_values_value_counts: Vec<usize>,
    pub eth_block_input_count: usize,
    pub eth_block_input_hashes: Vec<[u8; 32]>,
    pub eth_block_input_byte_counts: Vec<usize>,
    pub eth_block_input_block_rlp_byte_counts: Vec<usize>,
    pub eth_block_input_extra_header_field_counts: Vec<usize>,
    pub eth_block_input_extra_body_field_counts: Vec<usize>,
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
    pub eth_block_input_receipts_rlp_byte_counts: Vec<Option<usize>>,
    pub eth_block_input_receipt_preimage_counts: Vec<Option<usize>>,
    pub eth_block_input_legacy_receipt_counts: Vec<Option<usize>>,
    pub eth_block_input_typed_receipt_counts: Vec<Option<usize>>,
    pub eth_block_input_withdrawal_roots: Vec<Option<[u8; 32]>>,
    pub eth_block_input_withdrawal_counts: Vec<Option<usize>>,
    pub eth_block_input_withdrawal_preimage_counts: Vec<Option<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofPreflightError {
    SetupHashMismatch,
    PublicValuesDigest(PublicValuesError),
    PublicValuesHashMismatch,
    PublicValuesField(PublicValueFieldError),
    ProgramImageCache(ProgramImageCacheSegmentError),
    ProgramImageCacheSetupHashMismatch,
    ProgramImageCacheTreeRootNonCanonical {
        word_index: usize,
        source: FieldError,
    },
    MissingProgramImageCachePublicValue {
        name: String,
    },
    ProgramImageCachePublicValueElementCountMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    ProgramImageCachePublicValueMismatch {
        name: String,
    },
    ChallengeValues(ChallengeValuesSegmentError),
    ChallengeValueNonCanonical {
        value_index: usize,
        word_index: usize,
        source: FieldError,
    },
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
            Self::ProgramImageCacheSetupHashMismatch => {
                write!(f, "program image cache setup hash mismatch")
            }
            Self::ProgramImageCacheTreeRootNonCanonical { word_index, source } => write!(
                f,
                "program image cache tree root word {word_index} is non-canonical: {source}"
            ),
            Self::MissingProgramImageCachePublicValue { name } => {
                write!(
                    f,
                    "missing program image cache proof segment for public value: {name}"
                )
            }
            Self::ProgramImageCachePublicValueElementCountMismatch {
                name,
                expected,
                found,
            } => {
                write!(
                    f,
                    "program image cache public value {name} element count mismatch: expected {expected}, found {found}"
                )
            }
            Self::ProgramImageCachePublicValueMismatch { name } => {
                write!(
                    f,
                    "program image cache tree root does not match public value: {name}"
                )
            }
            Self::ChallengeValues(error) => write!(f, "{error}"),
            Self::ChallengeValueNonCanonical {
                value_index,
                word_index,
                source,
            } => write!(
                f,
                "invalid challenge values segment value {value_index} word {word_index}: {source}"
            ),
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
            Self::ProgramImageCacheTreeRootNonCanonical { source, .. } => Some(source),
            Self::ChallengeValues(error) => Some(error),
            Self::ChallengeValueNonCanonical { source, .. } => Some(source),
            Self::EthBlockInput(error) => Some(error),
            Self::EthBlockPublicValues(error) => Some(error),
            Self::ProofArtifact(error) => Some(error),
            Self::SetupHashMismatch
            | Self::PublicValuesHashMismatch
            | Self::ProgramImageCacheSetupHashMismatch
            | Self::MissingProgramImageCachePublicValue { .. }
            | Self::ProgramImageCachePublicValueElementCountMismatch { .. }
            | Self::ProgramImageCachePublicValueMismatch { .. }
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
    let mut program_image_cache_hashes = Vec::new();
    for segment in proof
        .segments
        .iter()
        .filter(|segment| segment.id == PROGRAM_IMAGE_CACHE_SEGMENT_ID)
    {
        let cache = parse_program_image_cache_segment(&segment.data)
            .map_err(ProofPreflightError::ProgramImageCache)?;
        validate_program_image_cache_tree_root_canonical(&cache)?;
        if cache.constraint_system_digest != proof.setup_hash {
            return Err(ProofPreflightError::ProgramImageCacheSetupHashMismatch);
        }
        program_image_cache_hashes.push(program_image_cache_segment_digest(&segment.data));
        program_image_caches.push(cache);
    }
    let program_image_cache_count = program_image_caches.len();
    validate_program_image_cache_public_values(&program_image_caches, public_values)?;
    let mut challenge_values_segment_byte_counts = Vec::new();
    let mut challenge_values_value_counts = Vec::new();
    for segment in proof
        .segments
        .iter()
        .filter(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID)
    {
        let challenge_values = parse_challenge_values_segment(&segment.data)
            .map_err(ProofPreflightError::ChallengeValues)?;
        validate_challenge_values_canonical(&challenge_values.values)?;
        challenge_values_segment_byte_counts.push(segment.data.len());
        challenge_values_value_counts.push(challenge_values.values.len());
    }
    let challenge_values_segment_count = challenge_values_value_counts.len();
    let mut eth_block_input_hashes = Vec::new();
    let mut eth_block_input_byte_counts = Vec::new();
    let mut eth_block_input_block_rlp_byte_counts = Vec::new();
    let mut eth_block_input_extra_header_field_counts = Vec::new();
    let mut eth_block_input_extra_body_field_counts = Vec::new();
    let mut eth_block_input_block_hashes = Vec::new();
    let mut eth_block_input_parent_hashes = Vec::new();
    let mut eth_block_input_transaction_preimage_counts = Vec::new();
    let mut eth_block_input_legacy_transaction_counts = Vec::new();
    let mut eth_block_input_typed_transaction_counts = Vec::new();
    let mut eth_block_input_receipts_rlp_byte_counts = Vec::new();
    let mut eth_block_input_receipt_preimage_counts = Vec::new();
    let mut eth_block_input_legacy_receipt_counts = Vec::new();
    let mut eth_block_input_typed_receipt_counts = Vec::new();
    let mut eth_block_input_withdrawal_roots = Vec::new();
    let mut eth_block_input_withdrawal_counts = Vec::new();
    let mut eth_block_input_withdrawal_preimage_counts = Vec::new();
    let eth_block_input_count = proof
        .segments
        .iter()
        .filter(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .count();
    let mut eth_block_input_ommers_hashes = Vec::new();
    let mut eth_block_input_beneficiaries = Vec::new();
    let mut eth_block_input_state_roots = Vec::new();
    let mut eth_block_input_receipt_roots = Vec::new();
    let mut eth_block_input_logs_blooms = Vec::new();
    let mut eth_block_input_difficulties = Vec::new();
    let mut eth_block_input_block_numbers = Vec::new();
    let mut eth_block_input_timestamps = Vec::new();
    let mut eth_block_input_extra_data = Vec::new();
    let mut eth_block_input_gas_limits = Vec::new();
    let mut eth_block_input_gas_used_values = Vec::new();
    let mut eth_block_input_base_fees_per_gas = Vec::new();
    let mut eth_block_input_mix_hashes = Vec::new();
    let mut eth_block_input_nonces = Vec::new();
    let mut eth_block_input_transaction_roots = Vec::new();
    if eth_block_input_count == 0 && contains_named_eth_block_public_values(public_values) {
        return Err(ProofPreflightError::MissingEthBlockInput);
    }
    for segment in proof
        .segments
        .iter()
        .filter(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
    {
        let input = parse_eth_block_input_segment(&segment.data)
            .map_err(ProofPreflightError::EthBlockInput)?;
        eth_block_input_hashes.push(eth_block_input_bytes_digest(&segment.data));
        eth_block_input_byte_counts.push(segment.data.len());
        eth_block_input_block_rlp_byte_counts.push(input.block_rlp.len());
        let (extra_header_field_count, extra_body_field_count) =
            eth_block_input_extra_field_counts(&input)
                .map_err(ProofPreflightError::EthBlockInput)?;
        eth_block_input_extra_header_field_counts.push(extra_header_field_count);
        eth_block_input_extra_body_field_counts.push(extra_body_field_count);
        eth_block_input_block_hashes.push(input.block_hash);
        eth_block_input_parent_hashes.push(input.parent_hash);
        eth_block_input_ommers_hashes.push(input.ommers_hash);
        eth_block_input_beneficiaries.push(input.beneficiary);
        eth_block_input_state_roots.push(input.state_root);
        eth_block_input_receipt_roots.push(input.receipts_root);
        eth_block_input_logs_blooms.push(input.logs_bloom);
        eth_block_input_difficulties.push(input.difficulty);
        eth_block_input_block_numbers.push(input.block_number);
        eth_block_input_timestamps.push(input.timestamp);
        eth_block_input_extra_data.push(input.extra_data.clone());
        eth_block_input_gas_limits.push(input.gas_limit);
        eth_block_input_gas_used_values.push(input.gas_used);
        eth_block_input_base_fees_per_gas.push(input.base_fee_per_gas);
        eth_block_input_mix_hashes.push(input.mix_hash);
        eth_block_input_nonces.push(input.nonce);
        eth_block_input_transaction_roots.push(input.transactions_root);
        let (legacy_transaction_count, typed_transaction_count) =
            eth_block_input_transaction_kind_counts(&input)
                .map_err(ProofPreflightError::EthBlockInput)?;
        let receipt_kind_counts = eth_block_input_receipt_kind_counts(&input)
            .map_err(ProofPreflightError::EthBlockInput)?;
        let withdrawal_count =
            eth_block_input_withdrawal_count(&input).map_err(ProofPreflightError::EthBlockInput)?;
        eth_block_input_transaction_preimage_counts.push(input.transactions.hash_preimages.len());
        eth_block_input_legacy_transaction_counts.push(legacy_transaction_count);
        eth_block_input_typed_transaction_counts.push(typed_transaction_count);
        eth_block_input_receipts_rlp_byte_counts.push(
            input
                .receipts_rlp
                .as_ref()
                .map(|receipts_rlp| receipts_rlp.len()),
        );
        eth_block_input_receipt_preimage_counts.push(
            input
                .receipts
                .as_ref()
                .map(|receipts| receipts.hash_preimages.len()),
        );
        eth_block_input_legacy_receipt_counts
            .push(receipt_kind_counts.map(|(legacy_count, _)| legacy_count));
        eth_block_input_typed_receipt_counts
            .push(receipt_kind_counts.map(|(_, typed_count)| typed_count));
        eth_block_input_withdrawal_roots.push(input.withdrawals_root);
        eth_block_input_withdrawal_counts.push(withdrawal_count);
        eth_block_input_withdrawal_preimage_counts.push(
            input
                .withdrawals
                .as_ref()
                .map(|withdrawals| withdrawals.hash_preimages.len()),
        );
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
        program_image_cache_hashes,
        challenge_values_segment_count,
        challenge_values_segment_byte_counts,
        challenge_values_value_counts,
        eth_block_input_count,
        eth_block_input_hashes,
        eth_block_input_byte_counts,
        eth_block_input_block_rlp_byte_counts,
        eth_block_input_extra_header_field_counts,
        eth_block_input_extra_body_field_counts,
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
        eth_block_input_receipts_rlp_byte_counts,
        eth_block_input_receipt_preimage_counts,
        eth_block_input_legacy_receipt_counts,
        eth_block_input_typed_receipt_counts,
        eth_block_input_withdrawal_roots,
        eth_block_input_withdrawal_counts,
        eth_block_input_withdrawal_preimage_counts,
    })
}

pub(crate) fn contains_named_eth_block_public_values(public_values: &PublicValues) -> bool {
    public_values
        .values
        .iter()
        .any(|entry| is_eth_block_public_value_name(&entry.name))
}

fn validate_challenge_values_canonical(values: &[[u64; 3]]) -> Result<(), ProofPreflightError> {
    for (value_index, words) in values.iter().enumerate() {
        for (word_index, word) in words.iter().copied().enumerate() {
            Felt::from_canonical(word).map_err(|source| {
                ProofPreflightError::ChallengeValueNonCanonical {
                    value_index,
                    word_index,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn validate_program_image_cache_tree_root_canonical(
    cache: &ProgramImageCommitmentCache,
) -> Result<(), ProofPreflightError> {
    for (word_index, word) in cache.tree_root.iter().copied().enumerate() {
        Felt::from_canonical(word).map_err(|source| {
            ProofPreflightError::ProgramImageCacheTreeRootNonCanonical { word_index, source }
        })?;
    }
    Ok(())
}

fn validate_program_image_cache_public_values(
    caches: &[ProgramImageCommitmentCache],
    public_values: &PublicValues,
) -> Result<(), ProofPreflightError> {
    for entry in &public_values.values {
        if entry.name == "rom_root" {
            if entry.elements.len() != 4 {
                return Err(
                    ProofPreflightError::ProgramImageCachePublicValueElementCountMismatch {
                        name: entry.name.clone(),
                        expected: 4,
                        found: entry.elements.len(),
                    },
                );
            }
            let Some(cache) = caches.first() else {
                return Err(ProofPreflightError::MissingProgramImageCachePublicValue {
                    name: entry.name.clone(),
                });
            };
            if entry.elements.as_slice() != cache.tree_root.as_slice() {
                return Err(ProofPreflightError::ProgramImageCachePublicValueMismatch {
                    name: entry.name.clone(),
                });
            }
        }
    }
    Ok(())
}

fn is_eth_block_public_value_name(name: &str) -> bool {
    matches!(
        name,
        "eth_block_hash_u32_be"
            | "eth_parent_hash_u32_be"
            | "eth_beneficiary_u32_be"
            | "eth_state_root_u32_be"
            | "eth_receipts_root_u32_be"
            | "eth_logs_bloom_u32_be"
            | "eth_difficulty_u32_be"
            | "eth_block_number_u32_le"
            | "eth_block_timestamp_u32_le"
            | "eth_extra_data_len"
            | "eth_extra_data_u32_be"
            | "eth_gas_limit_u32_le"
            | "eth_gas_used_u32_le"
            | "eth_base_fee_per_gas_present"
            | "eth_base_fee_per_gas_u32_be"
            | "eth_mix_hash_u32_be"
            | "eth_nonce_u32_be"
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

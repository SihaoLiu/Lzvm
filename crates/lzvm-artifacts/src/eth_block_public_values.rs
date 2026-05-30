use std::fmt;

use crate::eth_block_input::EthBlockInput;
use crate::global_info::{GlobalInfo, PublicValue};
use crate::program_image::ProgramImageCommitmentCache;
use crate::public_values::{PublicValueEntry, PublicValues};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthBlockPublicValuesError {
    MissingEntry {
        name: String,
    },
    ValueMismatch {
        name: String,
    },
    MissingProgramImageCache {
        name: String,
    },
    ProgramImageCachePublicValueElementCountMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    ProgramImageCacheSetupHashMismatch,
    ProgramImageCacheMismatch {
        name: String,
    },
    UnsupportedPublicMetadata {
        name: String,
    },
    PublicMetadataCountOverflow {
        name: String,
    },
    ExtraDataOverflow {
        max_bytes: usize,
        found: usize,
    },
}

impl fmt::Display for EthBlockPublicValuesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntry { name } => write!(f, "missing ETH block public value: {name}"),
            Self::ValueMismatch { name } => {
                write!(f, "ETH block public value mismatch: {name}")
            }
            Self::MissingProgramImageCache { name } => {
                write!(
                    f,
                    "program image cache is required for public value: {name}"
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
            Self::ProgramImageCacheSetupHashMismatch => {
                write!(f, "program image cache setup hash mismatch")
            }
            Self::ProgramImageCacheMismatch { name } => {
                write!(
                    f,
                    "program image cache tree root does not match public value: {name}"
                )
            }
            Self::UnsupportedPublicMetadata { name } => {
                write!(f, "unsupported ETH block public metadata: {name}")
            }
            Self::PublicMetadataCountOverflow { name } => {
                write!(f, "ETH block public metadata count overflow: {name}")
            }
            Self::ExtraDataOverflow { max_bytes, found } => {
                write!(
                    f,
                    "ETH block public value extra data exceeds {max_bytes} bytes, found {found}"
                )
            }
        }
    }
}

impl std::error::Error for EthBlockPublicValuesError {}

pub fn public_values_from_eth_block_input(
    setup_hash: [u8; 32],
    input: &EthBlockInput,
) -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash,
        values: eth_block_public_value_entries(input),
    }
}

pub fn public_values_from_eth_block_input_for_metadata(
    setup_hash: [u8; 32],
    input: &EthBlockInput,
    global_info: &GlobalInfo,
    program_image_cache: Option<&ProgramImageCommitmentCache>,
) -> Result<PublicValues, EthBlockPublicValuesError> {
    let mut values = Vec::with_capacity(global_info.publics_map.len());
    for metadata in &global_info.publics_map {
        let elements =
            metadata_eth_block_elements(setup_hash, metadata, input, program_image_cache)?;
        values.push(PublicValueEntry {
            name: metadata.name.clone(),
            elements,
        });
    }
    Ok(PublicValues {
        schema_version: 1,
        setup_hash,
        values,
    })
}

pub fn validate_eth_block_public_values(
    input: &EthBlockInput,
    public_values: &PublicValues,
) -> Result<(), EthBlockPublicValuesError> {
    let has_packed_inputs = if let Some(entry) = public_values
        .values
        .iter()
        .find(|entry| entry.name == "inputs")
    {
        if entry.elements.len() == 64 {
            validate_packed_eth_block_public_outputs(input, entry)?;
            true
        } else {
            false
        }
    } else {
        false
    };

    if has_packed_inputs {
        validate_present_eth_block_public_values(input, public_values)?;
        return Ok(());
    }

    let mut matched_eth_entry = false;
    for entry in &public_values.values {
        let Some(expected_elements) = eth_block_public_value_elements(input, &entry.name)? else {
            continue;
        };
        matched_eth_entry = true;
        if entry.elements != expected_elements {
            return Err(EthBlockPublicValuesError::ValueMismatch {
                name: entry.name.clone(),
            });
        }
    }
    if !matched_eth_entry {
        let name = ETH_BLOCK_PUBLIC_VALUE_NAMES
            .first()
            .map(|name| (*name).to_owned())
            .unwrap_or_else(|| "eth_block_hash_u32_be".to_owned());
        return Err(EthBlockPublicValuesError::MissingEntry { name });
    }
    Ok(())
}

pub fn validate_eth_block_public_values_with_program_image_cache(
    input: &EthBlockInput,
    public_values: &PublicValues,
    program_image_cache: Option<&ProgramImageCommitmentCache>,
) -> Result<(), EthBlockPublicValuesError> {
    validate_eth_block_public_values(input, public_values)?;
    validate_program_image_cache_public_values(public_values, program_image_cache)
}

fn eth_block_public_value_entries(input: &EthBlockInput) -> Vec<PublicValueEntry> {
    ETH_BLOCK_PUBLIC_VALUE_NAMES
        .iter()
        .map(|name| PublicValueEntry {
            name: (*name).to_owned(),
            elements: eth_block_public_value_elements(input, name)
                .expect("ETH block input dynamic public value fields are checked")
                .expect("ETH block public value name is known"),
        })
        .collect()
}

const ETH_BLOCK_PUBLIC_VALUE_NAMES: &[&str] = &[
    "eth_block_hash_u32_be",
    "eth_parent_hash_u32_be",
    "eth_beneficiary_u32_be",
    "eth_state_root_u32_be",
    "eth_receipts_root_u32_be",
    "eth_logs_bloom_u32_be",
    "eth_difficulty_u32_be",
    "eth_block_number_u32_le",
    "eth_block_timestamp_u32_le",
    "eth_extra_data_len",
    "eth_extra_data_u32_be",
    "eth_gas_limit_u32_le",
    "eth_gas_used_u32_le",
    "eth_base_fee_per_gas_present",
    "eth_base_fee_per_gas_u32_be",
    "eth_mix_hash_u32_be",
    "eth_nonce_u32_be",
    "eth_ommers_hash_u32_be",
    "eth_transactions_root_u32_be",
    "eth_withdrawals_root_present",
    "eth_withdrawals_root_u32_be",
];

fn eth_block_public_value_elements(
    input: &EthBlockInput,
    name: &str,
) -> Result<Option<Vec<u64>>, EthBlockPublicValuesError> {
    let elements = match name {
        "eth_block_hash_u32_be" => hash_u32_be(&input.block_hash),
        "eth_parent_hash_u32_be" => hash_u32_be(&input.parent_hash),
        "eth_beneficiary_u32_be" => bytes_u32_be(&input.beneficiary),
        "eth_state_root_u32_be" => hash_u32_be(&input.state_root),
        "eth_receipts_root_u32_be" => hash_u32_be(&input.receipts_root),
        "eth_logs_bloom_u32_be" => bytes_u32_be(&input.logs_bloom),
        "eth_difficulty_u32_be" => hash_u32_be(&input.difficulty),
        "eth_block_number_u32_le" => u64_u32_le(input.block_number),
        "eth_block_timestamp_u32_le" => u64_u32_le(input.timestamp),
        "eth_extra_data_len" => vec![input.extra_data.len() as u64],
        "eth_extra_data_u32_be" => padded_32_bytes_u32_be(&input.extra_data)?,
        "eth_gas_limit_u32_le" => u64_u32_le(input.gas_limit),
        "eth_gas_used_u32_le" => u64_u32_le(input.gas_used),
        "eth_base_fee_per_gas_present" => vec![u64::from(input.base_fee_per_gas.is_some())],
        "eth_base_fee_per_gas_u32_be" => input
            .base_fee_per_gas
            .as_ref()
            .map(hash_u32_be)
            .unwrap_or_else(|| vec![0; 8]),
        "eth_mix_hash_u32_be" => hash_u32_be(&input.mix_hash),
        "eth_nonce_u32_be" => bytes_u32_be(&input.nonce),
        "eth_ommers_hash_u32_be" => hash_u32_be(&input.ommers_hash),
        "eth_transactions_root_u32_be" => hash_u32_be(&input.transactions_root),
        "eth_withdrawals_root_present" => vec![u64::from(input.withdrawals_root.is_some())],
        "eth_withdrawals_root_u32_be" => input
            .withdrawals_root
            .as_ref()
            .map(hash_u32_be)
            .unwrap_or_else(|| vec![0; 8]),
        _ => return Ok(None),
    };
    Ok(Some(elements))
}

fn metadata_eth_block_elements(
    setup_hash: [u8; 32],
    metadata: &PublicValue,
    input: &EthBlockInput,
    program_image_cache: Option<&ProgramImageCommitmentCache>,
) -> Result<Vec<u64>, EthBlockPublicValuesError> {
    let count = public_value_element_count(metadata)?;
    if metadata.name == "rom_root" && count == 4 {
        let cache = program_image_cache.ok_or_else(|| {
            EthBlockPublicValuesError::MissingProgramImageCache {
                name: metadata.name.clone(),
            }
        })?;
        if cache.constraint_system_digest != setup_hash {
            return Err(EthBlockPublicValuesError::ProgramImageCacheSetupHashMismatch);
        }
        return Ok(cache.tree_root.to_vec());
    }
    if metadata.name == "inputs" && count == 64 {
        return Ok(packed_block_hash_public_outputs(input, count));
    }
    if let Some(elements) = eth_block_public_value_elements(input, &metadata.name)? {
        if elements.len() == count {
            return Ok(elements);
        }
    }
    Err(EthBlockPublicValuesError::UnsupportedPublicMetadata {
        name: metadata.name.clone(),
    })
}

fn public_value_element_count(metadata: &PublicValue) -> Result<usize, EthBlockPublicValuesError> {
    let count = if metadata.lengths.is_empty() {
        1_u64
    } else {
        metadata.lengths.iter().try_fold(1_u64, |count, length| {
            count.checked_mul(*length).ok_or_else(|| {
                EthBlockPublicValuesError::PublicMetadataCountOverflow {
                    name: metadata.name.clone(),
                }
            })
        })?
    };
    usize::try_from(count).map_err(|_| EthBlockPublicValuesError::PublicMetadataCountOverflow {
        name: metadata.name.clone(),
    })
}

fn validate_packed_eth_block_public_outputs(
    input: &EthBlockInput,
    entry: &PublicValueEntry,
) -> Result<(), EthBlockPublicValuesError> {
    let expected = packed_block_hash_public_outputs(input, entry.elements.len());
    if entry.elements != expected {
        return Err(EthBlockPublicValuesError::ValueMismatch {
            name: entry.name.clone(),
        });
    }
    Ok(())
}

fn validate_present_eth_block_public_values(
    input: &EthBlockInput,
    public_values: &PublicValues,
) -> Result<(), EthBlockPublicValuesError> {
    for entry in &public_values.values {
        let Some(expected_elements) = eth_block_public_value_elements(input, &entry.name)? else {
            continue;
        };
        if entry.elements != expected_elements {
            return Err(EthBlockPublicValuesError::ValueMismatch {
                name: entry.name.clone(),
            });
        }
    }
    Ok(())
}

pub fn validate_program_image_cache_public_values(
    public_values: &PublicValues,
    program_image_cache: Option<&ProgramImageCommitmentCache>,
) -> Result<(), EthBlockPublicValuesError> {
    for entry in &public_values.values {
        if entry.name == "rom_root" {
            if entry.elements.len() != 4 {
                return Err(
                    EthBlockPublicValuesError::ProgramImageCachePublicValueElementCountMismatch {
                        name: entry.name.clone(),
                        expected: 4,
                        found: entry.elements.len(),
                    },
                );
            }
            let cache = program_image_cache.ok_or_else(|| {
                EthBlockPublicValuesError::MissingProgramImageCache {
                    name: entry.name.clone(),
                }
            })?;
            if cache.constraint_system_digest != public_values.setup_hash {
                return Err(EthBlockPublicValuesError::ProgramImageCacheSetupHashMismatch);
            }
            if entry.elements.as_slice() != cache.tree_root.as_slice() {
                return Err(EthBlockPublicValuesError::ProgramImageCacheMismatch {
                    name: entry.name.clone(),
                });
            }
        }
    }
    Ok(())
}

fn packed_block_hash_public_outputs(input: &EthBlockInput, count: usize) -> Vec<u64> {
    let mut outputs = input
        .block_hash
        .chunks_exact(4)
        .map(|chunk| {
            u64::from(u32::from_le_bytes(
                chunk.try_into().expect("chunk has 4 bytes"),
            ))
        })
        .collect::<Vec<_>>();
    outputs.resize(count, 0);
    outputs
}

fn hash_u32_be(bytes: &[u8; 32]) -> Vec<u64> {
    bytes_u32_be(bytes)
}

fn bytes_u32_be(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            u64::from(u32::from_be_bytes(
                chunk.try_into().expect("chunk has 4 bytes"),
            ))
        })
        .collect()
}

fn padded_32_bytes_u32_be(bytes: &[u8]) -> Result<Vec<u64>, EthBlockPublicValuesError> {
    if bytes.len() > 32 {
        return Err(EthBlockPublicValuesError::ExtraDataOverflow {
            max_bytes: 32,
            found: bytes.len(),
        });
    }
    let mut padded = [0_u8; 32];
    padded[..bytes.len()].copy_from_slice(bytes);
    Ok(bytes_u32_be(&padded))
}

fn u64_u32_le(value: u64) -> Vec<u64> {
    vec![value & 0xffff_ffff, value >> 32]
}

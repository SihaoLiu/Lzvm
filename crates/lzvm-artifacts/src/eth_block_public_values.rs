use std::fmt;

use crate::eth_block_input::EthBlockInput;
use crate::public_values::{PublicValueEntry, PublicValues};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthBlockPublicValuesError {
    MissingEntry { name: String },
    ValueMismatch { name: String },
}

impl fmt::Display for EthBlockPublicValuesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntry { name } => write!(f, "missing ETH block public value: {name}"),
            Self::ValueMismatch { name } => {
                write!(f, "ETH block public value mismatch: {name}")
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
        values: vec![
            PublicValueEntry {
                name: "eth_block_hash_u32_be".to_owned(),
                elements: hash_u32_be(&input.block_hash),
            },
            PublicValueEntry {
                name: "eth_parent_hash_u32_be".to_owned(),
                elements: hash_u32_be(&input.parent_hash),
            },
            PublicValueEntry {
                name: "eth_beneficiary_u32_be".to_owned(),
                elements: bytes_u32_be(&input.beneficiary),
            },
            PublicValueEntry {
                name: "eth_state_root_u32_be".to_owned(),
                elements: hash_u32_be(&input.state_root),
            },
            PublicValueEntry {
                name: "eth_receipts_root_u32_be".to_owned(),
                elements: hash_u32_be(&input.receipts_root),
            },
            PublicValueEntry {
                name: "eth_logs_bloom_u32_be".to_owned(),
                elements: bytes_u32_be(&input.logs_bloom),
            },
            PublicValueEntry {
                name: "eth_difficulty_u32_be".to_owned(),
                elements: hash_u32_be(&input.difficulty),
            },
            PublicValueEntry {
                name: "eth_block_number_u32_le".to_owned(),
                elements: u64_u32_le(input.block_number),
            },
            PublicValueEntry {
                name: "eth_block_timestamp_u32_le".to_owned(),
                elements: u64_u32_le(input.timestamp),
            },
            PublicValueEntry {
                name: "eth_gas_limit_u32_le".to_owned(),
                elements: u64_u32_le(input.gas_limit),
            },
            PublicValueEntry {
                name: "eth_gas_used_u32_le".to_owned(),
                elements: u64_u32_le(input.gas_used),
            },
            PublicValueEntry {
                name: "eth_base_fee_per_gas_present".to_owned(),
                elements: vec![u64::from(input.base_fee_per_gas.is_some())],
            },
            PublicValueEntry {
                name: "eth_base_fee_per_gas_u32_be".to_owned(),
                elements: input
                    .base_fee_per_gas
                    .as_ref()
                    .map(hash_u32_be)
                    .unwrap_or_else(|| vec![0; 8]),
            },
            PublicValueEntry {
                name: "eth_mix_hash_u32_be".to_owned(),
                elements: hash_u32_be(&input.mix_hash),
            },
            PublicValueEntry {
                name: "eth_nonce_u32_be".to_owned(),
                elements: bytes_u32_be(&input.nonce),
            },
            PublicValueEntry {
                name: "eth_ommers_hash_u32_be".to_owned(),
                elements: hash_u32_be(&input.ommers_hash),
            },
            PublicValueEntry {
                name: "eth_transactions_root_u32_be".to_owned(),
                elements: hash_u32_be(&input.transactions_root),
            },
            PublicValueEntry {
                name: "eth_withdrawals_root_present".to_owned(),
                elements: vec![u64::from(input.withdrawals_root.is_some())],
            },
            PublicValueEntry {
                name: "eth_withdrawals_root_u32_be".to_owned(),
                elements: input
                    .withdrawals_root
                    .as_ref()
                    .map(hash_u32_be)
                    .unwrap_or_else(|| vec![0; 8]),
            },
        ],
    }
}

pub fn validate_eth_block_public_values(
    input: &EthBlockInput,
    public_values: &PublicValues,
) -> Result<(), EthBlockPublicValuesError> {
    let expected = public_values_from_eth_block_input(public_values.setup_hash, input);
    for expected_entry in expected.values {
        let entry = public_values
            .values
            .iter()
            .find(|entry| entry.name == expected_entry.name)
            .ok_or_else(|| EthBlockPublicValuesError::MissingEntry {
                name: expected_entry.name.clone(),
            })?;
        if entry.elements != expected_entry.elements {
            return Err(EthBlockPublicValuesError::ValueMismatch {
                name: expected_entry.name,
            });
        }
    }
    Ok(())
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

fn u64_u32_le(value: u64) -> Vec<u64> {
    vec![value & 0xffff_ffff, value >> 32]
}

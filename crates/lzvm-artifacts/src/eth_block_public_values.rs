use crate::eth_block_input::EthBlockInput;
use crate::public_values::{PublicValueEntry, PublicValues};

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
                name: "eth_block_number_u32_le".to_owned(),
                elements: u64_u32_le(input.block_number),
            },
            PublicValueEntry {
                name: "eth_block_timestamp_u32_le".to_owned(),
                elements: u64_u32_le(input.timestamp),
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

fn hash_u32_be(bytes: &[u8; 32]) -> Vec<u64> {
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

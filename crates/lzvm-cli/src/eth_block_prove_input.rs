use std::fs;
use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::eth_block_input::{
    eth_block_input_bytes_digest, parse_eth_block_input, EthBlockInput,
};

use crate::prove_plan::format_hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EthBlockInputSummary {
    pub(crate) path: PathBuf,
    pub(crate) byte_len: u64,
    pub(crate) digest: [u8; 32],
    pub(crate) input: EthBlockInput,
    pub(crate) block_rlp_len: usize,
    pub(crate) block_hash: [u8; 32],
    pub(crate) parent_hash: [u8; 32],
    pub(crate) beneficiary: [u8; 20],
    pub(crate) state_root: [u8; 32],
    pub(crate) receipts_root: [u8; 32],
    pub(crate) difficulty: [u8; 32],
    pub(crate) block_number: u64,
    pub(crate) timestamp: u64,
    pub(crate) gas_limit: u64,
    pub(crate) gas_used: u64,
    pub(crate) base_fee_per_gas: Option<[u8; 32]>,
    pub(crate) mix_hash: [u8; 32],
    pub(crate) nonce: [u8; 8],
    pub(crate) transactions_root: [u8; 32],
    pub(crate) transaction_preimage_count: usize,
    pub(crate) withdrawal_preimage_count: Option<usize>,
}

pub(crate) fn validate_eth_block_input(
    path: &Option<PathBuf>,
) -> Result<Option<EthBlockInputSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let metadata = fs::metadata(path)
        .map_err(|error| format!("ETH block input is missing: {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("ETH block input is not a file: {}", path.display()));
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("ETH block input read failed: {}: {error}", path.display()))?;
    let input = parse_eth_block_input(&bytes)
        .map_err(|error| format!("ETH block input failed: {}: {error}", path.display()))?;

    Ok(Some(EthBlockInputSummary {
        path: path.clone(),
        byte_len: metadata.len(),
        digest: eth_block_input_bytes_digest(&bytes),
        input: input.clone(),
        block_rlp_len: input.block_rlp.len(),
        block_hash: input.block_hash,
        parent_hash: input.parent_hash,
        beneficiary: input.beneficiary,
        state_root: input.state_root,
        receipts_root: input.receipts_root,
        difficulty: input.difficulty,
        block_number: input.block_number,
        timestamp: input.timestamp,
        gas_limit: input.gas_limit,
        gas_used: input.gas_used,
        base_fee_per_gas: input.base_fee_per_gas,
        mix_hash: input.mix_hash,
        nonce: input.nonce,
        transactions_root: input.transactions_root,
        transaction_preimage_count: input.transactions.hash_preimages.len(),
        withdrawal_preimage_count: input
            .withdrawals
            .as_ref()
            .map(|withdrawals| withdrawals.hash_preimages.len()),
    }))
}

pub(crate) fn write_eth_block_input_summary(
    stdout: &mut dyn Write,
    summary: &EthBlockInputSummary,
) {
    let _ = writeln!(stdout, "eth_block_input={}", summary.path.display());
    let _ = writeln!(stdout, "eth_block_input_bytes={}", summary.byte_len);
    let _ = writeln!(
        stdout,
        "eth_block_input_hash={}",
        format_hash(&summary.digest)
    );
    let _ = writeln!(stdout, "eth_block_rlp_bytes={}", summary.block_rlp_len);
    let _ = writeln!(
        stdout,
        "eth_block_hash={}",
        format_hash(&summary.block_hash)
    );
    let _ = writeln!(
        stdout,
        "eth_parent_hash={}",
        format_hash(&summary.parent_hash)
    );
    let _ = writeln!(
        stdout,
        "eth_beneficiary={}",
        format_hex(&summary.beneficiary)
    );
    let _ = writeln!(
        stdout,
        "eth_state_root={}",
        format_hash(&summary.state_root)
    );
    let _ = writeln!(
        stdout,
        "eth_receipts_root={}",
        format_hash(&summary.receipts_root)
    );
    let _ = writeln!(
        stdout,
        "eth_difficulty={}",
        format_u256(&summary.difficulty)
    );
    let _ = writeln!(stdout, "eth_block_number={}", summary.block_number);
    let _ = writeln!(stdout, "eth_block_timestamp={}", summary.timestamp);
    let _ = writeln!(stdout, "eth_gas_limit={}", summary.gas_limit);
    let _ = writeln!(stdout, "eth_gas_used={}", summary.gas_used);
    let _ = writeln!(
        stdout,
        "eth_base_fee_per_gas={}",
        format_optional_u256(summary.base_fee_per_gas.as_ref())
    );
    let _ = writeln!(stdout, "eth_mix_hash={}", format_hash(&summary.mix_hash));
    let _ = writeln!(stdout, "eth_nonce={}", format_hex(&summary.nonce));
    let _ = writeln!(
        stdout,
        "eth_transactions_root={}",
        format_hash(&summary.transactions_root)
    );
    let _ = writeln!(
        stdout,
        "eth_transaction_trie_preimages={}",
        summary.transaction_preimage_count
    );
    match summary.withdrawal_preimage_count {
        Some(count) => {
            let _ = writeln!(stdout, "eth_withdrawals=present");
            let _ = writeln!(stdout, "eth_withdrawal_trie_preimages={count}");
        }
        None => {
            let _ = writeln!(stdout, "eth_withdrawals=absent");
        }
    }
}

fn format_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn format_optional_u256(value: Option<&[u8; 32]>) -> String {
    match value {
        Some(bytes) => format_u256(bytes),
        None => "absent".to_owned(),
    }
}

fn format_u256(bytes: &[u8; 32]) -> String {
    let first = bytes.iter().position(|byte| *byte != 0);
    match first {
        Some(index) => format_hex(&bytes[index..]),
        None => "0".to_owned(),
    }
}

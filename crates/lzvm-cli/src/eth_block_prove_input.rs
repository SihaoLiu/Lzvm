use std::fs;
use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::eth_block_input::{parse_eth_block_input, EthBlockInput};

use crate::prove_plan::format_hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EthBlockInputSummary {
    pub(crate) path: PathBuf,
    pub(crate) byte_len: u64,
    pub(crate) input: EthBlockInput,
    pub(crate) block_rlp_len: usize,
    pub(crate) block_hash: [u8; 32],
    pub(crate) block_number: u64,
    pub(crate) timestamp: u64,
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
        input: input.clone(),
        block_rlp_len: input.block_rlp.len(),
        block_hash: input.block_hash,
        block_number: input.block_number,
        timestamp: input.timestamp,
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
    let _ = writeln!(stdout, "eth_block_rlp_bytes={}", summary.block_rlp_len);
    let _ = writeln!(
        stdout,
        "eth_block_hash={}",
        format_hash(&summary.block_hash)
    );
    let _ = writeln!(stdout, "eth_block_number={}", summary.block_number);
    let _ = writeln!(stdout, "eth_block_timestamp={}", summary.timestamp);
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

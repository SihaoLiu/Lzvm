use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, encode_eth_block_input, eth_block_input_bytes_digest,
    eth_block_input_extra_field_counts, eth_block_input_receipt_kind_counts,
    eth_block_input_transaction_kind_counts, eth_block_input_withdrawal_count,
    parse_eth_block_input, EthBlockInput,
};
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::validate_eth_block_public_values;
use lzvm_artifacts::proof::read_proof_artifact_file;
use lzvm_artifacts::public_values::read_public_values_file;

use crate::eth_block_prove_input::{parse_eth_public_block_for_mode, EthPublicInputMode};

pub(super) struct EthBlockInputBinding {
    pub(super) hash: [u8; 32],
    pub(super) bytes: usize,
    pub(super) block_rlp_bytes: usize,
    pub(super) extra_header_field_count: usize,
    pub(super) extra_body_field_count: usize,
    pub(super) block_hash: [u8; 32],
    pub(super) parent_hash: [u8; 32],
    pub(super) ommers_hash: [u8; 32],
    pub(super) beneficiary: [u8; 20],
    pub(super) state_root: [u8; 32],
    pub(super) receipts_root: [u8; 32],
    pub(super) logs_bloom: [u8; 256],
    pub(super) difficulty: [u8; 32],
    pub(super) block_number: u64,
    pub(super) timestamp: u64,
    pub(super) extra_data: Vec<u8>,
    pub(super) gas_limit: u64,
    pub(super) gas_used: u64,
    pub(super) base_fee_per_gas: Option<[u8; 32]>,
    pub(super) mix_hash: [u8; 32],
    pub(super) nonce: [u8; 8],
    pub(super) transactions_root: [u8; 32],
    pub(super) transaction_preimage_count: usize,
    pub(super) legacy_transaction_count: usize,
    pub(super) typed_transaction_count: usize,
    pub(super) receipts_rlp_bytes: Option<usize>,
    pub(super) receipt_preimage_count: Option<usize>,
    pub(super) legacy_receipt_count: Option<usize>,
    pub(super) typed_receipt_count: Option<usize>,
    pub(super) withdrawal_root: Option<[u8; 32]>,
    pub(super) withdrawal_count: Option<usize>,
    pub(super) withdrawal_preimage_count: Option<usize>,
}

pub(super) fn verify_eth_block_input_binding(
    proof_bin: &str,
    public_values_path: &str,
    input_path: &str,
) -> Result<EthBlockInputBinding, String> {
    let input_bytes = std::fs::read(input_path)
        .map_err(|error| format!("read ETH block input failed: {input_path}: {error}"))?;
    let input = parse_eth_block_input(&input_bytes)
        .map_err(|error| format!("ETH block input failed: {input_path}: {error}"))?;
    verify_eth_block_input_binding_from_input(proof_bin, public_values_path, input, input_bytes)
}

pub(super) fn verify_eth_public_input_binding_with_mode(
    proof_bin: &str,
    public_values_path: &str,
    input_path: &str,
    mode: EthPublicInputMode,
) -> Result<EthBlockInputBinding, String> {
    let public_bytes = std::fs::read(input_path)
        .map_err(|error| format!("read ETH public input failed: {input_path}: {error}"))?;
    let public_block = parse_eth_public_block_for_mode(&public_bytes, mode)
        .map_err(|error| format!("ETH public input failed: {input_path}: {error}"))?;
    let block_rlp = public_block.block_rlp();
    let input = build_eth_block_input(&block_rlp)
        .map_err(|error| format!("ETH public input block failed: {error}"))?;
    let input_bytes = encode_eth_block_input(&input)
        .map_err(|error| format!("ETH public input block failed: {error}"))?;
    verify_eth_block_input_binding_from_input(proof_bin, public_values_path, input, input_bytes)
}

fn verify_eth_block_input_binding_from_input(
    proof_bin: &str,
    public_values_path: &str,
    input: EthBlockInput,
    input_bytes: Vec<u8>,
) -> Result<EthBlockInputBinding, String> {
    let proof = read_proof_artifact_file(proof_bin)
        .map_err(|error| format!("read proof artifact failed: {proof_bin}: {error}"))?;
    let input_hash = eth_block_input_bytes_digest(&input_bytes);
    let transaction_preimage_count = input.transactions.hash_preimages.len();
    let (legacy_transaction_count, typed_transaction_count) =
        eth_block_input_transaction_kind_counts(&input)
            .map_err(|error| format!("ETH block input transaction count failed: {error}"))?;
    let (extra_header_field_count, extra_body_field_count) =
        eth_block_input_extra_field_counts(&input)
            .map_err(|error| format!("ETH block input extra field count failed: {error}"))?;
    let receipt_preimage_count = input
        .receipts
        .as_ref()
        .map(|receipts| receipts.hash_preimages.len());
    let receipts_rlp_bytes = input
        .receipts_rlp
        .as_ref()
        .map(|receipts_rlp| receipts_rlp.len());
    let receipt_kind_counts = eth_block_input_receipt_kind_counts(&input)
        .map_err(|error| format!("ETH block input receipt count failed: {error}"))?;
    let withdrawal_count = eth_block_input_withdrawal_count(&input)
        .map_err(|error| format!("ETH block input withdrawal count failed: {error}"))?;
    let withdrawal_root = input.withdrawals_root;
    let withdrawal_preimage_count = input
        .withdrawals
        .as_ref()
        .map(|withdrawals| withdrawals.hash_preimages.len());
    let expected = encode_eth_block_input_segment(&input)
        .map_err(|error| format!("encode ETH block input segment failed: {error}"))?;
    let segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .ok_or_else(|| "missing ETH block input proof segment".to_owned())?;
    if segment.data != expected {
        return Err("ETH block input proof segment mismatch".to_owned());
    }
    let public_values = read_public_values_file(public_values_path)
        .map_err(|error| format!("read public-values failed: {public_values_path}: {error}"))?;
    validate_eth_block_public_values(&input, &public_values).map_err(|error| error.to_string())?;
    Ok(EthBlockInputBinding {
        hash: input_hash,
        bytes: input_bytes.len(),
        block_rlp_bytes: input.block_rlp.len(),
        extra_header_field_count,
        extra_body_field_count,
        block_hash: input.block_hash,
        parent_hash: input.parent_hash,
        ommers_hash: input.ommers_hash,
        beneficiary: input.beneficiary,
        state_root: input.state_root,
        receipts_root: input.receipts_root,
        logs_bloom: input.logs_bloom,
        difficulty: input.difficulty,
        block_number: input.block_number,
        timestamp: input.timestamp,
        extra_data: input.extra_data,
        gas_limit: input.gas_limit,
        gas_used: input.gas_used,
        base_fee_per_gas: input.base_fee_per_gas,
        mix_hash: input.mix_hash,
        nonce: input.nonce,
        transactions_root: input.transactions_root,
        transaction_preimage_count,
        legacy_transaction_count,
        typed_transaction_count,
        receipts_rlp_bytes,
        receipt_preimage_count,
        legacy_receipt_count: receipt_kind_counts.map(|(legacy_count, _)| legacy_count),
        typed_receipt_count: receipt_kind_counts.map(|(_, typed_count)| typed_count),
        withdrawal_root,
        withdrawal_count,
        withdrawal_preimage_count,
    })
}

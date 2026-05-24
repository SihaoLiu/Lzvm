use crate::setup_preflight::SetupPreflightReport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionEthBlockInputReport {
    pub hash: [u8; 32],
    pub byte_count: usize,
    pub block_rlp_byte_count: usize,
    pub extra_header_field_count: usize,
    pub extra_body_field_count: usize,
    pub block_hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub ommers_hash: [u8; 32],
    pub beneficiary: [u8; 20],
    pub state_root: [u8; 32],
    pub receipts_root: [u8; 32],
    pub logs_bloom: [u8; 256],
    pub difficulty: [u8; 32],
    pub block_number: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub base_fee_per_gas: Option<[u8; 32]>,
    pub mix_hash: [u8; 32],
    pub nonce: [u8; 8],
    pub transactions_root: [u8; 32],
    pub transaction_preimage_count: usize,
    pub legacy_transaction_count: usize,
    pub typed_transaction_count: usize,
    pub receipts_rlp_byte_count: Option<usize>,
    pub receipt_preimage_count: Option<usize>,
    pub legacy_receipt_count: Option<usize>,
    pub typed_receipt_count: Option<usize>,
    pub withdrawal_root: Option<[u8; 32]>,
    pub withdrawal_count: Option<usize>,
    pub withdrawal_preimage_count: Option<usize>,
}

pub(crate) fn contribution_eth_block_input_reports(
    report: &SetupPreflightReport,
) -> Vec<ContributionEthBlockInputReport> {
    (0..report.eth_block_input_hashes.len())
        .map(|index| ContributionEthBlockInputReport {
            hash: report.eth_block_input_hashes[index],
            byte_count: report
                .eth_block_input_byte_counts
                .get(index)
                .copied()
                .unwrap_or_default(),
            block_rlp_byte_count: report
                .eth_block_input_block_rlp_byte_counts
                .get(index)
                .copied()
                .unwrap_or_default(),
            extra_header_field_count: report
                .eth_block_input_extra_header_field_counts
                .get(index)
                .copied()
                .unwrap_or_default(),
            extra_body_field_count: report
                .eth_block_input_extra_body_field_counts
                .get(index)
                .copied()
                .unwrap_or_default(),
            block_hash: report
                .eth_block_input_block_hashes
                .get(index)
                .copied()
                .unwrap_or([0; 32]),
            parent_hash: report
                .eth_block_input_parent_hashes
                .get(index)
                .copied()
                .unwrap_or([0; 32]),
            ommers_hash: report
                .eth_block_input_ommers_hashes
                .get(index)
                .copied()
                .unwrap_or([0; 32]),
            beneficiary: report
                .eth_block_input_beneficiaries
                .get(index)
                .copied()
                .unwrap_or([0; 20]),
            state_root: report
                .eth_block_input_state_roots
                .get(index)
                .copied()
                .unwrap_or([0; 32]),
            receipts_root: report
                .eth_block_input_receipt_roots
                .get(index)
                .copied()
                .unwrap_or([0; 32]),
            logs_bloom: report
                .eth_block_input_logs_blooms
                .get(index)
                .copied()
                .unwrap_or([0; 256]),
            difficulty: report
                .eth_block_input_difficulties
                .get(index)
                .copied()
                .unwrap_or([0; 32]),
            block_number: report
                .eth_block_input_block_numbers
                .get(index)
                .copied()
                .unwrap_or_default(),
            timestamp: report
                .eth_block_input_timestamps
                .get(index)
                .copied()
                .unwrap_or_default(),
            extra_data: report
                .eth_block_input_extra_data
                .get(index)
                .cloned()
                .unwrap_or_default(),
            gas_limit: report
                .eth_block_input_gas_limits
                .get(index)
                .copied()
                .unwrap_or_default(),
            gas_used: report
                .eth_block_input_gas_used_values
                .get(index)
                .copied()
                .unwrap_or_default(),
            base_fee_per_gas: report
                .eth_block_input_base_fees_per_gas
                .get(index)
                .copied()
                .unwrap_or(None),
            mix_hash: report
                .eth_block_input_mix_hashes
                .get(index)
                .copied()
                .unwrap_or([0; 32]),
            nonce: report
                .eth_block_input_nonces
                .get(index)
                .copied()
                .unwrap_or([0; 8]),
            transactions_root: report
                .eth_block_input_transaction_roots
                .get(index)
                .copied()
                .unwrap_or([0; 32]),
            transaction_preimage_count: report
                .eth_block_input_transaction_preimage_counts
                .get(index)
                .copied()
                .unwrap_or_default(),
            legacy_transaction_count: report
                .eth_block_input_legacy_transaction_counts
                .get(index)
                .copied()
                .unwrap_or_default(),
            typed_transaction_count: report
                .eth_block_input_typed_transaction_counts
                .get(index)
                .copied()
                .unwrap_or_default(),
            receipts_rlp_byte_count: report
                .eth_block_input_receipts_rlp_byte_counts
                .get(index)
                .copied()
                .unwrap_or(None),
            receipt_preimage_count: report
                .eth_block_input_receipt_preimage_counts
                .get(index)
                .copied()
                .unwrap_or(None),
            legacy_receipt_count: report
                .eth_block_input_legacy_receipt_counts
                .get(index)
                .copied()
                .unwrap_or(None),
            typed_receipt_count: report
                .eth_block_input_typed_receipt_counts
                .get(index)
                .copied()
                .unwrap_or(None),
            withdrawal_root: report
                .eth_block_input_withdrawal_roots
                .get(index)
                .copied()
                .unwrap_or(None),
            withdrawal_count: report
                .eth_block_input_withdrawal_counts
                .get(index)
                .copied()
                .unwrap_or(None),
            withdrawal_preimage_count: report
                .eth_block_input_withdrawal_preimage_counts
                .get(index)
                .copied()
                .unwrap_or(None),
        })
        .collect()
}

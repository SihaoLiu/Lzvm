use std::io::Write;

use lzvm_prover::proof_preflight::ProofPreflightReport;

use super::eth_block_input::EthBlockInputBinding;
use crate::prove_plan;

pub(super) fn write_report_eth_block_input_summary(
    stdout: &mut dyn Write,
    report: &ProofPreflightReport,
    index: usize,
) {
    if let Some(block_input_bytes) = report.eth_block_input_byte_counts.get(index) {
        let _ = writeln!(stdout, "eth_block_input_bytes={block_input_bytes}");
    }
    if let Some(block_rlp_bytes) = report.eth_block_input_block_rlp_byte_counts.get(index) {
        let _ = writeln!(stdout, "eth_block_rlp_bytes={block_rlp_bytes}");
    }
    write_eth_extra_field_summary(
        stdout,
        report
            .eth_block_input_extra_header_field_counts
            .get(index)
            .copied()
            .unwrap_or(0),
        report
            .eth_block_input_extra_body_field_counts
            .get(index)
            .copied()
            .unwrap_or(0),
    );
    if let Some(block_hash) = report.eth_block_input_block_hashes.get(index) {
        let _ = writeln!(
            stdout,
            "eth_block_hash={}",
            prove_plan::format_hash(block_hash)
        );
    }
    if let Some(parent_hash) = report.eth_block_input_parent_hashes.get(index) {
        let _ = writeln!(
            stdout,
            "eth_parent_hash={}",
            prove_plan::format_hash(parent_hash)
        );
    }
    if let Some(ommers_hash) = report.eth_block_input_ommers_hashes.get(index) {
        let _ = writeln!(
            stdout,
            "eth_ommers_hash={}",
            prove_plan::format_hash(ommers_hash)
        );
    }
    if let Some(beneficiary) = report.eth_block_input_beneficiaries.get(index) {
        let _ = writeln!(stdout, "eth_beneficiary={}", format_bytes_hex(beneficiary));
    }
    if let Some(state_root) = report.eth_block_input_state_roots.get(index) {
        let _ = writeln!(
            stdout,
            "eth_state_root={}",
            prove_plan::format_hash(state_root)
        );
    }
    if let Some(receipt_root) = report.eth_block_input_receipt_roots.get(index) {
        let _ = writeln!(
            stdout,
            "eth_receipts_root={}",
            prove_plan::format_hash(receipt_root)
        );
    }
    if let Some(logs_bloom) = report.eth_block_input_logs_blooms.get(index) {
        let _ = writeln!(stdout, "eth_logs_bloom={}", format_bytes_hex(logs_bloom));
    }
    if let Some(difficulty) = report.eth_block_input_difficulties.get(index) {
        let _ = writeln!(stdout, "eth_difficulty={}", format_u256(difficulty));
    }
    if let Some(block_number) = report.eth_block_input_block_numbers.get(index) {
        let _ = writeln!(stdout, "eth_block_number={block_number}");
    }
    if let Some(timestamp) = report.eth_block_input_timestamps.get(index) {
        let _ = writeln!(stdout, "eth_block_timestamp={timestamp}");
    }
    if let Some(extra_data) = report.eth_block_input_extra_data.get(index) {
        let _ = writeln!(stdout, "eth_extra_data={}", format_bytes_hex(extra_data));
    }
    if let Some(gas_limit) = report.eth_block_input_gas_limits.get(index) {
        let _ = writeln!(stdout, "eth_gas_limit={gas_limit}");
    }
    if let Some(gas_used) = report.eth_block_input_gas_used_values.get(index) {
        let _ = writeln!(stdout, "eth_gas_used={gas_used}");
    }
    if let Some(base_fee_per_gas) = report.eth_block_input_base_fees_per_gas.get(index) {
        let _ = writeln!(
            stdout,
            "eth_base_fee_per_gas={}",
            format_optional_u256(base_fee_per_gas.as_ref())
        );
    }
    if let Some(mix_hash) = report.eth_block_input_mix_hashes.get(index) {
        let _ = writeln!(stdout, "eth_mix_hash={}", prove_plan::format_hash(mix_hash));
    }
    if let Some(nonce) = report.eth_block_input_nonces.get(index) {
        let _ = writeln!(stdout, "eth_nonce={}", format_bytes_hex(nonce));
    }
    if let Some(transactions_root) = report.eth_block_input_transaction_roots.get(index) {
        let _ = writeln!(
            stdout,
            "eth_transactions_root={}",
            prove_plan::format_hash(transactions_root)
        );
    }
    write_eth_transaction_preimage_summary(
        stdout,
        report
            .eth_block_input_transaction_preimage_counts
            .get(index)
            .copied()
            .unwrap_or(0),
    );
    let legacy_transaction_count = report
        .eth_block_input_legacy_transaction_counts
        .get(index)
        .copied()
        .unwrap_or(0);
    let typed_transaction_count = report
        .eth_block_input_typed_transaction_counts
        .get(index)
        .copied()
        .unwrap_or(0);
    write_eth_transaction_count_summary(stdout, legacy_transaction_count + typed_transaction_count);
    write_eth_transaction_kind_summary(stdout, legacy_transaction_count, typed_transaction_count);
    write_eth_receipt_preimage_summary(
        stdout,
        report
            .eth_block_input_receipt_preimage_counts
            .get(index)
            .copied()
            .unwrap_or(None),
        report
            .eth_block_input_receipts_rlp_byte_counts
            .get(index)
            .copied()
            .unwrap_or(None),
    );
    let legacy_receipt_count = report
        .eth_block_input_legacy_receipt_counts
        .get(index)
        .copied()
        .unwrap_or(None);
    let typed_receipt_count = report
        .eth_block_input_typed_receipt_counts
        .get(index)
        .copied()
        .unwrap_or(None);
    if let (Some(legacy_count), Some(typed_count)) = (legacy_receipt_count, typed_receipt_count) {
        write_eth_receipt_count_summary(stdout, legacy_count + typed_count);
    }
    write_eth_receipt_kind_summary(stdout, legacy_receipt_count, typed_receipt_count);
    write_eth_withdrawal_summary(
        stdout,
        report
            .eth_block_input_withdrawal_roots
            .get(index)
            .copied()
            .unwrap_or(None),
        report
            .eth_block_input_withdrawal_counts
            .get(index)
            .copied()
            .unwrap_or(None),
        report
            .eth_block_input_withdrawal_preimage_counts
            .get(index)
            .copied()
            .unwrap_or(None),
    );
}

pub(super) fn write_eth_block_input_binding_summary(
    stdout: &mut dyn Write,
    proof_input_hashes: &[[u8; 32]],
    binding: EthBlockInputBinding,
) {
    if proof_input_hashes.is_empty() {
        let _ = writeln!(
            stdout,
            "eth_block_input_hash={}",
            prove_plan::format_hash(&binding.hash)
        );
    }
    let _ = writeln!(stdout, "eth_block_input_match=ok");
    let _ = writeln!(stdout, "eth_block_input_bytes={}", binding.bytes);
    let _ = writeln!(stdout, "eth_block_rlp_bytes={}", binding.block_rlp_bytes);
    write_eth_extra_field_summary(
        stdout,
        binding.extra_header_field_count,
        binding.extra_body_field_count,
    );
    let _ = writeln!(
        stdout,
        "eth_block_hash={}",
        prove_plan::format_hash(&binding.block_hash)
    );
    let _ = writeln!(
        stdout,
        "eth_parent_hash={}",
        prove_plan::format_hash(&binding.parent_hash)
    );
    let _ = writeln!(
        stdout,
        "eth_ommers_hash={}",
        prove_plan::format_hash(&binding.ommers_hash)
    );
    let _ = writeln!(
        stdout,
        "eth_beneficiary={}",
        format_bytes_hex(&binding.beneficiary)
    );
    let _ = writeln!(
        stdout,
        "eth_state_root={}",
        prove_plan::format_hash(&binding.state_root)
    );
    let _ = writeln!(
        stdout,
        "eth_receipts_root={}",
        prove_plan::format_hash(&binding.receipts_root)
    );
    let _ = writeln!(
        stdout,
        "eth_logs_bloom={}",
        format_bytes_hex(&binding.logs_bloom)
    );
    let _ = writeln!(
        stdout,
        "eth_difficulty={}",
        format_u256(&binding.difficulty)
    );
    let _ = writeln!(stdout, "eth_block_number={}", binding.block_number);
    let _ = writeln!(stdout, "eth_block_timestamp={}", binding.timestamp);
    let _ = writeln!(
        stdout,
        "eth_extra_data={}",
        format_bytes_hex(&binding.extra_data)
    );
    let _ = writeln!(stdout, "eth_gas_limit={}", binding.gas_limit);
    let _ = writeln!(stdout, "eth_gas_used={}", binding.gas_used);
    let _ = writeln!(
        stdout,
        "eth_base_fee_per_gas={}",
        format_optional_u256(binding.base_fee_per_gas.as_ref())
    );
    let _ = writeln!(
        stdout,
        "eth_mix_hash={}",
        prove_plan::format_hash(&binding.mix_hash)
    );
    let _ = writeln!(stdout, "eth_nonce={}", format_bytes_hex(&binding.nonce));
    let _ = writeln!(
        stdout,
        "eth_transactions_root={}",
        prove_plan::format_hash(&binding.transactions_root)
    );
    write_eth_transaction_preimage_summary(stdout, binding.transaction_preimage_count);
    write_eth_transaction_count_summary(
        stdout,
        binding.legacy_transaction_count + binding.typed_transaction_count,
    );
    write_eth_transaction_kind_summary(
        stdout,
        binding.legacy_transaction_count,
        binding.typed_transaction_count,
    );
    write_eth_receipt_preimage_summary(
        stdout,
        binding.receipt_preimage_count,
        binding.receipts_rlp_bytes,
    );
    if let (Some(legacy_count), Some(typed_count)) =
        (binding.legacy_receipt_count, binding.typed_receipt_count)
    {
        write_eth_receipt_count_summary(stdout, legacy_count + typed_count);
    }
    write_eth_receipt_kind_summary(
        stdout,
        binding.legacy_receipt_count,
        binding.typed_receipt_count,
    );
    write_eth_withdrawal_summary(
        stdout,
        binding.withdrawal_root,
        binding.withdrawal_count,
        binding.withdrawal_preimage_count,
    );
}

pub(super) fn write_eth_transaction_preimage_summary(
    stdout: &mut dyn Write,
    transaction_preimage_count: usize,
) {
    let _ = writeln!(
        stdout,
        "eth_transaction_trie_preimages={transaction_preimage_count}"
    );
}

pub(super) fn write_eth_extra_field_summary(
    stdout: &mut dyn Write,
    extra_header_field_count: usize,
    extra_body_field_count: usize,
) {
    if extra_header_field_count == 0 && extra_body_field_count == 0 {
        return;
    }
    let _ = writeln!(stdout, "eth_extra_header_fields={extra_header_field_count}");
    let _ = writeln!(stdout, "eth_extra_body_fields={extra_body_field_count}");
}

pub(super) fn format_bytes_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub(super) fn format_u256(bytes: &[u8; 32]) -> String {
    match bytes.iter().position(|byte| *byte != 0) {
        Some(index) => format_bytes_hex(&bytes[index..]),
        None => "0".to_owned(),
    }
}

pub(super) fn format_optional_u256(value: Option<&[u8; 32]>) -> String {
    match value {
        Some(bytes) => format_u256(bytes),
        None => "absent".to_owned(),
    }
}

pub(super) fn write_eth_transaction_kind_summary(
    stdout: &mut dyn Write,
    legacy_transaction_count: usize,
    typed_transaction_count: usize,
) {
    let _ = writeln!(stdout, "eth_legacy_transactions={legacy_transaction_count}");
    let _ = writeln!(stdout, "eth_typed_transactions={typed_transaction_count}");
}

pub(super) fn write_eth_transaction_count_summary(
    stdout: &mut dyn Write,
    transaction_count: usize,
) {
    let _ = writeln!(stdout, "eth_transaction_count={transaction_count}");
}

pub(super) fn write_eth_receipt_count_summary(stdout: &mut dyn Write, receipt_count: usize) {
    let _ = writeln!(stdout, "eth_receipt_count={receipt_count}");
}

pub(super) fn write_eth_receipt_preimage_summary(
    stdout: &mut dyn Write,
    receipt_preimage_count: Option<usize>,
    receipts_rlp_bytes: Option<usize>,
) {
    match receipt_preimage_count {
        Some(count) => {
            let _ = writeln!(stdout, "eth_receipts=present");
            if let Some(bytes) = receipts_rlp_bytes {
                let _ = writeln!(stdout, "eth_receipts_rlp_bytes={bytes}");
            }
            let _ = writeln!(stdout, "eth_receipt_trie_preimages={count}");
        }
        None => {
            let _ = writeln!(stdout, "eth_receipts=absent");
        }
    }
}

pub(super) fn write_eth_receipt_kind_summary(
    stdout: &mut dyn Write,
    legacy_receipt_count: Option<usize>,
    typed_receipt_count: Option<usize>,
) {
    if let (Some(legacy_count), Some(typed_count)) = (legacy_receipt_count, typed_receipt_count) {
        let _ = writeln!(stdout, "eth_legacy_receipts={legacy_count}");
        let _ = writeln!(stdout, "eth_typed_receipts={typed_count}");
    }
}

pub(super) fn write_eth_withdrawal_summary(
    stdout: &mut dyn Write,
    withdrawal_root: Option<[u8; 32]>,
    withdrawal_count: Option<usize>,
    withdrawal_preimage_count: Option<usize>,
) {
    match withdrawal_preimage_count {
        Some(count) => {
            let _ = writeln!(stdout, "eth_withdrawals=present");
            if let Some(root) = withdrawal_root {
                let _ = writeln!(
                    stdout,
                    "eth_withdrawals_root={}",
                    prove_plan::format_hash(&root)
                );
            }
            if let Some(withdrawal_count) = withdrawal_count {
                let _ = writeln!(stdout, "eth_withdrawal_count={withdrawal_count}");
            }
            let _ = writeln!(stdout, "eth_withdrawal_trie_preimages={count}");
        }
        None => {
            let _ = writeln!(stdout, "eth_withdrawals=absent");
        }
    }
}

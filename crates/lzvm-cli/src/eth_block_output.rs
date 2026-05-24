use std::io::Write;

use lzvm_prover::contribution::ContributionEthBlockInputReport;

use crate::prove_plan;

pub(crate) fn write_contribution_eth_block_input(
    stdout: &mut dyn Write,
    input: &ContributionEthBlockInputReport,
) {
    let _ = writeln!(
        stdout,
        "eth_block_input_hash={}",
        prove_plan::format_hash(&input.hash)
    );
    let _ = writeln!(stdout, "eth_block_input_bytes={}", input.byte_count);
    let _ = writeln!(stdout, "eth_block_rlp_bytes={}", input.block_rlp_byte_count);
    write_eth_extra_field_summary(
        stdout,
        input.extra_header_field_count,
        input.extra_body_field_count,
    );
    let _ = writeln!(
        stdout,
        "eth_block_hash={}",
        prove_plan::format_hash(&input.block_hash)
    );
    let _ = writeln!(
        stdout,
        "eth_parent_hash={}",
        prove_plan::format_hash(&input.parent_hash)
    );
    let _ = writeln!(
        stdout,
        "eth_ommers_hash={}",
        prove_plan::format_hash(&input.ommers_hash)
    );
    let _ = writeln!(
        stdout,
        "eth_beneficiary={}",
        format_bytes_hex(&input.beneficiary)
    );
    let _ = writeln!(
        stdout,
        "eth_state_root={}",
        prove_plan::format_hash(&input.state_root)
    );
    let _ = writeln!(
        stdout,
        "eth_receipts_root={}",
        prove_plan::format_hash(&input.receipts_root)
    );
    let _ = writeln!(
        stdout,
        "eth_logs_bloom={}",
        format_bytes_hex(&input.logs_bloom)
    );
    let _ = writeln!(stdout, "eth_difficulty={}", format_u256(&input.difficulty));
    let _ = writeln!(stdout, "eth_block_number={}", input.block_number);
    let _ = writeln!(stdout, "eth_block_timestamp={}", input.timestamp);
    let _ = writeln!(
        stdout,
        "eth_extra_data={}",
        format_bytes_hex(&input.extra_data)
    );
    let _ = writeln!(stdout, "eth_gas_limit={}", input.gas_limit);
    let _ = writeln!(stdout, "eth_gas_used={}", input.gas_used);
    let _ = writeln!(
        stdout,
        "eth_base_fee_per_gas={}",
        format_optional_u256(input.base_fee_per_gas.as_ref())
    );
    let _ = writeln!(
        stdout,
        "eth_mix_hash={}",
        prove_plan::format_hash(&input.mix_hash)
    );
    let _ = writeln!(stdout, "eth_nonce={}", format_bytes_hex(&input.nonce));
    let _ = writeln!(
        stdout,
        "eth_transactions_root={}",
        prove_plan::format_hash(&input.transactions_root)
    );
    write_eth_transaction_preimage_summary(stdout, input.transaction_preimage_count);
    write_eth_transaction_count_summary(
        stdout,
        input.legacy_transaction_count + input.typed_transaction_count,
    );
    write_eth_transaction_kind_summary(
        stdout,
        input.legacy_transaction_count,
        input.typed_transaction_count,
    );
    write_eth_receipt_preimage_summary(
        stdout,
        input.receipt_preimage_count,
        input.receipts_rlp_byte_count,
    );
    if let (Some(legacy_count), Some(typed_count)) =
        (input.legacy_receipt_count, input.typed_receipt_count)
    {
        write_eth_receipt_count_summary(stdout, legacy_count + typed_count);
    }
    write_eth_receipt_kind_summary(
        stdout,
        input.legacy_receipt_count,
        input.typed_receipt_count,
    );
    write_eth_withdrawal_summary(
        stdout,
        input.withdrawal_root,
        input.withdrawal_count,
        input.withdrawal_preimage_count,
    );
}

fn write_eth_extra_field_summary(
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

fn write_eth_transaction_preimage_summary(
    stdout: &mut dyn Write,
    transaction_preimage_count: usize,
) {
    let _ = writeln!(
        stdout,
        "eth_transaction_trie_preimages={transaction_preimage_count}"
    );
}

fn write_eth_transaction_kind_summary(
    stdout: &mut dyn Write,
    legacy_transaction_count: usize,
    typed_transaction_count: usize,
) {
    let _ = writeln!(stdout, "eth_legacy_transactions={legacy_transaction_count}");
    let _ = writeln!(stdout, "eth_typed_transactions={typed_transaction_count}");
}

fn write_eth_transaction_count_summary(stdout: &mut dyn Write, transaction_count: usize) {
    let _ = writeln!(stdout, "eth_transaction_count={transaction_count}");
}

fn write_eth_receipt_count_summary(stdout: &mut dyn Write, receipt_count: usize) {
    let _ = writeln!(stdout, "eth_receipt_count={receipt_count}");
}

fn write_eth_receipt_preimage_summary(
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

fn write_eth_receipt_kind_summary(
    stdout: &mut dyn Write,
    legacy_receipt_count: Option<usize>,
    typed_receipt_count: Option<usize>,
) {
    if let (Some(legacy_count), Some(typed_count)) = (legacy_receipt_count, typed_receipt_count) {
        let _ = writeln!(stdout, "eth_legacy_receipts={legacy_count}");
        let _ = writeln!(stdout, "eth_typed_receipts={typed_count}");
    }
}

fn write_eth_withdrawal_summary(
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

fn format_bytes_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn format_u256(bytes: &[u8; 32]) -> String {
    match bytes.iter().position(|byte| *byte != 0) {
        Some(index) => format_bytes_hex(&bytes[index..]),
        None => "0".to_owned(),
    }
}

fn format_optional_u256(value: Option<&[u8; 32]>) -> String {
    match value {
        Some(bytes) => format_u256(bytes),
        None => "absent".to_owned(),
    }
}

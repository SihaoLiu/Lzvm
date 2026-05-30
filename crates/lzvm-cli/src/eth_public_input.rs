use std::io::Write;
use std::path::Path;

use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, encode_eth_block_input, eth_block_input_bytes_digest,
};
use lzvm_artifacts::eth_public_input::{
    eth_public_header_hash, parse_eth_public_block_prefix, parse_eth_public_header_prefix,
    parse_eth_public_transactions_prefix, EthPublicBlockPrefix, EthPublicHeader,
    EthPublicTransactionsPrefix,
};

pub(crate) fn run_summary(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        [input_path] => summarize_public_input(input_path, stdout, stderr),
        _ => write_usage(stderr),
    }
}

pub(crate) fn run_write_block_rlp(
    args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match args {
        [input_path, output_path] => write_block_rlp(input_path, output_path, stdout, stderr),
        _ => write_block_rlp_usage(stderr),
    }
}

pub(crate) fn run_write_block_input(
    args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match args {
        [input_path, output_path] => write_block_input(input_path, output_path, stdout, stderr),
        _ => write_block_input_usage(stderr),
    }
}

fn summarize_public_input(input_path: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let bytes = match std::fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth public input summary failed: read input failed: {input_path}: {error}"
            );
            return 1;
        }
    };
    let parsed = match parse_eth_public_header_prefix(&bytes) {
        Ok(parsed) => parsed,
        Err(error) => {
            let _ = writeln!(stderr, "eth public input summary failed: {error}");
            return 1;
        }
    };
    let transactions = match parse_eth_public_transactions_prefix(&bytes) {
        Ok(transactions) => transactions,
        Err(error) => {
            let _ = writeln!(stderr, "eth public input summary failed: {error}");
            return 1;
        }
    };
    let block = match parse_eth_public_block_prefix(&bytes) {
        Ok(block) => block,
        Err(error) => {
            let _ = writeln!(stderr, "eth public input summary failed: {error}");
            return 1;
        }
    };
    let block_hash = eth_public_header_hash(&parsed.header);

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "public_input={}", Path::new(input_path).display());
    let _ = writeln!(stdout, "bytes={}", bytes.len());
    let _ = writeln!(stdout, "header_bytes={}", parsed.consumed);
    let _ = writeln!(stdout, "transaction_prefix_bytes={}", transactions.consumed);
    let _ = writeln!(stdout, "block_prefix_bytes={}", block.consumed);
    let _ = writeln!(stdout, "remaining_bytes={}", bytes.len() - block.consumed);
    let _ = writeln!(stdout, "block_hash={}", format_hash(&block_hash));
    write_header_summary(stdout, &parsed.header, &transactions, &block);
    0
}

fn write_block_input(
    input_path: &str,
    output_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let bytes = match std::fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth public block input failed: read input failed: {input_path}: {error}"
            );
            return 1;
        }
    };
    let block = match parse_eth_public_block_prefix(&bytes) {
        Ok(block) => block,
        Err(error) => {
            let _ = writeln!(stderr, "eth public block input failed: {error}");
            return 1;
        }
    };
    let transaction_count = block.transactions.len();
    let withdrawal_count = block.withdrawals.as_ref().map(Vec::len);
    let block_rlp = block.block_rlp();
    let input = match build_eth_block_input(&block_rlp) {
        Ok(input) => input,
        Err(error) => {
            let _ = writeln!(stderr, "eth public block input failed: {error}");
            return 1;
        }
    };
    let encoded = match encode_eth_block_input(&input) {
        Ok(encoded) => encoded,
        Err(error) => {
            let _ = writeln!(stderr, "eth public block input failed: {error}");
            return 1;
        }
    };

    let output = Path::new(output_path);
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "eth public block input failed: create output directory failed: {}: {error}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(error) = std::fs::write(output, &encoded) {
        let _ = writeln!(
            stderr,
            "eth public block input failed: write output failed: {}: {error}",
            output.display()
        );
        return 1;
    }

    let digest = eth_block_input_bytes_digest(&encoded);
    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "public_input={}", Path::new(input_path).display());
    let _ = writeln!(stdout, "block_input={}", output.display());
    let _ = writeln!(stdout, "bytes={}", encoded.len());
    let _ = writeln!(stdout, "block_input_hash={}", format_hash(&digest));
    let _ = writeln!(stdout, "block_hash={}", format_hash(&input.block_hash));
    let _ = writeln!(stdout, "transaction_count={transaction_count}");
    let _ = writeln!(
        stdout,
        "withdrawal_count={}",
        format_optional_usize(withdrawal_count)
    );
    0
}

fn write_block_rlp(
    input_path: &str,
    output_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let bytes = match std::fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth public block rlp write failed: read input failed: {input_path}: {error}"
            );
            return 1;
        }
    };
    let block = match parse_eth_public_block_prefix(&bytes) {
        Ok(block) => block,
        Err(error) => {
            let _ = writeln!(stderr, "eth public block rlp write failed: {error}");
            return 1;
        }
    };
    if let Err(message) = validate_public_block_roots(&block) {
        let _ = writeln!(stderr, "eth public block rlp write failed: {message}");
        return 1;
    }
    let block_rlp = block.block_rlp();
    let output = Path::new(output_path);
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "eth public block rlp write failed: create output directory failed: {}: {error}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(error) = std::fs::write(output, &block_rlp) {
        let _ = writeln!(
            stderr,
            "eth public block rlp write failed: write output failed: {}: {error}",
            output.display()
        );
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "public_input={}", Path::new(input_path).display());
    let _ = writeln!(stdout, "bytes={}", block_rlp.len());
    let _ = writeln!(stdout, "output={}", output.display());
    0
}

fn validate_public_block_roots(block: &EthPublicBlockPrefix) -> Result<(), &'static str> {
    if !block.transactions_root_matches() {
        return Err("transactions_root mismatch");
    }
    if !block.ommers_hash_matches() {
        return Err("ommers_hash mismatch");
    }
    if !block.withdrawals_root_matches() {
        return Err("withdrawals_root mismatch");
    }
    Ok(())
}

fn write_header_summary(
    stdout: &mut dyn Write,
    header: &EthPublicHeader,
    transactions: &EthPublicTransactionsPrefix,
    block: &EthPublicBlockPrefix,
) {
    let _ = writeln!(stdout, "block_number={}", header.block_number);
    let _ = writeln!(stdout, "timestamp={}", header.timestamp);
    let _ = writeln!(stdout, "gas_limit={}", header.gas_limit);
    let _ = writeln!(stdout, "gas_used={}", header.gas_used);
    let _ = writeln!(stdout, "ommers_hash={}", format_hash(&header.ommers_hash));
    let computed_ommers_hash = block.ommers_hash();
    let _ = writeln!(
        stdout,
        "computed_ommers_hash={}",
        format_hash(&computed_ommers_hash)
    );
    let _ = writeln!(
        stdout,
        "ommers_hash_matches={}",
        block.ommers_hash_matches()
    );
    let _ = writeln!(stdout, "ommer_count={}", block.ommers.len());
    let _ = writeln!(
        stdout,
        "transactions_root={}",
        format_hash(&header.transactions_root)
    );
    let transaction_root = transactions.transactions_root();
    let _ = writeln!(
        stdout,
        "computed_transactions_root={}",
        format_hash(&transaction_root)
    );
    let _ = writeln!(
        stdout,
        "transactions_root_matches={}",
        transactions.transactions_root_matches()
    );
    let _ = writeln!(
        stdout,
        "transaction_count={}",
        transactions.transactions.len()
    );
    let _ = writeln!(
        stdout,
        "legacy_transactions={}",
        transactions.legacy_transaction_count()
    );
    let _ = writeln!(
        stdout,
        "typed_transactions={}",
        transactions.typed_transaction_count()
    );
    let _ = writeln!(
        stdout,
        "receipts_root={}",
        format_hash(&header.receipts_root)
    );
    let _ = writeln!(
        stdout,
        "withdrawals_root={}",
        format_optional_hash(header.withdrawals_root.as_ref())
    );
    let computed_withdrawals_root = block.withdrawals_root();
    let _ = writeln!(
        stdout,
        "computed_withdrawals_root={}",
        format_optional_plain_hash(computed_withdrawals_root.as_ref())
    );
    let _ = writeln!(
        stdout,
        "withdrawals_root_matches={}",
        block.withdrawals_root_matches()
    );
    let _ = writeln!(
        stdout,
        "withdrawal_count={}",
        format_optional_usize(block.withdrawals.as_ref().map(Vec::len))
    );
    let _ = writeln!(
        stdout,
        "base_fee_per_gas={}",
        format_optional_u64(header.base_fee_per_gas)
    );
    let _ = writeln!(
        stdout,
        "blob_gas_used={}",
        format_optional_u64(header.blob_gas_used)
    );
    let _ = writeln!(
        stdout,
        "excess_blob_gas={}",
        format_optional_u64(header.excess_blob_gas)
    );
    let _ = writeln!(
        stdout,
        "parent_beacon_block_root={}",
        format_optional_hash(header.parent_beacon_block_root.as_ref())
    );
    let _ = writeln!(
        stdout,
        "requests_hash={}",
        format_optional_hash(header.requests_hash.as_ref())
    );
    let _ = writeln!(stdout, "extra_data={}", format_hex(&header.extra_data));
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm eth public-input-summary <input>");
    2
}

fn write_block_rlp_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm eth write-public-block-rlp <input> <out>"
    );
    2
}

fn write_block_input_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm eth write-public-block-input <input> <out>"
    );
    2
}

fn format_optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "absent".to_owned())
}

fn format_optional_hash(value: Option<&[u8; 32]>) -> String {
    value
        .map(|value| format!("present:{}", format_hash(value)))
        .unwrap_or_else(|| "absent".to_owned())
}

fn format_optional_plain_hash(value: Option<&[u8; 32]>) -> String {
    value
        .map(format_hash)
        .unwrap_or_else(|| "absent".to_owned())
}

fn format_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "absent".to_owned())
}

fn format_hash(hash: &[u8; 32]) -> String {
    format_hex(hash)
}

fn format_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

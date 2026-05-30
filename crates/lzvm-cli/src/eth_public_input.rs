use std::io::Write;
use std::path::Path;

use lzvm_artifacts::eth_public_input::{
    eth_public_header_hash, parse_eth_public_header_prefix, EthPublicHeader,
};

pub(crate) fn run_summary(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        [input_path] => summarize_public_input(input_path, stdout, stderr),
        _ => write_usage(stderr),
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
    let block_hash = eth_public_header_hash(&parsed.header);

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "public_input={}", Path::new(input_path).display());
    let _ = writeln!(stdout, "bytes={}", bytes.len());
    let _ = writeln!(stdout, "header_bytes={}", parsed.consumed);
    let _ = writeln!(stdout, "remaining_bytes={}", bytes.len() - parsed.consumed);
    let _ = writeln!(stdout, "block_hash={}", format_hash(&block_hash));
    write_header_summary(stdout, &parsed.header);
    0
}

fn write_header_summary(stdout: &mut dyn Write, header: &EthPublicHeader) {
    let _ = writeln!(stdout, "block_number={}", header.block_number);
    let _ = writeln!(stdout, "timestamp={}", header.timestamp);
    let _ = writeln!(stdout, "gas_limit={}", header.gas_limit);
    let _ = writeln!(stdout, "gas_used={}", header.gas_used);
    let _ = writeln!(
        stdout,
        "transactions_root={}",
        format_hash(&header.transactions_root)
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

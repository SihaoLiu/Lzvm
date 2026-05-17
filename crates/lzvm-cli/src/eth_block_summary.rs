use std::fmt;
use std::io::Write;

use lzvm_artifacts::eth_block::{
    decode_eth_header_rlp, decode_eth_transactions_rlp, eth_header_hash, eth_ommers_hash,
    parse_eth_block_rlp, EthTransactionRlp,
};
use lzvm_artifacts::eth_trie::transaction_trie_root;

pub(crate) fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        [path] => summarize_block(path, false, stdout, stderr),
        ["--hex", path] => summarize_block(path, true, stdout, stderr),
        _ => write_usage(stderr),
    }
}

fn summarize_block(
    path: &str,
    hex_input: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let raw_bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth block summary failed: read input failed: {path}: {error}"
            );
            return 1;
        }
    };

    let bytes = if hex_input {
        match decode_hex_bytes(&raw_bytes) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(stderr, "eth block summary failed: {error}");
                return 1;
            }
        }
    } else {
        raw_bytes
    };

    let block = match parse_eth_block_rlp(&bytes) {
        Ok(block) => block,
        Err(error) => {
            let _ = writeln!(stderr, "eth block summary failed: {error}");
            return 1;
        }
    };
    let header = match decode_eth_header_rlp(&block.header) {
        Ok(header) => header,
        Err(error) => {
            let _ = writeln!(stderr, "eth block summary failed: {error}");
            return 1;
        }
    };
    let transactions = match decode_eth_transactions_rlp(&block.transactions) {
        Ok(transactions) => transactions,
        Err(error) => {
            let _ = writeln!(stderr, "eth block summary failed: {error}");
            return 1;
        }
    };
    let legacy_transactions = transactions
        .iter()
        .filter(|transaction| matches!(transaction, EthTransactionRlp::Legacy(_)))
        .count();
    let typed_transactions = transactions.len() - legacy_transactions;
    let computed_transactions_root = match transaction_trie_root(&block.transactions) {
        Ok(root) => root,
        Err(error) => {
            let _ = writeln!(stderr, "eth block summary failed: {error}");
            return 1;
        }
    };
    let computed_ommers_hash = eth_ommers_hash(&block.ommers);

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes={}", bytes.len());
    let _ = writeln!(stdout, "header_fields={}", block.header.len());
    let _ = writeln!(
        stdout,
        "block_hash={}",
        format_hash(&eth_header_hash(&block.header))
    );
    let _ = writeln!(stdout, "block_number={}", header.number);
    let _ = writeln!(stdout, "timestamp={}", header.timestamp);
    let _ = writeln!(stdout, "transactions={}", block.transactions.len());
    let _ = writeln!(
        stdout,
        "transactions_root={}",
        format_hash(&header.transactions_root)
    );
    let _ = writeln!(
        stdout,
        "computed_transactions_root={}",
        format_hash(&computed_transactions_root)
    );
    let _ = writeln!(
        stdout,
        "transactions_root_matches={}",
        header.transactions_root == computed_transactions_root
    );
    let _ = writeln!(stdout, "legacy_transactions={legacy_transactions}");
    let _ = writeln!(stdout, "typed_transactions={typed_transactions}");
    let _ = writeln!(stdout, "ommers={}", block.ommers.len());
    let _ = writeln!(stdout, "ommers_hash={}", format_hash(&header.ommers_hash));
    let _ = writeln!(
        stdout,
        "computed_ommers_hash={}",
        format_hash(&computed_ommers_hash)
    );
    let _ = writeln!(
        stdout,
        "ommers_hash_matches={}",
        header.ommers_hash == computed_ommers_hash
    );
    let _ = writeln!(
        stdout,
        "withdrawals={}",
        if block.withdrawals.is_some() {
            "present"
        } else {
            "absent"
        }
    );
    let _ = writeln!(
        stdout,
        "extra_body_fields={}",
        block.extra_body_fields.len()
    );
    let _ = writeln!(
        stdout,
        "extra_header_fields={}",
        header.extra_header_fields.len()
    );
    0
}

fn decode_hex_bytes(input: &[u8]) -> Result<Vec<u8>, HexDecodeError> {
    let mut start = input
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    if matches!(
        input.get(start..start + 2),
        Some(prefix) if prefix.eq_ignore_ascii_case(b"0x")
    ) {
        start += 2;
    }

    let mut digits = Vec::new();
    for (offset, byte) in input.iter().copied().enumerate().skip(start) {
        if byte.is_ascii_whitespace() {
            continue;
        }
        let Some(value) = hex_value(byte) else {
            return Err(HexDecodeError::InvalidDigit { offset, byte });
        };
        digits.push(value);
    }

    if !digits.len().is_multiple_of(2) {
        return Err(HexDecodeError::OddDigitCount);
    }

    Ok(digits
        .chunks_exact(2)
        .map(|pair| (pair[0] << 4) | pair[1])
        .collect())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn format_hash(hash: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in hash {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }
    output
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + value - 10),
        _ => unreachable!("hex nybble should be in range"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HexDecodeError {
    InvalidDigit { offset: usize, byte: u8 },
    OddDigitCount,
}

impl fmt::Display for HexDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDigit { offset, byte } => {
                write!(f, "invalid hex digit at offset {offset}: ")?;
                if byte.is_ascii_graphic() {
                    write!(f, "{}", char::from(*byte))
                } else {
                    write!(f, "0x{byte:02x}")
                }
            }
            Self::OddDigitCount => write!(f, "odd hex digit count"),
        }
    }
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm eth block-summary [--hex] <block-rlp>");
    2
}

use std::fmt;
use std::io::Write;
use std::path::Path;

use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, encode_eth_block_input, eth_block_input_bytes_digest,
    parse_eth_block_input, EthBlockInput,
};

pub(crate) fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        [block_path, output_path] => {
            write_block_input(block_path, output_path, false, stdout, stderr)
        }
        ["--hex", block_path, output_path] => {
            write_block_input(block_path, output_path, true, stdout, stderr)
        }
        _ => write_usage(stderr),
    }
}

pub(crate) fn run_summary(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        [input_path] => summarize_block_input(input_path, stdout, stderr),
        _ => write_summary_usage(stderr),
    }
}

fn write_block_input(
    block_path: &str,
    output_path: &str,
    hex_input: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let raw_bytes = match std::fs::read(block_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth block input failed: read input failed: {block_path}: {error}"
            );
            return 1;
        }
    };

    let block_rlp = if hex_input {
        match decode_hex_bytes(&raw_bytes) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(stderr, "eth block input failed: {error}");
                return 1;
            }
        }
    } else {
        raw_bytes
    };

    let input = match build_eth_block_input(&block_rlp) {
        Ok(input) => input,
        Err(error) => {
            let _ = writeln!(stderr, "eth block input failed: {error}");
            return 1;
        }
    };
    let encoded = match encode_eth_block_input(&input) {
        Ok(encoded) => encoded,
        Err(error) => {
            let _ = writeln!(stderr, "eth block input failed: {error}");
            return 1;
        }
    };

    let output_path = Path::new(output_path);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "eth block input failed: create output directory failed: {}: {error}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(error) = std::fs::write(output_path, &encoded) {
        let _ = writeln!(
            stderr,
            "eth block input failed: write output failed: {}: {error}",
            output_path.display()
        );
        return 1;
    }

    let digest = eth_block_input_bytes_digest(&encoded);
    write_input_summary(stdout, output_path, encoded.len(), &digest, &input, false);
    0
}

fn summarize_block_input(input_path: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let encoded = match std::fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth block input summary failed: read input failed: {input_path}: {error}"
            );
            return 1;
        }
    };
    let input = match parse_eth_block_input(&encoded) {
        Ok(input) => input,
        Err(error) => {
            let _ = writeln!(stderr, "eth block input summary failed: {error}");
            return 1;
        }
    };

    let digest = eth_block_input_bytes_digest(&encoded);
    write_input_summary(
        stdout,
        Path::new(input_path),
        encoded.len(),
        &digest,
        &input,
        true,
    );
    0
}

fn write_input_summary(
    stdout: &mut dyn Write,
    input_path: &Path,
    encoded_len: usize,
    digest: &[u8; 32],
    input: &EthBlockInput,
    include_block_rlp_bytes: bool,
) {
    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "block_input={}", input_path.display());
    let _ = writeln!(stdout, "bytes={encoded_len}");
    let _ = writeln!(stdout, "block_input_hash={}", format_hash(digest));
    if include_block_rlp_bytes {
        let _ = writeln!(stdout, "block_rlp_bytes={}", input.block_rlp.len());
    }
    let _ = writeln!(stdout, "block_hash={}", format_hash(&input.block_hash));
    let _ = writeln!(stdout, "state_root={}", format_hash(&input.state_root));
    let _ = writeln!(
        stdout,
        "receipts_root={}",
        format_hash(&input.receipts_root)
    );
    let _ = writeln!(stdout, "block_number={}", input.block_number);
    let _ = writeln!(stdout, "timestamp={}", input.timestamp);
    let _ = writeln!(stdout, "gas_limit={}", input.gas_limit);
    let _ = writeln!(stdout, "gas_used={}", input.gas_used);
    let _ = writeln!(
        stdout,
        "transactions_root={}",
        format_hash(&input.transactions_root)
    );
    let _ = writeln!(
        stdout,
        "transaction_trie_preimages={}",
        input.transactions.hash_preimages.len()
    );
    let _ = writeln!(
        stdout,
        "withdrawals={}",
        if input.withdrawals.is_some() {
            "present"
        } else {
            "absent"
        }
    );
    if let Some(withdrawals) = &input.withdrawals {
        let withdrawals_root = input
            .withdrawals_root
            .expect("withdrawals build requires withdrawals root");
        let _ = writeln!(
            stdout,
            "withdrawals_root={}",
            format_hash(&withdrawals_root)
        );
        let _ = writeln!(
            stdout,
            "withdrawals_trie_preimages={}",
            withdrawals.hash_preimages.len()
        );
    }
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
    let _ = writeln!(
        stderr,
        "usage: lzvm eth write-block-input [--hex] <block-rlp> <out-input>"
    );
    2
}

fn write_summary_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm eth block-input-summary <block-input>");
    2
}

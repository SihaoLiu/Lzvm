use std::fmt;
use std::io::Write;

use lzvm_artifacts::eth_block::{decode_eth_header_rlp, parse_eth_block_rlp};

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

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes={}", bytes.len());
    let _ = writeln!(stdout, "header_fields={}", block.header.len());
    let _ = writeln!(stdout, "block_number={}", header.number);
    let _ = writeln!(stdout, "timestamp={}", header.timestamp);
    let _ = writeln!(stdout, "transactions={}", block.transactions.len());
    let _ = writeln!(stdout, "ommers={}", block.ommers.len());
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

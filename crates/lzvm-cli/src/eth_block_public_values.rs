use std::fmt;
use std::io::Write;
use std::path::Path;

use lzvm_artifacts::eth_block_input::parse_eth_block_input;
use lzvm_artifacts::eth_block_public_values::public_values_from_eth_block_input;
use lzvm_artifacts::key_directory::key_directory_catalog_digest;
use lzvm_artifacts::public_values::{encode_public_values, public_values_digest, PublicValues};

use crate::prove_plan::read_checked_setup_catalog;

pub(crate) fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        ["--setup-hash", setup_hash, input_path, output_path] => {
            let setup_hash = match parse_hash_hex(setup_hash) {
                Ok(hash) => hash,
                Err(error) => {
                    let _ = writeln!(stderr, "eth block public values failed: {error}");
                    return 1;
                }
            };
            write_block_public_values(setup_hash, input_path, output_path, stdout, stderr)
        }
        ["--setup-dir", setup_dir, input_path, output_path] => {
            let setup_hash = match setup_hash_from_directory(setup_dir) {
                Ok(hash) => hash,
                Err(message) => {
                    let _ = writeln!(stderr, "eth block public values failed: {message}");
                    return 1;
                }
            };
            write_block_public_values(setup_hash, input_path, output_path, stdout, stderr)
        }
        _ => write_usage(stderr),
    }
}

fn write_block_public_values(
    setup_hash: [u8; 32],
    input_path: &str,
    output_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input_bytes = match std::fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth block public values failed: read input failed: {input_path}: {error}"
            );
            return 1;
        }
    };
    let input = match parse_eth_block_input(&input_bytes) {
        Ok(input) => input,
        Err(error) => {
            let _ = writeln!(stderr, "eth block public values failed: {error}");
            return 1;
        }
    };
    let public_values = public_values_from_eth_block_input(setup_hash, &input);
    let public_values_hash = match public_values_digest(&public_values) {
        Ok(hash) => hash,
        Err(error) => {
            let _ = writeln!(stderr, "eth block public values failed: {error}");
            return 1;
        }
    };
    let encoded = match encode_public_values(&public_values) {
        Ok(encoded) => encoded,
        Err(error) => {
            let _ = writeln!(stderr, "eth block public values failed: {error}");
            return 1;
        }
    };

    let output_path = Path::new(output_path);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "eth block public values failed: create output directory failed: {}: {error}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(error) = std::fs::write(output_path, &encoded) {
        let _ = writeln!(
            stderr,
            "eth block public values failed: write output failed: {}: {error}",
            output_path.display()
        );
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "public_values={}", output_path.display());
    let _ = writeln!(stdout, "bytes={}", encoded.len());
    let _ = writeln!(stdout, "setup_hash={}", format_hash(&setup_hash));
    let _ = writeln!(
        stdout,
        "public_values_hash={}",
        format_hash(&public_values_hash)
    );
    let _ = writeln!(stdout, "values={}", public_values.values.len());
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        public_values_field_count(&public_values)
    );
    let _ = writeln!(stdout, "block_hash={}", format_hash(&input.block_hash));
    let _ = writeln!(stdout, "parent_hash={}", format_hash(&input.parent_hash));
    let _ = writeln!(stdout, "beneficiary={}", format_hex(&input.beneficiary));
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
    let _ = writeln!(stdout, "mix_hash={}", format_hash(&input.mix_hash));
    let _ = writeln!(stdout, "nonce={}", format_hex(&input.nonce));
    let _ = writeln!(
        stdout,
        "transactions_root={}",
        format_hash(&input.transactions_root)
    );
    let _ = writeln!(
        stdout,
        "withdrawals={}",
        if input.withdrawals_root.is_some() {
            "present"
        } else {
            "absent"
        }
    );
    0
}

fn public_values_field_count(public_values: &PublicValues) -> usize {
    public_values
        .values
        .iter()
        .map(|entry| entry.elements.len())
        .sum()
}

fn setup_hash_from_directory(setup_dir: &str) -> Result<[u8; 32], String> {
    let catalog = read_checked_setup_catalog(Path::new(setup_dir))
        .map_err(|message| format!("read setup directory failed: {setup_dir}: {message}"))?;
    key_directory_catalog_digest(&catalog)
        .map_err(|error| format!("derive setup hash failed: {setup_dir}: {error}"))
}

fn parse_hash_hex(value: &str) -> Result<[u8; 32], HashHexError> {
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if value.len() != 64 {
        return Err(HashHexError::InvalidLength { found: value.len() });
    }

    let mut out = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_value(chunk[0]).ok_or(HashHexError::InvalidDigit {
            offset: index * 2,
            byte: chunk[0],
        })?;
        let low = hex_value(chunk[1]).ok_or(HashHexError::InvalidDigit {
            offset: index * 2 + 1,
            byte: chunk[1],
        })?;
        out[index] = (high << 4) | low;
    }
    Ok(out)
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
    format_hex(hash)
}

fn format_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
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
enum HashHexError {
    InvalidLength { found: usize },
    InvalidDigit { offset: usize, byte: u8 },
}

impl fmt::Display for HashHexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { found } => {
                write!(
                    f,
                    "invalid setup hash length: expected 64 hex digits, found {found}"
                )
            }
            Self::InvalidDigit { offset, byte } => {
                write!(f, "invalid setup hash digit at offset {offset}: ")?;
                if byte.is_ascii_graphic() {
                    write!(f, "{}", char::from(*byte))
                } else {
                    write!(f, "0x{byte:02x}")
                }
            }
        }
    }
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm eth write-block-public-values (--setup-hash <hex32> | --setup-dir <setup-dir>) <block-input> <out-public-values>"
    );
    2
}

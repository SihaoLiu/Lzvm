use crate::setup_info::UnitSetupInfo;
use crate::verification_key::VerificationKeyRoot;
use lzvm_field::{Felt, FieldError};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const WORD_BYTES: u64 = 8;
const HASH_WORDS: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantTreeHashKind {
    Gl,
    Bn128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantTree {
    pub hash_kind: ConstantTreeHashKind,
    pub extended_row_count: u64,
    pub constant_count: u64,
    pub leaf_byte_count: usize,
    pub node_byte_count: usize,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantTreeFileSummary {
    pub root: VerificationKeyRoot,
    pub digest: [u8; 32],
    pub byte_count: u64,
    pub leaf_byte_count: usize,
    pub node_byte_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantTreeError {
    InvalidArity {
        arity: u32,
    },
    DomainTooLarge {
        n_bits_ext: u32,
    },
    LengthOverflow,
    InvalidByteLength {
        expected: usize,
        found: usize,
    },
    RootNonCanonical {
        word_index: usize,
        source: FieldError,
    },
    RootMismatch {
        expected: VerificationKeyRoot,
        found: VerificationKeyRoot,
    },
    DigestMismatch {
        expected: [u8; 32],
        found: [u8; 32],
    },
    Io {
        message: String,
    },
}

impl fmt::Display for ConstantTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArity { arity } => write!(f, "invalid constant-tree arity: {arity}"),
            Self::DomainTooLarge { n_bits_ext } => {
                write!(
                    f,
                    "constant-tree extended domain is too large: {n_bits_ext}"
                )
            }
            Self::LengthOverflow => write!(f, "constant-tree length overflow"),
            Self::InvalidByteLength { expected, found } => write!(
                f,
                "invalid constant-tree byte length: expected {expected}, found {found}"
            ),
            Self::RootNonCanonical { word_index, source } => write!(
                f,
                "constant-tree root word {word_index} is non-canonical: {source}"
            ),
            Self::RootMismatch { expected, found } => {
                write!(
                    f,
                    "constant-tree root mismatch: expected {expected:?}, found {found:?}"
                )
            }
            Self::DigestMismatch { expected, found } => {
                write!(
                    f,
                    "constant-tree digest mismatch: expected {expected:?}, found {found:?}"
                )
            }
            Self::Io { message } => write!(f, "constant-tree io error: {message}"),
        }
    }
}

impl std::error::Error for ConstantTreeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RootNonCanonical { source, .. } => Some(source),
            Self::InvalidArity { .. }
            | Self::DomainTooLarge { .. }
            | Self::LengthOverflow
            | Self::InvalidByteLength { .. }
            | Self::RootMismatch { .. }
            | Self::DigestMismatch { .. }
            | Self::Io { .. } => None,
        }
    }
}

impl ConstantTree {
    pub fn root(&self) -> Result<VerificationKeyRoot, ConstantTreeError> {
        let root_bytes = checked_usize(checked_mul(HASH_WORDS, WORD_BYTES)?)?;
        if self.node_byte_count < root_bytes || self.bytes.len() < root_bytes {
            return Err(ConstantTreeError::LengthOverflow);
        }

        let root_start = self.bytes.len() - root_bytes;
        parse_root_bytes(&self.bytes[root_start..])
    }
}

pub fn read_constant_tree_file(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
) -> Result<ConstantTree, ConstantTreeError> {
    read_constant_tree_file_inner(path, setup, None)
}

pub fn read_constant_tree_file_with_digest(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    expected_digest: [u8; 32],
) -> Result<ConstantTree, ConstantTreeError> {
    read_constant_tree_file_inner(path, setup, Some(expected_digest))
}

fn read_constant_tree_file_inner(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    expected_digest: Option<[u8; 32]>,
) -> Result<ConstantTree, ConstantTreeError> {
    let bytes = std::fs::read(path).map_err(|error| ConstantTreeError::Io {
        message: error.to_string(),
    })?;
    if let Some(expected) = expected_digest {
        let found: [u8; 32] = Sha256::digest(&bytes).into();
        if found != expected {
            return Err(ConstantTreeError::DigestMismatch { expected, found });
        }
    }
    parse_constant_tree_bytes(bytes, setup)
}

pub fn summarize_constant_tree_file(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeFileSummary, ConstantTreeError> {
    let path = path.as_ref();
    let expected = expected_constant_tree_byte_count(setup)?;
    let metadata = std::fs::metadata(path).map_err(|error| ConstantTreeError::Io {
        message: error.to_string(),
    })?;
    if !metadata.is_file() {
        return Err(ConstantTreeError::Io {
            message: "path is not a file".to_owned(),
        });
    }
    if metadata.len() != u64::try_from(expected).map_err(|_| ConstantTreeError::LengthOverflow)? {
        return Err(ConstantTreeError::InvalidByteLength {
            expected,
            found: usize::try_from(metadata.len())
                .map_err(|_| ConstantTreeError::LengthOverflow)?,
        });
    }

    let mut file = File::open(path).map_err(|error| ConstantTreeError::Io {
        message: error.to_string(),
    })?;
    let digest = sha256_reader(&mut file)?;
    let root = read_constant_tree_root_from_file(&mut file)?;
    let (leaf_byte_count, node_byte_count) = expected_constant_tree_leaf_node_byte_counts(setup)?;

    Ok(ConstantTreeFileSummary {
        root,
        digest,
        byte_count: metadata.len(),
        leaf_byte_count,
        node_byte_count,
    })
}

pub fn parse_constant_tree_bytes(
    bytes: Vec<u8>,
    setup: &UnitSetupInfo,
) -> Result<ConstantTree, ConstantTreeError> {
    let expected = expected_constant_tree_byte_count(setup)?;
    if bytes.len() != expected {
        return Err(ConstantTreeError::InvalidByteLength {
            expected,
            found: bytes.len(),
        });
    }

    let extended_row_count = extended_row_count(setup)?;
    let constant_count = u64::from(setup.n_constants);
    let (leaf_byte_count, node_byte_count) = expected_constant_tree_leaf_node_byte_counts(setup)?;

    Ok(ConstantTree {
        hash_kind: hash_kind(setup),
        extended_row_count,
        constant_count,
        leaf_byte_count,
        node_byte_count,
        bytes,
    })
}

fn sha256_reader(reader: &mut File) -> Result<[u8; 32], ConstantTreeError> {
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|error| ConstantTreeError::Io {
            message: error.to_string(),
        })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| ConstantTreeError::Io {
                message: error.to_string(),
            })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().into())
}

fn read_constant_tree_root_from_file(
    file: &mut File,
) -> Result<VerificationKeyRoot, ConstantTreeError> {
    let root_bytes = checked_usize(checked_mul(HASH_WORDS, WORD_BYTES)?)?;
    file.seek(SeekFrom::End(
        -i64::try_from(root_bytes).map_err(|_| ConstantTreeError::LengthOverflow)?,
    ))
    .map_err(|error| ConstantTreeError::Io {
        message: error.to_string(),
    })?;
    let mut bytes = vec![0_u8; root_bytes];
    file.read_exact(&mut bytes)
        .map_err(|error| ConstantTreeError::Io {
            message: error.to_string(),
        })?;
    parse_root_bytes(&bytes)
}

fn parse_root_bytes(bytes: &[u8]) -> Result<VerificationKeyRoot, ConstantTreeError> {
    let mut values = Vec::with_capacity(HASH_WORDS as usize);
    for (word_index, chunk) in bytes.chunks_exact(WORD_BYTES as usize).enumerate() {
        let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
        Felt::from_canonical(value)
            .map_err(|source| ConstantTreeError::RootNonCanonical { word_index, source })?;
        values.push(value);
    }
    Ok(VerificationKeyRoot::FieldElements(values))
}

pub fn expected_constant_tree_byte_count(
    setup: &UnitSetupInfo,
) -> Result<usize, ConstantTreeError> {
    checked_usize(checked_mul(
        expected_constant_tree_word_count(setup)?,
        WORD_BYTES,
    )?)
}

pub fn expected_constant_tree_leaf_node_byte_counts(
    setup: &UnitSetupInfo,
) -> Result<(usize, usize), ConstantTreeError> {
    let extended_row_count = extended_row_count(setup)?;
    let constant_count = u64::from(setup.n_constants);
    let leaf_byte_count = checked_usize(checked_mul(
        checked_mul(extended_row_count, constant_count)?,
        WORD_BYTES,
    )?)?;
    let total_byte_count = expected_constant_tree_byte_count(setup)?;
    let node_byte_count = total_byte_count
        .checked_sub(leaf_byte_count)
        .ok_or(ConstantTreeError::LengthOverflow)?;
    Ok((leaf_byte_count, node_byte_count))
}

pub fn expected_constant_tree_word_count(setup: &UnitSetupInfo) -> Result<u64, ConstantTreeError> {
    let height = extended_row_count(setup)?;
    let constant_count = u64::from(setup.n_constants);
    let leaf_words = checked_mul(height, constant_count)?;
    let node_words = match hash_kind(setup) {
        ConstantTreeHashKind::Gl => gl_node_word_count(height, setup.stark.merkle_tree_arity)?,
        ConstantTreeHashKind::Bn128 => {
            bn128_node_word_count(height, setup.stark.merkle_tree_arity)?
        }
    };
    checked_add(leaf_words, node_words)
}

fn extended_row_count(setup: &UnitSetupInfo) -> Result<u64, ConstantTreeError> {
    1_u64
        .checked_shl(setup.stark.n_bits_ext)
        .ok_or(ConstantTreeError::DomainTooLarge {
            n_bits_ext: setup.stark.n_bits_ext,
        })
}

fn hash_kind(setup: &UnitSetupInfo) -> ConstantTreeHashKind {
    if setup.stark.verification_hash_type.as_deref() == Some("BN128") {
        ConstantTreeHashKind::Bn128
    } else {
        ConstantTreeHashKind::Gl
    }
}

fn gl_node_word_count(height: u64, arity: u32) -> Result<u64, ConstantTreeError> {
    let arity = validate_arity(arity)?;
    let mut total_nodes = height;
    let mut level_nodes = height;

    while level_nodes > 1 {
        let extra_zeros = (arity - (level_nodes % arity)) % arity;
        total_nodes = checked_add(total_nodes, extra_zeros)?;
        let next = ceil_div(level_nodes, arity);
        total_nodes = checked_add(total_nodes, next)?;
        level_nodes = next;
    }

    checked_mul(total_nodes, HASH_WORDS)
}

fn bn128_node_word_count(height: u64, arity: u32) -> Result<u64, ConstantTreeError> {
    let arity = validate_arity(arity)?;
    let mut n_tmp = height;
    let mut next = ceil_div(n_tmp, arity);
    let mut acc = checked_mul(next, arity)?;

    while n_tmp > 1 {
        n_tmp = next;
        next = ceil_div(n_tmp, arity);
        if n_tmp > 1 {
            acc = checked_add(acc, checked_mul(next, arity)?)?;
        } else {
            acc = checked_add(acc, 1)?;
        }
    }

    checked_mul(acc, HASH_WORDS)
}

fn validate_arity(arity: u32) -> Result<u64, ConstantTreeError> {
    if arity < 2 {
        return Err(ConstantTreeError::InvalidArity { arity });
    }
    Ok(u64::from(arity))
}

fn ceil_div(value: u64, divisor: u64) -> u64 {
    value.div_ceil(divisor)
}

fn checked_add(a: u64, b: u64) -> Result<u64, ConstantTreeError> {
    a.checked_add(b).ok_or(ConstantTreeError::LengthOverflow)
}

fn checked_mul(a: u64, b: u64) -> Result<u64, ConstantTreeError> {
    a.checked_mul(b).ok_or(ConstantTreeError::LengthOverflow)
}

fn checked_usize(value: u64) -> Result<usize, ConstantTreeError> {
    usize::try_from(value).map_err(|_| ConstantTreeError::LengthOverflow)
}

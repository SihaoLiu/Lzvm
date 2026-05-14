use crate::setup_info::UnitSetupInfo;
use std::fmt;
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
pub enum ConstantTreeError {
    InvalidArity { arity: u32 },
    DomainTooLarge { n_bits_ext: u32 },
    LengthOverflow,
    InvalidByteLength { expected: usize, found: usize },
    Io { message: String },
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
            Self::Io { message } => write!(f, "constant-tree io error: {message}"),
        }
    }
}

impl std::error::Error for ConstantTreeError {}

pub fn read_constant_tree_file(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
) -> Result<ConstantTree, ConstantTreeError> {
    let bytes = std::fs::read(path).map_err(|error| ConstantTreeError::Io {
        message: error.to_string(),
    })?;
    parse_constant_tree_bytes(bytes, setup)
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
    let leaf_byte_count = checked_usize(checked_mul(
        checked_mul(extended_row_count, constant_count)?,
        WORD_BYTES,
    )?)?;
    let node_byte_count = expected
        .checked_sub(leaf_byte_count)
        .ok_or(ConstantTreeError::LengthOverflow)?;

    Ok(ConstantTree {
        hash_kind: hash_kind(setup),
        extended_row_count,
        constant_count,
        leaf_byte_count,
        node_byte_count,
        bytes,
    })
}

pub fn expected_constant_tree_byte_count(
    setup: &UnitSetupInfo,
) -> Result<usize, ConstantTreeError> {
    checked_usize(checked_mul(
        expected_constant_tree_word_count(setup)?,
        WORD_BYTES,
    )?)
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

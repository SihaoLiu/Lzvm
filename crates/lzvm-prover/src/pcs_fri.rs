use std::fmt;

use lzvm_field::{intt_in_place, DomainError, Ext3, Felt, FieldError, SHIFT};

use crate::merkle_hash::{
    linear_hash, parent_hash, root_from_digest_level, MerkleHashError, HASH_WORDS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriFoldError {
    InvalidLayerBits { current_bits: u32, prev_bits: u32 },
    InvalidExtensionBits { n_bits_ext: u32, prev_bits: u32 },
    ValueLengthMismatch { expected: usize, found: usize },
    UnsupportedRoot { bits: u32 },
    ZeroEvaluationPoint,
    Domain(DomainError),
    Field(FieldError),
    LengthOverflow,
}

impl fmt::Display for PcsFriFoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLayerBits {
                current_bits,
                prev_bits,
            } => write!(
                f,
                "PCS FRI fold layer bits are invalid: current {current_bits}, previous {prev_bits}"
            ),
            Self::InvalidExtensionBits {
                n_bits_ext,
                prev_bits,
            } => write!(
                f,
                "PCS FRI fold extension bits are invalid: extended {n_bits_ext}, previous {prev_bits}"
            ),
            Self::ValueLengthMismatch { expected, found } => write!(
                f,
                "PCS FRI fold value length mismatch: expected {expected}, found {found}"
            ),
            Self::UnsupportedRoot { bits } => {
                write!(f, "PCS FRI fold root is unsupported for bits {bits}")
            }
            Self::ZeroEvaluationPoint => write!(f, "PCS FRI fold evaluation point is zero"),
            Self::Domain(error) => write!(f, "PCS FRI fold domain error: {error}"),
            Self::Field(error) => write!(f, "PCS FRI fold field error: {error}"),
            Self::LengthOverflow => write!(f, "PCS FRI fold length overflow"),
        }
    }
}

impl std::error::Error for PcsFriFoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::Field(error) => Some(error),
            Self::InvalidLayerBits { .. }
            | Self::InvalidExtensionBits { .. }
            | Self::ValueLengthMismatch { .. }
            | Self::UnsupportedRoot { .. }
            | Self::ZeroEvaluationPoint
            | Self::LengthOverflow => None,
        }
    }
}

impl From<DomainError> for PcsFriFoldError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<FieldError> for PcsFriFoldError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriMerkleError {
    UnsupportedArity { arity: usize },
    EmptyValues,
    EmptyLastLevel,
    InvalidSiblingCount { expected: usize, found: usize },
    LastLevelIndexOutOfRange { index: u64, node_count: usize },
    LengthOverflow,
}

impl fmt::Display for PcsFriMerkleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArity { arity } => {
                write!(f, "PCS FRI Merkle arity is unsupported: {arity}")
            }
            Self::EmptyValues => write!(f, "PCS FRI Merkle query has no values"),
            Self::EmptyLastLevel => write!(f, "PCS FRI Merkle last level is empty"),
            Self::InvalidSiblingCount { expected, found } => write!(
                f,
                "PCS FRI Merkle sibling count mismatch: expected {expected}, found {found}"
            ),
            Self::LastLevelIndexOutOfRange { index, node_count } => write!(
                f,
                "PCS FRI Merkle last-level index {index} is outside node count {node_count}"
            ),
            Self::LengthOverflow => write!(f, "PCS FRI Merkle length overflow"),
        }
    }
}

impl std::error::Error for PcsFriMerkleError {}

impl From<MerkleHashError> for PcsFriMerkleError {
    fn from(error: MerkleHashError) -> Self {
        match error {
            MerkleHashError::UnsupportedArity { arity } => Self::UnsupportedArity { arity },
            MerkleHashError::InvalidChildCount { expected, found } => {
                Self::InvalidSiblingCount { expected, found }
            }
            MerkleHashError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

pub fn verify_fri_fold(
    n_bits_ext: u32,
    current_bits: u32,
    prev_bits: u32,
    challenge: Ext3,
    index: u64,
    values: &[Ext3],
) -> Result<Ext3, PcsFriFoldError> {
    if prev_bits <= current_bits {
        return Err(PcsFriFoldError::InvalidLayerBits {
            current_bits,
            prev_bits,
        });
    }
    if n_bits_ext < prev_bits {
        return Err(PcsFriFoldError::InvalidExtensionBits {
            n_bits_ext,
            prev_bits,
        });
    }

    let fold_bits = prev_bits - current_bits;
    let expected_len = 1_usize
        .checked_shl(fold_bits)
        .ok_or(PcsFriFoldError::LengthOverflow)?;
    if values.len() != expected_len {
        return Err(PcsFriFoldError::ValueLengthMismatch {
            expected: expected_len,
            found: values.len(),
        });
    }

    let coefficients = interpolate_fold_values(values, fold_bits as usize)?;
    let shift = fold_shift(n_bits_ext, prev_bits);
    let root = Felt::root_of_unity(prev_bits as usize)
        .ok_or(PcsFriFoldError::UnsupportedRoot { bits: prev_bits })?;
    let point = shift * root.pow(index);
    let inverse = point
        .inverse()
        .ok_or(PcsFriFoldError::ZeroEvaluationPoint)?;
    Ok(evaluate_extension_polynomial(
        &coefficients,
        scale_extension(challenge, inverse),
    ))
}

pub fn verify_fri_query_path(
    root: [Felt; HASH_WORDS],
    last_level: &[[Felt; HASH_WORDS]],
    arity: usize,
    row_index: u64,
    values: &[Ext3],
    siblings: &[Vec<[Felt; HASH_WORDS]>],
) -> Result<bool, PcsFriMerkleError> {
    if values.is_empty() {
        return Err(PcsFriMerkleError::EmptyValues);
    }
    let flattened_values = flatten_extension_values(values)?;
    let mut digest = linear_hash(&flattened_values, arity)?;
    let mut path_index = row_index;
    let arity_u64 = u64::try_from(arity).map_err(|_| PcsFriMerkleError::LengthOverflow)?;
    let expected_siblings = arity
        .checked_sub(1)
        .ok_or(PcsFriMerkleError::LengthOverflow)?;

    for level in siblings {
        if level.len() != expected_siblings {
            return Err(PcsFriMerkleError::InvalidSiblingCount {
                expected: expected_siblings,
                found: level.len(),
            });
        }

        let child_slot = usize::try_from(path_index % arity_u64)
            .map_err(|_| PcsFriMerkleError::LengthOverflow)?;
        let mut children = vec![[Felt::ZERO; HASH_WORDS]; arity];
        let mut sibling_index = 0;
        for (slot, child) in children.iter_mut().enumerate() {
            if slot == child_slot {
                *child = digest;
            } else {
                *child = level[sibling_index];
                sibling_index += 1;
            }
        }
        digest = parent_hash(&children, arity)?;
        path_index /= arity_u64;
    }

    if last_level.is_empty() {
        Ok(digest == root)
    } else {
        let index = usize::try_from(path_index).map_err(|_| PcsFriMerkleError::LengthOverflow)?;
        let target = last_level
            .get(index)
            .ok_or(PcsFriMerkleError::LastLevelIndexOutOfRange {
                index: path_index,
                node_count: last_level.len(),
            })?;
        Ok(digest == *target)
    }
}

pub fn verify_fri_last_level_root(
    root: [Felt; HASH_WORDS],
    arity: usize,
    last_level: &[[Felt; HASH_WORDS]],
) -> Result<bool, PcsFriMerkleError> {
    if last_level.is_empty() {
        return Err(PcsFriMerkleError::EmptyLastLevel);
    }
    Ok(root_from_digest_level(last_level, arity)? == root)
}

fn interpolate_fold_values(values: &[Ext3], bits: usize) -> Result<Vec<Ext3>, PcsFriFoldError> {
    let mut c0 = Vec::with_capacity(values.len());
    let mut c1 = Vec::with_capacity(values.len());
    let mut c2 = Vec::with_capacity(values.len());
    for value in values {
        c0.push(value.c0);
        c1.push(value.c1);
        c2.push(value.c2);
    }
    intt_in_place(&mut c0, bits)?;
    intt_in_place(&mut c1, bits)?;
    intt_in_place(&mut c2, bits)?;
    Ok(c0
        .into_iter()
        .zip(c1)
        .zip(c2)
        .map(|((c0, c1), c2)| Ext3::new(c0, c1, c2))
        .collect())
}

fn fold_shift(n_bits_ext: u32, prev_bits: u32) -> Felt {
    let mut shift = SHIFT;
    for _ in 0..(n_bits_ext - prev_bits) {
        shift = shift * shift;
    }
    shift
}

fn evaluate_extension_polynomial(coefficients: &[Ext3], point: Ext3) -> Ext3 {
    coefficients
        .iter()
        .rev()
        .fold(Ext3::ZERO, |acc, coefficient| acc * point + *coefficient)
}

fn scale_extension(value: Ext3, scalar: Felt) -> Ext3 {
    Ext3::new(value.c0 * scalar, value.c1 * scalar, value.c2 * scalar)
}

fn flatten_extension_values(values: &[Ext3]) -> Result<Vec<Felt>, PcsFriMerkleError> {
    let len = values
        .len()
        .checked_mul(3)
        .ok_or(PcsFriMerkleError::LengthOverflow)?;
    let mut out = Vec::with_capacity(len);
    for value in values {
        out.push(value.c0);
        out.push(value.c1);
        out.push(value.c2);
    }
    Ok(out)
}

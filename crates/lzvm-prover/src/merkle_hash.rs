use std::fmt;

use lzvm_field::{poseidon2_hash_16, poseidon2_hash_8, Felt};

pub(crate) const HASH_WORDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MerkleHashError {
    UnsupportedArity { arity: usize },
    InvalidChildCount { expected: usize, found: usize },
    LengthOverflow,
}

impl fmt::Display for MerkleHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArity { arity } => {
                write!(f, "unsupported Merkle hash arity: {arity}")
            }
            Self::InvalidChildCount { expected, found } => write!(
                f,
                "invalid Merkle hash child count: expected {expected}, found {found}"
            ),
            Self::LengthOverflow => write!(f, "Merkle hash length overflow"),
        }
    }
}

impl std::error::Error for MerkleHashError {}

pub(crate) fn linear_hash(
    values: &[Felt],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
    match arity {
        2 => Ok(linear_hash_arity2(values)),
        4 => Ok(linear_hash_arity4(values)),
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

pub(crate) fn parent_hash(
    children: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
    if children.len() != arity {
        return Err(MerkleHashError::InvalidChildCount {
            expected: arity,
            found: children.len(),
        });
    }

    match arity {
        2 => Ok(parent_hash_arity2(children[0], children[1])),
        4 => Ok(parent_hash_arity4(children)),
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

pub(crate) fn root_from_digest_level(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
    validate_arity(arity)?;
    if level.is_empty() {
        return Ok([Felt::ZERO; HASH_WORDS]);
    }

    let mut level = level.to_vec();
    while level.len() > 1 {
        let extra_zeros = (arity - (level.len() % arity)) % arity;
        level.resize(
            level
                .len()
                .checked_add(extra_zeros)
                .ok_or(MerkleHashError::LengthOverflow)?,
            [Felt::ZERO; HASH_WORDS],
        );

        let mut next = Vec::with_capacity(level.len() / arity);
        for children in level.chunks_exact(arity) {
            next.push(parent_hash(children, arity)?);
        }
        level = next;
    }
    Ok(level[0])
}

fn validate_arity(arity: usize) -> Result<(), MerkleHashError> {
    match arity {
        2 | 4 => Ok(()),
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

fn linear_hash_arity2(values: &[Felt]) -> [Felt; HASH_WORDS] {
    if values.len() <= HASH_WORDS {
        let mut digest = [Felt::ZERO; HASH_WORDS];
        digest[..values.len()].copy_from_slice(values);
        return digest;
    }

    let mut state = [Felt::ZERO; 8];
    let mut offset = 0;
    while offset < values.len() {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[4..].copy_from_slice(&capacity);
        state[..HASH_WORDS].fill(Felt::ZERO);

        let chunk_len = (values.len() - offset).min(HASH_WORDS);
        state[..chunk_len].copy_from_slice(&values[offset..offset + chunk_len]);
        state = poseidon2_hash_8(state);
        offset += chunk_len;
    }

    [state[0], state[1], state[2], state[3]]
}

fn linear_hash_arity4(values: &[Felt]) -> [Felt; HASH_WORDS] {
    const RATE: usize = 12;

    if values.len() <= HASH_WORDS {
        let mut digest = [Felt::ZERO; HASH_WORDS];
        digest[..values.len()].copy_from_slice(values);
        return digest;
    }

    let mut state = [Felt::ZERO; 16];
    let mut offset = 0;
    while offset < values.len() {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[RATE..].copy_from_slice(&capacity);
        state[..RATE].fill(Felt::ZERO);

        let chunk_len = (values.len() - offset).min(RATE);
        state[..chunk_len].copy_from_slice(&values[offset..offset + chunk_len]);
        state = poseidon2_hash_16(state);
        offset += chunk_len;
    }

    [state[0], state[1], state[2], state[3]]
}

fn parent_hash_arity2(left: [Felt; HASH_WORDS], right: [Felt; HASH_WORDS]) -> [Felt; HASH_WORDS] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

fn parent_hash_arity4(children: &[[Felt; HASH_WORDS]]) -> [Felt; HASH_WORDS] {
    let state = poseidon2_hash_16([
        children[0][0],
        children[0][1],
        children[0][2],
        children[0][3],
        children[1][0],
        children[1][1],
        children[1][2],
        children[1][3],
        children[2][0],
        children[2][1],
        children[2][2],
        children[2][3],
        children[3][0],
        children[3][1],
        children[3][2],
        children[3][3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

use std::fmt;

use lzvm_field::{
    coset_extend_evaluations, poseidon2_hash_16, poseidon2_hash_8, DomainError, Felt, FieldError,
};

use crate::witness_layout::WitnessTraceStageValues;

const HASH_WORDS: usize = 4;
const WORD_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessStageLeaves {
    stage_index: usize,
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    bytes: Vec<u8>,
}

impl WitnessStageLeaves {
    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn source_row_count(&self) -> usize {
        self.source_rows
    }

    pub fn extended_row_count(&self) -> usize {
        self.extended_rows
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessStageLeafError {
    Domain(DomainError),
    LengthOverflow,
}

impl fmt::Display for WitnessStageLeafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => write!(f, "witness stage leaf domain error: {error}"),
            Self::LengthOverflow => write!(f, "witness stage leaf length overflow"),
        }
    }
}

impl std::error::Error for WitnessStageLeafError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::LengthOverflow => None,
        }
    }
}

impl From<DomainError> for WitnessStageLeafError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessStageCommitment {
    stage_index: usize,
    arity: usize,
    root: [Felt; HASH_WORDS],
    tree_bytes: Vec<u8>,
}

impl WitnessStageCommitment {
    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn arity(&self) -> usize {
        self.arity
    }

    pub fn root(&self) -> [Felt; HASH_WORDS] {
        self.root
    }

    pub fn tree_bytes(&self) -> &[u8] {
        &self.tree_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessStageCommitmentError {
    Field(FieldError),
    InvalidLeafByteLength { expected: usize, found: usize },
    UnsupportedArity { arity: usize },
    EmptyStage,
    LengthOverflow,
}

impl fmt::Display for WitnessStageCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Field(error) => write!(f, "witness stage commitment field error: {error}"),
            Self::InvalidLeafByteLength { expected, found } => write!(
                f,
                "invalid witness stage leaf byte length: expected {expected}, found {found}"
            ),
            Self::UnsupportedArity { arity } => {
                write!(f, "unsupported witness stage commitment arity: {arity}")
            }
            Self::EmptyStage => write!(f, "witness stage commitment has no rows"),
            Self::LengthOverflow => write!(f, "witness stage commitment length overflow"),
        }
    }
}

impl std::error::Error for WitnessStageCommitmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field(error) => Some(error),
            Self::InvalidLeafByteLength { .. }
            | Self::UnsupportedArity { .. }
            | Self::EmptyStage
            | Self::LengthOverflow => None,
        }
    }
}

impl From<FieldError> for WitnessStageCommitmentError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

pub fn extend_witness_stage_leaves(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
) -> Result<WitnessStageLeaves, WitnessStageLeafError> {
    let columns = stage.column_count();
    let rows = stage.row_count();
    let mut extended_columns = Vec::with_capacity(columns);
    for column in 0..columns {
        let mut source = Vec::with_capacity(rows);
        for row in 0..rows {
            let index = row
                .checked_mul(columns)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(WitnessStageLeafError::LengthOverflow)?;
            source.push(stage.values()[index]);
        }
        extended_columns.push(coset_extend_evaluations(&source, source_bits, target_bits)?);
    }

    let extended_rows = extended_columns.first().map_or(0, Vec::len);
    let byte_count = extended_rows
        .checked_mul(columns)
        .and_then(|count| count.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    let mut bytes = Vec::with_capacity(byte_count);
    for row in 0..extended_rows {
        for column_values in &extended_columns {
            bytes.extend_from_slice(&column_values[row].to_le_bytes());
        }
    }

    Ok(WitnessStageLeaves {
        stage_index: stage.stage_index(),
        source_rows: rows,
        extended_rows,
        columns,
        bytes,
    })
}

pub fn commit_witness_stage_leaves(
    leaves: &WitnessStageLeaves,
    arity: usize,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    validate_witness_commitment_arity(arity)?;
    let rows = read_witness_stage_leaf_rows(leaves)?;
    if rows.is_empty() {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }

    let mut out = Vec::with_capacity(leaves.bytes().len());
    out.extend_from_slice(leaves.bytes());

    let mut level = Vec::with_capacity(rows.len());
    for row in &rows {
        let digest = linear_hash(row, arity)?;
        append_digest(&mut out, digest);
        level.push(digest);
    }

    while level.len() > 1 {
        let extra_zeros = (arity - (level.len() % arity)) % arity;
        for _ in 0..extra_zeros {
            let zero = [Felt::ZERO; HASH_WORDS];
            append_digest(&mut out, zero);
            level.push(zero);
        }

        let mut next = Vec::with_capacity(level.len() / arity);
        for children in level.chunks_exact(arity) {
            let digest = parent_hash(children, arity)?;
            append_digest(&mut out, digest);
            next.push(digest);
        }
        level = next;
    }

    Ok(WitnessStageCommitment {
        stage_index: leaves.stage_index(),
        arity,
        root: level[0],
        tree_bytes: out,
    })
}

fn validate_witness_commitment_arity(arity: usize) -> Result<(), WitnessStageCommitmentError> {
    if matches!(arity, 2 | 4) {
        Ok(())
    } else {
        Err(WitnessStageCommitmentError::UnsupportedArity { arity })
    }
}

fn read_witness_stage_leaf_rows(
    leaves: &WitnessStageLeaves,
) -> Result<Vec<Vec<Felt>>, WitnessStageCommitmentError> {
    let expected = leaves
        .extended_row_count()
        .checked_mul(leaves.column_count())
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if leaves.bytes().len() != expected {
        return Err(WitnessStageCommitmentError::InvalidLeafByteLength {
            expected,
            found: leaves.bytes().len(),
        });
    }

    let mut rows = Vec::with_capacity(leaves.extended_row_count());
    for row in 0..leaves.extended_row_count() {
        let mut values = Vec::with_capacity(leaves.column_count());
        for column in 0..leaves.column_count() {
            let word_index = row
                .checked_mul(leaves.column_count())
                .and_then(|offset| offset.checked_add(column))
                .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(WORD_BYTES)
                .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
            let value = u64::from_le_bytes(
                leaves.bytes()[byte_index..byte_index + WORD_BYTES]
                    .try_into()
                    .expect("slice length checked"),
            );
            values.push(Felt::from_canonical(value)?);
        }
        rows.push(values);
    }
    Ok(rows)
}

fn linear_hash(
    values: &[Felt],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], WitnessStageCommitmentError> {
    match arity {
        2 => Ok(linear_hash_arity2(values)),
        4 => Ok(linear_hash_arity4(values)),
        _ => Err(WitnessStageCommitmentError::UnsupportedArity { arity }),
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

fn parent_hash(
    children: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], WitnessStageCommitmentError> {
    match arity {
        2 => Ok(parent_hash_arity2(children[0], children[1])),
        4 => Ok(parent_hash_arity4(children)),
        _ => Err(WitnessStageCommitmentError::UnsupportedArity { arity }),
    }
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

fn append_digest(out: &mut Vec<u8>, digest: [Felt; HASH_WORDS]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

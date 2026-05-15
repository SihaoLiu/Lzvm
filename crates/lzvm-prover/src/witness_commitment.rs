use std::fmt;

use lzvm_field::{coset_extend_evaluations, DomainError, Felt, FieldError};

use crate::merkle_hash::{linear_hash, parent_hash, MerkleHashError};
use crate::witness_layout::{
    derive_witness_trace_layout, WitnessTraceLayoutError, WitnessTraceStageValues,
};
use crate::witness_trace::WitnessTraceBuffer;
use crate::ProveUnitSchedule;

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
pub struct WitnessStageOpening {
    row_index: u64,
    values: Vec<Felt>,
    siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
}

impl WitnessStageOpening {
    pub fn new(
        row_index: u64,
        values: Vec<Felt>,
        siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
    ) -> Result<Self, WitnessStageOpeningError> {
        if values.is_empty() {
            return Err(WitnessStageOpeningError::EmptyValues);
        }
        Ok(Self {
            row_index,
            values,
            siblings,
        })
    }

    pub fn row_index(&self) -> u64 {
        self.row_index
    }

    pub fn values(&self) -> &[Felt] {
        &self.values
    }

    pub fn siblings(&self) -> &[Vec<[Felt; HASH_WORDS]>] {
        &self.siblings
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessStageOpeningError {
    Field(FieldError),
    Commitment(WitnessStageCommitmentError),
    RowOutOfRange { row_index: u64, row_count: u64 },
    ZeroRows,
    ZeroColumns,
    EmptyValues,
    InvalidTreeByteLength { expected: usize, found: usize },
    InvalidSiblingCount { expected: usize, found: usize },
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

impl fmt::Display for WitnessStageOpeningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Field(error) => write!(f, "witness stage opening field error: {error}"),
            Self::Commitment(error) => write!(f, "witness stage opening commitment error: {error}"),
            Self::RowOutOfRange {
                row_index,
                row_count,
            } => write!(
                f,
                "witness stage opening row index {row_index} is outside row count {row_count}"
            ),
            Self::ZeroRows => write!(f, "witness stage opening has no rows"),
            Self::ZeroColumns => write!(f, "witness stage opening has no columns"),
            Self::EmptyValues => write!(f, "witness stage opening has no values"),
            Self::InvalidTreeByteLength { expected, found } => write!(
                f,
                "invalid witness stage opening tree byte length: expected {expected}, found {found}"
            ),
            Self::InvalidSiblingCount { expected, found } => write!(
                f,
                "invalid witness stage opening sibling count: expected {expected}, found {found}"
            ),
            Self::LengthOverflow => write!(f, "witness stage opening length overflow"),
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

impl std::error::Error for WitnessStageOpeningError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field(error) => Some(error),
            Self::Commitment(error) => Some(error),
            Self::RowOutOfRange { .. }
            | Self::ZeroRows
            | Self::ZeroColumns
            | Self::EmptyValues
            | Self::InvalidTreeByteLength { .. }
            | Self::InvalidSiblingCount { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<FieldError> for WitnessStageCommitmentError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

impl From<MerkleHashError> for WitnessStageCommitmentError {
    fn from(error: MerkleHashError) -> Self {
        match error {
            MerkleHashError::UnsupportedArity { arity } => Self::UnsupportedArity { arity },
            MerkleHashError::InvalidChildCount { .. } => Self::LengthOverflow,
            MerkleHashError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

impl From<FieldError> for WitnessStageOpeningError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

impl From<MerkleHashError> for WitnessStageOpeningError {
    fn from(error: MerkleHashError) -> Self {
        match error {
            MerkleHashError::UnsupportedArity { arity } => {
                Self::Commitment(WitnessStageCommitmentError::UnsupportedArity { arity })
            }
            MerkleHashError::InvalidChildCount { expected, found } => {
                Self::InvalidSiblingCount { expected, found }
            }
            MerkleHashError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

impl From<WitnessStageCommitmentError> for WitnessStageOpeningError {
    fn from(error: WitnessStageCommitmentError) -> Self {
        Self::Commitment(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceCommitments {
    commitments: Vec<WitnessStageCommitment>,
}

impl WitnessTraceCommitments {
    pub fn stage_count(&self) -> usize {
        self.commitments.len()
    }

    pub fn commitments(&self) -> &[WitnessStageCommitment] {
        &self.commitments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessStageExtendedValues {
    stage_index: usize,
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    values: Vec<Felt>,
}

impl WitnessStageExtendedValues {
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

    pub fn values(&self) -> &[Felt] {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessTraceCommitmentError {
    Layout(WitnessTraceLayoutError),
    StageLeaf(WitnessStageLeafError),
    StageCommitment(WitnessStageCommitmentError),
    LengthOverflow,
}

impl fmt::Display for WitnessTraceCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(error) => write!(f, "witness trace commitment layout error: {error}"),
            Self::StageLeaf(error) => {
                write!(f, "witness trace commitment leaf error: {error}")
            }
            Self::StageCommitment(error) => {
                write!(f, "witness trace commitment tree error: {error}")
            }
            Self::LengthOverflow => write!(f, "witness trace commitment length overflow"),
        }
    }
}

impl std::error::Error for WitnessTraceCommitmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Layout(error) => Some(error),
            Self::StageLeaf(error) => Some(error),
            Self::StageCommitment(error) => Some(error),
            Self::LengthOverflow => None,
        }
    }
}

impl From<WitnessTraceLayoutError> for WitnessTraceCommitmentError {
    fn from(error: WitnessTraceLayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<WitnessStageLeafError> for WitnessTraceCommitmentError {
    fn from(error: WitnessStageLeafError) -> Self {
        Self::StageLeaf(error)
    }
}

impl From<WitnessStageCommitmentError> for WitnessTraceCommitmentError {
    fn from(error: WitnessStageCommitmentError) -> Self {
        Self::StageCommitment(error)
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

pub fn commit_witness_trace_stages(
    trace: &WitnessTraceBuffer,
    unit: &ProveUnitSchedule,
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    let layout = derive_witness_trace_layout(unit)?;
    let source_bits = usize::try_from(unit.base_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;
    let target_bits = usize::try_from(unit.extended_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;
    let arity = usize::try_from(unit.merkle_tree_arity)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;

    let mut commitments = Vec::with_capacity(layout.stage_count());
    for stage_info in layout.stages() {
        let stage = layout.stage_trace(trace, stage_info.stage_index)?;
        let leaves = extend_witness_stage_leaves(&stage, source_bits, target_bits)?;
        let commitment = commit_witness_stage_leaves(&leaves, arity)?;
        commitments.push(commitment);
    }

    Ok(WitnessTraceCommitments { commitments })
}

pub fn extend_witness_trace_stage_values(
    trace: &WitnessTraceBuffer,
    unit: &ProveUnitSchedule,
) -> Result<Vec<WitnessStageExtendedValues>, WitnessTraceCommitmentError> {
    let layout = derive_witness_trace_layout(unit)?;
    let source_bits = usize::try_from(unit.base_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;
    let target_bits = usize::try_from(unit.extended_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;

    let mut stages = Vec::with_capacity(layout.stage_count());
    for stage_info in layout.stages() {
        let stage = layout.stage_trace(trace, stage_info.stage_index)?;
        let leaves = extend_witness_stage_leaves(&stage, source_bits, target_bits)?;
        let values = decode_witness_stage_leaf_values(&leaves)?;
        stages.push(WitnessStageExtendedValues {
            stage_index: leaves.stage_index(),
            source_rows: leaves.source_row_count(),
            extended_rows: leaves.extended_row_count(),
            columns: leaves.column_count(),
            values,
        });
    }

    Ok(stages)
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

pub fn open_witness_stage_commitment(
    commitment: &WitnessStageCommitment,
    row_index: u64,
    row_count: u64,
    column_count: usize,
) -> Result<WitnessStageOpening, WitnessStageOpeningError> {
    validate_witness_commitment_arity(commitment.arity())?;
    if row_count == 0 {
        return Err(WitnessStageOpeningError::ZeroRows);
    }
    if column_count == 0 {
        return Err(WitnessStageOpeningError::ZeroColumns);
    }
    if row_index >= row_count {
        return Err(WitnessStageOpeningError::RowOutOfRange {
            row_index,
            row_count,
        });
    }

    let rows = usize::try_from(row_count).map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
    let query_row =
        usize::try_from(row_index).map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
    let row_byte_count = column_count
        .checked_mul(WORD_BYTES)
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let expected_tree_bytes =
        expected_witness_stage_tree_byte_count(rows, column_count, commitment.arity())?;
    if commitment.tree_bytes().len() != expected_tree_bytes {
        return Err(WitnessStageOpeningError::InvalidTreeByteLength {
            expected: expected_tree_bytes,
            found: commitment.tree_bytes().len(),
        });
    }

    let row_offset = query_row
        .checked_mul(row_byte_count)
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let values = read_witness_opening_values(commitment.tree_bytes(), row_offset, row_byte_count)?;

    let mut siblings = Vec::new();
    let mut level_offset = rows
        .checked_mul(row_byte_count)
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let mut level_len = rows;
    let mut level_query = query_row;
    while level_len > 1 {
        let padded_len = round_up_to_arity(level_len, commitment.arity())?;
        let child_slot = level_query % commitment.arity();
        let group_start = (level_query / commitment.arity())
            .checked_mul(commitment.arity())
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let mut level_siblings = Vec::with_capacity(commitment.arity() - 1);
        for slot in 0..commitment.arity() {
            if slot == child_slot {
                continue;
            }
            let child_index = group_start
                .checked_add(slot)
                .ok_or(WitnessStageOpeningError::LengthOverflow)?;
            if child_index < level_len {
                level_siblings.push(read_digest_at(
                    commitment.tree_bytes(),
                    level_offset,
                    child_index,
                )?);
            } else {
                level_siblings.push([Felt::ZERO; HASH_WORDS]);
            }
        }
        siblings.push(level_siblings);

        level_offset = level_offset
            .checked_add(
                padded_len
                    .checked_mul(HASH_WORDS * WORD_BYTES)
                    .ok_or(WitnessStageOpeningError::LengthOverflow)?,
            )
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        level_len = padded_len / commitment.arity();
        level_query /= commitment.arity();
    }

    Ok(WitnessStageOpening {
        row_index,
        values,
        siblings,
    })
}

pub fn decode_witness_stage_leaf_values(
    leaves: &WitnessStageLeaves,
) -> Result<Vec<Felt>, WitnessStageCommitmentError> {
    Ok(read_witness_stage_leaf_rows(leaves)?
        .into_iter()
        .flatten()
        .collect())
}

pub fn verify_witness_stage_opening_root(
    root: [Felt; HASH_WORDS],
    arity: usize,
    opening: &WitnessStageOpening,
) -> Result<bool, WitnessStageOpeningError> {
    validate_witness_commitment_arity(arity)?;
    if opening.values().is_empty() {
        return Err(WitnessStageOpeningError::EmptyValues);
    }

    let mut digest = linear_hash(opening.values(), arity)?;
    let mut row_index = opening.row_index();
    let arity_u64 = u64::try_from(arity).map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
    for level in opening.siblings() {
        let expected = arity
            .checked_sub(1)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        if level.len() != expected {
            return Err(WitnessStageOpeningError::InvalidSiblingCount {
                expected,
                found: level.len(),
            });
        }
        let child_slot = usize::try_from(row_index % arity_u64)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
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
        row_index /= arity_u64;
    }

    Ok(digest == root)
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

fn expected_witness_stage_tree_byte_count(
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<usize, WitnessStageOpeningError> {
    let raw_byte_count = row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let mut digest_count = row_count;
    let mut level_len = row_count;
    while level_len > 1 {
        let padded_len = round_up_to_arity(level_len, arity)?;
        digest_count = digest_count
            .checked_add(padded_len - level_len)
            .and_then(|count| count.checked_add(padded_len / arity))
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        level_len = padded_len / arity;
    }
    raw_byte_count
        .checked_add(
            digest_count
                .checked_mul(HASH_WORDS * WORD_BYTES)
                .ok_or(WitnessStageOpeningError::LengthOverflow)?,
        )
        .ok_or(WitnessStageOpeningError::LengthOverflow)
}

fn round_up_to_arity(value: usize, arity: usize) -> Result<usize, WitnessStageOpeningError> {
    let extra = (arity - (value % arity)) % arity;
    value
        .checked_add(extra)
        .ok_or(WitnessStageOpeningError::LengthOverflow)
}

fn read_witness_opening_values(
    bytes: &[u8],
    row_offset: usize,
    row_byte_count: usize,
) -> Result<Vec<Felt>, WitnessStageOpeningError> {
    let end = row_offset
        .checked_add(row_byte_count)
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let row =
        bytes
            .get(row_offset..end)
            .ok_or(WitnessStageOpeningError::InvalidTreeByteLength {
                expected: end,
                found: bytes.len(),
            })?;
    row.chunks_exact(WORD_BYTES)
        .map(|chunk| {
            let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
            Felt::from_canonical(value).map_err(WitnessStageOpeningError::Field)
        })
        .collect()
}

fn read_digest_at(
    bytes: &[u8],
    level_offset: usize,
    index: usize,
) -> Result<[Felt; HASH_WORDS], WitnessStageOpeningError> {
    let digest_offset = index
        .checked_mul(HASH_WORDS * WORD_BYTES)
        .and_then(|offset| offset.checked_add(level_offset))
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let digest_end = digest_offset
        .checked_add(HASH_WORDS * WORD_BYTES)
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let digest_bytes = bytes.get(digest_offset..digest_end).ok_or(
        WitnessStageOpeningError::InvalidTreeByteLength {
            expected: digest_end,
            found: bytes.len(),
        },
    )?;
    let mut digest = [Felt::ZERO; HASH_WORDS];
    for (word, chunk) in digest.iter_mut().zip(digest_bytes.chunks_exact(WORD_BYTES)) {
        let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
        *word = Felt::from_canonical(value)?;
    }
    Ok(digest)
}

fn append_digest(out: &mut Vec<u8>, digest: [Felt; HASH_WORDS]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

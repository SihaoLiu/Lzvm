use lzvm_field::Felt;

use super::{errors::WitnessStageOpeningError, HASH_WORDS, WORD_BYTES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessStageLeaves {
    stage_index: usize,
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    bytes: Vec<u8>,
}

impl WitnessStageLeaves {
    pub(crate) fn new(
        stage_index: usize,
        source_rows: usize,
        extended_rows: usize,
        columns: usize,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            stage_index,
            source_rows,
            extended_rows,
            columns,
            bytes,
        }
    }

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

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
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
    pub(crate) fn new(
        stage_index: usize,
        arity: usize,
        root: [Felt; HASH_WORDS],
        tree_bytes: Vec<u8>,
    ) -> Self {
        Self {
            stage_index,
            arity,
            root,
            tree_bytes,
        }
    }

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

    pub fn tree_byte_count(&self) -> usize {
        self.tree_bytes.len()
    }

    pub(crate) fn read_opening_values(
        &self,
        row_offset: usize,
        row_byte_count: usize,
    ) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        let end = row_offset
            .checked_add(row_byte_count)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let row = self.tree_bytes.get(row_offset..end).ok_or(
            WitnessStageOpeningError::InvalidTreeByteLength {
                expected: end,
                found: self.tree_byte_count(),
            },
        )?;
        row.chunks_exact(WORD_BYTES)
            .map(|chunk| {
                let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
                Felt::from_canonical(value).map_err(WitnessStageOpeningError::Field)
            })
            .collect()
    }

    pub(crate) fn read_digest_at(
        &self,
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
        let digest_bytes = self.tree_bytes.get(digest_offset..digest_end).ok_or(
            WitnessStageOpeningError::InvalidTreeByteLength {
                expected: digest_end,
                found: self.tree_byte_count(),
            },
        )?;
        let mut digest = [Felt::ZERO; HASH_WORDS];
        for (word, chunk) in digest.iter_mut().zip(digest_bytes.chunks_exact(WORD_BYTES)) {
            let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
            *word = Felt::from_canonical(value)?;
        }
        Ok(digest)
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
pub struct WitnessTraceCommitments {
    commitments: Vec<WitnessStageCommitment>,
}

impl WitnessTraceCommitments {
    pub(crate) fn new(commitments: Vec<WitnessStageCommitment>) -> Self {
        Self { commitments }
    }

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
    pub(crate) fn new(
        stage_index: usize,
        source_rows: usize,
        extended_rows: usize,
        columns: usize,
        values: Vec<Felt>,
    ) -> Self {
        Self {
            stage_index,
            source_rows,
            extended_rows,
            columns,
            values,
        }
    }

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

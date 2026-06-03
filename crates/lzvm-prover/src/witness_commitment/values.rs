use std::sync::OnceLock;

#[cfg(feature = "cuda")]
use lzvm_accel::{cuda_goldilocks_coset_extend_row_major_columns_device, CudaDeviceBuffer};
#[cfg(not(feature = "cuda"))]
use lzvm_field::coset_extend_evaluations;
use lzvm_field::Felt;

use super::{errors::WitnessStageOpeningError, HASH_WORDS, WORD_BYTES};
#[cfg(feature = "cuda")]
use crate::gpu_setup::prepare_gpu_setup;

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

#[derive(Debug, Clone)]
pub struct WitnessStageCommitment {
    stage_index: usize,
    arity: usize,
    root: [Felt; HASH_WORDS],
    tree: WitnessStageTreeStorage,
}

#[derive(Debug, Clone)]
pub(crate) struct WitnessStageCompactTreeParts {
    pub(crate) source_rows: usize,
    pub(crate) extended_rows: usize,
    pub(crate) columns: usize,
    pub(crate) source_bits: usize,
    pub(crate) target_bits: usize,
    pub(crate) source_values: Vec<Felt>,
    pub(crate) raw_leaf_bytes: usize,
    pub(crate) logical_tree_bytes: usize,
    pub(crate) digest_tree: Vec<u8>,
}

#[derive(Debug, Clone)]
enum WitnessStageTreeStorage {
    Host(Vec<u8>),
    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    Compact(Box<WitnessStageCompactTreeStorage>),
}

#[derive(Debug)]
#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
struct WitnessStageCompactTreeStorage {
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    source_bits: usize,
    target_bits: usize,
    source_values: Vec<Felt>,
    raw_leaf_bytes: usize,
    logical_tree_bytes: usize,
    digest_tree: Vec<u8>,
    materialized_tree: OnceLock<Vec<u8>>,
}

impl Clone for WitnessStageCompactTreeStorage {
    fn clone(&self) -> Self {
        let materialized_tree = OnceLock::new();
        if let Some(bytes) = self.materialized_tree.get() {
            let _ = materialized_tree.set(bytes.clone());
        }
        Self {
            source_rows: self.source_rows,
            extended_rows: self.extended_rows,
            columns: self.columns,
            source_bits: self.source_bits,
            target_bits: self.target_bits,
            source_values: self.source_values.clone(),
            raw_leaf_bytes: self.raw_leaf_bytes,
            logical_tree_bytes: self.logical_tree_bytes,
            digest_tree: self.digest_tree.clone(),
            materialized_tree,
        }
    }
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
            tree: WitnessStageTreeStorage::Host(tree_bytes),
        }
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn new_compact(
        stage_index: usize,
        arity: usize,
        root: [Felt; HASH_WORDS],
        parts: WitnessStageCompactTreeParts,
    ) -> Self {
        Self {
            stage_index,
            arity,
            root,
            tree: WitnessStageTreeStorage::Compact(Box::new(WitnessStageCompactTreeStorage {
                source_rows: parts.source_rows,
                extended_rows: parts.extended_rows,
                columns: parts.columns,
                source_bits: parts.source_bits,
                target_bits: parts.target_bits,
                source_values: parts.source_values,
                raw_leaf_bytes: parts.raw_leaf_bytes,
                logical_tree_bytes: parts.logical_tree_bytes,
                digest_tree: parts.digest_tree,
                materialized_tree: OnceLock::new(),
            })),
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
        match &self.tree {
            WitnessStageTreeStorage::Host(bytes) => bytes,
            WitnessStageTreeStorage::Compact(storage) => storage.materialized_tree_bytes(),
        }
    }

    pub fn tree_byte_count(&self) -> usize {
        match &self.tree {
            WitnessStageTreeStorage::Host(bytes) => bytes.len(),
            WitnessStageTreeStorage::Compact(storage) => storage.logical_tree_bytes,
        }
    }

    pub(crate) fn read_opening_values(
        &self,
        row_offset: usize,
        row_byte_count: usize,
    ) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        let end = row_offset
            .checked_add(row_byte_count)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let row = match &self.tree {
            WitnessStageTreeStorage::Host(tree_bytes) => tree_bytes.get(row_offset..end).ok_or(
                WitnessStageOpeningError::InvalidTreeByteLength {
                    expected: end,
                    found: self.tree_byte_count(),
                },
            )?,
            WitnessStageTreeStorage::Compact(storage) => {
                return storage.read_opening_values(row_offset, row_byte_count);
            }
        };
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
        let digest_bytes = match &self.tree {
            WitnessStageTreeStorage::Host(tree_bytes) => tree_bytes.get(digest_offset..digest_end),
            WitnessStageTreeStorage::Compact(storage) => {
                storage.read_digest_bytes(digest_offset, digest_end)
            }
        }
        .ok_or(WitnessStageOpeningError::InvalidTreeByteLength {
            expected: digest_end,
            found: self.tree_byte_count(),
        })?;
        let mut digest = [Felt::ZERO; HASH_WORDS];
        for (word, chunk) in digest.iter_mut().zip(digest_bytes.chunks_exact(WORD_BYTES)) {
            let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
            *word = Felt::from_canonical(value)?;
        }
        Ok(digest)
    }
}

impl PartialEq for WitnessStageCommitment {
    fn eq(&self, other: &Self) -> bool {
        self.stage_index == other.stage_index
            && self.arity == other.arity
            && self.root == other.root
            && self.tree_byte_count() == other.tree_byte_count()
            && self.tree_bytes() == other.tree_bytes()
    }
}

impl Eq for WitnessStageCommitment {}

impl WitnessStageCompactTreeStorage {
    fn read_opening_values(
        &self,
        row_offset: usize,
        row_byte_count: usize,
    ) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        if row_byte_count != self.columns * WORD_BYTES || !row_offset.is_multiple_of(row_byte_count)
        {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        let row = row_offset / row_byte_count;
        if row >= self.extended_rows {
            return Err(WitnessStageOpeningError::InvalidTreeByteLength {
                expected: row_offset + row_byte_count,
                found: self.logical_tree_bytes,
            });
        }
        self.extended_row_values(row)
    }

    fn read_digest_bytes(&self, digest_offset: usize, digest_end: usize) -> Option<&[u8]> {
        if digest_offset < self.raw_leaf_bytes || digest_end < self.raw_leaf_bytes {
            return None;
        }
        let start = digest_offset - self.raw_leaf_bytes;
        let end = digest_end - self.raw_leaf_bytes;
        self.digest_tree.get(start..end)
    }

    fn materialized_tree_bytes(&self) -> &[u8] {
        self.materialized_tree
            .get_or_init(|| {
                let mut bytes = self
                    .extended_leaf_bytes()
                    .expect("compact witness leaves should materialize");
                bytes.extend_from_slice(&self.digest_tree);
                bytes
            })
            .as_slice()
    }

    fn extended_row_values(&self, row: usize) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        #[cfg(feature = "cuda")]
        {
            self.extended_row_values_cuda(row)
        }
        #[cfg(not(feature = "cuda"))]
        {
            self.extended_row_values_cpu(row)
        }
    }

    #[cfg(not(feature = "cuda"))]
    fn extended_row_values_cpu(&self, row: usize) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        let mut out = Vec::with_capacity(self.columns);
        for column in 0..self.columns {
            let source = self.source_column_values(column)?;
            let extended = coset_extend_evaluations(&source, self.source_bits, self.target_bits)
                .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
            out.push(extended[row]);
        }
        Ok(out)
    }

    fn extended_leaf_bytes(&self) -> Result<Vec<u8>, WitnessStageOpeningError> {
        #[cfg(feature = "cuda")]
        {
            self.extended_leaf_bytes_cuda()
        }
        #[cfg(not(feature = "cuda"))]
        {
            self.extended_leaf_bytes_cpu()
        }
    }

    #[cfg(not(feature = "cuda"))]
    fn extended_leaf_bytes_cpu(&self) -> Result<Vec<u8>, WitnessStageOpeningError> {
        let mut extended_columns = Vec::with_capacity(self.columns);
        for column in 0..self.columns {
            let source = self.source_column_values(column)?;
            extended_columns.push(
                coset_extend_evaluations(&source, self.source_bits, self.target_bits)
                    .map_err(|_| WitnessStageOpeningError::LengthOverflow)?,
            );
        }
        let mut bytes = Vec::with_capacity(self.raw_leaf_bytes);
        for row in 0..self.extended_rows {
            for column_values in &extended_columns {
                bytes.extend_from_slice(&column_values[row].to_le_bytes());
            }
        }
        Ok(bytes)
    }

    #[cfg(feature = "cuda")]
    fn extended_row_values_cuda(&self, row: usize) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        let output = self.extended_rows_device()?;
        let row_byte_count = self
            .columns
            .checked_mul(WORD_BYTES)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let row_offset = row
            .checked_mul(row_byte_count)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let mut bytes = vec![0_u8; row_byte_count];
        output
            .copy_range_to(row_offset, &mut bytes)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        bytes
            .chunks_exact(WORD_BYTES)
            .map(|chunk| {
                let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
                Felt::from_canonical(value).map_err(WitnessStageOpeningError::Field)
            })
            .collect()
    }

    #[cfg(feature = "cuda")]
    fn extended_leaf_bytes_cuda(&self) -> Result<Vec<u8>, WitnessStageOpeningError> {
        self.extended_rows_device()?
            .to_vec()
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)
    }

    #[cfg(feature = "cuda")]
    fn extended_rows_device(&self) -> Result<CudaDeviceBuffer, WitnessStageOpeningError> {
        prepare_gpu_setup(self.target_bits)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        let source_buffer =
            CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&self.source_values))
                .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        let mut output_buffer = CudaDeviceBuffer::new(self.raw_leaf_bytes)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source_buffer,
            &mut output_buffer,
            self.columns,
            self.source_bits,
            self.target_bits,
        )
        .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        Ok(output_buffer)
    }

    #[cfg(not(feature = "cuda"))]
    fn source_column_values(&self, column: usize) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        if column >= self.columns {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        if self.source_values.len() != self.source_rows * self.columns {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        Ok((0..self.source_rows)
            .map(|row| self.source_values[row * self.columns + column])
            .collect())
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

use std::fmt;

#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_device_synchronize,
    cuda_poseidon2_begin_width16_linear_round_row_major_digest_device_on_stream,
    cuda_poseidon2_begin_width8_linear_round_row_major_digest_device_on_stream,
    cuda_poseidon2_width16_linear_round_column_major_digest_device,
    cuda_poseidon2_width16_linear_round_device,
    cuda_poseidon2_width16_linear_round_row_major_digest_device,
    cuda_poseidon2_width16_merkle_digest_opening_path_device,
    cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_buffer,
    cuda_poseidon2_width16_merkle_digest_opening_prefix_device,
    cuda_poseidon2_width16_merkle_digest_opening_suffixes_batch_device_buffers,
    cuda_poseidon2_width16_merkle_digest_parent_device,
    cuda_poseidon2_width16_merkle_digest_root_device,
    cuda_poseidon2_width16_merkle_digest_root_device_buffer,
    cuda_poseidon2_width16_merkle_digest_selected_parent_device,
    cuda_poseidon2_width16_merkle_parent_device,
    cuda_poseidon2_width8_linear_round_column_major_digest_device,
    cuda_poseidon2_width8_linear_round_device,
    cuda_poseidon2_width8_linear_round_row_major_digest_device,
    cuda_poseidon2_width8_merkle_digest_opening_path_device,
    cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_buffer,
    cuda_poseidon2_width8_merkle_digest_opening_prefix_device,
    cuda_poseidon2_width8_merkle_digest_opening_suffixes_batch_device_buffers,
    cuda_poseidon2_width8_merkle_digest_parent_device,
    cuda_poseidon2_width8_merkle_digest_root_device,
    cuda_poseidon2_width8_merkle_digest_root_device_buffer,
    cuda_poseidon2_width8_merkle_digest_selected_parent_device,
    cuda_poseidon2_width8_merkle_parent_device, CudaDeviceBuffer,
    CudaMerkleDigestOpeningSuffixSource as AccelMerkleDigestOpeningSuffixSource,
    CudaPinnedHostBuffer, CudaStream,
};
#[cfg(all(test, feature = "cuda"))]
use lzvm_accel::{cuda_poseidon2_width16_device, cuda_poseidon2_width8_device};
use lzvm_field::{poseidon2_hash_16, poseidon2_hash_8, Felt, FieldError};

pub(crate) const HASH_WORDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MerkleHashError {
    UnsupportedArity {
        arity: usize,
    },
    InvalidChildCount {
        expected: usize,
        found: usize,
    },
    Field(FieldError),
    #[cfg(feature = "cuda")]
    Accel(lzvm_accel::AccelError),
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
            Self::Field(error) => write!(f, "Merkle hash field error: {error}"),
            #[cfg(feature = "cuda")]
            Self::Accel(error) => write!(f, "Merkle hash cuda error: {error}"),
            Self::LengthOverflow => write!(f, "Merkle hash length overflow"),
        }
    }
}

impl std::error::Error for MerkleHashError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::Accel(error) => Some(error),
            Self::UnsupportedArity { .. }
            | Self::InvalidChildCount { .. }
            | Self::LengthOverflow => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MerkleParentLevel {
    pub(crate) padding_count: usize,
    pub(crate) parents: Vec<[Felt; HASH_WORDS]>,
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) struct CudaDigestLevel {
    digests: CudaDeviceBuffer,
    state_count: usize,
    arity: usize,
    root_operation: CudaPoseidon2RootOp,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) struct CudaMerkleOpeningPath {
    pub(crate) root: [Felt; HASH_WORDS],
    pub(crate) siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
}

#[cfg(feature = "cuda")]
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub(crate) struct CudaDigestCheckpointLevel {
    level: CudaDigestLevel,
    source_state_count: usize,
    folded_level_count: usize,
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) struct CudaMerkleSiblingBatchDeviceBuffer {
    buffer: CudaDeviceBuffer,
    row_count: usize,
    level_count: usize,
    arity: usize,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Copy)]
pub(crate) struct CudaDigestCheckpointOpeningSource<'a> {
    checkpoint: &'a CudaDigestCheckpointLevel,
    source_row: usize,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CudaDigestCheckpointOpeningKey {
    checkpoint_device_address: usize,
    source_row: usize,
}

#[cfg(feature = "cuda")]
#[derive(Clone)]
pub(crate) struct CudaDigestCheckpointSiblingBatch {
    key: CudaDigestCheckpointOpeningKey,
    siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
}

#[cfg(feature = "cuda")]
pub(crate) type CudaMerkleSiblingBatch = Vec<Vec<Vec<[Felt; HASH_WORDS]>>>;

#[cfg(feature = "cuda")]
pub(crate) type CudaMerkleSiblingBatchGroup = Vec<CudaMerkleSiblingBatch>;

#[cfg(feature = "cuda")]
impl CudaMerkleSiblingBatchDeviceBuffer {
    fn new(
        buffer: CudaDeviceBuffer,
        row_count: usize,
        level_count: usize,
        arity: usize,
    ) -> Result<Self, MerkleHashError> {
        let row_words = merkle_sibling_row_word_count(level_count, arity)?;
        let expected_bytes = row_count
            .checked_mul(row_words)
            .and_then(|words| words.checked_mul(std::mem::size_of::<u64>()))
            .ok_or(MerkleHashError::LengthOverflow)?;
        if buffer.len() != expected_bytes {
            return Err(MerkleHashError::LengthOverflow);
        }
        Ok(Self {
            buffer,
            row_count,
            level_count,
            arity,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn buffer(&self) -> &CudaDeviceBuffer {
        &self.buffer
    }

    pub(crate) fn into_siblings(self) -> Result<CudaMerkleSiblingBatch, MerkleHashError> {
        let sibling_words = self.buffer.to_u64_words().map_err(MerkleHashError::Accel)?;
        decoded_merkle_siblings_from_words(
            &sibling_words,
            self.row_count,
            self.level_count,
            self.arity,
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn into_siblings_many(
        batches: Vec<Self>,
    ) -> Result<CudaMerkleSiblingBatchGroup, MerkleHashError> {
        let Some(first) = batches.first() else {
            return Ok(Vec::new());
        };
        let level_count = first.level_count;
        let arity = first.arity;
        if batches
            .iter()
            .any(|batch| batch.level_count != level_count || batch.arity != arity)
        {
            return Err(MerkleHashError::LengthOverflow);
        }
        let row_words = merkle_sibling_row_word_count(level_count, arity)?;
        let total_rows = batches.iter().try_fold(0usize, |acc, batch| {
            acc.checked_add(batch.row_count)
                .ok_or(MerkleHashError::LengthOverflow)
        })?;
        let mut sources = Vec::with_capacity(total_rows);
        for batch in &batches {
            for row in 0..batch.row_count {
                sources.push((&batch.buffer, batch.row_count, row));
            }
        }
        let words = if sources.is_empty() {
            Vec::new()
        } else {
            CudaDeviceBuffer::from_device_row_major_u64_rows(&sources, row_words)
                .map_err(MerkleHashError::Accel)?
                .to_u64_words()
                .map_err(MerkleHashError::Accel)?
        };
        let decoded = decoded_merkle_siblings_from_words(&words, total_rows, level_count, arity)?;
        let mut decoded = decoded.into_iter();
        let mut out = Vec::with_capacity(batches.len());
        for batch in batches {
            out.push(decoded.by_ref().take(batch.row_count).collect());
        }
        Ok(out)
    }

    pub(crate) fn concat_levels(self, suffix: Self) -> Result<Self, MerkleHashError> {
        if self.row_count != suffix.row_count || self.arity != suffix.arity {
            return Err(MerkleHashError::LengthOverflow);
        }
        let prefix_row_words = merkle_sibling_row_word_count(self.level_count, self.arity)?;
        let suffix_row_words = merkle_sibling_row_word_count(suffix.level_count, suffix.arity)?;
        let level_count = self
            .level_count
            .checked_add(suffix.level_count)
            .ok_or(MerkleHashError::LengthOverflow)?;
        let buffer = CudaDeviceBuffer::from_device_row_major_u64_row_concat(
            &self.buffer,
            self.row_count,
            prefix_row_words,
            &suffix.buffer,
            suffix.row_count,
            suffix_row_words,
        )
        .map_err(MerkleHashError::Accel)?;
        Self::new(buffer, self.row_count, level_count, self.arity)
    }
}

#[cfg(feature = "cuda")]
impl<'a> CudaDigestCheckpointOpeningSource<'a> {
    pub(crate) fn new(
        checkpoint: &'a CudaDigestCheckpointLevel,
        source_row: usize,
    ) -> Result<Self, MerkleHashError> {
        if source_row >= checkpoint.source_state_count() {
            return Err(MerkleHashError::LengthOverflow);
        }
        Ok(Self {
            checkpoint,
            source_row,
        })
    }

    pub(crate) fn batch_key(&self) -> Result<(usize, usize), MerkleHashError> {
        Ok((
            self.checkpoint.arity(),
            merkle_opening_level_count(self.checkpoint.state_count(), self.checkpoint.arity())?,
        ))
    }

    pub(crate) fn key(&self) -> CudaDigestCheckpointOpeningKey {
        CudaDigestCheckpointOpeningKey {
            checkpoint_device_address: self.checkpoint.level.digests.as_raw_ptr() as usize,
            source_row: self.source_row,
        }
    }

    fn checkpoint_row(&self) -> Result<usize, MerkleHashError> {
        Ok(self.source_row / self.checkpoint.source_leaf_span()?)
    }
}

#[cfg(feature = "cuda")]
impl CudaDigestCheckpointSiblingBatch {
    pub(crate) fn into_siblings_for_source_rows(
        self,
        checkpoint: &CudaDigestCheckpointLevel,
        source_rows: &[usize],
    ) -> Result<CudaMerkleSiblingBatch, MerkleHashError> {
        if checkpoint.level.digests.as_raw_ptr() as usize != self.key.checkpoint_device_address
            || source_rows != [self.key.source_row]
        {
            return Err(MerkleHashError::LengthOverflow);
        }
        Ok(vec![self.siblings])
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn opening_path_siblings_across_digest_checkpoints(
    sources: &[CudaDigestCheckpointOpeningSource<'_>],
) -> Result<Vec<CudaDigestCheckpointSiblingBatch>, MerkleHashError> {
    const GROUP_SIZE: usize = 64;

    let Some(first) = sources.first() else {
        return Ok(Vec::new());
    };
    let (arity, level_count) = first.batch_key()?;
    if sources
        .iter()
        .any(|source| source.batch_key() != Ok((arity, level_count)))
    {
        return Err(MerkleHashError::LengthOverflow);
    }

    let mut device_batches = Vec::with_capacity(sources.len());
    for source_group in sources.chunks(GROUP_SIZE) {
        let accel_sources = source_group
            .iter()
            .map(|source| {
                Ok(AccelMerkleDigestOpeningSuffixSource {
                    values: &source.checkpoint.level.digests,
                    query_index: source.checkpoint_row()?,
                })
            })
            .collect::<Result<Vec<_>, MerkleHashError>>()?;
        let buffers = match arity {
            2 => cuda_poseidon2_width8_merkle_digest_opening_suffixes_batch_device_buffers(
                &accel_sources,
            ),
            4 => cuda_poseidon2_width16_merkle_digest_opening_suffixes_batch_device_buffers(
                &accel_sources,
            ),
            _ => return Err(MerkleHashError::UnsupportedArity { arity }),
        }
        .map_err(MerkleHashError::Accel)?;
        if buffers.len() != source_group.len() {
            return Err(MerkleHashError::LengthOverflow);
        }
        for buffer in buffers {
            device_batches.push(CudaMerkleSiblingBatchDeviceBuffer::new(
                buffer,
                1,
                level_count,
                arity,
            )?);
        }
    }

    let decoded = CudaMerkleSiblingBatchDeviceBuffer::into_siblings_many(device_batches)?;
    if decoded.len() != sources.len() {
        return Err(MerkleHashError::LengthOverflow);
    }
    sources
        .iter()
        .zip(decoded)
        .map(|(source, mut sibling_batch)| {
            if sibling_batch.len() != 1 {
                return Err(MerkleHashError::LengthOverflow);
            }
            Ok(CudaDigestCheckpointSiblingBatch {
                key: source.key(),
                siblings: sibling_batch.pop().ok_or(MerkleHashError::LengthOverflow)?,
            })
        })
        .collect()
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) struct CudaDigestRoot {
    root: CudaDeviceBuffer,
}

#[cfg(feature = "cuda")]
pub(crate) struct PendingCudaDigestRootMaterialization {
    _root: CudaDigestRoot,
    output: CudaPinnedHostBuffer,
}

#[cfg(feature = "cuda")]
pub(crate) struct PendingCudaDigestRootMaterializationBatch {
    _roots: Vec<CudaDigestRoot>,
    _gathered_roots: CudaDeviceBuffer,
    output: CudaPinnedHostBuffer,
    root_count: usize,
}

#[cfg(feature = "cuda")]
impl CudaDigestRoot {
    fn new(root: CudaDeviceBuffer) -> Self {
        Self { root }
    }

    pub(crate) fn begin_materialize_on_default_stream(
        self,
    ) -> Result<PendingCudaDigestRootMaterialization, MerkleHashError> {
        let mut output = CudaPinnedHostBuffer::new(HASH_WORDS * std::mem::size_of::<u64>())
            .map_err(MerkleHashError::Accel)?;
        unsafe {
            self.root
                .copy_to_pinned_on_default_stream(&mut output)
                .map_err(MerkleHashError::Accel)?;
        }
        Ok(PendingCudaDigestRootMaterialization {
            _root: self,
            output,
        })
    }

    pub(crate) fn begin_materialize_batch_on_default_stream(
        roots: Vec<Self>,
    ) -> Result<PendingCudaDigestRootMaterializationBatch, MerkleHashError> {
        let root_count = roots.len();
        if root_count == 0 {
            return Err(MerkleHashError::LengthOverflow);
        }
        let byte_count = root_count
            .checked_mul(HASH_WORDS)
            .and_then(|word_count| word_count.checked_mul(std::mem::size_of::<u64>()))
            .ok_or(MerkleHashError::LengthOverflow)?;
        let root_rows = roots
            .iter()
            .map(|root| (&root.root, 1, 0))
            .collect::<Vec<_>>();
        let gathered_roots =
            CudaDeviceBuffer::from_device_row_major_u64_rows(&root_rows, HASH_WORDS)
                .map_err(MerkleHashError::Accel)?;
        let mut output = CudaPinnedHostBuffer::new(byte_count).map_err(MerkleHashError::Accel)?;
        unsafe {
            gathered_roots
                .copy_to_pinned_on_default_stream(&mut output)
                .map_err(MerkleHashError::Accel)?;
        }
        Ok(PendingCudaDigestRootMaterializationBatch {
            _roots: roots,
            _gathered_roots: gathered_roots,
            output,
            root_count,
        })
    }
}

#[cfg(feature = "cuda")]
impl PendingCudaDigestRootMaterialization {
    pub(crate) fn finish_after_device_synchronize(
        self,
    ) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
        digest_from_state_bytes(unsafe { self.output.as_bytes() })
    }
}

#[cfg(feature = "cuda")]
impl PendingCudaDigestRootMaterializationBatch {
    pub(crate) fn finish_index_after_device_synchronize(
        &self,
        index: usize,
    ) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
        if index >= self.root_count {
            return Err(MerkleHashError::LengthOverflow);
        }
        let root_byte_count = HASH_WORDS * std::mem::size_of::<u64>();
        let start = index
            .checked_mul(root_byte_count)
            .ok_or(MerkleHashError::LengthOverflow)?;
        let end = start
            .checked_add(root_byte_count)
            .ok_or(MerkleHashError::LengthOverflow)?;
        let bytes = unsafe { self.output.as_bytes() };
        let root_bytes = bytes
            .get(start..end)
            .ok_or(MerkleHashError::LengthOverflow)?;
        digest_from_state_bytes(root_bytes)
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn synchronize_cuda_digest_root_materializations() -> Result<(), MerkleHashError> {
    cuda_device_synchronize().map_err(MerkleHashError::Accel)
}

#[cfg(feature = "cuda")]
fn digest_from_state_bytes(bytes: &[u8]) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
    if bytes.len() != HASH_WORDS * std::mem::size_of::<u64>() {
        return Err(MerkleHashError::LengthOverflow);
    }
    let mut words = [0_u64; HASH_WORDS];
    for (word, chunk) in words.iter_mut().zip(bytes.chunks_exact(8)) {
        let mut raw = [0_u8; 8];
        raw.copy_from_slice(chunk);
        *word = u64::from_le_bytes(raw);
    }
    digest_from_state_words(&words)
}

#[cfg(feature = "cuda")]
fn merkle_sibling_row_word_count(
    level_count: usize,
    arity: usize,
) -> Result<usize, MerkleHashError> {
    match arity {
        2 | 4 => level_count
            .checked_mul(arity - 1)
            .and_then(|count| count.checked_mul(HASH_WORDS))
            .ok_or(MerkleHashError::LengthOverflow),
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

#[cfg(feature = "cuda")]
fn merkle_opening_level_count(
    mut state_count: usize,
    arity: usize,
) -> Result<usize, MerkleHashError> {
    match arity {
        2 | 4 => {}
        _ => return Err(MerkleHashError::UnsupportedArity { arity }),
    }
    let mut level_count = 0usize;
    while state_count > 1 {
        level_count = level_count
            .checked_add(1)
            .ok_or(MerkleHashError::LengthOverflow)?;
        state_count = state_count.div_ceil(arity);
    }
    Ok(level_count)
}

#[cfg(feature = "cuda")]
fn decoded_merkle_siblings_from_words(
    sibling_words: &[u64],
    row_count: usize,
    level_count: usize,
    arity: usize,
) -> Result<Vec<Vec<Vec<[Felt; HASH_WORDS]>>>, MerkleHashError> {
    let row_words = merkle_sibling_row_word_count(level_count, arity)?;
    let expected_words = row_count
        .checked_mul(row_words)
        .ok_or(MerkleHashError::LengthOverflow)?;
    if sibling_words.len() != expected_words {
        return Err(MerkleHashError::LengthOverflow);
    }

    let mut batch_siblings = Vec::with_capacity(row_count);
    for row_words_slice in sibling_words.chunks_exact(row_words) {
        let mut cursor = 0usize;
        let mut siblings = Vec::with_capacity(level_count);
        for _ in 0..level_count {
            let mut level_siblings = Vec::with_capacity(arity - 1);
            for _ in 0..arity - 1 {
                let end = cursor
                    .checked_add(HASH_WORDS)
                    .ok_or(MerkleHashError::LengthOverflow)?;
                let words = row_words_slice
                    .get(cursor..end)
                    .ok_or(MerkleHashError::LengthOverflow)?;
                level_siblings.push(digest_from_state_words(words)?);
                cursor = end;
            }
            siblings.push(level_siblings);
        }
        batch_siblings.push(siblings);
    }
    Ok(batch_siblings)
}

#[cfg(feature = "cuda")]
impl CudaDigestLevel {
    fn new(
        digests: CudaDeviceBuffer,
        state_count: usize,
        arity: usize,
        root_operation: CudaPoseidon2RootOp,
    ) -> Self {
        Self {
            digests,
            state_count,
            arity,
            root_operation,
        }
    }

    pub(crate) fn state_count(&self) -> usize {
        self.state_count
    }

    pub(crate) fn arity(&self) -> usize {
        self.arity
    }

    pub(crate) fn byte_len(&self) -> usize {
        self.digests.len()
    }

    pub(crate) fn to_digests(&self) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
        let output = self
            .digests
            .to_u64_words()
            .map_err(MerkleHashError::Accel)?;
        digests_from_hashed_states(&output, HASH_WORDS)
    }

    pub(crate) fn root(&self) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
        let root_words = (self.root_operation)(&self.digests).map_err(MerkleHashError::Accel)?;
        digest_from_state_words(&root_words)
    }

    pub(crate) fn root_device(&self) -> Result<CudaDigestRoot, MerkleHashError> {
        let root = match self.arity {
            2 => cuda_poseidon2_width8_merkle_digest_root_device_buffer(&self.digests),
            4 => cuda_poseidon2_width16_merkle_digest_root_device_buffer(&self.digests),
            _ => return Err(MerkleHashError::UnsupportedArity { arity: self.arity }),
        }
        .map_err(MerkleHashError::Accel)?;
        Ok(CudaDigestRoot::new(root))
    }

    pub(crate) fn parent_level(&self) -> Result<Self, MerkleHashError> {
        if self.state_count <= 1 {
            return Err(MerkleHashError::LengthOverflow);
        }
        let parent_count = self.state_count.div_ceil(self.arity);
        let mut parent_digests = CudaDeviceBuffer::new(
            parent_count
                .checked_mul(HASH_WORDS)
                .and_then(|word_count| word_count.checked_mul(8))
                .ok_or(MerkleHashError::LengthOverflow)?,
        )
        .map_err(MerkleHashError::Accel)?;
        match self.arity {
            2 => cuda_poseidon2_width8_merkle_digest_parent_device(
                &self.digests,
                &mut parent_digests,
            ),
            4 => cuda_poseidon2_width16_merkle_digest_parent_device(
                &self.digests,
                &mut parent_digests,
            ),
            _ => return Err(MerkleHashError::UnsupportedArity { arity: self.arity }),
        }
        .map_err(MerkleHashError::Accel)?;
        Ok(Self::new(
            parent_digests,
            parent_count,
            self.arity,
            self.root_operation,
        ))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn into_parent_checkpoint_level(
        self,
        max_state_count: usize,
    ) -> Result<CudaDigestCheckpointLevel, MerkleHashError> {
        if self.state_count == 0 || max_state_count == 0 {
            return Err(MerkleHashError::LengthOverflow);
        }
        let source_state_count = self.state_count;
        let mut level = self;
        let mut folded_level_count = 0;
        while level.state_count() > max_state_count && level.state_count() > 1 {
            level = level.parent_level()?;
            folded_level_count += 1;
        }
        Ok(CudaDigestCheckpointLevel {
            level,
            source_state_count,
            folded_level_count,
        })
    }

    pub(crate) fn parent_checkpoint_level(
        &self,
        max_state_count: usize,
    ) -> Result<Option<CudaDigestCheckpointLevel>, MerkleHashError> {
        if self.state_count == 0 || max_state_count == 0 {
            return Err(MerkleHashError::LengthOverflow);
        }
        if self.state_count <= max_state_count || self.state_count <= 1 {
            return Ok(None);
        }
        let source_state_count = self.state_count;
        let mut level = self.parent_level()?;
        let mut folded_level_count = 1;
        while level.state_count() > max_state_count && level.state_count() > 1 {
            level = level.parent_level()?;
            folded_level_count += 1;
        }
        Ok(Some(CudaDigestCheckpointLevel {
            level,
            source_state_count,
            folded_level_count,
        }))
    }

    #[allow(dead_code)]
    pub(crate) fn selected_parent(
        &self,
        parent_index: usize,
    ) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
        let parent_count = self.state_count.div_ceil(self.arity);
        if self.state_count == 0 || parent_index >= parent_count {
            return Err(MerkleHashError::LengthOverflow);
        }
        let parent_words = match self.arity {
            2 => cuda_poseidon2_width8_merkle_digest_selected_parent_device(
                &self.digests,
                parent_index,
            ),
            4 => cuda_poseidon2_width16_merkle_digest_selected_parent_device(
                &self.digests,
                parent_index,
            ),
            _ => return Err(MerkleHashError::UnsupportedArity { arity: self.arity }),
        }
        .map_err(MerkleHashError::Accel)?;
        digest_from_state_words(&parent_words)
    }

    #[allow(dead_code)]
    pub(crate) fn opening_path(
        &self,
        query_row: usize,
    ) -> Result<CudaMerkleOpeningPath, MerkleHashError> {
        if query_row >= self.state_count {
            return Err(MerkleHashError::LengthOverflow);
        }
        let path = match self.arity {
            2 => cuda_poseidon2_width8_merkle_digest_opening_path_device(&self.digests, query_row),
            4 => cuda_poseidon2_width16_merkle_digest_opening_path_device(&self.digests, query_row),
            _ => return Err(MerkleHashError::UnsupportedArity { arity: self.arity }),
        }
        .map_err(MerkleHashError::Accel)?;
        let root = digest_from_state_words(&path.root)?;
        let mut state_count = self.state_count;
        let mut siblings = Vec::new();
        let mut cursor = 0_usize;

        while state_count > 1 {
            let mut level_siblings = Vec::with_capacity(self.arity - 1);
            for _ in 0..self.arity - 1 {
                let end = cursor
                    .checked_add(HASH_WORDS)
                    .ok_or(MerkleHashError::LengthOverflow)?;
                let words = path
                    .siblings
                    .get(cursor..end)
                    .ok_or(MerkleHashError::LengthOverflow)?;
                level_siblings.push(digest_from_state_words(words)?);
                cursor = end;
            }
            siblings.push(level_siblings);
            state_count = state_count.div_ceil(self.arity);
        }

        if cursor != path.siblings.len() {
            return Err(MerkleHashError::LengthOverflow);
        }
        Ok(CudaMerkleOpeningPath { root, siblings })
    }

    pub(crate) fn opening_path_siblings(
        &self,
        query_row: usize,
    ) -> Result<Vec<Vec<[Felt; HASH_WORDS]>>, MerkleHashError> {
        if query_row >= self.state_count {
            return Err(MerkleHashError::LengthOverflow);
        }
        let mut level_count = 0usize;
        let mut state_count = self.state_count;
        while state_count > 1 {
            level_count = level_count
                .checked_add(1)
                .ok_or(MerkleHashError::LengthOverflow)?;
            state_count = state_count.div_ceil(self.arity);
        }
        self.opening_path_prefix_for_source_row(query_row, level_count)
    }

    pub(crate) fn opening_path_siblings_batch(
        &self,
        query_rows: &[usize],
    ) -> Result<Vec<Vec<Vec<[Felt; HASH_WORDS]>>>, MerkleHashError> {
        if query_rows.is_empty() {
            return Ok(Vec::new());
        }
        if query_rows
            .iter()
            .any(|query_row| *query_row >= self.state_count)
        {
            return Err(MerkleHashError::LengthOverflow);
        }
        let mut level_count = 0usize;
        let mut state_count = self.state_count;
        while state_count > 1 {
            level_count = level_count
                .checked_add(1)
                .ok_or(MerkleHashError::LengthOverflow)?;
            state_count = state_count.div_ceil(self.arity);
        }
        self.opening_path_prefix_batch_for_source_rows(query_rows, level_count)
    }

    pub(crate) fn opening_path_siblings_batch_device(
        &self,
        query_rows: &[usize],
    ) -> Result<CudaMerkleSiblingBatchDeviceBuffer, MerkleHashError> {
        if query_rows
            .iter()
            .any(|query_row| *query_row >= self.state_count)
        {
            return Err(MerkleHashError::LengthOverflow);
        }
        let level_count = merkle_opening_level_count(self.state_count, self.arity)?;
        self.opening_path_prefix_batch_device_for_source_rows(query_rows, level_count)
    }

    #[allow(dead_code)]
    pub(crate) fn opening_path_prefix_for_source_row(
        &self,
        source_row: usize,
        folded_level_count: usize,
    ) -> Result<Vec<Vec<[Felt; HASH_WORDS]>>, MerkleHashError> {
        if source_row >= self.state_count {
            return Err(MerkleHashError::LengthOverflow);
        }
        if folded_level_count == 0 {
            return Ok(Vec::new());
        }

        let mut path_level_count = 0usize;
        let mut state_count = self.state_count;
        while state_count > 1 {
            path_level_count = path_level_count
                .checked_add(1)
                .ok_or(MerkleHashError::LengthOverflow)?;
            state_count = state_count.div_ceil(self.arity);
        }
        if folded_level_count > path_level_count {
            return Err(MerkleHashError::LengthOverflow);
        }

        let sibling_words = match self.arity {
            2 => cuda_poseidon2_width8_merkle_digest_opening_prefix_device(
                &self.digests,
                source_row,
                folded_level_count,
            ),
            4 => cuda_poseidon2_width16_merkle_digest_opening_prefix_device(
                &self.digests,
                source_row,
                folded_level_count,
            ),
            _ => return Err(MerkleHashError::UnsupportedArity { arity: self.arity }),
        }
        .map_err(MerkleHashError::Accel)?;
        let expected_words = folded_level_count
            .checked_mul(self.arity.saturating_sub(1))
            .and_then(|count| count.checked_mul(HASH_WORDS))
            .ok_or(MerkleHashError::LengthOverflow)?;
        if sibling_words.len() != expected_words {
            return Err(MerkleHashError::LengthOverflow);
        }

        let mut cursor = 0usize;
        let mut siblings = Vec::with_capacity(folded_level_count);
        for _ in 0..folded_level_count {
            let mut level_siblings = Vec::with_capacity(self.arity - 1);
            for _ in 0..self.arity - 1 {
                let end = cursor
                    .checked_add(HASH_WORDS)
                    .ok_or(MerkleHashError::LengthOverflow)?;
                let words = sibling_words
                    .get(cursor..end)
                    .ok_or(MerkleHashError::LengthOverflow)?;
                level_siblings.push(digest_from_state_words(words)?);
                cursor = end;
            }
            siblings.push(level_siblings);
        }
        Ok(siblings)
    }

    pub(crate) fn opening_path_prefix_batch_device_for_source_rows(
        &self,
        source_rows: &[usize],
        folded_level_count: usize,
    ) -> Result<CudaMerkleSiblingBatchDeviceBuffer, MerkleHashError> {
        if source_rows
            .iter()
            .any(|source_row| *source_row >= self.state_count)
        {
            return Err(MerkleHashError::LengthOverflow);
        }

        let path_level_count = merkle_opening_level_count(self.state_count, self.arity)?;
        if folded_level_count > path_level_count {
            return Err(MerkleHashError::LengthOverflow);
        }

        let sibling_buffer = match self.arity {
            2 => cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_buffer(
                &self.digests,
                source_rows,
                folded_level_count,
            ),
            4 => cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_buffer(
                &self.digests,
                source_rows,
                folded_level_count,
            ),
            _ => return Err(MerkleHashError::UnsupportedArity { arity: self.arity }),
        }
        .map_err(MerkleHashError::Accel)?;
        CudaMerkleSiblingBatchDeviceBuffer::new(
            sibling_buffer,
            source_rows.len(),
            folded_level_count,
            self.arity,
        )
    }

    pub(crate) fn opening_path_prefix_batch_for_source_rows(
        &self,
        source_rows: &[usize],
        folded_level_count: usize,
    ) -> Result<Vec<Vec<Vec<[Felt; HASH_WORDS]>>>, MerkleHashError> {
        self.opening_path_prefix_batch_device_for_source_rows(source_rows, folded_level_count)?
            .into_siblings()
    }
}

#[cfg(feature = "cuda")]
#[cfg_attr(not(test), allow(dead_code))]
impl CudaDigestCheckpointLevel {
    pub(crate) fn source_state_count(&self) -> usize {
        self.source_state_count
    }

    pub(crate) fn folded_level_count(&self) -> usize {
        self.folded_level_count
    }

    pub(crate) fn state_count(&self) -> usize {
        self.level.state_count()
    }

    pub(crate) fn arity(&self) -> usize {
        self.level.arity()
    }

    pub(crate) fn byte_len(&self) -> usize {
        self.level.byte_len()
    }

    pub(crate) fn to_digests(&self) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
        self.level.to_digests()
    }

    pub(crate) fn root(&self) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
        self.level.root()
    }

    pub(crate) fn root_device(&self) -> Result<CudaDigestRoot, MerkleHashError> {
        self.level.root_device()
    }

    #[allow(dead_code)]
    pub(crate) fn opening_path_for_source_row(
        &self,
        source_row: usize,
    ) -> Result<CudaMerkleOpeningPath, MerkleHashError> {
        if source_row >= self.source_state_count {
            return Err(MerkleHashError::LengthOverflow);
        }
        let leaf_span = self.source_leaf_span()?;
        let checkpoint_row = source_row / leaf_span;
        self.level.opening_path(checkpoint_row)
    }

    pub(crate) fn opening_path_siblings_for_source_row(
        &self,
        source_row: usize,
    ) -> Result<Vec<Vec<[Felt; HASH_WORDS]>>, MerkleHashError> {
        if source_row >= self.source_state_count {
            return Err(MerkleHashError::LengthOverflow);
        }
        let leaf_span = self.source_leaf_span()?;
        let checkpoint_row = source_row / leaf_span;
        self.level.opening_path_siblings(checkpoint_row)
    }

    pub(crate) fn opening_path_siblings_batch_for_source_rows(
        &self,
        source_rows: &[usize],
    ) -> Result<Vec<Vec<Vec<[Felt; HASH_WORDS]>>>, MerkleHashError> {
        if source_rows
            .iter()
            .any(|source_row| *source_row >= self.source_state_count)
        {
            return Err(MerkleHashError::LengthOverflow);
        }
        let leaf_span = self.source_leaf_span()?;
        let checkpoint_rows = source_rows
            .iter()
            .map(|source_row| source_row / leaf_span)
            .collect::<Vec<_>>();
        self.level.opening_path_siblings_batch(&checkpoint_rows)
    }

    pub(crate) fn opening_path_siblings_batch_device_for_source_rows(
        &self,
        source_rows: &[usize],
    ) -> Result<CudaMerkleSiblingBatchDeviceBuffer, MerkleHashError> {
        if source_rows
            .iter()
            .any(|source_row| *source_row >= self.source_state_count)
        {
            return Err(MerkleHashError::LengthOverflow);
        }
        let leaf_span = self.source_leaf_span()?;
        let checkpoint_rows = source_rows
            .iter()
            .map(|source_row| source_row / leaf_span)
            .collect::<Vec<_>>();
        let level_count = merkle_opening_level_count(self.state_count(), self.arity())?;
        self.level
            .opening_path_prefix_batch_device_for_source_rows(&checkpoint_rows, level_count)
    }

    fn source_leaf_span(&self) -> Result<usize, MerkleHashError> {
        let mut leaf_span = 1usize;
        for _ in 0..self.folded_level_count {
            leaf_span = leaf_span
                .checked_mul(self.arity())
                .ok_or(MerkleHashError::LengthOverflow)?;
        }
        Ok(leaf_span)
    }
}

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

pub(crate) fn linear_hashes(
    rows: &[Vec<Felt>],
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    validate_arity(arity)?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    #[cfg(feature = "cuda")]
    {
        if rows.iter().all(|row| row.len() == rows[0].len()) {
            return cuda_linear_hashes(rows, arity);
        }
    }

    rows.iter().map(|row| linear_hash(row, arity)).collect()
}

pub(crate) fn linear_hashes_from_row_major_bytes(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    validate_arity(arity)?;
    let expected = row_major_byte_count(row_count, column_count)?;
    if bytes.len() != expected {
        return Err(MerkleHashError::LengthOverflow);
    }
    if row_count == 0 {
        return Ok(Vec::new());
    }
    if column_count <= HASH_WORDS {
        return padded_digests_from_row_major_bytes(bytes, row_count, column_count);
    }

    #[cfg(feature = "cuda")]
    {
        cuda_linear_hashes_from_row_major_bytes(bytes, row_count, column_count, arity)
    }

    #[cfg(not(feature = "cuda"))]
    {
        linear_hashes_from_row_major_bytes_on_host(bytes, row_count, column_count, arity)
    }
}

pub(crate) fn linear_hashes_from_row_major_bytes_on_host(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    validate_arity(arity)?;
    let expected = row_major_byte_count(row_count, column_count)?;
    if bytes.len() != expected {
        return Err(MerkleHashError::LengthOverflow);
    }
    if row_count == 0 {
        return Ok(Vec::new());
    }
    if column_count <= HASH_WORDS {
        return padded_digests_from_row_major_bytes(bytes, row_count, column_count);
    }

    cpu_linear_hashes_from_row_major_bytes(bytes, row_count, column_count, arity)
}

#[cfg(feature = "cuda")]
pub(crate) fn linear_hashes_from_validated_wide_row_major_device_buffer(
    row_values: &CudaDeviceBuffer,
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    validate_arity(arity)?;
    let expected = row_major_byte_count(row_count, column_count)?;
    if row_values.len() != expected {
        return Err(MerkleHashError::LengthOverflow);
    }
    if row_count == 0 {
        return Ok(Vec::new());
    }
    if column_count <= HASH_WORDS {
        return Err(MerkleHashError::LengthOverflow);
    }

    linear_hash_level_from_validated_wide_row_major_device_buffer(
        row_values,
        row_count,
        column_count,
        arity,
    )?
    .to_digests()
}

#[cfg(feature = "cuda")]
pub(crate) fn linear_hash_level_from_validated_row_major_device_buffer(
    row_values: &CudaDeviceBuffer,
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<CudaDigestLevel, MerkleHashError> {
    validate_arity(arity)?;
    let expected = row_major_byte_count(row_count, column_count)?;
    if row_values.len() != expected {
        return Err(MerkleHashError::LengthOverflow);
    }
    if row_count == 0 {
        return Err(MerkleHashError::LengthOverflow);
    }

    match arity {
        2 => {
            let digests = if column_count <= HASH_WORDS {
                CudaDeviceBuffer::from_device_state_prefix_u64_words(
                    row_values,
                    row_count,
                    HASH_WORDS,
                    column_count,
                )
                .map_err(MerkleHashError::Accel)?
            } else {
                let states = cuda_linear_hash_states_with_row_major_device_rounds(
                    row_count,
                    column_count,
                    HASH_WORDS,
                    8,
                    cuda_poseidon2_width8_linear_round_row_major_digest_device,
                    row_values,
                )?;
                compact_digest_buffer_from_state_buffer(&states, row_count, 8)?
            };
            Ok(CudaDigestLevel::new(
                digests,
                row_count,
                arity,
                cuda_poseidon2_width8_merkle_digest_root_device,
            ))
        }
        4 => {
            let digests = if column_count <= HASH_WORDS {
                CudaDeviceBuffer::from_device_state_prefix_u64_words(
                    row_values,
                    row_count,
                    HASH_WORDS,
                    column_count,
                )
                .map_err(MerkleHashError::Accel)?
            } else {
                let states = cuda_linear_hash_states_with_row_major_device_rounds(
                    row_count,
                    column_count,
                    12,
                    16,
                    cuda_poseidon2_width16_linear_round_row_major_digest_device,
                    row_values,
                )?;
                compact_digest_buffer_from_state_buffer(&states, row_count, 16)?
            };
            Ok(CudaDigestLevel::new(
                digests,
                row_count,
                arity,
                cuda_poseidon2_width16_merkle_digest_root_device,
            ))
        }
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn linear_hash_level_from_validated_column_major_device_buffer(
    column_values: &CudaDeviceBuffer,
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<CudaDigestLevel, MerkleHashError> {
    validate_arity(arity)?;
    let expected = row_major_byte_count(row_count, column_count)?;
    if column_values.len() != expected || row_count == 0 || column_count <= HASH_WORDS {
        return Err(MerkleHashError::LengthOverflow);
    }

    match arity {
        2 => {
            let states = cuda_linear_hash_states_with_row_major_device_rounds(
                row_count,
                column_count,
                HASH_WORDS,
                8,
                cuda_poseidon2_width8_linear_round_column_major_digest_device,
                column_values,
            )?;
            let digests = compact_digest_buffer_from_state_buffer(&states, row_count, 8)?;
            Ok(CudaDigestLevel::new(
                digests,
                row_count,
                arity,
                cuda_poseidon2_width8_merkle_digest_root_device,
            ))
        }
        4 => {
            let states = cuda_linear_hash_states_with_row_major_device_rounds(
                row_count,
                column_count,
                12,
                16,
                cuda_poseidon2_width16_linear_round_column_major_digest_device,
                column_values,
            )?;
            let digests = compact_digest_buffer_from_state_buffer(&states, row_count, 16)?;
            Ok(CudaDigestLevel::new(
                digests,
                row_count,
                arity,
                cuda_poseidon2_width16_merkle_digest_root_device,
            ))
        }
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn linear_hash_level_from_validated_row_major_device_buffer_on_stream(
    row_values: &CudaDeviceBuffer,
    row_count: usize,
    column_count: usize,
    arity: usize,
    stream: &CudaStream,
) -> Result<CudaDigestLevel, MerkleHashError> {
    validate_arity(arity)?;
    let expected = row_major_byte_count(row_count, column_count)?;
    if row_values.len() != expected {
        return Err(MerkleHashError::LengthOverflow);
    }
    if row_count == 0 {
        return Err(MerkleHashError::LengthOverflow);
    }

    match arity {
        2 => {
            let digests = if column_count <= HASH_WORDS {
                unsafe {
                    CudaDeviceBuffer::from_device_state_prefix_u64_words_on_stream(
                        row_values,
                        row_count,
                        HASH_WORDS,
                        column_count,
                        stream,
                    )
                }
                .map_err(MerkleHashError::Accel)?
            } else {
                let states = cuda_linear_hash_states_with_row_major_device_rounds_on_stream(
                    row_count,
                    column_count,
                    HASH_WORDS,
                    8,
                    cuda_poseidon2_begin_width8_linear_round_row_major_digest_device_on_stream,
                    row_values,
                    stream,
                )?;
                compact_digest_buffer_from_state_buffer_on_stream(&states, row_count, 8, stream)?
            };
            Ok(CudaDigestLevel::new(
                digests,
                row_count,
                arity,
                cuda_poseidon2_width8_merkle_digest_root_device,
            ))
        }
        4 => {
            let digests = if column_count <= HASH_WORDS {
                unsafe {
                    CudaDeviceBuffer::from_device_state_prefix_u64_words_on_stream(
                        row_values,
                        row_count,
                        HASH_WORDS,
                        column_count,
                        stream,
                    )
                }
                .map_err(MerkleHashError::Accel)?
            } else {
                let states = cuda_linear_hash_states_with_row_major_device_rounds_on_stream(
                    row_count,
                    column_count,
                    12,
                    16,
                    cuda_poseidon2_begin_width16_linear_round_row_major_digest_device_on_stream,
                    row_values,
                    stream,
                )?;
                compact_digest_buffer_from_state_buffer_on_stream(&states, row_count, 16, stream)?
            };
            Ok(CudaDigestLevel::new(
                digests,
                row_count,
                arity,
                cuda_poseidon2_width16_merkle_digest_root_device,
            ))
        }
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn linear_hash_level_from_validated_wide_row_major_device_buffer(
    row_values: &CudaDeviceBuffer,
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<CudaDigestLevel, MerkleHashError> {
    if column_count <= HASH_WORDS {
        return Err(MerkleHashError::LengthOverflow);
    }
    linear_hash_level_from_validated_row_major_device_buffer(
        row_values,
        row_count,
        column_count,
        arity,
    )
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

pub(crate) fn parent_hashes(
    children: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    validate_arity(arity)?;
    if children.is_empty() {
        return Ok(Vec::new());
    }
    if !children.len().is_multiple_of(arity) {
        return Err(MerkleHashError::InvalidChildCount {
            expected: arity,
            found: children.len(),
        });
    }

    #[cfg(feature = "cuda")]
    {
        match arity {
            2 => parent_hashes_arity2_cuda(children),
            4 => parent_hashes_arity4_cuda(children),
            _ => unreachable!("arity is validated"),
        }
    }

    #[cfg(not(feature = "cuda"))]
    {
        children
            .chunks_exact(arity)
            .map(|chunk| parent_hash(chunk, arity))
            .collect()
    }
}

pub(crate) fn parent_levels_from_digest_level(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<Vec<MerkleParentLevel>, MerkleHashError> {
    validate_arity(arity)?;
    if level.is_empty() {
        return Ok(Vec::new());
    }

    #[cfg(feature = "cuda")]
    {
        parent_levels_from_digest_level_on_cuda(level, arity)
    }

    #[cfg(not(feature = "cuda"))]
    {
        parent_levels_from_digest_level_on_cpu(level, arity)
    }
}

pub(crate) fn parent_levels_from_digest_level_on_host(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<Vec<MerkleParentLevel>, MerkleHashError> {
    validate_arity(arity)?;
    if level.is_empty() {
        return Ok(Vec::new());
    }
    parent_levels_from_digest_level_on_cpu(level, arity)
}

fn parent_levels_from_digest_level_on_cpu(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<Vec<MerkleParentLevel>, MerkleHashError> {
    let mut current = level.to_vec();
    let mut levels = Vec::new();
    while current.len() > 1 {
        let padding_count = padding_count(current.len(), arity)?;
        let padded_child_count = current
            .len()
            .checked_add(padding_count)
            .ok_or(MerkleHashError::LengthOverflow)?;
        current.resize(padded_child_count, [Felt::ZERO; HASH_WORDS]);

        let parents = current
            .chunks_exact(arity)
            .map(|chunk| parent_hash(chunk, arity))
            .collect::<Result<Vec<_>, _>>()?;
        levels.push(MerkleParentLevel {
            padding_count,
            parents: parents.clone(),
        });
        current = parents;
    }
    Ok(levels)
}

#[cfg(feature = "cuda")]
fn parent_levels_from_digest_level_on_cuda(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<Vec<MerkleParentLevel>, MerkleHashError> {
    let root_operation: CudaPoseidon2RootOp = match arity {
        2 => cuda_poseidon2_width8_merkle_digest_root_device,
        4 => cuda_poseidon2_width16_merkle_digest_root_device,
        _ => unreachable!("arity is validated"),
    };

    let input_words = digest_level_as_words(level)?;
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input_words).map_err(MerkleHashError::Accel)?;
    let mut current = CudaDigestLevel::new(input_buffer, level.len(), arity, root_operation);
    let mut levels = Vec::new();
    while current.state_count() > 1 {
        let padding_count = padding_count(current.state_count(), arity)?;
        let next = current.parent_level()?;
        let parents = next.to_digests()?;
        levels.push(MerkleParentLevel {
            padding_count,
            parents,
        });
        current = next;
    }
    Ok(levels)
}

pub(crate) fn root_from_digest_level(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
    validate_arity(arity)?;
    if level.is_empty() {
        return Ok([Felt::ZERO; HASH_WORDS]);
    }
    if level.len() == 1 {
        return Ok(level[0]);
    }

    #[cfg(feature = "cuda")]
    {
        root_from_digest_level_on_cuda(level, arity)
    }

    #[cfg(not(feature = "cuda"))]
    {
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

            level = parent_hashes(&level, arity)?;
        }
        Ok(level[0])
    }
}

fn padding_count(count: usize, arity: usize) -> Result<usize, MerkleHashError> {
    let extra_zeros = (arity - (count % arity)) % arity;
    count
        .checked_add(extra_zeros)
        .ok_or(MerkleHashError::LengthOverflow)?;
    Ok(extra_zeros)
}

#[cfg(feature = "cuda")]
fn root_from_digest_level_on_cuda(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
    validate_arity(arity)?;
    if level.is_empty() {
        return Ok([Felt::ZERO; HASH_WORDS]);
    }
    if level.len() == 1 {
        return Ok(level[0]);
    }

    let input_words = digest_level_as_words(level)?;
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input_words).map_err(MerkleHashError::Accel)?;
    let root_words = match arity {
        2 => cuda_poseidon2_width8_merkle_digest_root_device(&input_buffer),
        4 => cuda_poseidon2_width16_merkle_digest_root_device(&input_buffer),
        _ => unreachable!("arity is validated"),
    }
    .map_err(MerkleHashError::Accel)?;
    digest_from_state_words(&root_words)
}

#[cfg(feature = "cuda")]
fn digest_level_as_words(level: &[[Felt; HASH_WORDS]]) -> Result<Vec<u64>, MerkleHashError> {
    let mut words = Vec::with_capacity(
        level
            .len()
            .checked_mul(HASH_WORDS)
            .ok_or(MerkleHashError::LengthOverflow)?,
    );
    for digest in level {
        for value in digest {
            words.push(value.to_u64());
        }
    }
    Ok(words)
}

#[cfg(feature = "cuda")]
fn digest_level_as_state_words(
    level: &[[Felt; HASH_WORDS]],
    width: usize,
) -> Result<Vec<u64>, MerkleHashError> {
    let mut words = vec![
        0_u64;
        level
            .len()
            .checked_mul(width)
            .ok_or(MerkleHashError::LengthOverflow)?
    ];
    for (index, digest) in level.iter().enumerate() {
        let offset = index
            .checked_mul(width)
            .ok_or(MerkleHashError::LengthOverflow)?;
        for (word_index, value) in digest.iter().enumerate() {
            words[offset + word_index] = value.to_u64();
        }
    }
    Ok(words)
}

#[cfg(feature = "cuda")]
fn compact_digest_buffer_from_state_buffer(
    states: &CudaDeviceBuffer,
    row_count: usize,
    state_width: usize,
) -> Result<CudaDeviceBuffer, MerkleHashError> {
    let mut digests = CudaDeviceBuffer::new(
        row_count
            .checked_mul(HASH_WORDS)
            .and_then(|word_count| word_count.checked_mul(8))
            .ok_or(MerkleHashError::LengthOverflow)?,
    )
    .map_err(MerkleHashError::Accel)?;
    digests
        .copy_from_device_row_major_u64_slice(states, row_count, state_width, 0, HASH_WORDS)
        .map_err(MerkleHashError::Accel)?;
    Ok(digests)
}

#[cfg(feature = "cuda")]
fn compact_digest_buffer_from_state_buffer_on_stream(
    states: &CudaDeviceBuffer,
    row_count: usize,
    state_width: usize,
    stream: &CudaStream,
) -> Result<CudaDeviceBuffer, MerkleHashError> {
    unsafe {
        CudaDeviceBuffer::from_device_row_major_u64_slice_on_stream(
            states,
            row_count,
            state_width,
            0,
            HASH_WORDS,
            stream,
        )
    }
    .map_err(MerkleHashError::Accel)
}

fn validate_arity(arity: usize) -> Result<(), MerkleHashError> {
    match arity {
        2 | 4 => Ok(()),
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

fn row_major_byte_count(row_count: usize, column_count: usize) -> Result<usize, MerkleHashError> {
    row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(8))
        .ok_or(MerkleHashError::LengthOverflow)
}

fn padded_digests_from_row_major_bytes(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    let mut out = Vec::with_capacity(row_count);
    for row in 0..row_count {
        let mut digest = [Felt::ZERO; HASH_WORDS];
        for (column, slot) in digest.iter_mut().enumerate().take(column_count) {
            *slot = read_row_major_felt(bytes, column_count, row, column)?;
        }
        out.push(digest);
    }
    Ok(out)
}

fn cpu_linear_hashes_from_row_major_bytes(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    let mut out = Vec::with_capacity(row_count);
    for row in 0..row_count {
        let mut values = Vec::with_capacity(column_count);
        for column in 0..column_count {
            values.push(read_row_major_felt(bytes, column_count, row, column)?);
        }
        out.push(linear_hash(&values, arity)?);
    }
    Ok(out)
}

fn read_row_major_felt(
    bytes: &[u8],
    column_count: usize,
    row: usize,
    column: usize,
) -> Result<Felt, MerkleHashError> {
    let word_index = row
        .checked_mul(column_count)
        .and_then(|offset| offset.checked_add(column))
        .ok_or(MerkleHashError::LengthOverflow)?;
    let byte_index = word_index
        .checked_mul(8)
        .ok_or(MerkleHashError::LengthOverflow)?;
    let word = u64::from_le_bytes(
        bytes[byte_index..byte_index + 8]
            .try_into()
            .expect("row-major byte length checked"),
    );
    Felt::from_canonical(word).map_err(MerkleHashError::Field)
}

#[cfg(feature = "cuda")]
fn validate_row_major_bytes(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
) -> Result<(), MerkleHashError> {
    let expected = row_major_byte_count(row_count, column_count)?;
    if bytes.len() != expected {
        return Err(MerkleHashError::LengthOverflow);
    }
    for chunk in bytes.chunks_exact(8) {
        let word = u64::from_le_bytes(chunk.try_into().expect("chunks have one word"));
        Felt::from_canonical(word).map_err(MerkleHashError::Field)?;
    }
    Ok(())
}

#[cfg(feature = "cuda")]
fn copy_row_major_bytes_to_device(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
) -> Result<CudaDeviceBuffer, MerkleHashError> {
    validate_row_major_bytes(bytes, row_count, column_count)?;
    let mut buffer = CudaDeviceBuffer::new(bytes.len()).map_err(MerkleHashError::Accel)?;
    buffer.copy_from(bytes).map_err(MerkleHashError::Accel)?;
    Ok(buffer)
}

fn linear_hash_arity2(values: &[Felt]) -> [Felt; HASH_WORDS] {
    if values.len() <= HASH_WORDS {
        return padded_digest(values);
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
        return padded_digest(values);
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

fn padded_digest(values: &[Felt]) -> [Felt; HASH_WORDS] {
    let mut digest = [Felt::ZERO; HASH_WORDS];
    digest[..values.len()].copy_from_slice(values);
    digest
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes(
    rows: &[Vec<Felt>],
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    match arity {
        2 => cuda_linear_hashes_arity2(rows),
        4 => cuda_linear_hashes_arity4(rows),
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_arity2(
    rows: &[Vec<Felt>],
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    const WIDTH: usize = 8;

    let value_count = rows.first().map_or(0, Vec::len);
    if value_count <= HASH_WORDS {
        return Ok(rows.iter().map(|row| padded_digest(row)).collect());
    }

    cuda_linear_hashes_with_packed_rounds(
        rows.len(),
        value_count,
        HASH_WORDS,
        WIDTH,
        cuda_poseidon2_width8_linear_round_device,
        |offset, chunk_len| pack_linear_round_values(rows, offset, chunk_len),
    )
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_arity4(
    rows: &[Vec<Felt>],
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    const RATE: usize = 12;
    const WIDTH: usize = 16;

    let value_count = rows.first().map_or(0, Vec::len);
    if value_count <= HASH_WORDS {
        return Ok(rows.iter().map(|row| padded_digest(row)).collect());
    }

    cuda_linear_hashes_with_packed_rounds(
        rows.len(),
        value_count,
        RATE,
        WIDTH,
        cuda_poseidon2_width16_linear_round_device,
        |offset, chunk_len| pack_linear_round_values(rows, offset, chunk_len),
    )
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_from_row_major_bytes(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    match arity {
        2 => cuda_linear_hashes_row_major_arity2(bytes, row_count, column_count),
        4 => cuda_linear_hashes_row_major_arity4(bytes, row_count, column_count),
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_row_major_arity2(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    const WIDTH: usize = 8;

    let row_values_buffer = copy_row_major_bytes_to_device(bytes, row_count, column_count)?;
    cuda_linear_hashes_with_row_major_device_rounds(
        row_count,
        column_count,
        HASH_WORDS,
        WIDTH,
        cuda_poseidon2_width8_linear_round_row_major_digest_device,
        &row_values_buffer,
    )
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_row_major_arity4(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    const RATE: usize = 12;
    const WIDTH: usize = 16;

    let row_values_buffer = copy_row_major_bytes_to_device(bytes, row_count, column_count)?;
    cuda_linear_hashes_with_row_major_device_rounds(
        row_count,
        column_count,
        RATE,
        WIDTH,
        cuda_poseidon2_width16_linear_round_row_major_digest_device,
        &row_values_buffer,
    )
}

#[cfg(feature = "cuda")]
type CudaPoseidon2LinearRoundOp = fn(
    &CudaDeviceBuffer,
    &CudaDeviceBuffer,
    &mut CudaDeviceBuffer,
    usize,
) -> Result<(), lzvm_accel::AccelError>;

#[cfg(feature = "cuda")]
type CudaPoseidon2LinearRoundRowMajorOp = fn(
    &CudaDeviceBuffer,
    &CudaDeviceBuffer,
    &mut CudaDeviceBuffer,
    usize,
    usize,
    usize,
) -> Result<(), lzvm_accel::AccelError>;

#[cfg(feature = "cuda")]
type CudaPoseidon2BeginLinearRoundRowMajorOp = unsafe fn(
    &CudaDeviceBuffer,
    &CudaDeviceBuffer,
    &mut CudaDeviceBuffer,
    usize,
    usize,
    usize,
    &CudaStream,
) -> Result<(), lzvm_accel::AccelError>;

#[cfg(feature = "cuda")]
type CudaPoseidon2RootOp =
    fn(&CudaDeviceBuffer) -> Result<[u64; HASH_WORDS], lzvm_accel::AccelError>;

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_with_packed_rounds(
    row_count: usize,
    value_count: usize,
    rate: usize,
    width: usize,
    operation: CudaPoseidon2LinearRoundOp,
    mut pack_round: impl FnMut(usize, usize) -> Result<Vec<u64>, MerkleHashError>,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    let mut current_states = zero_state_buffer(row_count, width)?;
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(rate);
        let row_values = pack_round(offset, chunk_len)?;
        let row_values_buffer =
            CudaDeviceBuffer::from_u64_words(&row_values).map_err(MerkleHashError::Accel)?;
        let mut next_states = CudaDeviceBuffer::new(
            row_count
                .checked_mul(width)
                .and_then(|words| words.checked_mul(8))
                .ok_or(MerkleHashError::LengthOverflow)?,
        )
        .map_err(MerkleHashError::Accel)?;
        operation(
            &current_states,
            &row_values_buffer,
            &mut next_states,
            chunk_len,
        )
        .map_err(MerkleHashError::Accel)?;
        current_states = next_states;
        offset += chunk_len;
    }

    let output = current_states
        .to_state_prefix_u64_words(row_count, width, HASH_WORDS)
        .map_err(MerkleHashError::Accel)?;
    digests_from_hashed_states(&output, HASH_WORDS)
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_with_row_major_device_rounds(
    row_count: usize,
    value_count: usize,
    rate: usize,
    width: usize,
    operation: CudaPoseidon2LinearRoundRowMajorOp,
    row_values: &CudaDeviceBuffer,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    let current_states = cuda_linear_hash_states_with_row_major_device_rounds(
        row_count,
        value_count,
        rate,
        width,
        operation,
        row_values,
    )?;

    let output = current_states
        .to_state_prefix_u64_words(row_count, width, HASH_WORDS)
        .map_err(MerkleHashError::Accel)?;
    digests_from_hashed_states(&output, HASH_WORDS)
}

#[cfg(feature = "cuda")]
fn cuda_linear_hash_states_with_row_major_device_rounds(
    row_count: usize,
    value_count: usize,
    rate: usize,
    width: usize,
    operation: CudaPoseidon2LinearRoundRowMajorOp,
    row_values: &CudaDeviceBuffer,
) -> Result<CudaDeviceBuffer, MerkleHashError> {
    let mut current_states = zero_state_buffer(row_count, width)?;
    let state_byte_count = row_count
        .checked_mul(width)
        .and_then(|words| words.checked_mul(8))
        .ok_or(MerkleHashError::LengthOverflow)?;
    let mut next_states =
        CudaDeviceBuffer::new(state_byte_count).map_err(MerkleHashError::Accel)?;
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(rate);
        operation(
            &current_states,
            row_values,
            &mut next_states,
            value_count,
            offset,
            chunk_len,
        )
        .map_err(MerkleHashError::Accel)?;
        std::mem::swap(&mut current_states, &mut next_states);
        offset += chunk_len;
    }

    Ok(current_states)
}

#[cfg(feature = "cuda")]
fn cuda_linear_hash_states_with_row_major_device_rounds_on_stream(
    row_count: usize,
    value_count: usize,
    rate: usize,
    width: usize,
    operation: CudaPoseidon2BeginLinearRoundRowMajorOp,
    row_values: &CudaDeviceBuffer,
    stream: &CudaStream,
) -> Result<CudaDeviceBuffer, MerkleHashError> {
    let mut current_states = zero_state_buffer_on_stream(row_count, width, stream)?;
    let state_byte_count = row_count
        .checked_mul(width)
        .and_then(|words| words.checked_mul(8))
        .ok_or(MerkleHashError::LengthOverflow)?;
    let mut next_states =
        CudaDeviceBuffer::new(state_byte_count).map_err(MerkleHashError::Accel)?;
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(rate);
        unsafe {
            operation(
                &current_states,
                row_values,
                &mut next_states,
                value_count,
                offset,
                chunk_len,
                stream,
            )
        }
        .map_err(MerkleHashError::Accel)?;
        std::mem::swap(&mut current_states, &mut next_states);
        offset += chunk_len;
    }

    Ok(current_states)
}

#[cfg(feature = "cuda")]
fn push_felt_words(out: &mut Vec<u64>, values: &[Felt]) {
    out.extend(values.iter().map(|value| value.to_u64()));
}

#[cfg(feature = "cuda")]
fn pack_linear_round_values(
    rows: &[Vec<Felt>],
    offset: usize,
    chunk_len: usize,
) -> Result<Vec<u64>, MerkleHashError> {
    let mut input = Vec::with_capacity(
        rows.len()
            .checked_mul(chunk_len)
            .ok_or(MerkleHashError::LengthOverflow)?,
    );
    for row in rows {
        push_felt_words(&mut input, &row[offset..offset + chunk_len]);
    }
    Ok(input)
}

#[cfg(feature = "cuda")]
fn zero_state_buffer(row_count: usize, width: usize) -> Result<CudaDeviceBuffer, MerkleHashError> {
    let words = row_count
        .checked_mul(width)
        .ok_or(MerkleHashError::LengthOverflow)?;
    if words == 0 {
        return CudaDeviceBuffer::new(0).map_err(MerkleHashError::Accel);
    }
    CudaDeviceBuffer::zeroed(
        words
            .checked_mul(8)
            .ok_or(MerkleHashError::LengthOverflow)?,
    )
    .map_err(MerkleHashError::Accel)
}

#[cfg(feature = "cuda")]
fn zero_state_buffer_on_stream(
    row_count: usize,
    width: usize,
    stream: &CudaStream,
) -> Result<CudaDeviceBuffer, MerkleHashError> {
    let words = row_count
        .checked_mul(width)
        .ok_or(MerkleHashError::LengthOverflow)?;
    let byte_count = words
        .checked_mul(8)
        .ok_or(MerkleHashError::LengthOverflow)?;
    if byte_count == 0 {
        return CudaDeviceBuffer::new(0).map_err(MerkleHashError::Accel);
    }
    unsafe { CudaDeviceBuffer::zeroed_on_stream(byte_count, stream) }
        .map_err(MerkleHashError::Accel)
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

#[cfg(feature = "cuda")]
fn parent_hashes_arity2_cuda(
    children: &[[Felt; HASH_WORDS]],
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    cuda_parent_hashes_on_device(children, 2, 8, cuda_poseidon2_width8_merkle_parent_device)
}

#[cfg(feature = "cuda")]
fn parent_hashes_arity4_cuda(
    children: &[[Felt; HASH_WORDS]],
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    cuda_parent_hashes_on_device(children, 4, 16, cuda_poseidon2_width16_merkle_parent_device)
}

#[cfg(feature = "cuda")]
type CudaPoseidon2DeviceOp =
    fn(&CudaDeviceBuffer, &mut CudaDeviceBuffer) -> Result<(), lzvm_accel::AccelError>;

#[cfg(all(test, feature = "cuda"))]
fn cuda_poseidon2_width8_device_words(words: &[u64]) -> Result<Vec<u64>, MerkleHashError> {
    cuda_poseidon2_words_device(words, cuda_poseidon2_width8_device)
}

#[cfg(all(test, feature = "cuda"))]
fn cuda_poseidon2_width16_device_words(words: &[u64]) -> Result<Vec<u64>, MerkleHashError> {
    cuda_poseidon2_words_device(words, cuda_poseidon2_width16_device)
}

#[cfg(feature = "cuda")]
fn cuda_parent_hashes_on_device(
    children: &[[Felt; HASH_WORDS]],
    arity: usize,
    width: usize,
    operation: CudaPoseidon2DeviceOp,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    if children.is_empty() {
        return Ok(Vec::new());
    }

    let input_words = digest_level_as_state_words(children, width)?;
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input_words).map_err(MerkleHashError::Accel)?;
    let parent_count = children.len().div_ceil(arity);
    let mut output_buffer = CudaDeviceBuffer::new(
        parent_count
            .checked_mul(width)
            .and_then(|word_count| word_count.checked_mul(8))
            .ok_or(MerkleHashError::LengthOverflow)?,
    )
    .map_err(MerkleHashError::Accel)?;
    operation(&input_buffer, &mut output_buffer).map_err(MerkleHashError::Accel)?;
    let hashed = output_buffer
        .to_u64_words()
        .map_err(MerkleHashError::Accel)?;
    digests_from_hashed_states(&hashed, width)
}

#[cfg(all(test, feature = "cuda"))]
fn cuda_poseidon2_words_device(
    words: &[u64],
    operation: CudaPoseidon2DeviceOp,
) -> Result<Vec<u64>, MerkleHashError> {
    if words.is_empty() {
        return Ok(Vec::new());
    }

    let input_buffer = CudaDeviceBuffer::from_u64_words(words).map_err(MerkleHashError::Accel)?;
    let mut output_buffer = CudaDeviceBuffer::new(
        words
            .len()
            .checked_mul(8)
            .ok_or(MerkleHashError::LengthOverflow)?,
    )
    .map_err(MerkleHashError::Accel)?;
    operation(&input_buffer, &mut output_buffer).map_err(MerkleHashError::Accel)?;
    output_buffer.to_u64_words().map_err(MerkleHashError::Accel)
}

#[cfg(feature = "cuda")]
fn digest_from_state_words(words: &[u64]) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
    let mut digest = [Felt::ZERO; HASH_WORDS];
    for (slot, word) in digest.iter_mut().zip(words.iter()) {
        *slot = Felt::from_canonical(*word).map_err(MerkleHashError::Field)?;
    }
    Ok(digest)
}

#[cfg(feature = "cuda")]
fn digests_from_hashed_states(
    states: &[u64],
    state_width: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    let mut out = Vec::with_capacity(states.len() / state_width);
    for state in states.chunks_exact(state_width) {
        out.push(digest_from_state_words(state)?);
    }
    Ok(out)
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::{
        cuda_poseidon2_width16_device_words, cuda_poseidon2_width8_device_words, linear_hash,
        linear_hash_level_from_validated_column_major_device_buffer, linear_hashes,
        linear_hashes_from_row_major_bytes, opening_path_siblings_across_digest_checkpoints,
        parent_hash, parent_hashes, parent_levels_from_digest_level,
        parent_levels_from_digest_level_on_cpu, root_from_digest_level_on_cuda,
        CudaDigestCheckpointOpeningSource, CudaDigestLevel, CudaMerkleSiblingBatchDeviceBuffer,
    };
    use lzvm_accel::{
        cuda_poseidon2_width16_merkle_digest_root_device,
        cuda_poseidon2_width8_merkle_digest_root_device, CudaDeviceBuffer,
    };
    use lzvm_field::{poseidon2_hash_16, poseidon2_hash_8, Felt};

    #[test]
    fn device_poseidon2_state_hashes_match_cpu_reference() {
        let width8_input = (1_u64..=16).collect::<Vec<_>>();
        let width8_expected = width8_input
            .chunks_exact(8)
            .flat_map(|chunk| poseidon2_hash_8(felt_array::<8>(chunk)).map(Felt::to_u64))
            .collect::<Vec<_>>();
        let width8_actual = cuda_poseidon2_width8_device_words(&width8_input)
            .expect("width-8 device hash should run");
        assert_eq!(width8_actual, width8_expected);

        let width16_input = (17_u64..=48).collect::<Vec<_>>();
        let width16_expected = width16_input
            .chunks_exact(16)
            .flat_map(|chunk| poseidon2_hash_16(felt_array::<16>(chunk)).map(Felt::to_u64))
            .collect::<Vec<_>>();
        let width16_actual = cuda_poseidon2_width16_device_words(&width16_input)
            .expect("width-16 device hash should run");
        assert_eq!(width16_actual, width16_expected);
    }

    #[test]
    fn linear_hashes_row_major_bytes_match_row_vectors() {
        let rows = vec![
            (1_u64..=9).map(Felt::from_u64).collect::<Vec<_>>(),
            (21_u64..=29).map(Felt::from_u64).collect::<Vec<_>>(),
            (41_u64..=49).map(Felt::from_u64).collect::<Vec<_>>(),
        ];
        let mut bytes = Vec::new();
        for row in &rows {
            for value in row {
                bytes.extend_from_slice(&value.to_u64().to_le_bytes());
            }
        }

        let direct = linear_hashes_from_row_major_bytes(&bytes, rows.len(), rows[0].len(), 2)
            .expect("arity-2 row-major hashes should compute");
        let expected = linear_hashes(&rows, 2).expect("arity-2 row hashes should compute");

        assert_eq!(direct, expected);

        let rows = vec![
            (1_u64..=13).map(Felt::from_u64).collect::<Vec<_>>(),
            (21_u64..=33).map(Felt::from_u64).collect::<Vec<_>>(),
            (41_u64..=53).map(Felt::from_u64).collect::<Vec<_>>(),
        ];
        let mut bytes = Vec::new();
        for row in &rows {
            for value in row {
                bytes.extend_from_slice(&value.to_u64().to_le_bytes());
            }
        }

        let direct = linear_hashes_from_row_major_bytes(&bytes, rows.len(), rows[0].len(), 4)
            .expect("row-major hashes should compute");
        let expected = linear_hashes(&rows, 4).expect("row hashes should compute");

        assert_eq!(direct, expected);
    }

    #[test]
    fn linear_hashes_row_major_bytes_reject_non_canonical_words() {
        let mut bytes = Vec::new();
        for value in [1_u64, 2, 3, 4, 5, 0xffff_ffff_0000_0001] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        let error = linear_hashes_from_row_major_bytes(&bytes, 1, 6, 2)
            .expect_err("non-canonical row-major word should be rejected");

        assert!(error.to_string().contains("non-canonical field element"));
    }

    #[test]
    fn cuda_linear_hashes_match_cpu_reference() {
        let rows = vec![
            values(&[1, 2, 3, 4, 5, 6]),
            values(&[7, 8, 9, 10, 11, 12]),
            values(&[13, 14, 15, 16, 17, 18]),
        ];

        let actual_arity2 = linear_hashes(&rows, 2).expect("cuda arity-2 leaf hashes should run");
        let expected_arity2 = rows
            .iter()
            .map(|row| linear_hash(row, 2).expect("cpu arity-2 leaf should hash"))
            .collect::<Vec<_>>();
        assert_eq!(actual_arity2, expected_arity2);

        let actual_arity4 = linear_hashes(&rows, 4).expect("cuda arity-4 leaf hashes should run");
        let expected_arity4 = rows
            .iter()
            .map(|row| linear_hash(row, 4).expect("cpu arity-4 leaf should hash"))
            .collect::<Vec<_>>();
        assert_eq!(actual_arity4, expected_arity4);
    }

    #[test]
    fn cuda_linear_hashes_multi_round_match_cpu_reference() {
        let rows = vec![
            values(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
            values(&[16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30]),
            values(&[31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45]),
        ];

        let actual_arity2 = linear_hashes(&rows, 2).expect("cuda arity-2 leaf hashes should run");
        let expected_arity2 = rows
            .iter()
            .map(|row| linear_hash(row, 2).expect("cpu arity-2 leaf should hash"))
            .collect::<Vec<_>>();
        assert_eq!(actual_arity2, expected_arity2);

        let actual_arity4 = linear_hashes(&rows, 4).expect("cuda arity-4 leaf hashes should run");
        let expected_arity4 = rows
            .iter()
            .map(|row| linear_hash(row, 4).expect("cpu arity-4 leaf should hash"))
            .collect::<Vec<_>>();
        assert_eq!(actual_arity4, expected_arity4);
    }

    #[test]
    fn cuda_column_major_linear_hashes_match_cpu_reference() {
        let row_count = 8;
        let column_count = 19;
        let rows = (0..row_count)
            .map(|row| {
                (0..column_count)
                    .map(|column| Felt::from_u64((row * column_count + column + 1) as u64 * 37))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut column_words = vec![0_u64; row_count * column_count];
        for row in 0..row_count {
            for column in 0..column_count {
                column_words[column * row_count + row] = rows[row][column].to_u64();
            }
        }
        let column_values =
            CudaDeviceBuffer::from_u64_words(&column_words).expect("columns should upload");

        for arity in [2, 4] {
            let actual = linear_hash_level_from_validated_column_major_device_buffer(
                &column_values,
                row_count,
                column_count,
                arity,
            )
            .expect("column-major leaf hashes should run")
            .to_digests()
            .expect("column-major digests should download");
            let expected = linear_hashes(&rows, arity).expect("CPU leaf hashes should run");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn cuda_parent_hashes_match_cpu_reference() {
        let children = vec![
            digest([1, 2, 3, 4]),
            digest([5, 6, 7, 8]),
            digest([9, 10, 11, 12]),
            digest([13, 14, 15, 16]),
        ];

        let actual = parent_hashes(&children, 2).expect("cuda parent hashes should run");
        let expected = vec![
            parent_hash(&children[0..2], 2).expect("first parent should hash"),
            parent_hash(&children[2..4], 2).expect("second parent should hash"),
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn cuda_digest_level_selected_parent_matches_cpu_reference() {
        let arity2 = vec![
            digest([1, 2, 3, 4]),
            digest([5, 6, 7, 8]),
            digest([9, 10, 11, 12]),
        ];
        let arity2_words = digest_words(&arity2);
        let arity2_buffer =
            CudaDeviceBuffer::from_u64_words(&arity2_words).expect("arity-2 digests should upload");
        let arity2_level = CudaDigestLevel::new(
            arity2_buffer,
            arity2.len(),
            2,
            cuda_poseidon2_width8_merkle_digest_root_device,
        );
        let arity2_expected =
            parent_hash(&[arity2[2], [Felt::ZERO; 4]], 2).expect("partial parent should hash");

        let arity2_actual = arity2_level
            .selected_parent(1)
            .expect("arity-2 selected parent should hash");

        assert_eq!(arity2_actual, arity2_expected);

        let arity4 = vec![
            digest([11, 12, 13, 14]),
            digest([21, 22, 23, 24]),
            digest([31, 32, 33, 34]),
            digest([41, 42, 43, 44]),
            digest([51, 52, 53, 54]),
            digest([61, 62, 63, 64]),
        ];
        let arity4_words = digest_words(&arity4);
        let arity4_buffer =
            CudaDeviceBuffer::from_u64_words(&arity4_words).expect("arity-4 digests should upload");
        let arity4_level = CudaDigestLevel::new(
            arity4_buffer,
            arity4.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let arity4_expected =
            parent_hash(&[arity4[4], arity4[5], [Felt::ZERO; 4], [Felt::ZERO; 4]], 4)
                .expect("arity-4 partial parent should hash");

        let arity4_actual = arity4_level
            .selected_parent(1)
            .expect("arity-4 selected parent should hash");

        assert_eq!(arity4_actual, arity4_expected);
    }

    #[test]
    fn cuda_digest_level_parent_level_matches_cpu_reference() {
        let arity2 = vec![
            digest([1, 2, 3, 4]),
            digest([5, 6, 7, 8]),
            digest([9, 10, 11, 12]),
        ];
        let arity2_words = digest_words(&arity2);
        let arity2_buffer =
            CudaDeviceBuffer::from_u64_words(&arity2_words).expect("arity-2 digests should upload");
        let arity2_level = CudaDigestLevel::new(
            arity2_buffer,
            arity2.len(),
            2,
            cuda_poseidon2_width8_merkle_digest_root_device,
        );
        let arity2_expected = vec![
            parent_hash(&arity2[0..2], 2).expect("first arity-2 parent should hash"),
            parent_hash(&[arity2[2], [Felt::ZERO; 4]], 2)
                .expect("padded arity-2 parent should hash"),
        ];

        let arity2_parent = arity2_level
            .parent_level()
            .expect("arity-2 parent level should hash");

        assert_eq!(arity2_parent.state_count(), arity2_expected.len());
        assert_eq!(arity2_parent.arity(), 2);
        assert_eq!(arity2_parent.to_digests().unwrap(), arity2_expected);
        assert_eq!(arity2_parent.root().unwrap(), arity2_level.root().unwrap());

        let arity4 = vec![
            digest([11, 12, 13, 14]),
            digest([21, 22, 23, 24]),
            digest([31, 32, 33, 34]),
            digest([41, 42, 43, 44]),
            digest([51, 52, 53, 54]),
            digest([61, 62, 63, 64]),
        ];
        let arity4_words = digest_words(&arity4);
        let arity4_buffer =
            CudaDeviceBuffer::from_u64_words(&arity4_words).expect("arity-4 digests should upload");
        let arity4_level = CudaDigestLevel::new(
            arity4_buffer,
            arity4.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let arity4_expected = vec![
            parent_hash(&arity4[0..4], 4).expect("first arity-4 parent should hash"),
            parent_hash(&[arity4[4], arity4[5], [Felt::ZERO; 4], [Felt::ZERO; 4]], 4)
                .expect("padded arity-4 parent should hash"),
        ];

        let arity4_parent = arity4_level
            .parent_level()
            .expect("arity-4 parent level should hash");

        assert_eq!(arity4_parent.state_count(), arity4_expected.len());
        assert_eq!(arity4_parent.arity(), 4);
        assert_eq!(arity4_parent.to_digests().unwrap(), arity4_expected);
        assert_eq!(arity4_parent.root().unwrap(), arity4_level.root().unwrap());
    }

    #[test]
    fn cuda_digest_checkpoint_level_stops_at_parent_threshold() {
        let level = (0..19)
            .map(|index| {
                digest([
                    100 + index * 4,
                    101 + index * 4,
                    102 + index * 4,
                    103 + index * 4,
                ])
            })
            .collect::<Vec<_>>();
        let words = digest_words(&level);
        let buffer = CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload");
        let digest_level = CudaDigestLevel::new(
            buffer,
            level.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let expected_levels = parent_levels_from_digest_level_on_cpu(&level, 4)
            .expect("cpu parent levels should hash");

        let checkpoint = digest_level
            .into_parent_checkpoint_level(2)
            .expect("checkpoint level should hash");

        assert_eq!(checkpoint.source_state_count(), level.len());
        assert_eq!(checkpoint.folded_level_count(), 2);
        assert_eq!(checkpoint.state_count(), 2);
        assert_eq!(checkpoint.arity(), 4);
        assert_eq!(checkpoint.to_digests().unwrap(), expected_levels[1].parents);
        assert_eq!(
            checkpoint.root().unwrap(),
            root_from_digest_level_on_cuda(&level, 4).unwrap()
        );
    }

    #[test]
    fn cuda_digest_checkpoint_lower_prefix_matches_full_path_for_padded_multi_level_source_row() {
        let level = (0..70)
            .map(|index| {
                digest([
                    500 + index * 4,
                    501 + index * 4,
                    502 + index * 4,
                    503 + index * 4,
                ])
            })
            .collect::<Vec<_>>();
        let query_row = 69;
        let words = digest_words(&level);
        let buffer = CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload");
        let digest_level = CudaDigestLevel::new(
            buffer,
            level.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let checkpoint = digest_level
            .parent_checkpoint_level(2)
            .expect("checkpoint level should hash")
            .expect("checkpoint should fold multiple levels");
        assert_eq!(checkpoint.folded_level_count(), 3);

        let full_path = digest_level
            .opening_path(query_row)
            .expect("full opening path should hash");
        let lower_prefix = digest_level
            .opening_path_prefix_for_source_row(query_row, checkpoint.folded_level_count())
            .expect("lower checkpoint prefix should hash");
        let upper_suffix = checkpoint
            .opening_path_for_source_row(query_row)
            .expect("upper checkpoint suffix should hash");

        assert_eq!(
            lower_prefix,
            full_path.siblings[..checkpoint.folded_level_count()]
        );
        assert_eq!(
            upper_suffix.siblings,
            full_path.siblings[checkpoint.folded_level_count()..]
        );
        let mut stitched_path = lower_prefix;
        stitched_path.extend(upper_suffix.siblings);
        assert_eq!(stitched_path, full_path.siblings);
    }

    #[test]
    fn cuda_digest_checkpoint_batched_suffix_matches_per_row_suffixes() {
        let level = (0..70)
            .map(|index| {
                digest([
                    900 + index * 4,
                    901 + index * 4,
                    902 + index * 4,
                    903 + index * 4,
                ])
            })
            .collect::<Vec<_>>();
        let query_rows = vec![0, 3, 16, 17, 35, 69];
        let words = digest_words(&level);
        let buffer = CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload");
        let digest_level = CudaDigestLevel::new(
            buffer,
            level.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let checkpoint = digest_level
            .parent_checkpoint_level(2)
            .expect("checkpoint level should hash")
            .expect("checkpoint should fold multiple levels");

        let batched = checkpoint
            .opening_path_siblings_batch_for_source_rows(&query_rows)
            .expect("batched checkpoint suffix should hash");
        let expected = query_rows
            .iter()
            .map(|query_row| {
                checkpoint
                    .opening_path_siblings_for_source_row(*query_row)
                    .expect("single checkpoint suffix should hash")
            })
            .collect::<Vec<_>>();

        assert_eq!(batched, expected);
    }

    #[test]
    fn cuda_digest_opening_prefix_device_batch_decodes_like_host_batch() {
        let level = (0..70)
            .map(|index| {
                digest([
                    1300 + index * 4,
                    1301 + index * 4,
                    1302 + index * 4,
                    1303 + index * 4,
                ])
            })
            .collect::<Vec<_>>();
        let query_rows = vec![0, 3, 16, 17, 35, 69];
        let words = digest_words(&level);
        let buffer = CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload");
        let digest_level = CudaDigestLevel::new(
            buffer,
            level.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let prefix_level_count = 3;
        let expected = digest_level
            .opening_path_prefix_batch_for_source_rows(&query_rows, prefix_level_count)
            .expect("host-decoded prefix batch should hash");

        let actual = digest_level
            .opening_path_prefix_batch_device_for_source_rows(&query_rows, prefix_level_count)
            .expect("device prefix batch should hash")
            .into_siblings()
            .expect("device prefix batch should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn cuda_digest_checkpoint_batched_suffix_device_decodes_like_host_batch() {
        let level = (0..70)
            .map(|index| {
                digest([
                    1700 + index * 4,
                    1701 + index * 4,
                    1702 + index * 4,
                    1703 + index * 4,
                ])
            })
            .collect::<Vec<_>>();
        let query_rows = vec![0, 3, 16, 17, 35, 69];
        let words = digest_words(&level);
        let buffer = CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload");
        let digest_level = CudaDigestLevel::new(
            buffer,
            level.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let checkpoint = digest_level
            .parent_checkpoint_level(2)
            .expect("checkpoint level should hash")
            .expect("checkpoint should fold multiple levels");
        let expected = checkpoint
            .opening_path_siblings_batch_for_source_rows(&query_rows)
            .expect("host-decoded checkpoint suffix should hash");

        let actual = checkpoint
            .opening_path_siblings_batch_device_for_source_rows(&query_rows)
            .expect("device checkpoint suffix should hash")
            .into_siblings()
            .expect("device checkpoint suffix should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn cuda_digest_checkpoint_cross_buffer_suffixes_match_individual_paths() {
        let state_counts = [70usize, 81, 129];
        let query_rows = [69usize, 80, 128];
        let checkpoints = state_counts
            .iter()
            .enumerate()
            .map(|(group, state_count)| {
                let level = (0..*state_count)
                    .map(|index| {
                        let base = 3000 + group as u64 * 1000 + index as u64 * 4;
                        digest([base, base + 1, base + 2, base + 3])
                    })
                    .collect::<Vec<_>>();
                let buffer = CudaDeviceBuffer::from_u64_words(&digest_words(&level))
                    .expect("digests should upload");
                CudaDigestLevel::new(
                    buffer,
                    level.len(),
                    4,
                    cuda_poseidon2_width16_merkle_digest_root_device,
                )
                .parent_checkpoint_level(40)
                .expect("checkpoint should hash")
                .expect("checkpoint should fold one level")
            })
            .collect::<Vec<_>>();
        let sources = checkpoints
            .iter()
            .zip(query_rows)
            .map(|(checkpoint, source_row)| {
                CudaDigestCheckpointOpeningSource::new(checkpoint, source_row)
                    .expect("opening source should validate")
            })
            .collect::<Vec<_>>();

        let batches = opening_path_siblings_across_digest_checkpoints(&sources)
            .expect("cross-buffer paths should hash");
        for (((checkpoint, source_row), batch), expected) in
            checkpoints.iter().zip(query_rows).zip(batches).zip(
                checkpoints
                    .iter()
                    .zip(query_rows)
                    .map(|(checkpoint, source_row)| {
                        checkpoint
                            .opening_path_siblings_batch_for_source_rows(&[source_row])
                            .expect("individual path should hash")
                    }),
            )
        {
            let actual = batch
                .into_siblings_for_source_rows(checkpoint, &[source_row])
                .expect("batch should match its checkpoint");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn cuda_digest_checkpoint_cross_buffer_suffix_rejects_swapped_checkpoint() {
        let checkpoints = [0_u64, 1000]
            .into_iter()
            .map(|offset| {
                let level = (0..70)
                    .map(|index| {
                        let base = 6000 + offset + index * 4;
                        digest([base, base + 1, base + 2, base + 3])
                    })
                    .collect::<Vec<_>>();
                let buffer = CudaDeviceBuffer::from_u64_words(&digest_words(&level))
                    .expect("digests should upload");
                CudaDigestLevel::new(
                    buffer,
                    level.len(),
                    4,
                    cuda_poseidon2_width16_merkle_digest_root_device,
                )
                .parent_checkpoint_level(20)
                .expect("checkpoint should hash")
                .expect("checkpoint should fold")
            })
            .collect::<Vec<_>>();
        let sources = checkpoints
            .iter()
            .map(|checkpoint| {
                CudaDigestCheckpointOpeningSource::new(checkpoint, 69)
                    .expect("opening source should validate")
            })
            .collect::<Vec<_>>();
        let mut batches = opening_path_siblings_across_digest_checkpoints(&sources)
            .expect("cross-buffer paths should hash");
        let first = batches.remove(0);

        assert!(first
            .into_siblings_for_source_rows(&checkpoints[1], &[69])
            .is_err());
    }

    #[test]
    fn cuda_merkle_sibling_device_batches_concat_levels_like_host_decode() {
        let level = (0..70)
            .map(|index| {
                digest([
                    2100 + index * 4,
                    2101 + index * 4,
                    2102 + index * 4,
                    2103 + index * 4,
                ])
            })
            .collect::<Vec<_>>();
        let query_rows = vec![0, 3, 16, 17, 35, 69];
        let words = digest_words(&level);
        let buffer = CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload");
        let digest_level = CudaDigestLevel::new(
            buffer,
            level.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let checkpoint = digest_level
            .parent_checkpoint_level(2)
            .expect("checkpoint level should hash")
            .expect("checkpoint should fold multiple levels");

        let lower = digest_level
            .opening_path_prefix_batch_for_source_rows(&query_rows, checkpoint.folded_level_count())
            .expect("host lower prefixes should decode");
        let upper = checkpoint
            .opening_path_siblings_batch_for_source_rows(&query_rows)
            .expect("host upper suffixes should decode");
        let expected = lower
            .into_iter()
            .zip(upper)
            .map(|(mut lower, upper)| {
                lower.extend(upper);
                lower
            })
            .collect::<Vec<_>>();

        let actual = digest_level
            .opening_path_prefix_batch_device_for_source_rows(
                &query_rows,
                checkpoint.folded_level_count(),
            )
            .expect("device lower prefixes should hash")
            .concat_levels(
                checkpoint
                    .opening_path_siblings_batch_device_for_source_rows(&query_rows)
                    .expect("device upper suffixes should hash"),
            )
            .expect("device sibling batches should concatenate")
            .into_siblings()
            .expect("combined device siblings should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn cuda_merkle_sibling_device_batches_decode_many_like_individual_decodes() {
        let level = (0..70)
            .map(|index| {
                digest([
                    2500 + index * 4,
                    2501 + index * 4,
                    2502 + index * 4,
                    2503 + index * 4,
                ])
            })
            .collect::<Vec<_>>();
        let query_groups = [vec![0, 3], vec![16, 17, 35, 69]];
        let words = digest_words(&level);
        let buffer = CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload");
        let digest_level = CudaDigestLevel::new(
            buffer,
            level.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );
        let checkpoint = digest_level
            .parent_checkpoint_level(2)
            .expect("checkpoint level should hash")
            .expect("checkpoint should fold multiple levels");

        let mut expected = Vec::new();
        let mut device_batches = Vec::new();
        for query_rows in &query_groups {
            let lower = digest_level
                .opening_path_prefix_batch_for_source_rows(
                    query_rows,
                    checkpoint.folded_level_count(),
                )
                .expect("host lower prefixes should decode");
            let upper = checkpoint
                .opening_path_siblings_batch_for_source_rows(query_rows)
                .expect("host upper suffixes should decode");
            expected.push(
                lower
                    .into_iter()
                    .zip(upper)
                    .map(|(mut lower, upper)| {
                        lower.extend(upper);
                        lower
                    })
                    .collect::<Vec<_>>(),
            );
            device_batches.push(
                digest_level
                    .opening_path_prefix_batch_device_for_source_rows(
                        query_rows,
                        checkpoint.folded_level_count(),
                    )
                    .expect("device lower prefixes should hash")
                    .concat_levels(
                        checkpoint
                            .opening_path_siblings_batch_device_for_source_rows(query_rows)
                            .expect("device upper suffixes should hash"),
                    )
                    .expect("device sibling batches should concatenate"),
            );
        }

        let actual = CudaMerkleSiblingBatchDeviceBuffer::into_siblings_many(device_batches)
            .expect("combined device sibling batches should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn cuda_digest_opening_path_siblings_match_full_path_siblings() {
        let level = (0..37)
            .map(|index| {
                digest([
                    700 + index * 4,
                    701 + index * 4,
                    702 + index * 4,
                    703 + index * 4,
                ])
            })
            .collect::<Vec<_>>();
        let query_row = 35;
        let words = digest_words(&level);
        let buffer = CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload");
        let digest_level = CudaDigestLevel::new(
            buffer,
            level.len(),
            4,
            cuda_poseidon2_width16_merkle_digest_root_device,
        );

        let full_path = digest_level
            .opening_path(query_row)
            .expect("full opening path should hash");
        let siblings_only = digest_level
            .opening_path_siblings(query_row)
            .expect("siblings-only opening path should hash");

        assert_eq!(siblings_only, full_path.siblings);
    }

    #[test]
    fn cuda_root_from_digest_level_matches_cpu_reference() {
        let level = vec![
            digest([1, 2, 3, 4]),
            digest([5, 6, 7, 8]),
            digest([9, 10, 11, 12]),
            digest([13, 14, 15, 16]),
            digest([17, 18, 19, 20]),
        ];

        let actual_arity2 =
            root_from_digest_level_on_cuda(&level, 2).expect("cuda arity-2 root should hash");
        let expected_arity2 =
            cpu_root_from_digest_level(&level, 2).expect("cpu arity-2 root should hash");
        assert_eq!(actual_arity2, expected_arity2);

        let actual_arity4 =
            root_from_digest_level_on_cuda(&level, 4).expect("cuda arity-4 root should hash");
        let expected_arity4 =
            cpu_root_from_digest_level(&level, 4).expect("cpu arity-4 root should hash");
        assert_eq!(actual_arity4, expected_arity4);
    }

    #[test]
    fn cuda_parent_levels_match_cpu_reference() {
        let level = vec![
            digest([1, 2, 3, 4]),
            digest([5, 6, 7, 8]),
            digest([9, 10, 11, 12]),
            digest([13, 14, 15, 16]),
            digest([17, 18, 19, 20]),
            digest([21, 22, 23, 24]),
            digest([25, 26, 27, 28]),
            digest([29, 30, 31, 32]),
        ];

        let actual =
            parent_levels_from_digest_level(&level, 4).expect("cuda parent levels should hash");
        let expected = parent_levels_from_digest_level_on_cpu(&level, 4)
            .expect("cpu parent levels should hash");

        assert_eq!(actual, expected);
        assert_eq!(
            actual
                .iter()
                .map(|level| level.padding_count)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
    }

    #[test]
    fn cuda_arity2_parent_levels_match_cpu_reference_with_padding() {
        let level = vec![
            digest([1, 2, 3, 4]),
            digest([5, 6, 7, 8]),
            digest([9, 10, 11, 12]),
            digest([13, 14, 15, 16]),
            digest([17, 18, 19, 20]),
        ];

        let actual =
            parent_levels_from_digest_level(&level, 2).expect("cuda parent levels should hash");
        let expected = parent_levels_from_digest_level_on_cpu(&level, 2)
            .expect("cpu parent levels should hash");

        assert_eq!(actual, expected);
        assert_eq!(
            actual
                .iter()
                .map(|level| level.padding_count)
                .collect::<Vec<_>>(),
            vec![1, 1, 0]
        );
    }

    fn cpu_root_from_digest_level(
        level: &[[Felt; 4]],
        arity: usize,
    ) -> Result<[Felt; 4], super::MerkleHashError> {
        if level.is_empty() {
            return Ok([Felt::ZERO; 4]);
        }

        let mut level = level.to_vec();
        while level.len() > 1 {
            let extra_zeros = (arity - (level.len() % arity)) % arity;
            level.resize(level.len() + extra_zeros, [Felt::ZERO; 4]);
            level = level
                .chunks_exact(arity)
                .map(|children| parent_hash(children, arity))
                .collect::<Result<Vec<_>, _>>()?;
        }
        Ok(level[0])
    }

    fn digest(values: [u64; 4]) -> [Felt; 4] {
        values.map(Felt::from_u64)
    }

    fn values(values: &[u64]) -> Vec<Felt> {
        values.iter().copied().map(Felt::from_u64).collect()
    }

    fn digest_words(digests: &[[Felt; 4]]) -> Vec<u64> {
        digests
            .iter()
            .flat_map(|digest| digest.iter().map(|value| value.to_u64()))
            .collect()
    }

    fn felt_array<const WIDTH: usize>(words: &[u64]) -> [Felt; WIDTH] {
        let mut values = [Felt::ZERO; WIDTH];
        for (value, word) in values.iter_mut().zip(words) {
            *value = Felt::from_u64(*word);
        }
        values
    }
}

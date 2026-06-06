use std::fmt;

#[cfg(all(test, feature = "cuda"))]
use lzvm_accel::{cuda_poseidon2_width16_device, cuda_poseidon2_width8_device};
#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_poseidon2_width16_linear_round_device,
    cuda_poseidon2_width16_linear_round_row_major_digest_device,
    cuda_poseidon2_width16_merkle_opening_path_device, cuda_poseidon2_width16_merkle_parent_device,
    cuda_poseidon2_width16_merkle_root_device, cuda_poseidon2_width8_linear_round_device,
    cuda_poseidon2_width8_linear_round_row_major_digest_device,
    cuda_poseidon2_width8_merkle_opening_path_device, cuda_poseidon2_width8_merkle_parent_device,
    cuda_poseidon2_width8_merkle_root_device, CudaDeviceBuffer,
};
use lzvm_field::{poseidon2_hash_16, poseidon2_hash_8, Felt, FieldError};

pub(crate) const HASH_WORDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MerkleHashError {
    UnsupportedArity { arity: usize },
    InvalidChildCount { expected: usize, found: usize },
    Field(FieldError),
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
            Self::LengthOverflow => write!(f, "Merkle hash length overflow"),
        }
    }
}

impl std::error::Error for MerkleHashError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MerkleParentLevel {
    pub(crate) padding_count: usize,
    pub(crate) parents: Vec<[Felt; HASH_WORDS]>,
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) struct CudaDigestLevel {
    states: CudaDeviceBuffer,
    state_count: usize,
    arity: usize,
    width: usize,
    root_operation: CudaPoseidon2RootOp,
}

#[cfg(feature = "cuda")]
pub(crate) struct CudaMerkleOpeningPath {
    pub(crate) root: [Felt; HASH_WORDS],
    pub(crate) siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
}

#[cfg(feature = "cuda")]
impl CudaDigestLevel {
    fn new(
        states: CudaDeviceBuffer,
        state_count: usize,
        arity: usize,
        width: usize,
        root_operation: CudaPoseidon2RootOp,
    ) -> Self {
        Self {
            states,
            state_count,
            arity,
            width,
            root_operation,
        }
    }

    pub(crate) fn state_count(&self) -> usize {
        self.state_count
    }

    pub(crate) fn arity(&self) -> usize {
        self.arity
    }

    pub(crate) fn to_digests(&self) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
        let output = self
            .states
            .to_state_prefix_u64_words(self.state_count, self.width, HASH_WORDS)
            .map_err(|_| MerkleHashError::LengthOverflow)?;
        digests_from_hashed_states(&output, HASH_WORDS)
    }

    pub(crate) fn root(&self) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
        let root_words =
            (self.root_operation)(&self.states).map_err(|_| MerkleHashError::LengthOverflow)?;
        digest_from_state_words(&root_words)
    }

    pub(crate) fn into_opening_path(
        self,
        query_row: usize,
    ) -> Result<CudaMerkleOpeningPath, MerkleHashError> {
        self.opening_path(query_row)
    }

    pub(crate) fn opening_path(
        &self,
        query_row: usize,
    ) -> Result<CudaMerkleOpeningPath, MerkleHashError> {
        if query_row >= self.state_count {
            return Err(MerkleHashError::LengthOverflow);
        }
        let path = match self.arity {
            2 => cuda_poseidon2_width8_merkle_opening_path_device(&self.states, query_row),
            4 => cuda_poseidon2_width16_merkle_opening_path_device(&self.states, query_row),
            _ => return Err(MerkleHashError::UnsupportedArity { arity: self.arity }),
        }
        .map_err(|_| MerkleHashError::LengthOverflow)?;
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
        cpu_linear_hashes_from_row_major_bytes(bytes, row_count, column_count, arity)
    }
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
            let states = if column_count <= HASH_WORDS {
                CudaDeviceBuffer::from_device_state_prefix_u64_words(
                    row_values,
                    row_count,
                    8,
                    column_count,
                )
                .map_err(|_| MerkleHashError::LengthOverflow)?
            } else {
                cuda_linear_hash_states_with_row_major_device_rounds(
                    row_count,
                    column_count,
                    HASH_WORDS,
                    8,
                    cuda_poseidon2_width8_linear_round_row_major_digest_device,
                    row_values,
                )?
            };
            Ok(CudaDigestLevel::new(
                states,
                row_count,
                arity,
                8,
                cuda_poseidon2_width8_merkle_root_device,
            ))
        }
        4 => {
            let states = if column_count <= HASH_WORDS {
                CudaDeviceBuffer::from_device_state_prefix_u64_words(
                    row_values,
                    row_count,
                    16,
                    column_count,
                )
                .map_err(|_| MerkleHashError::LengthOverflow)?
            } else {
                cuda_linear_hash_states_with_row_major_device_rounds(
                    row_count,
                    column_count,
                    12,
                    16,
                    cuda_poseidon2_width16_linear_round_row_major_digest_device,
                    row_values,
                )?
            };
            Ok(CudaDigestLevel::new(
                states,
                row_count,
                arity,
                16,
                cuda_poseidon2_width16_merkle_root_device,
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

#[cfg(any(test, not(feature = "cuda")))]
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
    let (width, operation): (usize, CudaPoseidon2DeviceOp) = match arity {
        2 => (8, cuda_poseidon2_width8_merkle_parent_device),
        4 => (16, cuda_poseidon2_width16_merkle_parent_device),
        _ => unreachable!("arity is validated"),
    };

    let current = state_buffer_from_digest_level(level, width)?;
    parent_levels_from_device_buffer(current, level.len(), arity, width, operation)
}

#[cfg(feature = "cuda")]
fn parent_levels_from_device_buffer(
    mut current: CudaDeviceBuffer,
    mut state_count: usize,
    arity: usize,
    width: usize,
    operation: CudaPoseidon2DeviceOp,
) -> Result<Vec<MerkleParentLevel>, MerkleHashError> {
    let mut levels = Vec::new();
    while state_count > 1 {
        let padding_count = padding_count(state_count, arity)?;
        let parent_count = state_count.div_ceil(arity);
        let mut next = CudaDeviceBuffer::new(
            parent_count
                .checked_mul(width)
                .and_then(|word_count| word_count.checked_mul(8))
                .ok_or(MerkleHashError::LengthOverflow)?,
        )
        .map_err(|_| MerkleHashError::LengthOverflow)?;
        operation(&current, &mut next).map_err(|_| MerkleHashError::LengthOverflow)?;
        let parent_words = next
            .to_state_prefix_u64_words(parent_count, width, HASH_WORDS)
            .map_err(|_| MerkleHashError::LengthOverflow)?;
        let parents = digests_from_hashed_states(&parent_words, HASH_WORDS)?;
        levels.push(MerkleParentLevel {
            padding_count,
            parents,
        });
        current = next;
        state_count = parent_count;
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

    let width = match arity {
        2 => 8,
        4 => 16,
        _ => unreachable!("arity is validated"),
    };

    let input_words = digest_level_as_state_words(level, width)?;
    let input_buffer = CudaDeviceBuffer::from_u64_words(&input_words)
        .map_err(|_| MerkleHashError::LengthOverflow)?;
    let root_words = match arity {
        2 => cuda_poseidon2_width8_merkle_root_device(&input_buffer),
        4 => cuda_poseidon2_width16_merkle_root_device(&input_buffer),
        _ => unreachable!("arity is validated"),
    }
    .map_err(|_| MerkleHashError::LengthOverflow)?;
    digest_from_state_words(&root_words)
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
fn state_buffer_from_digest_level(
    level: &[[Felt; HASH_WORDS]],
    width: usize,
) -> Result<CudaDeviceBuffer, MerkleHashError> {
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
    CudaDeviceBuffer::from_state_prefix_u64_words(&words, level.len(), width, HASH_WORDS)
        .map_err(|_| MerkleHashError::LengthOverflow)
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

#[cfg(not(feature = "cuda"))]
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
    let mut buffer =
        CudaDeviceBuffer::new(bytes.len()).map_err(|_| MerkleHashError::LengthOverflow)?;
    buffer
        .copy_from(bytes)
        .map_err(|_| MerkleHashError::LengthOverflow)?;
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
        let row_values_buffer = CudaDeviceBuffer::from_u64_words(&row_values)
            .map_err(|_| MerkleHashError::LengthOverflow)?;
        let mut next_states = CudaDeviceBuffer::new(
            row_count
                .checked_mul(width)
                .and_then(|words| words.checked_mul(8))
                .ok_or(MerkleHashError::LengthOverflow)?,
        )
        .map_err(|_| MerkleHashError::LengthOverflow)?;
        operation(
            &current_states,
            &row_values_buffer,
            &mut next_states,
            chunk_len,
        )
        .map_err(|_| MerkleHashError::LengthOverflow)?;
        current_states = next_states;
        offset += chunk_len;
    }

    let output = current_states
        .to_state_prefix_u64_words(row_count, width, HASH_WORDS)
        .map_err(|_| MerkleHashError::LengthOverflow)?;
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
        .map_err(|_| MerkleHashError::LengthOverflow)?;
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
        CudaDeviceBuffer::new(state_byte_count).map_err(|_| MerkleHashError::LengthOverflow)?;
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
        .map_err(|_| MerkleHashError::LengthOverflow)?;
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
        return CudaDeviceBuffer::new(0).map_err(|_| MerkleHashError::LengthOverflow);
    }
    CudaDeviceBuffer::zeroed(
        words
            .checked_mul(8)
            .ok_or(MerkleHashError::LengthOverflow)?,
    )
    .map_err(|_| MerkleHashError::LengthOverflow)
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
    let input_buffer = CudaDeviceBuffer::from_u64_words(&input_words)
        .map_err(|_| MerkleHashError::LengthOverflow)?;
    let parent_count = children.len().div_ceil(arity);
    let mut output_buffer = CudaDeviceBuffer::new(
        parent_count
            .checked_mul(width)
            .and_then(|word_count| word_count.checked_mul(8))
            .ok_or(MerkleHashError::LengthOverflow)?,
    )
    .map_err(|_| MerkleHashError::LengthOverflow)?;
    operation(&input_buffer, &mut output_buffer).map_err(|_| MerkleHashError::LengthOverflow)?;
    let hashed = output_buffer
        .to_u64_words()
        .map_err(|_| MerkleHashError::LengthOverflow)?;
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

    let input_buffer =
        CudaDeviceBuffer::from_u64_words(words).map_err(|_| MerkleHashError::LengthOverflow)?;
    let mut output_buffer = CudaDeviceBuffer::new(
        words
            .len()
            .checked_mul(8)
            .ok_or(MerkleHashError::LengthOverflow)?,
    )
    .map_err(|_| MerkleHashError::LengthOverflow)?;
    operation(&input_buffer, &mut output_buffer).map_err(|_| MerkleHashError::LengthOverflow)?;
    output_buffer
        .to_u64_words()
        .map_err(|_| MerkleHashError::LengthOverflow)
}

#[cfg(feature = "cuda")]
fn digest_from_state_words(words: &[u64]) -> Result<[Felt; HASH_WORDS], MerkleHashError> {
    let mut digest = [Felt::ZERO; HASH_WORDS];
    for (slot, word) in digest.iter_mut().zip(words.iter()) {
        *slot = Felt::from_canonical(*word).map_err(|_| MerkleHashError::LengthOverflow)?;
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
        linear_hashes, linear_hashes_from_row_major_bytes, parent_hash, parent_hashes,
        parent_levels_from_digest_level, parent_levels_from_digest_level_on_cpu,
        root_from_digest_level_on_cuda,
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

    fn felt_array<const WIDTH: usize>(words: &[u64]) -> [Felt; WIDTH] {
        let mut values = [Felt::ZERO; WIDTH];
        for (value, word) in values.iter_mut().zip(words) {
            *value = Felt::from_u64(*word);
        }
        values
    }
}

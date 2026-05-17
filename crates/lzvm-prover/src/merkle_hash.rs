use std::fmt;

#[cfg(all(test, feature = "cuda"))]
use lzvm_accel::{cuda_poseidon2_width16_device, cuda_poseidon2_width8_device};
#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_poseidon2_width16_linear_round_device, cuda_poseidon2_width16_merkle_parent_device,
    cuda_poseidon2_width8_linear_round_device, cuda_poseidon2_width8_merkle_parent_device,
    CudaDeviceBuffer,
};
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

    let (width, operation): (usize, CudaPoseidon2DeviceOp) = match arity {
        2 => (8, cuda_poseidon2_width8_merkle_parent_device),
        4 => (16, cuda_poseidon2_width16_merkle_parent_device),
        _ => unreachable!("arity is validated"),
    };

    let input_words = digest_level_as_state_words(level, width)?;
    let mut current = CudaDeviceBuffer::from_u64_words(&input_words)
        .map_err(|_| MerkleHashError::LengthOverflow)?;
    let mut state_count = level.len();
    while state_count > 1 {
        let next_state_count = state_count.div_ceil(arity);
        let next_byte_count = next_state_count
            .checked_mul(width)
            .and_then(|word_count| word_count.checked_mul(8))
            .ok_or(MerkleHashError::LengthOverflow)?;
        let mut next =
            CudaDeviceBuffer::new(next_byte_count).map_err(|_| MerkleHashError::LengthOverflow)?;
        operation(&current, &mut next).map_err(|_| MerkleHashError::LengthOverflow)?;
        current = next;
        state_count = next_state_count;
    }

    let root_state = current
        .to_u64_words()
        .map_err(|_| MerkleHashError::LengthOverflow)?;
    digest_from_state_words(&root_state)
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

fn validate_arity(arity: usize) -> Result<(), MerkleHashError> {
    match arity {
        2 | 4 => Ok(()),
        _ => Err(MerkleHashError::UnsupportedArity { arity }),
    }
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

    let mut current_states = zero_state_buffer(rows.len(), WIDTH)?;
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(HASH_WORDS);
        let row_values = pack_linear_round_values(rows, offset, chunk_len)?;
        let row_values_buffer = CudaDeviceBuffer::from_u64_words(&row_values)
            .map_err(|_| MerkleHashError::LengthOverflow)?;
        let mut next_states = CudaDeviceBuffer::new(
            rows.len()
                .checked_mul(WIDTH)
                .and_then(|words| words.checked_mul(8))
                .ok_or(MerkleHashError::LengthOverflow)?,
        )
        .map_err(|_| MerkleHashError::LengthOverflow)?;
        cuda_poseidon2_width8_linear_round_device(
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
        .to_u64_words()
        .map_err(|_| MerkleHashError::LengthOverflow)?;
    digests_from_hashed_states(&output, WIDTH)
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

    let mut current_states = zero_state_buffer(rows.len(), WIDTH)?;
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(RATE);
        let row_values = pack_linear_round_values(rows, offset, chunk_len)?;
        let row_values_buffer = CudaDeviceBuffer::from_u64_words(&row_values)
            .map_err(|_| MerkleHashError::LengthOverflow)?;
        let mut next_states = CudaDeviceBuffer::new(
            rows.len()
                .checked_mul(WIDTH)
                .and_then(|words| words.checked_mul(8))
                .ok_or(MerkleHashError::LengthOverflow)?,
        )
        .map_err(|_| MerkleHashError::LengthOverflow)?;
        cuda_poseidon2_width16_linear_round_device(
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
        .to_u64_words()
        .map_err(|_| MerkleHashError::LengthOverflow)?;
    digests_from_hashed_states(&output, WIDTH)
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
    let zeros = vec![0_u64; words];
    CudaDeviceBuffer::from_u64_words(&zeros).map_err(|_| MerkleHashError::LengthOverflow)
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
        linear_hashes, parent_hash, parent_hashes, root_from_digest_level_on_cuda,
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

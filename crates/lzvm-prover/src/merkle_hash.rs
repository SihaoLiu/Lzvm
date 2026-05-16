use std::fmt;

#[cfg(feature = "cuda")]
use lzvm_accel::{cuda_poseidon2_width16, cuda_poseidon2_width8};
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

    let mut states = vec![[Felt::ZERO; WIDTH]; rows.len()];
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(HASH_WORDS);
        let mut input = Vec::with_capacity(rows.len() * WIDTH);
        for (state, row) in states.iter().zip(rows) {
            let capacity = [state[0], state[1], state[2], state[3]];
            let mut next = [Felt::ZERO; WIDTH];
            next[..chunk_len].copy_from_slice(&row[offset..offset + chunk_len]);
            next[HASH_WORDS..].copy_from_slice(&capacity);
            push_felt_words(&mut input, &next);
        }

        let output = cuda_poseidon2_width8(&input).map_err(|_| MerkleHashError::LengthOverflow)?;
        for (state, chunk) in states.iter_mut().zip(output.chunks_exact(WIDTH)) {
            *state = felt_array_from_words(chunk)?;
        }
        offset += chunk_len;
    }

    Ok(states
        .into_iter()
        .map(|state| [state[0], state[1], state[2], state[3]])
        .collect())
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

    let mut states = vec![[Felt::ZERO; WIDTH]; rows.len()];
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(RATE);
        let mut input = Vec::with_capacity(rows.len() * WIDTH);
        for (state, row) in states.iter().zip(rows) {
            let capacity = [state[0], state[1], state[2], state[3]];
            let mut next = [Felt::ZERO; WIDTH];
            next[..chunk_len].copy_from_slice(&row[offset..offset + chunk_len]);
            next[RATE..].copy_from_slice(&capacity);
            push_felt_words(&mut input, &next);
        }

        let output = cuda_poseidon2_width16(&input).map_err(|_| MerkleHashError::LengthOverflow)?;
        for (state, chunk) in states.iter_mut().zip(output.chunks_exact(WIDTH)) {
            *state = felt_array_from_words(chunk)?;
        }
        offset += chunk_len;
    }

    Ok(states
        .into_iter()
        .map(|state| [state[0], state[1], state[2], state[3]])
        .collect())
}

#[cfg(feature = "cuda")]
fn push_felt_words(out: &mut Vec<u64>, values: &[Felt]) {
    out.extend(values.iter().map(|value| value.to_u64()));
}

#[cfg(feature = "cuda")]
fn felt_array_from_words<const N: usize>(words: &[u64]) -> Result<[Felt; N], MerkleHashError> {
    let mut out = [Felt::ZERO; N];
    for (slot, word) in out.iter_mut().zip(words.iter()) {
        *slot = Felt::from_canonical(*word).map_err(|_| MerkleHashError::LengthOverflow)?;
    }
    Ok(out)
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
    let mut states = Vec::with_capacity(children.len() * HASH_WORDS);
    for child in children {
        states.extend(child.iter().map(|value| value.to_u64()));
    }
    let hashed = cuda_poseidon2_width8(&states).map_err(|_| MerkleHashError::LengthOverflow)?;
    digests_from_hashed_states(&hashed, 8)
}

#[cfg(feature = "cuda")]
fn parent_hashes_arity4_cuda(
    children: &[[Felt; HASH_WORDS]],
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    let mut states = Vec::with_capacity(children.len() * HASH_WORDS);
    for child in children {
        states.extend(child.iter().map(|value| value.to_u64()));
    }
    let hashed = cuda_poseidon2_width16(&states).map_err(|_| MerkleHashError::LengthOverflow)?;
    digests_from_hashed_states(&hashed, 16)
}

#[cfg(feature = "cuda")]
fn digests_from_hashed_states(
    states: &[u64],
    state_width: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, MerkleHashError> {
    let mut out = Vec::with_capacity(states.len() / state_width);
    for state in states.chunks_exact(state_width) {
        out.push([
            Felt::from_canonical(state[0]).map_err(|_| MerkleHashError::LengthOverflow)?,
            Felt::from_canonical(state[1]).map_err(|_| MerkleHashError::LengthOverflow)?,
            Felt::from_canonical(state[2]).map_err(|_| MerkleHashError::LengthOverflow)?,
            Felt::from_canonical(state[3]).map_err(|_| MerkleHashError::LengthOverflow)?,
        ]);
    }
    Ok(out)
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::{linear_hash, linear_hashes, parent_hash, parent_hashes};
    use lzvm_field::Felt;

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

    fn digest(values: [u64; 4]) -> [Felt; 4] {
        values.map(Felt::from_u64)
    }

    fn values(values: &[u64]) -> Vec<Felt> {
        values.iter().copied().map(Felt::from_u64).collect()
    }
}

use lzvm_field::Felt;

use crate::merkle_hash::{
    linear_hash, linear_hashes_from_row_major_bytes, parent_hash, parent_levels_from_digest_level,
};

use super::{
    WitnessStageCommitment, WitnessStageCommitmentError, WitnessStageLeaves, WitnessStageOpening,
    WitnessStageOpeningError, HASH_WORDS, WORD_BYTES,
};

pub fn commit_witness_stage_leaves(
    leaves: &WitnessStageLeaves,
    arity: usize,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let expected_leaf_bytes = validate_witness_stage_leaves(leaves, arity)?;

    let mut out = Vec::with_capacity(expected_leaf_bytes);
    out.extend_from_slice(leaves.bytes());

    commit_validated_witness_stage_bytes(
        leaves.stage_index(),
        leaves.extended_row_count(),
        leaves.column_count(),
        arity,
        expected_leaf_bytes,
        out,
    )
}

#[cfg_attr(feature = "cuda", allow(dead_code))]
pub(crate) fn commit_witness_stage_leaves_owned(
    leaves: WitnessStageLeaves,
    arity: usize,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let expected_leaf_bytes = validate_witness_stage_leaves(&leaves, arity)?;
    let stage_index = leaves.stage_index();
    let extended_row_count = leaves.extended_row_count();
    let column_count = leaves.column_count();
    let out = leaves.into_bytes();

    commit_validated_witness_stage_bytes(
        stage_index,
        extended_row_count,
        column_count,
        arity,
        expected_leaf_bytes,
        out,
    )
}

#[cfg(any(feature = "cuda", test))]
pub(crate) fn commit_witness_stage_leaves_owned_with_leaf_hashes(
    leaves: WitnessStageLeaves,
    arity: usize,
    leaf_hashes: Vec<[Felt; HASH_WORDS]>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    validate_witness_stage_leaves(&leaves, arity)?;
    let expected_leaf_hashes = leaves.extended_row_count();
    if leaf_hashes.len() != expected_leaf_hashes {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: expected_leaf_hashes,
            found: leaf_hashes.len(),
        });
    }
    let stage_index = leaves.stage_index();
    let extended_row_count = leaves.extended_row_count();
    let column_count = leaves.column_count();
    let out = leaves.into_bytes();

    commit_validated_witness_stage_bytes_with_leaf_hashes(
        stage_index,
        extended_row_count,
        column_count,
        arity,
        out,
        leaf_hashes,
    )
}

fn validate_witness_stage_leaves(
    leaves: &WitnessStageLeaves,
    arity: usize,
) -> Result<usize, WitnessStageCommitmentError> {
    validate_witness_commitment_arity(arity)?;
    if leaves.extended_row_count() == 0 {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }
    let expected_leaf_bytes = leaves
        .extended_row_count()
        .checked_mul(leaves.column_count())
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if leaves.bytes().len() != expected_leaf_bytes {
        return Err(WitnessStageCommitmentError::InvalidLeafByteLength {
            expected: expected_leaf_bytes,
            found: leaves.bytes().len(),
        });
    }

    Ok(expected_leaf_bytes)
}

fn commit_validated_witness_stage_bytes(
    stage_index: usize,
    extended_row_count: usize,
    column_count: usize,
    arity: usize,
    leaf_byte_count: usize,
    out: Vec<u8>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let level = linear_hashes_from_row_major_bytes(
        &out[..leaf_byte_count],
        extended_row_count,
        column_count,
        arity,
    )?;
    commit_validated_witness_stage_bytes_with_leaf_hashes(
        stage_index,
        extended_row_count,
        column_count,
        arity,
        out,
        level,
    )
}

fn commit_validated_witness_stage_bytes_with_leaf_hashes(
    stage_index: usize,
    extended_row_count: usize,
    column_count: usize,
    arity: usize,
    mut out: Vec<u8>,
    mut level: Vec<[Felt; HASH_WORDS]>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let tree_byte_count =
        expected_witness_stage_commitment_tree_byte_count(extended_row_count, column_count, arity)?;
    out.reserve_exact(tree_byte_count.saturating_sub(out.len()));
    for digest in &level {
        append_digest(&mut out, *digest);
    }

    for parent_level in parent_levels_from_digest_level(&level, arity)? {
        for _ in 0..parent_level.padding_count {
            append_digest(&mut out, [Felt::ZERO; HASH_WORDS]);
        }

        for digest in &parent_level.parents {
            append_digest(&mut out, *digest);
        }
        level = parent_level.parents;
    }

    Ok(WitnessStageCommitment::new(
        stage_index,
        arity,
        level[0],
        out,
    ))
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
        expected_witness_stage_opening_tree_byte_count(rows, column_count, commitment.arity())?;
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
        let padded_len = round_up_to_arity(
            level_len,
            commitment.arity(),
            WitnessStageOpeningError::LengthOverflow,
        )?;
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

    WitnessStageOpening::new(row_index, values, siblings)
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

fn expected_witness_stage_commitment_tree_byte_count(
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<usize, WitnessStageCommitmentError> {
    expected_witness_stage_tree_byte_count(
        row_count,
        column_count,
        arity,
        WitnessStageCommitmentError::LengthOverflow,
    )
}

fn expected_witness_stage_opening_tree_byte_count(
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<usize, WitnessStageOpeningError> {
    expected_witness_stage_tree_byte_count(
        row_count,
        column_count,
        arity,
        WitnessStageOpeningError::LengthOverflow,
    )
}

fn expected_witness_stage_tree_byte_count<E: Clone>(
    row_count: usize,
    column_count: usize,
    arity: usize,
    length_overflow: E,
) -> Result<usize, E> {
    let raw_byte_count = row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or_else(|| length_overflow.clone())?;
    let mut digest_count = row_count;
    let mut level_len = row_count;
    while level_len > 1 {
        let padded_len = round_up_to_arity(level_len, arity, length_overflow.clone())?;
        digest_count = digest_count
            .checked_add(padded_len - level_len)
            .and_then(|count| count.checked_add(padded_len / arity))
            .ok_or_else(|| length_overflow.clone())?;
        level_len = padded_len / arity;
    }
    raw_byte_count
        .checked_add(
            digest_count
                .checked_mul(HASH_WORDS * WORD_BYTES)
                .ok_or_else(|| length_overflow.clone())?,
        )
        .ok_or(length_overflow)
}

fn round_up_to_arity<E>(value: usize, arity: usize, length_overflow: E) -> Result<usize, E> {
    let extra = (arity - (value % arity)) % arity;
    value.checked_add(extra).ok_or(length_overflow)
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

#[cfg(test)]
mod tests {
    use super::{
        commit_witness_stage_leaves, commit_witness_stage_leaves_owned,
        commit_witness_stage_leaves_owned_with_leaf_hashes, open_witness_stage_commitment,
        verify_witness_stage_opening_root, WitnessStageCommitmentError, WitnessStageLeaves,
        WORD_BYTES,
    };
    use lzvm_field::{Felt, FieldError, MODULUS};

    #[test]
    fn rejects_malformed_witness_stage_leaf_byte_lengths() {
        let expected = 2 * 3 * WORD_BYTES;
        let leaves = WitnessStageLeaves::new(1, 2, 2, 3, vec![0_u8; expected - 1]);

        assert!(matches!(
            commit_witness_stage_leaves(&leaves, 2),
            Err(WitnessStageCommitmentError::InvalidLeafByteLength { expected, found })
                if expected == 2 * 3 * WORD_BYTES && found == expected - 1
        ));
    }

    #[test]
    fn owned_witness_stage_commitment_rejects_noncanonical_leaf_words_like_borrowed_commitment() {
        let mut bytes = Vec::new();
        for value in [1, 2, 3, 4, 5, MODULUS] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let leaves = WitnessStageLeaves::new(1, 2, 2, 3, bytes);

        assert_eq!(
            commit_witness_stage_leaves(&leaves, 2).expect_err("borrowed commit should fail"),
            WitnessStageCommitmentError::Field(FieldError::NonCanonical { value: MODULUS })
        );
        assert_eq!(
            commit_witness_stage_leaves_owned(leaves, 2).expect_err("owned commit should fail"),
            WitnessStageCommitmentError::Field(FieldError::NonCanonical { value: MODULUS })
        );
    }

    #[test]
    fn witness_stage_opening_verifies_root_with_padded_parent_level() {
        let row_count = 5;
        let column_count = 6;
        let mut bytes = Vec::new();
        let mut rows = Vec::new();
        for row in 0..row_count {
            let mut values = Vec::new();
            for column in 0..column_count {
                let value = Felt::from_u64((row * 100 + column + 1) as u64);
                bytes.extend_from_slice(&value.to_le_bytes());
                values.push(value);
            }
            rows.push(values);
        }
        let leaves = WitnessStageLeaves::new(7, row_count, row_count, column_count, bytes);
        let commitment =
            commit_witness_stage_leaves(&leaves, 4).expect("stage commitment should build");

        let opening = open_witness_stage_commitment(&commitment, 4, row_count as u64, column_count)
            .expect("stage row should open");
        let verifies = verify_witness_stage_opening_root(commitment.root(), 4, &opening)
            .expect("opening root check should run");

        assert_eq!(opening.values(), rows[4].as_slice());
        assert!(verifies);
    }

    #[test]
    fn owned_witness_stage_commitment_matches_borrowed_commitment() {
        let row_count = 5;
        let column_count = 6;
        let mut bytes = Vec::new();
        for row in 0..row_count {
            for column in 0..column_count {
                let value = Felt::from_u64((row * 100 + column + 1) as u64);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        let leaves = WitnessStageLeaves::new(7, row_count, row_count, column_count, bytes);

        let borrowed =
            commit_witness_stage_leaves(&leaves, 4).expect("stage commitment should build");
        let owned =
            commit_witness_stage_leaves_owned(leaves, 4).expect("stage commitment should build");

        assert_eq!(owned.stage_index(), borrowed.stage_index());
        assert_eq!(owned.arity(), borrowed.arity());
        assert_eq!(owned.root(), borrowed.root());
        assert_eq!(owned.tree_bytes(), borrowed.tree_bytes());

        let opening = open_witness_stage_commitment(&owned, 4, row_count as u64, column_count)
            .expect("stage row should open");
        assert!(verify_witness_stage_opening_root(owned.root(), 4, &opening)
            .expect("opening root check should run"));
    }

    #[test]
    fn prehashed_witness_stage_commitment_rejects_leaf_digest_count_mismatch() {
        let row_count = 5;
        let column_count = 6;
        let mut bytes = Vec::new();
        for row in 0..row_count {
            for column in 0..column_count {
                let value = Felt::from_u64((row * 100 + column + 1) as u64);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        let leaves = WitnessStageLeaves::new(7, row_count, row_count, column_count, bytes);
        let leaf_hashes = vec![[Felt::ZERO; 4]; row_count - 1];

        assert!(matches!(
            commit_witness_stage_leaves_owned_with_leaf_hashes(leaves, 4, leaf_hashes),
            Err(WitnessStageCommitmentError::InvalidLeafDigestCount { expected, found })
                if expected == row_count && found == row_count - 1
        ));
    }
}

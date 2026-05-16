mod errors;
mod extend;
mod load;
mod values;

pub use errors::*;
pub use extend::*;
pub use load::*;
pub use values::*;

use std::thread;

use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::Felt;
use sha2::{Digest, Sha256};

use crate::merkle_hash::{linear_hash, linear_hashes, parent_hash, parent_hashes};
use crate::witness_execution::ProveWitnessCommitments;
use crate::witness_layout::derive_witness_trace_layout;
use crate::witness_trace::WitnessTraceBuffer;
use crate::ProveUnitSchedule;

const HASH_WORDS: usize = 4;
const WORD_BYTES: usize = 8;

pub fn build_witness_commitment_segment(
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessSegmentError> {
    let unit_index =
        u32::try_from(output.unit_index()).map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
    let id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
        .checked_add(unit_index)
        .ok_or(ProveWitnessSegmentError::LengthOverflow)?;
    let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
    for commitment in output.stage_commitments().commitments() {
        let stage_index = u32::try_from(commitment.stage_index())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let arity = u32::try_from(commitment.arity())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let tree_byte_count = u64::try_from(commitment.tree_bytes().len())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        stages.push(WitnessCommitmentStageSegment {
            stage_index,
            arity,
            root: commitment.root().map(|value| value.to_u64()),
            tree_byte_count,
            tree_digest: Sha256::digest(commitment.tree_bytes()).into(),
        });
    }

    let segment = WitnessCommitmentSegment {
        unit_index,
        input_byte_count: u64::try_from(output.input_byte_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        trace_rows: u64::try_from(output.trace_row_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        trace_columns: u64::try_from(output.trace_column_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        stages,
    };
    Ok(ProofSegment {
        id,
        data: encode_witness_commitment_segment(&segment)?,
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

    Ok(WitnessTraceCommitments::new(commitments))
}

pub fn commit_witness_trace_stages_with_workers(
    trace: &WitnessTraceBuffer,
    unit: &ProveUnitSchedule,
    worker_count: usize,
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    let worker_count = worker_count.max(1);
    if worker_count == 1 || unit.stage_commit_widths.len() <= 1 {
        return commit_witness_trace_stages(trace, unit);
    }

    let layout = derive_witness_trace_layout(unit)?;
    let source_bits = usize::try_from(unit.base_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;
    let target_bits = usize::try_from(unit.extended_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;
    let arity = usize::try_from(unit.merkle_tree_arity)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;
    let stage_indices = layout
        .stages()
        .iter()
        .map(|stage| stage.stage_index)
        .collect::<Vec<_>>();
    let worker_count = worker_count.min(stage_indices.len());
    let chunk_size = stage_indices.len().div_ceil(worker_count);

    let mut commitments = Vec::with_capacity(stage_indices.len());
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in stage_indices.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            let layout = &layout;
            handles.push(scope.spawn(move || {
                let mut out = Vec::with_capacity(chunk.len());
                for stage_index in chunk {
                    let stage = layout.stage_trace(trace, stage_index)?;
                    let leaves = extend_witness_stage_leaves(&stage, source_bits, target_bits)?;
                    let commitment = commit_witness_stage_leaves(&leaves, arity)?;
                    out.push((stage_index, commitment));
                }
                Ok::<_, WitnessTraceCommitmentError>(out)
            }));
        }

        for handle in handles {
            let chunk = handle
                .join()
                .map_err(|_| WitnessTraceCommitmentError::WorkerPanic)??;
            commitments.extend(chunk);
        }
        Ok::<(), WitnessTraceCommitmentError>(())
    })?;

    commitments.sort_by_key(|(stage_index, _)| *stage_index);
    Ok(WitnessTraceCommitments::new(
        commitments
            .into_iter()
            .map(|(_, commitment)| commitment)
            .collect(),
    ))
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
        stages.push(WitnessStageExtendedValues::new(
            leaves.stage_index(),
            leaves.source_row_count(),
            leaves.extended_row_count(),
            leaves.column_count(),
            values,
        ));
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

    let mut level = linear_hashes(&rows, arity)?;
    for digest in &level {
        append_digest(&mut out, *digest);
    }

    while level.len() > 1 {
        let extra_zeros = (arity - (level.len() % arity)) % arity;
        for _ in 0..extra_zeros {
            let zero = [Felt::ZERO; HASH_WORDS];
            append_digest(&mut out, zero);
            level.push(zero);
        }

        let next = parent_hashes(&level, arity)?;
        for digest in &next {
            append_digest(&mut out, *digest);
        }
        level = next;
    }

    Ok(WitnessStageCommitment::new(
        leaves.stage_index(),
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

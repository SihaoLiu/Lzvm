mod errors;
mod extend;
mod load;
mod tree;
mod values;

pub use errors::*;
pub use extend::*;
pub use load::*;
pub use tree::*;
pub use values::*;

use std::thread;

use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use sha2::{Digest, Sha256};

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

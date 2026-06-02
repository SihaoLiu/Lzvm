use std::thread;

use crate::witness_layout::derive_witness_trace_layout;
use crate::witness_trace::WitnessTraceBuffer;
use crate::ProveUnitSchedule;

#[cfg(not(feature = "cuda"))]
use super::commit_witness_stage_leaves_owned;
#[cfg(feature = "cuda")]
use super::{
    commit_witness_stage_leaves_owned_with_leaf_hashes,
    extend_witness_stage_leaves_with_leaf_hashes,
};
use super::{
    decode_witness_stage_leaf_values, extend_witness_stage_leaves, WitnessStageExtendedValues,
    WitnessTraceCommitmentError, WitnessTraceCommitments,
};

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
        let commitment = commit_extended_witness_stage(&stage, source_bits, target_bits, arity)?;
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
                    let commitment =
                        commit_extended_witness_stage(&stage, source_bits, target_bits, arity)?;
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

fn commit_extended_witness_stage(
    stage: &crate::witness_layout::WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
) -> Result<super::WitnessStageCommitment, WitnessTraceCommitmentError> {
    #[cfg(feature = "cuda")]
    {
        let (leaves, leaf_hashes) =
            extend_witness_stage_leaves_with_leaf_hashes(stage, source_bits, target_bits, arity)?;
        commit_witness_stage_leaves_owned_with_leaf_hashes(leaves, arity, leaf_hashes)
            .map_err(WitnessTraceCommitmentError::from)
    }

    #[cfg(not(feature = "cuda"))]
    {
        let leaves = extend_witness_stage_leaves(stage, source_bits, target_bits)?;
        commit_witness_stage_leaves_owned(leaves, arity).map_err(WitnessTraceCommitmentError::from)
    }
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

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::{commit_extended_witness_stage, extend_witness_stage_leaves};
    use crate::witness_commitment::commit_witness_stage_leaves;
    use crate::witness_layout::derive_witness_trace_layout;
    use crate::witness_trace::WitnessTraceBuffer;
    use crate::{KeyUnitKind, PcsFriLayer, ProveUnitSchedule};
    use lzvm_field::Felt;

    #[test]
    fn cuda_combined_witness_stage_commitment_matches_separate_path() {
        assert_combined_witness_stage_commitment_matches_separate_path(3);
        assert_combined_witness_stage_commitment_matches_separate_path(6);
    }

    fn assert_combined_witness_stage_commitment_matches_separate_path(width: u32) {
        let unit = sample_unit(4, vec![width]);
        let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
        let value_count = 4 * usize::try_from(width).expect("stage width fits");
        let values = (0..value_count)
            .map(|value| Felt::from_u64(value as u64))
            .collect::<Vec<_>>();
        let trace = WitnessTraceBuffer::from_values(4, usize::try_from(width).unwrap(), values)
            .expect("trace shape should be valid");
        let stage = layout
            .stage_trace(&trace, 1)
            .expect("stage should be present");
        let source_bits = usize::try_from(unit.base_domain_bits).expect("source bits fit");
        let target_bits = usize::try_from(unit.extended_domain_bits).expect("target bits fit");
        let arity = usize::try_from(unit.merkle_tree_arity).expect("arity fits");

        let separate_leaves =
            extend_witness_stage_leaves(&stage, source_bits, target_bits).expect("stage extends");
        let separate = commit_witness_stage_leaves(&separate_leaves, arity)
            .expect("separate commitment should build");
        let combined = commit_extended_witness_stage(&stage, source_bits, target_bits, arity)
            .expect("combined commitment should build");

        assert_eq!(combined.stage_index(), separate.stage_index());
        assert_eq!(combined.arity(), separate.arity());
        assert_eq!(combined.root(), separate.root());
        assert_eq!(combined.tree_bytes(), separate.tree_bytes());
    }

    fn sample_unit(rows: u64, stage_commit_widths: Vec<u32>) -> ProveUnitSchedule {
        let mut transcript_root_challenge_draws = vec![1; stage_commit_widths.len()];
        if let Some(first) = transcript_root_challenge_draws.first_mut() {
            *first = 2;
        }
        ProveUnitSchedule {
            kind: KeyUnitKind::Basic,
            group_id: Some(0),
            unit_id: Some(0),
            group_name: Some("group-a".to_owned()),
            unit_name: Some("unit-a".to_owned()),
            base_domain_bits: 2,
            extended_domain_bits: 3,
            base_domain_size: rows,
            extended_domain_size: 8,
            blowup_factor: 2,
            query_count: 2,
            proof_of_work_bits: 0,
            merkle_tree_arity: 4,
            last_level_verification: 0,
            transcript_arity: Some(4),
            hash_commits: true,
            transcript_root_challenge_draws,
            challenge_count: 6,
            evaluation_value_count: 2,
            evaluation_map: Vec::new(),
            transcript_evaluation_challenge_draws: 2,
            constant_width: 1,
            stage_commit_widths,
            commitment_columns: Vec::new(),
            unit_value_map: Vec::new(),
            group_value_map: Vec::new(),
            opening_points: vec![0],
            fri_layers: vec![PcsFriLayer {
                input_bits: 3,
                output_bits: 1,
                folding_factor: 4,
            }],
            final_layer_bits: 1,
            fixed_bytes: 16,
            constant_tree_root: None,
            pcs_material_bytes: None,
            pcs_material_plan_digest: None,
            pcs_material_fixed_column_digest: None,
            pcs_material_constant_tree_digest: None,
            pcs_material_constant_tree_root: None,
            pcs_material_fixed_byte_count: None,
            pcs_material_constant_tree_byte_count: None,
            pcs_material_leaf_byte_count: None,
            pcs_material_node_byte_count: None,
        }
    }
}

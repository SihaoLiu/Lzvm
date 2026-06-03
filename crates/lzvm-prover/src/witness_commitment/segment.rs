use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, witness_commitment_segment_id, WitnessCommitmentSegment,
    WitnessCommitmentSegmentIdentity, WitnessCommitmentStageSegment,
};

use crate::witness_execution::ProveWitnessCommitments;

use super::ProveWitnessSegmentError;

pub fn build_witness_commitment_segment(
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessSegmentError> {
    if output.trace_instance_index() != 0 {
        return Err(ProveWitnessSegmentError::UnsupportedTraceInstance {
            unit_index: output.unit_index(),
            trace_instance_index: output.trace_instance_index(),
        });
    }
    let unit_count = output
        .unit_index()
        .checked_add(1)
        .ok_or(ProveWitnessSegmentError::LengthOverflow)?;
    build_witness_commitment_segment_for_schedule(unit_count, output)
}

pub fn build_witness_commitment_segment_for_schedule(
    unit_count: usize,
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessSegmentError> {
    let unit_count =
        u32::try_from(unit_count).map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
    let unit_index =
        u32::try_from(output.unit_index()).map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
    let id = witness_commitment_segment_id(
        unit_count,
        WitnessCommitmentSegmentIdentity {
            unit_index,
            trace_instance_index: output.trace_instance_index(),
        },
    )?;
    let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
    for commitment in output.stage_commitments().commitments() {
        let stage_index = u32::try_from(commitment.stage_index())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let arity = u32::try_from(commitment.arity())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let tree_byte_count = u64::try_from(commitment.tree_byte_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        stages.push(WitnessCommitmentStageSegment {
            stage_index,
            arity,
            root: commitment.root().map(|value| value.to_u64()),
            tree_byte_count,
            tree_digest: [0; 32],
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

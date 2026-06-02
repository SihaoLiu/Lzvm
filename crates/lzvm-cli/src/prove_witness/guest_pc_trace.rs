use lzvm_prover::{
    run_prove_witness_commitments_with_guest_pc_trace_segment_commitments,
    run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_with_timings,
    run_prove_witness_commitments_with_guest_pc_trace_segments, ProveExecutionPlan,
    ProveWitnessAuxiliaryInputs, ProveWitnessCommitmentError, ProveWitnessGuestPcTraceTiming,
    ProveWitnessTraceCommitments,
};

pub(super) struct GuestPcTraceWitnessRun {
    pub(super) outputs: Vec<ProveWitnessTraceCommitments>,
    pub(super) timing: Option<ProveWitnessGuestPcTraceTiming>,
}

pub(super) fn run_guest_pc_trace_witness(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    instruction_limit: u64,
    trace_can_be_dropped: bool,
    timings_enabled: bool,
) -> Result<GuestPcTraceWitnessRun, ProveWitnessCommitmentError> {
    let mut timing = None;
    let outputs = if trace_can_be_dropped {
        if timings_enabled {
            let mut observe_timing = |observed| {
                timing = Some(observed);
            };
            run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_with_timings(
                plan,
                unit_index,
                auxiliary_inputs,
                instruction_limit,
                &mut observe_timing,
            )?
        } else {
            run_prove_witness_commitments_with_guest_pc_trace_segment_commitments(
                plan,
                unit_index,
                auxiliary_inputs,
                instruction_limit,
            )?
        }
    } else {
        run_prove_witness_commitments_with_guest_pc_trace_segments(
            plan,
            unit_index,
            auxiliary_inputs,
            instruction_limit,
        )?
    };
    Ok(GuestPcTraceWitnessRun { outputs, timing })
}

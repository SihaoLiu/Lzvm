use lzvm_prover::{
    run_prove_witness_commitments_with_guest_pc_trace_segment_commitments,
    run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_with_timings,
    run_prove_witness_commitments_with_guest_pc_trace_segments, ProveExecutionPlan,
    ProveWitnessAuxiliaryInputs, ProveWitnessCommitmentError, ProveWitnessGuestPcTraceTiming,
    ProveWitnessTraceCommitments,
};

use super::timing::TimingRecorder;

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

pub(super) fn record_guest_pc_trace_timing(
    timings: &mut TimingRecorder,
    timing: ProveWitnessGuestPcTraceTiming,
) {
    timings.record("guest_trace_stream", timing.guest_trace_stream_duration());
    timings.record(
        "guest_segment_commit",
        timing.guest_segment_commit_duration(),
    );
    timings.record(
        "guest_regular_constraints",
        timing.guest_regular_constraint_duration(),
    );
    timings.record("guest_regular_hints", timing.guest_regular_hint_duration());
    timings.record("guest_stage_commit", timing.guest_stage_commit_duration());
    timings.record(
        "guest_stage_trace_extract",
        timing.guest_stage_trace_extract_duration(),
    );
    timings.record(
        "guest_stage_leaf_extend_work",
        timing.guest_stage_leaf_extend_work_duration(),
    );
    timings.record(
        "guest_stage_leaf_setup_work",
        timing.guest_stage_leaf_setup_work_duration(),
    );
    timings.record(
        "guest_stage_leaf_upload_work",
        timing.guest_stage_leaf_upload_work_duration(),
    );
    timings.record(
        "guest_stage_leaf_kernel_work",
        timing.guest_stage_leaf_kernel_work_duration(),
    );
    timings.record(
        "guest_stage_leaf_download_work",
        timing.guest_stage_leaf_download_work_duration(),
    );
    timings.record(
        "guest_stage_leaf_validate_work",
        timing.guest_stage_leaf_validate_work_duration(),
    );
    timings.record(
        "guest_stage_leaf_hash_work",
        timing.guest_stage_leaf_hash_work_duration(),
    );
    timings.record(
        "guest_stage_tree_commit_work",
        timing.guest_stage_tree_commit_work_duration(),
    );
}

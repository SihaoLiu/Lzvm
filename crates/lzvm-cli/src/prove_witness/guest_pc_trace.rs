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
    timings.record_count("guest_segment_count", timing.segment_count());
    timings.record("guest_trace_stream", timing.guest_trace_stream_duration());
    timings.record(
        "guest_segment_commit",
        timing.guest_segment_commit_duration(),
    );
    timings.record("guest_trace_runner", timing.guest_trace_runner_duration());
    timings.record("guest_trace_lowerer", timing.guest_trace_lowerer_duration());
    timings.record("guest_trace_lower", timing.guest_trace_lower_duration());
    timings.record(
        "guest_trace_pending_send_wait",
        timing.guest_trace_pending_send_wait_duration(),
    );
    timings.record(
        "guest_trace_pending_receive_wait",
        timing.guest_trace_pending_receive_wait_duration(),
    );
    timings.record(
        "guest_trace_segment_send_wait",
        timing.guest_trace_segment_send_wait_duration(),
    );
    timings.record(
        "guest_trace_segment_receive_wait",
        timing.guest_trace_segment_receive_wait_duration(),
    );
    timings.record(
        "guest_device_source_build",
        timing.guest_device_source_build_duration(),
    );
    timings.record(
        "guest_device_source_descriptor_upload",
        timing.guest_device_source_descriptor_upload_duration(),
    );
    timings.record_count(
        "guest_device_source_descriptor_upload_bytes",
        timing.guest_device_source_descriptor_upload_byte_count(),
    );
    timings.record_count(
        "guest_device_source_descriptor_upload_rows",
        timing.guest_device_source_descriptor_upload_row_count(),
    );
    timings.record(
        "guest_device_source_trace_expand",
        timing.guest_device_source_trace_expand_duration(),
    );
    timings.record_count(
        "guest_stage_source_retention_attempts",
        timing.guest_stage_source_retention_attempt_count(),
    );
    timings.record_count(
        "guest_stage_source_retention_retained",
        timing.guest_stage_source_retention_retained_count(),
    );
    timings.record_count(
        "guest_stage_source_retention_rejected",
        timing.guest_stage_source_retention_rejected_count(),
    );
    timings.record_count(
        "guest_stage_source_retention_rejected_bytes",
        timing.guest_stage_source_retention_rejected_byte_count(),
    );
    timings.record_count(
        "guest_stage_source_retention_limit_bytes",
        timing.guest_stage_source_retention_limit_byte_count(),
    );
    timings.record_count(
        "guest_descriptor_buffer_retention_attempts",
        timing.guest_descriptor_buffer_retention_attempt_count(),
    );
    timings.record_count(
        "guest_descriptor_buffer_retention_retained",
        timing.guest_descriptor_buffer_retention_retained_count(),
    );
    timings.record_count(
        "guest_descriptor_buffer_retention_rejected",
        timing.guest_descriptor_buffer_retention_rejected_count(),
    );
    timings.record_count(
        "guest_descriptor_buffer_retention_retained_bytes",
        timing.guest_descriptor_buffer_retention_retained_byte_count(),
    );
    timings.record_count(
        "guest_descriptor_buffer_retention_rejected_bytes",
        timing.guest_descriptor_buffer_retention_rejected_byte_count(),
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
    timings.record_count(
        "guest_stage_leaf_hash_rows",
        timing.guest_stage_leaf_hash_row_count(),
    );
    timings.record_count(
        "guest_stage_leaf_hash_bytes",
        timing.guest_stage_leaf_hash_byte_count(),
    );
    timings.record_count(
        "guest_stage_leaf_hash_arity2_rows",
        timing.guest_stage_leaf_hash_arity2_row_count(),
    );
    timings.record_count(
        "guest_stage_leaf_hash_arity2_bytes",
        timing.guest_stage_leaf_hash_arity2_byte_count(),
    );
    timings.record_count(
        "guest_stage_leaf_hash_arity4_rows",
        timing.guest_stage_leaf_hash_arity4_row_count(),
    );
    timings.record_count(
        "guest_stage_leaf_hash_arity4_bytes",
        timing.guest_stage_leaf_hash_arity4_byte_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_calls",
        timing.guest_stage_leaf_coset_extend_call_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_output_bytes",
        timing.guest_stage_leaf_coset_extend_output_byte_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_columns",
        timing.guest_stage_leaf_coset_extend_column_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_max_columns",
        timing.guest_stage_leaf_coset_extend_max_column_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_ntt_launches",
        timing.guest_stage_leaf_coset_extend_ntt_launch_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_bit_reverse_launches",
        timing.guest_stage_leaf_coset_extend_bit_reverse_launch_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_ntt_stage_launches",
        timing.guest_stage_leaf_coset_extend_ntt_stage_launch_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_ntt_block_twiddle_launches",
        timing.guest_stage_leaf_coset_extend_ntt_block_twiddle_launch_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_normalize_launches",
        timing.guest_stage_leaf_coset_extend_normalize_launch_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_pack_launches",
        timing.guest_stage_leaf_coset_extend_pack_launch_count(),
    );
    timings.record_count(
        "guest_stage_leaf_coset_extend_unpack_launches",
        timing.guest_stage_leaf_coset_extend_unpack_launch_count(),
    );
    timings.record(
        "guest_stage_tree_commit_work",
        timing.guest_stage_tree_commit_work_duration(),
    );
    for stage_timing in timing.guest_stage_timings() {
        let stage_index = stage_timing.stage_index();
        timings.record_dynamic(
            format!("guest_stage_{stage_index}_leaf_extend_work"),
            stage_timing.leaf_extend_work_duration(),
        );
        timings.record_dynamic(
            format!("guest_stage_{stage_index}_leaf_setup_work"),
            stage_timing.leaf_setup_work_duration(),
        );
        timings.record_dynamic(
            format!("guest_stage_{stage_index}_leaf_upload_work"),
            stage_timing.leaf_upload_work_duration(),
        );
        timings.record_dynamic(
            format!("guest_stage_{stage_index}_leaf_kernel_work"),
            stage_timing.leaf_kernel_work_duration(),
        );
        timings.record_dynamic(
            format!("guest_stage_{stage_index}_leaf_download_work"),
            stage_timing.leaf_download_work_duration(),
        );
        timings.record_dynamic(
            format!("guest_stage_{stage_index}_leaf_validate_work"),
            stage_timing.leaf_validate_work_duration(),
        );
        timings.record_dynamic(
            format!("guest_stage_{stage_index}_leaf_hash_work"),
            stage_timing.leaf_hash_work_duration(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_hash_arity2_rows"),
            stage_timing.leaf_hash_arity2_row_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_hash_arity2_bytes"),
            stage_timing.leaf_hash_arity2_byte_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_hash_arity4_rows"),
            stage_timing.leaf_hash_arity4_row_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_hash_arity4_bytes"),
            stage_timing.leaf_hash_arity4_byte_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_calls"),
            stage_timing.leaf_coset_extend_call_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_output_bytes"),
            stage_timing.leaf_coset_extend_output_byte_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_columns"),
            stage_timing.leaf_coset_extend_column_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_max_columns"),
            stage_timing.leaf_coset_extend_max_column_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_ntt_launches"),
            stage_timing.leaf_coset_extend_ntt_launch_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_bit_reverse_launches"),
            stage_timing.leaf_coset_extend_bit_reverse_launch_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_ntt_stage_launches"),
            stage_timing.leaf_coset_extend_ntt_stage_launch_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_ntt_block_twiddle_launches"),
            stage_timing.leaf_coset_extend_ntt_block_twiddle_launch_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_normalize_launches"),
            stage_timing.leaf_coset_extend_normalize_launch_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_pack_launches"),
            stage_timing.leaf_coset_extend_pack_launch_count(),
        );
        timings.record_count_dynamic(
            format!("guest_stage_{stage_index}_leaf_coset_extend_unpack_launches"),
            stage_timing.leaf_coset_extend_unpack_launch_count(),
        );
        timings.record_dynamic(
            format!("guest_stage_{stage_index}_tree_commit_work"),
            stage_timing.tree_commit_work_duration(),
        );
    }
}

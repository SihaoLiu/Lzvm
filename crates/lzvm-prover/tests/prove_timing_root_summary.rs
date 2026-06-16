use std::process::Command;

#[test]
fn prove_timing_root_summary_reports_root_grouping_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let source = std::fs::read_to_string(&script_path)
        .expect("prove timing root summary source should read");

    for required in [
        "timing_guest_stage_tree_commit_root_count",
        "timing_guest_stage_tree_commit_root_materialization_groups",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size",
        "timing_guest_stage_leaf_kernel_work_ms",
        "timing_guest_stage_leaf_coset_extend_calls",
        "timing_guest_stage_leaf_coset_extend_columns",
        "timing_guest_stage_leaf_coset_extend_ntt_launches",
        "timing_guest_stage_leaf_coset_extend_ntt_stage_launches",
        "timing_guest_stage_leaf_coset_extend_ntt_block_twiddle_launches",
        "timing_cuda_direct_copy_d2h_wait_ns",
        "timing_guest_trace_runner_ms",
        "timing_guest_trace_lowerer_ms",
        "timing_guest_trace_stream_elapsed_ms",
        "timing_guest_trace_stream_ms",
        "timing_guest_segment_commit_ms",
        "timing_guest_trace_segment_receive_wait_ms",
        "timing_guest_trace_pending_receive_wait_ms",
        "timing_guest_trace_pending_send_wait_ms",
        "timing_guest_trace_parallel_lower_workers",
        "timing_guest_trace_parallel_lower_dispatched",
        "timing_guest_trace_parallel_lower_received",
        "timing_guest_trace_parallel_lower_emitted",
        "timing_guest_trace_parallel_lower_max_reorder",
        "timing_guest_trace_seed_direct_lift_attempts",
        "timing_guest_trace_seed_direct_lift_successes",
        "timing_guest_trace_seed_full_advances",
        "timing_finish_witness_opening_ms",
        "needs_cross_segment_root_pipeline",
        "leaf_launch_pressure",
        "primary_bottleneck",
        "trace_to_leaf_ratio",
        "stream_commit_residual_ms",
    ] {
        assert!(
            source.contains(required),
            "prove timing root summary should expose {required}"
        );
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .output()
        .expect("prove timing root summary self-test should run");

    assert!(
        output.status.success(),
        "prove timing root summary self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    for required in [
        "profile,total_ms,runner_ms,lowerer_ms,stream_elapsed_ms,stream_worker_ms,segment_commit_ms,stream_commit_residual_ms,segment_receive_wait_ms,pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,parallel_lower_max_reorder,seed_direct_lift_attempts,seed_direct_lift_successes,seed_full_advances,finish_opening_ms,root_count,materialization_groups,materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline,leaf_kernel_ms,leaf_coset_calls,leaf_coset_columns,leaf_ntt_launches,leaf_ntt_stage_launches,leaf_ntt_block_twiddle_launches,leaf_ntt_launches_per_call,direct_d2h_wait_ms,leaf_launch_pressure,trace_to_leaf_ratio,primary_bottleneck",
        "single-root-groups,9050,7800,7812,9912,7812,2100,0,6000,1200,345,2,23,23,23,1,22,22,1,476,23,23,1,1.000,yes,858,23,874,41078,15732,23598,1786.000,192.974,yes,9.105,stream_elapsed",
        "batched-roots,9050,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,23,1,23,23.000,no,0,0,0,0,0,0,0.000,0.000,no,0.000,total",
    ] {
        assert!(
            stdout.contains(required),
            "prove timing root summary should print {required}"
        );
    }
}

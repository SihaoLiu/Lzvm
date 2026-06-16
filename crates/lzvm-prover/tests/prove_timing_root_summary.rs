use std::io::Write;
use std::process::{Command, Stdio};

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
        "timing_guest_trace_descriptor_rows",
        "timing_guest_trace_descriptor_compact_rows",
        "timing_guest_trace_descriptor_wide_rows",
        "timing_guest_device_source_descriptor_upload_bytes",
        "timing_guest_device_source_descriptor_upload_rows",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_values",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_rows",
        "descriptor_shape_hint",
        "timing_guest_trace_seed_direct_lift_attempts",
        "timing_guest_trace_seed_direct_lift_successes",
        "timing_guest_trace_seed_full_advances",
        "timing_finish_witness_opening_ms",
        "timing_finish_witness_opening_query_unit_count",
        "timing_finish_witness_opening_single_query_unit_count",
        "timing_finish_witness_opening_retained_leaf_digest_openings",
        "timing_finish_witness_opening_retained_leaf_digest_rows",
        "timing_finish_witness_opening_retained_leaf_digest_all_single_row_openings",
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_launches",
        "timing_constant_material_validation_elapsed_ms",
        "timing_constant_material_validation_join_wait_ms",
        "constant_material_validation_overlap_hint",
        "input_bytes",
        "needs_cross_segment_root_pipeline",
        "opening_batching_hint",
        "leaf_launch_pressure",
        "primary_bottleneck",
        "trace_to_leaf_ratio",
        "root_pipeline_policy_hint",
        "large_input_root_pipeline_gated",
        "stream_commit_residual_ms",
        "AGGREGATE_HEADER",
        "sample_spread_pct",
        "close_samples",
        "max_outlier",
        "perf_lowered_report_row_self_pct",
        "perf_memmove_self_pct",
        "perf_memmove_guest_machine_pct",
        "perf_memmove_trace_slice_pct",
        "perf_memmove_source_hint",
        "perf_pending_segment_drop_self_pct",
        "perf_sha256_self_pct",
        "perf_sha256_source_hint",
        "cpu_trace_hotspot_hint",
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
        "profile,input_bytes,total_ms,constant_material_validation_elapsed_ms,constant_material_validation_join_wait_ms,constant_material_validation_overlap_hint,runner_ms,lowerer_ms,stream_elapsed_ms,stream_worker_ms,segment_commit_ms,stream_commit_residual_ms,segment_receive_wait_ms,pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,parallel_lower_max_reorder,descriptor_rows,descriptor_compact_rows,descriptor_wide_rows,descriptor_upload_bytes,descriptor_bytes_per_row,descriptor_high32_nonzero_values,descriptor_high32_nonzero_rows,descriptor_high32_row_pct,descriptor_shape_hint,seed_direct_lift_attempts,seed_direct_lift_successes,seed_full_advances,finish_opening_ms,opening_query_units,opening_single_query_units,retained_leaf_openings,retained_leaf_rows,retained_leaf_all_single_row,retained_leaf_path_launches,opening_batching_hint,root_count,materialization_groups,materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline,root_pipeline_policy_hint,leaf_kernel_ms,leaf_coset_calls,leaf_coset_columns,leaf_ntt_launches,leaf_ntt_stage_launches,leaf_ntt_block_twiddle_launches,leaf_ntt_launches_per_call,direct_d2h_wait_ms,leaf_launch_pressure,trace_to_leaf_ratio,primary_bottleneck,perf_lowered_report_row_self_pct,perf_memmove_self_pct,perf_memmove_guest_machine_pct,perf_memmove_trace_slice_pct,perf_memmove_source_hint,perf_pending_segment_drop_self_pct,perf_sha256_self_pct,perf_sha256_source_hint,cpu_trace_hotspot_hint",
        "single-root-groups,2758032,9050,0,0,none,7800,7812,9912,7812,2100,0,6000,1200,345,2,23,23,23,1,1000,1000,0,88000,88.000,6,4,0.400,high32_sparse_compact_descriptor,22,22,1,476,23,23,23,23,yes,276,cross_segment_retained_leaf_opening_candidate,23,23,1,1.000,yes,enable_cross_segment_root_pipeline,858,23,874,41078,15732,23598,1786.000,192.974,yes,9.105,stream_elapsed,26.350,20.940,10.610,8.670,guest_machine_and_trace_slice,7.410,23.170,sha256_digest_unresolved,report_lifetime_and_data_movement",
        "batched-roots,2758032,9050,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0.000,none,0,0,0,0,0,0,0,0,no,0,none,23,1,23,23.000,no,root_batches_already_grouped,0,0,0,0,0,0,0.000,0.000,no,0.000,total,0.000,0.000,0.000,0.000,none,0.000,0.000,none,none",
        "slow-sample,12447640,18100,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0.000,none,0,0,0,0,0,0,0,0,no,0,none,120,120,1,1.000,yes,large_input_root_pipeline_gated,0,0,0,0,0,0,0.000,0.000,no,0.000,total,0.000,0.000,0.000,0.000,none,0.000,0.000,none,none",
        "aggregate,total_count,valid_total_count,total_min_ms,total_mean_ms,total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier",
        "aggregate,3,3,9050,12066.667,9050.000,18100,100.000,no,yes",
    ] {
        assert!(
            stdout.contains(required),
            "prove timing root summary should print {required}"
        );
    }
}

#[test]
fn prove_timing_root_summary_uses_thread_name_for_memmove_source_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=1000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    21.23%    21.23%  lzvm-gp-runner  libc.so.6             [.] __memmove_avx512_unaligned_erms",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "stdin,0,1000,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0.000,none,0,0,0,0,0,0,0,0,no,0,none,1,1,1,1.000,no,none,0,0,0,0,0,0,0.000,0.000,no,0.000,total,0.000,21.230,0.000,0.000,guest_runner_thread,0.000,0.000,none,guest_state_copies"
        ),
        "prove timing root summary should classify command-column memmove source: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_constant_material_overlap() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=10000",
        "timing_constant_material_validation_elapsed_ms=9000",
        "timing_constant_material_validation_join_wait_ms=125",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "profile,input_bytes,total_ms,constant_material_validation_elapsed_ms,constant_material_validation_join_wait_ms,constant_material_validation_overlap_hint,"
        ),
        "prove timing root summary should expose constant material overlap columns: stdout={stdout}"
    );
    assert!(
        stdout.contains("stdin,0,10000,9000,125,mostly_overlapped,"),
        "prove timing root summary should classify background validation overlap: stdout={stdout}"
    );
}

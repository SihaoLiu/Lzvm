use std::io::Write;
use std::process::{Command, Stdio};

#[cfg(unix)]
#[test]
fn prove_timing_root_summary_script_is_directly_executable() {
    use std::os::unix::fs::PermissionsExt;

    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let mode = std::fs::metadata(&script_path)
        .expect("prove timing root summary metadata should read")
        .permissions()
        .mode();
    assert_ne!(
        mode & 0o111,
        0,
        "prove timing root summary should be executable as a profiling helper"
    );

    let output = Command::new(&script_path)
        .arg("--self-test")
        .output()
        .expect("prove timing root summary should run directly through its shebang");
    assert!(
        output.status.success(),
        "prove timing root summary direct self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

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
        "timing_cuda_direct_copy_d2h_hot_bytes",
        "timing_cuda_direct_copy_d2h_hot_count",
        "timing_cuda_direct_copy_d2h_hot_wait_ns",
        "timing_guest_trace_runner_ms",
        "timing_guest_trace_lowerer_ms",
        "timing_guest_trace_stream_elapsed_ms",
        "timing_guest_trace_stream_ms",
        "timing_guest_segment_commit_ms",
        "timing_guest_segment_commit_initial_workers",
        "timing_guest_segment_commit_effective_workers",
        "timing_guest_segment_commit_oom_retries",
        "timing_guest_trace_segment_receive_wait_ms",
        "timing_guest_trace_pending_receive_wait_ms",
        "timing_guest_trace_pending_send_wait_ms",
        "timing_guest_trace_parallel_lower_workers",
        "timing_guest_trace_parallel_lower_dispatched",
        "timing_guest_trace_parallel_lower_received",
        "timing_guest_trace_parallel_lower_emitted",
        "timing_guest_trace_parallel_lower_max_reorder",
        "timing_guest_trace_reports",
        "timing_guest_trace_report_rows",
        "timing_guest_trace_report_buffer_capacity",
        "timing_guest_trace_report_buffer_max_capacity",
        "timing_guest_trace_report_buffer_excess_capacity",
        "trace_report_buffer_shape_hint",
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
        "timing_finish_witness_opening_query_count",
        "timing_finish_witness_opening_max_queries_per_unit",
        "timing_finish_witness_opening_stage_count",
        "timing_finish_witness_opening_retained_source_count",
        "timing_finish_witness_opening_external_source_count",
        "timing_finish_witness_opening_embedded_source_count",
        "timing_finish_witness_opening_missing_source_count",
        "timing_finish_witness_opening_row_values_device_rows",
        "timing_finish_witness_opening_row_values_source_rows",
        "timing_finish_witness_opening_retained_leaf_digest_openings",
        "timing_finish_witness_opening_retained_leaf_digest_rows",
        "timing_finish_witness_opening_retained_leaf_digest_all_single_row_openings",
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_launches",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches",
        "timing_finish_witness_opening_path_parent_hash_launches_per_stage",
        "timing_finish_witness_opening_row_values_device_download_batches",
        "timing_constant_material_validation_elapsed_ms",
        "timing_constant_material_validation_join_wait_ms",
        "constant_material_validation_overlap_hint",
        "input_bytes",
        "needs_cross_segment_root_pipeline",
        "opening_batching_hint",
        "leaf_launch_pressure",
        "primary_bottleneck",
        "trace_structure_hint",
        "trace_to_leaf_ratio",
        "proof_12s_gap_ms",
        "proof_12s_gap_hint",
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
        "timing_guest_trace_report_detail_samples",
        "trace_report_detail_sample_hint",
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
        "profile,input_bytes,total_ms,constant_material_validation_elapsed_ms,constant_material_validation_join_wait_ms,constant_material_validation_overlap_hint,runner_ms,lowerer_ms,stream_elapsed_ms,stream_worker_ms,segment_commit_ms,segment_commit_initial_workers,segment_commit_effective_workers,segment_commit_oom_retries,stream_commit_residual_ms,segment_receive_wait_ms,pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,parallel_lower_max_reorder,trace_reports,trace_report_rows,trace_rows_per_report,trace_report_buffer_capacity,trace_report_buffer_max_capacity,trace_report_buffer_excess_capacity,trace_report_buffer_excess_pct,trace_report_buffer_shape_hint,trace_report_lifetime_hint,descriptor_rows,descriptor_compact_rows,descriptor_wide_rows,descriptor_upload_bytes,descriptor_bytes_per_row,descriptor_high32_nonzero_values,descriptor_high32_nonzero_rows,descriptor_high32_row_pct,descriptor_shape_hint,seed_direct_lift_attempts,seed_direct_lift_successes,seed_full_advances,finish_opening_ms,opening_query_units,opening_single_query_units,opening_queries,opening_max_queries_per_unit,opening_stage_count,opening_source_shape_hint,opening_row_value_device_rows,opening_row_value_source_rows,retained_leaf_openings,retained_leaf_rows,retained_leaf_all_single_row,retained_leaf_path_launches,retained_parent_checkpoint_openings,retained_parent_checkpoint_rows,retained_parent_checkpoint_all_single_row,retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches,opening_path_parent_hash_launches_per_stage,opening_row_value_device_download_batches,opening_batching_hint,root_count,materialization_groups,materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline,root_pipeline_policy_hint,leaf_kernel_ms,leaf_coset_calls,leaf_coset_columns,leaf_ntt_launches,leaf_ntt_stage_launches,leaf_ntt_block_twiddle_launches,leaf_ntt_launches_per_call,direct_d2h_wait_ms,leaf_launch_pressure,trace_to_leaf_ratio,primary_bottleneck,trace_structure_hint,proof_12s_gap_ms,proof_12s_gap_hint,perf_lowered_report_row_self_pct,perf_memmove_self_pct,perf_memmove_guest_machine_pct,perf_memmove_trace_slice_pct,perf_memmove_source_hint,perf_pending_segment_drop_self_pct,perf_sha256_self_pct,perf_sha256_source_hint,cpu_trace_hotspot_hint",
        "single-root-groups,2758032,9050,0,0,none,7800,7812,9912,7812,2100,2,2,0,0,6000,1200,345,2,23,23,23,1,93843537,93917088,1.001,94371840,4194304,528303,0.560,report_buffer_capacity_tight,tight_report_buffer_and_pending_drop,1000,1000,0,88000,88.000,6,4,0.400,high32_sparse_compact_descriptor,22,22,1,476,23,23,0,0,0,single_query_cross_root_with_no_sources,0,0,23,23,yes,276,0,0,no,0,0,0,0,0,0,0,0,cross_segment_retained_leaf_opening_candidate,23,23,1,1.000,yes,enable_cross_segment_root_pipeline,858,23,874,41078,15732,23598,1786.000,192.974,yes,9.105,stream_elapsed,parallel_lower_waiting,0,within_12s_target,26.350,20.940,10.610,8.670,guest_machine_and_trace_slice,7.410,23.170,sha256_digest_unresolved,report_lifetime_and_data_movement",
        "batched-roots,2758032,9050,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0,0.000,none,none,0,0,0,0,0.000,0,0,0.000,none,0,0,0,0,0,0,0,0,0,none,0,0,0,0,no,0,0,0,no,0,0,0,0,0,0,0,0,none,23,1,23,23.000,no,root_batches_already_grouped,0,0,0,0,0,0,0.000,0.000,no,0.000,total,none,0,within_12s_target,0.000,0.000,0.000,0.000,none,0.000,0.000,none,none",
        "slow-sample,12447640,18100,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0,0.000,none,none,0,0,0,0,0.000,0,0,0.000,none,0,0,0,0,0,0,0,0,0,none,0,0,0,0,no,0,0,0,no,0,0,0,0,0,0,0,0,none,120,120,1,1.000,yes,large_input_root_pipeline_gated,0,0,0,0,0,0,0.000,0.000,no,0.000,total,none,6100,target_gap_needs_timing_breakdown,0.000,0.000,0.000,0.000,none,0.000,0.000,none,none",
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
fn prove_timing_root_summary_reports_trace_report_detail_sample_coverage() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1000",
        "timing_guest_trace_report_detail_samples=10",
        "timing_guest_trace_report_sampled_ns=1000",
        "timing_guest_trace_report_lowering_sampled_ns=150",
        "timing_guest_trace_report_row_validation_sampled_ns=500",
        "timing_guest_trace_report_memory_columns_sampled_ns=60",
        "timing_guest_trace_report_source_values_sampled_ns=180",
        "timing_guest_trace_report_source_a_value_sampled_ns=140",
        "timing_guest_trace_report_source_b_value_sampled_ns=90",
        "timing_guest_trace_report_register_access_sampled_ns=100",
        "timing_guest_trace_report_memory_access_sampled_ns=80",
        "timing_guest_trace_report_precompile_memory_sampled_ns=20",
        "timing_guest_trace_report_visit_sampled_ns=200",
        "timing_guest_trace_descriptor_sampled_ns=50",
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
            "trace_report_detail_samples,trace_report_detail_sample_pct,trace_report_detail_sample_ppm,trace_report_detail_sample_hint,trace_report_detail_avg_ns,trace_report_detail_hotspot,trace_report_detail_hotspot_pct,trace_report_row_validation_hotspot,trace_report_row_validation_hotspot_pct,trace_report_row_validation_explained_pct,trace_report_row_validation_residual_pct,trace_report_source_values_lookup_pct,trace_report_source_values_residual_pct,trace_report_detail_visit_pct,trace_report_visit_descriptor_pct,trace_report_visit_residual_pct"
        ),
        "prove timing root summary should expose detail sample, source-value, and visit drilldown columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",10,1.000,10000.000,detail_timing_sampled,100,row_validation,50.000,source_a_value,28.000,98.000,2.000,127.778,0.000,20.000,25.000,75.000"
        ),
        "prove timing root summary should classify sampled detail, source-value lookup coverage, row-validation, and visit hotspots: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_direct_d2h_hot_copy_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=58552",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=4156548184",
        "timing_cuda_direct_copy_d2h_hot_bytes=1152",
        "timing_cuda_direct_copy_d2h_hot_count=41",
        "timing_cuda_direct_copy_d2h_hot_wait_ns=3389722844",
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
        stdout.contains("direct_d2h_hot_bytes,direct_d2h_hot_count,direct_d2h_hot_wait_ms"),
        "prove timing root summary should expose hot direct D2H copy shape: stdout={stdout}"
    );
    assert!(
        stdout.contains(",1152,41,3389.723"),
        "prove timing root summary should report the dominant direct D2H wait bucket: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_shape_counts() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1100",
        "timing_guest_trace_single_row_reports=900",
        "timing_guest_trace_multi_row_reports=100",
        "timing_guest_trace_pending_dma_reports=50",
        "timing_guest_trace_amo_reports=25",
        "timing_guest_trace_store_conditional_reports=10",
        "timing_guest_trace_external_op_rows=300",
        "timing_guest_trace_copy_rows=400",
        "timing_guest_trace_flag_rows=20",
        "timing_guest_trace_precompile_rows=8",
        "timing_guest_trace_indirect_memory_rows=500",
        "timing_guest_trace_register_source_reads=1400",
        "timing_guest_trace_memory_source_reads=300",
        "timing_guest_trace_register_store_rows=700",
        "timing_guest_trace_memory_store_rows=200",
        "timing_guest_trace_no_store_rows=100",
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
            "single_row_reports,multi_row_reports,pending_dma_reports,amo_reports,store_conditional_reports,external_op_rows,copy_rows,flag_rows,precompile_rows,indirect_memory_rows,indirect_memory_row_pct,register_source_reads,memory_source_reads,memory_source_read_pct,register_store_rows,memory_store_rows,memory_store_row_pct,no_store_rows,no_store_row_pct,trace_shape_sample_hint"
        ),
        "prove timing root summary should expose trace shape columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",900,100,50,25,10,300,400,20,8,500,45.455,1400,300,27.273,700,200,18.182,100,9.091,shape_timing_enabled,"
        ),
        "prove timing root summary should classify trace shape ratios: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_marks_trace_shape_timing_disabled_or_zero() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1100",
        "timing_guest_trace_single_row_reports=0",
        "timing_guest_trace_indirect_memory_rows=0",
        "timing_guest_trace_memory_source_reads=0",
        "timing_guest_trace_memory_store_rows=0",
        "timing_guest_trace_no_store_rows=0",
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
            ",0,0.000,0,0,0.000,0,0,0.000,0,0.000,shape_timing_disabled_or_zero,"
        ),
        "prove timing root summary should say shape timing is disabled instead of implying zero-shape rows: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_tiny_detail_sample_coverage_ppm() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=1000000",
        "timing_guest_trace_report_rows=1000000",
        "timing_guest_trace_report_detail_samples=1",
        "timing_guest_trace_report_sampled_ns=1000",
        "timing_guest_trace_report_row_validation_sampled_ns=500",
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
        stdout.contains("trace_report_detail_sample_pct,trace_report_detail_sample_ppm,"),
        "prove timing root summary should expose ppm coverage next to percent coverage: stdout={stdout}"
    );
    assert!(
        stdout.contains(",1,0.000,1.000,detail_timing_sampled,1000,"),
        "prove timing root summary should preserve tiny sampled coverage that rounds to zero percent: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_retained_parent_checkpoint_opening_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52335",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_retained_leaf_digest_openings=77",
        "timing_finish_witness_opening_retained_leaf_digest_rows=77",
        "timing_finish_witness_opening_retained_leaf_digest_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_launches=0",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
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
            "retained_parent_checkpoint_openings,retained_parent_checkpoint_rows,retained_parent_checkpoint_all_single_row,retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches"
        ),
        "prove timing root summary should expose retained parent checkpoint opening shape: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",77,77,yes,0,79,79,yes,0,0,79,0,0,790,0,0,cross_segment_retained_parent_checkpoint_opening_candidate,"
        ),
        "prove timing root summary should classify single-query retained parent checkpoint openings as a cross-unit batching target: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_opening_query_unit_scope() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52335",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_stage_count=240",
        "timing_finish_witness_opening_retained_source_count=77",
        "timing_finish_witness_opening_external_source_count=79",
        "timing_finish_witness_opening_embedded_source_count=84",
        "timing_finish_witness_opening_missing_source_count=0",
        "timing_finish_witness_opening_row_values_device_rows=79",
        "timing_finish_witness_opening_row_values_source_rows=77",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
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
            "opening_queries,opening_max_queries_per_unit,opening_stage_count,opening_source_shape_hint,opening_row_value_device_rows,opening_row_value_source_rows"
        ),
        "prove timing root summary should expose opening query-unit scope columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",120,120,120,1,240,single_query_cross_root_with_mixed_sources,79,77,"
        ),
        "prove timing root summary should classify single-query cross-root opening shape: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_opening_parent_hash_work_scope() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=58552",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_stage_count=240",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows=165675008",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes=21206401024",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows=13808034",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes=1767428352",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_launches_per_stage=5",
        "timing_finish_witness_opening_row_values_device_download_batches=43",
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
            "retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches,opening_path_parent_hash_launches_per_stage,opening_row_value_device_download_batches"
        ),
        "prove timing root summary should expose opening parent-hash work scope columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",79,79,yes,165675008,21206401024,79,13808034,1767428352,790,5,43,"
        ),
        "prove timing root summary should report retained parent checkpoint work scope: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_report_buffer_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_rows=93917088",
        "timing_guest_trace_report_buffer_capacity=94371840",
        "timing_guest_trace_report_buffer_max_capacity=4194304",
        "timing_guest_trace_report_buffer_excess_capacity=528303",
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
            "trace_reports,trace_report_rows,trace_rows_per_report,trace_report_buffer_capacity,trace_report_buffer_max_capacity,trace_report_buffer_excess_capacity,trace_report_buffer_excess_pct,trace_report_buffer_shape_hint,"
        ),
        "prove timing root summary should expose trace report buffer columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "93843537,93917088,1.001,94371840,4194304,528303,0.560,report_buffer_capacity_tight,"
        ),
        "prove timing root summary should classify tight report buffer capacity: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_report_lifetime_pressure() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_rows=93917088",
        "timing_guest_trace_report_buffer_capacity=94371840",
        "timing_guest_trace_report_buffer_max_capacity=4194304",
        "timing_guest_trace_report_buffer_excess_capacity=528303",
        "     7.41%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
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
        stdout.contains("trace_report_lifetime_hint,"),
        "prove timing root summary should expose trace report lifetime hint: stdout={stdout}"
    );
    assert!(
        stdout.contains("tight_report_buffer_and_pending_drop"),
        "prove timing root summary should classify report lifetime pressure: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_serial_trace_structure_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_trace_runner_ms=7600",
        "timing_guest_trace_lowerer_ms=7900",
        "timing_guest_trace_stream_elapsed_ms=8200",
        "timing_guest_trace_stream_ms=6100",
        "timing_guest_segment_commit_ms=900",
        "timing_guest_trace_segment_receive_wait_ms=6000",
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_stage_leaf_kernel_work_ms=850",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=23",
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
        stdout.contains("primary_bottleneck,trace_structure_hint,"),
        "prove timing root summary should expose trace structure hint: stdout={stdout}"
    );
    assert!(
        stdout.contains("stream_elapsed,trace_stream_cpu_floor,"),
        "prove timing root summary should classify serial CPU trace structure: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_twelve_second_gap_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=55200",
        "timing_guest_trace_runner_ms=42100",
        "timing_guest_trace_lowerer_ms=43800",
        "timing_guest_trace_stream_elapsed_ms=44100",
        "timing_guest_trace_stream_ms=39700",
        "timing_guest_segment_commit_ms=10700",
        "timing_finish_witness_opening_ms=3200",
        "timing_guest_stage_leaf_kernel_work_ms=6200",
        "timing_guest_stage_tree_commit_root_count=79",
        "timing_guest_stage_tree_commit_root_materialization_groups=79",
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
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("summary should include {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should include {name}: stdout={stdout}"))
    };
    assert_eq!(value("proof_12s_gap_ms"), "43200");
    assert_eq!(
        value("proof_12s_gap_hint"),
        "cpu_trace_generation_above_target"
    );
}

#[test]
fn prove_timing_root_summary_distinguishes_disabled_high32_stats_from_zero_high32_stats() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let disabled_input = [
        "timing_total_ms=8000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_descriptor_rows=1000",
        "timing_guest_trace_descriptor_compact_rows=1000",
        "timing_guest_trace_descriptor_wide_rows=0",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_values=0",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_rows=0",
    ]
    .join("\n");
    let enabled_zero_input = [
        "timing_total_ms=8000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_descriptor_rows=1000",
        "timing_guest_trace_descriptor_compact_rows=1000",
        "timing_guest_trace_descriptor_wide_rows=0",
        "timing_guest_trace_descriptor_high32_stats_enabled=1",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_values=0",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_rows=0",
    ]
    .join("\n");

    for (label, input, expected_hint) in [
        (
            "disabled",
            disabled_input,
            "compact_descriptor_no_high32_stats",
        ),
        (
            "enabled-zero",
            enabled_zero_input,
            "high32_zero_compact_descriptor",
        ),
    ] {
        let mut child = Command::new("python3")
            .arg(&script_path)
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("{label} summary should spawn: {error}"));
        child
            .stdin
            .as_mut()
            .expect("stdin should be open")
            .write_all(input.as_bytes())
            .expect("stdin should write");
        let output = child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("{label} summary should run: {error}"));

        assert!(
            output.status.success(),
            "{label} summary should pass: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert!(
            stdout.contains(expected_hint),
            "{label} summary should report {expected_hint}: stdout={stdout}"
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
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("summary should include {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should include {name}: stdout={stdout}"))
    };
    assert_eq!(value("perf_memmove_source_hint"), "guest_runner_thread");
    assert_eq!(value("cpu_trace_hotspot_hint"), "guest_state_copies");
}

#[test]
fn prove_timing_root_summary_reads_sibling_perf_report() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let temp_dir = crate_root.join("../../temp").join(format!(
        "prove-timing-root-summary-perf-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("test temp directory should be created");
    let log_path = temp_dir.join("sample.log");
    let perf_report_path = temp_dir.join("sample.perf.report");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=1000",
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing log should be written");
    std::fs::write(
        &perf_report_path,
        [
            "    22.58%  [.] lzvm_prover::guest_pc_trace_backend::apply_main_lowered_report_row",
            "    17.70%  [.] __memmove_avx512_unaligned_erms",
            "     4.70%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
        ]
        .join("\n"),
    )
    .expect("sibling perf report should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            ",22.580,17.700,0.000,0.000,none,4.700,0.000,none,report_lifetime_and_data_movement"
        ),
        "prove timing root summary should merge sibling perf report hotspots: stdout={stdout}"
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

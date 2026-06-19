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
        "direct_d2h_hot_wait_pct",
        "direct_d2h_action_hint",
        "timing_cuda_allocator_host_register_wait_ns",
        "timing_cuda_allocator_copy_h2d_bytes",
        "timing_cuda_allocator_copy_h2d_wait_ns",
        "timing_cuda_allocator_copy_h2d_hot_bytes",
        "timing_cuda_allocator_copy_h2d_hot_count",
        "timing_cuda_allocator_copy_h2d_hot_wait_ns",
        "timing_guest_trace_runner_ms",
        "timing_guest_trace_lowerer_ms",
        "timing_guest_trace_lower_ms",
        "trace_lower_ms",
        "trace_runner_lowerer_overlap_ms",
        "trace_lowerer_non_lower_ms",
        "timing_guest_trace_stream_elapsed_ms",
        "timing_guest_trace_stream_ms",
        "timing_guest_segment_commit_ms",
        "timing_guest_segment_commit_initial_workers",
        "timing_guest_segment_commit_effective_workers",
        "timing_guest_segment_commit_oom_retries",
        "timing_guest_segment_commit_attempt_ms",
        "timing_guest_segment_commit_oom_retry_ms",
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
        "timing_guest_trace_external_op_runs",
        "timing_guest_trace_external_op_max_run",
        "timing_guest_trace_copy_runs",
        "timing_guest_trace_copy_max_run",
        "trace_shape_run_hint",
        "timing_guest_trace_report_buffer_capacity",
        "timing_guest_trace_report_buffer_max_capacity",
        "timing_guest_trace_report_buffer_excess_capacity",
        "timing_guest_trace_report_record_size_bytes",
        "timing_guest_trace_report_instruction_size_bytes",
        "timing_guest_trace_report_register_write_list_size_bytes",
        "timing_guest_trace_report_memory_access_list_size_bytes",
        "timing_guest_trace_report_precompile_access_list_size_bytes",
        "timing_guest_trace_report_storage_bytes",
        "timing_guest_trace_report_buffer_capacity_bytes",
        "timing_guest_trace_report_buffer_excess_bytes",
        "trace_report_buffer_shape_hint",
        "trace_report_storage_gib",
        "trace_report_buffer_capacity_gib",
        "timing_guest_trace_descriptor_rows",
        "timing_guest_trace_descriptor_compact_rows",
        "timing_guest_trace_descriptor_wide_rows",
        "timing_guest_device_source_descriptor_upload_bytes",
        "timing_guest_device_source_descriptor_upload_rows",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_values",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_rows",
        "timing_guest_trace_descriptor_high32_a_values",
        "timing_guest_trace_descriptor_high32_b_values",
        "timing_guest_trace_descriptor_high32_c_values",
        "timing_guest_trace_descriptor_high32_a_payload_values",
        "timing_guest_trace_descriptor_high32_b_payload_values",
        "timing_guest_trace_descriptor_high32_store_payload_values",
        "timing_guest_trace_descriptor_high32_store_prev_value_values",
        "timing_guest_trace_descriptor_high32_rows_with_0_fields",
        "timing_guest_trace_descriptor_high32_rows_with_7_fields",
        "descriptor_sparse_high32_estimated_upload_bytes",
        "descriptor_sparse_high32_shape_hint",
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
        "timing_guest_stage_source_retention_attempts",
        "timing_guest_stage_source_retention_retained",
        "timing_guest_stage_source_retention_rejected",
        "timing_guest_stage_source_retention_max_retained_bytes",
        "timing_guest_stage_source_retention_max_rejected_bytes",
        "timing_guest_stage_source_retention_limit_bytes",
        "opening_source_rebuild_hint",
        "timing_finish_witness_opening_row_values_device_rows",
        "timing_finish_witness_opening_row_values_source_rows",
        "timing_finish_witness_opening_row_value_source_extend_ms",
        "opening_row_value_source_extend_ms",
        "opening_row_value_source_extend_pct",
        "opening_source_row_value_action_hint",
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
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms",
        "timing_finish_witness_opening_path_parent_hash_launches_per_stage",
        "timing_finish_witness_opening_row_values_device_download_batches",
        "timing_constant_material_validation_elapsed_ms",
        "timing_constant_material_validation_join_wait_ms",
        "constant_material_validation_overlap_hint",
        "input_bytes",
        "needs_cross_segment_root_pipeline",
        "opening_batching_hint",
        "opening_retained_parent_checkpoint_action_hint",
        "retained_parent_checkpoint_path_time_secondary",
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
        "dominant_trace_pipeline_action_hint",
        "trace_pipeline_action_consensus",
        "cuda_host_register_wait_ms",
        "cuda_h2d_bytes",
        "cuda_transfer_action_hint",
        "dominant_cuda_transfer_action_hint",
        "cuda_transfer_action_consensus",
        "perf_lowered_report_row_self_pct",
        "perf_memmove_self_pct",
        "perf_memmove_guest_machine_pct",
        "perf_memmove_trace_slice_pct",
        "perf_memmove_source_hint",
        "perf_pending_segment_drop_self_pct",
        "perf_sha256_self_pct",
        "perf_sha256_source_hint",
        "cpu_trace_hotspot_hint",
        "perf_prepare_instruction_self_pct",
        "perf_append_descriptor_self_pct",
        "perf_source_value_self_pct",
        "cpu_trace_lowerer_action_hint",
        "perf_trace_segment_build_self_pct",
        "perf_advance_guest_machine_self_pct",
        "perf_guest_memory_write_self_pct",
        "perf_biguint_modpow_self_pct",
        "perf_guest_memory_read_self_pct",
        "perf_decode_instruction_self_pct",
        "perf_effect_record_memory_write_self_pct",
        "perf_effect_record_memory_read_self_pct",
        "cpu_runner_hotspot_hint",
        "trace_pipeline_action_hint",
        "timing_guest_trace_report_detail_samples",
        "trace_report_detail_sample_hint",
        "trace_report_detail_action_hint",
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
        "profile,input_bytes,total_ms,constant_material_validation_elapsed_ms,constant_material_validation_join_wait_ms,constant_material_validation_overlap_hint,runner_ms,lowerer_ms,trace_lower_ms,trace_runner_lowerer_overlap_ms,trace_lowerer_non_lower_ms,stream_elapsed_ms,stream_worker_ms,segment_commit_ms,segment_commit_initial_workers,segment_commit_effective_workers,segment_commit_oom_retries,segment_commit_attempt_ms,segment_commit_oom_retry_ms,stream_commit_residual_ms,segment_receive_wait_ms,pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,parallel_lower_max_reorder,trace_reports,trace_report_rows,trace_rows_per_report,trace_report_buffer_capacity,trace_report_buffer_max_capacity,trace_report_buffer_excess_capacity,trace_report_buffer_excess_pct,trace_report_buffer_shape_hint,trace_report_lifetime_hint,descriptor_rows,descriptor_compact_rows,descriptor_wide_rows,descriptor_upload_bytes,descriptor_bytes_per_row,descriptor_high32_nonzero_values,descriptor_high32_nonzero_rows,descriptor_high32_row_pct,descriptor_high32_a_values,descriptor_high32_b_values,descriptor_high32_c_values,descriptor_high32_a_payload_values,descriptor_high32_b_payload_values,descriptor_high32_store_payload_values,descriptor_high32_store_prev_value_values,descriptor_high32_rows_with_0_fields,descriptor_high32_rows_with_1_fields,descriptor_high32_rows_with_2_fields,descriptor_high32_rows_with_3_fields,descriptor_high32_rows_with_4_fields,descriptor_high32_rows_with_5_fields,descriptor_high32_rows_with_6_fields,descriptor_high32_rows_with_7_fields,descriptor_sparse_high32_estimated_upload_bytes,descriptor_sparse_high32_estimated_upload_savings_pct,descriptor_sparse_high32_high_words,descriptor_sparse_high32_shape_hint,descriptor_shape_hint,seed_direct_lift_attempts,seed_direct_lift_successes,seed_full_advances,finish_opening_ms,opening_query_units,opening_single_query_units,opening_queries,opening_max_queries_per_unit,opening_stage_count,opening_source_shape_hint,opening_row_value_device_rows,opening_row_value_source_rows,opening_row_value_source_extend_ms,opening_row_value_source_extend_pct,opening_source_row_value_action_hint,retained_leaf_openings,retained_leaf_rows,retained_leaf_all_single_row,retained_leaf_path_launches,retained_parent_checkpoint_openings,retained_parent_checkpoint_rows,retained_parent_checkpoint_all_single_row,retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches,retained_parent_checkpoint_path_launches,retained_parent_checkpoint_cross_stage_gather_estimated_launches,retained_parent_checkpoint_cross_stage_gather_launch_savings,opening_path_parent_hash_launches_per_stage,opening_row_value_device_download_batches,opening_row_value_device_single_downloads,opening_row_value_device_single_stage_count,opening_row_value_device_single_max_stage,opening_row_value_device_cross_unit_batch_savings,opening_batching_hint,root_count,materialization_groups,materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline,root_pipeline_policy_hint,leaf_kernel_ms,leaf_coset_calls,leaf_coset_columns,leaf_ntt_launches,leaf_ntt_stage_launches,leaf_ntt_block_twiddle_launches,leaf_ntt_launches_per_call,direct_d2h_wait_ms,leaf_launch_pressure,trace_to_leaf_ratio,primary_bottleneck,trace_structure_hint,proof_12s_gap_ms,proof_12s_gap_hint,perf_lowered_report_row_self_pct,perf_memmove_self_pct,perf_memmove_guest_machine_pct,perf_memmove_trace_slice_pct,perf_memmove_source_hint,perf_pending_segment_drop_self_pct,perf_sha256_self_pct,perf_sha256_source_hint,cpu_trace_hotspot_hint",
        "single-root-groups,2758032,9050,0,0,none,7800,7812,0,5700,0,9912,7812,2100,2,2,0,0,0,0,6000,1200,345,2,23,23,23,1,93843537,93917088,1.001,94371840,4194304,528303,0.560,report_buffer_capacity_tight,tight_report_buffer_and_pending_drop,1000,1000,0,88000,88.000,6,4,0.400,1,0,2,0,1,0,2,10,3,2,1,0,1,0,0,72024,18.155,3,sparse_high32_descriptor_candidate,high32_sparse_compact_descriptor,22,22,1,476,23,23,0,0,0,single_query_cross_root_with_no_sources,0,0,0,0.000,none,23,23,yes,276,0,0,no,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,cross_segment_retained_leaf_opening_candidate,23,23,1,1.000,yes,enable_cross_segment_root_pipeline,858,23,874,41078,15732,23598,1786.000,192.974,yes,9.105,stream_elapsed,parallel_lower_waiting,0,within_12s_target,26.350,20.940,10.610,8.670,guest_machine_and_trace_slice,7.410,23.170,sha256_digest_unresolved,report_lifetime_and_data_movement",
        "batched-roots,2758032,9050,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0,0.000,none,none,0,0,0,0,0.000,0,0,0.000,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,none,none,0,0,0,0,0,0,0,0,0,none,0,0,0,0.000,none,0,0,no,0,0,0,no,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,none,23,1,23,23.000,no,root_batches_already_grouped,0,0,0,0,0,0,0.000,0.000,no,0.000,total,none,0,within_12s_target,0.000,0.000,0.000,0.000,none,0.000,0.000,none,none",
        "slow-sample,12447640,18100,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0,0.000,none,none,0,0,0,0,0.000,0,0,0.000,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,none,none,0,0,0,0,0,0,0,0,0,none,0,0,0,0.000,none,0,0,no,0,0,0,no,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,none,120,120,1,1.000,yes,large_input_root_pipeline_gated,0,0,0,0,0,0,0.000,0.000,no,0.000,total,none,6100,target_gap_needs_timing_breakdown,0.000,0.000,0.000,0.000,none,0.000,0.000,none,none",
        "aggregate,total_count,valid_total_count,total_min_ms,total_mean_ms,total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier",
        "aggregate,3,3,9050,12066.667,9050.000,18100,100.000,no,yes",
    ] {
        let required = if required.starts_with("profile,input_bytes,total_ms") {
            "profile,input_bytes,total_ms,constant_material_validation_elapsed_ms"
        } else if required.starts_with("single-root-groups,") {
            "single-root-groups,2758032,9050"
        } else if required.starts_with("batched-roots,") {
            "batched-roots,2758032,9050"
        } else if required.starts_with("slow-sample,") {
            "slow-sample,12447640,18100"
        } else {
            required
        };
        assert!(
            stdout.contains(required),
            "prove timing root summary should print {required}"
        );
    }
}

#[test]
fn prove_timing_root_summary_reports_source_retention_rebuild_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-source-retention");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("source retention fixture dir should be created");
    let log_path = dir.join("source-retention.log");
    let input = [
        "timing_total_ms=51642",
        "input_bytes=12447640",
        "timing_guest_trace_stream_elapsed_ms=42310",
        "timing_guest_segment_commit_ms=20214",
        "timing_finish_witness_opening_ms=8993",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_stage_count=240",
        "timing_finish_witness_opening_retained_source_count=0",
        "timing_finish_witness_opening_external_source_count=240",
        "timing_finish_witness_opening_embedded_source_count=0",
        "timing_finish_witness_opening_missing_source_count=0",
        "timing_guest_stage_source_retention_attempts=240",
        "timing_guest_stage_source_retention_retained=0",
        "timing_guest_stage_source_retention_rejected=240",
        "timing_guest_stage_source_retention_retained_bytes=0",
        "timing_guest_stage_source_retention_rejected_bytes=314069483520",
        "timing_guest_stage_source_retention_max_retained_bytes=0",
        "timing_guest_stage_source_retention_max_rejected_bytes=1308622848",
        "timing_guest_stage_source_retention_limit_bytes=0",
        "timing_guest_segment_commit_cuda_memory_total_bytes=33711521792",
        "timing_cuda_allocator_copy_h2d_bytes=88120305952",
        "timing_cuda_allocator_copy_h2d_wait_ns=7040040536",
        "timing_cuda_allocator_host_register_wait_ns=1609017316",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");
    std::fs::write(&log_path, input).expect("source retention fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should finish");

    assert!(
        output.status.success(),
        "prove timing root summary should parse source retention input: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines.next().expect("summary should include a header");
    let row = lines.next().expect("summary should include a data row");
    let headers = headers.split(',').collect::<Vec<_>>();
    let row = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| -> &str {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("missing header {name}: {headers:?}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("missing value for {name}: {row:?}"))
    };

    assert_eq!(value("source_retention_attempts"), "240");
    assert_eq!(value("source_retention_retained"), "0");
    assert_eq!(value("source_retention_rejected"), "240");
    assert_eq!(value("source_retention_retained_bytes"), "0");
    assert_eq!(value("source_retention_rejected_bytes"), "314069483520");
    assert_eq!(value("source_retention_max_retained_bytes"), "0");
    assert_eq!(value("source_retention_max_rejected_bytes"), "1308622848");
    assert_eq!(value("source_retention_limit_bytes"), "0");
    assert_eq!(
        value("source_retention_rejected_total_exceeds_device_memory"),
        "yes"
    );
    assert_eq!(
        value("source_retention_max_rejected_exceeds_device_memory"),
        "no"
    );
    assert_eq!(
        value("opening_source_rebuild_hint"),
        "retained_source_disabled_external_rebuild"
    );
    assert_eq!(
        value("data_residency_action_hint"),
        "full_source_retention_exceeds_device_memory"
    );
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
        "timing_guest_trace_lowerer_ms=2000",
        "timing_guest_trace_lower_ms=1500",
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
            "trace_report_detail_samples,trace_report_detail_sample_pct,trace_report_detail_sample_ppm,trace_report_detail_sample_hint,trace_report_detail_avg_ns,trace_report_detail_lowerer_share_ms,trace_report_row_validation_lowerer_share_ms,trace_report_memory_columns_lowerer_share_ms,trace_report_source_values_lowerer_share_ms,trace_report_source_lookup_lowerer_share_ms,trace_report_source_values_residual_lowerer_share_ms,trace_report_precompile_memory_lowerer_share_ms,trace_report_instruction_result_lowerer_share_ms,trace_report_next_pc_lowerer_share_ms,trace_report_register_access_lowerer_share_ms,trace_report_memory_access_lowerer_share_ms,trace_report_store_apply_lowerer_share_ms,trace_report_row_validation_residual_lowerer_share_ms,trace_report_visit_lowerer_share_ms,trace_report_descriptor_lowerer_share_ms,trace_report_detail_hotspot,trace_report_detail_hotspot_pct,trace_report_detail_action_hint,trace_report_row_validation_hotspot,trace_report_row_validation_hotspot_pct,trace_report_row_validation_explained_pct,trace_report_row_validation_residual_pct,trace_report_source_values_lookup_pct,trace_report_source_values_residual_pct,trace_report_detail_visit_pct,trace_report_visit_descriptor_pct,trace_report_visit_residual_pct"
        ),
        "prove timing root summary should expose detail sample, lowerer-share cost, source-value, and visit drilldown columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",10,1.000,10000.000,detail_timing_sampled,100,1500.000,750.000,90.000,270.000,345.000,0.000,30.000,0.000,0.000,150.000,120.000,0.000,15.000,300.000,75.000,row_validation,50.000,profile_row_validation,source_a_value,28.000,98.000,2.000,127.778,0.000,20.000,25.000,75.000"
        ),
        "prove timing root summary should classify sampled detail, scale costs by actual trace lower work, source-value lookup coverage, row-validation, and visit hotspots: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "trace_report_source_values_residual_ns_per_row,trace_report_row_validation_residual_ns_per_row,trace_report_visit_residual_ns_per_row,trace_report_descriptor_ns_per_row"
        ),
        "prove timing root summary should expose per-row residual and descriptor costs: stdout={stdout}"
    );
    assert!(
        stdout.contains(",0.000,15000.000,225000.000,75000.000"),
        "prove timing root summary should scale residual and descriptor costs to ns per trace row: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_report_layout_breakdown() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_trace_reports=67108864",
        "timing_guest_trace_report_record_size_bytes=192",
        "timing_guest_trace_report_instruction_size_bytes=16",
        "timing_guest_trace_report_register_write_list_size_bytes=32",
        "timing_guest_trace_report_memory_access_list_size_bytes=80",
        "timing_guest_trace_report_precompile_access_list_size_bytes=24",
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
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_report_instruction_size_bytes"), "16");
    assert_eq!(value("trace_report_register_write_list_size_bytes"), "32");
    assert_eq!(value("trace_report_memory_access_list_size_bytes"), "80");
    assert_eq!(
        value("trace_report_precompile_access_list_size_bytes"),
        "24"
    );
    assert_eq!(value("trace_report_instruction_storage_gib"), "1.000");
    assert_eq!(
        value("trace_report_register_write_list_storage_gib"),
        "2.000"
    );
    assert_eq!(
        value("trace_report_memory_access_list_storage_gib"),
        "5.000"
    );
    assert_eq!(
        value("trace_report_precompile_access_list_storage_gib"),
        "1.500"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_lower_work_and_wall_overlap() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=2758032",
        "timing_total_ms=9900",
        "timing_guest_trace_runner_ms=7800",
        "timing_guest_trace_lowerer_ms=7812",
        "timing_guest_trace_lower_ms=6200",
        "timing_guest_trace_stream_elapsed_ms=9912",
        "timing_guest_trace_stream_ms=7812",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=23",
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
            "runner_ms,lowerer_ms,trace_lower_ms,trace_runner_lowerer_overlap_ms,trace_lowerer_non_lower_ms,stream_elapsed_ms"
        ),
        "prove timing root summary should expose actual trace lower work and runner/lowerer wall overlap columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(",7800,7812,6200,5700,1612,9912,"),
        "prove timing root summary should compute overlap and non-lowerer work from timing_guest_trace_lower_ms: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_classifies_trace_pipeline_action() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52000",
        "timing_guest_trace_runner_ms=41000",
        "timing_guest_trace_lowerer_ms=35000",
        "timing_guest_trace_lower_ms=33000",
        "timing_guest_trace_stream_elapsed_ms=43000",
        "timing_guest_trace_stream_ms=22000",
        "timing_guest_segment_commit_ms=21000",
        "timing_guest_trace_segment_receive_wait_ms=22000",
        "timing_guest_trace_pending_receive_wait_ms=1000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
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
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let hint_index = headers
        .iter()
        .position(|header| *header == "trace_pipeline_action_hint")
        .expect("summary should expose trace pipeline action hint");
    assert_eq!(
        fields.get(hint_index),
        Some(&"trace_generation_and_commit_pipeline_candidate"),
        "trace-heavy run with a large commit gate should point at the combined pipeline lever: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_source_row_value_extend_priority() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52000",
        "timing_guest_trace_runner_ms=41000",
        "timing_guest_trace_lowerer_ms=35000",
        "timing_guest_trace_lower_ms=33000",
        "timing_guest_trace_stream_elapsed_ms=43000",
        "timing_guest_trace_stream_ms=22000",
        "timing_guest_segment_commit_ms=21000",
        "timing_guest_trace_segment_receive_wait_ms=22000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_external_source_count=120",
        "timing_finish_witness_opening_embedded_source_count=120",
        "timing_finish_witness_opening_row_values_source_rows=77",
        "timing_finish_witness_opening_row_value_source_extend_ms=1134",
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
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("opening_row_value_source_extend_ms"), "1134");
    assert_eq!(value("opening_row_value_source_extend_pct"), "2.181");
    assert_eq!(
        value("opening_source_row_value_action_hint"),
        "trace_pipeline_before_source_row_values"
    );
}

#[test]
fn prove_timing_root_summary_aggregates_trace_pipeline_action() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-aggregate-action");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64| {
        [
            "input_bytes=12447640".to_owned(),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_trace_runner_ms=41000".to_owned(),
            "timing_guest_trace_lowerer_ms=35000".to_owned(),
            "timing_guest_trace_lower_ms=33000".to_owned(),
            "timing_guest_trace_stream_elapsed_ms=43000".to_owned(),
            "timing_guest_trace_stream_ms=22000".to_owned(),
            "timing_guest_segment_commit_ms=21000".to_owned(),
            "timing_guest_trace_segment_receive_wait_ms=22000".to_owned(),
            "timing_guest_trace_pending_receive_wait_ms=1000".to_owned(),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ]
        .join("\n")
    };
    let paths = [52000_u64, 52100, 51950]
        .into_iter()
        .enumerate()
        .map(|(index, total_ms)| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(total_ms)).expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("dominant_trace_pipeline_action_hint,trace_pipeline_action_consensus"),
        "aggregate summary should expose cross-sample action hint stability: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "aggregate,3,3,51950,52016.667,52000.000,52100,0.288,yes,no,trace_generation_and_commit_pipeline_candidate,yes"
        ),
        "aggregate row should report the dominant trace pipeline action and consensus: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_excludes_diagnostic_shape_profiles_from_aggregate() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-aggregate-diagnostic-shape");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64, shape_profile: bool| {
        let mut lines = vec![
            "input_bytes=2758032".to_owned(),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_trace_report_rows=1000".to_owned(),
            "timing_guest_stage_tree_commit_root_count=23".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=23".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ];
        if shape_profile {
            lines.push("timing_guest_trace_external_op_rows=500".to_owned());
        }
        lines.join("\n")
    };
    let fixtures = [
        (8_000_u64, false),
        (8_100, false),
        (8_200, false),
        (20_000, true),
    ];
    let paths = fixtures
        .into_iter()
        .enumerate()
        .map(|(index, (total_ms, shape_profile))| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(total_ms, shape_profile))
                .expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.len() >= 6,
        "multi-sample summary should include data and aggregate rows: stdout={stdout}"
    );
    let profile_headers = lines[0].split(',').collect::<Vec<_>>();
    let diagnostic_row = lines[4].split(',').collect::<Vec<_>>();
    let profile_hint_index = profile_headers
        .iter()
        .position(|header| *header == "trace_shape_profile_hint")
        .expect("summary should expose trace shape profile hint");
    assert_eq!(
        diagnostic_row.get(profile_hint_index),
        Some(&"diagnostic_only_shape_profile"),
        "shape-profile sample should be tagged before aggregation: stdout={stdout}"
    );

    let aggregate_headers = lines[5].split(',').collect::<Vec<_>>();
    let aggregate_fields = lines[6].split(',').collect::<Vec<_>>();
    let aggregate_value = |name: &str| {
        let index = aggregate_headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("aggregate should expose {name}: stdout={stdout}"));
        aggregate_fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("aggregate row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(aggregate_value("total_count"), "4");
    assert_eq!(aggregate_value("valid_total_count"), "3");
    assert_eq!(aggregate_value("total_mean_ms"), "8100.000");
    assert_eq!(aggregate_value("total_max_ms"), "8200");
    assert_eq!(aggregate_value("close_samples"), "yes");
}

#[test]
fn prove_timing_root_summary_aggregates_cuda_transfer_action() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-transfer-action");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64| {
        [
            "input_bytes=12447640".to_owned(),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
            "timing_cuda_allocator_host_register_wait_ns=5286762509".to_owned(),
            "timing_cuda_allocator_copy_h2d_bytes=88120303688".to_owned(),
            "timing_cuda_allocator_copy_h2d_wait_ns=7329175000".to_owned(),
            "timing_cuda_direct_copy_d2h_hot_bytes=1152".to_owned(),
            "timing_cuda_direct_copy_d2h_hot_count=41".to_owned(),
            "timing_cuda_direct_copy_d2h_hot_wait_ns=3388755526".to_owned(),
        ]
        .join("\n")
    };
    let paths = [60139_u64, 60080, 60200]
        .into_iter()
        .enumerate()
        .map(|(index, total_ms)| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(total_ms)).expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.len() >= 6,
        "multi-sample summary should include data and aggregate rows: stdout={stdout}"
    );
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    let transfer_hint_index = headers
        .iter()
        .position(|header| *header == "cuda_transfer_action_hint")
        .expect("summary should expose CUDA transfer action hint");
    assert_eq!(
        fields.get(transfer_hint_index),
        Some(&"reduce_bulk_h2d_source_uploads"),
        "large H2D upload and host registration waits should point at source upload reduction: stdout={stdout}"
    );

    let aggregate_headers = lines[4].split(',').collect::<Vec<_>>();
    let aggregate_fields = lines[5].split(',').collect::<Vec<_>>();
    let dominant_index = aggregate_headers
        .iter()
        .position(|header| *header == "dominant_cuda_transfer_action_hint")
        .expect("aggregate summary should expose dominant CUDA transfer action hint");
    let consensus_index = aggregate_headers
        .iter()
        .position(|header| *header == "cuda_transfer_action_consensus")
        .expect("aggregate summary should expose CUDA transfer action consensus");
    assert_eq!(
        aggregate_fields.get(dominant_index),
        Some(&"reduce_bulk_h2d_source_uploads"),
        "aggregate row should report the dominant CUDA transfer action: stdout={stdout}"
    );
    assert_eq!(
        aggregate_fields.get(consensus_index),
        Some(&"yes"),
        "aggregate row should report stable CUDA transfer action consensus: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_allocator_d2h_wait_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=48396",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
        "timing_cuda_allocator_copy_d2h_bytes=291360",
        "timing_cuda_allocator_copy_d2h_wait_ns=3429156569",
        "timing_cuda_allocator_copy_d2h_hot_bytes=304",
        "timing_cuda_allocator_copy_d2h_hot_count=120",
        "timing_cuda_allocator_copy_d2h_hot_wait_ns=3409364047",
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
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("cuda_allocator_d2h_bytes"), "291360");
    assert_eq!(value("cuda_allocator_d2h_wait_ms"), "3429.157");
    assert_eq!(value("cuda_allocator_d2h_hot_bytes"), "304");
    assert_eq!(value("cuda_allocator_d2h_hot_count"), "120");
    assert_eq!(value("cuda_allocator_d2h_hot_wait_ms"), "3409.364");
    assert_eq!(value("cuda_allocator_d2h_hot_wait_pct"), "99.423");
    assert_eq!(
        value("cuda_allocator_d2h_action_hint"),
        "opening_row_value_d2h_wait_secondary"
    );
}

#[test]
fn prove_timing_root_summary_groups_aggregate_samples_by_input_size() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-input-size-aggregate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |input_bytes: u64, total_ms: u64| {
        [
            format!("input_bytes={input_bytes}"),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_trace_runner_ms=41000".to_owned(),
            "timing_guest_trace_lowerer_ms=35000".to_owned(),
            "timing_guest_trace_lower_ms=33000".to_owned(),
            "timing_guest_trace_stream_elapsed_ms=43000".to_owned(),
            "timing_guest_trace_stream_ms=22000".to_owned(),
            "timing_guest_segment_commit_ms=21000".to_owned(),
            "timing_guest_trace_segment_receive_wait_ms=22000".to_owned(),
            "timing_guest_trace_pending_receive_wait_ms=1000".to_owned(),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ]
        .join("\n")
    };
    let fixtures = [
        (2_758_032_u64, 8_270_u64),
        (12_447_640, 50_650),
        (2_758_032, 8_330),
        (12_447_640, 51_026),
        (2_758_032, 8_365),
        (12_447_640, 50_792),
    ];
    let paths = fixtures
        .into_iter()
        .enumerate()
        .map(|(index, (input_bytes, total_ms))| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(input_bytes, total_ms))
                .expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("aggregate,6,6,8270,29572.167,29507.500,51026,144.899,no,yes"),
        "global aggregate should still show mixed small and large samples are not close: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "aggregate_by_input_bytes,input_bytes,total_count,valid_total_count,total_min_ms,total_mean_ms,total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier"
        ),
        "grouped aggregate should expose the input size discriminator: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "aggregate_by_input_bytes,2758032,3,3,8270,8321.667,8330.000,8365,1.140,yes,no"
        ),
        "small samples should be summarized as a close input-size group: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "aggregate_by_input_bytes,12447640,3,3,50650,50822.667,50792.000,51026,0.740,yes,no"
        ),
        "large samples should be summarized as a close input-size group: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_segment_commit_memory_margin() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-segment-commit-memory");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let input = [
        "input_bytes=12447640",
        "timing_total_ms=50142",
        "timing_guest_segment_commit_initial_workers=3",
        "timing_guest_segment_commit_effective_workers=3",
        "timing_guest_segment_commit_oom_retries=0",
        "timing_guest_segment_commit_cuda_memory_total_bytes=34359738368",
        "timing_guest_segment_commit_cuda_memory_initial_free_bytes=12025908428",
        "timing_guest_segment_commit_cuda_memory_effective_free_bytes=12025908428",
        "timing_guest_segment_commit_cuda_memory_min_free_bytes=1717986918",
        "timing_guest_segment_commit_cuda_allocator_initial_cached_bytes=0",
        "timing_guest_segment_commit_cuda_allocator_effective_cached_bytes=0",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");
    let path = dir.join("sample.log");
    std::fs::write(&path, input).expect("sample timing log should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    for (header, expected) in [
        ("segment_commit_cuda_memory_total_bytes", "34359738368"),
        (
            "segment_commit_cuda_memory_initial_free_bytes",
            "12025908428",
        ),
        (
            "segment_commit_cuda_memory_effective_free_bytes",
            "12025908428",
        ),
        ("segment_commit_cuda_memory_min_free_bytes", "1717986918"),
        ("segment_commit_cuda_allocator_initial_cached_bytes", "0"),
        ("segment_commit_cuda_allocator_effective_cached_bytes", "0"),
        ("segment_commit_cuda_memory_min_free_pct", "5.000"),
        (
            "segment_commit_memory_pressure_hint",
            "segment_commit_memory_pressure",
        ),
    ] {
        let index = headers
            .iter()
            .position(|candidate| *candidate == header)
            .unwrap_or_else(|| panic!("summary should expose {header}: stdout={stdout}"));
        assert_eq!(
            fields.get(index),
            Some(&expected),
            "summary should report {header}: stdout={stdout}"
        );
    }
}

#[test]
fn prove_timing_root_summary_reports_descriptor_retention_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-descriptor-retention");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let input = [
        "input_bytes=12447640",
        "timing_total_ms=58706",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_allocator_copy_h2d_bytes=80369229896",
        "timing_cuda_allocator_copy_h2d_wait_ns=3444077241",
        "timing_cuda_direct_copy_d2h_hot_bytes=1152",
        "timing_cuda_direct_copy_d2h_hot_count=61",
        "timing_cuda_direct_copy_d2h_hot_wait_ns=4960767295",
        "timing_guest_descriptor_buffer_retention_attempts=120",
        "timing_guest_descriptor_buffer_retention_retained=21",
        "timing_guest_descriptor_buffer_retention_rejected=99",
        "timing_guest_descriptor_buffer_retention_retained_bytes=7751073792",
        "timing_guest_descriptor_buffer_retention_rejected_bytes=36241643328",
        "timing_guest_descriptor_buffer_retention_limit_bytes=8000000000",
    ]
    .join("\n");
    let path = dir.join("sample.log");
    std::fs::write(&path, input).expect("sample timing log should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    for (header, expected) in [
        ("descriptor_retention_attempts", "120"),
        ("descriptor_retention_retained", "21"),
        ("descriptor_retention_rejected", "99"),
        ("descriptor_retention_retained_bytes", "7751073792"),
        ("descriptor_retention_rejected_bytes", "36241643328"),
        ("descriptor_retention_limit_bytes", "8000000000"),
        (
            "cuda_transfer_action_hint",
            "retained_descriptor_d2h_tradeoff",
        ),
    ] {
        let index = headers
            .iter()
            .position(|candidate| *candidate == header)
            .unwrap_or_else(|| panic!("summary should expose {header}: stdout={stdout}"));
        assert_eq!(
            fields.get(index),
            Some(&expected),
            "summary should report {header}: stdout={stdout}"
        );
    }
}

#[test]
fn prove_timing_root_summary_classifies_initial_descriptor_upload_when_retained() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=2758032",
        "timing_total_ms=8250",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=23",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_allocator_copy_h2d_bytes=8264703744",
        "timing_cuda_allocator_copy_h2d_wait_ns=645000000",
        "timing_guest_device_source_descriptor_upload_bytes=8264703744",
        "timing_guest_device_source_descriptor_upload_rows=93917088",
        "timing_guest_descriptor_buffer_retention_attempts=23",
        "timing_guest_descriptor_buffer_retention_retained=23",
        "timing_guest_descriptor_buffer_retention_rejected=0",
        "timing_guest_descriptor_buffer_retention_retained_bytes=8264703744",
        "timing_guest_descriptor_buffer_retention_rejected_bytes=0",
        "timing_guest_descriptor_buffer_retention_limit_bytes=10000000000",
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
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|candidate| *candidate == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("descriptor_upload_bytes"), "8264703744");
    assert_eq!(value("descriptor_retention_attempts"), "23");
    assert_eq!(value("descriptor_retention_retained"), "23");
    assert_eq!(value("descriptor_retention_rejected"), "0");
    assert_eq!(
        value("cuda_transfer_action_hint"),
        "initial_descriptor_upload_retention_active"
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
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
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
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    for (header, expected) in [
        ("direct_d2h_hot_wait_pct", "81.551"),
        (
            "direct_d2h_action_hint",
            "single_query_unit_boundary_blocks_row_value_batch",
        ),
    ] {
        let index = headers
            .iter()
            .position(|candidate| *candidate == header)
            .unwrap_or_else(|| panic!("summary should expose {header}: stdout={stdout}"));
        assert_eq!(
            fields.get(index),
            Some(&expected),
            "summary should report {header}: stdout={stdout}"
        );
    }
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
fn prove_timing_root_summary_reports_trace_shape_row_mix_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=72000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=500000000",
        "timing_guest_trace_report_rows=500000000",
        "timing_guest_trace_single_row_reports=499000000",
        "timing_guest_trace_external_op_rows=237000000",
        "timing_guest_trace_copy_rows=254000000",
        "timing_guest_trace_indirect_memory_rows=224000000",
        "timing_guest_trace_memory_source_reads=146000000",
        "timing_guest_trace_memory_store_rows=81200000",
        "timing_guest_trace_no_store_rows=46400000",
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
        stdout.contains("external_op_row_pct,copy_row_pct,trace_shape_row_mix_hint"),
        "prove timing root summary should expose external-op and copy row mix columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(",47.400,50.800,copy_and_external_op_rows_dominate"),
        "prove timing root summary should report row mix percentages and hotspot hint: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_shape_duration_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=72000",
        "timing_guest_trace_lower_ms=1000",
        "timing_guest_trace_report_rows=1000",
        "timing_guest_trace_external_op_rows=300",
        "timing_guest_trace_copy_rows=400",
        "timing_guest_trace_external_op_row_lower_ms=474",
        "timing_guest_trace_copy_row_lower_ms=508",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
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
            "external_op_row_lower_ms,copy_row_lower_ms,external_op_row_lower_ns_per_row,copy_row_lower_ns_per_row,external_op_row_lower_pct,copy_row_lower_pct,trace_shape_duration_hint,trace_shape_unit_cost_hint"
        ),
        "prove timing root summary should expose external-op and copy duration, per-row, and unit-cost hint columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",474,508,1580000.000,1270000.000,47.400,50.800,copy_and_external_op_duration_dominate,external_op_unit_cost_higher"
        ),
        "prove timing root summary should classify external-op and copy duration dominance and per-row cost skew: stdout={stdout}"
    );

    let balanced_input = [
        "timing_total_ms=72000",
        "timing_guest_trace_lower_ms=51154",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_external_op_rows=237231598",
        "timing_guest_trace_copy_rows=253826801",
        "timing_guest_trace_external_op_row_lower_ms=14926",
        "timing_guest_trace_copy_row_lower_ms=16895",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
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
        .write_all(balanced_input.as_bytes())
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
            ",14926,16895,62.917,66.561,29.179,33.028,mixed_trace_shape_duration,row_volume_dominates_shape_duration"
        ),
        "prove timing root summary should classify balanced shape unit costs as row-volume dominated: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_shape_run_lengths() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lower_ms=1500",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1000",
        "timing_guest_trace_single_row_reports=1000",
        "timing_guest_trace_external_op_rows=600",
        "timing_guest_trace_copy_rows=300",
        "timing_guest_trace_external_op_row_lower_ms=600",
        "timing_guest_trace_copy_row_lower_ms=300",
        "timing_guest_trace_external_op_runs=30",
        "timing_guest_trace_external_op_max_run=80",
        "timing_guest_trace_copy_runs=150",
        "timing_guest_trace_copy_max_run=3",
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
            "external_op_runs,external_op_avg_run,external_op_max_run,copy_runs,copy_avg_run,copy_max_run,trace_shape_run_hint"
        ),
        "prove timing root summary should expose trace shape run-length columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(",30,20.000,80,150,2.000,3,external_op_runs_long"),
        "prove timing root summary should classify long external-op row runs: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_parallel_reexecution_hint_for_row_volume_floor() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=54000",
        "timing_guest_trace_runner_ms=41000",
        "timing_guest_trace_lowerer_ms=40500",
        "timing_guest_trace_lower_ms=27000",
        "timing_guest_trace_stream_elapsed_ms=41200",
        "timing_guest_trace_stream_ms=21000",
        "timing_guest_segment_commit_ms=20000",
        "timing_guest_trace_segment_receive_wait_ms=19000",
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_report_rows=500000000",
        "timing_guest_trace_external_op_rows=235000000",
        "timing_guest_trace_copy_rows=255000000",
        "timing_guest_trace_indirect_memory_rows=220000000",
        "timing_guest_trace_external_op_row_lower_ms=12600",
        "timing_guest_trace_copy_row_lower_ms=13900",
        "timing_guest_trace_external_op_runs=78000000",
        "timing_guest_trace_copy_runs=77000000",
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
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_shape_unit_cost_hint"),
        "row_volume_dominates_shape_duration"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "parallel_segment_reexecution_candidate"
    );
    assert_eq!(
        value("trace_shape_profile_hint"),
        "diagnostic_only_shape_profile"
    );
}

#[test]
fn prove_timing_root_summary_reports_spiky_trace_shape_run_lengths() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=74000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lower_ms=53000",
        "timing_guest_trace_reports=499520693",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_single_row_reports=499366777",
        "timing_guest_trace_external_op_rows=237231598",
        "timing_guest_trace_copy_rows=253826801",
        "timing_guest_trace_external_op_row_lower_ms=15939",
        "timing_guest_trace_copy_row_lower_ms=17901",
        "timing_guest_trace_external_op_runs=78604119",
        "timing_guest_trace_external_op_max_run=99",
        "timing_guest_trace_copy_runs=77229084",
        "timing_guest_trace_copy_max_run=250",
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
        stdout.contains(",78604119,3.018,99,77229084,3.287,250,shape_runs_spiky"),
        "prove timing root summary should distinguish sparse long-tail runs from strong long-run batching candidates: stdout={stdout}"
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
fn prove_timing_root_summary_requests_shape_profile_after_detail_only_trace_sample() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=63027",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lower_ms=38619",
        "timing_guest_trace_reports=499520693",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_single_row_reports=0",
        "timing_guest_trace_indirect_memory_rows=0",
        "timing_guest_trace_memory_source_reads=0",
        "timing_guest_trace_memory_store_rows=0",
        "timing_guest_trace_no_store_rows=0",
        "timing_guest_trace_report_detail_samples=596",
        "timing_guest_trace_report_sampled_ns=1861501",
        "timing_guest_trace_report_row_validation_sampled_ns=987695",
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
        stdout.contains("shape_timing_missing_for_detail_profile"),
        "prove timing root summary should request shape timing when detail samples exist but shape counters are absent: stdout={stdout}"
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
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=3",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=14",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
        "timing_finish_witness_opening_row_values_device_single_downloads=43",
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
            "retained_parent_checkpoint_openings,retained_parent_checkpoint_rows,retained_parent_checkpoint_all_single_row,retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_prefix_ms,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches,retained_parent_checkpoint_suffix_ms,retained_parent_checkpoint_path_launches,retained_parent_checkpoint_path_ms,retained_parent_checkpoint_cross_stage_gather_estimated_launches,retained_parent_checkpoint_cross_stage_gather_launch_savings"
        ),
        "prove timing root summary should expose retained parent checkpoint opening shape: stdout={stdout}"
    );
    assert!(
        stdout.contains("retained_parent_checkpoint_batching_hint"),
        "prove timing root summary should expose retained parent checkpoint batching shape: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "opening_row_value_device_download_batches,opening_row_value_device_single_downloads,opening_row_value_device_single_stage_count,opening_row_value_device_single_max_stage,opening_row_value_device_cross_unit_batch_savings,opening_batching_hint"
        ),
        "prove timing root summary should expose single-row device row-value downloads: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",43,0,0,0.000,none,77,77,yes,0,79,79,yes,0,0,79,3,0,0,790,14,869,17,11,858,device_batched_path_secondary,0,0,43,0,0,0,single_query_unit_boundary_blocks_row_value_batch,"
        ),
        "prove timing root summary should avoid recommending row-value gather for single-query unit boundaries: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_retained_parent_checkpoint_action_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52335",
        "timing_finish_witness_opening_ms=9000",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=1300",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=2600",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
        "timing_finish_witness_opening_row_values_device_single_downloads=43",
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
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("opening_batching_hint"),
        "single_query_unit_boundary_blocks_row_value_batch"
    );
    assert_eq!(
        value("opening_retained_parent_checkpoint_action_hint"),
        "cross_stage_retained_parent_checkpoint_prefix_suffix_gather_candidate"
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
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("opening_queries"), "120");
    assert_eq!(value("opening_max_queries_per_unit"), "1");
    assert_eq!(value("opening_stage_count"), "240");
    assert_eq!(
        value("opening_source_shape_hint"),
        "single_query_cross_root_with_mixed_sources"
    );
    assert_eq!(value("source_retention_attempts"), "0");
    assert_eq!(value("source_retention_retained"), "0");
    assert_eq!(value("source_retention_rejected"), "0");
    assert_eq!(
        value("opening_source_rebuild_hint"),
        "mixed_retained_and_external_sources"
    );
    assert_eq!(value("opening_row_value_device_rows"), "79");
    assert_eq!(value("opening_row_value_source_rows"), "77");
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
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
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
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=3",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows=13808034",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes=1767428352",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=170",
        "timing_finish_witness_opening_path_parent_hash_launches_per_stage=5",
        "timing_finish_witness_opening_row_values_device_download_batches=43",
        "timing_finish_witness_opening_row_values_device_single_downloads=0",
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
            "retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_prefix_ms,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches,retained_parent_checkpoint_suffix_ms,retained_parent_checkpoint_path_launches,retained_parent_checkpoint_path_ms,retained_parent_checkpoint_cross_stage_gather_estimated_launches,retained_parent_checkpoint_cross_stage_gather_launch_savings,retained_parent_checkpoint_batching_hint,opening_path_parent_hash_launches_per_stage,opening_row_value_device_download_batches,opening_row_value_device_single_downloads,opening_row_value_device_single_stage_count,opening_row_value_device_single_max_stage,opening_row_value_device_cross_unit_batch_savings"
        ),
        "prove timing root summary should expose opening parent-hash work scope columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",79,79,yes,165675008,21206401024,79,3,13808034,1767428352,790,170,869,173,11,858,device_batched_path_secondary,5,43,0,"
        ),
        "prove timing root summary should report retained parent checkpoint cross-stage gather launch savings: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",device_batched_path_secondary,5,43,0,0,0,0,retained_parent_checkpoint_path_time_secondary,"
        ),
        "prove timing root summary should downgrade retained parent checkpoint batching when measured path time is secondary: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_device_row_value_single_download_stage_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=58552",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_stage_count=240",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
        "timing_finish_witness_opening_row_values_device_single_downloads=43",
        "timing_finish_witness_stage_1_opening_row_values_device_single_downloads=31",
        "timing_finish_witness_stage_2_opening_row_values_device_single_downloads=12",
        "timing_finish_witness_stage_3_opening_row_values_device_single_downloads=0",
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
            "opening_row_value_device_single_stage_count,opening_row_value_device_single_max_stage,opening_row_value_device_cross_unit_batch_savings"
        ),
        "prove timing root summary should expose stage-level device single-download shape: stdout={stdout}"
    );
    assert!(
        stdout.contains(",0,43,2,31,41,single_query_unit_boundary_blocks_row_value_batch,"),
        "prove timing root summary should report the single-query unit boundary instead of a row-value gather estimate: stdout={stdout}"
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
        "timing_guest_trace_report_record_size_bytes=128",
        "timing_guest_trace_report_storage_bytes=12011972736",
        "timing_guest_trace_report_buffer_capacity_bytes=12079595520",
        "timing_guest_trace_report_buffer_excess_bytes=67622784",
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
            "trace_reports,trace_report_rows,trace_rows_per_report,trace_report_record_size_bytes,trace_report_instruction_size_bytes,trace_report_register_write_list_size_bytes,trace_report_memory_access_list_size_bytes,trace_report_precompile_access_list_size_bytes,trace_report_instruction_storage_gib,trace_report_register_write_list_storage_gib,trace_report_memory_access_list_storage_gib,trace_report_precompile_access_list_storage_gib,trace_report_storage_bytes,trace_report_storage_gib,trace_report_buffer_capacity,trace_report_buffer_max_capacity,trace_report_buffer_excess_capacity,trace_report_buffer_capacity_bytes,trace_report_buffer_capacity_gib,trace_report_buffer_excess_bytes,trace_report_buffer_excess_pct,trace_report_buffer_shape_hint,"
        ),
        "prove timing root summary should expose trace report buffer columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "93843537,93917088,1.001,128,0,0,0,0,0.000,0.000,0.000,0.000,12011972736,11.187,94371840,4194304,528303,12079595520,11.250,67622784,0.560,report_buffer_capacity_tight,"
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
fn prove_timing_root_summary_distinguishes_elided_report_buffer_from_missing_data() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=13203",
        "timing_guest_trace_runner_ms=12433",
        "timing_guest_trace_lowerer_ms=0",
        "timing_guest_trace_stream_elapsed_ms=12582",
        "timing_guest_trace_stream_ms=10816",
        "timing_guest_segment_commit_ms=1766",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_rows=93917088",
        "timing_guest_trace_report_buffer_capacity=0",
        "timing_guest_trace_report_buffer_max_capacity=0",
        "timing_guest_trace_report_buffer_excess_capacity=0",
        "timing_guest_trace_report_record_size_bytes=144",
        "timing_guest_trace_report_storage_bytes=13513469328",
        "timing_guest_trace_report_buffer_capacity_bytes=0",
        "timing_guest_trace_report_buffer_excess_bytes=0",
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
        stdout.contains("report_buffer_elided,report_buffer_elided_but_trace_serialized"),
        "prove timing root summary should distinguish elided report buffers from missing timing and warn when lowerer overlap is gone: stdout={stdout}"
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
fn prove_timing_root_summary_reads_sibling_nsys_cpu_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let temp_dir = crate_root.join("../../temp").join(format!(
        "prove-timing-root-summary-nsys-cpu-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("test temp directory should be created");
    let log_path = temp_dir.join("sample.log");
    let cpu_summary_path = temp_dir.join("sample.cpu-summary.txt");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=8314",
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing log should be written");
    std::fs::write(
        &cpu_summary_path,
        [
            "application_cpu_hotspots",
            "symbol,module,samples,application_sample_pct",
            "lzvm_prover::guest_pc_trace_backend::apply_main_lowered_report_row,/path/lzvm,2884,34.975",
            "lzvm_prover::guest_machine::advance_guest_machine_prepared_inner,/path/lzvm,1066,12.927",
            "core::ptr::drop_in_place$LT$lzvm_prover..guest_pc_trace_backend..GuestPcTracePendingSegmentSlice$GT$::hash,/path/lzvm,376,4.560",
        ]
        .join("\n"),
    )
    .expect("sibling nsys CPU summary should be written");

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
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_lowered_report_row_self_pct"), "34.975");
    assert_eq!(value("perf_advance_guest_machine_self_pct"), "12.927");
    assert_eq!(value("perf_pending_segment_drop_self_pct"), "4.560");
    assert_eq!(value("cpu_runner_hotspot_hint"), "guest_machine_advance");
}

#[test]
fn prove_timing_root_summary_reads_short_sibling_nsys_cpu_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let temp_dir = crate_root.join("../../temp").join(format!(
        "prove-timing-root-summary-short-nsys-cpu-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("test temp directory should be created");
    let log_path = temp_dir.join("sample.log");
    let cpu_summary_path = temp_dir.join("sample.cpu.txt");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=48330",
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing log should be written");
    std::fs::write(
        &cpu_summary_path,
        [
            "application_cpu_hotspots",
            "symbol,module,samples,application_sample_pct",
            "lzvm_prover::guest_machine::prepare_current_guest_instruction,/path/lzvm,971,11.234",
            "lzvm_prover::guest_machine::advance_guest_machine_prepared_inner,/path/lzvm,1094,12.663",
        ]
        .join("\n"),
    )
    .expect("short sibling nsys CPU summary should be written");

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
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_prepare_instruction_self_pct"), "11.234");
    assert_eq!(value("perf_advance_guest_machine_self_pct"), "12.663");
    assert_eq!(
        value("cpu_runner_hotspot_hint"),
        "instruction_prepare_and_advance"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_report_storage_action_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=8076",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    18.96%  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "    10.73%  [.] memmove",
        "     8.67%  [.] lzvm_prover::guest_pc_trace_backend::GuestPcTraceSegmentSlice::from_segment_trace",
        "     5.72%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
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
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("cpu_trace_hotspot_hint"),
        "report_lifetime_and_data_movement"
    );
    assert_eq!(
        value("cpu_trace_report_storage_action_hint"),
        "report_sidecar_storage_candidate"
    );
}

#[test]
fn prove_timing_root_summary_flags_missing_trace_report_storage_fields() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=51642",
        "timing_guest_trace_reports=499520693",
        "timing_guest_trace_runner_ms=42305",
        "timing_guest_trace_lowerer_ms=42305",
        "timing_guest_trace_stream_elapsed_ms=42310",
        "timing_guest_trace_segment_receive_wait_ms=22093",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
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
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_report_record_size_bytes"), "0");
    assert_eq!(
        value("cpu_trace_report_storage_action_hint"),
        "refresh_trace_report_storage_timing"
    );
}

#[test]
fn prove_timing_root_summary_reports_lowerer_perf_action_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=50750",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    16.21%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "     6.12%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::append_main_device_trace_descriptor",
        "     2.92%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::zisk_main_source_value",
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
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_append_descriptor_self_pct"), "6.120");
    assert_eq!(value("perf_source_value_self_pct"), "2.920");
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "descriptor_append_candidate"
    );
}

#[test]
fn prove_timing_root_summary_prioritizes_trace_pipeline_over_secondary_opening_launches() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=51505",
        "timing_guest_trace_runner_ms=42145",
        "timing_guest_trace_lowerer_ms=42195",
        "timing_guest_trace_lower_ms=38619",
        "timing_guest_trace_stream_elapsed_ms=42407",
        "timing_guest_segment_commit_ms=21355",
        "timing_guest_trace_segment_receive_wait_ms=22300",
        "timing_guest_trace_parallel_lower_workers=1",
        "timing_guest_stage_leaf_kernel_work_ms=11000",
        "timing_finish_witness_opening_ms=9600",
        "timing_finish_witness_opening_query_unit_count=41",
        "timing_finish_witness_opening_single_query_unit_count=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=41",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=410",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=92",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=459",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=93",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
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
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("opening_retained_parent_checkpoint_action_hint"),
        "retained_parent_checkpoint_path_time_secondary"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "trace_pipeline_over_secondary_opening_launches"
    );
}

#[test]
fn prove_timing_root_summary_reports_runner_perf_hotspots() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=1000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "     5.30%  [.] lzvm_prover::guest_machine::prepare_current_guest_instruction",
        "     5.17%  [.] lzvm_prover::guest_pc_trace_backend::build_layout_zisk_main_trace_segment_for_segment_output",
        "     4.26%  [.] lzvm_prover::guest_machine::advance_guest_machine_prepared_inner",
        "     2.81%  [.] lzvm_prover::guest_machine::memory::GuestMachineMemorySegment::write_range",
        "     1.97%  [.] num_bigint::biguint::monty::monty_modpow",
        "     1.70%  [.] lzvm_prover::guest_machine::memory::GuestMachineMemory::read_range_into",
        "     0.26%  [.] lzvm_prover::guest_machine::GuestInstructionEffects::record_memory_write",
        "     0.14%  [.] lzvm_prover::guest_machine::GuestInstructionEffects::record_memory_read",
        "     0.03%  [.] lzvm_prover::guest_instruction::decode_guest_instruction",
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
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("summary should include {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should include {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_prepare_instruction_self_pct"), "5.300");
    assert_eq!(value("perf_trace_segment_build_self_pct"), "5.170");
    assert_eq!(value("perf_advance_guest_machine_self_pct"), "4.260");
    assert_eq!(value("perf_guest_memory_write_self_pct"), "2.810");
    assert_eq!(value("perf_biguint_modpow_self_pct"), "1.970");
    assert_eq!(value("perf_guest_memory_read_self_pct"), "1.700");
    assert_eq!(value("perf_decode_instruction_self_pct"), "0.030");
    assert_eq!(value("perf_effect_record_memory_write_self_pct"), "0.260");
    assert_eq!(value("perf_effect_record_memory_read_self_pct"), "0.140");
    assert_eq!(
        value("cpu_runner_hotspot_hint"),
        "instruction_prepare_and_advance"
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

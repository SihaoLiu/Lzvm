#!/usr/bin/env python3
import argparse
import re
import sys
from pathlib import Path


INPUT_BYTES_KEY = "input_bytes"
ROOT_COUNT_KEY = "timing_guest_stage_tree_commit_root_count"
ROOT_GROUPS_KEY = "timing_guest_stage_tree_commit_root_materialization_groups"
ROOT_MAX_GROUP_KEY = "timing_guest_stage_tree_commit_root_materialization_max_group_size"
TOTAL_MS_KEY = "timing_total_ms"
CONSTANT_MATERIAL_VALIDATION_ELAPSED_MS_KEY = (
    "timing_constant_material_validation_elapsed_ms"
)
CONSTANT_MATERIAL_VALIDATION_JOIN_WAIT_MS_KEY = (
    "timing_constant_material_validation_join_wait_ms"
)
RUNNER_MS_KEY = "timing_guest_trace_runner_ms"
LOWERER_MS_KEY = "timing_guest_trace_lowerer_ms"
STREAM_ELAPSED_MS_KEY = "timing_guest_trace_stream_elapsed_ms"
STREAM_WORKER_MS_KEY = "timing_guest_trace_stream_ms"
SEGMENT_COMMIT_MS_KEY = "timing_guest_segment_commit_ms"
SEGMENT_COMMIT_INITIAL_WORKERS_KEY = "timing_guest_segment_commit_initial_workers"
SEGMENT_COMMIT_EFFECTIVE_WORKERS_KEY = "timing_guest_segment_commit_effective_workers"
SEGMENT_COMMIT_OOM_RETRIES_KEY = "timing_guest_segment_commit_oom_retries"
SEGMENT_RECEIVE_WAIT_MS_KEY = "timing_guest_trace_segment_receive_wait_ms"
PENDING_RECEIVE_WAIT_MS_KEY = "timing_guest_trace_pending_receive_wait_ms"
PENDING_SEND_WAIT_MS_KEY = "timing_guest_trace_pending_send_wait_ms"
PARALLEL_LOWER_WORKERS_KEY = "timing_guest_trace_parallel_lower_workers"
PARALLEL_LOWER_DISPATCHED_KEY = "timing_guest_trace_parallel_lower_dispatched"
PARALLEL_LOWER_RECEIVED_KEY = "timing_guest_trace_parallel_lower_received"
PARALLEL_LOWER_EMITTED_KEY = "timing_guest_trace_parallel_lower_emitted"
PARALLEL_LOWER_MAX_REORDER_KEY = "timing_guest_trace_parallel_lower_max_reorder"
TRACE_REPORTS_KEY = "timing_guest_trace_reports"
TRACE_REPORT_ROWS_KEY = "timing_guest_trace_report_rows"
TRACE_SINGLE_ROW_REPORTS_KEY = "timing_guest_trace_single_row_reports"
TRACE_MULTI_ROW_REPORTS_KEY = "timing_guest_trace_multi_row_reports"
TRACE_PENDING_DMA_REPORTS_KEY = "timing_guest_trace_pending_dma_reports"
TRACE_AMO_REPORTS_KEY = "timing_guest_trace_amo_reports"
TRACE_STORE_CONDITIONAL_REPORTS_KEY = "timing_guest_trace_store_conditional_reports"
TRACE_EXTERNAL_OP_ROWS_KEY = "timing_guest_trace_external_op_rows"
TRACE_COPY_ROWS_KEY = "timing_guest_trace_copy_rows"
TRACE_FLAG_ROWS_KEY = "timing_guest_trace_flag_rows"
TRACE_PRECOMPILE_ROWS_KEY = "timing_guest_trace_precompile_rows"
TRACE_INDIRECT_MEMORY_ROWS_KEY = "timing_guest_trace_indirect_memory_rows"
TRACE_REGISTER_SOURCE_READS_KEY = "timing_guest_trace_register_source_reads"
TRACE_MEMORY_SOURCE_READS_KEY = "timing_guest_trace_memory_source_reads"
TRACE_REGISTER_STORE_ROWS_KEY = "timing_guest_trace_register_store_rows"
TRACE_MEMORY_STORE_ROWS_KEY = "timing_guest_trace_memory_store_rows"
TRACE_NO_STORE_ROWS_KEY = "timing_guest_trace_no_store_rows"
TRACE_SHAPE_KEYS = (
    TRACE_SINGLE_ROW_REPORTS_KEY,
    TRACE_MULTI_ROW_REPORTS_KEY,
    TRACE_PENDING_DMA_REPORTS_KEY,
    TRACE_AMO_REPORTS_KEY,
    TRACE_STORE_CONDITIONAL_REPORTS_KEY,
    TRACE_EXTERNAL_OP_ROWS_KEY,
    TRACE_COPY_ROWS_KEY,
    TRACE_FLAG_ROWS_KEY,
    TRACE_PRECOMPILE_ROWS_KEY,
    TRACE_INDIRECT_MEMORY_ROWS_KEY,
    TRACE_REGISTER_SOURCE_READS_KEY,
    TRACE_MEMORY_SOURCE_READS_KEY,
    TRACE_REGISTER_STORE_ROWS_KEY,
    TRACE_MEMORY_STORE_ROWS_KEY,
    TRACE_NO_STORE_ROWS_KEY,
)
TRACE_REPORT_DETAIL_SAMPLES_KEY = "timing_guest_trace_report_detail_samples"
TRACE_REPORT_SAMPLED_NS_KEY = "timing_guest_trace_report_sampled_ns"
TRACE_REPORT_LOWERING_SAMPLED_NS_KEY = "timing_guest_trace_report_lowering_sampled_ns"
TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_row_validation_sampled_ns"
)
TRACE_REPORT_MEMORY_COLUMNS_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_memory_columns_sampled_ns"
)
TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_values_sampled_ns"
)
TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_a_value_sampled_ns"
)
TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_b_value_sampled_ns"
)
TRACE_REPORT_PRECOMPILE_MEMORY_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_precompile_memory_sampled_ns"
)
TRACE_REPORT_INSTRUCTION_RESULT_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_instruction_result_sampled_ns"
)
TRACE_REPORT_NEXT_PC_SAMPLED_NS_KEY = "timing_guest_trace_report_next_pc_sampled_ns"
TRACE_REPORT_REGISTER_ACCESS_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_register_access_sampled_ns"
)
TRACE_REPORT_MEMORY_ACCESS_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_memory_access_sampled_ns"
)
TRACE_REPORT_STORE_APPLY_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_store_apply_sampled_ns"
)
TRACE_REPORT_VISIT_SAMPLED_NS_KEY = "timing_guest_trace_report_visit_sampled_ns"
TRACE_DESCRIPTOR_SAMPLED_NS_KEY = "timing_guest_trace_descriptor_sampled_ns"
TRACE_REPORT_BUFFER_CAPACITY_KEY = "timing_guest_trace_report_buffer_capacity"
TRACE_REPORT_BUFFER_MAX_CAPACITY_KEY = "timing_guest_trace_report_buffer_max_capacity"
TRACE_REPORT_BUFFER_EXCESS_CAPACITY_KEY = (
    "timing_guest_trace_report_buffer_excess_capacity"
)
DESCRIPTOR_ROWS_KEY = "timing_guest_trace_descriptor_rows"
DESCRIPTOR_COMPACT_ROWS_KEY = "timing_guest_trace_descriptor_compact_rows"
DESCRIPTOR_WIDE_ROWS_KEY = "timing_guest_trace_descriptor_wide_rows"
DESCRIPTOR_UPLOAD_BYTES_KEY = "timing_guest_device_source_descriptor_upload_bytes"
DESCRIPTOR_UPLOAD_ROWS_KEY = "timing_guest_device_source_descriptor_upload_rows"
DESCRIPTOR_HIGH32_VALUES_KEY = (
    "timing_guest_trace_descriptor_unpaired_high32_nonzero_values"
)
DESCRIPTOR_HIGH32_ROWS_KEY = (
    "timing_guest_trace_descriptor_unpaired_high32_nonzero_rows"
)
DESCRIPTOR_HIGH32_STATS_ENABLED_KEY = (
    "timing_guest_trace_descriptor_high32_stats_enabled"
)
SEED_DIRECT_LIFT_ATTEMPTS_KEY = "timing_guest_trace_seed_direct_lift_attempts"
SEED_DIRECT_LIFT_SUCCESSES_KEY = "timing_guest_trace_seed_direct_lift_successes"
SEED_FULL_ADVANCES_KEY = "timing_guest_trace_seed_full_advances"
FINISH_OPENING_MS_KEY = "timing_finish_witness_opening_ms"
OPENING_QUERY_COUNT_KEY = "timing_finish_witness_opening_query_count"
OPENING_QUERY_UNITS_KEY = "timing_finish_witness_opening_query_unit_count"
OPENING_SINGLE_QUERY_UNITS_KEY = "timing_finish_witness_opening_single_query_unit_count"
OPENING_MAX_QUERIES_PER_UNIT_KEY = (
    "timing_finish_witness_opening_max_queries_per_unit"
)
OPENING_STAGE_COUNT_KEY = "timing_finish_witness_opening_stage_count"
OPENING_RETAINED_SOURCE_COUNT_KEY = "timing_finish_witness_opening_retained_source_count"
OPENING_EXTERNAL_SOURCE_COUNT_KEY = "timing_finish_witness_opening_external_source_count"
OPENING_EMBEDDED_SOURCE_COUNT_KEY = "timing_finish_witness_opening_embedded_source_count"
OPENING_MISSING_SOURCE_COUNT_KEY = "timing_finish_witness_opening_missing_source_count"
OPENING_ROW_VALUE_DEVICE_ROWS_KEY = "timing_finish_witness_opening_row_values_device_rows"
OPENING_ROW_VALUE_SOURCE_ROWS_KEY = "timing_finish_witness_opening_row_values_source_rows"
OPENING_RETAINED_LEAF_COUNT_KEY = (
    "timing_finish_witness_opening_retained_leaf_digest_openings"
)
OPENING_RETAINED_LEAF_ROWS_KEY = "timing_finish_witness_opening_retained_leaf_digest_rows"
OPENING_RETAINED_LEAF_ALL_SINGLE_ROW_KEY = (
    "timing_finish_witness_opening_retained_leaf_digest_all_single_row_openings"
)
OPENING_RETAINED_LEAF_PATH_LAUNCHES_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_launches"
)
OPENING_RETAINED_PARENT_CHECKPOINT_COUNT_KEY = (
    "timing_finish_witness_opening_retained_parent_checkpoint_openings"
)
OPENING_RETAINED_PARENT_CHECKPOINT_ROWS_KEY = (
    "timing_finish_witness_opening_retained_parent_checkpoint_rows"
)
OPENING_RETAINED_PARENT_CHECKPOINT_ALL_SINGLE_ROW_KEY = (
    "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings"
)
OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_LAUNCHES_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches"
)
OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_ROWS_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows"
)
OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_BYTES_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes"
)
OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_LAUNCHES_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches"
)
OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_ROWS_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows"
)
OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_BYTES_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes"
)
OPENING_PATH_PARENT_HASH_LAUNCHES_PER_STAGE_KEY = (
    "timing_finish_witness_opening_path_parent_hash_launches_per_stage"
)
OPENING_ROW_VALUE_DEVICE_DOWNLOAD_BATCHES_KEY = (
    "timing_finish_witness_opening_row_values_device_download_batches"
)
LEAF_KERNEL_MS_KEY = "timing_guest_stage_leaf_kernel_work_ms"
LEAF_COSET_CALLS_KEY = "timing_guest_stage_leaf_coset_extend_calls"
LEAF_COSET_COLUMNS_KEY = "timing_guest_stage_leaf_coset_extend_columns"
LEAF_NTT_LAUNCHES_KEY = "timing_guest_stage_leaf_coset_extend_ntt_launches"
LEAF_NTT_STAGE_LAUNCHES_KEY = "timing_guest_stage_leaf_coset_extend_ntt_stage_launches"
LEAF_NTT_BLOCK_TWIDDLE_LAUNCHES_KEY = (
    "timing_guest_stage_leaf_coset_extend_ntt_block_twiddle_launches"
)
DIRECT_D2H_WAIT_NS_KEY = "timing_cuda_direct_copy_d2h_wait_ns"
DIRECT_D2H_HOT_BYTES_KEY = "timing_cuda_direct_copy_d2h_hot_bytes"
DIRECT_D2H_HOT_COUNT_KEY = "timing_cuda_direct_copy_d2h_hot_count"
DIRECT_D2H_HOT_WAIT_NS_KEY = "timing_cuda_direct_copy_d2h_hot_wait_ns"
PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY = "perf_lowered_report_row_self_pct"
PERF_MEMMOVE_SELF_PCT_KEY = "perf_memmove_self_pct"
PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY = "perf_memmove_guest_machine_pct"
PERF_MEMMOVE_TRACE_SLICE_PCT_KEY = "perf_memmove_trace_slice_pct"
PERF_MEMMOVE_RUNNER_THREAD_PCT_KEY = "perf_memmove_runner_thread_pct"
PERF_MEMMOVE_LOWER_THREAD_PCT_KEY = "perf_memmove_lower_thread_pct"
PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY = "perf_pending_segment_drop_self_pct"
PERF_SHA256_SELF_PCT_KEY = "perf_sha256_self_pct"
PERF_SHA256_GUEST_MACHINE_PCT_KEY = "perf_sha256_guest_machine_pct"
PERF_SHA256_TRACE_SLICE_PCT_KEY = "perf_sha256_trace_slice_pct"
ROOT_PIPELINE_INPUT_BYTE_LIMIT = 8 * 1024 * 1024
OPENING_BATCHING_D2H_WAIT_MS_THRESHOLD = 100.0
PROOF_TARGET_MS = 12_000
PERF_SELF_PERCENT_RE = re.compile(r"^\s*(\d+(?:\.\d+)?)%\s+(.*)$")
PERF_SECOND_SELF_PERCENT_RE = re.compile(r"^\s*(\d+(?:\.\d+)?)%\s+(.*)$")
PERF_CALLCHAIN_PERCENT_RE = re.compile(r"(\d+(?:\.\d+)?)%--(.*)$")

HEADER = (
    "profile,input_bytes,total_ms,constant_material_validation_elapsed_ms,"
    "constant_material_validation_join_wait_ms,constant_material_validation_overlap_hint,"
    "runner_ms,lowerer_ms,stream_elapsed_ms,stream_worker_ms,"
    "segment_commit_ms,segment_commit_initial_workers,"
    "segment_commit_effective_workers,segment_commit_oom_retries,"
    "stream_commit_residual_ms,segment_receive_wait_ms,"
    "pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,"
    "parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,"
    "parallel_lower_max_reorder,trace_reports,trace_report_rows,"
    "trace_rows_per_report,trace_report_buffer_capacity,"
    "trace_report_buffer_max_capacity,trace_report_buffer_excess_capacity,"
    "trace_report_buffer_excess_pct,trace_report_buffer_shape_hint,"
    "trace_report_lifetime_hint,"
    "descriptor_rows,descriptor_compact_rows,"
    "descriptor_wide_rows,descriptor_upload_bytes,descriptor_bytes_per_row,"
    "descriptor_high32_nonzero_values,descriptor_high32_nonzero_rows,"
    "descriptor_high32_row_pct,descriptor_shape_hint,seed_direct_lift_attempts,"
    "seed_direct_lift_successes,seed_full_advances,"
    "finish_opening_ms,opening_query_units,opening_single_query_units,"
    "opening_queries,opening_max_queries_per_unit,opening_stage_count,"
    "opening_source_shape_hint,opening_row_value_device_rows,"
    "opening_row_value_source_rows,"
    "retained_leaf_openings,retained_leaf_rows,retained_leaf_all_single_row,"
    "retained_leaf_path_launches,retained_parent_checkpoint_openings,"
    "retained_parent_checkpoint_rows,retained_parent_checkpoint_all_single_row,"
    "retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,"
    "retained_parent_checkpoint_prefix_launches,"
    "retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,"
    "retained_parent_checkpoint_suffix_launches,"
    "opening_path_parent_hash_launches_per_stage,"
    "opening_row_value_device_download_batches,opening_batching_hint,"
    "root_count,materialization_groups,"
    "materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline,"
    "root_pipeline_policy_hint,leaf_kernel_ms,leaf_coset_calls,leaf_coset_columns,leaf_ntt_launches,"
    "leaf_ntt_stage_launches,leaf_ntt_block_twiddle_launches,"
    "leaf_ntt_launches_per_call,direct_d2h_wait_ms,leaf_launch_pressure,"
    "trace_to_leaf_ratio,primary_bottleneck,trace_structure_hint,"
    "proof_12s_gap_ms,proof_12s_gap_hint,"
    "perf_lowered_report_row_self_pct,perf_memmove_self_pct,perf_memmove_guest_machine_pct,"
    "perf_memmove_trace_slice_pct,perf_memmove_source_hint,"
    "perf_pending_segment_drop_self_pct,perf_sha256_self_pct,"
    "perf_sha256_source_hint,cpu_trace_hotspot_hint,"
    "single_row_reports,multi_row_reports,pending_dma_reports,amo_reports,"
    "store_conditional_reports,external_op_rows,copy_rows,flag_rows,"
    "precompile_rows,indirect_memory_rows,indirect_memory_row_pct,"
    "register_source_reads,memory_source_reads,memory_source_read_pct,"
    "register_store_rows,memory_store_rows,memory_store_row_pct,"
    "no_store_rows,no_store_row_pct,trace_shape_sample_hint,"
    "trace_report_detail_samples,trace_report_detail_sample_pct,"
    "trace_report_detail_sample_ppm,trace_report_detail_sample_hint,"
    "trace_report_detail_avg_ns,"
    "trace_report_detail_hotspot,trace_report_detail_hotspot_pct,"
    "trace_report_row_validation_hotspot,trace_report_row_validation_hotspot_pct,"
    "trace_report_row_validation_explained_pct,trace_report_row_validation_residual_pct,"
    "trace_report_source_values_lookup_pct,trace_report_source_values_residual_pct,"
    "trace_report_detail_visit_pct,trace_report_visit_descriptor_pct,"
    "trace_report_visit_residual_pct,"
    "direct_d2h_hot_bytes,direct_d2h_hot_count,direct_d2h_hot_wait_ms"
)
AGGREGATE_HEADER = (
    "aggregate,total_count,valid_total_count,total_min_ms,total_mean_ms,"
    "total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier"
)
CLOSE_SAMPLE_SPREAD_PCT = 5.0
OUTLIER_RATIO_THRESHOLD = 1.5

TIMING_KEYS = {
    INPUT_BYTES_KEY,
    TOTAL_MS_KEY,
    CONSTANT_MATERIAL_VALIDATION_ELAPSED_MS_KEY,
    CONSTANT_MATERIAL_VALIDATION_JOIN_WAIT_MS_KEY,
    RUNNER_MS_KEY,
    LOWERER_MS_KEY,
    STREAM_ELAPSED_MS_KEY,
    STREAM_WORKER_MS_KEY,
    SEGMENT_COMMIT_MS_KEY,
    SEGMENT_COMMIT_INITIAL_WORKERS_KEY,
    SEGMENT_COMMIT_EFFECTIVE_WORKERS_KEY,
    SEGMENT_COMMIT_OOM_RETRIES_KEY,
    SEGMENT_RECEIVE_WAIT_MS_KEY,
    PENDING_RECEIVE_WAIT_MS_KEY,
    PENDING_SEND_WAIT_MS_KEY,
    PARALLEL_LOWER_WORKERS_KEY,
    PARALLEL_LOWER_DISPATCHED_KEY,
    PARALLEL_LOWER_RECEIVED_KEY,
    PARALLEL_LOWER_EMITTED_KEY,
    PARALLEL_LOWER_MAX_REORDER_KEY,
    TRACE_REPORTS_KEY,
    TRACE_REPORT_ROWS_KEY,
    TRACE_SINGLE_ROW_REPORTS_KEY,
    TRACE_MULTI_ROW_REPORTS_KEY,
    TRACE_PENDING_DMA_REPORTS_KEY,
    TRACE_AMO_REPORTS_KEY,
    TRACE_STORE_CONDITIONAL_REPORTS_KEY,
    TRACE_EXTERNAL_OP_ROWS_KEY,
    TRACE_COPY_ROWS_KEY,
    TRACE_FLAG_ROWS_KEY,
    TRACE_PRECOMPILE_ROWS_KEY,
    TRACE_INDIRECT_MEMORY_ROWS_KEY,
    TRACE_REGISTER_SOURCE_READS_KEY,
    TRACE_MEMORY_SOURCE_READS_KEY,
    TRACE_REGISTER_STORE_ROWS_KEY,
    TRACE_MEMORY_STORE_ROWS_KEY,
    TRACE_NO_STORE_ROWS_KEY,
    TRACE_REPORT_DETAIL_SAMPLES_KEY,
    TRACE_REPORT_SAMPLED_NS_KEY,
    TRACE_REPORT_LOWERING_SAMPLED_NS_KEY,
    TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY,
    TRACE_REPORT_MEMORY_COLUMNS_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY,
    TRACE_REPORT_PRECOMPILE_MEMORY_SAMPLED_NS_KEY,
    TRACE_REPORT_INSTRUCTION_RESULT_SAMPLED_NS_KEY,
    TRACE_REPORT_NEXT_PC_SAMPLED_NS_KEY,
    TRACE_REPORT_REGISTER_ACCESS_SAMPLED_NS_KEY,
    TRACE_REPORT_MEMORY_ACCESS_SAMPLED_NS_KEY,
    TRACE_REPORT_STORE_APPLY_SAMPLED_NS_KEY,
    TRACE_REPORT_VISIT_SAMPLED_NS_KEY,
    TRACE_DESCRIPTOR_SAMPLED_NS_KEY,
    TRACE_REPORT_BUFFER_CAPACITY_KEY,
    TRACE_REPORT_BUFFER_MAX_CAPACITY_KEY,
    TRACE_REPORT_BUFFER_EXCESS_CAPACITY_KEY,
    DESCRIPTOR_ROWS_KEY,
    DESCRIPTOR_COMPACT_ROWS_KEY,
    DESCRIPTOR_WIDE_ROWS_KEY,
    DESCRIPTOR_UPLOAD_BYTES_KEY,
    DESCRIPTOR_UPLOAD_ROWS_KEY,
    DESCRIPTOR_HIGH32_VALUES_KEY,
    DESCRIPTOR_HIGH32_ROWS_KEY,
    DESCRIPTOR_HIGH32_STATS_ENABLED_KEY,
    SEED_DIRECT_LIFT_ATTEMPTS_KEY,
    SEED_DIRECT_LIFT_SUCCESSES_KEY,
    SEED_FULL_ADVANCES_KEY,
    FINISH_OPENING_MS_KEY,
    OPENING_QUERY_COUNT_KEY,
    OPENING_QUERY_UNITS_KEY,
    OPENING_SINGLE_QUERY_UNITS_KEY,
    OPENING_MAX_QUERIES_PER_UNIT_KEY,
    OPENING_STAGE_COUNT_KEY,
    OPENING_RETAINED_SOURCE_COUNT_KEY,
    OPENING_EXTERNAL_SOURCE_COUNT_KEY,
    OPENING_EMBEDDED_SOURCE_COUNT_KEY,
    OPENING_MISSING_SOURCE_COUNT_KEY,
    OPENING_ROW_VALUE_DEVICE_ROWS_KEY,
    OPENING_ROW_VALUE_SOURCE_ROWS_KEY,
    OPENING_RETAINED_LEAF_COUNT_KEY,
    OPENING_RETAINED_LEAF_ROWS_KEY,
    OPENING_RETAINED_LEAF_ALL_SINGLE_ROW_KEY,
    OPENING_RETAINED_LEAF_PATH_LAUNCHES_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_COUNT_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_ROWS_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_ALL_SINGLE_ROW_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_ROWS_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_BYTES_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_LAUNCHES_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_ROWS_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_BYTES_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_LAUNCHES_KEY,
    OPENING_PATH_PARENT_HASH_LAUNCHES_PER_STAGE_KEY,
    OPENING_ROW_VALUE_DEVICE_DOWNLOAD_BATCHES_KEY,
    ROOT_COUNT_KEY,
    ROOT_GROUPS_KEY,
    ROOT_MAX_GROUP_KEY,
    LEAF_KERNEL_MS_KEY,
    LEAF_COSET_CALLS_KEY,
    LEAF_COSET_COLUMNS_KEY,
    LEAF_NTT_LAUNCHES_KEY,
    LEAF_NTT_STAGE_LAUNCHES_KEY,
    LEAF_NTT_BLOCK_TWIDDLE_LAUNCHES_KEY,
    DIRECT_D2H_WAIT_NS_KEY,
    DIRECT_D2H_HOT_BYTES_KEY,
    DIRECT_D2H_HOT_COUNT_KEY,
    DIRECT_D2H_HOT_WAIT_NS_KEY,
}


def parse_timing_log(text: str) -> dict[str, int]:
    values: dict[str, int] = {}
    for line in text.splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        if key not in TIMING_KEYS:
            continue
        try:
            values[key] = int(value.strip())
        except ValueError:
            continue
    return values


def parse_perf_self_hotspots(text: str) -> dict[str, float]:
    hotspots = {
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY: 0.0,
        PERF_MEMMOVE_SELF_PCT_KEY: 0.0,
        PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY: 0.0,
        PERF_MEMMOVE_TRACE_SLICE_PCT_KEY: 0.0,
        PERF_MEMMOVE_RUNNER_THREAD_PCT_KEY: 0.0,
        PERF_MEMMOVE_LOWER_THREAD_PCT_KEY: 0.0,
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY: 0.0,
        PERF_SHA256_SELF_PCT_KEY: 0.0,
        PERF_SHA256_GUEST_MACHINE_PCT_KEY: 0.0,
        PERF_SHA256_TRACE_SLICE_PCT_KEY: 0.0,
    }
    in_memmove_callchain = False
    in_sha256_callchain = False
    for line in text.splitlines():
        match = PERF_SELF_PERCENT_RE.match(line)
        if match:
            try:
                pct = float(match.group(1))
            except ValueError:
                continue
            symbol_text = match.group(2)
            second_pct_match = PERF_SECOND_SELF_PERCENT_RE.match(symbol_text)
            if second_pct_match:
                try:
                    pct = float(second_pct_match.group(1))
                    symbol_text = second_pct_match.group(2)
                except ValueError:
                    pass
            in_memmove_callchain = "memmove" in symbol_text
            in_sha256_callchain = (
                "sha2::sha256" in symbol_text or "digest_blocks" in symbol_text
            )
            if "lowered_report_row" in symbol_text:
                key = PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY
            elif "memmove" in symbol_text:
                key = PERF_MEMMOVE_SELF_PCT_KEY
                if "lzvm-gp-runner" in symbol_text:
                    hotspots[PERF_MEMMOVE_RUNNER_THREAD_PCT_KEY] = max(
                        hotspots[PERF_MEMMOVE_RUNNER_THREAD_PCT_KEY], pct
                    )
                elif "lzvm-gp-lower" in symbol_text:
                    hotspots[PERF_MEMMOVE_LOWER_THREAD_PCT_KEY] = max(
                        hotspots[PERF_MEMMOVE_LOWER_THREAD_PCT_KEY], pct
                    )
            elif in_sha256_callchain:
                key = PERF_SHA256_SELF_PCT_KEY
            elif (
                "GuestPcTracePendingSegmentSlice" in symbol_text
                and "drop_in_place" in symbol_text
            ):
                key = PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY
            else:
                continue
            hotspots[key] = max(hotspots[key], pct)
            continue

        if not in_memmove_callchain:
            if not in_sha256_callchain:
                continue
        callchain_match = PERF_CALLCHAIN_PERCENT_RE.search(line)
        if not callchain_match:
            continue
        try:
            pct = float(callchain_match.group(1))
        except ValueError:
            continue
        symbol_text = callchain_match.group(2)
        key = None
        if in_memmove_callchain:
            if "advance_guest_machine_prepared_inner" in symbol_text:
                key = PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY
            elif "run_guest_pc_trace_segment_slice" in symbol_text:
                key = PERF_MEMMOVE_TRACE_SLICE_PCT_KEY
        if in_sha256_callchain:
            if "advance_guest_machine_prepared_inner" in symbol_text:
                key = PERF_SHA256_GUEST_MACHINE_PCT_KEY
            elif "run_guest_pc_trace_segment_slice" in symbol_text:
                key = PERF_SHA256_TRACE_SLICE_PCT_KEY
        if key is None:
            continue
        hotspots[key] = max(hotspots[key], pct)
    return hotspots


def primary_bottleneck(
    total_ms: int,
    runner_ms: int,
    lowerer_ms: int,
    stream_elapsed_ms: int,
    stream_worker_ms: int,
    segment_commit_ms: int,
    segment_receive_wait_ms: int,
    finish_opening_ms: int,
    leaf_kernel_ms: int,
    direct_d2h_wait_ms: float,
) -> str:
    candidates = [
        ("trace_runner", float(runner_ms)),
        ("trace_lowerer", float(lowerer_ms)),
        ("stream_elapsed", float(stream_elapsed_ms)),
        ("stream_worker", float(stream_worker_ms)),
        ("segment_commit", float(segment_commit_ms)),
        ("segment_receive_wait", float(segment_receive_wait_ms)),
        ("finish_opening", float(finish_opening_ms)),
        ("leaf_kernel", float(leaf_kernel_ms)),
        ("direct_d2h_wait", direct_d2h_wait_ms),
    ]
    name, value = max(candidates, key=lambda item: item[1])
    return name if value > 0.0 else "total" if total_ms > 0 else "unknown"


def proof_target_gap_hint(
    total_ms: int,
    runner_ms: int,
    lowerer_ms: int,
    stream_elapsed_ms: int,
    segment_commit_ms: int,
    finish_opening_ms: int,
    leaf_kernel_ms: int,
    direct_d2h_wait_ms: float,
) -> str:
    if total_ms <= 0:
        return "unknown"
    if total_ms <= PROOF_TARGET_MS:
        return "within_12s_target"
    trace_floor_ms = max(runner_ms, lowerer_ms, stream_elapsed_ms)
    gpu_or_opening_ms = max(
        float(segment_commit_ms),
        float(finish_opening_ms),
        float(leaf_kernel_ms),
        direct_d2h_wait_ms,
    )
    if trace_floor_ms <= 0 and gpu_or_opening_ms <= 0.0:
        return "target_gap_needs_timing_breakdown"
    if trace_floor_ms >= PROOF_TARGET_MS and trace_floor_ms >= gpu_or_opening_ms:
        return "cpu_trace_generation_above_target"
    if float(segment_commit_ms) >= PROOF_TARGET_MS or float(leaf_kernel_ms) >= PROOF_TARGET_MS:
        return "gpu_commit_above_target"
    if float(finish_opening_ms) >= PROOF_TARGET_MS or direct_d2h_wait_ms >= PROOF_TARGET_MS:
        return "opening_above_target"
    if trace_floor_ms >= gpu_or_opening_ms * 1.5:
        return "cpu_trace_generation_dominant_gap"
    if gpu_or_opening_ms >= float(trace_floor_ms) * 1.5:
        return "gpu_or_opening_dominant_gap"
    return "mixed_pipeline_gap"


def cpu_trace_hotspot_hint(perf_hotspots: dict[str, float]) -> str:
    lowered_report_row_pct = perf_hotspots.get(
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY, 0.0
    )
    memmove_pct = perf_hotspots.get(PERF_MEMMOVE_SELF_PCT_KEY, 0.0)
    pending_drop_pct = perf_hotspots.get(
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY, 0.0
    )
    if lowered_report_row_pct >= 20.0 and memmove_pct >= 15.0:
        return "report_lifetime_and_data_movement"
    if lowered_report_row_pct >= 20.0:
        return "lowered_report_rows"
    if memmove_pct >= 15.0:
        return "guest_state_copies"
    if pending_drop_pct >= 5.0:
        return "pending_segment_lifetime"
    return "none"


def memmove_source_hint(perf_hotspots: dict[str, float]) -> str:
    guest_machine_pct = perf_hotspots.get(PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY, 0.0)
    trace_slice_pct = perf_hotspots.get(PERF_MEMMOVE_TRACE_SLICE_PCT_KEY, 0.0)
    if guest_machine_pct > 0.0 and trace_slice_pct > 0.0:
        if guest_machine_pct >= trace_slice_pct * 1.25:
            return "guest_machine_dominant"
        if trace_slice_pct >= guest_machine_pct * 1.25:
            return "trace_slice_dominant"
        return "guest_machine_and_trace_slice"
    if guest_machine_pct > 0.0:
        return "guest_machine"
    if trace_slice_pct > 0.0:
        return "trace_slice"
    runner_thread_pct = perf_hotspots.get(PERF_MEMMOVE_RUNNER_THREAD_PCT_KEY, 0.0)
    lower_thread_pct = perf_hotspots.get(PERF_MEMMOVE_LOWER_THREAD_PCT_KEY, 0.0)
    if runner_thread_pct > 0.0 and lower_thread_pct > 0.0:
        if runner_thread_pct >= lower_thread_pct * 1.25:
            return "guest_runner_thread"
        if lower_thread_pct >= runner_thread_pct * 1.25:
            return "trace_lower_thread"
        return "guest_runner_and_trace_lower_threads"
    if runner_thread_pct > 0.0:
        return "guest_runner_thread"
    if lower_thread_pct > 0.0:
        return "trace_lower_thread"
    return "none"


def sha256_source_hint(perf_hotspots: dict[str, float]) -> str:
    sha256_pct = perf_hotspots.get(PERF_SHA256_SELF_PCT_KEY, 0.0)
    if sha256_pct <= 0.0:
        return "none"
    guest_machine_pct = perf_hotspots.get(PERF_SHA256_GUEST_MACHINE_PCT_KEY, 0.0)
    trace_slice_pct = perf_hotspots.get(PERF_SHA256_TRACE_SLICE_PCT_KEY, 0.0)
    if guest_machine_pct > 0.0 and trace_slice_pct > 0.0:
        if guest_machine_pct >= trace_slice_pct * 1.25:
            return "guest_machine"
        if trace_slice_pct >= guest_machine_pct * 1.25:
            return "trace_slice"
        return "guest_machine_and_trace_slice"
    if guest_machine_pct > 0.0:
        return "guest_machine"
    if trace_slice_pct > 0.0:
        return "trace_slice"
    return "sha256_digest_unresolved"


def descriptor_shape_hint(
    descriptor_rows: int,
    compact_rows: int,
    wide_rows: int,
    high32_rows_present: bool,
    high32_row_pct: float,
) -> str:
    if descriptor_rows <= 0:
        return "none"
    if wide_rows > 0:
        return "wide_descriptor_fallback_present"
    if compact_rows != descriptor_rows:
        return "descriptor_row_count_mismatch"
    if not high32_rows_present:
        return "compact_descriptor_no_high32_stats"
    if high32_row_pct == 0.0:
        return "high32_zero_compact_descriptor"
    if high32_row_pct < 5.0:
        return "high32_sparse_compact_descriptor"
    return "high32_dense_compact_descriptor"


def root_pipeline_policy_hint(
    input_bytes: int,
    root_count: int,
    groups: int,
    max_group_size: int,
) -> str:
    if root_count <= 1:
        return "none"
    if groups < root_count or max_group_size > 1:
        return "root_batches_already_grouped"
    if input_bytes >= ROOT_PIPELINE_INPUT_BYTE_LIMIT:
        return "large_input_root_pipeline_gated"
    return "enable_cross_segment_root_pipeline"


def opening_batching_hint(
    query_units: int,
    single_query_units: int,
    retained_leaf_openings: int,
    retained_leaf_rows: int,
    retained_leaf_all_single_row: int,
    retained_leaf_path_launches: int,
    retained_parent_checkpoint_openings: int,
    retained_parent_checkpoint_rows: int,
    retained_parent_checkpoint_all_single_row: int,
    retained_parent_checkpoint_prefix_launches: int,
    retained_parent_checkpoint_suffix_launches: int,
    direct_d2h_wait_ms: float,
) -> str:
    if direct_d2h_wait_ms < OPENING_BATCHING_D2H_WAIT_MS_THRESHOLD:
        return "none"
    if query_units and single_query_units != query_units:
        return "none"
    if (
        retained_leaf_openings > 1
        and retained_leaf_rows == retained_leaf_openings
        and retained_leaf_all_single_row > 0
        and single_query_units >= retained_leaf_openings
        and retained_leaf_path_launches > retained_leaf_openings
    ):
        return "cross_segment_retained_leaf_opening_candidate"
    retained_parent_checkpoint_path_launches = (
        retained_parent_checkpoint_prefix_launches
        + retained_parent_checkpoint_suffix_launches
    )
    if (
        retained_parent_checkpoint_openings > 1
        and retained_parent_checkpoint_rows == retained_parent_checkpoint_openings
        and retained_parent_checkpoint_all_single_row > 0
        and single_query_units >= retained_parent_checkpoint_openings
        and retained_parent_checkpoint_path_launches > retained_parent_checkpoint_openings
    ):
        return "cross_segment_retained_parent_checkpoint_opening_candidate"
    return "none"


def opening_source_shape_hint(
    query_units: int,
    single_query_units: int,
    max_queries_per_unit: int,
    root_count: int,
    retained_source_count: int,
    external_source_count: int,
    embedded_source_count: int,
    missing_source_count: int,
) -> str:
    if query_units <= 0:
        return "none"

    source_kinds = sum(
        1
        for count in (
            retained_source_count,
            external_source_count,
            embedded_source_count,
            missing_source_count,
        )
        if count > 0
    )
    if source_kinds > 1:
        source_shape = "mixed_sources"
    elif retained_source_count > 0:
        source_shape = "retained_source"
    elif external_source_count > 0:
        source_shape = "external_source"
    elif embedded_source_count > 0:
        source_shape = "embedded_source"
    elif missing_source_count > 0:
        source_shape = "missing_source"
    else:
        source_shape = "no_sources"

    if single_query_units == query_units and max_queries_per_unit <= 1:
        if query_units > 1 and root_count >= query_units:
            query_shape = "single_query_cross_root"
        else:
            query_shape = "single_query_units"
    elif max_queries_per_unit > 1:
        query_shape = "multi_query_units"
    else:
        query_shape = "mixed_query_units"

    return f"{query_shape}_with_{source_shape}"


def constant_material_overlap_hint(elapsed_ms: int, join_wait_ms: int) -> str:
    if elapsed_ms <= 0:
        return "none"
    if join_wait_ms <= 0:
        return "fully_overlapped"
    wait_ratio = join_wait_ms / elapsed_ms
    if wait_ratio < 0.25:
        return "mostly_overlapped"
    if wait_ratio >= 0.75:
        return "foreground_wait"
    return "partial_overlap"


def trace_report_buffer_shape_hint(
    reports: int,
    report_rows: int,
    buffer_capacity: int,
    buffer_excess_capacity: int,
) -> str:
    if reports <= 0 and report_rows <= 0 and buffer_capacity <= 0:
        return "none"
    if buffer_capacity <= 0:
        return "report_buffer_capacity_missing"
    excess_pct = buffer_excess_capacity * 100.0 / buffer_capacity
    if excess_pct <= 1.0:
        return "report_buffer_capacity_tight"
    if excess_pct <= 5.0:
        return "report_buffer_capacity_moderate"
    return "report_buffer_capacity_slack"


def trace_report_lifetime_hint(
    reports: int,
    buffer_excess_pct: float,
    pending_drop_pct: float,
) -> str:
    if reports <= 0:
        return "none"
    if buffer_excess_pct <= 1.0 and pending_drop_pct >= 5.0:
        return "tight_report_buffer_and_pending_drop"
    if pending_drop_pct >= 5.0:
        return "pending_segment_drop_pressure"
    if buffer_excess_pct <= 1.0:
        return "report_buffer_tight"
    return "none"


def trace_report_detail_sample_hint(reports: int, detail_samples: int) -> str:
    if reports <= 0:
        return "none"
    if detail_samples <= 0:
        return "detail_timing_disabled"
    if detail_samples >= reports:
        return "detail_timing_full"
    return "detail_timing_sampled"


def trace_shape_sample_hint(values: dict[str, int], rows: int) -> str:
    if rows <= 0:
        return "none"
    if any(values.get(key, 0) > 0 for key in TRACE_SHAPE_KEYS):
        return "shape_timing_enabled"
    return "shape_timing_disabled_or_zero"


DETAIL_SAMPLE_HOTSPOT_KEYS = [
    ("row_validation", TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY),
    ("lowering", TRACE_REPORT_LOWERING_SAMPLED_NS_KEY),
    ("visit", TRACE_REPORT_VISIT_SAMPLED_NS_KEY),
    ("descriptor", TRACE_DESCRIPTOR_SAMPLED_NS_KEY),
    ("source_values", TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY),
    ("source_a_value", TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY),
    ("source_b_value", TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY),
    ("instruction_result", TRACE_REPORT_INSTRUCTION_RESULT_SAMPLED_NS_KEY),
    ("next_pc", TRACE_REPORT_NEXT_PC_SAMPLED_NS_KEY),
    ("register_access", TRACE_REPORT_REGISTER_ACCESS_SAMPLED_NS_KEY),
    ("memory_access", TRACE_REPORT_MEMORY_ACCESS_SAMPLED_NS_KEY),
    ("store_apply", TRACE_REPORT_STORE_APPLY_SAMPLED_NS_KEY),
    ("precompile_memory", TRACE_REPORT_PRECOMPILE_MEMORY_SAMPLED_NS_KEY),
]

SOURCE_VALUE_DETAIL_HOTSPOT_KEYS = [
    ("source_a_value", TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY),
    ("source_b_value", TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY),
]

ROW_VALIDATION_PREFIX_HOTSPOT_KEYS = [
    ("memory_columns", TRACE_REPORT_MEMORY_COLUMNS_SAMPLED_NS_KEY),
]

ROW_VALIDATION_SUFFIX_HOTSPOT_KEYS = [
    ("instruction_result", TRACE_REPORT_INSTRUCTION_RESULT_SAMPLED_NS_KEY),
    ("next_pc", TRACE_REPORT_NEXT_PC_SAMPLED_NS_KEY),
    ("register_access", TRACE_REPORT_REGISTER_ACCESS_SAMPLED_NS_KEY),
    ("memory_access", TRACE_REPORT_MEMORY_ACCESS_SAMPLED_NS_KEY),
    ("store_apply", TRACE_REPORT_STORE_APPLY_SAMPLED_NS_KEY),
    ("precompile_memory", TRACE_REPORT_PRECOMPILE_MEMORY_SAMPLED_NS_KEY),
]


def row_validation_hotspot_keys(values: dict[str, int]) -> list[tuple[str, str]]:
    source_value_keys = (
        SOURCE_VALUE_DETAIL_HOTSPOT_KEYS
        if any(key in values for _, key in SOURCE_VALUE_DETAIL_HOTSPOT_KEYS)
        else [("source_values", TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY)]
    )
    return (
        ROW_VALIDATION_PREFIX_HOTSPOT_KEYS
        + source_value_keys
        + ROW_VALIDATION_SUFFIX_HOTSPOT_KEYS
    )


def trace_report_detail_hotspot(values: dict[str, int]) -> tuple[int, str, float]:
    samples = values.get(TRACE_REPORT_DETAIL_SAMPLES_KEY, 0)
    sampled_ns = values.get(TRACE_REPORT_SAMPLED_NS_KEY, 0)
    if samples <= 0 or sampled_ns <= 0:
        return (0, "none", 0.0)
    avg_ns = sampled_ns // samples
    hotspot_name = "none"
    hotspot_ns = 0
    for name, key in DETAIL_SAMPLE_HOTSPOT_KEYS:
        value = values.get(key, 0)
        if value > hotspot_ns:
            hotspot_name = name
            hotspot_ns = value
    hotspot_pct = hotspot_ns * 100.0 / sampled_ns if sampled_ns else 0.0
    return (avg_ns, hotspot_name, hotspot_pct)


def trace_report_row_validation_hotspot(values: dict[str, int]) -> tuple[str, float]:
    row_validation_ns = values.get(TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY, 0)
    if row_validation_ns <= 0:
        return ("none", 0.0)
    hotspot_name = "none"
    hotspot_ns = 0
    for name, key in row_validation_hotspot_keys(values):
        value = values.get(key, 0)
        if value > hotspot_ns:
            hotspot_name = name
            hotspot_ns = value
    hotspot_pct = hotspot_ns * 100.0 / row_validation_ns
    return (hotspot_name, hotspot_pct)


def trace_report_row_validation_coverage(values: dict[str, int]) -> tuple[float, float]:
    row_validation_ns = values.get(TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY, 0)
    if row_validation_ns <= 0:
        return (0.0, 0.0)
    child_ns = sum(values.get(key, 0) for _, key in row_validation_hotspot_keys(values))
    explained_ns = min(child_ns, row_validation_ns)
    residual_ns = max(row_validation_ns - child_ns, 0)
    return (
        explained_ns * 100.0 / row_validation_ns,
        residual_ns * 100.0 / row_validation_ns,
    )


def trace_report_source_values_lookup_coverage(
    values: dict[str, int],
) -> tuple[float, float]:
    source_values_ns = values.get(TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY, 0)
    if source_values_ns <= 0:
        return (0.0, 0.0)
    lookup_ns = values.get(TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY, 0) + values.get(
        TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY, 0
    )
    residual_ns = max(source_values_ns - lookup_ns, 0)
    return (
        lookup_ns * 100.0 / source_values_ns,
        residual_ns * 100.0 / source_values_ns,
    )


def trace_structure_hint(
    total_ms: int,
    runner_ms: int,
    lowerer_ms: int,
    stream_elapsed_ms: int,
    segment_receive_wait_ms: int,
    parallel_lower_workers: int,
    leaf_kernel_ms: int,
) -> str:
    trace_ms = max(runner_ms, lowerer_ms, stream_elapsed_ms)
    if total_ms <= 0 or trace_ms <= 0:
        return "none"
    receive_wait_ratio = (
        segment_receive_wait_ms / stream_elapsed_ms if stream_elapsed_ms else 0.0
    )
    leaf_ratio = leaf_kernel_ms / trace_ms if trace_ms else 0.0
    trace_total_ratio = trace_ms / total_ms if total_ms else 0.0
    if parallel_lower_workers > 0:
        if receive_wait_ratio >= 0.5:
            return "parallel_lower_waiting"
        return "parallel_lower_active"
    if trace_total_ratio >= 0.6 and receive_wait_ratio >= 0.5 and leaf_ratio <= 0.2:
        return "trace_stream_cpu_floor"
    if trace_total_ratio >= 0.6:
        return "cpu_trace_dominant"
    return "none"


def summarize_profile_values(
    label: str,
    values: dict[str, int],
    perf_hotspots: dict[str, float] | None = None,
) -> str:
    missing = [
        key
        for key in [ROOT_COUNT_KEY, ROOT_GROUPS_KEY, ROOT_MAX_GROUP_KEY]
        if key not in values
    ]
    if missing:
        raise SystemExit(f"{label}: missing timing fields: {', '.join(missing)}")

    input_bytes = values.get(INPUT_BYTES_KEY, 0)
    total_ms = values.get(TOTAL_MS_KEY, 0)
    constant_material_elapsed_ms = values.get(
        CONSTANT_MATERIAL_VALIDATION_ELAPSED_MS_KEY, 0
    )
    constant_material_join_wait_ms = values.get(
        CONSTANT_MATERIAL_VALIDATION_JOIN_WAIT_MS_KEY, 0
    )
    constant_material_hint = constant_material_overlap_hint(
        constant_material_elapsed_ms,
        constant_material_join_wait_ms,
    )
    runner_ms = values.get(RUNNER_MS_KEY, 0)
    lowerer_ms = values.get(LOWERER_MS_KEY, 0)
    stream_elapsed_ms = values.get(STREAM_ELAPSED_MS_KEY, 0)
    stream_worker_ms = values.get(STREAM_WORKER_MS_KEY, 0)
    segment_commit_ms = values.get(SEGMENT_COMMIT_MS_KEY, 0)
    segment_commit_initial_workers = values.get(SEGMENT_COMMIT_INITIAL_WORKERS_KEY, 0)
    segment_commit_effective_workers = values.get(SEGMENT_COMMIT_EFFECTIVE_WORKERS_KEY, 0)
    segment_commit_oom_retries = values.get(SEGMENT_COMMIT_OOM_RETRIES_KEY, 0)
    stream_commit_residual_ms = (
        stream_elapsed_ms - stream_worker_ms - segment_commit_ms
    )
    segment_receive_wait_ms = values.get(SEGMENT_RECEIVE_WAIT_MS_KEY, 0)
    pending_receive_wait_ms = values.get(PENDING_RECEIVE_WAIT_MS_KEY, 0)
    pending_send_wait_ms = values.get(PENDING_SEND_WAIT_MS_KEY, 0)
    parallel_lower_workers = values.get(PARALLEL_LOWER_WORKERS_KEY, 0)
    parallel_lower_dispatched = values.get(PARALLEL_LOWER_DISPATCHED_KEY, 0)
    parallel_lower_received = values.get(PARALLEL_LOWER_RECEIVED_KEY, 0)
    parallel_lower_emitted = values.get(PARALLEL_LOWER_EMITTED_KEY, 0)
    parallel_lower_max_reorder = values.get(PARALLEL_LOWER_MAX_REORDER_KEY, 0)
    trace_reports = values.get(TRACE_REPORTS_KEY, 0)
    trace_report_rows = values.get(TRACE_REPORT_ROWS_KEY, 0)
    single_row_reports = values.get(TRACE_SINGLE_ROW_REPORTS_KEY, 0)
    multi_row_reports = values.get(TRACE_MULTI_ROW_REPORTS_KEY, 0)
    pending_dma_reports = values.get(TRACE_PENDING_DMA_REPORTS_KEY, 0)
    amo_reports = values.get(TRACE_AMO_REPORTS_KEY, 0)
    store_conditional_reports = values.get(
        TRACE_STORE_CONDITIONAL_REPORTS_KEY, 0
    )
    external_op_rows = values.get(TRACE_EXTERNAL_OP_ROWS_KEY, 0)
    copy_rows = values.get(TRACE_COPY_ROWS_KEY, 0)
    flag_rows = values.get(TRACE_FLAG_ROWS_KEY, 0)
    precompile_rows = values.get(TRACE_PRECOMPILE_ROWS_KEY, 0)
    indirect_memory_rows = values.get(TRACE_INDIRECT_MEMORY_ROWS_KEY, 0)
    register_source_reads = values.get(TRACE_REGISTER_SOURCE_READS_KEY, 0)
    memory_source_reads = values.get(TRACE_MEMORY_SOURCE_READS_KEY, 0)
    register_store_rows = values.get(TRACE_REGISTER_STORE_ROWS_KEY, 0)
    memory_store_rows = values.get(TRACE_MEMORY_STORE_ROWS_KEY, 0)
    no_store_rows = values.get(TRACE_NO_STORE_ROWS_KEY, 0)
    indirect_memory_row_pct = (
        indirect_memory_rows * 100.0 / trace_report_rows
        if trace_report_rows
        else 0.0
    )
    memory_source_read_pct = (
        memory_source_reads * 100.0 / trace_report_rows
        if trace_report_rows
        else 0.0
    )
    memory_store_row_pct = (
        memory_store_rows * 100.0 / trace_report_rows
        if trace_report_rows
        else 0.0
    )
    no_store_row_pct = (
        no_store_rows * 100.0 / trace_report_rows
        if trace_report_rows
        else 0.0
    )
    trace_shape_hint = trace_shape_sample_hint(values, trace_report_rows)
    trace_report_detail_samples = values.get(TRACE_REPORT_DETAIL_SAMPLES_KEY, 0)
    trace_report_detail_sample_pct = (
        trace_report_detail_samples * 100.0 / trace_reports if trace_reports else 0.0
    )
    trace_report_detail_sample_ppm = (
        trace_report_detail_samples * 1_000_000.0 / trace_reports
        if trace_reports
        else 0.0
    )
    trace_report_detail_hint = trace_report_detail_sample_hint(
        trace_reports,
        trace_report_detail_samples,
    )
    (
        trace_report_detail_avg_ns,
        trace_report_detail_hotspot_name,
        trace_report_detail_hotspot_pct,
    ) = trace_report_detail_hotspot(values)
    (
        trace_report_row_validation_hotspot_name,
        trace_report_row_validation_hotspot_pct,
    ) = trace_report_row_validation_hotspot(values)
    (
        trace_report_row_validation_explained_pct,
        trace_report_row_validation_residual_pct,
    ) = trace_report_row_validation_coverage(values)
    (
        trace_report_source_values_lookup_pct,
        trace_report_source_values_residual_pct,
    ) = trace_report_source_values_lookup_coverage(values)
    trace_report_detail_visit_pct = (
        values.get(TRACE_REPORT_VISIT_SAMPLED_NS_KEY, 0) * 100.0
        / values.get(TRACE_REPORT_SAMPLED_NS_KEY, 0)
        if values.get(TRACE_REPORT_SAMPLED_NS_KEY, 0)
        else 0.0
    )
    trace_report_visit_sampled_ns = values.get(TRACE_REPORT_VISIT_SAMPLED_NS_KEY, 0)
    trace_descriptor_sampled_ns = values.get(TRACE_DESCRIPTOR_SAMPLED_NS_KEY, 0)
    trace_report_visit_descriptor_pct = (
        trace_descriptor_sampled_ns * 100.0 / trace_report_visit_sampled_ns
        if trace_report_visit_sampled_ns
        else 0.0
    )
    trace_report_visit_residual_pct = (
        max(trace_report_visit_sampled_ns - trace_descriptor_sampled_ns, 0)
        * 100.0
        / trace_report_visit_sampled_ns
        if trace_report_visit_sampled_ns
        else 0.0
    )
    trace_rows_per_report = (
        trace_report_rows / trace_reports if trace_reports else 0.0
    )
    trace_report_buffer_capacity = values.get(TRACE_REPORT_BUFFER_CAPACITY_KEY, 0)
    trace_report_buffer_max_capacity = values.get(
        TRACE_REPORT_BUFFER_MAX_CAPACITY_KEY, 0
    )
    trace_report_buffer_excess_capacity = values.get(
        TRACE_REPORT_BUFFER_EXCESS_CAPACITY_KEY, 0
    )
    trace_report_buffer_excess_pct = (
        trace_report_buffer_excess_capacity * 100.0 / trace_report_buffer_capacity
        if trace_report_buffer_capacity
        else 0.0
    )
    trace_report_buffer_hint = trace_report_buffer_shape_hint(
        trace_reports,
        trace_report_rows,
        trace_report_buffer_capacity,
        trace_report_buffer_excess_capacity,
    )
    descriptor_rows = values.get(DESCRIPTOR_ROWS_KEY, 0)
    descriptor_compact_rows = values.get(DESCRIPTOR_COMPACT_ROWS_KEY, 0)
    descriptor_wide_rows = values.get(DESCRIPTOR_WIDE_ROWS_KEY, 0)
    descriptor_upload_bytes = values.get(DESCRIPTOR_UPLOAD_BYTES_KEY, 0)
    descriptor_upload_rows = values.get(DESCRIPTOR_UPLOAD_ROWS_KEY, 0)
    descriptor_bytes_per_row = (
        descriptor_upload_bytes / descriptor_upload_rows
        if descriptor_upload_rows
        else 0.0
    )
    descriptor_high32_values = values.get(DESCRIPTOR_HIGH32_VALUES_KEY, 0)
    descriptor_high32_rows = values.get(DESCRIPTOR_HIGH32_ROWS_KEY, 0)
    descriptor_high32_stats_enabled = values.get(
        DESCRIPTOR_HIGH32_STATS_ENABLED_KEY,
        1 if descriptor_high32_values > 0 or descriptor_high32_rows > 0 else 0,
    )
    descriptor_high32_rows_present = (
        descriptor_high32_stats_enabled > 0 and DESCRIPTOR_HIGH32_ROWS_KEY in values
    )
    descriptor_high32_row_pct = (
        descriptor_high32_rows * 100.0 / descriptor_rows if descriptor_rows else 0.0
    )
    descriptor_hint = descriptor_shape_hint(
        descriptor_rows,
        descriptor_compact_rows,
        descriptor_wide_rows,
        descriptor_high32_rows_present,
        descriptor_high32_row_pct,
    )
    seed_direct_lift_attempts = values.get(SEED_DIRECT_LIFT_ATTEMPTS_KEY, 0)
    seed_direct_lift_successes = values.get(SEED_DIRECT_LIFT_SUCCESSES_KEY, 0)
    seed_full_advances = values.get(SEED_FULL_ADVANCES_KEY, 0)
    finish_opening_ms = values.get(FINISH_OPENING_MS_KEY, 0)
    opening_queries = values.get(OPENING_QUERY_COUNT_KEY, 0)
    opening_query_units = values.get(OPENING_QUERY_UNITS_KEY, 0)
    opening_single_query_units = values.get(OPENING_SINGLE_QUERY_UNITS_KEY, 0)
    opening_max_queries_per_unit = values.get(OPENING_MAX_QUERIES_PER_UNIT_KEY, 0)
    opening_stage_count = values.get(OPENING_STAGE_COUNT_KEY, 0)
    opening_retained_source_count = values.get(OPENING_RETAINED_SOURCE_COUNT_KEY, 0)
    opening_external_source_count = values.get(OPENING_EXTERNAL_SOURCE_COUNT_KEY, 0)
    opening_embedded_source_count = values.get(OPENING_EMBEDDED_SOURCE_COUNT_KEY, 0)
    opening_missing_source_count = values.get(OPENING_MISSING_SOURCE_COUNT_KEY, 0)
    opening_row_value_device_rows = values.get(OPENING_ROW_VALUE_DEVICE_ROWS_KEY, 0)
    opening_row_value_source_rows = values.get(OPENING_ROW_VALUE_SOURCE_ROWS_KEY, 0)
    retained_leaf_openings = values.get(OPENING_RETAINED_LEAF_COUNT_KEY, 0)
    retained_leaf_rows = values.get(OPENING_RETAINED_LEAF_ROWS_KEY, 0)
    retained_leaf_all_single_row_value = values.get(
        OPENING_RETAINED_LEAF_ALL_SINGLE_ROW_KEY, 0
    )
    retained_leaf_all_single_row = (
        "yes" if retained_leaf_all_single_row_value > 0 else "no"
    )
    retained_leaf_path_launches = values.get(
        OPENING_RETAINED_LEAF_PATH_LAUNCHES_KEY, 0
    )
    retained_parent_checkpoint_openings = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_COUNT_KEY, 0
    )
    retained_parent_checkpoint_rows = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_ROWS_KEY, 0
    )
    retained_parent_checkpoint_all_single_row_value = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_ALL_SINGLE_ROW_KEY, 0
    )
    retained_parent_checkpoint_all_single_row = (
        "yes" if retained_parent_checkpoint_all_single_row_value > 0 else "no"
    )
    retained_parent_checkpoint_prefix_rows = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_ROWS_KEY, 0
    )
    retained_parent_checkpoint_prefix_bytes = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_BYTES_KEY, 0
    )
    retained_parent_checkpoint_prefix_launches = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_LAUNCHES_KEY, 0
    )
    retained_parent_checkpoint_suffix_rows = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_ROWS_KEY, 0
    )
    retained_parent_checkpoint_suffix_bytes = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_BYTES_KEY, 0
    )
    retained_parent_checkpoint_suffix_launches = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_LAUNCHES_KEY, 0
    )
    opening_path_parent_hash_launches_per_stage = values.get(
        OPENING_PATH_PARENT_HASH_LAUNCHES_PER_STAGE_KEY, 0
    )
    opening_row_value_device_download_batches = values.get(
        OPENING_ROW_VALUE_DEVICE_DOWNLOAD_BATCHES_KEY, 0
    )
    root_count = values[ROOT_COUNT_KEY]
    groups = values[ROOT_GROUPS_KEY]
    max_group_size = values[ROOT_MAX_GROUP_KEY]
    roots_per_group = root_count / groups if groups else 0.0
    needs_cross_segment_root_pipeline = (
        "yes" if root_count > 1 and groups >= root_count and max_group_size <= 1 else "no"
    )
    policy_hint = root_pipeline_policy_hint(
        input_bytes, root_count, groups, max_group_size
    )
    leaf_kernel_ms = values.get(LEAF_KERNEL_MS_KEY, 0)
    leaf_coset_calls = values.get(LEAF_COSET_CALLS_KEY, 0)
    leaf_coset_columns = values.get(LEAF_COSET_COLUMNS_KEY, 0)
    leaf_ntt_launches = values.get(LEAF_NTT_LAUNCHES_KEY, 0)
    leaf_ntt_stage_launches = values.get(LEAF_NTT_STAGE_LAUNCHES_KEY, 0)
    leaf_ntt_block_twiddle_launches = values.get(LEAF_NTT_BLOCK_TWIDDLE_LAUNCHES_KEY, 0)
    ntt_launches_per_call = leaf_ntt_launches / leaf_coset_calls if leaf_coset_calls else 0.0
    direct_d2h_wait_ms = values.get(DIRECT_D2H_WAIT_NS_KEY, 0) / 1_000_000.0
    direct_d2h_hot_bytes = values.get(DIRECT_D2H_HOT_BYTES_KEY, 0)
    direct_d2h_hot_count = values.get(DIRECT_D2H_HOT_COUNT_KEY, 0)
    direct_d2h_hot_wait_ms = values.get(DIRECT_D2H_HOT_WAIT_NS_KEY, 0) / 1_000_000.0
    opening_source_hint = opening_source_shape_hint(
        opening_query_units,
        opening_single_query_units,
        opening_max_queries_per_unit,
        root_count,
        opening_retained_source_count,
        opening_external_source_count,
        opening_embedded_source_count,
        opening_missing_source_count,
    )
    opening_hint = opening_batching_hint(
        opening_query_units,
        opening_single_query_units,
        retained_leaf_openings,
        retained_leaf_rows,
        retained_leaf_all_single_row_value,
        retained_leaf_path_launches,
        retained_parent_checkpoint_openings,
        retained_parent_checkpoint_rows,
        retained_parent_checkpoint_all_single_row_value,
        retained_parent_checkpoint_prefix_launches,
        retained_parent_checkpoint_suffix_launches,
        direct_d2h_wait_ms,
    )
    leaf_launch_pressure = "yes" if leaf_ntt_launches >= 10_000 else "no"
    trace_to_leaf_ratio = (
        max(runner_ms, lowerer_ms) / leaf_kernel_ms if leaf_kernel_ms else 0.0
    )
    bottleneck = primary_bottleneck(
        total_ms,
        runner_ms,
        lowerer_ms,
        stream_elapsed_ms,
        stream_worker_ms,
        segment_commit_ms,
        segment_receive_wait_ms,
        finish_opening_ms,
        leaf_kernel_ms,
        direct_d2h_wait_ms,
    )
    trace_hint = trace_structure_hint(
        total_ms,
        runner_ms,
        lowerer_ms,
        stream_elapsed_ms,
        segment_receive_wait_ms,
        parallel_lower_workers,
        leaf_kernel_ms,
    )
    proof_12s_gap_ms = max(total_ms - PROOF_TARGET_MS, 0) if total_ms > 0 else 0
    proof_12s_hint = proof_target_gap_hint(
        total_ms,
        runner_ms,
        lowerer_ms,
        stream_elapsed_ms,
        segment_commit_ms,
        finish_opening_ms,
        leaf_kernel_ms,
        direct_d2h_wait_ms,
    )
    if perf_hotspots is None:
        perf_hotspots = parse_perf_self_hotspots("")
    pending_drop_pct = perf_hotspots.get(
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY, 0.0
    )
    trace_lifetime_hint = trace_report_lifetime_hint(
        trace_reports,
        trace_report_buffer_excess_pct,
        pending_drop_pct,
    )
    lowered_report_row_pct = perf_hotspots.get(
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY, 0.0
    )
    memmove_pct = perf_hotspots.get(PERF_MEMMOVE_SELF_PCT_KEY, 0.0)
    memmove_guest_machine_pct = perf_hotspots.get(
        PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY, 0.0
    )
    memmove_trace_slice_pct = perf_hotspots.get(
        PERF_MEMMOVE_TRACE_SLICE_PCT_KEY, 0.0
    )
    memmove_hint = memmove_source_hint(perf_hotspots)
    sha256_pct = perf_hotspots.get(PERF_SHA256_SELF_PCT_KEY, 0.0)
    sha256_hint = sha256_source_hint(perf_hotspots)
    cpu_hint = cpu_trace_hotspot_hint(perf_hotspots)
    return (
        f"{label},{input_bytes},{total_ms},"
        f"{constant_material_elapsed_ms},{constant_material_join_wait_ms},"
        f"{constant_material_hint},{runner_ms},{lowerer_ms},"
        f"{stream_elapsed_ms},{stream_worker_ms},{segment_commit_ms},"
        f"{segment_commit_initial_workers},{segment_commit_effective_workers},"
        f"{segment_commit_oom_retries},"
        f"{stream_commit_residual_ms},{segment_receive_wait_ms},"
        f"{pending_receive_wait_ms},{pending_send_wait_ms},"
        f"{parallel_lower_workers},{parallel_lower_dispatched},"
        f"{parallel_lower_received},{parallel_lower_emitted},"
        f"{parallel_lower_max_reorder},{trace_reports},"
        f"{trace_report_rows},{trace_rows_per_report:.3f},"
        f"{trace_report_buffer_capacity},{trace_report_buffer_max_capacity},"
        f"{trace_report_buffer_excess_capacity},"
        f"{trace_report_buffer_excess_pct:.3f},{trace_report_buffer_hint},"
        f"{trace_lifetime_hint},"
        f"{descriptor_rows},"
        f"{descriptor_compact_rows},{descriptor_wide_rows},"
        f"{descriptor_upload_bytes},{descriptor_bytes_per_row:.3f},"
        f"{descriptor_high32_values},{descriptor_high32_rows},"
        f"{descriptor_high32_row_pct:.3f},{descriptor_hint},"
        f"{seed_direct_lift_attempts},"
        f"{seed_direct_lift_successes},{seed_full_advances},"
        f"{finish_opening_ms},{opening_query_units},{opening_single_query_units},"
        f"{opening_queries},{opening_max_queries_per_unit},{opening_stage_count},"
        f"{opening_source_hint},{opening_row_value_device_rows},"
        f"{opening_row_value_source_rows},"
        f"{retained_leaf_openings},{retained_leaf_rows},"
        f"{retained_leaf_all_single_row},{retained_leaf_path_launches},"
        f"{retained_parent_checkpoint_openings},{retained_parent_checkpoint_rows},"
        f"{retained_parent_checkpoint_all_single_row},"
        f"{retained_parent_checkpoint_prefix_rows},"
        f"{retained_parent_checkpoint_prefix_bytes},"
        f"{retained_parent_checkpoint_prefix_launches},"
        f"{retained_parent_checkpoint_suffix_rows},"
        f"{retained_parent_checkpoint_suffix_bytes},"
        f"{retained_parent_checkpoint_suffix_launches},"
        f"{opening_path_parent_hash_launches_per_stage},"
        f"{opening_row_value_device_download_batches},"
        f"{opening_hint},"
        f"{root_count},{groups},{max_group_size},"
        f"{roots_per_group:.3f},{needs_cross_segment_root_pipeline},{policy_hint},"
        f"{leaf_kernel_ms},{leaf_coset_calls},{leaf_coset_columns},{leaf_ntt_launches},"
        f"{leaf_ntt_stage_launches},{leaf_ntt_block_twiddle_launches},"
        f"{ntt_launches_per_call:.3f},{direct_d2h_wait_ms:.3f},{leaf_launch_pressure},"
        f"{trace_to_leaf_ratio:.3f},{bottleneck},{trace_hint},"
        f"{proof_12s_gap_ms},{proof_12s_hint},"
        f"{lowered_report_row_pct:.3f},{memmove_pct:.3f},{memmove_guest_machine_pct:.3f},"
        f"{memmove_trace_slice_pct:.3f},{memmove_hint},"
        f"{pending_drop_pct:.3f},{sha256_pct:.3f},{sha256_hint},{cpu_hint},"
        f"{single_row_reports},{multi_row_reports},{pending_dma_reports},"
        f"{amo_reports},{store_conditional_reports},{external_op_rows},"
        f"{copy_rows},{flag_rows},{precompile_rows},"
        f"{indirect_memory_rows},{indirect_memory_row_pct:.3f},"
        f"{register_source_reads},{memory_source_reads},"
        f"{memory_source_read_pct:.3f},{register_store_rows},"
        f"{memory_store_rows},{memory_store_row_pct:.3f},"
        f"{no_store_rows},{no_store_row_pct:.3f},{trace_shape_hint},"
        f"{trace_report_detail_samples},{trace_report_detail_sample_pct:.3f},"
        f"{trace_report_detail_sample_ppm:.3f},{trace_report_detail_hint},"
        f"{trace_report_detail_avg_ns},"
        f"{trace_report_detail_hotspot_name},{trace_report_detail_hotspot_pct:.3f},"
        f"{trace_report_row_validation_hotspot_name},"
        f"{trace_report_row_validation_hotspot_pct:.3f},"
        f"{trace_report_row_validation_explained_pct:.3f},"
        f"{trace_report_row_validation_residual_pct:.3f},"
        f"{trace_report_source_values_lookup_pct:.3f},"
        f"{trace_report_source_values_residual_pct:.3f},"
        f"{trace_report_detail_visit_pct:.3f},"
        f"{trace_report_visit_descriptor_pct:.3f},"
        f"{trace_report_visit_residual_pct:.3f},"
        f"{direct_d2h_hot_bytes},{direct_d2h_hot_count},"
        f"{direct_d2h_hot_wait_ms:.3f}"
    )


def summarize_profile(label: str, text: str) -> str:
    return summarize_profile_values(
        label, parse_timing_log(text), parse_perf_self_hotspots(text)
    )


def median_int(values: list[int]) -> float:
    ordered = sorted(values)
    midpoint = len(ordered) // 2
    if len(ordered) % 2 == 1:
        return float(ordered[midpoint])
    return (ordered[midpoint - 1] + ordered[midpoint]) / 2.0


def summarize_total_samples(parsed_inputs: list[tuple[str, dict[str, int]]]) -> str:
    total_count = len(parsed_inputs)
    totals = [
        values[TOTAL_MS_KEY]
        for _, values in parsed_inputs
        if values.get(TOTAL_MS_KEY, 0) > 0
    ]
    valid_total_count = len(totals)
    if not totals:
        return f"aggregate,{total_count},0,0,0.000,0.000,0,0.000,no,no"

    total_min_ms = min(totals)
    total_mean_ms = sum(totals) / valid_total_count
    total_median_ms = median_int(totals)
    total_max_ms = max(totals)
    sample_spread_pct = (
        (total_max_ms - total_min_ms) * 100.0 / total_median_ms
        if total_median_ms
        else 0.0
    )
    close_samples = (
        "yes"
        if valid_total_count >= 3 and sample_spread_pct <= CLOSE_SAMPLE_SPREAD_PCT
        else "no"
    )
    max_outlier = (
        "yes"
        if valid_total_count >= 3 and total_max_ms > total_median_ms * OUTLIER_RATIO_THRESHOLD
        else "no"
    )
    return (
        f"aggregate,{total_count},{valid_total_count},{total_min_ms},"
        f"{total_mean_ms:.3f},{total_median_ms:.3f},{total_max_ms},"
        f"{sample_spread_pct:.3f},{close_samples},{max_outlier}"
    )


def print_summary(inputs: list[tuple[str, str]]) -> None:
    parsed_inputs = [
        (label, parse_timing_log(text), parse_perf_self_hotspots(text))
        for label, text in inputs
    ]
    print(HEADER)
    for label, values, perf_hotspots in parsed_inputs:
        print(summarize_profile_values(label, values, perf_hotspots))
    if len(parsed_inputs) > 1:
        print(AGGREGATE_HEADER)
        print(
            summarize_total_samples(
                [(label, values) for label, values, _ in parsed_inputs]
            )
        )


def self_test() -> None:
    print_summary(
        [
            (
                "single-root-groups",
                "\n".join(
                    [
                        f"{TOTAL_MS_KEY}=9050",
                        f"{RUNNER_MS_KEY}=7800",
                        f"{LOWERER_MS_KEY}=7812",
                        f"{STREAM_ELAPSED_MS_KEY}=9912",
                        f"{STREAM_WORKER_MS_KEY}=7812",
                        f"{SEGMENT_COMMIT_MS_KEY}=2100",
                        f"{SEGMENT_COMMIT_INITIAL_WORKERS_KEY}=2",
                        f"{SEGMENT_COMMIT_EFFECTIVE_WORKERS_KEY}=2",
                        f"{SEGMENT_COMMIT_OOM_RETRIES_KEY}=0",
                        f"{SEGMENT_RECEIVE_WAIT_MS_KEY}=6000",
                        f"{PENDING_RECEIVE_WAIT_MS_KEY}=1200",
                        f"{PENDING_SEND_WAIT_MS_KEY}=345",
                        f"{PARALLEL_LOWER_WORKERS_KEY}=2",
                        f"{PARALLEL_LOWER_DISPATCHED_KEY}=23",
                        f"{PARALLEL_LOWER_RECEIVED_KEY}=23",
                        f"{PARALLEL_LOWER_EMITTED_KEY}=23",
                        f"{PARALLEL_LOWER_MAX_REORDER_KEY}=1",
                        f"{TRACE_REPORTS_KEY}=93843537",
                        f"{TRACE_REPORT_ROWS_KEY}=93917088",
                        f"{TRACE_REPORT_BUFFER_CAPACITY_KEY}=94371840",
                        f"{TRACE_REPORT_BUFFER_MAX_CAPACITY_KEY}=4194304",
                        f"{TRACE_REPORT_BUFFER_EXCESS_CAPACITY_KEY}=528303",
                        f"{DESCRIPTOR_ROWS_KEY}=1000",
                        f"{DESCRIPTOR_COMPACT_ROWS_KEY}=1000",
                        f"{DESCRIPTOR_WIDE_ROWS_KEY}=0",
                        f"{DESCRIPTOR_UPLOAD_BYTES_KEY}=88000",
                        f"{DESCRIPTOR_UPLOAD_ROWS_KEY}=1000",
                        f"{DESCRIPTOR_HIGH32_VALUES_KEY}=6",
                        f"{DESCRIPTOR_HIGH32_ROWS_KEY}=4",
                        f"{SEED_DIRECT_LIFT_ATTEMPTS_KEY}=22",
                        f"{SEED_DIRECT_LIFT_SUCCESSES_KEY}=22",
                        f"{SEED_FULL_ADVANCES_KEY}=1",
                        f"{FINISH_OPENING_MS_KEY}=476",
                        f"{OPENING_QUERY_UNITS_KEY}=23",
                        f"{OPENING_SINGLE_QUERY_UNITS_KEY}=23",
                        f"{OPENING_RETAINED_LEAF_COUNT_KEY}=23",
                        f"{OPENING_RETAINED_LEAF_ROWS_KEY}=23",
                        f"{OPENING_RETAINED_LEAF_ALL_SINGLE_ROW_KEY}=1",
                        f"{OPENING_RETAINED_LEAF_PATH_LAUNCHES_KEY}=276",
                        f"{INPUT_BYTES_KEY}=2758032",
                        f"{ROOT_COUNT_KEY}=23",
                        f"{ROOT_GROUPS_KEY}=23",
                        f"{ROOT_MAX_GROUP_KEY}=1",
                        f"{LEAF_KERNEL_MS_KEY}=858",
                        f"{LEAF_COSET_CALLS_KEY}=23",
                        f"{LEAF_COSET_COLUMNS_KEY}=874",
                        f"{LEAF_NTT_LAUNCHES_KEY}=41078",
                        f"{LEAF_NTT_STAGE_LAUNCHES_KEY}=15732",
                        f"{LEAF_NTT_BLOCK_TWIDDLE_LAUNCHES_KEY}=23598",
                        f"{DIRECT_D2H_WAIT_NS_KEY}=192973857",
                        "    23.99%    23.17%  [.] sha2::sha256::x86::digest_blocks",
                        "            |--3.36%--0xf2b2442ea4d72b97",
                        "    26.35%  [.] lzvm_prover::guest_pc_trace_backend::apply_main_lowered_report_row",
                        "    20.94%  [.] __memmove_avx512_unaligned_erms",
                        "            |--10.61%--lzvm_prover::guest_machine::advance_guest_machine_prepared_inner",
                        "             --8.67%--lzvm_prover::guest_pc_trace_backend::run_guest_pc_trace_segment_slice",
                        "     7.41%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
                    ]
                ),
            ),
            (
                "batched-roots",
                "\n".join(
                    [
                        f"{TOTAL_MS_KEY}=9050",
                        f"{INPUT_BYTES_KEY}=2758032",
                        f"{ROOT_COUNT_KEY}=23",
                        f"{ROOT_GROUPS_KEY}=1",
                        f"{ROOT_MAX_GROUP_KEY}=23",
                    ]
                ),
            ),
            (
                "slow-sample",
                "\n".join(
                    [
                        f"{TOTAL_MS_KEY}=18100",
                        f"{INPUT_BYTES_KEY}=12447640",
                        f"{ROOT_COUNT_KEY}=120",
                        f"{ROOT_GROUPS_KEY}=120",
                        f"{ROOT_MAX_GROUP_KEY}=1",
                    ]
                ),
            ),
        ]
    )


def sibling_perf_report_paths(input_path: Path) -> list[Path]:
    candidates = [
        input_path.with_suffix(".perf.report"),
        input_path.with_name(f"{input_path.name}.perf.report"),
    ]
    paths = []
    seen = set()
    for candidate in candidates:
        if candidate == input_path or candidate in seen:
            continue
        seen.add(candidate)
        paths.append(candidate)
    return paths


def read_input(path: str | None) -> tuple[str, str]:
    if path is None or path == "-":
        return ("stdin", sys.stdin.read())
    input_path = Path(path)
    text = input_path.read_text(encoding="utf-8")
    perf_reports = [
        perf_report.read_text(encoding="utf-8")
        for perf_report in sibling_perf_report_paths(input_path)
        if perf_report.is_file()
    ]
    if perf_reports:
        text = "\n".join([text, *perf_reports])
    return (str(input_path), text)


def main() -> None:
    parser = argparse.ArgumentParser(description="Summarize prove timing root materialization shape.")
    parser.add_argument("logs", nargs="*", help="prove --timings log paths, or '-' for stdin")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return
    if not args.logs:
        raise SystemExit("at least one log path is required unless --self-test is used")
    print_summary([read_input(path) for path in args.logs])


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
import argparse
import csv
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
TRACE_LOWER_MS_KEY = "timing_guest_trace_lower_ms"
TRACE_REPORT_MS_KEY = "timing_guest_trace_report_ms"
STREAM_ELAPSED_MS_KEY = "timing_guest_trace_stream_elapsed_ms"
STREAM_WORKER_MS_KEY = "timing_guest_trace_stream_ms"
SEGMENT_COMMIT_MS_KEY = "timing_guest_segment_commit_ms"
SEGMENT_COMMIT_ATTEMPT_MS_KEY = "timing_guest_segment_commit_attempt_ms"
SEGMENT_COMMIT_OOM_RETRY_MS_KEY = "timing_guest_segment_commit_oom_retry_ms"
SEGMENT_COMMIT_INITIAL_WORKERS_KEY = "timing_guest_segment_commit_initial_workers"
SEGMENT_COMMIT_EFFECTIVE_WORKERS_KEY = "timing_guest_segment_commit_effective_workers"
SEGMENT_COMMIT_WORKER_SUBMITS_KEY = "timing_guest_segment_commit_worker_submits"
SEGMENT_COMMIT_WORKER_JOINS_KEY = "timing_guest_segment_commit_worker_joins"
SEGMENT_COMMIT_WORKER_BACKPRESSURE_JOINS_KEY = (
    "timing_guest_segment_commit_worker_backpressure_joins"
)
SEGMENT_COMMIT_WORKER_BACKPRESSURE_JOIN_MS_KEY = (
    "timing_guest_segment_commit_worker_backpressure_join_ms"
)
SEGMENT_COMMIT_WORKER_FINISH_JOINS_KEY = (
    "timing_guest_segment_commit_worker_finish_joins"
)
SEGMENT_COMMIT_WORKER_FINISH_JOIN_MS_KEY = (
    "timing_guest_segment_commit_worker_finish_join_ms"
)
SEGMENT_COMMIT_WORKER_MAX_IN_FLIGHT_KEY = (
    "timing_guest_segment_commit_worker_max_in_flight"
)
SEGMENT_COMMIT_OOM_RETRIES_KEY = "timing_guest_segment_commit_oom_retries"
SEGMENT_COMMIT_CUDA_MEMORY_TOTAL_BYTES_KEY = (
    "timing_guest_segment_commit_cuda_memory_total_bytes"
)
SEGMENT_COMMIT_CUDA_MEMORY_INITIAL_FREE_BYTES_KEY = (
    "timing_guest_segment_commit_cuda_memory_initial_free_bytes"
)
SEGMENT_COMMIT_CUDA_MEMORY_EFFECTIVE_FREE_BYTES_KEY = (
    "timing_guest_segment_commit_cuda_memory_effective_free_bytes"
)
SEGMENT_COMMIT_CUDA_MEMORY_MIN_FREE_BYTES_KEY = (
    "timing_guest_segment_commit_cuda_memory_min_free_bytes"
)
SEGMENT_COMMIT_CUDA_ALLOCATOR_INITIAL_CACHED_BYTES_KEY = (
    "timing_guest_segment_commit_cuda_allocator_initial_cached_bytes"
)
SEGMENT_COMMIT_CUDA_ALLOCATOR_EFFECTIVE_CACHED_BYTES_KEY = (
    "timing_guest_segment_commit_cuda_allocator_effective_cached_bytes"
)
SEGMENT_RECEIVE_WAIT_MS_KEY = "timing_guest_trace_segment_receive_wait_ms"
PENDING_RECEIVE_WAIT_MS_KEY = "timing_guest_trace_pending_receive_wait_ms"
PENDING_SEND_WAIT_MS_KEY = "timing_guest_trace_pending_send_wait_ms"
PARALLEL_LOWER_WORKERS_KEY = "timing_guest_trace_parallel_lower_workers"
PARALLEL_LOWER_DISPATCHED_KEY = "timing_guest_trace_parallel_lower_dispatched"
PARALLEL_LOWER_RECEIVED_KEY = "timing_guest_trace_parallel_lower_received"
PARALLEL_LOWER_EMITTED_KEY = "timing_guest_trace_parallel_lower_emitted"
PARALLEL_LOWER_MAX_REORDER_KEY = "timing_guest_trace_parallel_lower_max_reorder"
PARALLEL_LOWER_SNAPSHOT_REPLAY_KEY = (
    "timing_guest_trace_parallel_lower_snapshot_replay_count"
)
PARALLEL_LOWER_REPORT_ELIDED_KEY = (
    "timing_guest_trace_parallel_lower_report_elided_count"
)
PARALLEL_LOWER_DISPATCH_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_dispatch_wait_ms"
)
PARALLEL_LOWER_RESULT_RECEIVE_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_result_receive_wait_ms"
)
PARALLEL_LOWER_DISPATCH_BLOCKED_KEY = (
    "timing_guest_trace_parallel_lower_dispatch_blocked_count"
)
SEGMENT_REPLAY_COUNT_KEY = "timing_guest_trace_segment_replay_count"
TRACE_REPORTS_KEY = "timing_guest_trace_reports"
TRACE_REPORT_ROWS_KEY = "timing_guest_trace_report_rows"
TRACE_REPORT_CHUNK_SENT_KEY = "timing_guest_trace_report_chunk_sent"
TRACE_REPORT_CHUNK_RECEIVED_KEY = "timing_guest_trace_report_chunk_received"
TRACE_REPORT_CHUNK_REPORTS_KEY = "timing_guest_trace_report_chunk_reports"
TRACE_REPORT_CHUNK_ROWS_KEY = "timing_guest_trace_report_chunk_rows"
TRACE_REPORT_CHUNK_MAX_QUEUED_KEY = "timing_guest_trace_report_chunk_max_queued"
TRACE_REPORT_VALIDATION_MS_KEY = "timing_guest_trace_report_validation_ms"
TRACE_REPORT_LOWERING_MS_KEY = "timing_guest_trace_report_lowering_ms"
TRACE_REPORT_ROW_VALIDATION_MS_KEY = "timing_guest_trace_report_row_validation_ms"
TRACE_REPORT_MEMORY_COLUMNS_MS_KEY = "timing_guest_trace_report_memory_columns_ms"
TRACE_REPORT_SOURCE_VALUES_MS_KEY = "timing_guest_trace_report_source_values_ms"
TRACE_REPORT_SOURCE_A_VALUE_MS_KEY = "timing_guest_trace_report_source_a_value_ms"
TRACE_REPORT_SOURCE_B_VALUE_MS_KEY = "timing_guest_trace_report_source_b_value_ms"
TRACE_REPORT_SOURCE_IMMEDIATE_READ_MS_KEY = (
    "timing_guest_trace_report_source_immediate_read_ms"
)
TRACE_REPORT_SOURCE_REGISTER_READ_MS_KEY = (
    "timing_guest_trace_report_source_register_read_ms"
)
TRACE_REPORT_SOURCE_MEMORY_READ_MS_KEY = (
    "timing_guest_trace_report_source_memory_read_ms"
)
TRACE_REPORT_SOURCE_INDIRECT_READ_MS_KEY = (
    "timing_guest_trace_report_source_indirect_read_ms"
)
TRACE_REPORT_SOURCE_LAST_C_READ_MS_KEY = (
    "timing_guest_trace_report_source_last_c_read_ms"
)
TRACE_REPORT_PRECOMPILE_MEMORY_MS_KEY = (
    "timing_guest_trace_report_precompile_memory_ms"
)
TRACE_REPORT_INSTRUCTION_RESULT_MS_KEY = (
    "timing_guest_trace_report_instruction_result_ms"
)
TRACE_REPORT_NEXT_PC_MS_KEY = "timing_guest_trace_report_next_pc_ms"
TRACE_REPORT_REGISTER_ACCESS_MS_KEY = "timing_guest_trace_report_register_access_ms"
TRACE_REPORT_MEMORY_ACCESS_MS_KEY = "timing_guest_trace_report_memory_access_ms"
TRACE_REPORT_STORE_APPLY_MS_KEY = "timing_guest_trace_report_store_apply_ms"
TRACE_REPORT_VISIT_MS_KEY = "timing_guest_trace_report_visit_ms"
TRACE_REPORT_EMIT_MS_KEY = "timing_guest_trace_emit_ms"
TRACE_DESCRIPTOR_MS_KEY = "timing_guest_trace_descriptor_ms"
TRACE_SINGLE_ROW_REPORTS_KEY = "timing_guest_trace_single_row_reports"
TRACE_MULTI_ROW_REPORTS_KEY = "timing_guest_trace_multi_row_reports"
TRACE_PENDING_DMA_REPORTS_KEY = "timing_guest_trace_pending_dma_reports"
TRACE_AMO_REPORTS_KEY = "timing_guest_trace_amo_reports"
TRACE_STORE_CONDITIONAL_REPORTS_KEY = "timing_guest_trace_store_conditional_reports"
TRACE_EXTERNAL_OP_ROWS_KEY = "timing_guest_trace_external_op_rows"
TRACE_COPY_ROWS_KEY = "timing_guest_trace_copy_rows"
TRACE_COPY_MEMORY_SOURCE_ROWS_KEY = "timing_guest_trace_copy_memory_source_rows"
TRACE_COPY_INDIRECT_MEMORY_ROWS_KEY = "timing_guest_trace_copy_indirect_memory_rows"
TRACE_COPY_REGISTER_STORE_ROWS_KEY = "timing_guest_trace_copy_register_store_rows"
TRACE_COPY_MEMORY_STORE_ROWS_KEY = "timing_guest_trace_copy_memory_store_rows"
TRACE_COPY_NO_STORE_ROWS_KEY = "timing_guest_trace_copy_no_store_rows"
TRACE_COPY_NO_MEMORY_ROWS_KEY = "timing_guest_trace_copy_no_memory_rows"
TRACE_EXTERNAL_OP_RUNS_KEY = "timing_guest_trace_external_op_runs"
TRACE_EXTERNAL_OP_MAX_RUN_KEY = "timing_guest_trace_external_op_max_run"
TRACE_COPY_RUNS_KEY = "timing_guest_trace_copy_runs"
TRACE_COPY_MAX_RUN_KEY = "timing_guest_trace_copy_max_run"
TRACE_EXTERNAL_OP_ROW_LOWER_MS_KEY = "timing_guest_trace_external_op_row_lower_ms"
TRACE_COPY_ROW_LOWER_MS_KEY = "timing_guest_trace_copy_row_lower_ms"
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
    TRACE_COPY_MEMORY_SOURCE_ROWS_KEY,
    TRACE_COPY_INDIRECT_MEMORY_ROWS_KEY,
    TRACE_COPY_REGISTER_STORE_ROWS_KEY,
    TRACE_COPY_MEMORY_STORE_ROWS_KEY,
    TRACE_COPY_NO_STORE_ROWS_KEY,
    TRACE_COPY_NO_MEMORY_ROWS_KEY,
    TRACE_EXTERNAL_OP_RUNS_KEY,
    TRACE_EXTERNAL_OP_MAX_RUN_KEY,
    TRACE_COPY_RUNS_KEY,
    TRACE_COPY_MAX_RUN_KEY,
    TRACE_EXTERNAL_OP_ROW_LOWER_MS_KEY,
    TRACE_COPY_ROW_LOWER_MS_KEY,
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
TRACE_REPORT_SOURCE_IMMEDIATE_READS_KEY = (
    "timing_guest_trace_report_source_immediate_reads"
)
TRACE_REPORT_SOURCE_REGISTER_READS_KEY = (
    "timing_guest_trace_report_source_register_reads"
)
TRACE_REPORT_SOURCE_MEMORY_READS_KEY = "timing_guest_trace_report_source_memory_reads"
TRACE_REPORT_SOURCE_INDIRECT_READS_KEY = (
    "timing_guest_trace_report_source_indirect_reads"
)
TRACE_REPORT_SOURCE_LAST_C_READS_KEY = "timing_guest_trace_report_source_last_c_reads"
TRACE_REPORT_SOURCE_IMMEDIATE_READ_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_immediate_read_sampled_ns"
)
TRACE_REPORT_SOURCE_REGISTER_READ_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_register_read_sampled_ns"
)
TRACE_REPORT_SOURCE_MEMORY_READ_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_memory_read_sampled_ns"
)
TRACE_REPORT_SOURCE_INDIRECT_READ_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_indirect_read_sampled_ns"
)
TRACE_REPORT_SOURCE_LAST_C_READ_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_last_c_read_sampled_ns"
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
TRACE_RUNNER_REPORT_BUFFER_CAPACITY_KEY = (
    "timing_guest_trace_runner_report_buffer_capacity"
)
TRACE_RUNNER_REPORT_BUFFER_MAX_CAPACITY_KEY = (
    "timing_guest_trace_runner_report_buffer_max_capacity"
)
TRACE_RUNNER_REPORT_BUFFER_EXCESS_CAPACITY_KEY = (
    "timing_guest_trace_runner_report_buffer_excess_capacity"
)
TRACE_REPORT_RECORD_SIZE_BYTES_KEY = "timing_guest_trace_report_record_size_bytes"
TRACE_REPORT_INSTRUCTION_SIZE_BYTES_KEY = (
    "timing_guest_trace_report_instruction_size_bytes"
)
TRACE_REPORT_REGISTER_WRITE_LIST_SIZE_BYTES_KEY = (
    "timing_guest_trace_report_register_write_list_size_bytes"
)
TRACE_REPORT_MEMORY_ACCESS_LIST_SIZE_BYTES_KEY = (
    "timing_guest_trace_report_memory_access_list_size_bytes"
)
TRACE_REPORT_PRECOMPILE_ACCESS_LIST_SIZE_BYTES_KEY = (
    "timing_guest_trace_report_precompile_access_list_size_bytes"
)
TRACE_REPORT_STORAGE_BYTES_KEY = "timing_guest_trace_report_storage_bytes"
TRACE_REPORT_BUFFER_CAPACITY_BYTES_KEY = (
    "timing_guest_trace_report_buffer_capacity_bytes"
)
TRACE_REPORT_BUFFER_EXCESS_BYTES_KEY = "timing_guest_trace_report_buffer_excess_bytes"
TRACE_RUNNER_REPORT_BUFFER_CAPACITY_BYTES_KEY = (
    "timing_guest_trace_runner_report_buffer_capacity_bytes"
)
TRACE_RUNNER_REPORT_BUFFER_EXCESS_BYTES_KEY = (
    "timing_guest_trace_runner_report_buffer_excess_bytes"
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
DESCRIPTOR_HIGH32_A_VALUES_KEY = "timing_guest_trace_descriptor_high32_a_values"
DESCRIPTOR_HIGH32_B_VALUES_KEY = "timing_guest_trace_descriptor_high32_b_values"
DESCRIPTOR_HIGH32_C_VALUES_KEY = "timing_guest_trace_descriptor_high32_c_values"
DESCRIPTOR_HIGH32_A_PAYLOAD_VALUES_KEY = (
    "timing_guest_trace_descriptor_high32_a_payload_values"
)
DESCRIPTOR_HIGH32_B_PAYLOAD_VALUES_KEY = (
    "timing_guest_trace_descriptor_high32_b_payload_values"
)
DESCRIPTOR_HIGH32_STORE_PAYLOAD_VALUES_KEY = (
    "timing_guest_trace_descriptor_high32_store_payload_values"
)
DESCRIPTOR_HIGH32_STORE_PREV_VALUE_VALUES_KEY = (
    "timing_guest_trace_descriptor_high32_store_prev_value_values"
)
DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS = (
    "timing_guest_trace_descriptor_high32_rows_with_0_fields",
    "timing_guest_trace_descriptor_high32_rows_with_1_fields",
    "timing_guest_trace_descriptor_high32_rows_with_2_fields",
    "timing_guest_trace_descriptor_high32_rows_with_3_fields",
    "timing_guest_trace_descriptor_high32_rows_with_4_fields",
    "timing_guest_trace_descriptor_high32_rows_with_5_fields",
    "timing_guest_trace_descriptor_high32_rows_with_6_fields",
    "timing_guest_trace_descriptor_high32_rows_with_7_fields",
)
SPARSE_HIGH32_DESCRIPTOR_BASE_WORDS_PER_ROW = 9
WORD_BYTES = 8
SEED_DIRECT_LIFT_ATTEMPTS_KEY = "timing_guest_trace_seed_direct_lift_attempts"
SEED_DIRECT_LIFT_SUCCESSES_KEY = "timing_guest_trace_seed_direct_lift_successes"
SEED_DIRECT_LIFT_EMPTY_SEGMENTS_KEY = (
    "timing_guest_trace_seed_direct_lift_empty_segments"
)
SEED_DIRECT_LIFT_PENDING_DMA_SINGLE_REPORTS_KEY = (
    "timing_guest_trace_seed_direct_lift_pending_dma_single_reports"
)
SEED_DIRECT_LIFT_AMO_BOUNDARIES_KEY = (
    "timing_guest_trace_seed_direct_lift_amo_boundaries"
)
SEED_DIRECT_LIFT_STORE_CONDITIONAL_BOUNDARIES_KEY = (
    "timing_guest_trace_seed_direct_lift_store_conditional_boundaries"
)
SEED_DIRECT_LIFT_DMA_PREPARE_MISSING_LOOKAHEADS_KEY = (
    "timing_guest_trace_seed_direct_lift_dma_prepare_missing_lookaheads"
)
SEED_DIRECT_LIFT_BOUNDARY_C_UNAVAILABLE_KEY = (
    "timing_guest_trace_seed_direct_lift_boundary_c_unavailable"
)
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
OPENING_ROW_VALUE_SOURCE_EXTEND_MS_KEY = (
    "timing_finish_witness_opening_row_value_source_extend_ms"
)
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
OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_MS_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms"
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
OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_MS_KEY = (
    "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms"
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
OPENING_ROW_VALUE_DEVICE_SINGLE_DOWNLOADS_KEY = (
    "timing_finish_witness_opening_row_values_device_single_downloads"
)
OPENING_STAGE_ROW_VALUE_DEVICE_SINGLE_DOWNLOAD_RE = re.compile(
    r"^timing_finish_witness_stage_(\d+)_opening_row_values_device_single_downloads$"
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
CUDA_HOST_REGISTER_WAIT_NS_KEY = "timing_cuda_allocator_host_register_wait_ns"
CUDA_COPY_H2D_BYTES_KEY = "timing_cuda_allocator_copy_h2d_bytes"
CUDA_COPY_H2D_WAIT_NS_KEY = "timing_cuda_allocator_copy_h2d_wait_ns"
CUDA_COPY_H2D_HOT_BYTES_KEY = "timing_cuda_allocator_copy_h2d_hot_bytes"
CUDA_COPY_H2D_HOT_COUNT_KEY = "timing_cuda_allocator_copy_h2d_hot_count"
CUDA_COPY_H2D_HOT_WAIT_NS_KEY = "timing_cuda_allocator_copy_h2d_hot_wait_ns"
CUDA_COPY_D2H_BYTES_KEY = "timing_cuda_allocator_copy_d2h_bytes"
CUDA_COPY_D2H_WAIT_NS_KEY = "timing_cuda_allocator_copy_d2h_wait_ns"
CUDA_COPY_D2H_HOT_BYTES_KEY = "timing_cuda_allocator_copy_d2h_hot_bytes"
CUDA_COPY_D2H_HOT_COUNT_KEY = "timing_cuda_allocator_copy_d2h_hot_count"
CUDA_COPY_D2H_HOT_WAIT_NS_KEY = "timing_cuda_allocator_copy_d2h_hot_wait_ns"
SOURCE_RETENTION_ATTEMPTS_KEY = "timing_guest_stage_source_retention_attempts"
SOURCE_RETENTION_RETAINED_KEY = "timing_guest_stage_source_retention_retained"
SOURCE_RETENTION_REJECTED_KEY = "timing_guest_stage_source_retention_rejected"
SOURCE_RETENTION_RETAINED_BYTES_KEY = "timing_guest_stage_source_retention_retained_bytes"
SOURCE_RETENTION_REJECTED_BYTES_KEY = "timing_guest_stage_source_retention_rejected_bytes"
SOURCE_RETENTION_MAX_RETAINED_BYTES_KEY = (
    "timing_guest_stage_source_retention_max_retained_bytes"
)
SOURCE_RETENTION_MAX_REJECTED_BYTES_KEY = (
    "timing_guest_stage_source_retention_max_rejected_bytes"
)
SOURCE_RETENTION_LIMIT_BYTES_KEY = "timing_guest_stage_source_retention_limit_bytes"
DESCRIPTOR_RETENTION_ATTEMPTS_KEY = "timing_guest_descriptor_buffer_retention_attempts"
DESCRIPTOR_RETENTION_RETAINED_KEY = "timing_guest_descriptor_buffer_retention_retained"
DESCRIPTOR_RETENTION_REJECTED_KEY = "timing_guest_descriptor_buffer_retention_rejected"
DESCRIPTOR_RETENTION_RETAINED_BYTES_KEY = (
    "timing_guest_descriptor_buffer_retention_retained_bytes"
)
DESCRIPTOR_RETENTION_REJECTED_BYTES_KEY = (
    "timing_guest_descriptor_buffer_retention_rejected_bytes"
)
DESCRIPTOR_RETENTION_LIMIT_BYTES_KEY = (
    "timing_guest_descriptor_buffer_retention_limit_bytes"
)
NSYS_COPY_TRACE_DESCRIPTOR_RESIDENCY_PIPELINE_KEY = (
    "nsys_copy_trace_descriptor_residency_pipeline"
)
NSYS_COPY_GPU_RESIDENCY_HINT_KEY = "nsys_copy_gpu_residency_hint"
NSYS_COPY_H2D_BULK_APP_FRAME_HINT_KEY = "nsys_copy_h2d_bulk_app_frame_hint"
NSYS_COPY_SMALL_D2H_BATCHING_HINT_KEY = "nsys_copy_small_d2h_batching_hint"
NSYS_COPY_CUDA_API_BACKTRACE_HINT_KEY = "nsys_copy_cuda_api_backtrace_hint"
NSYS_KERNEL_GRAPH_FUSION_PRIORITY_HINT_KEY = (
    "nsys_kernel_graph_fusion_priority_hint"
)
NSYS_KERNEL_NEXT_ACTION_HINT_KEY = "nsys_kernel_next_action_hint"
NSYS_KERNEL_GRAPH_FUSION_UPPER_BOUND_MS_KEY = (
    "nsys_kernel_graph_fusion_upper_bound_ms"
)
NSYS_KERNEL_TOP_STREAM_IDLE_MS_KEY = "nsys_kernel_top_stream_idle_ms"
NSYS_KERNEL_SEPARATION_HINT_KEY = "nsys_kernel_separation_hint"
NSYS_KERNEL_TOP_STREAM_IDLE_GAP_PREVIOUS_KEY = (
    "nsys_kernel_top_stream_idle_gap_previous_kernel"
)
NSYS_KERNEL_TOP_STREAM_IDLE_GAP_NEXT_KEY = "nsys_kernel_top_stream_idle_gap_next_kernel"
NSYS_KERNEL_TOP_STREAM_IDLE_GAP_CALLS_KEY = "nsys_kernel_top_stream_idle_gap_calls"
NSYS_KERNEL_TOP_STREAM_IDLE_GAP_MS_KEY = "nsys_kernel_top_stream_idle_gap_ms"
NCU_METRIC_COLLECTION_HINT_KEY = "ncu_metric_collection_hint"
NCU_TOP_KERNEL_KEY = "ncu_top_kernel"
NCU_TOP_KERNEL_DURATION_MS_KEY = "ncu_top_kernel_duration_ms"
NCU_TOP_KERNEL_SM_THROUGHPUT_PCT_KEY = "ncu_top_kernel_sm_throughput_pct"
NCU_TOP_KERNEL_DRAM_THROUGHPUT_PCT_KEY = "ncu_top_kernel_dram_throughput_pct"
NCU_TOP_KERNEL_REGISTERS_PER_THREAD_KEY = "ncu_top_kernel_registers_per_thread"
NCU_TOP_KERNEL_LIMITING_FACTORS_KEY = "ncu_top_kernel_limiting_factors"
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
PERF_PREPARE_INSTRUCTION_SELF_PCT_KEY = "perf_prepare_instruction_self_pct"
PERF_TRACE_SEGMENT_BUILD_SELF_PCT_KEY = "perf_trace_segment_build_self_pct"
PERF_APPEND_DESCRIPTOR_SELF_PCT_KEY = "perf_append_descriptor_self_pct"
PERF_SOURCE_VALUE_SELF_PCT_KEY = "perf_source_value_self_pct"
PERF_ADVANCE_GUEST_MACHINE_SELF_PCT_KEY = "perf_advance_guest_machine_self_pct"
PERF_GUEST_MEMORY_WRITE_SELF_PCT_KEY = "perf_guest_memory_write_self_pct"
PERF_BIGUINT_MODPOW_SELF_PCT_KEY = "perf_biguint_modpow_self_pct"
PERF_GUEST_MEMORY_READ_SELF_PCT_KEY = "perf_guest_memory_read_self_pct"
PERF_DECODE_INSTRUCTION_SELF_PCT_KEY = "perf_decode_instruction_self_pct"
PERF_EFFECT_RECORD_MEMORY_WRITE_SELF_PCT_KEY = (
    "perf_effect_record_memory_write_self_pct"
)
PERF_EFFECT_RECORD_MEMORY_READ_SELF_PCT_KEY = "perf_effect_record_memory_read_self_pct"
CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY = (
    "cpu_trace_memcpy_report_storage_hint_pct"
)
CPU_TRACE_MEMCPY_REPORT_STORAGE_TOTAL_PCT_KEY = (
    "cpu_trace_memcpy_report_storage_total_pct"
)
CPU_TRACE_REPORT_STORAGE_STRUCTURAL_TOTAL_PCT_THRESHOLD = 5.0
ROOT_PIPELINE_INPUT_BYTE_LIMIT = 8 * 1024 * 1024
OPENING_BATCHING_D2H_WAIT_MS_THRESHOLD = 100.0
SINGLE_QUERY_ROW_VALUE_BOUNDARY_HINT = (
    "single_query_unit_boundary_blocks_row_value_batch"
)
EXTERNAL_SOURCE_ROW_VALUE_BOUNDARY_HINT = (
    "external_source_unit_boundary_blocks_row_value_batch"
)
RETAINED_PARENT_CHECKPOINT_PATH_SECONDARY_MS_THRESHOLD = 500
CUDA_TRANSFER_BULK_H2D_BYTES_THRESHOLD = 8 * 1024 * 1024 * 1024
CUDA_TRANSFER_WAIT_MS_THRESHOLD = 500.0
CUDA_TRANSFER_HOT_COPY_COUNT_THRESHOLD = 8
DIRECT_D2H_HOT_WAIT_PCT_THRESHOLD = 50.0
SEGMENT_COMMIT_MEMORY_PRESSURE_PCT_THRESHOLD = 8.0
SEGMENT_COMMIT_MEMORY_THIN_MARGIN_PCT_THRESHOLD = 15.0
SOURCE_ROW_VALUE_SECONDARY_PCT_THRESHOLD = 5.0
PROOF_TARGET_MS = 12_000
PERF_SELF_PERCENT_RE = re.compile(r"^\s*(\d+(?:\.\d+)?)%\s+(.*)$")
PERF_SECOND_SELF_PERCENT_RE = re.compile(r"^\s*(\d+(?:\.\d+)?)%\s+(.*)$")
PERF_CALLCHAIN_PERCENT_RE = re.compile(r"(\d+(?:\.\d+)?)%--(.*)$")
NSYS_CPU_HOTSPOT_BLOCKS = {
    "top_cpu_self_samples",
    "application_cpu_hotspots",
}
NSYS_CPU_MEMCPY_ACTION_HINT_BLOCK = "cpu_trace_memcpy_action_hints"

HEADER = (
    "profile,input_bytes,total_ms,constant_material_validation_elapsed_ms,"
    "constant_material_validation_join_wait_ms,constant_material_validation_overlap_hint,"
    "runner_ms,lowerer_ms,trace_lower_ms,trace_report_ms,trace_non_report_ms,"
    "trace_runner_lowerer_overlap_ms,trace_lowerer_non_lower_ms,"
    "stream_elapsed_ms,stream_worker_ms,"
    "segment_commit_ms,segment_commit_initial_workers,"
    "segment_commit_effective_workers,segment_commit_worker_submits,"
    "segment_commit_worker_joins,"
    "segment_commit_worker_backpressure_joins,"
    "segment_commit_worker_backpressure_join_ms,"
    "segment_commit_worker_finish_joins,segment_commit_worker_finish_join_ms,"
    "segment_commit_worker_max_in_flight,"
    "segment_commit_worker_pressure_hint,"
    "segment_commit_oom_retries,"
    "segment_commit_attempt_ms,segment_commit_oom_retry_ms,"
    "stream_commit_residual_ms,segment_receive_wait_ms,"
    "pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,"
    "parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,"
    "parallel_lower_max_reorder,parallel_lower_snapshot_replay_count,"
    "parallel_lower_report_elided_count,parallel_lower_dispatch_wait_ms,"
    "parallel_lower_result_receive_wait_ms,"
    "parallel_lower_dispatch_blocked_count,segment_replay_count,trace_reports,trace_report_rows,"
    "trace_rows_per_report,trace_report_record_size_bytes,"
    "trace_report_instruction_size_bytes,"
    "trace_report_register_write_list_size_bytes,"
    "trace_report_memory_access_list_size_bytes,"
    "trace_report_precompile_access_list_size_bytes,"
    "trace_report_instruction_storage_gib,"
    "trace_report_register_write_list_storage_gib,"
    "trace_report_memory_access_list_storage_gib,"
    "trace_report_precompile_access_list_storage_gib,"
    "trace_report_storage_bytes,trace_report_storage_gib,"
    "trace_report_buffer_capacity,trace_report_buffer_max_capacity,"
    "trace_report_buffer_excess_capacity,trace_report_buffer_capacity_bytes,"
    "trace_report_buffer_capacity_gib,trace_report_buffer_excess_bytes,"
    "trace_report_buffer_excess_pct,trace_report_buffer_shape_hint,"
    "trace_runner_report_buffer_capacity,trace_runner_report_buffer_max_capacity,"
    "trace_runner_report_buffer_excess_capacity,"
    "trace_runner_report_buffer_capacity_bytes,"
    "trace_runner_report_buffer_capacity_gib,"
    "trace_runner_report_buffer_excess_bytes,"
    "trace_runner_report_buffer_excess_pct,trace_runner_report_buffer_shape_hint,"
    "trace_report_lifetime_hint,trace_report_chunk_sent,"
    "trace_report_chunk_received,trace_report_chunk_reports,"
    "trace_report_chunk_rows,trace_report_chunk_max_queued,"
    "descriptor_rows,descriptor_compact_rows,"
    "descriptor_wide_rows,descriptor_upload_bytes,descriptor_bytes_per_row,"
    "descriptor_high32_nonzero_values,descriptor_high32_nonzero_rows,"
    "descriptor_high32_row_pct,descriptor_high32_a_values,"
    "descriptor_high32_b_values,descriptor_high32_c_values,"
    "descriptor_high32_a_payload_values,descriptor_high32_b_payload_values,"
    "descriptor_high32_store_payload_values,"
    "descriptor_high32_store_prev_value_values,"
    "descriptor_high32_rows_with_0_fields,descriptor_high32_rows_with_1_fields,"
    "descriptor_high32_rows_with_2_fields,descriptor_high32_rows_with_3_fields,"
    "descriptor_high32_rows_with_4_fields,descriptor_high32_rows_with_5_fields,"
    "descriptor_high32_rows_with_6_fields,descriptor_high32_rows_with_7_fields,"
    "descriptor_sparse_high32_estimated_upload_bytes,"
    "descriptor_sparse_high32_estimated_upload_savings_pct,"
    "descriptor_sparse_high32_high_words,descriptor_sparse_high32_shape_hint,"
    "descriptor_shape_hint,seed_direct_lift_attempts,"
    "seed_direct_lift_successes,seed_direct_lift_success_pct,"
    "seed_direct_lift_dominant_miss_reason,seed_direct_lift_action_hint,"
    "seed_direct_lift_empty_segments,"
    "seed_direct_lift_pending_dma_single_reports,seed_direct_lift_amo_boundaries,"
    "seed_direct_lift_store_conditional_boundaries,"
    "seed_direct_lift_dma_prepare_missing_lookaheads,"
    "seed_direct_lift_boundary_c_unavailable,seed_full_advances,"
    "finish_opening_ms,opening_query_units,opening_single_query_units,"
    "opening_queries,opening_max_queries_per_unit,opening_stage_count,"
    "opening_source_shape_hint,"
    "source_retention_attempts,source_retention_retained,"
    "source_retention_rejected,source_retention_retained_bytes,"
    "source_retention_rejected_bytes,source_retention_max_retained_bytes,"
    "source_retention_max_rejected_bytes,source_retention_limit_bytes,"
    "source_retention_rejected_total_exceeds_device_memory,"
    "source_retention_max_rejected_exceeds_device_memory,"
    "opening_source_rebuild_hint,opening_row_value_device_rows,"
    "opening_row_value_source_rows,opening_row_value_source_extend_ms,"
    "opening_row_value_source_extend_pct,opening_source_row_value_action_hint,"
    "retained_leaf_openings,retained_leaf_rows,retained_leaf_all_single_row,"
    "retained_leaf_path_launches,retained_parent_checkpoint_openings,"
    "retained_parent_checkpoint_rows,retained_parent_checkpoint_all_single_row,"
    "retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,"
    "retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_prefix_ms,"
    "retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,"
    "retained_parent_checkpoint_suffix_launches,retained_parent_checkpoint_suffix_ms,"
    "retained_parent_checkpoint_path_launches,retained_parent_checkpoint_path_ms,"
    "retained_parent_checkpoint_cross_stage_gather_estimated_launches,"
    "retained_parent_checkpoint_cross_stage_gather_launch_savings,"
    "retained_parent_checkpoint_batching_hint,"
    "opening_path_parent_hash_launches_per_stage,"
    "opening_row_value_device_download_batches,"
    "opening_row_value_device_single_downloads,"
    "opening_row_value_device_single_stage_count,"
    "opening_row_value_device_single_max_stage,"
    "opening_row_value_device_cross_unit_batch_savings,"
    "opening_batching_hint,opening_external_source_boundary_hint,"
    "opening_retained_parent_checkpoint_action_hint,"
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
    "cpu_trace_report_storage_action_hint,"
    "cpu_trace_memcpy_report_storage_hint_pct,"
    "cpu_trace_memcpy_report_storage_total_pct,"
    "perf_append_descriptor_self_pct,perf_source_value_self_pct,"
    "cpu_trace_lowerer_action_hint,"
    "perf_prepare_instruction_self_pct,perf_trace_segment_build_self_pct,"
    "perf_advance_guest_machine_self_pct,perf_guest_memory_write_self_pct,"
    "perf_biguint_modpow_self_pct,perf_guest_memory_read_self_pct,"
    "perf_decode_instruction_self_pct,perf_effect_record_memory_write_self_pct,"
    "perf_effect_record_memory_read_self_pct,"
    "cpu_runner_hotspot_hint,"
    "single_row_reports,multi_row_reports,pending_dma_reports,amo_reports,"
    "store_conditional_reports,external_op_rows,copy_rows,flag_rows,"
    "precompile_rows,indirect_memory_rows,indirect_memory_row_pct,"
    "register_source_reads,memory_source_reads,memory_source_read_pct,"
    "register_store_rows,memory_store_rows,memory_store_row_pct,"
    "no_store_rows,no_store_row_pct,trace_shape_sample_hint,"
    "copy_memory_source_rows,copy_memory_source_row_pct,"
    "copy_indirect_memory_rows,copy_indirect_memory_row_pct,"
    "copy_register_store_rows,copy_memory_store_rows,"
    "copy_no_store_rows,copy_no_memory_rows,copy_no_memory_row_pct,"
    "trace_copy_shape_hint,"
    "trace_report_validation_ms,trace_report_emit_ms,trace_descriptor_ms,"
    "trace_report_lowering_ms,trace_report_row_validation_ms,"
    "trace_report_memory_columns_ms,trace_report_source_values_ms,"
    "trace_report_source_a_value_ms,trace_report_source_b_value_ms,"
    "trace_report_precompile_memory_ms,trace_report_instruction_result_ms,"
    "trace_report_next_pc_ms,trace_report_register_access_ms,"
    "trace_report_memory_access_ms,trace_report_store_apply_ms,"
    "trace_report_visit_ms,trace_report_exact_hotspot,"
    "trace_report_exact_hotspot_pct,trace_report_exact_action_hint,"
    "trace_report_detail_samples,trace_report_detail_sample_pct,"
    "trace_report_detail_sample_ppm,trace_report_detail_sample_hint,"
    "trace_report_detail_avg_ns,"
    "trace_report_detail_lowerer_share_ms,trace_report_row_validation_lowerer_share_ms,"
    "trace_report_memory_columns_lowerer_share_ms,"
    "trace_report_source_values_lowerer_share_ms,"
    "trace_report_source_lookup_lowerer_share_ms,"
    "trace_report_source_values_residual_lowerer_share_ms,"
    "trace_report_precompile_memory_lowerer_share_ms,"
    "trace_report_instruction_result_lowerer_share_ms,"
    "trace_report_next_pc_lowerer_share_ms,"
    "trace_report_register_access_lowerer_share_ms,"
    "trace_report_memory_access_lowerer_share_ms,"
    "trace_report_store_apply_lowerer_share_ms,"
    "trace_report_row_validation_residual_lowerer_share_ms,"
    "trace_report_visit_lowerer_share_ms,trace_report_descriptor_lowerer_share_ms,"
    "trace_report_detail_hotspot,trace_report_detail_hotspot_pct,"
    "trace_report_detail_action_hint,"
    "trace_report_row_validation_hotspot,trace_report_row_validation_hotspot_pct,"
    "trace_report_row_validation_explained_pct,trace_report_row_validation_residual_pct,"
    "trace_report_source_values_lookup_pct,trace_report_source_values_residual_pct,"
    "source_immediate_reads,source_immediate_read_pct,"
    "source_register_reads,source_register_read_pct,"
    "source_memory_reads,source_memory_read_pct,"
    "source_indirect_reads,source_indirect_read_pct,"
    "source_last_c_reads,source_last_c_read_pct,"
    "trace_report_source_kind_hotspot,trace_report_source_kind_hotspot_pct,"
    "trace_report_source_kind_coverage_pct,trace_report_source_kind_residual_pct,"
    "trace_report_detail_visit_pct,trace_report_visit_descriptor_pct,"
    "trace_report_visit_residual_pct,"
    "direct_d2h_hot_bytes,direct_d2h_hot_count,direct_d2h_hot_wait_ms,"
    "direct_d2h_hot_wait_pct,direct_d2h_action_hint,"
    "cuda_allocator_d2h_bytes,cuda_allocator_d2h_wait_ms,"
    "cuda_allocator_d2h_hot_bytes,cuda_allocator_d2h_hot_count,"
    "cuda_allocator_d2h_hot_wait_ms,cuda_allocator_d2h_hot_wait_pct,"
    "cuda_allocator_d2h_action_hint,"
    "cuda_host_register_wait_ms,cuda_h2d_bytes,cuda_transfer_action_hint,"
    "data_residency_action_hint,"
    "copy_summary_gpu_residency_hint,copy_summary_h2d_bulk_app_frame_hint,"
    "copy_summary_small_d2h_batching_hint,"
    "copy_summary_cuda_api_backtrace_hint,"
    "kernel_graph_fusion_priority_hint,kernel_next_action_hint,"
    "kernel_graph_fusion_upper_bound_ms,"
    "kernel_top_stream_idle_ms,kernel_separation_hint,"
    "kernel_top_stream_idle_gap_previous_kernel,"
    "kernel_top_stream_idle_gap_next_kernel,"
    "kernel_top_stream_idle_gap_calls,kernel_top_stream_idle_gap_ms,"
    "kernel_stream_idle_boundary_hint,"
    "ncu_metric_collection_hint,ncu_top_kernel,ncu_top_kernel_duration_ms,"
    "ncu_top_kernel_sm_throughput_pct,ncu_top_kernel_dram_throughput_pct,"
    "ncu_top_kernel_registers_per_thread,ncu_top_kernel_limiting_factors,"
    "segment_commit_cuda_memory_total_bytes,"
    "segment_commit_cuda_memory_initial_free_bytes,"
    "segment_commit_cuda_memory_effective_free_bytes,"
    "segment_commit_cuda_memory_min_free_bytes,"
    "segment_commit_cuda_allocator_initial_cached_bytes,"
    "segment_commit_cuda_allocator_effective_cached_bytes,"
    "segment_commit_cuda_memory_min_free_pct,"
    "segment_commit_memory_pressure_hint,"
    "descriptor_retention_attempts,descriptor_retention_retained,"
    "descriptor_retention_rejected,descriptor_retention_retained_bytes,"
    "descriptor_retention_rejected_bytes,descriptor_retention_limit_bytes,"
    "external_op_row_pct,copy_row_pct,trace_shape_row_mix_hint,"
    "external_op_row_lower_ms,copy_row_lower_ms,"
    "external_op_row_lower_ns_per_row,copy_row_lower_ns_per_row,"
    "external_op_row_lower_pct,copy_row_lower_pct,trace_shape_duration_hint,"
    "trace_shape_unit_cost_hint,"
    "trace_report_source_values_residual_ns_per_row,"
    "trace_report_row_validation_residual_ns_per_row,"
    "trace_report_visit_residual_ns_per_row,"
    "trace_report_descriptor_ns_per_row,"
    "external_op_runs,external_op_avg_run,external_op_max_run,"
    "copy_runs,copy_avg_run,copy_max_run,trace_shape_run_hint,"
    "trace_pipeline_action_hint,performance_focus_hint,trace_shape_profile_hint"
)
AGGREGATE_HEADER = (
    "aggregate,total_count,valid_total_count,total_min_ms,total_mean_ms,"
    "total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier,"
    "dominant_trace_pipeline_action_hint,trace_pipeline_action_consensus,"
    "dominant_trace_structure_hint,trace_structure_consensus,"
    "dominant_cuda_transfer_action_hint,cuda_transfer_action_consensus"
)
AGGREGATE_BY_INPUT_BYTES_HEADER = (
    "aggregate_by_input_bytes,input_bytes,total_count,valid_total_count,total_min_ms,"
    "total_mean_ms,total_median_ms,total_max_ms,sample_spread_pct,close_samples,"
    "max_outlier,dominant_trace_pipeline_action_hint,trace_pipeline_action_consensus,"
    "dominant_trace_structure_hint,trace_structure_consensus,"
    "dominant_cuda_transfer_action_hint,cuda_transfer_action_consensus"
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
    TRACE_LOWER_MS_KEY,
    TRACE_REPORT_MS_KEY,
    STREAM_ELAPSED_MS_KEY,
    STREAM_WORKER_MS_KEY,
    SEGMENT_COMMIT_MS_KEY,
    SEGMENT_COMMIT_ATTEMPT_MS_KEY,
    SEGMENT_COMMIT_OOM_RETRY_MS_KEY,
    SEGMENT_COMMIT_INITIAL_WORKERS_KEY,
    SEGMENT_COMMIT_EFFECTIVE_WORKERS_KEY,
    SEGMENT_COMMIT_WORKER_SUBMITS_KEY,
    SEGMENT_COMMIT_WORKER_JOINS_KEY,
    SEGMENT_COMMIT_WORKER_BACKPRESSURE_JOINS_KEY,
    SEGMENT_COMMIT_WORKER_BACKPRESSURE_JOIN_MS_KEY,
    SEGMENT_COMMIT_WORKER_FINISH_JOINS_KEY,
    SEGMENT_COMMIT_WORKER_FINISH_JOIN_MS_KEY,
    SEGMENT_COMMIT_WORKER_MAX_IN_FLIGHT_KEY,
    SEGMENT_COMMIT_OOM_RETRIES_KEY,
    SEGMENT_COMMIT_CUDA_MEMORY_TOTAL_BYTES_KEY,
    SEGMENT_COMMIT_CUDA_MEMORY_INITIAL_FREE_BYTES_KEY,
    SEGMENT_COMMIT_CUDA_MEMORY_EFFECTIVE_FREE_BYTES_KEY,
    SEGMENT_COMMIT_CUDA_MEMORY_MIN_FREE_BYTES_KEY,
    SEGMENT_COMMIT_CUDA_ALLOCATOR_INITIAL_CACHED_BYTES_KEY,
    SEGMENT_COMMIT_CUDA_ALLOCATOR_EFFECTIVE_CACHED_BYTES_KEY,
    SEGMENT_RECEIVE_WAIT_MS_KEY,
    PENDING_RECEIVE_WAIT_MS_KEY,
    PENDING_SEND_WAIT_MS_KEY,
    PARALLEL_LOWER_WORKERS_KEY,
    PARALLEL_LOWER_DISPATCHED_KEY,
    PARALLEL_LOWER_RECEIVED_KEY,
    PARALLEL_LOWER_EMITTED_KEY,
    PARALLEL_LOWER_MAX_REORDER_KEY,
    PARALLEL_LOWER_SNAPSHOT_REPLAY_KEY,
    PARALLEL_LOWER_REPORT_ELIDED_KEY,
    PARALLEL_LOWER_DISPATCH_WAIT_MS_KEY,
    PARALLEL_LOWER_RESULT_RECEIVE_WAIT_MS_KEY,
    PARALLEL_LOWER_DISPATCH_BLOCKED_KEY,
    SEGMENT_REPLAY_COUNT_KEY,
    TRACE_REPORTS_KEY,
    TRACE_REPORT_ROWS_KEY,
    TRACE_REPORT_CHUNK_SENT_KEY,
    TRACE_REPORT_CHUNK_RECEIVED_KEY,
    TRACE_REPORT_CHUNK_REPORTS_KEY,
    TRACE_REPORT_CHUNK_ROWS_KEY,
    TRACE_REPORT_CHUNK_MAX_QUEUED_KEY,
    TRACE_REPORT_VALIDATION_MS_KEY,
    TRACE_REPORT_LOWERING_MS_KEY,
    TRACE_REPORT_ROW_VALIDATION_MS_KEY,
    TRACE_REPORT_MEMORY_COLUMNS_MS_KEY,
    TRACE_REPORT_SOURCE_VALUES_MS_KEY,
    TRACE_REPORT_SOURCE_A_VALUE_MS_KEY,
    TRACE_REPORT_SOURCE_B_VALUE_MS_KEY,
    TRACE_REPORT_SOURCE_IMMEDIATE_READ_MS_KEY,
    TRACE_REPORT_SOURCE_REGISTER_READ_MS_KEY,
    TRACE_REPORT_SOURCE_MEMORY_READ_MS_KEY,
    TRACE_REPORT_SOURCE_INDIRECT_READ_MS_KEY,
    TRACE_REPORT_SOURCE_LAST_C_READ_MS_KEY,
    TRACE_REPORT_PRECOMPILE_MEMORY_MS_KEY,
    TRACE_REPORT_INSTRUCTION_RESULT_MS_KEY,
    TRACE_REPORT_NEXT_PC_MS_KEY,
    TRACE_REPORT_REGISTER_ACCESS_MS_KEY,
    TRACE_REPORT_MEMORY_ACCESS_MS_KEY,
    TRACE_REPORT_STORE_APPLY_MS_KEY,
    TRACE_REPORT_VISIT_MS_KEY,
    TRACE_REPORT_EMIT_MS_KEY,
    TRACE_DESCRIPTOR_MS_KEY,
    TRACE_SINGLE_ROW_REPORTS_KEY,
    TRACE_MULTI_ROW_REPORTS_KEY,
    TRACE_PENDING_DMA_REPORTS_KEY,
    TRACE_AMO_REPORTS_KEY,
    TRACE_STORE_CONDITIONAL_REPORTS_KEY,
    TRACE_EXTERNAL_OP_ROWS_KEY,
    TRACE_COPY_ROWS_KEY,
    TRACE_COPY_MEMORY_SOURCE_ROWS_KEY,
    TRACE_COPY_INDIRECT_MEMORY_ROWS_KEY,
    TRACE_COPY_REGISTER_STORE_ROWS_KEY,
    TRACE_COPY_MEMORY_STORE_ROWS_KEY,
    TRACE_COPY_NO_STORE_ROWS_KEY,
    TRACE_COPY_NO_MEMORY_ROWS_KEY,
    TRACE_EXTERNAL_OP_RUNS_KEY,
    TRACE_EXTERNAL_OP_MAX_RUN_KEY,
    TRACE_COPY_RUNS_KEY,
    TRACE_COPY_MAX_RUN_KEY,
    TRACE_EXTERNAL_OP_ROW_LOWER_MS_KEY,
    TRACE_COPY_ROW_LOWER_MS_KEY,
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
    TRACE_REPORT_SOURCE_IMMEDIATE_READS_KEY,
    TRACE_REPORT_SOURCE_REGISTER_READS_KEY,
    TRACE_REPORT_SOURCE_MEMORY_READS_KEY,
    TRACE_REPORT_SOURCE_INDIRECT_READS_KEY,
    TRACE_REPORT_SOURCE_LAST_C_READS_KEY,
    TRACE_REPORT_SOURCE_IMMEDIATE_READ_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_REGISTER_READ_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_MEMORY_READ_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_INDIRECT_READ_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_LAST_C_READ_SAMPLED_NS_KEY,
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
    TRACE_RUNNER_REPORT_BUFFER_CAPACITY_KEY,
    TRACE_RUNNER_REPORT_BUFFER_MAX_CAPACITY_KEY,
    TRACE_RUNNER_REPORT_BUFFER_EXCESS_CAPACITY_KEY,
    TRACE_REPORT_RECORD_SIZE_BYTES_KEY,
    TRACE_REPORT_INSTRUCTION_SIZE_BYTES_KEY,
    TRACE_REPORT_REGISTER_WRITE_LIST_SIZE_BYTES_KEY,
    TRACE_REPORT_MEMORY_ACCESS_LIST_SIZE_BYTES_KEY,
    TRACE_REPORT_PRECOMPILE_ACCESS_LIST_SIZE_BYTES_KEY,
    TRACE_REPORT_STORAGE_BYTES_KEY,
    TRACE_RUNNER_REPORT_BUFFER_CAPACITY_BYTES_KEY,
    TRACE_RUNNER_REPORT_BUFFER_EXCESS_BYTES_KEY,
    TRACE_REPORT_BUFFER_CAPACITY_BYTES_KEY,
    TRACE_REPORT_BUFFER_EXCESS_BYTES_KEY,
    DESCRIPTOR_ROWS_KEY,
    DESCRIPTOR_COMPACT_ROWS_KEY,
    DESCRIPTOR_WIDE_ROWS_KEY,
    DESCRIPTOR_UPLOAD_BYTES_KEY,
    DESCRIPTOR_UPLOAD_ROWS_KEY,
    DESCRIPTOR_HIGH32_VALUES_KEY,
    DESCRIPTOR_HIGH32_ROWS_KEY,
    DESCRIPTOR_HIGH32_STATS_ENABLED_KEY,
    DESCRIPTOR_HIGH32_A_VALUES_KEY,
    DESCRIPTOR_HIGH32_B_VALUES_KEY,
    DESCRIPTOR_HIGH32_C_VALUES_KEY,
    DESCRIPTOR_HIGH32_A_PAYLOAD_VALUES_KEY,
    DESCRIPTOR_HIGH32_B_PAYLOAD_VALUES_KEY,
    DESCRIPTOR_HIGH32_STORE_PAYLOAD_VALUES_KEY,
    DESCRIPTOR_HIGH32_STORE_PREV_VALUE_VALUES_KEY,
    *DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS,
    SEED_DIRECT_LIFT_ATTEMPTS_KEY,
    SEED_DIRECT_LIFT_SUCCESSES_KEY,
    SEED_DIRECT_LIFT_EMPTY_SEGMENTS_KEY,
    SEED_DIRECT_LIFT_PENDING_DMA_SINGLE_REPORTS_KEY,
    SEED_DIRECT_LIFT_AMO_BOUNDARIES_KEY,
    SEED_DIRECT_LIFT_STORE_CONDITIONAL_BOUNDARIES_KEY,
    SEED_DIRECT_LIFT_DMA_PREPARE_MISSING_LOOKAHEADS_KEY,
    SEED_DIRECT_LIFT_BOUNDARY_C_UNAVAILABLE_KEY,
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
    SOURCE_RETENTION_ATTEMPTS_KEY,
    SOURCE_RETENTION_RETAINED_KEY,
    SOURCE_RETENTION_REJECTED_KEY,
    SOURCE_RETENTION_RETAINED_BYTES_KEY,
    SOURCE_RETENTION_REJECTED_BYTES_KEY,
    SOURCE_RETENTION_MAX_RETAINED_BYTES_KEY,
    SOURCE_RETENTION_MAX_REJECTED_BYTES_KEY,
    SOURCE_RETENTION_LIMIT_BYTES_KEY,
    OPENING_ROW_VALUE_DEVICE_ROWS_KEY,
    OPENING_ROW_VALUE_SOURCE_ROWS_KEY,
    OPENING_ROW_VALUE_SOURCE_EXTEND_MS_KEY,
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
    OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_MS_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_ROWS_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_BYTES_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_LAUNCHES_KEY,
    OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_MS_KEY,
    OPENING_PATH_PARENT_HASH_LAUNCHES_PER_STAGE_KEY,
    OPENING_ROW_VALUE_DEVICE_DOWNLOAD_BATCHES_KEY,
    OPENING_ROW_VALUE_DEVICE_SINGLE_DOWNLOADS_KEY,
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
    CUDA_HOST_REGISTER_WAIT_NS_KEY,
    CUDA_COPY_H2D_BYTES_KEY,
    CUDA_COPY_H2D_WAIT_NS_KEY,
    CUDA_COPY_H2D_HOT_BYTES_KEY,
    CUDA_COPY_H2D_HOT_COUNT_KEY,
    CUDA_COPY_H2D_HOT_WAIT_NS_KEY,
    CUDA_COPY_D2H_BYTES_KEY,
    CUDA_COPY_D2H_WAIT_NS_KEY,
    CUDA_COPY_D2H_HOT_BYTES_KEY,
    CUDA_COPY_D2H_HOT_COUNT_KEY,
    CUDA_COPY_D2H_HOT_WAIT_NS_KEY,
    DESCRIPTOR_RETENTION_ATTEMPTS_KEY,
    DESCRIPTOR_RETENTION_RETAINED_KEY,
    DESCRIPTOR_RETENTION_REJECTED_KEY,
    DESCRIPTOR_RETENTION_RETAINED_BYTES_KEY,
    DESCRIPTOR_RETENTION_REJECTED_BYTES_KEY,
    DESCRIPTOR_RETENTION_LIMIT_BYTES_KEY,
}


def compact_csv_token(value: str) -> str:
    return value.replace(",", "|").replace(" ", "_")


def parse_timing_log(text: str) -> dict[str, int | str]:
    values: dict[str, int | str] = {}
    nsys_copy_block = None
    nsys_copy_backtrace_block = None
    nsys_kernel_block = None
    nsys_kernel_idle_gap_block = None
    ncu_metric_quality_block = None
    ncu_kernel_metric_block = None
    ncu_occupancy_block = None
    ncu_top_kernel: str | None = None
    ncu_top_duration_ms = -1.0
    ncu_top_kernel_limits: dict[str, str] = {}
    for line in text.splitlines():
        stripped = line.strip()
        if stripped == "cuda_transfer_triage":
            nsys_copy_block = stripped
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            continue
        if stripped == "cuda_api_backtrace_hint":
            nsys_copy_backtrace_block = stripped
            nsys_copy_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            continue
        if stripped == "stream_idle_gap_hotspots":
            nsys_kernel_idle_gap_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            continue
        if stripped == "cuda_graph_fusion_separation_triage":
            nsys_kernel_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            continue
        if stripped == "metric_collection_quality":
            ncu_metric_quality_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            continue
        if stripped == "kernel_metric_summary":
            ncu_kernel_metric_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_occupancy_block = None
            continue
        if stripped == "occupancy_limits":
            ncu_occupancy_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            continue
        if nsys_copy_block is not None:
            if not stripped:
                nsys_copy_block = None
                continue
            if stripped.startswith("metric,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                nsys_copy_block = None
                continue
            if (
                len(row) >= 2
                and row[0].strip() == "h2d_structural_hint"
                and row[1].strip() == "trace_descriptor_residency_pipeline"
            ):
                values[NSYS_COPY_TRACE_DESCRIPTOR_RESIDENCY_PIPELINE_KEY] = 1
            elif len(row) >= 2 and row[0].strip() == "gpu_residency_hint":
                values[NSYS_COPY_GPU_RESIDENCY_HINT_KEY] = row[1].strip()
            elif len(row) >= 2 and row[0].strip() == "small_d2h_batching_hint":
                values[NSYS_COPY_SMALL_D2H_BATCHING_HINT_KEY] = row[1].strip()
            elif (
                len(row) >= 3
                and row[0].strip() == "h2d_bulk_app_frame_hint"
                and row[1].strip() == "reuse_device_source_for_hot_frame"
                and (
                    "guest_pc_trace_backend::record_device_source_build_duration"
                    in row[2]
                    or "build_guest_pc_trace_stage_source_devices_from_device_material_timing"
                    in row[2]
                )
            ):
                values[NSYS_COPY_TRACE_DESCRIPTOR_RESIDENCY_PIPELINE_KEY] = 1
                values[NSYS_COPY_H2D_BULK_APP_FRAME_HINT_KEY] = compact_csv_token(
                    ",".join(row[2:]).strip()
                )
            continue
        if nsys_copy_backtrace_block is not None:
            if not stripped:
                nsys_copy_backtrace_block = None
                continue
            if stripped.startswith("missing_callchain_calls,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                nsys_copy_backtrace_block = None
                continue
            if len(row) >= 3 and ",".join(row[2:]).strip() != "none":
                values[NSYS_COPY_CUDA_API_BACKTRACE_HINT_KEY] = compact_csv_token(
                    ",".join(row[2:]).strip()
                )
            continue
        if nsys_kernel_idle_gap_block is not None:
            if not stripped:
                nsys_kernel_idle_gap_block = None
                continue
            if stripped.startswith("previous_kernel,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                nsys_kernel_idle_gap_block = None
                continue
            if (
                len(row) >= 4
                and NSYS_KERNEL_TOP_STREAM_IDLE_GAP_PREVIOUS_KEY not in values
            ):
                values[NSYS_KERNEL_TOP_STREAM_IDLE_GAP_PREVIOUS_KEY] = row[0].strip()
                values[NSYS_KERNEL_TOP_STREAM_IDLE_GAP_NEXT_KEY] = row[1].strip()
                values[NSYS_KERNEL_TOP_STREAM_IDLE_GAP_CALLS_KEY] = row[2].strip()
                values[NSYS_KERNEL_TOP_STREAM_IDLE_GAP_MS_KEY] = row[3].strip()
            continue
        if nsys_kernel_block is not None:
            if not stripped:
                nsys_kernel_block = None
                continue
            if stripped.startswith("metric,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                nsys_kernel_block = None
                continue
            if len(row) >= 2:
                metric = row[0].strip()
                value = row[1].strip()
                if metric == "graph_fusion_priority_hint":
                    values[NSYS_KERNEL_GRAPH_FUSION_PRIORITY_HINT_KEY] = value
                elif metric == "next_action_hint":
                    values[NSYS_KERNEL_NEXT_ACTION_HINT_KEY] = value
                elif metric == "graph_or_fusion_upper_bound_ms":
                    values[NSYS_KERNEL_GRAPH_FUSION_UPPER_BOUND_MS_KEY] = value
                elif metric == "top_stream_idle_ms":
                    values[NSYS_KERNEL_TOP_STREAM_IDLE_MS_KEY] = value
                elif metric == "kernel_separation_hint":
                    values[NSYS_KERNEL_SEPARATION_HINT_KEY] = value
            continue
        if ncu_metric_quality_block is not None:
            if not stripped:
                ncu_metric_quality_block = None
                continue
            if stripped.startswith("metric,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                ncu_metric_quality_block = None
                continue
            if len(row) >= 2 and row[0].strip() == "collection_hint":
                values[NCU_METRIC_COLLECTION_HINT_KEY] = row[1].strip()
            continue
        if ncu_kernel_metric_block is not None:
            if not stripped:
                ncu_kernel_metric_block = None
                continue
            if stripped.startswith("kernel,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                ncu_kernel_metric_block = None
                continue
            if len(row) >= 11:
                kernel = row[0].strip()
                try:
                    duration_ms = float(row[2].strip())
                except ValueError:
                    continue
                if duration_ms > ncu_top_duration_ms:
                    ncu_top_duration_ms = duration_ms
                    ncu_top_kernel = kernel
                    values[NCU_TOP_KERNEL_KEY] = kernel
                    values[NCU_TOP_KERNEL_DURATION_MS_KEY] = row[2].strip()
                    values[NCU_TOP_KERNEL_SM_THROUGHPUT_PCT_KEY] = row[4].strip()
                    values[NCU_TOP_KERNEL_DRAM_THROUGHPUT_PCT_KEY] = row[5].strip()
                    values[NCU_TOP_KERNEL_REGISTERS_PER_THREAD_KEY] = row[9].strip()
            continue
        if ncu_occupancy_block is not None:
            if not stripped:
                ncu_occupancy_block = None
                continue
            if stripped.startswith("kernel,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                ncu_occupancy_block = None
                continue
            if len(row) >= 9:
                ncu_top_kernel_limits[row[0].strip()] = compact_csv_token(row[8].strip())
            continue
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        if (
            key not in TIMING_KEYS
            and OPENING_STAGE_ROW_VALUE_DEVICE_SINGLE_DOWNLOAD_RE.match(key) is None
        ):
            continue
        try:
            values[key] = int(value.strip())
        except ValueError:
            continue
    if ncu_top_kernel is not None and ncu_top_kernel in ncu_top_kernel_limits:
        values[NCU_TOP_KERNEL_LIMITING_FACTORS_KEY] = ncu_top_kernel_limits[ncu_top_kernel]
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
        PERF_PREPARE_INSTRUCTION_SELF_PCT_KEY: 0.0,
        PERF_TRACE_SEGMENT_BUILD_SELF_PCT_KEY: 0.0,
        PERF_APPEND_DESCRIPTOR_SELF_PCT_KEY: 0.0,
        PERF_SOURCE_VALUE_SELF_PCT_KEY: 0.0,
        PERF_ADVANCE_GUEST_MACHINE_SELF_PCT_KEY: 0.0,
        PERF_GUEST_MEMORY_WRITE_SELF_PCT_KEY: 0.0,
        PERF_BIGUINT_MODPOW_SELF_PCT_KEY: 0.0,
        PERF_GUEST_MEMORY_READ_SELF_PCT_KEY: 0.0,
        PERF_DECODE_INSTRUCTION_SELF_PCT_KEY: 0.0,
        PERF_EFFECT_RECORD_MEMORY_WRITE_SELF_PCT_KEY: 0.0,
        PERF_EFFECT_RECORD_MEMORY_READ_SELF_PCT_KEY: 0.0,
        CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY: 0.0,
    }

    def record_symbol_hotspot(symbol_text: str, pct: float) -> bool:
        key = None
        if "lowered_report_row" in symbol_text:
            key = PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY
        elif "memmove" in symbol_text or "memcpy" in symbol_text:
            key = PERF_MEMMOVE_SELF_PCT_KEY
            if "lzvm-gp-runner" in symbol_text:
                hotspots[PERF_MEMMOVE_RUNNER_THREAD_PCT_KEY] = max(
                    hotspots[PERF_MEMMOVE_RUNNER_THREAD_PCT_KEY], pct
                )
            elif "lzvm-gp-lower" in symbol_text:
                hotspots[PERF_MEMMOVE_LOWER_THREAD_PCT_KEY] = max(
                    hotspots[PERF_MEMMOVE_LOWER_THREAD_PCT_KEY], pct
                )
        elif "sha2::sha256" in symbol_text or "digest_blocks" in symbol_text:
            key = PERF_SHA256_SELF_PCT_KEY
        elif (
            "GuestPcTracePendingSegmentSlice" in symbol_text
            and "drop_in_place" in symbol_text
        ):
            key = PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY
        elif "prepare_current_guest_instruction" in symbol_text:
            key = PERF_PREPARE_INSTRUCTION_SELF_PCT_KEY
        elif ("build_layout_" "z" "isk_main_trace_segment_for_segment_output") in symbol_text:
            key = PERF_TRACE_SEGMENT_BUILD_SELF_PCT_KEY
        elif "append_main_device_trace_descriptor" in symbol_text:
            key = PERF_APPEND_DESCRIPTOR_SELF_PCT_KEY
        elif ("z" "isk_main_source_value") in symbol_text:
            key = PERF_SOURCE_VALUE_SELF_PCT_KEY
        elif "advance_guest_machine_prepared_inner" in symbol_text:
            key = PERF_ADVANCE_GUEST_MACHINE_SELF_PCT_KEY
        elif "GuestMachineMemorySegment::write_range" in symbol_text:
            key = PERF_GUEST_MEMORY_WRITE_SELF_PCT_KEY
        elif "monty_modpow" in symbol_text:
            key = PERF_BIGUINT_MODPOW_SELF_PCT_KEY
        elif "GuestMachineMemory::read_range_into" in symbol_text:
            key = PERF_GUEST_MEMORY_READ_SELF_PCT_KEY
        elif (
            "decode_guest_instruction" in symbol_text
            or "decode_riscv_instruction" in symbol_text
        ):
            key = PERF_DECODE_INSTRUCTION_SELF_PCT_KEY
        elif "GuestInstructionEffects::record_memory_write" in symbol_text:
            key = PERF_EFFECT_RECORD_MEMORY_WRITE_SELF_PCT_KEY
        elif "GuestInstructionEffects::record_memory_read" in symbol_text:
            key = PERF_EFFECT_RECORD_MEMORY_READ_SELF_PCT_KEY
        if key is None:
            return False
        hotspots[key] = max(hotspots[key], pct)
        return True

    in_memmove_callchain = False
    in_sha256_callchain = False
    nsys_cpu_block = None
    for line in text.splitlines():
        stripped = line.strip()
        if stripped in NSYS_CPU_HOTSPOT_BLOCKS:
            nsys_cpu_block = stripped
            continue
        if stripped == NSYS_CPU_MEMCPY_ACTION_HINT_BLOCK:
            nsys_cpu_block = stripped
            continue
        if nsys_cpu_block is not None:
            if not stripped:
                nsys_cpu_block = None
                continue
            if stripped.startswith("symbol,") or stripped.startswith(
                "nearest_app_symbol,"
            ):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                nsys_cpu_block = None
                continue
            if nsys_cpu_block == NSYS_CPU_MEMCPY_ACTION_HINT_BLOCK:
                if len(row) < 4:
                    nsys_cpu_block = None
                    continue
                try:
                    pct = float(row[2])
                except ValueError:
                    nsys_cpu_block = None
                    continue
                if row[3].strip() == "trace_report_storage_structural_candidate":
                    hotspots[CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY] = max(
                        hotspots[CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY],
                        pct,
                    )
                continue
            if len(row) < 4:
                nsys_cpu_block = None
                continue
            try:
                pct = float(row[3])
            except ValueError:
                nsys_cpu_block = None
                continue
            if record_symbol_hotspot(row[0], pct):
                continue

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
            if not record_symbol_hotspot(symbol_text, pct):
                continue
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
    trace_lower_ms: int,
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
        ("trace_lower", float(trace_lower_ms)),
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
    trace_lower_ms: int,
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
    trace_work_ms = max(runner_ms, trace_lower_ms or lowerer_ms)
    trace_floor_ms = max(trace_work_ms, lowerer_ms, stream_elapsed_ms)
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


def trace_pipeline_action_hint(
    total_ms: int,
    runner_ms: int,
    lowerer_ms: int,
    trace_lower_ms: int,
    stream_elapsed_ms: int,
    segment_commit_ms: int,
    segment_receive_wait_ms: int,
    pending_receive_wait_ms: int,
    parallel_lower_workers: int,
) -> str:
    if total_ms <= 0:
        return "unknown"
    if total_ms <= PROOF_TARGET_MS:
        return "within_target"

    trace_floor_ms = max(runner_ms, lowerer_ms, trace_lower_ms, stream_elapsed_ms)
    if trace_floor_ms <= 0 and segment_commit_ms <= 0:
        return "timing_breakdown_needed"

    trace_is_long = trace_floor_ms >= PROOF_TARGET_MS or trace_floor_ms >= total_ms * 0.55
    commit_is_long = (
        segment_commit_ms >= PROOF_TARGET_MS
        or segment_commit_ms >= total_ms * 0.25
        or (
            segment_receive_wait_ms >= total_ms * 0.30
            and segment_commit_ms >= total_ms * 0.15
        )
    )
    queue_wait_is_long = (
        segment_receive_wait_ms >= total_ms * 0.25
        or pending_receive_wait_ms >= total_ms * 0.15
    )
    pending_receive_wait_ratio = (
        pending_receive_wait_ms / stream_elapsed_ms if stream_elapsed_ms else 0.0
    )
    runner_stream_ratio = runner_ms / stream_elapsed_ms if stream_elapsed_ms else 0.0

    if (
        trace_is_long
        and parallel_lower_workers > 1
        and pending_receive_wait_ratio >= 0.5
        and runner_stream_ratio >= 0.75
    ):
        return "parallel_lower_active_compare_default"

    if trace_is_long and commit_is_long and queue_wait_is_long:
        return "trace_generation_and_commit_pipeline_candidate"
    if trace_is_long:
        if lowerer_ms >= runner_ms * 0.75 and trace_lower_ms >= lowerer_ms * 0.70:
            if parallel_lower_workers <= 1:
                return "parallel_trace_lowering_candidate"
            return "parallel_trace_lowering_active"
        if runner_ms >= lowerer_ms * 1.25:
            return "guest_runner_parallelism_candidate"
        return "trace_generation_parallelism_candidate"
    if commit_is_long and queue_wait_is_long:
        return "commit_trace_overlap_candidate"
    if commit_is_long:
        return "segment_commit_candidate"
    if queue_wait_is_long:
        return "trace_queue_backpressure_candidate"
    return "balanced_pipeline"


def segment_commit_worker_pressure_hint(
    worker_submits: int,
    worker_backpressure_joins: int,
    worker_backpressure_join_ms: int,
    worker_finish_joins: int,
    worker_finish_join_ms: int,
    worker_max_in_flight: int,
    effective_workers: int,
) -> str:
    if worker_submits <= 0 or effective_workers <= 0:
        return "none"
    backpressure_ratio = worker_backpressure_joins / worker_submits
    worker_queue_filled = (
        effective_workers > 1 and worker_max_in_flight >= effective_workers
    )
    backpressure_is_dominant = (
        worker_backpressure_join_ms >= 1000
        and worker_backpressure_join_ms >= worker_finish_join_ms * 2
    )
    if worker_queue_filled and worker_backpressure_joins > 0:
        if backpressure_ratio >= 0.5 or backpressure_is_dominant:
            return "worker_backpressure_dominant"
        return "worker_backpressure_present"
    if (
        worker_finish_joins > 0
        and worker_finish_join_ms >= 1000
        and worker_finish_join_ms > worker_backpressure_join_ms
    ):
        return "worker_finish_drain_dominant"
    if worker_queue_filled:
        return "worker_queue_filled"
    return "no_worker_pressure"


def trace_pipeline_action_hint_from_values(values: dict[str, int]) -> str:
    base_hint = trace_pipeline_action_hint(
        values.get(TOTAL_MS_KEY, 0),
        values.get(RUNNER_MS_KEY, 0),
        values.get(LOWERER_MS_KEY, 0),
        values.get(TRACE_LOWER_MS_KEY, 0),
        values.get(STREAM_ELAPSED_MS_KEY, 0),
        values.get(SEGMENT_COMMIT_MS_KEY, 0),
        values.get(SEGMENT_RECEIVE_WAIT_MS_KEY, 0),
        values.get(PENDING_RECEIVE_WAIT_MS_KEY, 0),
        values.get(PARALLEL_LOWER_WORKERS_KEY, 0),
    )
    if base_hint in {
        "trace_generation_and_commit_pipeline_candidate",
        "parallel_trace_lowering_candidate",
        "trace_generation_parallelism_candidate",
    } and trace_shape_points_to_segment_reexecution(values):
        return "parallel_segment_reexecution_authorization_required"
    return base_hint


def trace_shape_points_to_segment_reexecution(values: dict[str, int]) -> bool:
    trace_report_rows = values.get(TRACE_REPORT_ROWS_KEY, 0)
    if trace_report_rows <= 0:
        return False
    trace_shape_hint = trace_shape_sample_hint(values, trace_report_rows)
    external_op_rows = values.get(TRACE_EXTERNAL_OP_ROWS_KEY, 0)
    copy_rows = values.get(TRACE_COPY_ROWS_KEY, 0)
    indirect_memory_rows = values.get(TRACE_INDIRECT_MEMORY_ROWS_KEY, 0)
    external_op_row_pct = external_op_rows * 100.0 / trace_report_rows
    copy_row_pct = copy_rows * 100.0 / trace_report_rows
    indirect_memory_row_pct = indirect_memory_rows * 100.0 / trace_report_rows
    trace_shape_row_mix = trace_shape_row_mix_hint(
        trace_shape_hint,
        external_op_row_pct,
        copy_row_pct,
        indirect_memory_row_pct,
    )
    trace_lower_ms = values.get(TRACE_LOWER_MS_KEY, 0)
    external_op_row_lower_ms = values.get(TRACE_EXTERNAL_OP_ROW_LOWER_MS_KEY, 0)
    copy_row_lower_ms = values.get(TRACE_COPY_ROW_LOWER_MS_KEY, 0)
    external_op_row_lower_pct = (
        external_op_row_lower_ms * 100.0 / trace_lower_ms if trace_lower_ms else 0.0
    )
    copy_row_lower_pct = (
        copy_row_lower_ms * 100.0 / trace_lower_ms if trace_lower_ms else 0.0
    )
    external_op_row_lower_ns_per_row = ns_per_row_from_ms(
        external_op_row_lower_ms,
        external_op_rows,
    )
    copy_row_lower_ns_per_row = ns_per_row_from_ms(
        copy_row_lower_ms,
        copy_rows,
    )
    trace_shape_duration = trace_shape_duration_hint(
        external_op_row_lower_pct,
        copy_row_lower_pct,
    )
    trace_shape_unit_cost = trace_shape_unit_cost_hint(
        external_op_row_lower_ns_per_row,
        copy_row_lower_ns_per_row,
        trace_shape_row_mix,
    )
    return (
        trace_shape_hint == "shape_timing_enabled"
        and trace_shape_row_mix == "copy_and_external_op_rows_dominate"
        and trace_shape_duration in {
            "copy_and_external_op_duration_dominate",
            "mixed_trace_shape_duration",
        }
        and trace_shape_unit_cost == "row_volume_dominates_shape_duration"
    )


def cuda_transfer_action_hint_from_values(values: dict[str, int]) -> str:
    h2d_bytes = values.get(CUDA_COPY_H2D_BYTES_KEY, 0)
    h2d_wait_ms = values.get(CUDA_COPY_H2D_WAIT_NS_KEY, 0) / 1_000_000.0
    host_register_wait_ms = (
        values.get(CUDA_HOST_REGISTER_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    h2d_hot_count = values.get(CUDA_COPY_H2D_HOT_COUNT_KEY, 0)
    h2d_hot_wait_ms = values.get(CUDA_COPY_H2D_HOT_WAIT_NS_KEY, 0) / 1_000_000.0
    d2h_hot_count = values.get(DIRECT_D2H_HOT_COUNT_KEY, 0)
    d2h_hot_wait_ms = values.get(DIRECT_D2H_HOT_WAIT_NS_KEY, 0) / 1_000_000.0
    allocator_d2h_hot_count = values.get(CUDA_COPY_D2H_HOT_COUNT_KEY, 0)
    allocator_d2h_hot_wait_ms = (
        values.get(CUDA_COPY_D2H_HOT_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    descriptor_upload_bytes = values.get(DESCRIPTOR_UPLOAD_BYTES_KEY, 0)
    descriptor_attempts = values.get(DESCRIPTOR_RETENTION_ATTEMPTS_KEY, 0)
    descriptor_retained = values.get(DESCRIPTOR_RETENTION_RETAINED_KEY, 0)
    descriptor_rejected = values.get(DESCRIPTOR_RETENTION_REJECTED_KEY, 0)

    if (
        descriptor_upload_bytes > 0
        and max(h2d_wait_ms, host_register_wait_ms) >= CUDA_TRANSFER_WAIT_MS_THRESHOLD
        and descriptor_attempts > 0
        and descriptor_retained == descriptor_attempts
        and descriptor_rejected == 0
    ):
        return "initial_descriptor_upload_retention_active"
    if (
        descriptor_retained > 0
        and descriptor_rejected > 0
        and d2h_hot_count >= CUDA_TRANSFER_HOT_COPY_COUNT_THRESHOLD
        and d2h_hot_wait_ms >= CUDA_TRANSFER_WAIT_MS_THRESHOLD
    ):
        return "retained_descriptor_d2h_tradeoff"
    if (
        h2d_bytes >= CUDA_TRANSFER_BULK_H2D_BYTES_THRESHOLD
        and max(h2d_wait_ms, host_register_wait_ms) >= CUDA_TRANSFER_WAIT_MS_THRESHOLD
    ):
        return "reduce_bulk_h2d_source_uploads"
    if (
        h2d_hot_count >= CUDA_TRANSFER_HOT_COPY_COUNT_THRESHOLD
        and h2d_hot_wait_ms >= CUDA_TRANSFER_WAIT_MS_THRESHOLD
    ):
        return "coalesce_hot_h2d_uploads"
    if (
        d2h_hot_count >= CUDA_TRANSFER_HOT_COPY_COUNT_THRESHOLD
        and d2h_hot_wait_ms >= CUDA_TRANSFER_WAIT_MS_THRESHOLD
    ):
        return "batch_or_keep_small_d2h_on_device"
    if (
        allocator_d2h_hot_count >= CUDA_TRANSFER_HOT_COPY_COUNT_THRESHOLD
        and allocator_d2h_hot_wait_ms >= CUDA_TRANSFER_WAIT_MS_THRESHOLD
    ):
        return "batch_or_keep_small_d2h_on_device"
    if h2d_bytes >= CUDA_TRANSFER_BULK_H2D_BYTES_THRESHOLD:
        return "inspect_bulk_h2d_source_uploads"
    return "none"


def allocator_d2h_action_hint(
    allocator_d2h_wait_ms: float,
    allocator_d2h_hot_count: int,
    allocator_d2h_hot_wait_pct: float,
    opening_query_units: int,
    opening_single_query_units: int,
    opening_row_value_device_rows: int,
    opening_row_value_device_download_batches: int,
) -> str:
    if allocator_d2h_wait_ms < CUDA_TRANSFER_WAIT_MS_THRESHOLD:
        return "none"
    hot_bucket = (
        allocator_d2h_hot_count >= CUDA_TRANSFER_HOT_COPY_COUNT_THRESHOLD
        and allocator_d2h_hot_wait_pct >= DIRECT_D2H_HOT_WAIT_PCT_THRESHOLD
    )
    if hot_bucket:
        if (
            opening_query_units > 1
            and opening_single_query_units >= opening_query_units
            and opening_row_value_device_rows > 0
            and opening_row_value_device_download_batches == 0
        ):
            return "opening_row_value_d2h_wait_secondary"
        return "batch_or_keep_hot_allocator_d2h_on_device"
    return "inspect_allocator_d2h_waits"


def direct_d2h_action_hint(
    direct_d2h_wait_ms: float,
    direct_d2h_hot_count: int,
    direct_d2h_hot_wait_pct: float,
    opening_query_units: int,
    opening_single_query_units: int,
    opening_row_value_device_rows: int,
    opening_row_value_device_download_batches: int,
    root_count: int,
    materialization_groups: int,
    materialization_max_group_size: int,
) -> str:
    if direct_d2h_wait_ms < CUDA_TRANSFER_WAIT_MS_THRESHOLD:
        return "none"
    hot_bucket = (
        direct_d2h_hot_count >= CUDA_TRANSFER_HOT_COPY_COUNT_THRESHOLD
        and direct_d2h_hot_wait_pct >= DIRECT_D2H_HOT_WAIT_PCT_THRESHOLD
    )
    single_root_groups = (
        root_count > 1
        and materialization_groups >= root_count
        and materialization_max_group_size <= 1
    )
    single_query_row_value_reads = (
        opening_row_value_device_rows > 0
        and opening_row_value_device_download_batches == 0
        and opening_query_units > 0
        and opening_single_query_units >= opening_query_units
    )
    if hot_bucket and single_query_row_value_reads:
        return SINGLE_QUERY_ROW_VALUE_BOUNDARY_HINT
    if hot_bucket and single_root_groups:
        return "batch_hot_direct_d2h_root_reads"
    if hot_bucket:
        return "keep_hot_direct_d2h_on_device"
    return "inspect_direct_d2h_waits"


def segment_commit_memory_pressure_hint_from_values(values: dict[str, int]) -> str:
    total_bytes = values.get(SEGMENT_COMMIT_CUDA_MEMORY_TOTAL_BYTES_KEY, 0)
    min_free_bytes = values.get(SEGMENT_COMMIT_CUDA_MEMORY_MIN_FREE_BYTES_KEY, 0)
    initial_workers = values.get(SEGMENT_COMMIT_INITIAL_WORKERS_KEY, 0)
    effective_workers = values.get(SEGMENT_COMMIT_EFFECTIVE_WORKERS_KEY, 0)
    oom_retries = values.get(SEGMENT_COMMIT_OOM_RETRIES_KEY, 0)

    if oom_retries > 0 or (
        initial_workers > 0
        and effective_workers > 0
        and effective_workers < initial_workers
    ):
        return "segment_commit_oom_fallback"
    if total_bytes <= 0:
        return "memory_timing_missing"

    min_free_pct = min_free_bytes * 100.0 / total_bytes
    if min_free_pct <= SEGMENT_COMMIT_MEMORY_PRESSURE_PCT_THRESHOLD:
        return "segment_commit_memory_pressure"
    if min_free_pct <= SEGMENT_COMMIT_MEMORY_THIN_MARGIN_PCT_THRESHOLD:
        return "segment_commit_memory_thin_margin"
    return "segment_commit_memory_margin_ok"


def cpu_trace_hotspot_hint(perf_hotspots: dict[str, float]) -> str:
    lowered_report_row_pct = perf_hotspots.get(
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY, 0.0
    )
    memmove_pct = perf_hotspots.get(PERF_MEMMOVE_SELF_PCT_KEY, 0.0)
    pending_drop_pct = perf_hotspots.get(
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY, 0.0
    )
    if (
        lowered_report_row_pct >= 15.0
        and memmove_pct >= 10.0
        and pending_drop_pct >= 5.0
    ):
        return "report_lifetime_and_data_movement"
    if lowered_report_row_pct >= 20.0 and memmove_pct >= 15.0:
        return "report_lifetime_and_data_movement"
    if lowered_report_row_pct >= 20.0:
        return "lowered_report_rows"
    if memmove_pct >= 15.0:
        return "guest_state_copies"
    if pending_drop_pct >= 5.0:
        return "pending_segment_lifetime"
    return "none"


def cpu_trace_report_storage_action_hint(
    values: dict[str, int],
    perf_hotspots: dict[str, float],
) -> str:
    if (
        values.get(TRACE_REPORTS_KEY, 0) > 0
        and TRACE_REPORT_RECORD_SIZE_BYTES_KEY not in values
        and TRACE_REPORT_STORAGE_BYTES_KEY not in values
    ):
        return "refresh_trace_report_storage_timing"
    lowered_report_row_pct = perf_hotspots.get(
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY, 0.0
    )
    memmove_pct = perf_hotspots.get(PERF_MEMMOVE_SELF_PCT_KEY, 0.0)
    memmove_trace_slice_pct = perf_hotspots.get(
        PERF_MEMMOVE_TRACE_SLICE_PCT_KEY, 0.0
    )
    pending_drop_pct = perf_hotspots.get(
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY, 0.0
    )
    trace_reports = values.get(TRACE_REPORTS_KEY, 0)
    buffer_capacity = values.get(TRACE_REPORT_BUFFER_CAPACITY_KEY, 0)
    runner_buffer_capacity = values.get(TRACE_RUNNER_REPORT_BUFFER_CAPACITY_KEY, 0)
    buffer_excess_capacity = values.get(TRACE_REPORT_BUFFER_EXCESS_CAPACITY_KEY, 0)
    chunks_sent = values.get(TRACE_REPORT_CHUNK_SENT_KEY, 0)
    buffer_excess_pct = (
        buffer_excess_capacity * 100.0 / buffer_capacity if buffer_capacity else 0.0
    )
    if chunks_sent > 0 and buffer_capacity == 0 and runner_buffer_capacity > 0:
        return "post_segment_report_chunk_split"
    if (
        trace_reports > 0
        and buffer_capacity > 0
        and buffer_excess_pct <= 1.0
        and pending_drop_pct >= 5.0
        and chunks_sent <= 0
    ):
        return "runner_streaming_report_storage_candidate"
    report_storage_memcpy_pct = perf_hotspots.get(
        CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY, 0.0
    )
    report_storage_memcpy_total_pct = memmove_pct * report_storage_memcpy_pct / 100.0
    if report_storage_memcpy_pct > 0.0:
        if (
            report_storage_memcpy_total_pct
            >= CPU_TRACE_REPORT_STORAGE_STRUCTURAL_TOTAL_PCT_THRESHOLD
        ):
            return "trace_report_storage_structural_candidate"
        return "trace_report_storage_memcpy_secondary"
    if (
        lowered_report_row_pct >= 15.0
        and pending_drop_pct >= 5.0
        and (memmove_trace_slice_pct >= 5.0 or memmove_pct >= 10.0)
    ):
        return "report_sidecar_storage_candidate"
    if pending_drop_pct >= 5.0 and memmove_trace_slice_pct >= 5.0:
        return "trace_slice_drop_storage_candidate"
    return "none"


def cpu_trace_lowerer_action_hint(
    perf_hotspots: dict[str, float], trace_report_detail_action: str = "none"
) -> str:
    detail_hints = {
        "profile_row_validation_residual": "row_validation_residual_profile_candidate",
        "profile_row_validation": "row_validation_profile_candidate",
        "profile_source_values_residual": "source_values_residual_profile_candidate",
        "profile_source_values": "source_values_profile_candidate",
        "profile_descriptor_write": "descriptor_append_candidate",
        "profile_visit": "visit_profile_candidate",
    }
    if trace_report_detail_action in detail_hints:
        return detail_hints[trace_report_detail_action]
    lowered_report_row_pct = perf_hotspots.get(
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY, 0.0
    )
    append_descriptor_pct = perf_hotspots.get(
        PERF_APPEND_DESCRIPTOR_SELF_PCT_KEY, 0.0
    )
    source_value_pct = perf_hotspots.get(PERF_SOURCE_VALUE_SELF_PCT_KEY, 0.0)
    if lowered_report_row_pct < 10.0:
        return "none"
    if append_descriptor_pct >= 5.0:
        return "descriptor_append_candidate"
    if source_value_pct >= 2.5:
        return "source_value_candidate"
    return "lowered_report_row_candidate"


def cpu_runner_hotspot_hint(perf_hotspots: dict[str, float]) -> str:
    prepare_pct = perf_hotspots.get(PERF_PREPARE_INSTRUCTION_SELF_PCT_KEY, 0.0)
    advance_pct = perf_hotspots.get(PERF_ADVANCE_GUEST_MACHINE_SELF_PCT_KEY, 0.0)
    memory_pct = perf_hotspots.get(
        PERF_GUEST_MEMORY_WRITE_SELF_PCT_KEY, 0.0
    ) + perf_hotspots.get(PERF_GUEST_MEMORY_READ_SELF_PCT_KEY, 0.0)
    modpow_pct = perf_hotspots.get(PERF_BIGUINT_MODPOW_SELF_PCT_KEY, 0.0)
    if prepare_pct >= 4.0 and advance_pct >= 4.0:
        return "instruction_prepare_and_advance"
    if prepare_pct >= 4.0:
        return "instruction_prepare"
    if advance_pct >= 4.0:
        return "guest_machine_advance"
    if memory_pct >= 4.0:
        return "guest_memory_access"
    if modpow_pct >= 2.0:
        return "biguint_modpow"
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


def sparse_high32_descriptor_estimate(
    descriptor_rows: int,
    descriptor_upload_bytes: int,
    descriptor_high32_values: int,
    high32_rows_present: bool,
) -> tuple[int, float, int, str]:
    if (
        descriptor_rows <= 0
        or descriptor_upload_bytes <= 0
        or not high32_rows_present
    ):
        return 0, 0.0, 0, "none"
    high_words = (descriptor_high32_values + 1) // 2
    estimated_words = (
        descriptor_rows * SPARSE_HIGH32_DESCRIPTOR_BASE_WORDS_PER_ROW
        + high_words
    )
    estimated_bytes = estimated_words * WORD_BYTES
    if estimated_bytes >= descriptor_upload_bytes:
        return estimated_bytes, 0.0, high_words, "sparse_high32_not_smaller"
    savings_pct = (descriptor_upload_bytes - estimated_bytes) * 100.0 / descriptor_upload_bytes
    if savings_pct >= 10.0:
        hint = "sparse_high32_descriptor_candidate"
    else:
        hint = "sparse_high32_marginal"
    return estimated_bytes, savings_pct, high_words, hint


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
    row_value_device_rows: int,
    row_value_device_download_batches: int,
    retained_leaf_openings: int,
    retained_leaf_rows: int,
    retained_leaf_all_single_row: int,
    retained_leaf_path_launches: int,
    retained_parent_checkpoint_openings: int,
    retained_parent_checkpoint_rows: int,
    retained_parent_checkpoint_all_single_row: int,
    retained_parent_checkpoint_prefix_launches: int,
    retained_parent_checkpoint_suffix_launches: int,
    retained_parent_checkpoint_path_ms: int,
    direct_d2h_wait_ms: float,
) -> str:
    if direct_d2h_wait_ms < OPENING_BATCHING_D2H_WAIT_MS_THRESHOLD:
        return "none"
    if query_units and single_query_units != query_units:
        return "none"
    if (
        query_units > 1
        and row_value_device_rows > 1
        and row_value_device_download_batches == 0
    ):
        if single_query_units >= query_units:
            return SINGLE_QUERY_ROW_VALUE_BOUNDARY_HINT
        return "multi_buffer_device_row_value_gather_candidate"
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
        if (
            retained_parent_checkpoint_path_ms > 0
            and retained_parent_checkpoint_path_ms
            < RETAINED_PARENT_CHECKPOINT_PATH_SECONDARY_MS_THRESHOLD
        ):
            return "retained_parent_checkpoint_path_time_secondary"
        return "cross_stage_retained_parent_checkpoint_prefix_suffix_gather_candidate"
    return "none"


def opening_external_source_boundary_hint(
    external_source_count: int,
    query_units: int,
    single_query_units: int,
    row_value_device_rows: int,
    row_value_device_download_batches: int,
    row_value_device_single_downloads: int,
    direct_d2h_wait_ms: float,
) -> str:
    if direct_d2h_wait_ms < OPENING_BATCHING_D2H_WAIT_MS_THRESHOLD:
        return "none"
    if external_source_count <= 0 or query_units <= 1:
        return "none"
    if single_query_units < query_units:
        return "none"
    if row_value_device_rows <= 1 or row_value_device_download_batches != 0:
        return "none"
    if row_value_device_single_downloads <= 1:
        return "none"
    return EXTERNAL_SOURCE_ROW_VALUE_BOUNDARY_HINT


def opening_device_single_stage_shape(values: dict[str, int]) -> tuple[int, int, int]:
    stage_counts = [
        count
        for key, count in values.items()
        if OPENING_STAGE_ROW_VALUE_DEVICE_SINGLE_DOWNLOAD_RE.match(key) is not None
        and count > 0
    ]
    if not stage_counts:
        return (0, 0, 0)
    stage_count = len(stage_counts)
    max_stage_count = max(stage_counts)
    batch_savings = sum(count - 1 for count in stage_counts)
    return (stage_count, max_stage_count, batch_savings)


def retained_parent_checkpoint_cross_stage_gather_launch_shape(
    openings: int,
    rows: int,
    all_single_row: int,
    prefix_launches: int,
    suffix_launches: int,
) -> tuple[int, int, int]:
    current_launches = prefix_launches + suffix_launches
    if (
        openings <= 1
        or rows != openings
        or all_single_row <= 0
        or current_launches <= 0
    ):
        return (current_launches, 0, 0)
    prefix_group_launches = (
        (prefix_launches + openings - 1) // openings if prefix_launches > 0 else 0
    )
    suffix_group_launches = (
        (suffix_launches + openings - 1) // openings if suffix_launches > 0 else 0
    )
    estimated_launches = prefix_group_launches + suffix_group_launches
    launch_savings = max(current_launches - estimated_launches, 0)
    return (current_launches, estimated_launches, launch_savings)


def retained_parent_checkpoint_batching_hint(
    openings: int,
    rows: int,
    all_single_row: int,
    prefix_launches: int,
    suffix_launches: int,
    path_ms: int,
    cross_stage_gather_launch_savings: int,
) -> str:
    path_launches = prefix_launches + suffix_launches
    if openings <= 0 or rows <= 0 or path_launches <= 0:
        return "none"
    if rows != openings:
        return "multi_row_openings_batched"
    if all_single_row <= 0:
        return "mixed_query_opening_shape"
    if cross_stage_gather_launch_savings <= 0:
        return "device_batched_per_stage"
    if path_ms > 0 and path_ms < RETAINED_PARENT_CHECKPOINT_PATH_SECONDARY_MS_THRESHOLD:
        return "device_batched_path_secondary"
    return "device_batched_cross_stage_candidate"


def opening_retained_parent_checkpoint_action_hint(
    openings: int,
    rows: int,
    all_single_row: int,
    single_query_units: int,
    path_launches: int,
    path_ms: int,
    cross_stage_gather_launch_savings: int,
) -> str:
    if (
        openings <= 1
        or rows != openings
        or all_single_row <= 0
        or single_query_units < openings
        or path_launches <= openings
        or cross_stage_gather_launch_savings <= 0
    ):
        return "none"
    if path_ms > 0 and path_ms < RETAINED_PARENT_CHECKPOINT_PATH_SECONDARY_MS_THRESHOLD:
        return "retained_parent_checkpoint_path_time_secondary"
    return "cross_stage_retained_parent_checkpoint_prefix_suffix_gather_candidate"


def performance_focus_hint(
    trace_pipeline_hint: str,
    retained_parent_checkpoint_action_hint: str,
) -> str:
    trace_pipeline_hints = {
        "trace_generation_and_commit_pipeline_candidate",
        "parallel_segment_reexecution_candidate",
        "parallel_segment_reexecution_authorization_required",
        "parallel_trace_lowering_candidate",
        "trace_generation_parallelism_candidate",
        "commit_trace_overlap_candidate",
        "segment_commit_candidate",
        "trace_queue_backpressure_candidate",
    }
    if (
        trace_pipeline_hint in trace_pipeline_hints
        and retained_parent_checkpoint_action_hint
        == "retained_parent_checkpoint_path_time_secondary"
    ):
        return "trace_pipeline_over_secondary_opening_launches"
    if trace_pipeline_hint in trace_pipeline_hints:
        return trace_pipeline_hint
    if (
        trace_pipeline_hint == "within_target"
        and retained_parent_checkpoint_action_hint
        == "retained_parent_checkpoint_path_time_secondary"
    ):
        return "none"
    if retained_parent_checkpoint_action_hint != "none":
        return retained_parent_checkpoint_action_hint
    if trace_pipeline_hint not in {"none", "unknown", "within_target", "balanced_pipeline"}:
        return trace_pipeline_hint
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


def opening_source_rebuild_hint(
    external_source_count: int,
    retained_source_count: int,
    source_retention_attempts: int,
    source_retention_retained: int,
    source_retention_rejected: int,
    source_retention_limit_bytes: int,
) -> str:
    if external_source_count <= 0:
        return "none"
    if (
        source_retention_attempts > 0
        and source_retention_retained == 0
        and source_retention_limit_bytes == 0
    ):
        return "retained_source_disabled_external_rebuild"
    if (
        source_retention_attempts > 0
        and source_retention_retained == 0
        and source_retention_rejected > 0
    ):
        return "retained_source_budget_rejected_external_rebuild"
    if 0 < source_retention_retained < source_retention_attempts:
        return "partial_retained_source_external_rebuild"
    if retained_source_count > 0 and external_source_count > 0:
        return "mixed_retained_and_external_sources"
    return "external_source_rebuild"


def data_residency_action_hint(
    source_rebuild_hint: str,
    cuda_transfer_hint: str,
    source_retention_rejected_bytes: int,
    segment_commit_cuda_memory_total_bytes: int,
    trace_descriptor_residency_pipeline: bool,
) -> str:
    if (
        source_rebuild_hint
        in {
            "retained_source_disabled_external_rebuild",
            "retained_source_budget_rejected_external_rebuild",
        }
        and cuda_transfer_hint == "reduce_bulk_h2d_source_uploads"
        and segment_commit_cuda_memory_total_bytes > 0
        and source_retention_rejected_bytes > segment_commit_cuda_memory_total_bytes
    ):
        return "source_residency_requires_chunked_design"
    if (
        source_rebuild_hint == "retained_source_disabled_external_rebuild"
        and cuda_transfer_hint == "reduce_bulk_h2d_source_uploads"
    ):
        return "source_retention_disabled_bulk_h2d_rebuild"
    if (
        source_rebuild_hint == "retained_source_budget_rejected_external_rebuild"
        and cuda_transfer_hint == "reduce_bulk_h2d_source_uploads"
    ):
        return "reduce_source_retention_footprint_for_bulk_h2d"
    if (
        source_rebuild_hint == "partial_retained_source_external_rebuild"
        and cuda_transfer_hint == "reduce_bulk_h2d_source_uploads"
    ):
        return "increase_source_residency_coverage"
    if trace_descriptor_residency_pipeline:
        return "trace_descriptor_residency_pipeline"
    return "none"


def kernel_stream_idle_boundary_hint(
    previous_kernel: str,
    next_kernel: str,
    gap_calls: str,
    root_count: int,
) -> str:
    try:
        calls = int(gap_calls)
    except ValueError:
        return "none"
    if calls <= 0:
        return "none"
    if root_count > 0 and calls != root_count:
        return "none"

    previous = previous_kernel.lower()
    next_name = next_kernel.lower()
    if (
        "merkle_digest_parent" in previous
        and "trace_descriptor" in next_name
    ):
        return "commit_root_to_trace_descriptor_idle"
    return "none"


def source_retention_exceeds_device_memory_hint(
    byte_count: int,
    device_memory_bytes: int,
    needs_byte_evidence: bool,
) -> str:
    if device_memory_bytes <= 0:
        return "unknown"
    if byte_count <= 0:
        return "unknown" if needs_byte_evidence else "no"
    return "yes" if byte_count > device_memory_bytes else "no"


def opening_source_row_value_action_hint(
    total_ms: int,
    source_extend_ms: int,
    source_rows: int,
    external_source_count: int,
    query_units: int,
    single_query_units: int,
    trace_pipeline_hint: str,
) -> str:
    if source_rows <= 0 or source_extend_ms <= 0:
        return "none"

    source_extend_pct = source_extend_ms * 100.0 / total_ms if total_ms else 0.0
    if (
        trace_pipeline_hint == "trace_generation_and_commit_pipeline_candidate"
        and source_extend_pct < SOURCE_ROW_VALUE_SECONDARY_PCT_THRESHOLD
    ):
        return "trace_pipeline_before_source_row_values"
    if external_source_count > 0 and query_units > 0 and single_query_units >= query_units:
        return "profile_external_source_row_value_rebuilds"
    if source_extend_ms >= 1000:
        return "reduce_source_row_value_extension"
    return "source_row_values_secondary"


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
    buffer_capacity_present: bool,
) -> str:
    if reports <= 0 and report_rows <= 0 and buffer_capacity <= 0:
        return "none"
    if not buffer_capacity_present:
        return "report_buffer_capacity_missing"
    if reports > 0 and buffer_capacity <= 0:
        return "report_buffer_elided"
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
    buffer_capacity: int,
    buffer_capacity_present: bool,
    buffer_excess_pct: float,
    pending_drop_pct: float,
    lowerer_ms: int,
    stream_elapsed_ms: int,
) -> str:
    if reports <= 0:
        return "none"
    if buffer_capacity_present and buffer_capacity <= 0:
        if lowerer_ms <= 0 and stream_elapsed_ms > 0:
            return "report_buffer_elided_but_trace_serialized"
        return "report_buffer_elided"
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


def trace_report_detail_action_hint(
    hotspot_name: str,
    hotspot_pct: float,
    row_validation_residual_pct: float,
    source_values_residual_pct: float,
    visit_descriptor_pct: float,
) -> str:
    if hotspot_name == "none" or hotspot_pct <= 0.0:
        return "none"
    if hotspot_name == "row_validation":
        if row_validation_residual_pct >= 50.0:
            return "profile_row_validation_residual"
        return "profile_row_validation"
    if hotspot_name == "source_values":
        if source_values_residual_pct >= 50.0:
            return "profile_source_values_residual"
        return "profile_source_values"
    if hotspot_name == "visit":
        if visit_descriptor_pct >= 50.0:
            return "profile_descriptor_write"
        return "profile_visit"
    if hotspot_name == "descriptor":
        return "profile_descriptor_write"
    return f"profile_{hotspot_name}"


def trace_shape_sample_hint(values: dict[str, int], rows: int) -> str:
    if rows <= 0:
        return "none"
    if any(values.get(key, 0) > 0 for key in TRACE_SHAPE_KEYS):
        return "shape_timing_enabled"
    if (
        values.get(TRACE_REPORT_DETAIL_SAMPLES_KEY, 0) > 0
        and values.get(TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY, 0) > 0
    ):
        return "shape_timing_missing_for_detail_profile"
    return "shape_timing_disabled_or_zero"


def trace_shape_row_mix_hint(
    trace_shape_hint: str,
    external_op_row_pct: float,
    copy_row_pct: float,
    indirect_memory_row_pct: float,
) -> str:
    if trace_shape_hint != "shape_timing_enabled":
        return "none"
    if copy_row_pct >= 45.0 and external_op_row_pct >= 40.0:
        return "copy_and_external_op_rows_dominate"
    if external_op_row_pct >= 45.0:
        return "external_op_rows_dominate"
    if copy_row_pct >= 45.0:
        return "copy_rows_dominate"
    if indirect_memory_row_pct >= 40.0:
        return "indirect_memory_rows_dominate"
    return "mixed_trace_rows"


def trace_copy_shape_hint(
    copy_rows: int,
    copy_memory_source_row_pct: float,
    copy_indirect_memory_row_pct: float,
    copy_no_memory_row_pct: float,
) -> str:
    if copy_rows <= 0:
        return "none"
    if copy_no_memory_row_pct >= 50.0:
        return "copy_no_memory_fast_path_candidate"
    if copy_memory_source_row_pct >= 50.0:
        return "copy_memory_source_dominant"
    if copy_indirect_memory_row_pct >= 50.0:
        return "copy_indirect_memory_dominant"
    return "copy_shape_mixed"


def trace_shape_duration_hint(
    external_op_row_lower_pct: float,
    copy_row_lower_pct: float,
) -> str:
    if copy_row_lower_pct >= 45.0 and external_op_row_lower_pct >= 40.0:
        return "copy_and_external_op_duration_dominate"
    if external_op_row_lower_pct >= 45.0:
        return "external_op_duration_dominates"
    if copy_row_lower_pct >= 45.0:
        return "copy_duration_dominates"
    if external_op_row_lower_pct > 0.0 or copy_row_lower_pct > 0.0:
        return "mixed_trace_shape_duration"
    return "none"


def trace_shape_unit_cost_hint(
    external_op_row_lower_ns_per_row: float,
    copy_row_lower_ns_per_row: float,
    trace_shape_row_mix: str,
) -> str:
    if (
        external_op_row_lower_ns_per_row <= 0.0
        and copy_row_lower_ns_per_row <= 0.0
    ):
        return "none"
    if external_op_row_lower_ns_per_row <= 0.0 or copy_row_lower_ns_per_row <= 0.0:
        return "single_shape_unit_cost_sampled"
    if external_op_row_lower_ns_per_row >= copy_row_lower_ns_per_row * 1.20:
        return "external_op_unit_cost_higher"
    if copy_row_lower_ns_per_row >= external_op_row_lower_ns_per_row * 1.20:
        return "copy_unit_cost_higher"
    if trace_shape_row_mix == "copy_and_external_op_rows_dominate":
        return "row_volume_dominates_shape_duration"
    return "balanced_shape_unit_cost"


def trace_shape_run_hint(
    external_op_avg_run: float,
    external_op_max_run: int,
    copy_avg_run: float,
    copy_max_run: int,
) -> str:
    if (
        external_op_avg_run <= 0.0
        and external_op_max_run <= 0
        and copy_avg_run <= 0.0
        and copy_max_run <= 0
    ):
        return "none"
    if external_op_avg_run >= 8.0:
        return "external_op_runs_long"
    if copy_avg_run >= 8.0:
        return "copy_runs_long"
    if external_op_max_run >= 32 or copy_max_run >= 32:
        return "shape_runs_spiky"
    return "shape_runs_short"


def trace_shape_profile_hint(trace_shape_hint: str) -> str:
    if trace_shape_hint == "shape_timing_enabled":
        return "diagnostic_only_shape_profile"
    return "none"


def is_diagnostic_shape_profile(values: dict[str, int]) -> bool:
    trace_report_rows = values.get(TRACE_REPORT_ROWS_KEY, 0)
    trace_shape_hint = trace_shape_sample_hint(values, trace_report_rows)
    return trace_shape_profile_hint(trace_shape_hint) == "diagnostic_only_shape_profile"


def avg_run_length(rows: int, runs: int) -> float:
    if rows <= 0 or runs <= 0:
        return 0.0
    return rows / runs


def ns_per_row_from_ms(duration_ms: int, row_count: int) -> float:
    if duration_ms <= 0 or row_count <= 0:
        return 0.0
    return duration_ms * 1_000_000.0 / row_count


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

EXACT_REPORT_HOTSPOT_KEYS = [
    ("row_validation", TRACE_REPORT_ROW_VALIDATION_MS_KEY),
    ("lowering", TRACE_REPORT_LOWERING_MS_KEY),
    ("visit", TRACE_REPORT_VISIT_MS_KEY),
    ("descriptor", TRACE_DESCRIPTOR_MS_KEY),
    ("source_values", TRACE_REPORT_SOURCE_VALUES_MS_KEY),
    ("source_a_value", TRACE_REPORT_SOURCE_A_VALUE_MS_KEY),
    ("source_b_value", TRACE_REPORT_SOURCE_B_VALUE_MS_KEY),
    ("instruction_result", TRACE_REPORT_INSTRUCTION_RESULT_MS_KEY),
    ("next_pc", TRACE_REPORT_NEXT_PC_MS_KEY),
    ("register_access", TRACE_REPORT_REGISTER_ACCESS_MS_KEY),
    ("memory_access", TRACE_REPORT_MEMORY_ACCESS_MS_KEY),
    ("store_apply", TRACE_REPORT_STORE_APPLY_MS_KEY),
    ("precompile_memory", TRACE_REPORT_PRECOMPILE_MEMORY_MS_KEY),
    ("memory_columns", TRACE_REPORT_MEMORY_COLUMNS_MS_KEY),
]

SOURCE_VALUE_DETAIL_HOTSPOT_KEYS = [
    ("source_a_value", TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY),
    ("source_b_value", TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY),
]

SOURCE_VALUE_KIND_DETAIL_KEYS = [
    (
        "immediate_read",
        TRACE_REPORT_SOURCE_IMMEDIATE_READS_KEY,
        TRACE_REPORT_SOURCE_IMMEDIATE_READ_SAMPLED_NS_KEY,
    ),
    (
        "register_read",
        TRACE_REPORT_SOURCE_REGISTER_READS_KEY,
        TRACE_REPORT_SOURCE_REGISTER_READ_SAMPLED_NS_KEY,
    ),
    (
        "memory_read",
        TRACE_REPORT_SOURCE_MEMORY_READS_KEY,
        TRACE_REPORT_SOURCE_MEMORY_READ_SAMPLED_NS_KEY,
    ),
    (
        "indirect_read",
        TRACE_REPORT_SOURCE_INDIRECT_READS_KEY,
        TRACE_REPORT_SOURCE_INDIRECT_READ_SAMPLED_NS_KEY,
    ),
    (
        "last_c_read",
        TRACE_REPORT_SOURCE_LAST_C_READS_KEY,
        TRACE_REPORT_SOURCE_LAST_C_READ_SAMPLED_NS_KEY,
    ),
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


def trace_report_exact_action_hint(
    hotspot_name: str, hotspot_pct: float, has_sampled_detail: bool
) -> str:
    if hotspot_name == "none" or hotspot_pct <= 0.0:
        if has_sampled_detail:
            return "use_sampled_detail_breakdown"
        return "enable_detail_timing_for_report_breakdown"
    if hotspot_name == "row_validation":
        return "profile_row_validation"
    if hotspot_name == "source_values":
        return "profile_source_values"
    if hotspot_name == "visit":
        return "profile_visit"
    if hotspot_name == "descriptor":
        return "profile_descriptor_write"
    return f"profile_{hotspot_name}"


def trace_report_exact_hotspot(values: dict[str, int]) -> tuple[str, float, str]:
    report_ms = values.get(TRACE_REPORT_MS_KEY, 0)
    if report_ms <= 0:
        return ("none", 0.0, "none")
    hotspot_name = "none"
    hotspot_ms = 0
    for name, key in EXACT_REPORT_HOTSPOT_KEYS:
        value = values.get(key, 0)
        if value > hotspot_ms:
            hotspot_name = name
            hotspot_ms = value
    hotspot_pct = hotspot_ms * 100.0 / report_ms if hotspot_ms else 0.0
    has_sampled_detail = values.get(TRACE_REPORT_DETAIL_SAMPLES_KEY, 0) > 0
    return (
        hotspot_name,
        hotspot_pct,
        trace_report_exact_action_hint(hotspot_name, hotspot_pct, has_sampled_detail),
    )


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


def source_value_kind_counts(values: dict[str, int]) -> tuple[int, dict[str, int]]:
    counts = {
        name: values.get(count_key, 0)
        for name, count_key, _sample_key in SOURCE_VALUE_KIND_DETAIL_KEYS
    }
    return (sum(counts.values()), counts)


def source_value_kind_duration_hotspot(
    values: dict[str, int],
) -> tuple[str, float, float, float]:
    source_values_ns = values.get(TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY, 0)
    if source_values_ns <= 0:
        return ("none", 0.0, 0.0, 0.0)
    hotspot_name = "none"
    hotspot_ns = 0
    kind_ns = 0
    for name, _count_key, sample_key in SOURCE_VALUE_KIND_DETAIL_KEYS:
        value = values.get(sample_key, 0)
        kind_ns += value
        if value > hotspot_ns:
            hotspot_name = name
            hotspot_ns = value
    explained_ns = min(kind_ns, source_values_ns)
    residual_ns = max(source_values_ns - kind_ns, 0)
    return (
        hotspot_name,
        hotspot_ns * 100.0 / source_values_ns if hotspot_ns else 0.0,
        explained_ns * 100.0 / source_values_ns,
        residual_ns * 100.0 / source_values_ns,
    )


def trace_report_detail_lowerer_share_ms(
    values: dict[str, int],
    sample_key: str,
    lowerer_ms: int,
) -> float:
    return trace_report_sampled_ns_lowerer_share_ms(
        values,
        values.get(sample_key, 0),
        lowerer_ms,
    )


def trace_report_sampled_ns_lowerer_share_ms(
    values: dict[str, int],
    child_ns: int,
    lowerer_ms: int,
) -> float:
    sampled_ns = values.get(TRACE_REPORT_SAMPLED_NS_KEY, 0)
    if sampled_ns <= 0 or lowerer_ms <= 0:
        return 0.0
    return child_ns * lowerer_ms / sampled_ns


def trace_report_source_lookup_sampled_ns(values: dict[str, int]) -> int:
    return values.get(TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY, 0) + values.get(
        TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY, 0
    )


def trace_report_source_values_residual_sampled_ns(values: dict[str, int]) -> int:
    return max(
        values.get(TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY, 0)
        - trace_report_source_lookup_sampled_ns(values),
        0,
    )


def trace_report_row_validation_residual_sampled_ns(values: dict[str, int]) -> int:
    row_validation_ns = values.get(TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY, 0)
    child_ns = sum(values.get(key, 0) for _, key in row_validation_hotspot_keys(values))
    return max(row_validation_ns - child_ns, 0)


def seed_direct_lift_dominant_miss_reason(
    empty_segments: int,
    pending_dma_single_reports: int,
    amo_boundaries: int,
    store_conditional_boundaries: int,
    dma_prepare_missing_lookaheads: int,
    boundary_c_unavailable: int,
) -> str:
    reasons = [
        ("empty_segments", empty_segments),
        ("pending_dma_single_reports", pending_dma_single_reports),
        ("amo_boundaries", amo_boundaries),
        ("store_conditional_boundaries", store_conditional_boundaries),
        ("dma_prepare_missing_lookaheads", dma_prepare_missing_lookaheads),
        ("boundary_c_unavailable", boundary_c_unavailable),
    ]
    reason, count = max(reasons, key=lambda item: item[1])
    if count <= 0:
        return "none"
    return reason


def seed_direct_lift_action_hint(attempts: int, successes: int, dominant_reason: str) -> str:
    if attempts <= 0:
        return "none"
    if successes >= attempts:
        return "seed_direct_lift_ready"
    if dominant_reason == "none":
        return "profile_seed_direct_lift_missing_breakdown"
    return f"profile_{dominant_reason}"


def trace_structure_hint(
    total_ms: int,
    runner_ms: int,
    lowerer_ms: int,
    stream_elapsed_ms: int,
    segment_receive_wait_ms: int,
    pending_receive_wait_ms: int,
    parallel_lower_workers: int,
    leaf_kernel_ms: int,
) -> str:
    trace_ms = max(runner_ms, lowerer_ms, stream_elapsed_ms)
    if total_ms <= 0 or trace_ms <= 0:
        return "none"
    receive_wait_ratio = (
        segment_receive_wait_ms / stream_elapsed_ms if stream_elapsed_ms else 0.0
    )
    pending_receive_wait_ratio = (
        pending_receive_wait_ms / stream_elapsed_ms if stream_elapsed_ms else 0.0
    )
    runner_stream_ratio = runner_ms / stream_elapsed_ms if stream_elapsed_ms else 0.0
    leaf_ratio = leaf_kernel_ms / trace_ms if trace_ms else 0.0
    trace_total_ratio = trace_ms / total_ms if total_ms else 0.0
    if parallel_lower_workers > 0:
        if pending_receive_wait_ratio >= 0.5 and runner_stream_ratio >= 0.75:
            return "parallel_lower_runner_bound"
        if receive_wait_ratio >= 0.5:
            return "parallel_lower_waiting"
        return "parallel_lower_active"
    if trace_total_ratio >= 0.6 and receive_wait_ratio >= 0.5 and leaf_ratio <= 0.2:
        return "trace_stream_cpu_floor"
    if trace_total_ratio >= 0.6:
        return "cpu_trace_dominant"
    return "none"


def trace_structure_hint_from_values(values: dict[str, int]) -> str:
    return trace_structure_hint(
        values.get(TOTAL_MS_KEY, 0),
        values.get(RUNNER_MS_KEY, 0),
        values.get(LOWERER_MS_KEY, 0),
        values.get(STREAM_ELAPSED_MS_KEY, 0),
        values.get(SEGMENT_RECEIVE_WAIT_MS_KEY, 0),
        values.get(PENDING_RECEIVE_WAIT_MS_KEY, 0),
        values.get(PARALLEL_LOWER_WORKERS_KEY, 0),
        values.get(LEAF_KERNEL_MS_KEY, 0),
    )


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
    trace_lower_ms = values.get(TRACE_LOWER_MS_KEY, 0)
    trace_report_ms = values.get(TRACE_REPORT_MS_KEY, 0)
    trace_non_report_ms = (
        max(trace_lower_ms - trace_report_ms, 0)
        if TRACE_REPORT_MS_KEY in values and trace_lower_ms > 0
        else trace_lower_ms
    )
    trace_runner_lowerer_overlap_ms = (
        max(runner_ms + lowerer_ms - stream_elapsed_ms, 0)
        if runner_ms > 0 and lowerer_ms > 0 and stream_elapsed_ms > 0
        else 0
    )
    trace_lowerer_non_lower_ms = (
        max(lowerer_ms - trace_lower_ms, 0)
        if TRACE_LOWER_MS_KEY in values and lowerer_ms > 0
        else 0
    )
    stream_worker_ms = values.get(STREAM_WORKER_MS_KEY, 0)
    segment_commit_ms = values.get(SEGMENT_COMMIT_MS_KEY, 0)
    segment_commit_attempt_ms = values.get(SEGMENT_COMMIT_ATTEMPT_MS_KEY, 0)
    segment_commit_oom_retry_ms = values.get(SEGMENT_COMMIT_OOM_RETRY_MS_KEY, 0)
    segment_commit_initial_workers = values.get(SEGMENT_COMMIT_INITIAL_WORKERS_KEY, 0)
    segment_commit_effective_workers = values.get(SEGMENT_COMMIT_EFFECTIVE_WORKERS_KEY, 0)
    segment_commit_worker_submits = values.get(SEGMENT_COMMIT_WORKER_SUBMITS_KEY, 0)
    segment_commit_worker_joins = values.get(SEGMENT_COMMIT_WORKER_JOINS_KEY, 0)
    segment_commit_worker_backpressure_joins = values.get(
        SEGMENT_COMMIT_WORKER_BACKPRESSURE_JOINS_KEY, 0
    )
    segment_commit_worker_backpressure_join_ms = values.get(
        SEGMENT_COMMIT_WORKER_BACKPRESSURE_JOIN_MS_KEY, 0
    )
    segment_commit_worker_finish_joins = values.get(
        SEGMENT_COMMIT_WORKER_FINISH_JOINS_KEY, 0
    )
    segment_commit_worker_finish_join_ms = values.get(
        SEGMENT_COMMIT_WORKER_FINISH_JOIN_MS_KEY, 0
    )
    segment_commit_worker_max_in_flight = values.get(
        SEGMENT_COMMIT_WORKER_MAX_IN_FLIGHT_KEY, 0
    )
    segment_commit_worker_hint = segment_commit_worker_pressure_hint(
        segment_commit_worker_submits,
        segment_commit_worker_backpressure_joins,
        segment_commit_worker_backpressure_join_ms,
        segment_commit_worker_finish_joins,
        segment_commit_worker_finish_join_ms,
        segment_commit_worker_max_in_flight,
        segment_commit_effective_workers,
    )
    segment_commit_oom_retries = values.get(SEGMENT_COMMIT_OOM_RETRIES_KEY, 0)
    segment_commit_cuda_memory_total_bytes = values.get(
        SEGMENT_COMMIT_CUDA_MEMORY_TOTAL_BYTES_KEY, 0
    )
    segment_commit_cuda_memory_initial_free_bytes = values.get(
        SEGMENT_COMMIT_CUDA_MEMORY_INITIAL_FREE_BYTES_KEY, 0
    )
    segment_commit_cuda_memory_effective_free_bytes = values.get(
        SEGMENT_COMMIT_CUDA_MEMORY_EFFECTIVE_FREE_BYTES_KEY, 0
    )
    segment_commit_cuda_memory_min_free_bytes = values.get(
        SEGMENT_COMMIT_CUDA_MEMORY_MIN_FREE_BYTES_KEY, 0
    )
    segment_commit_cuda_allocator_initial_cached_bytes = values.get(
        SEGMENT_COMMIT_CUDA_ALLOCATOR_INITIAL_CACHED_BYTES_KEY, 0
    )
    segment_commit_cuda_allocator_effective_cached_bytes = values.get(
        SEGMENT_COMMIT_CUDA_ALLOCATOR_EFFECTIVE_CACHED_BYTES_KEY, 0
    )
    segment_commit_cuda_memory_min_free_pct = (
        segment_commit_cuda_memory_min_free_bytes
        * 100.0
        / segment_commit_cuda_memory_total_bytes
        if segment_commit_cuda_memory_total_bytes > 0
        else 0.0
    )
    segment_commit_memory_hint = segment_commit_memory_pressure_hint_from_values(values)
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
    parallel_lower_snapshot_replay = values.get(PARALLEL_LOWER_SNAPSHOT_REPLAY_KEY, 0)
    parallel_lower_report_elided = values.get(PARALLEL_LOWER_REPORT_ELIDED_KEY, 0)
    parallel_lower_dispatch_wait_ms = values.get(PARALLEL_LOWER_DISPATCH_WAIT_MS_KEY, 0)
    parallel_lower_result_receive_wait_ms = values.get(
        PARALLEL_LOWER_RESULT_RECEIVE_WAIT_MS_KEY, 0
    )
    parallel_lower_dispatch_blocked = values.get(PARALLEL_LOWER_DISPATCH_BLOCKED_KEY, 0)
    segment_replay_count = values.get(SEGMENT_REPLAY_COUNT_KEY, 0)
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
    copy_memory_source_rows = values.get(TRACE_COPY_MEMORY_SOURCE_ROWS_KEY, 0)
    copy_indirect_memory_rows = values.get(TRACE_COPY_INDIRECT_MEMORY_ROWS_KEY, 0)
    copy_register_store_rows = values.get(TRACE_COPY_REGISTER_STORE_ROWS_KEY, 0)
    copy_memory_store_rows = values.get(TRACE_COPY_MEMORY_STORE_ROWS_KEY, 0)
    copy_no_store_rows = values.get(TRACE_COPY_NO_STORE_ROWS_KEY, 0)
    copy_no_memory_rows = values.get(TRACE_COPY_NO_MEMORY_ROWS_KEY, 0)
    external_op_runs = values.get(TRACE_EXTERNAL_OP_RUNS_KEY, 0)
    external_op_max_run = values.get(TRACE_EXTERNAL_OP_MAX_RUN_KEY, 0)
    copy_runs = values.get(TRACE_COPY_RUNS_KEY, 0)
    copy_max_run = values.get(TRACE_COPY_MAX_RUN_KEY, 0)
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
    copy_memory_source_row_pct = (
        copy_memory_source_rows * 100.0 / copy_rows if copy_rows else 0.0
    )
    copy_indirect_memory_row_pct = (
        copy_indirect_memory_rows * 100.0 / copy_rows if copy_rows else 0.0
    )
    copy_no_memory_row_pct = (
        copy_no_memory_rows * 100.0 / copy_rows if copy_rows else 0.0
    )
    copy_shape_hint = trace_copy_shape_hint(
        copy_rows,
        copy_memory_source_row_pct,
        copy_indirect_memory_row_pct,
        copy_no_memory_row_pct,
    )
    trace_shape_hint = trace_shape_sample_hint(values, trace_report_rows)
    external_op_row_pct = (
        external_op_rows * 100.0 / trace_report_rows if trace_report_rows else 0.0
    )
    copy_row_pct = copy_rows * 100.0 / trace_report_rows if trace_report_rows else 0.0
    trace_shape_row_mix = trace_shape_row_mix_hint(
        trace_shape_hint,
        external_op_row_pct,
        copy_row_pct,
        indirect_memory_row_pct,
    )
    external_op_row_lower_ms = values.get(TRACE_EXTERNAL_OP_ROW_LOWER_MS_KEY, 0)
    copy_row_lower_ms = values.get(TRACE_COPY_ROW_LOWER_MS_KEY, 0)
    external_op_row_lower_ns_per_row = ns_per_row_from_ms(
        external_op_row_lower_ms,
        external_op_rows,
    )
    copy_row_lower_ns_per_row = ns_per_row_from_ms(
        copy_row_lower_ms,
        copy_rows,
    )
    external_op_row_lower_pct = (
        external_op_row_lower_ms * 100.0 / trace_lower_ms if trace_lower_ms else 0.0
    )
    copy_row_lower_pct = (
        copy_row_lower_ms * 100.0 / trace_lower_ms if trace_lower_ms else 0.0
    )
    trace_shape_duration = trace_shape_duration_hint(
        external_op_row_lower_pct,
        copy_row_lower_pct,
    )
    trace_shape_unit_cost = trace_shape_unit_cost_hint(
        external_op_row_lower_ns_per_row,
        copy_row_lower_ns_per_row,
        trace_shape_row_mix,
    )
    external_op_avg_run = avg_run_length(external_op_rows, external_op_runs)
    copy_avg_run = avg_run_length(copy_rows, copy_runs)
    trace_shape_run = trace_shape_run_hint(
        external_op_avg_run,
        external_op_max_run,
        copy_avg_run,
        copy_max_run,
    )
    trace_shape_profile = trace_shape_profile_hint(trace_shape_hint)
    trace_report_validation_ms = values.get(TRACE_REPORT_VALIDATION_MS_KEY, 0)
    trace_report_emit_ms = values.get(TRACE_REPORT_EMIT_MS_KEY, 0)
    trace_descriptor_ms = values.get(TRACE_DESCRIPTOR_MS_KEY, 0)
    trace_report_lowering_ms = values.get(TRACE_REPORT_LOWERING_MS_KEY, 0)
    trace_report_row_validation_ms = values.get(TRACE_REPORT_ROW_VALIDATION_MS_KEY, 0)
    trace_report_memory_columns_ms = values.get(TRACE_REPORT_MEMORY_COLUMNS_MS_KEY, 0)
    trace_report_source_values_ms = values.get(TRACE_REPORT_SOURCE_VALUES_MS_KEY, 0)
    trace_report_source_a_value_ms = values.get(TRACE_REPORT_SOURCE_A_VALUE_MS_KEY, 0)
    trace_report_source_b_value_ms = values.get(TRACE_REPORT_SOURCE_B_VALUE_MS_KEY, 0)
    trace_report_precompile_memory_ms = values.get(
        TRACE_REPORT_PRECOMPILE_MEMORY_MS_KEY, 0
    )
    trace_report_instruction_result_ms = values.get(
        TRACE_REPORT_INSTRUCTION_RESULT_MS_KEY, 0
    )
    trace_report_next_pc_ms = values.get(TRACE_REPORT_NEXT_PC_MS_KEY, 0)
    trace_report_register_access_ms = values.get(
        TRACE_REPORT_REGISTER_ACCESS_MS_KEY, 0
    )
    trace_report_memory_access_ms = values.get(TRACE_REPORT_MEMORY_ACCESS_MS_KEY, 0)
    trace_report_store_apply_ms = values.get(TRACE_REPORT_STORE_APPLY_MS_KEY, 0)
    trace_report_visit_ms = values.get(TRACE_REPORT_VISIT_MS_KEY, 0)
    (
        trace_report_exact_hotspot_name,
        trace_report_exact_hotspot_pct,
        trace_report_exact_action,
    ) = trace_report_exact_hotspot(values)
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
    trace_lowerer_share_scale_ms = (
        trace_lower_ms
        if TRACE_LOWER_MS_KEY in values and trace_lower_ms > 0
        else lowerer_ms
    )
    trace_report_detail_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_row_validation_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_memory_columns_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_MEMORY_COLUMNS_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_source_values_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_source_lookup_share_ms = trace_report_sampled_ns_lowerer_share_ms(
        values,
        trace_report_source_lookup_sampled_ns(values),
        trace_lowerer_share_scale_ms,
    )
    trace_report_source_values_residual_share_ms = (
        trace_report_sampled_ns_lowerer_share_ms(
            values,
            trace_report_source_values_residual_sampled_ns(values),
            trace_lowerer_share_scale_ms,
        )
    )
    trace_report_precompile_memory_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_PRECOMPILE_MEMORY_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_instruction_result_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_INSTRUCTION_RESULT_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_next_pc_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_NEXT_PC_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_register_access_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_REGISTER_ACCESS_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_memory_access_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_MEMORY_ACCESS_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_store_apply_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_STORE_APPLY_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_row_validation_residual_share_ms = (
        trace_report_sampled_ns_lowerer_share_ms(
            values,
            trace_report_row_validation_residual_sampled_ns(values),
            trace_lowerer_share_scale_ms,
        )
    )
    trace_report_visit_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_REPORT_VISIT_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_descriptor_share_ms = trace_report_detail_lowerer_share_ms(
        values,
        TRACE_DESCRIPTOR_SAMPLED_NS_KEY,
        trace_lowerer_share_scale_ms,
    )
    trace_report_source_values_residual_ns_per_row = ns_per_row_from_ms(
        trace_report_source_values_residual_share_ms,
        trace_report_rows,
    )
    trace_report_row_validation_residual_ns_per_row = ns_per_row_from_ms(
        trace_report_row_validation_residual_share_ms,
        trace_report_rows,
    )
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
    source_kind_read_total, source_kind_counts = source_value_kind_counts(values)
    source_immediate_reads = source_kind_counts["immediate_read"]
    source_register_reads = source_kind_counts["register_read"]
    source_memory_reads = source_kind_counts["memory_read"]
    source_indirect_reads = source_kind_counts["indirect_read"]
    source_last_c_reads = source_kind_counts["last_c_read"]
    source_immediate_read_pct = (
        source_immediate_reads * 100.0 / source_kind_read_total
        if source_kind_read_total
        else 0.0
    )
    source_register_read_pct = (
        source_register_reads * 100.0 / source_kind_read_total
        if source_kind_read_total
        else 0.0
    )
    source_memory_read_pct = (
        source_memory_reads * 100.0 / source_kind_read_total
        if source_kind_read_total
        else 0.0
    )
    source_indirect_read_pct = (
        source_indirect_reads * 100.0 / source_kind_read_total
        if source_kind_read_total
        else 0.0
    )
    source_last_c_read_pct = (
        source_last_c_reads * 100.0 / source_kind_read_total
        if source_kind_read_total
        else 0.0
    )
    (
        trace_report_source_kind_hotspot,
        trace_report_source_kind_hotspot_pct,
        trace_report_source_kind_coverage_pct,
        trace_report_source_kind_residual_pct,
    ) = source_value_kind_duration_hotspot(values)
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
    trace_report_detail_action = trace_report_detail_action_hint(
        trace_report_detail_hotspot_name,
        trace_report_detail_hotspot_pct,
        trace_report_row_validation_residual_pct,
        trace_report_source_values_residual_pct,
        trace_report_visit_descriptor_pct,
    )
    trace_report_visit_residual_pct = (
        max(trace_report_visit_sampled_ns - trace_descriptor_sampled_ns, 0)
        * 100.0
        / trace_report_visit_sampled_ns
        if trace_report_visit_sampled_ns
        else 0.0
    )
    trace_report_visit_residual_share_ms = trace_report_sampled_ns_lowerer_share_ms(
        values,
        max(trace_report_visit_sampled_ns - trace_descriptor_sampled_ns, 0),
        trace_lowerer_share_scale_ms,
    )
    trace_report_visit_residual_ns_per_row = ns_per_row_from_ms(
        trace_report_visit_residual_share_ms,
        trace_report_rows,
    )
    trace_report_descriptor_ns_per_row = ns_per_row_from_ms(
        trace_report_descriptor_share_ms,
        trace_report_rows,
    )
    trace_rows_per_report = (
        trace_report_rows / trace_reports if trace_reports else 0.0
    )
    trace_report_record_size_bytes = values.get(TRACE_REPORT_RECORD_SIZE_BYTES_KEY, 0)
    trace_report_instruction_size_bytes = values.get(
        TRACE_REPORT_INSTRUCTION_SIZE_BYTES_KEY, 0
    )
    trace_report_register_write_list_size_bytes = values.get(
        TRACE_REPORT_REGISTER_WRITE_LIST_SIZE_BYTES_KEY, 0
    )
    trace_report_memory_access_list_size_bytes = values.get(
        TRACE_REPORT_MEMORY_ACCESS_LIST_SIZE_BYTES_KEY, 0
    )
    trace_report_precompile_access_list_size_bytes = values.get(
        TRACE_REPORT_PRECOMPILE_ACCESS_LIST_SIZE_BYTES_KEY, 0
    )
    trace_report_instruction_storage_gib = (
        trace_reports * trace_report_instruction_size_bytes / (1024.0**3)
    )
    trace_report_register_write_list_storage_gib = (
        trace_reports * trace_report_register_write_list_size_bytes / (1024.0**3)
    )
    trace_report_memory_access_list_storage_gib = (
        trace_reports * trace_report_memory_access_list_size_bytes / (1024.0**3)
    )
    trace_report_precompile_access_list_storage_gib = (
        trace_reports * trace_report_precompile_access_list_size_bytes / (1024.0**3)
    )
    trace_report_storage_bytes = values.get(
        TRACE_REPORT_STORAGE_BYTES_KEY,
        trace_reports * trace_report_record_size_bytes,
    )
    trace_report_buffer_capacity = values.get(TRACE_REPORT_BUFFER_CAPACITY_KEY, 0)
    trace_report_buffer_capacity_present = TRACE_REPORT_BUFFER_CAPACITY_KEY in values
    trace_report_buffer_max_capacity = values.get(
        TRACE_REPORT_BUFFER_MAX_CAPACITY_KEY, 0
    )
    trace_report_buffer_excess_capacity = values.get(
        TRACE_REPORT_BUFFER_EXCESS_CAPACITY_KEY, 0
    )
    trace_report_buffer_capacity_bytes = values.get(
        TRACE_REPORT_BUFFER_CAPACITY_BYTES_KEY,
        trace_report_buffer_capacity * trace_report_record_size_bytes,
    )
    trace_report_buffer_excess_bytes = values.get(
        TRACE_REPORT_BUFFER_EXCESS_BYTES_KEY,
        trace_report_buffer_excess_capacity * trace_report_record_size_bytes,
    )
    trace_runner_report_buffer_capacity = values.get(
        TRACE_RUNNER_REPORT_BUFFER_CAPACITY_KEY, 0
    )
    trace_runner_report_buffer_capacity_present = (
        TRACE_RUNNER_REPORT_BUFFER_CAPACITY_KEY in values
    )
    trace_runner_report_buffer_max_capacity = values.get(
        TRACE_RUNNER_REPORT_BUFFER_MAX_CAPACITY_KEY, 0
    )
    trace_runner_report_buffer_excess_capacity = values.get(
        TRACE_RUNNER_REPORT_BUFFER_EXCESS_CAPACITY_KEY, 0
    )
    trace_runner_report_buffer_capacity_bytes = values.get(
        TRACE_RUNNER_REPORT_BUFFER_CAPACITY_BYTES_KEY,
        trace_runner_report_buffer_capacity * trace_report_record_size_bytes,
    )
    trace_runner_report_buffer_excess_bytes = values.get(
        TRACE_RUNNER_REPORT_BUFFER_EXCESS_BYTES_KEY,
        trace_runner_report_buffer_excess_capacity * trace_report_record_size_bytes,
    )
    trace_report_storage_gib = trace_report_storage_bytes / (1024.0**3)
    trace_report_buffer_capacity_gib = trace_report_buffer_capacity_bytes / (
        1024.0**3
    )
    trace_runner_report_buffer_capacity_gib = (
        trace_runner_report_buffer_capacity_bytes / (1024.0**3)
    )
    trace_report_chunk_sent = values.get(TRACE_REPORT_CHUNK_SENT_KEY, 0)
    trace_report_chunk_received = values.get(TRACE_REPORT_CHUNK_RECEIVED_KEY, 0)
    trace_report_chunk_reports = values.get(TRACE_REPORT_CHUNK_REPORTS_KEY, 0)
    trace_report_chunk_rows = values.get(TRACE_REPORT_CHUNK_ROWS_KEY, 0)
    trace_report_chunk_max_queued = values.get(TRACE_REPORT_CHUNK_MAX_QUEUED_KEY, 0)
    trace_report_buffer_excess_pct = (
        trace_report_buffer_excess_capacity * 100.0 / trace_report_buffer_capacity
        if trace_report_buffer_capacity
        else 0.0
    )
    trace_runner_report_buffer_excess_pct = (
        trace_runner_report_buffer_excess_capacity
        * 100.0
        / trace_runner_report_buffer_capacity
        if trace_runner_report_buffer_capacity
        else 0.0
    )
    trace_report_buffer_hint = trace_report_buffer_shape_hint(
        trace_reports,
        trace_report_rows,
        trace_report_buffer_capacity,
        trace_report_buffer_excess_capacity,
        trace_report_buffer_capacity_present,
    )
    trace_runner_report_buffer_hint = trace_report_buffer_shape_hint(
        trace_reports,
        trace_report_rows,
        trace_runner_report_buffer_capacity,
        trace_runner_report_buffer_excess_capacity,
        trace_runner_report_buffer_capacity_present,
    )
    if trace_runner_report_buffer_hint.startswith("report_buffer_"):
        trace_runner_report_buffer_hint = trace_runner_report_buffer_hint.replace(
            "report_buffer_", "runner_report_buffer_", 1
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
    descriptor_high32_a_values = values.get(DESCRIPTOR_HIGH32_A_VALUES_KEY, 0)
    descriptor_high32_b_values = values.get(DESCRIPTOR_HIGH32_B_VALUES_KEY, 0)
    descriptor_high32_c_values = values.get(DESCRIPTOR_HIGH32_C_VALUES_KEY, 0)
    descriptor_high32_a_payload_values = values.get(
        DESCRIPTOR_HIGH32_A_PAYLOAD_VALUES_KEY, 0
    )
    descriptor_high32_b_payload_values = values.get(
        DESCRIPTOR_HIGH32_B_PAYLOAD_VALUES_KEY, 0
    )
    descriptor_high32_store_payload_values = values.get(
        DESCRIPTOR_HIGH32_STORE_PAYLOAD_VALUES_KEY, 0
    )
    descriptor_high32_store_prev_value_values = values.get(
        DESCRIPTOR_HIGH32_STORE_PREV_VALUE_VALUES_KEY, 0
    )
    descriptor_high32_row_field_histogram = [
        values.get(key, 0) for key in DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS
    ]
    (
        sparse_high32_estimated_upload_bytes,
        sparse_high32_estimated_upload_savings_pct,
        sparse_high32_high_words,
        sparse_high32_shape_hint,
    ) = sparse_high32_descriptor_estimate(
        descriptor_rows,
        descriptor_upload_bytes,
        descriptor_high32_values,
        descriptor_high32_rows_present,
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
    seed_direct_lift_empty_segments = values.get(
        SEED_DIRECT_LIFT_EMPTY_SEGMENTS_KEY, 0
    )
    seed_direct_lift_pending_dma_single_reports = values.get(
        SEED_DIRECT_LIFT_PENDING_DMA_SINGLE_REPORTS_KEY, 0
    )
    seed_direct_lift_amo_boundaries = values.get(
        SEED_DIRECT_LIFT_AMO_BOUNDARIES_KEY, 0
    )
    seed_direct_lift_store_conditional_boundaries = values.get(
        SEED_DIRECT_LIFT_STORE_CONDITIONAL_BOUNDARIES_KEY, 0
    )
    seed_direct_lift_dma_prepare_missing_lookaheads = values.get(
        SEED_DIRECT_LIFT_DMA_PREPARE_MISSING_LOOKAHEADS_KEY, 0
    )
    seed_direct_lift_boundary_c_unavailable = values.get(
        SEED_DIRECT_LIFT_BOUNDARY_C_UNAVAILABLE_KEY, 0
    )
    seed_direct_lift_success_pct = (
        seed_direct_lift_successes * 100.0 / seed_direct_lift_attempts
        if seed_direct_lift_attempts
        else 0.0
    )
    seed_direct_lift_dominant_miss = seed_direct_lift_dominant_miss_reason(
        seed_direct_lift_empty_segments,
        seed_direct_lift_pending_dma_single_reports,
        seed_direct_lift_amo_boundaries,
        seed_direct_lift_store_conditional_boundaries,
        seed_direct_lift_dma_prepare_missing_lookaheads,
        seed_direct_lift_boundary_c_unavailable,
    )
    seed_direct_lift_action = seed_direct_lift_action_hint(
        seed_direct_lift_attempts,
        seed_direct_lift_successes,
        seed_direct_lift_dominant_miss,
    )
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
    source_retention_attempts = values.get(SOURCE_RETENTION_ATTEMPTS_KEY, 0)
    source_retention_retained = values.get(SOURCE_RETENTION_RETAINED_KEY, 0)
    source_retention_rejected = values.get(SOURCE_RETENTION_REJECTED_KEY, 0)
    source_retention_retained_bytes = values.get(SOURCE_RETENTION_RETAINED_BYTES_KEY, 0)
    source_retention_rejected_bytes = values.get(SOURCE_RETENTION_REJECTED_BYTES_KEY, 0)
    source_retention_max_retained_bytes = values.get(
        SOURCE_RETENTION_MAX_RETAINED_BYTES_KEY, 0
    )
    source_retention_max_rejected_bytes = values.get(
        SOURCE_RETENTION_MAX_REJECTED_BYTES_KEY, 0
    )
    source_retention_limit_bytes = values.get(SOURCE_RETENTION_LIMIT_BYTES_KEY, 0)
    opening_row_value_device_rows = values.get(OPENING_ROW_VALUE_DEVICE_ROWS_KEY, 0)
    opening_row_value_source_rows = values.get(OPENING_ROW_VALUE_SOURCE_ROWS_KEY, 0)
    opening_row_value_source_extend_ms = values.get(
        OPENING_ROW_VALUE_SOURCE_EXTEND_MS_KEY, 0
    )
    opening_row_value_source_extend_pct = (
        opening_row_value_source_extend_ms * 100.0 / total_ms if total_ms else 0.0
    )
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
    retained_parent_checkpoint_prefix_ms = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_PREFIX_MS_KEY, 0
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
    retained_parent_checkpoint_suffix_ms = values.get(
        OPENING_RETAINED_PARENT_CHECKPOINT_SUFFIX_MS_KEY, 0
    )
    retained_parent_checkpoint_path_ms = (
        retained_parent_checkpoint_prefix_ms + retained_parent_checkpoint_suffix_ms
    )
    (
        retained_parent_checkpoint_path_launches,
        retained_parent_checkpoint_cross_stage_gather_estimated_launches,
        retained_parent_checkpoint_cross_stage_gather_launch_savings,
    ) = retained_parent_checkpoint_cross_stage_gather_launch_shape(
        retained_parent_checkpoint_openings,
        retained_parent_checkpoint_rows,
        retained_parent_checkpoint_all_single_row_value,
        retained_parent_checkpoint_prefix_launches,
        retained_parent_checkpoint_suffix_launches,
    )
    retained_parent_checkpoint_batching_hint_value = (
        retained_parent_checkpoint_batching_hint(
            retained_parent_checkpoint_openings,
            retained_parent_checkpoint_rows,
            retained_parent_checkpoint_all_single_row_value,
            retained_parent_checkpoint_prefix_launches,
            retained_parent_checkpoint_suffix_launches,
            retained_parent_checkpoint_path_ms,
            retained_parent_checkpoint_cross_stage_gather_launch_savings,
        )
    )
    retained_parent_checkpoint_action_hint = (
        opening_retained_parent_checkpoint_action_hint(
            retained_parent_checkpoint_openings,
            retained_parent_checkpoint_rows,
            retained_parent_checkpoint_all_single_row_value,
            opening_single_query_units,
            retained_parent_checkpoint_path_launches,
            retained_parent_checkpoint_path_ms,
            retained_parent_checkpoint_cross_stage_gather_launch_savings,
        )
    )
    opening_path_parent_hash_launches_per_stage = values.get(
        OPENING_PATH_PARENT_HASH_LAUNCHES_PER_STAGE_KEY, 0
    )
    opening_row_value_device_download_batches = values.get(
        OPENING_ROW_VALUE_DEVICE_DOWNLOAD_BATCHES_KEY, 0
    )
    opening_row_value_device_single_downloads = values.get(
        OPENING_ROW_VALUE_DEVICE_SINGLE_DOWNLOADS_KEY, 0
    )
    (
        opening_row_value_device_single_stage_count,
        opening_row_value_device_single_max_stage,
        opening_row_value_device_cross_unit_batch_savings,
    ) = opening_device_single_stage_shape(values)
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
    direct_d2h_hot_wait_pct = (
        direct_d2h_hot_wait_ms * 100.0 / direct_d2h_wait_ms
        if direct_d2h_wait_ms
        else 0.0
    )
    cuda_allocator_d2h_bytes = values.get(CUDA_COPY_D2H_BYTES_KEY, 0)
    cuda_allocator_d2h_wait_ms = (
        values.get(CUDA_COPY_D2H_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_d2h_hot_bytes = values.get(CUDA_COPY_D2H_HOT_BYTES_KEY, 0)
    cuda_allocator_d2h_hot_count = values.get(CUDA_COPY_D2H_HOT_COUNT_KEY, 0)
    cuda_allocator_d2h_hot_wait_ms = (
        values.get(CUDA_COPY_D2H_HOT_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_d2h_hot_wait_pct = (
        cuda_allocator_d2h_hot_wait_ms * 100.0 / cuda_allocator_d2h_wait_ms
        if cuda_allocator_d2h_wait_ms
        else 0.0
    )
    cuda_allocator_d2h_hint = allocator_d2h_action_hint(
        cuda_allocator_d2h_wait_ms,
        cuda_allocator_d2h_hot_count,
        cuda_allocator_d2h_hot_wait_pct,
        opening_query_units,
        opening_single_query_units,
        opening_row_value_device_rows,
        opening_row_value_device_download_batches,
    )
    direct_d2h_hint = direct_d2h_action_hint(
        direct_d2h_wait_ms,
        direct_d2h_hot_count,
        direct_d2h_hot_wait_pct,
        opening_query_units,
        opening_single_query_units,
        opening_row_value_device_rows,
        opening_row_value_device_download_batches,
        root_count,
        groups,
        max_group_size,
    )
    opening_row_value_d2h_wait_ms = direct_d2h_wait_ms
    if cuda_allocator_d2h_hint == "opening_row_value_d2h_wait_secondary":
        opening_row_value_d2h_wait_ms = max(
            opening_row_value_d2h_wait_ms, cuda_allocator_d2h_wait_ms
        )
    cuda_host_register_wait_ms = (
        values.get(CUDA_HOST_REGISTER_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_h2d_bytes = values.get(CUDA_COPY_H2D_BYTES_KEY, 0)
    descriptor_retention_attempts = values.get(DESCRIPTOR_RETENTION_ATTEMPTS_KEY, 0)
    descriptor_retention_retained = values.get(DESCRIPTOR_RETENTION_RETAINED_KEY, 0)
    descriptor_retention_rejected = values.get(DESCRIPTOR_RETENTION_REJECTED_KEY, 0)
    descriptor_retention_retained_bytes = values.get(
        DESCRIPTOR_RETENTION_RETAINED_BYTES_KEY, 0
    )
    descriptor_retention_rejected_bytes = values.get(
        DESCRIPTOR_RETENTION_REJECTED_BYTES_KEY, 0
    )
    descriptor_retention_limit_bytes = values.get(
        DESCRIPTOR_RETENTION_LIMIT_BYTES_KEY, 0
    )
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
    source_rebuild_hint = opening_source_rebuild_hint(
        opening_external_source_count,
        opening_retained_source_count,
        source_retention_attempts,
        source_retention_retained,
        source_retention_rejected,
        source_retention_limit_bytes,
    )
    opening_hint = opening_batching_hint(
        opening_query_units,
        opening_single_query_units,
        opening_row_value_device_rows,
        opening_row_value_device_download_batches,
        retained_leaf_openings,
        retained_leaf_rows,
        retained_leaf_all_single_row_value,
        retained_leaf_path_launches,
        retained_parent_checkpoint_openings,
        retained_parent_checkpoint_rows,
        retained_parent_checkpoint_all_single_row_value,
        retained_parent_checkpoint_prefix_launches,
        retained_parent_checkpoint_suffix_launches,
        retained_parent_checkpoint_path_ms,
        opening_row_value_d2h_wait_ms,
    )
    opening_external_source_boundary = opening_external_source_boundary_hint(
        opening_external_source_count,
        opening_query_units,
        opening_single_query_units,
        opening_row_value_device_rows,
        opening_row_value_device_download_batches,
        opening_row_value_device_single_downloads,
        opening_row_value_d2h_wait_ms,
    )
    leaf_launch_pressure = "yes" if leaf_ntt_launches >= 10_000 else "no"
    trace_to_leaf_ratio = (
        max(runner_ms, lowerer_ms) / leaf_kernel_ms if leaf_kernel_ms else 0.0
    )
    bottleneck = primary_bottleneck(
        total_ms,
        runner_ms,
        lowerer_ms,
        trace_lower_ms,
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
        pending_receive_wait_ms,
        parallel_lower_workers,
        leaf_kernel_ms,
    )
    proof_12s_gap_ms = max(total_ms - PROOF_TARGET_MS, 0) if total_ms > 0 else 0
    proof_12s_hint = proof_target_gap_hint(
        total_ms,
        runner_ms,
        lowerer_ms,
        trace_lower_ms,
        stream_elapsed_ms,
        segment_commit_ms,
        finish_opening_ms,
        leaf_kernel_ms,
        direct_d2h_wait_ms,
    )
    trace_pipeline_hint = trace_pipeline_action_hint_from_values(values)
    performance_focus = performance_focus_hint(
        trace_pipeline_hint,
        retained_parent_checkpoint_action_hint,
    )
    opening_source_row_value_hint = opening_source_row_value_action_hint(
        total_ms,
        opening_row_value_source_extend_ms,
        opening_row_value_source_rows,
        opening_external_source_count,
        opening_query_units,
        opening_single_query_units,
        trace_pipeline_hint,
    )
    cuda_transfer_hint = cuda_transfer_action_hint_from_values(values)
    data_residency_hint = data_residency_action_hint(
        source_rebuild_hint,
        cuda_transfer_hint,
        source_retention_rejected_bytes,
        segment_commit_cuda_memory_total_bytes,
        values.get(NSYS_COPY_TRACE_DESCRIPTOR_RESIDENCY_PIPELINE_KEY, 0) > 0,
    )
    copy_summary_gpu_residency_hint = str(
        values.get(NSYS_COPY_GPU_RESIDENCY_HINT_KEY, "none")
    )
    copy_summary_h2d_bulk_app_frame_hint = str(
        values.get(NSYS_COPY_H2D_BULK_APP_FRAME_HINT_KEY, "none")
    )
    copy_summary_small_d2h_batching_hint = str(
        values.get(NSYS_COPY_SMALL_D2H_BATCHING_HINT_KEY, "none")
    )
    copy_summary_cuda_api_backtrace_hint = str(
        values.get(NSYS_COPY_CUDA_API_BACKTRACE_HINT_KEY, "none")
    )
    kernel_graph_fusion_priority_hint = str(
        values.get(NSYS_KERNEL_GRAPH_FUSION_PRIORITY_HINT_KEY, "none")
    )
    kernel_next_action_hint = str(values.get(NSYS_KERNEL_NEXT_ACTION_HINT_KEY, "none"))
    kernel_graph_fusion_upper_bound_ms = str(
        values.get(NSYS_KERNEL_GRAPH_FUSION_UPPER_BOUND_MS_KEY, "0.000")
    )
    kernel_top_stream_idle_ms = str(
        values.get(NSYS_KERNEL_TOP_STREAM_IDLE_MS_KEY, "0.000")
    )
    kernel_separation_hint = str(
        values.get(NSYS_KERNEL_SEPARATION_HINT_KEY, "none")
    )
    kernel_top_stream_idle_gap_previous = str(
        values.get(NSYS_KERNEL_TOP_STREAM_IDLE_GAP_PREVIOUS_KEY, "none")
    )
    kernel_top_stream_idle_gap_next = str(
        values.get(NSYS_KERNEL_TOP_STREAM_IDLE_GAP_NEXT_KEY, "none")
    )
    kernel_top_stream_idle_gap_calls = str(
        values.get(NSYS_KERNEL_TOP_STREAM_IDLE_GAP_CALLS_KEY, "0")
    )
    kernel_top_stream_idle_gap_ms = str(
        values.get(NSYS_KERNEL_TOP_STREAM_IDLE_GAP_MS_KEY, "0.000")
    )
    kernel_stream_idle_boundary = kernel_stream_idle_boundary_hint(
        kernel_top_stream_idle_gap_previous,
        kernel_top_stream_idle_gap_next,
        kernel_top_stream_idle_gap_calls,
        root_count,
    )
    ncu_metric_collection_hint = str(
        values.get(NCU_METRIC_COLLECTION_HINT_KEY, "none")
    )
    ncu_top_kernel = str(values.get(NCU_TOP_KERNEL_KEY, "none"))
    ncu_top_kernel_duration_ms = str(
        values.get(NCU_TOP_KERNEL_DURATION_MS_KEY, "0.000")
    )
    ncu_top_kernel_sm_throughput_pct = str(
        values.get(NCU_TOP_KERNEL_SM_THROUGHPUT_PCT_KEY, "0.000")
    )
    ncu_top_kernel_dram_throughput_pct = str(
        values.get(NCU_TOP_KERNEL_DRAM_THROUGHPUT_PCT_KEY, "0.000")
    )
    ncu_top_kernel_registers_per_thread = str(
        values.get(NCU_TOP_KERNEL_REGISTERS_PER_THREAD_KEY, "0.000")
    )
    ncu_top_kernel_limiting_factors = str(
        values.get(NCU_TOP_KERNEL_LIMITING_FACTORS_KEY, "unknown")
    )
    source_retention_total_exceeds_device_memory = (
        source_retention_exceeds_device_memory_hint(
            source_retention_rejected_bytes,
            segment_commit_cuda_memory_total_bytes,
            source_retention_rejected > 0,
        )
    )
    source_retention_max_exceeds_device_memory = (
        source_retention_exceeds_device_memory_hint(
            source_retention_max_rejected_bytes,
            segment_commit_cuda_memory_total_bytes,
            source_retention_rejected > 0,
        )
    )
    if perf_hotspots is None:
        perf_hotspots = parse_perf_self_hotspots("")
    pending_drop_pct = perf_hotspots.get(
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY, 0.0
    )
    trace_lifetime_hint = trace_report_lifetime_hint(
        trace_reports,
        trace_report_buffer_capacity,
        trace_report_buffer_capacity_present,
        trace_report_buffer_excess_pct,
        pending_drop_pct,
        lowerer_ms,
        stream_elapsed_ms,
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
    cpu_report_storage_hint = cpu_trace_report_storage_action_hint(values, perf_hotspots)
    cpu_report_storage_memcpy_pct = perf_hotspots.get(
        CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY, 0.0
    )
    cpu_report_storage_memcpy_total_pct = (
        memmove_pct * cpu_report_storage_memcpy_pct / 100.0
    )
    append_descriptor_pct = perf_hotspots.get(
        PERF_APPEND_DESCRIPTOR_SELF_PCT_KEY, 0.0
    )
    source_value_pct = perf_hotspots.get(PERF_SOURCE_VALUE_SELF_PCT_KEY, 0.0)
    lowerer_hint = cpu_trace_lowerer_action_hint(perf_hotspots, trace_report_detail_action)
    prepare_instruction_pct = perf_hotspots.get(
        PERF_PREPARE_INSTRUCTION_SELF_PCT_KEY, 0.0
    )
    trace_segment_build_pct = perf_hotspots.get(
        PERF_TRACE_SEGMENT_BUILD_SELF_PCT_KEY, 0.0
    )
    advance_guest_machine_pct = perf_hotspots.get(
        PERF_ADVANCE_GUEST_MACHINE_SELF_PCT_KEY, 0.0
    )
    guest_memory_write_pct = perf_hotspots.get(
        PERF_GUEST_MEMORY_WRITE_SELF_PCT_KEY, 0.0
    )
    biguint_modpow_pct = perf_hotspots.get(PERF_BIGUINT_MODPOW_SELF_PCT_KEY, 0.0)
    guest_memory_read_pct = perf_hotspots.get(
        PERF_GUEST_MEMORY_READ_SELF_PCT_KEY, 0.0
    )
    decode_instruction_pct = perf_hotspots.get(
        PERF_DECODE_INSTRUCTION_SELF_PCT_KEY, 0.0
    )
    effect_memory_write_pct = perf_hotspots.get(
        PERF_EFFECT_RECORD_MEMORY_WRITE_SELF_PCT_KEY, 0.0
    )
    effect_memory_read_pct = perf_hotspots.get(
        PERF_EFFECT_RECORD_MEMORY_READ_SELF_PCT_KEY, 0.0
    )
    runner_hint = cpu_runner_hotspot_hint(perf_hotspots)
    return (
        f"{label},{input_bytes},{total_ms},"
        f"{constant_material_elapsed_ms},{constant_material_join_wait_ms},"
        f"{constant_material_hint},{runner_ms},{lowerer_ms},"
        f"{trace_lower_ms},{trace_report_ms},{trace_non_report_ms},"
        f"{trace_runner_lowerer_overlap_ms},"
        f"{trace_lowerer_non_lower_ms},"
        f"{stream_elapsed_ms},{stream_worker_ms},{segment_commit_ms},"
        f"{segment_commit_initial_workers},{segment_commit_effective_workers},"
        f"{segment_commit_worker_submits},{segment_commit_worker_joins},"
        f"{segment_commit_worker_backpressure_joins},"
        f"{segment_commit_worker_backpressure_join_ms},"
        f"{segment_commit_worker_finish_joins},{segment_commit_worker_finish_join_ms},"
        f"{segment_commit_worker_max_in_flight},"
        f"{segment_commit_worker_hint},"
        f"{segment_commit_oom_retries},"
        f"{segment_commit_attempt_ms},{segment_commit_oom_retry_ms},"
        f"{stream_commit_residual_ms},{segment_receive_wait_ms},"
        f"{pending_receive_wait_ms},{pending_send_wait_ms},"
        f"{parallel_lower_workers},{parallel_lower_dispatched},"
        f"{parallel_lower_received},{parallel_lower_emitted},"
        f"{parallel_lower_max_reorder},{parallel_lower_snapshot_replay},"
        f"{parallel_lower_report_elided},{parallel_lower_dispatch_wait_ms},"
        f"{parallel_lower_result_receive_wait_ms},"
        f"{parallel_lower_dispatch_blocked},{segment_replay_count},{trace_reports},"
        f"{trace_report_rows},{trace_rows_per_report:.3f},"
        f"{trace_report_record_size_bytes},"
        f"{trace_report_instruction_size_bytes},"
        f"{trace_report_register_write_list_size_bytes},"
        f"{trace_report_memory_access_list_size_bytes},"
        f"{trace_report_precompile_access_list_size_bytes},"
        f"{trace_report_instruction_storage_gib:.3f},"
        f"{trace_report_register_write_list_storage_gib:.3f},"
        f"{trace_report_memory_access_list_storage_gib:.3f},"
        f"{trace_report_precompile_access_list_storage_gib:.3f},"
        f"{trace_report_storage_bytes},"
        f"{trace_report_storage_gib:.3f},"
        f"{trace_report_buffer_capacity},{trace_report_buffer_max_capacity},"
        f"{trace_report_buffer_excess_capacity},"
        f"{trace_report_buffer_capacity_bytes},"
        f"{trace_report_buffer_capacity_gib:.3f},"
        f"{trace_report_buffer_excess_bytes},"
        f"{trace_report_buffer_excess_pct:.3f},{trace_report_buffer_hint},"
        f"{trace_runner_report_buffer_capacity},"
        f"{trace_runner_report_buffer_max_capacity},"
        f"{trace_runner_report_buffer_excess_capacity},"
        f"{trace_runner_report_buffer_capacity_bytes},"
        f"{trace_runner_report_buffer_capacity_gib:.3f},"
        f"{trace_runner_report_buffer_excess_bytes},"
        f"{trace_runner_report_buffer_excess_pct:.3f},"
        f"{trace_runner_report_buffer_hint},"
        f"{trace_lifetime_hint},"
        f"{trace_report_chunk_sent},{trace_report_chunk_received},"
        f"{trace_report_chunk_reports},{trace_report_chunk_rows},"
        f"{trace_report_chunk_max_queued},"
        f"{descriptor_rows},"
        f"{descriptor_compact_rows},{descriptor_wide_rows},"
        f"{descriptor_upload_bytes},{descriptor_bytes_per_row:.3f},"
        f"{descriptor_high32_values},{descriptor_high32_rows},"
        f"{descriptor_high32_row_pct:.3f},{descriptor_high32_a_values},"
        f"{descriptor_high32_b_values},{descriptor_high32_c_values},"
        f"{descriptor_high32_a_payload_values},{descriptor_high32_b_payload_values},"
        f"{descriptor_high32_store_payload_values},"
        f"{descriptor_high32_store_prev_value_values},"
        f"{','.join(str(count) for count in descriptor_high32_row_field_histogram)},"
        f"{sparse_high32_estimated_upload_bytes},"
        f"{sparse_high32_estimated_upload_savings_pct:.3f},"
        f"{sparse_high32_high_words},{sparse_high32_shape_hint},"
        f"{descriptor_hint},"
        f"{seed_direct_lift_attempts},"
        f"{seed_direct_lift_successes},{seed_direct_lift_success_pct:.3f},"
        f"{seed_direct_lift_dominant_miss},{seed_direct_lift_action},"
        f"{seed_direct_lift_empty_segments},"
        f"{seed_direct_lift_pending_dma_single_reports},"
        f"{seed_direct_lift_amo_boundaries},"
        f"{seed_direct_lift_store_conditional_boundaries},"
        f"{seed_direct_lift_dma_prepare_missing_lookaheads},"
        f"{seed_direct_lift_boundary_c_unavailable},{seed_full_advances},"
        f"{finish_opening_ms},{opening_query_units},{opening_single_query_units},"
        f"{opening_queries},{opening_max_queries_per_unit},{opening_stage_count},"
        f"{opening_source_hint},"
        f"{source_retention_attempts},{source_retention_retained},"
        f"{source_retention_rejected},{source_retention_retained_bytes},"
        f"{source_retention_rejected_bytes},{source_retention_max_retained_bytes},"
        f"{source_retention_max_rejected_bytes},{source_retention_limit_bytes},"
        f"{source_retention_total_exceeds_device_memory},"
        f"{source_retention_max_exceeds_device_memory},"
        f"{source_rebuild_hint},{opening_row_value_device_rows},"
        f"{opening_row_value_source_rows},{opening_row_value_source_extend_ms},"
        f"{opening_row_value_source_extend_pct:.3f},{opening_source_row_value_hint},"
        f"{retained_leaf_openings},{retained_leaf_rows},"
        f"{retained_leaf_all_single_row},{retained_leaf_path_launches},"
        f"{retained_parent_checkpoint_openings},{retained_parent_checkpoint_rows},"
        f"{retained_parent_checkpoint_all_single_row},"
        f"{retained_parent_checkpoint_prefix_rows},"
        f"{retained_parent_checkpoint_prefix_bytes},"
        f"{retained_parent_checkpoint_prefix_launches},"
        f"{retained_parent_checkpoint_prefix_ms},"
        f"{retained_parent_checkpoint_suffix_rows},"
        f"{retained_parent_checkpoint_suffix_bytes},"
        f"{retained_parent_checkpoint_suffix_launches},"
        f"{retained_parent_checkpoint_suffix_ms},"
        f"{retained_parent_checkpoint_path_launches},"
        f"{retained_parent_checkpoint_path_ms},"
        f"{retained_parent_checkpoint_cross_stage_gather_estimated_launches},"
        f"{retained_parent_checkpoint_cross_stage_gather_launch_savings},"
        f"{retained_parent_checkpoint_batching_hint_value},"
        f"{opening_path_parent_hash_launches_per_stage},"
        f"{opening_row_value_device_download_batches},"
        f"{opening_row_value_device_single_downloads},"
        f"{opening_row_value_device_single_stage_count},"
        f"{opening_row_value_device_single_max_stage},"
        f"{opening_row_value_device_cross_unit_batch_savings},"
        f"{opening_hint},{opening_external_source_boundary},"
        f"{retained_parent_checkpoint_action_hint},"
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
        f"{cpu_report_storage_hint},"
        f"{cpu_report_storage_memcpy_pct:.3f},"
        f"{cpu_report_storage_memcpy_total_pct:.3f},"
        f"{append_descriptor_pct:.3f},{source_value_pct:.3f},{lowerer_hint},"
        f"{prepare_instruction_pct:.3f},{trace_segment_build_pct:.3f},"
        f"{advance_guest_machine_pct:.3f},{guest_memory_write_pct:.3f},"
        f"{biguint_modpow_pct:.3f},{guest_memory_read_pct:.3f},"
        f"{decode_instruction_pct:.3f},{effect_memory_write_pct:.3f},"
        f"{effect_memory_read_pct:.3f},{runner_hint},"
        f"{single_row_reports},{multi_row_reports},{pending_dma_reports},"
        f"{amo_reports},{store_conditional_reports},{external_op_rows},"
        f"{copy_rows},{flag_rows},{precompile_rows},"
        f"{indirect_memory_rows},{indirect_memory_row_pct:.3f},"
        f"{register_source_reads},{memory_source_reads},"
        f"{memory_source_read_pct:.3f},{register_store_rows},"
        f"{memory_store_rows},{memory_store_row_pct:.3f},"
        f"{no_store_rows},{no_store_row_pct:.3f},{trace_shape_hint},"
        f"{copy_memory_source_rows},{copy_memory_source_row_pct:.3f},"
        f"{copy_indirect_memory_rows},{copy_indirect_memory_row_pct:.3f},"
        f"{copy_register_store_rows},{copy_memory_store_rows},"
        f"{copy_no_store_rows},{copy_no_memory_rows},"
        f"{copy_no_memory_row_pct:.3f},{copy_shape_hint},"
        f"{trace_report_validation_ms},{trace_report_emit_ms},{trace_descriptor_ms},"
        f"{trace_report_lowering_ms},{trace_report_row_validation_ms},"
        f"{trace_report_memory_columns_ms},{trace_report_source_values_ms},"
        f"{trace_report_source_a_value_ms},{trace_report_source_b_value_ms},"
        f"{trace_report_precompile_memory_ms},"
        f"{trace_report_instruction_result_ms},{trace_report_next_pc_ms},"
        f"{trace_report_register_access_ms},{trace_report_memory_access_ms},"
        f"{trace_report_store_apply_ms},{trace_report_visit_ms},"
        f"{trace_report_exact_hotspot_name},"
        f"{trace_report_exact_hotspot_pct:.3f},{trace_report_exact_action},"
        f"{trace_report_detail_samples},{trace_report_detail_sample_pct:.3f},"
        f"{trace_report_detail_sample_ppm:.3f},{trace_report_detail_hint},"
        f"{trace_report_detail_avg_ns},"
        f"{trace_report_detail_share_ms:.3f},"
        f"{trace_report_row_validation_share_ms:.3f},"
        f"{trace_report_memory_columns_share_ms:.3f},"
        f"{trace_report_source_values_share_ms:.3f},"
        f"{trace_report_source_lookup_share_ms:.3f},"
        f"{trace_report_source_values_residual_share_ms:.3f},"
        f"{trace_report_precompile_memory_share_ms:.3f},"
        f"{trace_report_instruction_result_share_ms:.3f},"
        f"{trace_report_next_pc_share_ms:.3f},"
        f"{trace_report_register_access_share_ms:.3f},"
        f"{trace_report_memory_access_share_ms:.3f},"
        f"{trace_report_store_apply_share_ms:.3f},"
        f"{trace_report_row_validation_residual_share_ms:.3f},"
        f"{trace_report_visit_share_ms:.3f},"
        f"{trace_report_descriptor_share_ms:.3f},"
        f"{trace_report_detail_hotspot_name},{trace_report_detail_hotspot_pct:.3f},"
        f"{trace_report_detail_action},"
        f"{trace_report_row_validation_hotspot_name},"
        f"{trace_report_row_validation_hotspot_pct:.3f},"
        f"{trace_report_row_validation_explained_pct:.3f},"
        f"{trace_report_row_validation_residual_pct:.3f},"
        f"{trace_report_source_values_lookup_pct:.3f},"
        f"{trace_report_source_values_residual_pct:.3f},"
        f"{source_immediate_reads},{source_immediate_read_pct:.3f},"
        f"{source_register_reads},{source_register_read_pct:.3f},"
        f"{source_memory_reads},{source_memory_read_pct:.3f},"
        f"{source_indirect_reads},{source_indirect_read_pct:.3f},"
        f"{source_last_c_reads},{source_last_c_read_pct:.3f},"
        f"{trace_report_source_kind_hotspot},"
        f"{trace_report_source_kind_hotspot_pct:.3f},"
        f"{trace_report_source_kind_coverage_pct:.3f},"
        f"{trace_report_source_kind_residual_pct:.3f},"
        f"{trace_report_detail_visit_pct:.3f},"
        f"{trace_report_visit_descriptor_pct:.3f},"
        f"{trace_report_visit_residual_pct:.3f},"
        f"{direct_d2h_hot_bytes},{direct_d2h_hot_count},"
        f"{direct_d2h_hot_wait_ms:.3f},{direct_d2h_hot_wait_pct:.3f},"
        f"{direct_d2h_hint},"
        f"{cuda_allocator_d2h_bytes},{cuda_allocator_d2h_wait_ms:.3f},"
        f"{cuda_allocator_d2h_hot_bytes},{cuda_allocator_d2h_hot_count},"
        f"{cuda_allocator_d2h_hot_wait_ms:.3f},"
        f"{cuda_allocator_d2h_hot_wait_pct:.3f},{cuda_allocator_d2h_hint},"
        f"{cuda_host_register_wait_ms:.3f},{cuda_h2d_bytes},{cuda_transfer_hint},"
        f"{data_residency_hint},"
        f"{copy_summary_gpu_residency_hint},{copy_summary_h2d_bulk_app_frame_hint},"
        f"{copy_summary_small_d2h_batching_hint},"
        f"{copy_summary_cuda_api_backtrace_hint},"
        f"{kernel_graph_fusion_priority_hint},{kernel_next_action_hint},"
        f"{kernel_graph_fusion_upper_bound_ms},"
        f"{kernel_top_stream_idle_ms},{kernel_separation_hint},"
        f"{kernel_top_stream_idle_gap_previous},{kernel_top_stream_idle_gap_next},"
        f"{kernel_top_stream_idle_gap_calls},{kernel_top_stream_idle_gap_ms},"
        f"{kernel_stream_idle_boundary},"
        f"{ncu_metric_collection_hint},{ncu_top_kernel},"
        f"{ncu_top_kernel_duration_ms},{ncu_top_kernel_sm_throughput_pct},"
        f"{ncu_top_kernel_dram_throughput_pct},"
        f"{ncu_top_kernel_registers_per_thread},{ncu_top_kernel_limiting_factors},"
        f"{segment_commit_cuda_memory_total_bytes},"
        f"{segment_commit_cuda_memory_initial_free_bytes},"
        f"{segment_commit_cuda_memory_effective_free_bytes},"
        f"{segment_commit_cuda_memory_min_free_bytes},"
        f"{segment_commit_cuda_allocator_initial_cached_bytes},"
        f"{segment_commit_cuda_allocator_effective_cached_bytes},"
        f"{segment_commit_cuda_memory_min_free_pct:.3f},"
        f"{segment_commit_memory_hint},"
        f"{descriptor_retention_attempts},{descriptor_retention_retained},"
        f"{descriptor_retention_rejected},{descriptor_retention_retained_bytes},"
        f"{descriptor_retention_rejected_bytes},{descriptor_retention_limit_bytes},"
        f"{external_op_row_pct:.3f},{copy_row_pct:.3f},{trace_shape_row_mix},"
        f"{external_op_row_lower_ms},{copy_row_lower_ms},"
        f"{external_op_row_lower_ns_per_row:.3f},"
        f"{copy_row_lower_ns_per_row:.3f},"
        f"{external_op_row_lower_pct:.3f},{copy_row_lower_pct:.3f},"
        f"{trace_shape_duration},{trace_shape_unit_cost},"
        f"{trace_report_source_values_residual_ns_per_row:.3f},"
        f"{trace_report_row_validation_residual_ns_per_row:.3f},"
        f"{trace_report_visit_residual_ns_per_row:.3f},"
        f"{trace_report_descriptor_ns_per_row:.3f},"
        f"{external_op_runs},{external_op_avg_run:.3f},"
        f"{external_op_max_run},{copy_runs},{copy_avg_run:.3f},"
        f"{copy_max_run},{trace_shape_run},"
        f"{trace_pipeline_hint},{performance_focus},{trace_shape_profile}"
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


def dominant_hint_and_consensus(hints: list[str]) -> tuple[str, str]:
    hint_counts: dict[str, int] = {}
    for hint in hints:
        hint_counts[hint] = hint_counts.get(hint, 0) + 1

    dominant_hint = "none"
    dominant_count = 0
    for hint in hints:
        count = hint_counts[hint]
        if count > dominant_count:
            dominant_hint = hint
            dominant_count = count
    consensus = "yes" if hints and dominant_count == len(hints) else "no"
    return dominant_hint, consensus


def summarize_total_samples(parsed_inputs: list[tuple[str, dict[str, int]]]) -> str:
    total_count = len(parsed_inputs)
    valid_inputs = [
        (label, values)
        for label, values in parsed_inputs
        if values.get(TOTAL_MS_KEY, 0) > 0 and not is_diagnostic_shape_profile(values)
    ]
    totals = [
        values[TOTAL_MS_KEY]
        for _, values in valid_inputs
    ]
    valid_total_count = len(totals)
    if not totals:
        return (
            f"aggregate,{total_count},0,0,0.000,0.000,0,0.000,no,no,"
            "none,no,none,no,none,no"
        )

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
    action_hints = [
        trace_pipeline_action_hint_from_values(values)
        for _, values in valid_inputs
    ]
    dominant_action_hint, action_consensus = dominant_hint_and_consensus(action_hints)
    trace_structure_hints = [
        trace_structure_hint_from_values(values)
        for _, values in valid_inputs
    ]
    dominant_trace_structure_hint, trace_structure_consensus = (
        dominant_hint_and_consensus(trace_structure_hints)
    )
    transfer_hints = [
        cuda_transfer_action_hint_from_values(values)
        for _, values in valid_inputs
    ]
    dominant_transfer_hint, transfer_consensus = dominant_hint_and_consensus(transfer_hints)
    return (
        f"aggregate,{total_count},{valid_total_count},{total_min_ms},"
        f"{total_mean_ms:.3f},{total_median_ms:.3f},{total_max_ms},"
        f"{sample_spread_pct:.3f},{close_samples},{max_outlier},"
        f"{dominant_action_hint},{action_consensus},"
        f"{dominant_trace_structure_hint},{trace_structure_consensus},"
        f"{dominant_transfer_hint},{transfer_consensus}"
    )


def summarize_total_samples_by_input_bytes(
    input_bytes: int,
    parsed_inputs: list[tuple[str, dict[str, int]]],
) -> str:
    summary = summarize_total_samples(parsed_inputs)
    return f"aggregate_by_input_bytes,{input_bytes},{summary.split(',', 1)[1]}"


def grouped_total_samples_by_input_bytes(
    parsed_inputs: list[tuple[str, dict[str, int]]],
) -> list[tuple[int, list[tuple[str, dict[str, int]]]]]:
    groups: dict[int, list[tuple[str, dict[str, int]]]] = {}
    order: list[int] = []
    for label, values in parsed_inputs:
        input_bytes = values.get(INPUT_BYTES_KEY, 0)
        if input_bytes not in groups:
            groups[input_bytes] = []
            order.append(input_bytes)
        groups[input_bytes].append((label, values))
    return [(input_bytes, groups[input_bytes]) for input_bytes in order]


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
        grouped_inputs = grouped_total_samples_by_input_bytes(
            [(label, values) for label, values, _ in parsed_inputs]
        )
        if len(grouped_inputs) > 1:
            print(AGGREGATE_BY_INPUT_BYTES_HEADER)
            for input_bytes, group in grouped_inputs:
                print(summarize_total_samples_by_input_bytes(input_bytes, group))


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
                        f"{DESCRIPTOR_HIGH32_A_VALUES_KEY}=1",
                        f"{DESCRIPTOR_HIGH32_B_VALUES_KEY}=0",
                        f"{DESCRIPTOR_HIGH32_C_VALUES_KEY}=2",
                        f"{DESCRIPTOR_HIGH32_A_PAYLOAD_VALUES_KEY}=0",
                        f"{DESCRIPTOR_HIGH32_B_PAYLOAD_VALUES_KEY}=1",
                        f"{DESCRIPTOR_HIGH32_STORE_PAYLOAD_VALUES_KEY}=0",
                        f"{DESCRIPTOR_HIGH32_STORE_PREV_VALUE_VALUES_KEY}=2",
                        f"{DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS[0]}=10",
                        f"{DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS[1]}=3",
                        f"{DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS[2]}=2",
                        f"{DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS[3]}=1",
                        f"{DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS[4]}=0",
                        f"{DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS[5]}=1",
                        f"{DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS[6]}=0",
                        f"{DESCRIPTOR_HIGH32_ROW_FIELD_HISTOGRAM_KEYS[7]}=0",
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


def sibling_cpu_summary_paths(input_path: Path) -> list[Path]:
    candidates = [
        input_path.with_suffix(".cpu-summary.txt"),
        input_path.with_suffix(".cpu.txt"),
        input_path.with_name(f"{input_path.name}.cpu-summary.txt"),
        input_path.with_name(f"{input_path.name}.cpu.txt"),
    ]
    paths = []
    seen = set()
    for candidate in candidates:
        if candidate == input_path or candidate in seen:
            continue
        seen.add(candidate)
        paths.append(candidate)
    return paths


def sibling_copy_summary_paths(input_path: Path) -> list[Path]:
    stem = input_path.stem
    name = input_path.name
    candidates = [
        input_path.with_suffix(".copy-summary.txt"),
        input_path.with_suffix(".copy.txt"),
        input_path.with_name(f"{input_path.name}.copy-summary.txt"),
        input_path.with_name(f"{input_path.name}.copy.txt"),
        input_path.with_name(f"{stem}-copy-summary.txt"),
        input_path.with_name(f"{stem}-copy.txt"),
        input_path.with_name(f"{name}-copy-summary.txt"),
        input_path.with_name(f"{name}-copy.txt"),
    ]
    paths = []
    seen = set()
    for candidate in candidates:
        if candidate == input_path or candidate in seen:
            continue
        seen.add(candidate)
        paths.append(candidate)
    return paths


def sibling_kernel_summary_paths(input_path: Path) -> list[Path]:
    stem = input_path.stem
    name = input_path.name
    candidates = [
        input_path.with_suffix(".kernel-summary.txt"),
        input_path.with_suffix(".kernel.txt"),
        input_path.with_name(f"{input_path.name}.kernel-summary.txt"),
        input_path.with_name(f"{input_path.name}.kernel.txt"),
        input_path.with_name(f"{stem}-kernel-summary.txt"),
        input_path.with_name(f"{stem}-kernel.txt"),
        input_path.with_name(f"{name}-kernel-summary.txt"),
        input_path.with_name(f"{name}-kernel.txt"),
    ]
    paths = []
    seen = set()
    for candidate in candidates:
        if candidate == input_path or candidate in seen:
            continue
        seen.add(candidate)
        paths.append(candidate)
    return paths


def sibling_ncu_summary_paths(input_path: Path) -> list[Path]:
    stem = input_path.stem
    name = input_path.name
    candidates = [
        input_path.with_suffix(".ncu-summary.txt"),
        input_path.with_suffix(".ncu.txt"),
        input_path.with_name(f"{input_path.name}.ncu-summary.txt"),
        input_path.with_name(f"{input_path.name}.ncu.txt"),
        input_path.with_name(f"{stem}-ncu-summary.txt"),
        input_path.with_name(f"{stem}-ncu.txt"),
        input_path.with_name(f"{name}-ncu-summary.txt"),
        input_path.with_name(f"{name}-ncu.txt"),
    ]
    for pattern in [
        f"{stem}.ncu-*-summary.txt",
        f"{stem}-ncu-*-summary.txt",
        f"{name}.ncu-*-summary.txt",
        f"{name}-ncu-*-summary.txt",
    ]:
        candidates.extend(sorted(input_path.parent.glob(pattern)))
    paths = []
    seen = set()
    for candidate in candidates:
        if candidate == input_path or candidate in seen:
            continue
        seen.add(candidate)
        paths.append(candidate)
    return paths


def read_report_texts(paths: list[str]) -> list[str]:
    return [Path(path).read_text(encoding="utf-8") for path in paths]


def read_input(path: str | None, extra_reports: list[str] | None = None) -> tuple[str, str]:
    if path is None or path == "-":
        text = sys.stdin.read()
        if extra_reports:
            text = "\n".join([text, *read_report_texts(extra_reports)])
        return ("stdin", text)
    input_path = Path(path)
    text = input_path.read_text(encoding="utf-8")
    sibling_reports = [
        report_path.read_text(encoding="utf-8")
        for report_path in (
            sibling_perf_report_paths(input_path)
            + sibling_cpu_summary_paths(input_path)
            + sibling_copy_summary_paths(input_path)
            + sibling_kernel_summary_paths(input_path)
            + sibling_ncu_summary_paths(input_path)
        )
        if report_path.is_file()
    ]
    if extra_reports:
        sibling_reports.extend(read_report_texts(extra_reports))
    if sibling_reports:
        text = "\n".join([text, *sibling_reports])
    return (str(input_path), text)


def main() -> None:
    parser = argparse.ArgumentParser(description="Summarize prove timing root materialization shape.")
    parser.add_argument("logs", nargs="*", help="prove --timings log paths, or '-' for stdin")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--nsys-copy-summary",
        action="append",
        default=[],
        help="additional nsys CUDA copy summary text to merge into each log",
    )
    parser.add_argument(
        "--nsys-cpu-summary",
        action="append",
        default=[],
        help="additional nsys CPU sampling summary text to merge into each log",
    )
    parser.add_argument(
        "--nsys-kernel-summary",
        action="append",
        default=[],
        help="additional nsys CUDA kernel summary text to merge into each log",
    )
    parser.add_argument(
        "--ncu-kernel-summary",
        action="append",
        default=[],
        help="additional ncu CUDA kernel summary text to merge into each log",
    )
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return
    if not args.logs:
        raise SystemExit("at least one log path is required unless --self-test is used")
    extra_reports = (
        args.nsys_copy_summary
        + args.nsys_cpu_summary
        + args.nsys_kernel_summary
        + args.ncu_kernel_summary
    )
    print_summary([read_input(path, extra_reports) for path in args.logs])


if __name__ == "__main__":
    main()

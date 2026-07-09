#!/usr/bin/env python3
import argparse
import csv
import re
import sys
from pathlib import Path


INPUT_BYTES_KEY = "input_bytes"
LOWERER_SOURCE_VALUE_CLOSE_TO_DESCRIPTOR_RATIO = 0.75
SELECTED_DESCRIPTOR_ROW_REBUILD_RATIO = 1024
SELECTED_DESCRIPTOR_ROW_REBUILD_MIN_ROWS = 1024
ROOT_COUNT_KEY = "timing_guest_stage_tree_commit_root_count"
ROOT_GROUPS_KEY = "timing_guest_stage_tree_commit_root_materialization_groups"
ROOT_MAX_GROUP_KEY = "timing_guest_stage_tree_commit_root_materialization_max_group_size"
TOTAL_MS_KEY = "timing_total_ms"
CATALOG_MS_KEY = "timing_catalog_ms"
ETH_INPUT_MS_KEY = "timing_eth_input_ms"
PUBLIC_INPUTS_MS_KEY = "timing_public_inputs_ms"
PLAN_MS_KEY = "timing_plan_ms"
FRAMED_GUEST_INPUT_MS_KEY = "timing_framed_guest_input_ms"
GPU_MEMORY_PREFLIGHT_MS_KEY = "timing_gpu_memory_preflight_ms"
GPU_SETUP_MS_KEY = "timing_gpu_setup_ms"
AUXILIARY_INPUTS_MS_KEY = "timing_auxiliary_inputs_ms"
TRACE_INPUTS_MS_KEY = "timing_trace_inputs_ms"
WITNESS_MS_KEY = "timing_witness_ms"
PROOF_MS_KEY = "timing_proof_ms"
OUTPUT_WRITE_MS_KEY = "timing_output_write_ms"
SUMMARY_MS_KEY = "timing_summary_ms"
CONSTANT_MATERIAL_VALIDATION_ELAPSED_MS_KEY = (
    "timing_constant_material_validation_elapsed_ms"
)
CONSTANT_MATERIAL_VALIDATION_JOIN_WAIT_MS_KEY = (
    "timing_constant_material_validation_join_wait_ms"
)
RUNNER_MS_KEY = "timing_guest_trace_runner_ms"
RUNNER_DETAIL_SAMPLES_KEY = "timing_guest_trace_runner_detail_samples"
RUNNER_ADVANCE_FAST_PATHS_KEY = "timing_guest_trace_runner_advance_fast_paths"
RUNNER_ADVANCE_GENERIC_FALLBACKS_KEY = (
    "timing_guest_trace_runner_advance_generic_fallbacks"
)
RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_1_PATTERN_KEY = (
    "timing_guest_trace_runner_advance_generic_fallback_shape_top_1_pattern"
)
RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_1_COUNT_KEY = (
    "timing_guest_trace_runner_advance_generic_fallback_shape_top_1_count"
)
RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_2_PATTERN_KEY = (
    "timing_guest_trace_runner_advance_generic_fallback_shape_top_2_pattern"
)
RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_2_COUNT_KEY = (
    "timing_guest_trace_runner_advance_generic_fallback_shape_top_2_count"
)
RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_3_PATTERN_KEY = (
    "timing_guest_trace_runner_advance_generic_fallback_shape_top_3_pattern"
)
RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_3_COUNT_KEY = (
    "timing_guest_trace_runner_advance_generic_fallback_shape_top_3_count"
)
RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_4_PATTERN_KEY = (
    "timing_guest_trace_runner_advance_generic_fallback_shape_top_4_pattern"
)
RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_4_COUNT_KEY = (
    "timing_guest_trace_runner_advance_generic_fallback_shape_top_4_count"
)
RUNNER_CACHE_HITS_KEY = "timing_guest_trace_runner_instruction_cache_hits"
RUNNER_CACHE_MISSES_KEY = "timing_guest_trace_runner_instruction_cache_misses"
RUNNER_CACHE_CLEARS_KEY = "timing_guest_trace_runner_instruction_cache_clears"
RUNNER_CACHE_FCALL_CLEARS_KEY = (
    "timing_guest_trace_runner_instruction_cache_fcall_clears"
)
RUNNER_CACHE_DMA_CLEARS_KEY = "timing_guest_trace_runner_instruction_cache_dma_clears"
RUNNER_CACHE_INVALIDATION_RANGES_KEY = (
    "timing_guest_trace_runner_instruction_cache_write_invalidation_ranges"
)
RUNNER_CACHE_INVALIDATION_SKIPPED_RANGES_KEY = (
    "timing_guest_trace_runner_instruction_cache_write_invalidation_skipped_ranges"
)
RUNNER_CACHE_INVALIDATION_PROBES_KEY = (
    "timing_guest_trace_runner_instruction_cache_write_invalidation_probes"
)
RUNNER_CACHE_INVALIDATED_ENTRIES_KEY = (
    "timing_guest_trace_runner_instruction_cache_invalidated_entries"
)
RUNNER_DETAIL_SAMPLED_NS_KEY = "timing_guest_trace_runner_detail_sampled_ns"
RUNNER_PREPARE_INSTRUCTION_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_prepare_instruction_sampled_ns"
)
RUNNER_PRE_BOUNDARY_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_pre_boundary_sampled_ns"
)
RUNNER_ROW_PLAN_SAMPLED_NS_KEY = "timing_guest_trace_runner_row_plan_sampled_ns"
RUNNER_CACHE_POLICY_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_cache_policy_sampled_ns"
)
RUNNER_ADVANCE_SAMPLED_NS_KEY = "timing_guest_trace_runner_advance_sampled_ns"
RUNNER_ADVANCE_SETUP_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_advance_setup_sampled_ns"
)
RUNNER_ADVANCE_EXECUTE_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_advance_execute_sampled_ns"
)
RUNNER_ADVANCE_REPORT_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_advance_report_sampled_ns"
)
RUNNER_CACHE_UPDATE_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_cache_update_sampled_ns"
)
RUNNER_ROW_COUNT_SAMPLED_NS_KEY = "timing_guest_trace_runner_row_count_sampled_ns"
RUNNER_POST_BOUNDARY_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_post_boundary_sampled_ns"
)
RUNNER_COUNTER_UPDATE_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_counter_update_sampled_ns"
)
RUNNER_TIMER_BOOKKEEPING_SAMPLED_NS_KEY = (
    "timing_guest_trace_runner_timer_bookkeeping_sampled_ns"
)
LOWERER_MS_KEY = "timing_guest_trace_lowerer_ms"
TRACE_LOWER_MS_KEY = "timing_guest_trace_lower_ms"
TRACE_REPORT_MS_KEY = "timing_guest_trace_report_ms"
STREAM_ELAPSED_MS_KEY = "timing_guest_trace_stream_elapsed_ms"
STREAM_WORKER_MS_KEY = "timing_guest_trace_stream_ms"
SEGMENT_COMMIT_MS_KEY = "timing_guest_segment_commit_ms"
SEGMENT_COMMIT_ATTEMPT_MS_KEY = "timing_guest_segment_commit_attempt_ms"
SEGMENT_COMMIT_OOM_RETRY_MS_KEY = "timing_guest_segment_commit_oom_retry_ms"
SEGMENT_INPUT_GAP_MS_KEY = "timing_guest_segment_input_gap_ms"
SEGMENT_INPUT_GAP_MAX_MS_KEY = "timing_guest_segment_input_gap_max_ms"
SEGMENT_INPUT_GAP_COUNT_KEY = "timing_guest_segment_input_gap_count"
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
SEGMENT_COMMIT_CUDA_MEMORY_SAMPLE_MS_KEY = (
    "timing_guest_segment_commit_cuda_memory_sample_ms"
)
SEGMENT_COMMIT_CUDA_MEMORY_SAMPLE_COUNT_KEY = (
    "timing_guest_segment_commit_cuda_memory_samples"
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
PARALLEL_LOWER_SNAPSHOT_REPLAY_MS_KEY = (
    "timing_guest_trace_parallel_lower_snapshot_replay_ms"
)
PARALLEL_LOWER_REPORT_ELIDED_KEY = (
    "timing_guest_trace_parallel_lower_report_elided_count"
)
PARALLEL_LOWER_STREAM_SEGMENTS_KEY = (
    "timing_guest_trace_parallel_lower_stream_segments"
)
PARALLEL_LOWER_STREAM_CHUNKS_KEY = (
    "timing_guest_trace_parallel_lower_stream_chunks"
)
PARALLEL_LOWER_STREAM_FALLBACKS_KEY = (
    "timing_guest_trace_parallel_lower_stream_fallbacks"
)
PARALLEL_LOWER_STREAM_RETAINED_REPORTS_KEY = (
    "timing_guest_trace_parallel_lower_stream_retained_reports"
)
OWNED_STREAMING_LOWER_SEGMENTS_KEY = (
    "timing_guest_trace_owned_streaming_lower_segments"
)
PARALLEL_LOWER_DISPATCH_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_dispatch_wait_ms"
)
PARALLEL_LOWER_STREAM_START_DISPATCH_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_stream_start_dispatch_wait_ms"
)
PARALLEL_LOWER_STREAM_CHUNK_DISPATCH_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_stream_chunk_dispatch_wait_ms"
)
PARALLEL_LOWER_STREAM_CHUNK_PROCESS_MS_KEY = (
    "timing_guest_trace_parallel_lower_stream_chunk_process_ms"
)
PARALLEL_LOWER_JOB_RECEIVE_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_job_receive_wait_ms"
)
PARALLEL_LOWER_RESULT_SEND_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_result_send_wait_ms"
)
PARALLEL_LOWER_STREAM_SEGMENT_DISPATCH_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_stream_segment_dispatch_wait_ms"
)
PARALLEL_LOWER_STREAM_FINISH_DISPATCH_WAIT_MS_KEY = (
    "timing_guest_trace_parallel_lower_stream_finish_dispatch_wait_ms"
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
MAIN_REPORT_FAST_PATHS_KEY = "timing_guest_trace_main_report_fast_paths"
MAIN_REPORT_GENERIC_FALLBACKS_KEY = (
    "timing_guest_trace_main_report_generic_fallbacks"
)
MAIN_REPORT_FCALL_RESULT_FAST_PATHS_KEY = (
    "timing_guest_trace_main_report_fcall_result_fast_paths"
)
MAIN_REPORT_LOAD_COPY_FAST_PATHS_KEY = (
    "timing_guest_trace_main_report_load_copy_fast_paths"
)
MAIN_REPORT_LOAD_SIGN_EXTEND_FAST_PATHS_KEY = (
    "timing_guest_trace_main_report_load_sign_extend_fast_paths"
)
MAIN_REPORT_NO_MEMORY_FAST_PATHS_KEY = (
    "timing_guest_trace_main_report_no_memory_fast_paths"
)
MAIN_REPORT_STORE_COPY_FAST_PATHS_KEY = (
    "timing_guest_trace_main_report_store_copy_fast_paths"
)
MAIN_REPORT_SIMPLE_COPY_FAST_PATHS_KEY = (
    "timing_guest_trace_main_report_simple_copy_fast_paths"
)
MAIN_REPORT_JUMP_FAST_PATHS_KEY = (
    "timing_guest_trace_main_report_jump_fast_paths"
)
MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_1_PATTERN_KEY = (
    "timing_guest_trace_main_report_generic_fallback_shape_top_1_pattern"
)
MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_1_COUNT_KEY = (
    "timing_guest_trace_main_report_generic_fallback_shape_top_1_count"
)
MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_2_PATTERN_KEY = (
    "timing_guest_trace_main_report_generic_fallback_shape_top_2_pattern"
)
MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_2_COUNT_KEY = (
    "timing_guest_trace_main_report_generic_fallback_shape_top_2_count"
)
MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_3_PATTERN_KEY = (
    "timing_guest_trace_main_report_generic_fallback_shape_top_3_pattern"
)
MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_3_COUNT_KEY = (
    "timing_guest_trace_main_report_generic_fallback_shape_top_3_count"
)
MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_4_PATTERN_KEY = (
    "timing_guest_trace_main_report_generic_fallback_shape_top_4_pattern"
)
MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_4_COUNT_KEY = (
    "timing_guest_trace_main_report_generic_fallback_shape_top_4_count"
)
TRACE_REPORT_CHUNK_SENT_KEY = "timing_guest_trace_report_chunk_sent"
TRACE_REPORT_CHUNK_RECEIVED_KEY = "timing_guest_trace_report_chunk_received"
TRACE_REPORT_CHUNK_REPORTS_KEY = "timing_guest_trace_report_chunk_reports"
TRACE_REPORT_CHUNK_ROWS_KEY = "timing_guest_trace_report_chunk_rows"
TRACE_REPORT_CHUNK_MAX_QUEUED_KEY = "timing_guest_trace_report_chunk_max_queued"
TRACE_REPORT_VALIDATION_MS_KEY = "timing_guest_trace_report_validation_ms"
TRACE_REPORT_APPLY_MS_KEY = "timing_guest_trace_report_apply_ms"
TRACE_UNIT_SUMMARY_MS_KEY = "timing_guest_trace_unit_summary_ms"
TRACE_REPORT_LOWERING_MS_KEY = "timing_guest_trace_report_lowering_ms"
TRACE_REPORT_ROW_VALIDATION_MS_KEY = "timing_guest_trace_report_row_validation_ms"
TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_MS_KEY = (
    "timing_guest_trace_report_row_validation_timer_bookkeeping_ms"
)
TRACE_REPORT_MEMORY_COLUMNS_MS_KEY = "timing_guest_trace_report_memory_columns_ms"
TRACE_REPORT_SOURCE_VALUES_MS_KEY = "timing_guest_trace_report_source_values_ms"
TRACE_REPORT_SOURCE_A_VALUE_MS_KEY = "timing_guest_trace_report_source_a_value_ms"
TRACE_REPORT_SOURCE_B_VALUE_MS_KEY = "timing_guest_trace_report_source_b_value_ms"
TRACE_REPORT_SOURCE_VALUE_RECORD_MS_KEY = (
    "timing_guest_trace_report_source_value_record_ms"
)
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
TRACE_COPY_SOURCE_MEMORY_READ_MS_KEY = (
    "timing_guest_trace_copy_source_memory_read_ms"
)
TRACE_COPY_SOURCE_INDIRECT_READ_MS_KEY = (
    "timing_guest_trace_copy_source_indirect_read_ms"
)
TRACE_COPY_SOURCE_MEMORY_READS_KEY = "timing_guest_trace_copy_source_memory_reads"
TRACE_COPY_SOURCE_INDIRECT_READS_KEY = "timing_guest_trace_copy_source_indirect_reads"
TRACE_COPY_SOURCE_MEMORY_READ_SAMPLED_NS_KEY = (
    "timing_guest_trace_copy_source_memory_read_sampled_ns"
)
TRACE_COPY_SOURCE_INDIRECT_READ_SAMPLED_NS_KEY = (
    "timing_guest_trace_copy_source_indirect_read_sampled_ns"
)
TRACE_COPY_SOURCE_MEMORY_READ_AVG_SAMPLE_NS_KEY = (
    "timing_guest_trace_copy_source_memory_read_avg_sample_ns"
)
TRACE_COPY_SOURCE_INDIRECT_READ_AVG_SAMPLE_NS_KEY = (
    "timing_guest_trace_copy_source_indirect_read_avg_sample_ns"
)
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
TRACE_ROW_SHAPE_TOP_1_PATTERN_KEY = "timing_guest_trace_row_shape_top_1_pattern"
TRACE_ROW_SHAPE_TOP_1_COUNT_KEY = "timing_guest_trace_row_shape_top_1_count"
TRACE_ROW_SHAPE_TOP_2_PATTERN_KEY = "timing_guest_trace_row_shape_top_2_pattern"
TRACE_ROW_SHAPE_TOP_2_COUNT_KEY = "timing_guest_trace_row_shape_top_2_count"
TRACE_ROW_SHAPE_TOP_3_PATTERN_KEY = "timing_guest_trace_row_shape_top_3_pattern"
TRACE_ROW_SHAPE_TOP_3_COUNT_KEY = "timing_guest_trace_row_shape_top_3_count"
TRACE_ROW_SHAPE_TOP_4_PATTERN_KEY = "timing_guest_trace_row_shape_top_4_pattern"
TRACE_ROW_SHAPE_TOP_4_COUNT_KEY = "timing_guest_trace_row_shape_top_4_count"
TRACE_SHAPE_SAMPLES_KEY = "timing_guest_trace_shape_samples"
TRACE_SHAPE_SAMPLE_ROWS_KEY = "timing_guest_trace_shape_sample_rows"
TRACE_ROW_SHAPE_SOURCE_NAMES = {
    0: "imm",
    1: "reg",
    2: "mem",
    3: "indirect",
    4: "last_c",
}
TRACE_ROW_SHAPE_STORE_NAMES = {
    0: "none",
    1: "reg",
    2: "mem",
    3: "indirect",
}
TRACE_ROW_SHAPE_OP_NAMES = {
    0x00: "Flag",
    0x01: "CopyB",
    0x06: "Ltu",
    0x07: "Lt",
    0x09: "Eq",
    0x0A: "Add",
    0x0B: "Sub",
    0x0E: "And",
    0x0F: "Or",
    0x10: "Xor",
    0x1A: "AddW",
    0x1B: "SubW",
    0x21: "Sll",
    0x22: "Srl",
    0x23: "Sra",
    0x24: "SllW",
    0x25: "SrlW",
    0x26: "SraW",
    0x27: "SignExtendB",
    0x28: "SignExtendH",
    0x29: "SignExtendW",
    0xB1: "Mulhu",
    0xB3: "Mulhsu",
    0xB4: "Mul",
    0xB5: "Mulh",
    0xB6: "MulW",
    0xB8: "Divu",
    0xB9: "Remu",
    0xBA: "Div",
    0xBB: "Rem",
    0xBC: "DivuW",
    0xBD: "RemuW",
    0xBE: "DivW",
    0xBF: "RemW",
    0xD0: "DmaMemCpy",
    0xD1: "DmaMemCmp",
    0xD2: "DmaInputCpy",
    0xD6: "DmaXMemCpy",
    0xD7: "DmaXMemCmp",
    0xD9: "DmaXMemSet",
    0xF0: "Add256",
    0xF1: "Keccak",
    0xF2: "Arith256",
    0xF3: "Arith256Mod",
    0xF4: "Secp256k1Add",
    0xF5: "Secp256k1Dbl",
}
RUNNER_ADVANCE_SHAPE_KIND_NAMES = {
    1: "CompressedUnknown",
    2: "IllegalCompressed",
    3: "UnsupportedLong",
    4: "Lui",
    5: "Auipc",
    6: "Jal",
    7: "Jalr",
    8: "Branch",
    9: "Load",
    10: "Store",
    11: "OpImm",
    12: "OpImm32",
    13: "Op",
    14: "Op32",
    15: "Amo",
    16: "LoadReserved",
    17: "StoreConditional",
    18: "CsrRead",
    19: "ZiskPrecompile",
    20: "ZiskDmaPrepare",
    21: "ZiskFcallParam",
    22: "ZiskFcallInvoke",
    23: "ZiskFcallResult",
    24: "Fence",
    25: "Ecall",
    26: "Ebreak",
    27: "Unknown",
}
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
    TRACE_ROW_SHAPE_TOP_1_PATTERN_KEY,
    TRACE_ROW_SHAPE_TOP_1_COUNT_KEY,
    TRACE_ROW_SHAPE_TOP_2_PATTERN_KEY,
    TRACE_ROW_SHAPE_TOP_2_COUNT_KEY,
    TRACE_ROW_SHAPE_TOP_3_PATTERN_KEY,
    TRACE_ROW_SHAPE_TOP_3_COUNT_KEY,
    TRACE_ROW_SHAPE_TOP_4_PATTERN_KEY,
    TRACE_ROW_SHAPE_TOP_4_COUNT_KEY,
    TRACE_SHAPE_SAMPLES_KEY,
    TRACE_SHAPE_SAMPLE_ROWS_KEY,
)
TRACE_REPORT_DETAIL_SAMPLES_KEY = "timing_guest_trace_report_detail_samples"
TRACE_REPORT_SAMPLED_NS_KEY = "timing_guest_trace_report_sampled_ns"
TRACE_REPORT_LOWERING_SAMPLED_NS_KEY = "timing_guest_trace_report_lowering_sampled_ns"
TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_row_validation_sampled_ns"
)
TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_row_validation_timer_bookkeeping_sampled_ns"
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
TRACE_REPORT_SOURCE_VALUE_RECORD_SAMPLED_NS_KEY = (
    "timing_guest_trace_report_source_value_record_sampled_ns"
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
DEVICE_SOURCE_BUILD_MS_KEY = "timing_guest_device_source_build_ms"
DESCRIPTOR_UPLOAD_MS_KEY = "timing_guest_device_source_descriptor_upload_ms"
DEVICE_SOURCE_TRACE_EXPAND_MS_KEY = "timing_guest_device_source_trace_expand_ms"
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
SEED_DIRECT_LIFT_MS_KEY = "timing_guest_trace_seed_direct_lift_ms"
SEED_FULL_ADVANCE_MS_KEY = "timing_guest_trace_seed_full_advance_ms"
SEED_FULL_ADVANCES_KEY = "timing_guest_trace_seed_full_advances"
FINISH_OPENING_MS_KEY = "timing_finish_witness_opening_ms"
OPENING_EXTERNAL_SOURCE_MS_KEY = "timing_finish_witness_external_source_ms"
OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_MS_KEY = (
    "timing_finish_witness_external_source_descriptor_upload_ms"
)
OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_BYTES_KEY = (
    "timing_finish_witness_external_source_descriptor_upload_bytes"
)
OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_WORDS_KEY = (
    "timing_finish_witness_external_source_descriptor_upload_words"
)
OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_ROWS_KEY = (
    "timing_finish_witness_external_source_descriptor_upload_rows"
)
OPENING_EXTERNAL_SOURCE_TRACE_EXPAND_MS_KEY = (
    "timing_finish_witness_external_source_trace_expand_ms"
)
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
OPENING_ROW_VALUE_WORDS_KEY = "timing_finish_witness_opening_row_values_words"
OPENING_ROW_VALUE_BYTES_KEY = "timing_finish_witness_opening_row_values_bytes"
OPENING_ROW_VALUE_SOURCE_EXTEND_CALLS_KEY = (
    "timing_finish_witness_opening_row_value_source_extend_calls"
)
OPENING_ROW_VALUE_SOURCE_EXTEND_MAX_ROWS_KEY = (
    "timing_finish_witness_opening_row_value_source_extend_max_rows"
)
OPENING_ROW_VALUE_SOURCE_EXTEND_MS_KEY = (
    "timing_finish_witness_opening_row_value_source_extend_ms"
)
OPENING_ROW_VALUE_SOURCE_DOWNLOAD_MS_KEY = (
    "timing_finish_witness_opening_row_value_source_download_ms"
)
OPENING_ROW_VALUE_DEVICE_DOWNLOAD_MS_KEY = (
    "timing_finish_witness_opening_row_value_device_download_ms"
)
OPENING_ROW_DEDUP_INPUT_ROWS_KEY = "timing_finish_witness_opening_row_dedup_input_rows"
OPENING_ROW_DEDUP_UNIQUE_ROWS_KEY = "timing_finish_witness_opening_row_dedup_unique_rows"
OPENING_ROW_DEDUP_ELIDED_ROWS_KEY = "timing_finish_witness_opening_row_dedup_elided_rows"
FRI_OPENING_MS_KEY = "timing_finish_fri_opening_ms"
FRI_OPENING_UNIT_BUILD_MS_KEY = "timing_finish_fri_opening_unit_build_ms"
FRI_OPENING_LAYER_TREE_MS_KEY = "timing_finish_fri_opening_layer_tree_ms"
FRI_OPENING_QUERY_MS_KEY = "timing_finish_fri_opening_query_ms"
FRI_OPENING_FOLD_MS_KEY = "timing_finish_fri_opening_fold_ms"
FRI_OPENING_UNIT_COUNT_KEY = "timing_finish_fri_opening_unit_count"
FRI_OPENING_LAYER_COUNT_KEY = "timing_finish_fri_opening_layer_count"
FRI_OPENING_QUERY_COUNT_KEY = "timing_finish_fri_opening_query_count"
FRI_TRANSCRIPT_UNIT_BUILD_MS_KEY = "timing_finish_fri_transcript_unit_build_ms"
FRI_TRANSCRIPT_LAYER_TREE_MS_KEY = "timing_finish_fri_transcript_layer_tree_ms"
FRI_TRANSCRIPT_FOLD_MS_KEY = "timing_finish_fri_transcript_fold_ms"
FRI_TRANSCRIPT_UNIT_COUNT_KEY = "timing_finish_fri_transcript_unit_count"
FRI_TRANSCRIPT_LAYER_COUNT_KEY = "timing_finish_fri_transcript_layer_count"
CONTRIBUTION_SEGMENT_MS_KEY = "timing_finish_contribution_segment_ms"
CONTRIBUTION_VERIFY_MS_KEY = "timing_finish_contribution_verify_ms"
CONTRIBUTION_CHALLENGE_MS_KEY = "timing_finish_contribution_challenge_ms"
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
OPENING_STAGE_ROW_VALUE_DEVICE_DOWNLOAD_BATCH_RE = re.compile(
    r"^timing_finish_witness_stage_(\d+)_opening_row_values_device_download_batches$"
)
OPENING_STAGE_ROW_VALUE_DEVICE_SINGLE_DOWNLOAD_RE = re.compile(
    r"^timing_finish_witness_stage_(\d+)_opening_row_values_device_single_downloads$"
)
OPENING_STAGE_ROW_VALUE_SOURCE_EXTEND_CALLS_RE = re.compile(
    r"^timing_finish_witness_stage_(\d+)_opening_row_value_source_extend_calls$"
)
OPENING_STAGE_ROW_VALUE_SOURCE_EXTEND_MAX_ROWS_RE = re.compile(
    r"^timing_finish_witness_stage_(\d+)_opening_row_value_source_extend_max_rows$"
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
CUDA_SETUP_INIT_CALLS_KEY = "timing_cuda_setup_init_calls"
CUDA_SETUP_INIT_WAIT_NS_KEY = "timing_cuda_setup_init_wait_ns"
CUDA_SETUP_INIT_MAX_WAIT_NS_KEY = "timing_cuda_setup_init_max_wait_ns"
CUDA_SETUP_CACHE_HITS_KEY = "timing_cuda_setup_cache_hits"
CUDA_SETUP_CACHE_HIT_WAIT_NS_KEY = "timing_cuda_setup_cache_hit_wait_ns"
CUDA_SETUP_CACHE_HIT_MAX_WAIT_NS_KEY = "timing_cuda_setup_cache_hit_max_wait_ns"
CUDA_SETUP_NATIVE_INIT_CALLS_KEY = "timing_cuda_setup_native_init_calls"
CUDA_SETUP_NATIVE_INIT_WAIT_NS_KEY = "timing_cuda_setup_native_init_wait_ns"
CUDA_SETUP_NATIVE_INIT_MAX_WAIT_NS_KEY = "timing_cuda_setup_native_init_max_wait_ns"
CUDA_CURRENT_DEVICE_CALLS_KEY = "timing_cuda_current_device_calls"
CUDA_CURRENT_DEVICE_WAIT_NS_KEY = "timing_cuda_current_device_wait_ns"
CUDA_CURRENT_DEVICE_MAX_WAIT_NS_KEY = "timing_cuda_current_device_max_wait_ns"
CUDA_MEMORY_INFO_CALLS_KEY = "timing_cuda_memory_info_calls"
CUDA_MEMORY_INFO_WAIT_NS_KEY = "timing_cuda_memory_info_wait_ns"
CUDA_MEMORY_INFO_MAX_WAIT_NS_KEY = "timing_cuda_memory_info_max_wait_ns"
CUDA_MALLOC_CALLS_KEY = "timing_cuda_allocator_malloc_calls"
CUDA_MALLOC_WAIT_NS_KEY = "timing_cuda_allocator_malloc_wait_ns"
CUDA_MALLOC_MAX_WAIT_NS_KEY = "timing_cuda_allocator_malloc_max_wait_ns"
CUDA_HOST_REGISTER_WAIT_NS_KEY = "timing_cuda_allocator_host_register_wait_ns"
CUDA_HOST_UNREGISTER_WAIT_NS_KEY = "timing_cuda_allocator_host_unregister_wait_ns"
CUDA_COPY_H2D_BYTES_KEY = "timing_cuda_allocator_copy_h2d_bytes"
CUDA_COPY_H2D_WAIT_NS_KEY = "timing_cuda_allocator_copy_h2d_wait_ns"
CUDA_COPY_H2D_HOT_BYTES_KEY = "timing_cuda_allocator_copy_h2d_hot_bytes"
CUDA_COPY_H2D_HOT_COUNT_KEY = "timing_cuda_allocator_copy_h2d_hot_count"
CUDA_COPY_H2D_HOT_WAIT_NS_KEY = "timing_cuda_allocator_copy_h2d_hot_wait_ns"
CUDA_COPY_H2D_SECOND_HOT_BYTES_KEY = (
    "timing_cuda_allocator_copy_h2d_second_hot_bytes"
)
CUDA_COPY_H2D_SECOND_HOT_COUNT_KEY = (
    "timing_cuda_allocator_copy_h2d_second_hot_count"
)
CUDA_COPY_H2D_SECOND_HOT_WAIT_NS_KEY = (
    "timing_cuda_allocator_copy_h2d_second_hot_wait_ns"
)
CUDA_COPY_D2H_BYTES_KEY = "timing_cuda_allocator_copy_d2h_bytes"
CUDA_COPY_D2H_WAIT_NS_KEY = "timing_cuda_allocator_copy_d2h_wait_ns"
CUDA_COPY_D2H_HOT_BYTES_KEY = "timing_cuda_allocator_copy_d2h_hot_bytes"
CUDA_COPY_D2H_HOT_COUNT_KEY = "timing_cuda_allocator_copy_d2h_hot_count"
CUDA_COPY_D2H_HOT_WAIT_NS_KEY = "timing_cuda_allocator_copy_d2h_hot_wait_ns"
CUDA_EVENT_SYNC_CALLS_KEY = "timing_cuda_allocator_event_synchronize_calls"
CUDA_EVENT_SYNC_BYTES_KEY = "timing_cuda_allocator_event_synchronize_bytes"
CUDA_EVENT_SYNC_MAX_BYTES_KEY = "timing_cuda_allocator_event_synchronize_max_bytes"
CUDA_EVENT_SYNC_WAIT_NS_KEY = "timing_cuda_allocator_event_synchronize_wait_ns"
CUDA_EVENT_SYNC_MAX_WAIT_NS_KEY = (
    "timing_cuda_allocator_event_synchronize_max_wait_ns"
)
CUDA_EVENT_SYNC_HOT_BYTES_KEY = (
    "timing_cuda_allocator_event_synchronize_hot_bytes"
)
CUDA_EVENT_SYNC_HOT_COUNT_KEY = (
    "timing_cuda_allocator_event_synchronize_hot_count"
)
CUDA_EVENT_SYNC_HOT_WAIT_NS_KEY = (
    "timing_cuda_allocator_event_synchronize_hot_wait_ns"
)
CUDA_CACHED_REUSE_COUNT_KEY = "timing_cuda_allocator_cached_reuse_count"
CUDA_PENDING_REUSE_COUNT_KEY = "timing_cuda_allocator_pending_reuse_count"
CUDA_NO_WAIT_BYPASS_COUNT_KEY = "timing_cuda_allocator_no_wait_bypass_count"
CUDA_NO_WAIT_BYPASS_BYTES_KEY = "timing_cuda_allocator_no_wait_bypass_bytes"
CUDA_COPY_SITE_DIRECTIONS = ("h2d", "d2h")
CUDA_COPY_SITE_TOP_RANKS = (1, 2)
CUDA_COPY_SITE_TIMING_RE = re.compile(
    r"^timing_cuda_copy_site_(h2d|d2h)_top_(\d+)_(.+?)_"
    r"(calls|max_bytes|max_wait_ns|avg_wait_per_call_ns|wait_ns|bytes)$"
)
CUDA_COPY_SITE_SUMMARY_FIELDS = tuple(
    field
    for direction in CUDA_COPY_SITE_DIRECTIONS
    for rank in CUDA_COPY_SITE_TOP_RANKS
    for field in (
        f"cuda_copy_site_{direction}_top_{rank}_site",
        f"cuda_copy_site_{direction}_top_{rank}_calls",
        f"cuda_copy_site_{direction}_top_{rank}_bytes",
        f"cuda_copy_site_{direction}_top_{rank}_max_bytes",
        f"cuda_copy_site_{direction}_top_{rank}_wait_ms",
        f"cuda_copy_site_{direction}_top_{rank}_max_wait_ms",
        f"cuda_copy_site_{direction}_top_{rank}_avg_wait_ms",
    )
)
CUDA_COPY_SITE_HEADER = ",".join(CUDA_COPY_SITE_SUMMARY_FIELDS)
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
STAGE_SOURCE_UPLOAD_MS_KEY = "timing_guest_stage_source_upload_ms"
RETAINED_TRACE_ARTIFACT_MS_KEY = "timing_guest_retained_trace_artifact_ms"
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
NSYS_COPY_HOST_REGISTRATION_API_MS_KEY = "nsys_copy_host_registration_api_ms"
NSYS_COPY_HOST_REGISTRATION_HINT_KEY = "nsys_copy_host_registration_hint"
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
NCU_TOP_KERNEL_SEPARATION_HINT_KEY = "ncu_top_kernel_separation_hint"
NCU_DESCRIPTOR_EXPANSION_HINT_KEY = "ncu_descriptor_expansion_hint"
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
PERF_LIVE_STREAM_MESSAGE_SELF_PCT_KEY = "perf_live_stream_message_self_pct"
CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY = (
    "cpu_trace_memcpy_report_storage_hint_pct"
)
CPU_TRACE_MEMCPY_REPORT_STORAGE_TOTAL_PCT_KEY = (
    "cpu_trace_memcpy_report_storage_total_pct"
)
CPU_TRACE_REPORT_STORAGE_STRUCTURAL_TOTAL_PCT_THRESHOLD = 5.0
ROOT_PIPELINE_INPUT_BYTE_LIMIT = 2 * 1024 * 1024 * 1024
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
SEGMENT_COMMIT_MEMORY_DIAGNOSTIC_MS_THRESHOLD = 1000
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
    "profile,input_bytes,total_ms,catalog_ms,eth_input_ms,public_inputs_ms,"
    "plan_ms,framed_guest_input_ms,gpu_memory_preflight_ms,gpu_setup_ms,auxiliary_inputs_ms,"
    "trace_inputs_ms,witness_ms,proof_ms,output_write_ms,summary_ms,"
    "top_level_unattributed_ms,gpu_memory_preflight_pct,gpu_setup_pct,top_level_bottleneck,"
    "constant_material_validation_elapsed_ms,"
    "constant_material_validation_join_wait_ms,constant_material_validation_overlap_hint,"
    "runner_ms,runner_advance_fast_paths,runner_advance_generic_fallbacks,"
    "runner_advance_fast_path_pct,"
    "runner_advance_generic_fallback_shape_top_1_pattern,"
    "runner_advance_generic_fallback_shape_top_1_count,"
    "runner_advance_generic_fallback_shape_top_1_shape,"
    "runner_advance_generic_fallback_shape_top_2_pattern,"
    "runner_advance_generic_fallback_shape_top_2_count,"
    "runner_advance_generic_fallback_shape_top_2_shape,"
    "runner_advance_generic_fallback_shape_top_3_pattern,"
    "runner_advance_generic_fallback_shape_top_3_count,"
    "runner_advance_generic_fallback_shape_top_3_shape,"
    "runner_advance_generic_fallback_shape_top_4_pattern,"
    "runner_advance_generic_fallback_shape_top_4_count,"
    "runner_advance_generic_fallback_shape_top_4_shape,"
    "runner_instruction_cache_hits,runner_instruction_cache_misses,"
    "runner_instruction_cache_hit_pct,runner_instruction_cache_clears,"
    "runner_instruction_cache_fcall_clears,runner_instruction_cache_dma_clears,"
    "runner_instruction_cache_write_invalidation_ranges,"
    "runner_instruction_cache_write_invalidation_skipped_ranges,"
    "runner_instruction_cache_write_invalidation_skip_pct,"
    "runner_instruction_cache_write_invalidation_probes,"
    "runner_instruction_cache_invalidated_entries,"
    "trace_runner_detail_samples,trace_runner_detail_sample_pct,"
    "trace_runner_detail_avg_ns,trace_runner_prepare_instruction_sampled_ns,"
    "trace_runner_pre_boundary_sampled_ns,trace_runner_row_plan_sampled_ns,"
    "trace_runner_cache_policy_sampled_ns,trace_runner_advance_sampled_ns,"
    "trace_runner_advance_setup_sampled_ns,trace_runner_advance_execute_sampled_ns,"
    "trace_runner_advance_report_sampled_ns,"
    "trace_runner_cache_update_sampled_ns,trace_runner_row_count_sampled_ns,"
    "trace_runner_post_boundary_sampled_ns,trace_runner_counter_update_sampled_ns,"
    "trace_runner_timer_bookkeeping_sampled_ns,"
    "trace_runner_detail_hotspot,trace_runner_detail_hotspot_pct,"
    "trace_runner_detail_residual_pct,trace_runner_detail_action_hint,"
    "lowerer_ms,trace_lower_ms,trace_report_ms,"
    "trace_report_apply_ms,trace_unit_summary_ms,trace_non_report_ms,"
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
    "segment_input_gap_ms,segment_input_gap_max_ms,"
    "segment_input_gap_count,segment_input_gap_avg_ms,"
    "stream_commit_residual_ms,segment_receive_wait_ms,"
    "pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,"
    "parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,"
    "parallel_lower_max_reorder,parallel_lower_snapshot_replay_count,"
    "parallel_lower_snapshot_replay_ms,"
    "parallel_lower_report_elided_count,"
    "parallel_lower_stream_segments,parallel_lower_stream_chunks,"
    "parallel_lower_stream_fallbacks,parallel_lower_stream_retained_reports,"
    "owned_streaming_lower_segments,"
    "parallel_lower_stream_chunks_per_segment,"
    "parallel_lower_stream_reports_per_chunk,parallel_lower_stream_shape_hint,"
    "parallel_lower_dispatch_wait_ms,"
    "parallel_lower_stream_start_dispatch_wait_ms,"
    "parallel_lower_stream_chunk_dispatch_wait_ms,"
    "parallel_lower_stream_chunk_process_ms,"
    "parallel_lower_job_receive_wait_ms,"
    "parallel_lower_result_send_wait_ms,"
    "parallel_lower_stream_segment_dispatch_wait_ms,"
    "parallel_lower_stream_finish_dispatch_wait_ms,"
    "parallel_lower_result_receive_wait_ms,"
    "parallel_lower_dispatch_blocked_count,segment_replay_count,trace_reports,trace_report_rows,"
    "main_report_fast_paths,main_report_generic_fallbacks,"
    "main_report_fast_path_pct,"
    "main_report_fcall_result_fast_paths,main_report_load_copy_fast_paths,"
    "main_report_load_sign_extend_fast_paths,"
    "main_report_no_memory_fast_paths,main_report_store_copy_fast_paths,"
    "main_report_simple_copy_fast_paths,main_report_jump_fast_paths,"
    "main_report_generic_fallback_shape_top_1_pattern,"
    "main_report_generic_fallback_shape_top_1_count,"
    "main_report_generic_fallback_shape_top_1_shape,"
    "main_report_generic_fallback_shape_top_2_pattern,"
    "main_report_generic_fallback_shape_top_2_count,"
    "main_report_generic_fallback_shape_top_2_shape,"
    "main_report_generic_fallback_shape_top_3_pattern,"
    "main_report_generic_fallback_shape_top_3_count,"
    "main_report_generic_fallback_shape_top_3_shape,"
    "main_report_generic_fallback_shape_top_4_pattern,"
    "main_report_generic_fallback_shape_top_4_count,"
    "main_report_generic_fallback_shape_top_4_shape,"
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
    "descriptor_wide_rows,device_source_build_ms,descriptor_upload_ms,"
    "device_source_trace_expand_ms,descriptor_upload_bytes,descriptor_bytes_per_row,"
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
    "seed_direct_lift_boundary_c_unavailable,seed_direct_lift_ms,"
    "seed_full_advance_ms,seed_full_advances,"
    "seed_snapshot_runtime_hint,"
    "finish_opening_ms,opening_external_source_ms,"
    "opening_external_source_descriptor_upload_ms,"
    "opening_external_source_descriptor_upload_bytes,"
    "opening_external_source_descriptor_upload_words,"
    "opening_external_source_descriptor_upload_rows,"
    "opening_external_source_trace_expand_ms,"
    "opening_query_units,opening_single_query_units,"
    "opening_queries,opening_max_queries_per_unit,opening_stage_count,"
    "fri_opening_ms,fri_opening_unit_build_ms,fri_opening_layer_tree_ms,"
    "fri_opening_query_ms,fri_opening_fold_ms,"
    "fri_opening_units,fri_opening_layers,fri_opening_queries,"
    "fri_layers_per_unit,fri_queries_per_unit,"
    "fri_transcript_unit_build_ms,fri_transcript_layer_tree_ms,"
    "fri_transcript_fold_ms,fri_transcript_units,fri_transcript_layers,"
    "fri_transcript_layers_per_unit,contribution_segment_ms,"
    "contribution_verify_ms,contribution_challenge_ms,contribution_total_ms,"
    "fri_opening_total_pct,fri_transcript_unit_build_total_pct,"
    "contribution_total_pct,final_proof_timing_hint,"
    "opening_source_shape_hint,"
    "stage_source_upload_ms,retained_trace_artifact_ms,"
    "source_retention_attempts,source_retention_retained,"
    "source_retention_rejected,source_retention_retained_bytes,"
    "source_retention_rejected_bytes,source_retention_max_retained_bytes,"
    "source_retention_max_rejected_bytes,source_retention_limit_bytes,"
    "source_retention_rejected_total_exceeds_device_memory,"
    "source_retention_max_rejected_exceeds_device_memory,"
    "opening_source_rebuild_hint,opening_external_source_descriptor_action_hint,"
    "opening_row_value_device_rows,"
    "opening_row_value_source_rows,opening_row_value_words,opening_row_value_bytes,"
    "opening_row_value_source_extend_calls,"
    "opening_row_value_source_extend_max_rows,"
    "opening_row_value_source_extend_rows_per_call,"
    "opening_row_value_source_extend_ms_per_call,"
    "opening_row_value_source_extend_ms,"
    "opening_row_value_source_download_ms,"
    "opening_row_value_device_download_ms,"
    "opening_row_value_source_extend_pct,opening_source_row_value_action_hint,"
    "opening_row_dedup_input_rows,opening_row_dedup_unique_rows,"
    "opening_row_dedup_elided_rows,opening_row_dedup_elided_pct,"
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
    "opening_row_value_device_batch_stage_count,"
    "opening_row_value_device_batch_max_stage,"
    "opening_row_value_device_batch_stage_sum,"
    "opening_row_value_device_batch_unattributed,"
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
    "perf_live_stream_message_self_pct,cpu_trace_live_stream_action_hint,"
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
    "row_shape_top_1_pattern,row_shape_top_1_count,row_shape_top_1_shape,"
    "row_shape_top_2_pattern,row_shape_top_2_count,row_shape_top_2_shape,"
    "row_shape_top_3_pattern,row_shape_top_3_count,row_shape_top_3_shape,"
    "row_shape_top_4_pattern,row_shape_top_4_count,row_shape_top_4_shape,"
    "trace_precompile_action_hint,"
    "copy_memory_source_rows,copy_memory_source_row_pct,"
    "copy_indirect_memory_rows,copy_indirect_memory_row_pct,"
    "copy_register_store_rows,copy_memory_store_rows,"
    "copy_no_store_rows,copy_no_memory_rows,copy_no_memory_row_pct,"
    "trace_copy_shape_hint,trace_copy_action_hint,"
    "copy_source_memory_read_ms,copy_source_indirect_read_ms,"
    "copy_source_memory_read_pct,copy_source_indirect_read_pct,"
    "copy_source_memory_reads,copy_source_indirect_reads,"
    "copy_source_memory_read_sampled_ns,copy_source_indirect_read_sampled_ns,"
    "copy_source_memory_read_avg_sample_ns,copy_source_indirect_read_avg_sample_ns,"
    "trace_copy_source_action_hint,"
    "trace_report_validation_ms,trace_report_emit_ms,trace_descriptor_ms,"
    "trace_report_lowering_ms,trace_report_row_validation_ms,"
    "trace_report_row_validation_timer_bookkeeping_ms,"
    "trace_report_memory_columns_ms,trace_report_source_values_ms,"
    "trace_report_source_a_value_ms,trace_report_source_b_value_ms,"
    "trace_report_source_value_record_ms,"
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
    "trace_report_source_value_record_lowerer_share_ms,"
    "trace_report_source_values_residual_lowerer_share_ms,"
    "trace_report_precompile_memory_lowerer_share_ms,"
    "trace_report_instruction_result_lowerer_share_ms,"
    "trace_report_next_pc_lowerer_share_ms,"
    "trace_report_register_access_lowerer_share_ms,"
    "trace_report_memory_access_lowerer_share_ms,"
    "trace_report_store_apply_lowerer_share_ms,"
    "trace_report_row_validation_timer_bookkeeping_lowerer_share_ms,"
    "trace_report_row_validation_residual_lowerer_share_ms,"
    "trace_report_visit_lowerer_share_ms,trace_report_descriptor_lowerer_share_ms,"
    "trace_report_detail_hotspot,trace_report_detail_hotspot_pct,"
    "trace_report_detail_action_hint,"
    "trace_report_row_validation_hotspot,trace_report_row_validation_hotspot_pct,"
    "trace_report_row_validation_explained_pct,trace_report_row_validation_residual_pct,"
    "trace_report_source_values_lookup_pct,trace_report_source_values_record_pct,"
    "trace_report_source_values_residual_pct,"
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
    "cuda_setup_init_calls,cuda_setup_init_wait_ms,"
    "cuda_setup_init_max_wait_ms,cuda_setup_cache_hits,"
    "cuda_setup_cache_hit_wait_ms,cuda_setup_cache_hit_max_wait_ms,"
    "cuda_setup_native_init_calls,cuda_setup_native_init_wait_ms,"
    "cuda_setup_native_init_max_wait_ms,"
    "cuda_current_device_calls,cuda_current_device_wait_ms,"
    "cuda_current_device_max_wait_ms,"
    "cuda_memory_info_calls,cuda_memory_info_wait_ms,"
    "cuda_memory_info_max_wait_ms,"
    "cuda_allocator_malloc_calls,cuda_allocator_malloc_wait_ms,"
    "cuda_allocator_malloc_max_wait_ms,"
    "cuda_allocator_h2d_hot_bytes,cuda_allocator_h2d_hot_count,"
    "cuda_allocator_h2d_hot_wait_ms,cuda_allocator_h2d_hot_wait_pct,"
    "cuda_allocator_h2d_second_hot_bytes,cuda_allocator_h2d_second_hot_count,"
    "cuda_allocator_h2d_second_hot_wait_ms,"
    "cuda_allocator_h2d_second_hot_wait_pct,"
    "cuda_allocator_d2h_bytes,cuda_allocator_d2h_wait_ms,"
    "cuda_allocator_d2h_hot_bytes,cuda_allocator_d2h_hot_count,"
    "cuda_allocator_d2h_hot_wait_ms,cuda_allocator_d2h_hot_wait_pct,"
    "cuda_allocator_d2h_action_hint,"
    "cuda_allocator_event_sync_calls,cuda_allocator_event_sync_bytes,"
    "cuda_allocator_event_sync_max_bytes,cuda_allocator_event_sync_wait_ms,"
    "cuda_allocator_event_sync_max_wait_ms,"
    "cuda_allocator_event_sync_hot_bytes,cuda_allocator_event_sync_hot_count,"
    "cuda_allocator_event_sync_hot_wait_ms,"
    "cuda_allocator_event_sync_hot_wait_pct,"
    "cuda_allocator_cached_reuse_count,cuda_allocator_pending_reuse_count,"
    "cuda_allocator_no_wait_bypass_count,cuda_allocator_no_wait_bypass_bytes,"
    "cuda_allocator_reuse_action_hint,"
    "cuda_host_register_wait_ms,cuda_host_unregister_wait_ms,"
    "cuda_host_registration_total_wait_ms,cuda_h2d_bytes,"
    f"{CUDA_COPY_SITE_HEADER},"
    "cuda_copy_site_action_hint,"
    "cuda_transfer_action_hint,"
    "data_residency_action_hint,"
    "copy_summary_gpu_residency_hint,copy_summary_h2d_bulk_app_frame_hint,"
    "copy_summary_small_d2h_batching_hint,"
    "copy_summary_cuda_api_backtrace_hint,"
    "copy_summary_host_registration_api_ms,copy_summary_host_registration_hint,"
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
    "ncu_top_kernel_separation_hint,ncu_descriptor_expansion_hint,"
    "segment_commit_cuda_memory_total_bytes,"
    "segment_commit_cuda_memory_initial_free_bytes,"
    "segment_commit_cuda_memory_effective_free_bytes,"
    "segment_commit_cuda_memory_min_free_bytes,"
    "segment_commit_cuda_memory_sample_ms,"
    "segment_commit_cuda_memory_sample_count,"
    "segment_commit_cuda_allocator_initial_cached_bytes,"
    "segment_commit_cuda_allocator_effective_cached_bytes,"
    "segment_commit_cuda_memory_min_free_pct,"
    "segment_commit_memory_pressure_hint,"
    "segment_commit_memory_diagnostic_hint,"
    "descriptor_retention_attempts,descriptor_retention_retained,"
    "descriptor_retention_rejected,descriptor_retention_retained_bytes,"
    "descriptor_retention_rejected_bytes,descriptor_retention_limit_bytes,"
    "external_op_row_pct,copy_row_pct,trace_shape_row_mix_hint,"
    "external_op_row_lower_ms,copy_row_lower_ms,"
    "external_op_row_lower_ns_per_row,copy_row_lower_ns_per_row,"
    "external_op_row_lower_pct,copy_row_lower_pct,trace_shape_duration_hint,"
    "trace_shape_unit_cost_hint,"
    "trace_report_source_value_record_ns_per_row,"
    "trace_report_source_values_residual_ns_per_row,"
    "trace_report_row_validation_timer_bookkeeping_ns_per_row,"
    "trace_report_row_validation_residual_ns_per_row,"
    "trace_report_visit_residual_ns_per_row,"
    "trace_report_descriptor_ns_per_row,"
    "external_op_runs,external_op_avg_run,external_op_max_run,"
    "copy_runs,copy_avg_run,copy_max_run,trace_shape_run_hint,"
    "trace_pipeline_action_hint,performance_focus_hint,trace_shape_profile_hint,"
    "fri_opening_unit_build_scope_pct,"
    "fri_opening_layer_tree_nested_pct,fri_opening_query_nested_pct,"
    "fri_opening_fold_nested_pct,fri_opening_known_nested_ms,"
    "fri_opening_known_nested_pct,fri_opening_unit_build_residual_ms,"
    "fri_opening_unit_build_residual_pct,fri_opening_scope_hint"
)
AGGREGATE_MEAN_PROFILE_COLUMNS = (
    "witness_ms",
    "top_level_unattributed_ms",
    "runner_ms",
    "runner_advance_fast_paths",
    "runner_advance_generic_fallbacks",
    "runner_advance_fast_path_pct",
    "runner_instruction_cache_hits",
    "runner_instruction_cache_misses",
    "runner_instruction_cache_hit_pct",
    "runner_instruction_cache_clears",
    "runner_instruction_cache_fcall_clears",
    "runner_instruction_cache_dma_clears",
    "runner_instruction_cache_write_invalidation_ranges",
    "runner_instruction_cache_write_invalidation_skipped_ranges",
    "runner_instruction_cache_write_invalidation_skip_pct",
    "runner_instruction_cache_write_invalidation_probes",
    "runner_instruction_cache_invalidated_entries",
    "trace_runner_detail_samples",
    "trace_runner_detail_sample_pct",
    "trace_runner_detail_avg_ns",
    "trace_runner_prepare_instruction_sampled_ns",
    "trace_runner_pre_boundary_sampled_ns",
    "trace_runner_row_plan_sampled_ns",
    "trace_runner_cache_policy_sampled_ns",
    "trace_runner_advance_sampled_ns",
    "trace_runner_advance_setup_sampled_ns",
    "trace_runner_advance_execute_sampled_ns",
    "trace_runner_advance_report_sampled_ns",
    "trace_runner_cache_update_sampled_ns",
    "trace_runner_row_count_sampled_ns",
    "trace_runner_post_boundary_sampled_ns",
    "trace_runner_counter_update_sampled_ns",
    "trace_runner_timer_bookkeeping_sampled_ns",
    "trace_runner_detail_hotspot_pct",
    "trace_runner_detail_residual_pct",
    "lowerer_ms",
    "trace_lower_ms",
    "stream_elapsed_ms",
    "stream_worker_ms",
    "pending_receive_wait_ms",
    "parallel_lower_workers",
    "parallel_lower_dispatched",
    "parallel_lower_received",
    "parallel_lower_emitted",
    "parallel_lower_stream_segments",
    "parallel_lower_stream_chunks",
    "parallel_lower_stream_fallbacks",
    "owned_streaming_lower_segments",
    "parallel_lower_dispatch_wait_ms",
    "parallel_lower_stream_chunk_process_ms",
    "parallel_lower_result_receive_wait_ms",
    "segment_commit_ms",
    "main_report_fast_path_pct",
    "finish_opening_ms",
    "opening_external_source_ms",
    "opening_external_source_descriptor_upload_ms",
    "opening_row_value_source_extend_ms",
    "retained_parent_checkpoint_path_ms",
    "leaf_kernel_ms",
    "direct_d2h_wait_ms",
    "descriptor_upload_ms",
    "cuda_host_register_wait_ms",
    "cuda_host_unregister_wait_ms",
    "cuda_host_registration_total_wait_ms",
    "copy_summary_host_registration_api_ms",
    "cuda_h2d_bytes",
    "cuda_allocator_d2h_wait_ms",
    "proof_12s_gap_ms",
)
AGGREGATE_MEAN_HEADER_SUFFIX = "," + ",".join(
    f"{column}_mean" for column in AGGREGATE_MEAN_PROFILE_COLUMNS
)
AGGREGATE_HEADER = (
    "aggregate,total_count,valid_total_count,total_min_ms,total_mean_ms,"
    "total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier,"
    "dominant_trace_pipeline_action_hint,trace_pipeline_action_consensus,"
    "dominant_trace_structure_hint,trace_structure_consensus,"
    "dominant_cuda_transfer_action_hint,cuda_transfer_action_consensus,"
    "dominant_segment_commit_memory_pressure_hint,"
    "segment_commit_memory_pressure_consensus,"
    "dominant_segment_commit_memory_diagnostic_hint,"
    "segment_commit_memory_diagnostic_consensus,"
    "dominant_trace_runner_detail_hotspot,"
    "trace_runner_detail_hotspot_consensus,"
    "dominant_trace_runner_detail_action_hint,"
    "trace_runner_detail_action_consensus"
) + AGGREGATE_MEAN_HEADER_SUFFIX
AGGREGATE_BY_INPUT_BYTES_HEADER = (
    "aggregate_by_input_bytes,input_bytes,total_count,valid_total_count,total_min_ms,"
    "total_mean_ms,total_median_ms,total_max_ms,sample_spread_pct,close_samples,"
    "max_outlier,dominant_trace_pipeline_action_hint,trace_pipeline_action_consensus,"
    "dominant_trace_structure_hint,trace_structure_consensus,"
    "dominant_cuda_transfer_action_hint,cuda_transfer_action_consensus,"
    "dominant_segment_commit_memory_pressure_hint,"
    "segment_commit_memory_pressure_consensus,"
    "dominant_segment_commit_memory_diagnostic_hint,"
    "segment_commit_memory_diagnostic_consensus,"
    "dominant_trace_runner_detail_hotspot,"
    "trace_runner_detail_hotspot_consensus,"
    "dominant_trace_runner_detail_action_hint,"
    "trace_runner_detail_action_consensus"
) + AGGREGATE_MEAN_HEADER_SUFFIX
CLOSE_SAMPLE_SPREAD_PCT = 5.0
OUTLIER_RATIO_THRESHOLD = 1.5

TIMING_KEYS = {
    INPUT_BYTES_KEY,
    TOTAL_MS_KEY,
    CATALOG_MS_KEY,
    ETH_INPUT_MS_KEY,
    PUBLIC_INPUTS_MS_KEY,
    PLAN_MS_KEY,
    FRAMED_GUEST_INPUT_MS_KEY,
    GPU_MEMORY_PREFLIGHT_MS_KEY,
    GPU_SETUP_MS_KEY,
    AUXILIARY_INPUTS_MS_KEY,
    TRACE_INPUTS_MS_KEY,
    WITNESS_MS_KEY,
    PROOF_MS_KEY,
    OUTPUT_WRITE_MS_KEY,
    SUMMARY_MS_KEY,
    CONSTANT_MATERIAL_VALIDATION_ELAPSED_MS_KEY,
    CONSTANT_MATERIAL_VALIDATION_JOIN_WAIT_MS_KEY,
    RUNNER_MS_KEY,
    RUNNER_DETAIL_SAMPLES_KEY,
    RUNNER_ADVANCE_FAST_PATHS_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACKS_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_1_PATTERN_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_1_COUNT_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_2_PATTERN_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_2_COUNT_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_3_PATTERN_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_3_COUNT_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_4_PATTERN_KEY,
    RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_4_COUNT_KEY,
    RUNNER_CACHE_HITS_KEY,
    RUNNER_CACHE_MISSES_KEY,
    RUNNER_CACHE_CLEARS_KEY,
    RUNNER_CACHE_FCALL_CLEARS_KEY,
    RUNNER_CACHE_DMA_CLEARS_KEY,
    RUNNER_CACHE_INVALIDATION_RANGES_KEY,
    RUNNER_CACHE_INVALIDATION_SKIPPED_RANGES_KEY,
    RUNNER_CACHE_INVALIDATION_PROBES_KEY,
    RUNNER_CACHE_INVALIDATED_ENTRIES_KEY,
    RUNNER_DETAIL_SAMPLED_NS_KEY,
    RUNNER_PREPARE_INSTRUCTION_SAMPLED_NS_KEY,
    RUNNER_PRE_BOUNDARY_SAMPLED_NS_KEY,
    RUNNER_ROW_PLAN_SAMPLED_NS_KEY,
    RUNNER_CACHE_POLICY_SAMPLED_NS_KEY,
    RUNNER_ADVANCE_SAMPLED_NS_KEY,
    RUNNER_ADVANCE_SETUP_SAMPLED_NS_KEY,
    RUNNER_ADVANCE_EXECUTE_SAMPLED_NS_KEY,
    RUNNER_ADVANCE_REPORT_SAMPLED_NS_KEY,
    RUNNER_CACHE_UPDATE_SAMPLED_NS_KEY,
    RUNNER_ROW_COUNT_SAMPLED_NS_KEY,
    RUNNER_POST_BOUNDARY_SAMPLED_NS_KEY,
    RUNNER_COUNTER_UPDATE_SAMPLED_NS_KEY,
    RUNNER_TIMER_BOOKKEEPING_SAMPLED_NS_KEY,
    LOWERER_MS_KEY,
    TRACE_LOWER_MS_KEY,
    TRACE_REPORT_MS_KEY,
    STREAM_ELAPSED_MS_KEY,
    STREAM_WORKER_MS_KEY,
    SEGMENT_COMMIT_MS_KEY,
    SEGMENT_COMMIT_ATTEMPT_MS_KEY,
    SEGMENT_COMMIT_OOM_RETRY_MS_KEY,
    SEGMENT_INPUT_GAP_MS_KEY,
    SEGMENT_INPUT_GAP_MAX_MS_KEY,
    SEGMENT_INPUT_GAP_COUNT_KEY,
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
    SEGMENT_COMMIT_CUDA_MEMORY_SAMPLE_MS_KEY,
    SEGMENT_COMMIT_CUDA_MEMORY_SAMPLE_COUNT_KEY,
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
    PARALLEL_LOWER_SNAPSHOT_REPLAY_MS_KEY,
    PARALLEL_LOWER_REPORT_ELIDED_KEY,
    PARALLEL_LOWER_STREAM_SEGMENTS_KEY,
    PARALLEL_LOWER_STREAM_CHUNKS_KEY,
    PARALLEL_LOWER_STREAM_FALLBACKS_KEY,
    PARALLEL_LOWER_STREAM_RETAINED_REPORTS_KEY,
    OWNED_STREAMING_LOWER_SEGMENTS_KEY,
    PARALLEL_LOWER_DISPATCH_WAIT_MS_KEY,
    PARALLEL_LOWER_STREAM_START_DISPATCH_WAIT_MS_KEY,
    PARALLEL_LOWER_STREAM_CHUNK_DISPATCH_WAIT_MS_KEY,
    PARALLEL_LOWER_STREAM_CHUNK_PROCESS_MS_KEY,
    PARALLEL_LOWER_JOB_RECEIVE_WAIT_MS_KEY,
    PARALLEL_LOWER_RESULT_SEND_WAIT_MS_KEY,
    PARALLEL_LOWER_STREAM_SEGMENT_DISPATCH_WAIT_MS_KEY,
    PARALLEL_LOWER_STREAM_FINISH_DISPATCH_WAIT_MS_KEY,
    PARALLEL_LOWER_RESULT_RECEIVE_WAIT_MS_KEY,
    PARALLEL_LOWER_DISPATCH_BLOCKED_KEY,
    SEGMENT_REPLAY_COUNT_KEY,
    TRACE_REPORTS_KEY,
    TRACE_REPORT_ROWS_KEY,
    MAIN_REPORT_FAST_PATHS_KEY,
    MAIN_REPORT_GENERIC_FALLBACKS_KEY,
    MAIN_REPORT_FCALL_RESULT_FAST_PATHS_KEY,
    MAIN_REPORT_LOAD_COPY_FAST_PATHS_KEY,
    MAIN_REPORT_LOAD_SIGN_EXTEND_FAST_PATHS_KEY,
    MAIN_REPORT_NO_MEMORY_FAST_PATHS_KEY,
    MAIN_REPORT_STORE_COPY_FAST_PATHS_KEY,
    MAIN_REPORT_SIMPLE_COPY_FAST_PATHS_KEY,
    MAIN_REPORT_JUMP_FAST_PATHS_KEY,
    MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_1_PATTERN_KEY,
    MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_1_COUNT_KEY,
    MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_2_PATTERN_KEY,
    MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_2_COUNT_KEY,
    MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_3_PATTERN_KEY,
    MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_3_COUNT_KEY,
    MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_4_PATTERN_KEY,
    MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_4_COUNT_KEY,
    TRACE_REPORT_CHUNK_SENT_KEY,
    TRACE_REPORT_CHUNK_RECEIVED_KEY,
    TRACE_REPORT_CHUNK_REPORTS_KEY,
    TRACE_REPORT_CHUNK_ROWS_KEY,
    TRACE_REPORT_CHUNK_MAX_QUEUED_KEY,
    TRACE_REPORT_VALIDATION_MS_KEY,
    TRACE_REPORT_APPLY_MS_KEY,
    TRACE_UNIT_SUMMARY_MS_KEY,
    TRACE_REPORT_LOWERING_MS_KEY,
    TRACE_REPORT_ROW_VALIDATION_MS_KEY,
    TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_MS_KEY,
    TRACE_REPORT_MEMORY_COLUMNS_MS_KEY,
    TRACE_REPORT_SOURCE_VALUES_MS_KEY,
    TRACE_REPORT_SOURCE_A_VALUE_MS_KEY,
    TRACE_REPORT_SOURCE_B_VALUE_MS_KEY,
    TRACE_REPORT_SOURCE_VALUE_RECORD_MS_KEY,
    TRACE_REPORT_SOURCE_IMMEDIATE_READ_MS_KEY,
    TRACE_REPORT_SOURCE_REGISTER_READ_MS_KEY,
    TRACE_REPORT_SOURCE_MEMORY_READ_MS_KEY,
    TRACE_REPORT_SOURCE_INDIRECT_READ_MS_KEY,
    TRACE_REPORT_SOURCE_LAST_C_READ_MS_KEY,
    TRACE_COPY_SOURCE_MEMORY_READ_MS_KEY,
    TRACE_COPY_SOURCE_INDIRECT_READ_MS_KEY,
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
    TRACE_COPY_SOURCE_MEMORY_READS_KEY,
    TRACE_COPY_SOURCE_INDIRECT_READS_KEY,
    TRACE_COPY_SOURCE_MEMORY_READ_SAMPLED_NS_KEY,
    TRACE_COPY_SOURCE_INDIRECT_READ_SAMPLED_NS_KEY,
    TRACE_COPY_SOURCE_MEMORY_READ_AVG_SAMPLE_NS_KEY,
    TRACE_COPY_SOURCE_INDIRECT_READ_AVG_SAMPLE_NS_KEY,
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
    TRACE_ROW_SHAPE_TOP_1_PATTERN_KEY,
    TRACE_ROW_SHAPE_TOP_1_COUNT_KEY,
    TRACE_ROW_SHAPE_TOP_2_PATTERN_KEY,
    TRACE_ROW_SHAPE_TOP_2_COUNT_KEY,
    TRACE_ROW_SHAPE_TOP_3_PATTERN_KEY,
    TRACE_ROW_SHAPE_TOP_3_COUNT_KEY,
    TRACE_ROW_SHAPE_TOP_4_PATTERN_KEY,
    TRACE_ROW_SHAPE_TOP_4_COUNT_KEY,
    TRACE_SHAPE_SAMPLES_KEY,
    TRACE_SHAPE_SAMPLE_ROWS_KEY,
    TRACE_REPORT_DETAIL_SAMPLES_KEY,
    TRACE_REPORT_SAMPLED_NS_KEY,
    TRACE_REPORT_LOWERING_SAMPLED_NS_KEY,
    TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY,
    TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_SAMPLED_NS_KEY,
    TRACE_REPORT_MEMORY_COLUMNS_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY,
    TRACE_REPORT_SOURCE_VALUE_RECORD_SAMPLED_NS_KEY,
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
    DEVICE_SOURCE_BUILD_MS_KEY,
    DESCRIPTOR_UPLOAD_MS_KEY,
    DEVICE_SOURCE_TRACE_EXPAND_MS_KEY,
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
    SEED_DIRECT_LIFT_MS_KEY,
    SEED_FULL_ADVANCE_MS_KEY,
    SEED_FULL_ADVANCES_KEY,
    FINISH_OPENING_MS_KEY,
    OPENING_EXTERNAL_SOURCE_MS_KEY,
    OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_MS_KEY,
    OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_BYTES_KEY,
    OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_WORDS_KEY,
    OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_ROWS_KEY,
    OPENING_EXTERNAL_SOURCE_TRACE_EXPAND_MS_KEY,
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
    STAGE_SOURCE_UPLOAD_MS_KEY,
    RETAINED_TRACE_ARTIFACT_MS_KEY,
    OPENING_ROW_VALUE_DEVICE_ROWS_KEY,
    OPENING_ROW_VALUE_SOURCE_ROWS_KEY,
    OPENING_ROW_VALUE_WORDS_KEY,
    OPENING_ROW_VALUE_BYTES_KEY,
    OPENING_ROW_VALUE_SOURCE_EXTEND_CALLS_KEY,
    OPENING_ROW_VALUE_SOURCE_EXTEND_MAX_ROWS_KEY,
    OPENING_ROW_VALUE_SOURCE_EXTEND_MS_KEY,
    OPENING_ROW_VALUE_SOURCE_DOWNLOAD_MS_KEY,
    OPENING_ROW_VALUE_DEVICE_DOWNLOAD_MS_KEY,
    OPENING_ROW_DEDUP_INPUT_ROWS_KEY,
    OPENING_ROW_DEDUP_UNIQUE_ROWS_KEY,
    OPENING_ROW_DEDUP_ELIDED_ROWS_KEY,
    FRI_OPENING_MS_KEY,
    FRI_OPENING_UNIT_BUILD_MS_KEY,
    FRI_OPENING_LAYER_TREE_MS_KEY,
    FRI_OPENING_QUERY_MS_KEY,
    FRI_OPENING_FOLD_MS_KEY,
    FRI_OPENING_UNIT_COUNT_KEY,
    FRI_OPENING_LAYER_COUNT_KEY,
    FRI_OPENING_QUERY_COUNT_KEY,
    FRI_TRANSCRIPT_UNIT_BUILD_MS_KEY,
    FRI_TRANSCRIPT_LAYER_TREE_MS_KEY,
    FRI_TRANSCRIPT_FOLD_MS_KEY,
    FRI_TRANSCRIPT_UNIT_COUNT_KEY,
    FRI_TRANSCRIPT_LAYER_COUNT_KEY,
    CONTRIBUTION_SEGMENT_MS_KEY,
    CONTRIBUTION_VERIFY_MS_KEY,
    CONTRIBUTION_CHALLENGE_MS_KEY,
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
    CUDA_SETUP_INIT_CALLS_KEY,
    CUDA_SETUP_INIT_WAIT_NS_KEY,
    CUDA_SETUP_INIT_MAX_WAIT_NS_KEY,
    CUDA_SETUP_CACHE_HITS_KEY,
    CUDA_SETUP_CACHE_HIT_WAIT_NS_KEY,
    CUDA_SETUP_CACHE_HIT_MAX_WAIT_NS_KEY,
    CUDA_SETUP_NATIVE_INIT_CALLS_KEY,
    CUDA_SETUP_NATIVE_INIT_WAIT_NS_KEY,
    CUDA_SETUP_NATIVE_INIT_MAX_WAIT_NS_KEY,
    CUDA_CURRENT_DEVICE_CALLS_KEY,
    CUDA_CURRENT_DEVICE_WAIT_NS_KEY,
    CUDA_CURRENT_DEVICE_MAX_WAIT_NS_KEY,
    CUDA_MEMORY_INFO_CALLS_KEY,
    CUDA_MEMORY_INFO_WAIT_NS_KEY,
    CUDA_MEMORY_INFO_MAX_WAIT_NS_KEY,
    CUDA_MALLOC_CALLS_KEY,
    CUDA_MALLOC_WAIT_NS_KEY,
    CUDA_MALLOC_MAX_WAIT_NS_KEY,
    CUDA_HOST_REGISTER_WAIT_NS_KEY,
    CUDA_HOST_UNREGISTER_WAIT_NS_KEY,
    CUDA_COPY_H2D_BYTES_KEY,
    CUDA_COPY_H2D_WAIT_NS_KEY,
    CUDA_COPY_H2D_HOT_BYTES_KEY,
    CUDA_COPY_H2D_HOT_COUNT_KEY,
    CUDA_COPY_H2D_HOT_WAIT_NS_KEY,
    CUDA_COPY_H2D_SECOND_HOT_BYTES_KEY,
    CUDA_COPY_H2D_SECOND_HOT_COUNT_KEY,
    CUDA_COPY_H2D_SECOND_HOT_WAIT_NS_KEY,
    CUDA_COPY_D2H_BYTES_KEY,
    CUDA_COPY_D2H_WAIT_NS_KEY,
    CUDA_COPY_D2H_HOT_BYTES_KEY,
    CUDA_COPY_D2H_HOT_COUNT_KEY,
    CUDA_COPY_D2H_HOT_WAIT_NS_KEY,
    CUDA_EVENT_SYNC_CALLS_KEY,
    CUDA_EVENT_SYNC_BYTES_KEY,
    CUDA_EVENT_SYNC_MAX_BYTES_KEY,
    CUDA_EVENT_SYNC_WAIT_NS_KEY,
    CUDA_EVENT_SYNC_MAX_WAIT_NS_KEY,
    CUDA_EVENT_SYNC_HOT_BYTES_KEY,
    CUDA_EVENT_SYNC_HOT_COUNT_KEY,
    CUDA_EVENT_SYNC_HOT_WAIT_NS_KEY,
    CUDA_CACHED_REUSE_COUNT_KEY,
    CUDA_PENDING_REUSE_COUNT_KEY,
    CUDA_NO_WAIT_BYPASS_COUNT_KEY,
    CUDA_NO_WAIT_BYPASS_BYTES_KEY,
    DESCRIPTOR_RETENTION_ATTEMPTS_KEY,
    DESCRIPTOR_RETENTION_RETAINED_KEY,
    DESCRIPTOR_RETENTION_REJECTED_KEY,
    DESCRIPTOR_RETENTION_RETAINED_BYTES_KEY,
    DESCRIPTOR_RETENTION_REJECTED_BYTES_KEY,
    DESCRIPTOR_RETENTION_LIMIT_BYTES_KEY,
}


def compact_csv_token(value: str) -> str:
    return value.replace(",", "|").replace(" ", "_")


def csv_cell(value: object) -> str:
    text = str(value).replace("\r", " ").replace("\n", " ")
    if "," in text or '"' in text:
        return '"' + text.replace('"', '""') + '"'
    return text


def cuda_copy_site_site_key(direction: str, rank: int) -> str:
    return f"cuda_copy_site_{direction}_top_{rank}_site"


def cuda_copy_site_numeric_key(direction: str, rank: int, field: str) -> str:
    return f"cuda_copy_site_{direction}_top_{rank}_{field}"


def cuda_copy_site_summary_fields(values: dict[str, int | str]) -> str:
    fields: list[str] = []
    for direction in CUDA_COPY_SITE_DIRECTIONS:
        for rank in CUDA_COPY_SITE_TOP_RANKS:
            site = values.get(cuda_copy_site_site_key(direction, rank), "none")
            calls = values.get(cuda_copy_site_numeric_key(direction, rank, "calls"), 0)
            bytes_value = values.get(
                cuda_copy_site_numeric_key(direction, rank, "bytes"), 0
            )
            max_bytes = values.get(
                cuda_copy_site_numeric_key(direction, rank, "max_bytes"), 0
            )
            wait_ms = (
                values.get(cuda_copy_site_numeric_key(direction, rank, "wait_ns"), 0)
                / 1_000_000.0
            )
            max_wait_ms = (
                values.get(
                    cuda_copy_site_numeric_key(direction, rank, "max_wait_ns"), 0
                )
                / 1_000_000.0
            )
            avg_wait_ms = (
                values.get(
                    cuda_copy_site_numeric_key(
                        direction, rank, "avg_wait_per_call_ns"
                    ),
                    0,
                )
                / 1_000_000.0
            )
            fields.extend(
                [
                    csv_cell(site),
                    str(calls),
                    str(bytes_value),
                    str(max_bytes),
                    f"{wait_ms:.3f}",
                    f"{max_wait_ms:.3f}",
                    f"{avg_wait_ms:.3f}",
                ]
            )
    return ",".join(fields)


def parse_timing_log(text: str) -> dict[str, int | str]:
    values: dict[str, int | str] = {}
    nsys_copy_block = None
    nsys_copy_backtrace_block = None
    nsys_kernel_block = None
    nsys_kernel_idle_gap_block = None
    ncu_metric_quality_block = None
    ncu_kernel_metric_block = None
    ncu_occupancy_block = None
    ncu_descriptor_expansion_block = None
    ncu_kernel_separation_block = None
    ncu_top_kernel: str | None = None
    ncu_top_duration_ms = -1.0
    ncu_top_kernel_limits: dict[str, str] = {}
    ncu_top_kernel_separation_hints: dict[str, str] = {}
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
            ncu_descriptor_expansion_block = None
            ncu_kernel_separation_block = None
            continue
        if stripped == "cuda_api_backtrace_hint":
            nsys_copy_backtrace_block = stripped
            nsys_copy_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            ncu_descriptor_expansion_block = None
            ncu_kernel_separation_block = None
            continue
        if stripped == "stream_idle_gap_hotspots":
            nsys_kernel_idle_gap_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            ncu_descriptor_expansion_block = None
            ncu_kernel_separation_block = None
            continue
        if stripped == "cuda_graph_fusion_separation_triage":
            nsys_kernel_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            ncu_descriptor_expansion_block = None
            ncu_kernel_separation_block = None
            continue
        if stripped == "metric_collection_quality":
            ncu_metric_quality_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            ncu_descriptor_expansion_block = None
            ncu_kernel_separation_block = None
            continue
        if stripped == "kernel_metric_summary":
            ncu_kernel_metric_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_occupancy_block = None
            ncu_descriptor_expansion_block = None
            ncu_kernel_separation_block = None
            continue
        if stripped == "occupancy_limits":
            ncu_occupancy_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_descriptor_expansion_block = None
            ncu_kernel_separation_block = None
            continue
        if stripped == "descriptor_expansion_shape_candidates":
            ncu_descriptor_expansion_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            ncu_kernel_separation_block = None
            continue
        if stripped == "kernel_separation_candidates":
            ncu_kernel_separation_block = stripped
            nsys_copy_block = None
            nsys_copy_backtrace_block = None
            nsys_kernel_block = None
            nsys_kernel_idle_gap_block = None
            ncu_metric_quality_block = None
            ncu_kernel_metric_block = None
            ncu_occupancy_block = None
            ncu_descriptor_expansion_block = None
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
            elif len(row) >= 2 and row[0].strip() == "host_registration_api_ms":
                try:
                    values[NSYS_COPY_HOST_REGISTRATION_API_MS_KEY] = str(
                        float(row[1].strip())
                    )
                except ValueError:
                    pass
            elif len(row) >= 2 and row[0].strip() == "host_registration_hint":
                values[NSYS_COPY_HOST_REGISTRATION_HINT_KEY] = row[1].strip()
            elif (
                len(row) >= 2
                and row[0].strip() == "h2d_bulk_app_frame_hint"
            ):
                hint = row[1].strip()
                detail = ",".join(row[2:]).strip() if len(row) >= 3 else ""
                if (
                    hint == "reuse_device_source_for_hot_frame"
                    and (
                        "guest_pc_trace_backend::record_device_source_build_duration"
                        in detail
                        or "build_guest_pc_trace_stage_source_devices_from_device_material_timing"
                        in detail
                    )
                ):
                    values[NSYS_COPY_TRACE_DESCRIPTOR_RESIDENCY_PIPELINE_KEY] = 1
                    values[NSYS_COPY_H2D_BULK_APP_FRAME_HINT_KEY] = compact_csv_token(
                        detail
                    )
                elif values.get(NSYS_COPY_H2D_BULK_APP_FRAME_HINT_KEY, "none") in {
                    "",
                    "none",
                }:
                    values[NSYS_COPY_H2D_BULK_APP_FRAME_HINT_KEY] = hint or "none"
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
                    values[NCU_TOP_KERNEL_KEY] = compact_csv_token(kernel)
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
        if ncu_descriptor_expansion_block is not None:
            if not stripped:
                ncu_descriptor_expansion_block = None
                continue
            if stripped.startswith("kernel,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                ncu_descriptor_expansion_block = None
                continue
            if len(row) >= 8 and row[0].strip() != "none":
                values[NCU_DESCRIPTOR_EXPANSION_HINT_KEY] = compact_csv_token(row[7].strip())
            continue
        if ncu_kernel_separation_block is not None:
            if not stripped:
                ncu_kernel_separation_block = None
                continue
            if stripped.startswith("kernel,"):
                continue
            try:
                row = next(csv.reader([line]))
            except csv.Error:
                ncu_kernel_separation_block = None
                continue
            if len(row) >= 10 and row[0].strip() != "none":
                ncu_top_kernel_separation_hints[row[0].strip()] = compact_csv_token(
                    row[9].strip()
                )
            continue
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        copy_site_match = CUDA_COPY_SITE_TIMING_RE.match(key)
        if copy_site_match is not None:
            direction, rank_text, site, field = copy_site_match.groups()
            rank = int(rank_text)
            if rank not in CUDA_COPY_SITE_TOP_RANKS:
                continue
            site_key = cuda_copy_site_site_key(direction, rank)
            site_value = compact_csv_token(site)
            if site_key in values and values[site_key] != site_value:
                raise SystemExit(f"duplicate timing field: {site_key}")
            values[site_key] = site_value
            key = cuda_copy_site_numeric_key(direction, rank, field)
        else:
            if (
                key not in TIMING_KEYS
                and OPENING_STAGE_ROW_VALUE_DEVICE_DOWNLOAD_BATCH_RE.match(key) is None
                and OPENING_STAGE_ROW_VALUE_DEVICE_SINGLE_DOWNLOAD_RE.match(key) is None
                and OPENING_STAGE_ROW_VALUE_SOURCE_EXTEND_CALLS_RE.match(key) is None
                and OPENING_STAGE_ROW_VALUE_SOURCE_EXTEND_MAX_ROWS_RE.match(key) is None
            ):
                continue
        try:
            parsed_value = int(value.strip())
        except ValueError:
            raise SystemExit(f"invalid timing field: {key}")
        if parsed_value < 0:
            raise SystemExit(f"negative timing field: {key}")
        if key in values:
            if values[key] == parsed_value:
                continue
            raise SystemExit(f"duplicate timing field: {key}")
        values[key] = parsed_value
    if ncu_top_kernel is not None and ncu_top_kernel in ncu_top_kernel_limits:
        values[NCU_TOP_KERNEL_LIMITING_FACTORS_KEY] = ncu_top_kernel_limits[ncu_top_kernel]
    if ncu_top_kernel is not None and ncu_top_kernel in ncu_top_kernel_separation_hints:
        values[NCU_TOP_KERNEL_SEPARATION_HINT_KEY] = ncu_top_kernel_separation_hints[
            ncu_top_kernel
        ]
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
        PERF_LIVE_STREAM_MESSAGE_SELF_PCT_KEY: 0.0,
        CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY: 0.0,
    }

    def is_live_stream_message_symbol(symbol_text: str) -> bool:
        return (
            "produce_guest_pc_trace_live_pending_messages" in symbol_text
            or "ZiskMainOwnedStreamingDeviceReportFeeder::push_report" in symbol_text
            or "emit_guest_pc_trace_live_pending_segment_messages" in symbol_text
        )

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
            if (
                nsys_cpu_block == "application_cpu_hotspots"
                and is_live_stream_message_symbol(row[0])
            ):
                hotspots[PERF_LIVE_STREAM_MESSAGE_SELF_PCT_KEY] += pct
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


def top_level_bottleneck(
    total_ms: int,
    catalog_ms: int,
    eth_input_ms: int,
    public_inputs_ms: int,
    plan_ms: int,
    framed_guest_input_ms: int,
    gpu_memory_preflight_ms: int,
    gpu_setup_ms: int,
    auxiliary_inputs_ms: int,
    trace_inputs_ms: int,
    witness_ms: int,
    proof_ms: int,
    output_write_ms: int,
    summary_ms: int,
    top_level_unattributed_ms: int,
) -> str:
    candidates = [
        ("catalog", catalog_ms),
        ("eth_input", eth_input_ms),
        ("public_inputs", public_inputs_ms),
        ("plan", plan_ms),
        ("framed_guest_input", framed_guest_input_ms),
        ("gpu_memory_preflight", gpu_memory_preflight_ms),
        ("gpu_setup", gpu_setup_ms),
        ("auxiliary_inputs", auxiliary_inputs_ms),
        ("trace_inputs", trace_inputs_ms),
        ("witness", witness_ms),
        ("proof", proof_ms),
        ("output_write", output_write_ms),
        ("summary", summary_ms),
        ("top_level_unattributed", top_level_unattributed_ms),
    ]
    name, value = max(candidates, key=lambda item: item[1])
    return name if value > 0 else "total" if total_ms > 0 else "unknown"


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
        return "parallel_segment_reexecution_candidate"

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


def parallel_lower_live_stream_segment_serial_bound_from_values(
    values: dict[str, int],
) -> bool:
    stream_elapsed_ms = values.get(STREAM_ELAPSED_MS_KEY, 0)
    if (
        values.get(TOTAL_MS_KEY, 0) <= PROOF_TARGET_MS
        or values.get(PARALLEL_LOWER_WORKERS_KEY, 0) <= 1
        or stream_elapsed_ms <= 0
        or values.get(TRACE_REPORT_CHUNK_SENT_KEY, 0) <= 0
        or values.get(PARALLEL_LOWER_STREAM_CHUNK_PROCESS_MS_KEY, 0) <= 0
    ):
        return False
    if (
        PARALLEL_LOWER_JOB_RECEIVE_WAIT_MS_KEY not in values
        or PARALLEL_LOWER_RESULT_SEND_WAIT_MS_KEY not in values
    ):
        return False
    result_receive_wait_ms = values.get(PARALLEL_LOWER_RESULT_RECEIVE_WAIT_MS_KEY, 0)
    result_send_wait_ms = values.get(PARALLEL_LOWER_RESULT_SEND_WAIT_MS_KEY, 0)
    job_receive_wait_ms = values.get(PARALLEL_LOWER_JOB_RECEIVE_WAIT_MS_KEY, 0)
    return (
        result_receive_wait_ms >= stream_elapsed_ms * 0.5
        and result_send_wait_ms <= max(1, stream_elapsed_ms * 0.05)
        and job_receive_wait_ms >= stream_elapsed_ms * 0.5
    )


def seed_ready_streamed_lower_reexecution_regression_from_values(
    values: dict[str, int],
) -> bool:
    stream_elapsed_ms = values.get(STREAM_ELAPSED_MS_KEY, 0)
    seed_attempts = values.get(SEED_DIRECT_LIFT_ATTEMPTS_KEY, 0)
    if (
        values.get(TOTAL_MS_KEY, 0) <= PROOF_TARGET_MS
        or values.get(PARALLEL_LOWER_WORKERS_KEY, 0) <= 1
        or values.get(PARALLEL_LOWER_STREAM_SEGMENTS_KEY, 0) <= 0
        or values.get(OWNED_STREAMING_LOWER_SEGMENTS_KEY, 0) > 0
        or stream_elapsed_ms <= 0
        or seed_attempts <= 0
        or values.get(SEED_DIRECT_LIFT_SUCCESSES_KEY, 0) < seed_attempts
        or values.get(SEED_FULL_ADVANCES_KEY, 0) > 1
    ):
        return False
    runner_lowerer_floor_ms = max(
        values.get(RUNNER_MS_KEY, 0),
        values.get(LOWERER_MS_KEY, 0),
        1,
    )
    return stream_elapsed_ms >= runner_lowerer_floor_ms * 1.25


def parallel_lower_stream_shape_hint(
    stream_segments: int,
    stream_chunks: int,
    stream_fallbacks: int,
    stream_retained_reports: int,
) -> str:
    if stream_segments <= 0 and stream_chunks <= 0:
        return "none"
    if stream_chunks > 0 and stream_segments <= 0:
        return "missing_stream_segment_counts"
    chunks_per_segment = stream_chunks / stream_segments if stream_segments else 0.0
    if chunks_per_segment >= 128.0:
        return "many_chunks_per_segment"
    if stream_fallbacks > 0:
        return "stream_fallbacks_present"
    if stream_retained_reports > 0:
        return "stream_retained_reports_present"
    return "balanced_stream_chunks"


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
    if (
        values.get(TOTAL_MS_KEY, 0) > PROOF_TARGET_MS
        and segment_commit_memory_pressure_hint_from_values(values)
        == "segment_commit_oom_fallback"
    ):
        return "avoid_segment_commit_worker_oom_fallback"
    if (
        values.get(TOTAL_MS_KEY, 0) > PROOF_TARGET_MS
        and parallel_lower_replay_duplicate_work_from_values(values)
    ):
        return "avoid_replay_only_parallel_lower"
    if parallel_lower_live_stream_segment_serial_bound_from_values(values):
        return "parallel_lower_live_stream_segment_serial_bound"
    if seed_ready_streamed_lower_reexecution_regression_from_values(values):
        return "avoid_seed_ready_streamed_lower_reexecution"
    stream_elapsed_ms = values.get(STREAM_ELAPSED_MS_KEY, 0)
    segment_input_gap_ms = values.get(SEGMENT_INPUT_GAP_MS_KEY, 0)
    if (
        values.get(TOTAL_MS_KEY, 0) > PROOF_TARGET_MS
        and values.get(PARALLEL_LOWER_WORKERS_KEY, 0) <= 1
        and stream_elapsed_ms > 0
        and segment_input_gap_ms >= stream_elapsed_ms * 0.75
        and values.get(SEGMENT_INPUT_GAP_COUNT_KEY, 0) > 1
        and values.get(SEGMENT_RECEIVE_WAIT_MS_KEY, 0) >= stream_elapsed_ms * 0.75
        and values.get(SEGMENT_COMMIT_WORKER_BACKPRESSURE_JOIN_MS_KEY, 0)
        <= max(10, int(stream_elapsed_ms * 0.01))
    ):
        return "trace_producer_input_gap_dominant"
    seed_attempts = values.get(SEED_DIRECT_LIFT_ATTEMPTS_KEY, 0)
    seed_ready = (
        seed_attempts > 0
        and values.get(SEED_DIRECT_LIFT_SUCCESSES_KEY, 0) >= seed_attempts
        and values.get(SEED_FULL_ADVANCES_KEY, 0) <= 1
    )
    if (
        values.get(TOTAL_MS_KEY, 0) > PROOF_TARGET_MS
        and values.get(PARALLEL_LOWER_WORKERS_KEY, 0) > 1
        and stream_elapsed_ms > 0
        and seed_ready
        and values.get(PENDING_RECEIVE_WAIT_MS_KEY, 0) >= stream_elapsed_ms * 0.5
        and values.get(RUNNER_MS_KEY, 0) >= stream_elapsed_ms * 0.75
    ):
        return "parallel_lower_runner_bound_after_seed_ready"
    if (
        values.get(TOTAL_MS_KEY, 0) > PROOF_TARGET_MS
        and values.get(PARALLEL_LOWER_WORKERS_KEY, 0) > 1
        and values.get(TRACE_REPORT_STORAGE_BYTES_KEY, 0) == 0
        and stream_elapsed_ms > 0
        and values.get(PARALLEL_LOWER_RESULT_RECEIVE_WAIT_MS_KEY, 0)
        >= stream_elapsed_ms * 0.5
        and seed_ready
    ):
        return "parallel_lower_result_bound_after_seed_ready"
    if (
        values.get(TOTAL_MS_KEY, 0) > PROOF_TARGET_MS
        and values.get(TRACE_REPORT_CHUNK_SENT_KEY, 0) > 0
        and values.get(TRACE_REPORT_BUFFER_CAPACITY_KEY, 0) == 0
        and values.get(TRACE_RUNNER_REPORT_BUFFER_CAPACITY_KEY, 0) > 0
    ):
        return "report_chunks_post_segment_split_regression"
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
    if (
        base_hint == "parallel_segment_reexecution_candidate"
        and values.get(SEED_DIRECT_LIFT_ATTEMPTS_KEY, 0) > 0
        and values.get(SEED_DIRECT_LIFT_SUCCESSES_KEY, 0)
        < values.get(SEED_DIRECT_LIFT_ATTEMPTS_KEY, 0)
    ):
        return "seed_direct_lift_before_parallel_reexecution"
    if base_hint in {
        "trace_generation_and_commit_pipeline_candidate",
        "parallel_trace_lowering_candidate",
        "trace_generation_parallelism_candidate",
    } and trace_shape_points_to_segment_reexecution(values):
        return "parallel_segment_reexecution_candidate"
    return base_hint


def trace_shape_points_to_segment_reexecution(values: dict[str, int]) -> bool:
    trace_report_rows = values.get(TRACE_REPORT_ROWS_KEY, 0)
    if trace_report_rows <= 0:
        return False
    trace_shape_hint = trace_shape_sample_hint(values, trace_report_rows)
    profiled_rows = trace_shape_row_denominator(
        values,
        trace_shape_hint,
        trace_report_rows,
    )
    if profiled_rows <= 0:
        return False
    external_op_rows = values.get(TRACE_EXTERNAL_OP_ROWS_KEY, 0)
    copy_rows = values.get(TRACE_COPY_ROWS_KEY, 0)
    indirect_memory_rows = values.get(TRACE_INDIRECT_MEMORY_ROWS_KEY, 0)
    external_op_row_pct = external_op_rows * 100.0 / profiled_rows
    copy_row_pct = copy_rows * 100.0 / profiled_rows
    indirect_memory_row_pct = indirect_memory_rows * 100.0 / profiled_rows
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
        trace_shape_profile_available(trace_shape_hint)
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


def copy_site_bytes_close_to(value: int, target: int) -> bool:
    if value <= 0 or target <= 0:
        return False
    tolerance = max(64 * 1024, target // 100)
    return abs(value - target) <= tolerance


def cuda_copy_site_action_hint(values: dict[str, int | str]) -> str:
    h2d_site = str(values.get(cuda_copy_site_site_key("h2d", 1), "none"))
    h2d_bytes = int(
        values.get(cuda_copy_site_numeric_key("h2d", 1, "bytes"), 0) or 0
    )
    h2d_wait_ms = (
        int(values.get(cuda_copy_site_numeric_key("h2d", 1, "wait_ns"), 0) or 0)
        / 1_000_000.0
    )
    if h2d_site == "none" or h2d_bytes <= 0:
        return "none"
    if h2d_wait_ms < CUDA_TRANSFER_WAIT_MS_THRESHOLD:
        return "none"

    descriptor_upload_bytes = int(values.get(DESCRIPTOR_UPLOAD_BYTES_KEY, 0) or 0)
    opening_descriptor_upload_bytes = int(
        values.get(OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_BYTES_KEY, 0) or 0
    )
    descriptor_upload_total_bytes = (
        descriptor_upload_bytes + opening_descriptor_upload_bytes
    )
    if copy_site_bytes_close_to(h2d_bytes, descriptor_upload_total_bytes):
        return "descriptor_upload_h2d_top_site"
    if copy_site_bytes_close_to(h2d_bytes, descriptor_upload_bytes):
        return "guest_descriptor_upload_h2d_top_site"
    if copy_site_bytes_close_to(h2d_bytes, opening_descriptor_upload_bytes):
        return "external_descriptor_upload_h2d_top_site"
    if h2d_bytes >= CUDA_TRANSFER_BULK_H2D_BYTES_THRESHOLD:
        return "bulk_h2d_top_site"
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


def allocator_reuse_action_hint(
    event_sync_wait_ms: float,
    event_sync_hot_wait_pct: float,
    pending_reuse_count: int,
    no_wait_bypass_count: int,
) -> str:
    if no_wait_bypass_count > 0:
        return "pending_cache_no_wait_active"
    if event_sync_wait_ms < CUDA_TRANSFER_WAIT_MS_THRESHOLD:
        return "none"
    if (
        pending_reuse_count > 0
        and event_sync_hot_wait_pct >= DIRECT_D2H_HOT_WAIT_PCT_THRESHOLD
    ):
        return "raise_pending_cache_no_wait_limit"
    if pending_reuse_count > 0:
        return "inspect_pending_cache_waits"
    return "inspect_allocator_event_waits"


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
    has_min_free_bytes = SEGMENT_COMMIT_CUDA_MEMORY_MIN_FREE_BYTES_KEY in values
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
    if total_bytes <= 0 or not has_min_free_bytes:
        return "memory_timing_missing"

    min_free_pct = min_free_bytes * 100.0 / total_bytes
    if min_free_pct <= SEGMENT_COMMIT_MEMORY_PRESSURE_PCT_THRESHOLD:
        return "segment_commit_memory_pressure"
    if min_free_pct <= SEGMENT_COMMIT_MEMORY_THIN_MARGIN_PCT_THRESHOLD:
        return "segment_commit_memory_thin_margin"
    return "segment_commit_memory_margin_ok"


def segment_commit_memory_diagnostic_hint(
    segment_commit_ms: int, memory_pressure_hint: str
) -> str:
    if (
        segment_commit_ms >= SEGMENT_COMMIT_MEMORY_DIAGNOSTIC_MS_THRESHOLD
        and memory_pressure_hint == "memory_timing_missing"
    ):
        return "profile_segment_commit_memory_timing"
    return "none"


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
        if lowered_report_row_pct >= 15.0:
            return "fused_runner_lowerer_report_storage_candidate"
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


def cpu_trace_live_stream_action_hint(
    values: dict[str, int],
    perf_hotspots: dict[str, float],
) -> str:
    live_stream_message_pct = perf_hotspots.get(
        PERF_LIVE_STREAM_MESSAGE_SELF_PCT_KEY, 0.0
    )
    if live_stream_message_pct < 5.0:
        return "none"
    chunks_sent = values.get(TRACE_REPORT_CHUNK_SENT_KEY, 0)
    parallel_lower_workers = values.get(PARALLEL_LOWER_WORKERS_KEY, 0)
    parallel_lower_job_receive_wait_ms = values.get(
        PARALLEL_LOWER_JOB_RECEIVE_WAIT_MS_KEY, 0
    )
    if (
        chunks_sent > 0
        and parallel_lower_workers > 0
        and live_stream_message_pct >= 10.0
    ):
        return "reduce_live_report_message_overhead"
    if chunks_sent > 0 and parallel_lower_job_receive_wait_ms > 0:
        return "live_report_message_overhead_secondary"
    return "none"


def cpu_trace_lowerer_action_hint(
    perf_hotspots: dict[str, float], trace_report_detail_action: str = "none"
) -> str:
    detail_hints = {
        "enable_shape_timing_for_row_validation_residual": (
            "shape_timing_required_for_row_validation_residual"
        ),
        "split_row_validation_residual_timers": (
            "row_validation_residual_timer_split_candidate"
        ),
        "profile_row_validation_residual": "row_validation_residual_profile_candidate",
        "profile_row_validation": "row_validation_profile_candidate",
        "detail_timing_bookkeeping_overhead": "detail_timing_bookkeeping_overhead",
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
    if append_descriptor_pct >= 5.0 and lowered_report_row_pct < 10.0:
        return "descriptor_append_candidate"
    if lowered_report_row_pct < 10.0:
        return "none"
    if (
        source_value_pct >= 2.5
        and append_descriptor_pct > 0.0
        and source_value_pct
        >= append_descriptor_pct * LOWERER_SOURCE_VALUE_CLOSE_TO_DESCRIPTOR_RATIO
    ):
        return "source_value_candidate"
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
    total_ms: int,
    external_source_count: int,
    query_units: int,
    single_query_units: int,
    row_value_source_rows: int,
    row_value_source_extend_ms: int,
    row_value_device_rows: int,
    row_value_device_download_batches: int,
    row_value_device_single_downloads: int,
    direct_d2h_wait_ms: float,
) -> str:
    if external_source_count <= 0 or query_units <= 1:
        return "none"
    if single_query_units < query_units:
        return "none"
    source_extend_pct = row_value_source_extend_ms * 100.0 / total_ms if total_ms else 0.0
    source_row_boundary = (
        row_value_source_rows >= query_units
        and source_extend_pct >= SOURCE_ROW_VALUE_SECONDARY_PCT_THRESHOLD
    )
    device_row_boundary = (
        direct_d2h_wait_ms >= OPENING_BATCHING_D2H_WAIT_MS_THRESHOLD
        and row_value_device_rows > 1
        and row_value_device_download_batches == 0
        and row_value_device_single_downloads > 1
    )
    if source_row_boundary or device_row_boundary:
        return EXTERNAL_SOURCE_ROW_VALUE_BOUNDARY_HINT
    return "none"


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


def opening_device_batch_stage_shape(values: dict[str, int]) -> tuple[int, int, int]:
    stage_counts = [
        count
        for key, count in values.items()
        if OPENING_STAGE_ROW_VALUE_DEVICE_DOWNLOAD_BATCH_RE.match(key) is not None
        and count > 0
    ]
    if not stage_counts:
        return (0, 0, 0)
    return (len(stage_counts), max(stage_counts), sum(stage_counts))


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
    total_ms: int,
    openings: int,
    rows: int,
    all_single_row: int,
    external_source_count: int,
    query_units: int,
    single_query_units: int,
    source_rows: int,
    source_extend_ms: int,
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
        source_extend_pct = source_extend_ms * 100.0 / total_ms if total_ms else 0.0
        if (
            external_source_count > 0
            and query_units > 1
            and single_query_units >= query_units
            and source_rows >= query_units
            and source_extend_pct >= SOURCE_ROW_VALUE_SECONDARY_PCT_THRESHOLD
        ):
            return EXTERNAL_SOURCE_ROW_VALUE_BOUNDARY_HINT
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
    seed_direct_lift_action_hint: str,
    seed_full_advances: int,
    seed_snapshot_runtime_hint: str,
    cpu_report_storage_hint: str,
    cpu_lowerer_hint: str,
    cpu_lowerer_detail_driven: bool = False,
    trace_shape_reexecution_driven: bool = False,
) -> str:
    trace_pipeline_hints = {
        "trace_generation_and_commit_pipeline_candidate",
        "parallel_segment_reexecution_candidate",
        "seed_direct_lift_before_parallel_reexecution",
        "parallel_segment_reexecution_authorization_required",
        "parallel_trace_lowering_candidate",
        "parallel_lower_result_bound_after_seed_ready",
        "parallel_lower_runner_bound_after_seed_ready",
        "trace_generation_parallelism_candidate",
        "commit_trace_overlap_candidate",
        "segment_commit_candidate",
        "trace_queue_backpressure_candidate",
        "trace_producer_input_gap_dominant",
    }
    if trace_pipeline_hint in {
        "avoid_segment_commit_worker_oom_fallback",
        "avoid_replay_only_parallel_lower",
        "parallel_lower_live_stream_segment_serial_bound",
        "avoid_seed_ready_streamed_lower_reexecution",
    }:
        return trace_pipeline_hint
    if seed_snapshot_runtime_hint == "trusted_seed_snapshot_seed_only_probe":
        return seed_snapshot_runtime_hint
    if (
        trace_pipeline_hint == "parallel_segment_reexecution_candidate"
        and trace_shape_reexecution_driven
        and seed_direct_lift_action_hint == "profile_runner_seed_snapshot"
    ):
        return trace_pipeline_hint
    if trace_pipeline_hint in trace_pipeline_hints and cpu_report_storage_hint in {
        "fused_runner_lowerer_report_storage_candidate",
        "runner_streaming_report_storage_candidate",
        "trace_report_storage_structural_candidate",
        "report_sidecar_storage_candidate",
        "post_segment_report_chunk_split",
    }:
        return cpu_report_storage_hint
    if (
        trace_pipeline_hint
        in {
            "trace_generation_and_commit_pipeline_candidate",
            "parallel_segment_reexecution_candidate",
        }
        and seed_direct_lift_action_hint == "seed_direct_lift_ready"
        and seed_full_advances > 1
    ):
        return "avoid_untrusted_seed_snapshot_validation"
    if (
        trace_pipeline_hint
        in {
            "trace_generation_and_commit_pipeline_candidate",
            "parallel_segment_reexecution_candidate",
        }
        and seed_direct_lift_action_hint == "seed_direct_lift_ready"
        and seed_full_advances <= 1
    ):
        return "seed_ready_parallel_segment_reexecution_candidate"
    if trace_pipeline_hint in trace_pipeline_hints and cpu_lowerer_hint in {
        "row_validation_residual_profile_candidate",
        "row_validation_residual_timer_split_candidate",
        "row_validation_profile_candidate",
        "source_values_residual_profile_candidate",
        "source_values_profile_candidate",
        "source_value_candidate",
        "descriptor_append_candidate",
        "visit_profile_candidate",
    } and (
        cpu_lowerer_detail_driven
        or seed_direct_lift_action_hint != "profile_runner_seed_snapshot"
    ):
        return cpu_lowerer_hint
    if (
        trace_pipeline_hint
        in {
            "trace_generation_and_commit_pipeline_candidate",
            "parallel_segment_reexecution_candidate",
            "parallel_segment_reexecution_authorization_required",
            "parallel_trace_lowering_candidate",
            "trace_generation_parallelism_candidate",
        }
        and seed_direct_lift_action_hint == "profile_runner_seed_snapshot"
    ):
        return "profile_runner_seed_snapshot_before_parallel_reexecution"
    if (
        trace_pipeline_hint in trace_pipeline_hints
        and retained_parent_checkpoint_action_hint
        == "retained_parent_checkpoint_path_time_secondary"
    ):
        return "trace_pipeline_over_secondary_opening_launches"
    if (
        trace_pipeline_hint in trace_pipeline_hints
        and trace_pipeline_hint != "seed_direct_lift_before_parallel_reexecution"
        and seed_direct_lift_action_hint.startswith("profile_")
        and seed_direct_lift_action_hint != "profile_runner_seed_snapshot"
    ):
        return seed_direct_lift_action_hint
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


def seed_snapshot_runtime_hint(
    seed_direct_lift_action_hint: str,
    seed_full_advances: int,
    parallel_lower_workers: int,
) -> str:
    if (
        seed_direct_lift_action_hint == "seed_direct_lift_ready"
        and seed_full_advances <= 1
        and parallel_lower_workers <= 0
    ):
        return "trusted_seed_snapshot_seed_only_probe"
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


def fri_opening_scope_action_hint(
    total_ms: int,
    unit_build_ms: int,
    layer_tree_ms: int,
    query_ms: int,
    fold_ms: int,
    duration_breakdown_present: bool,
    unit_count: int,
) -> str:
    if unit_count > 0 and not duration_breakdown_present:
        return "fri_opening_duration_breakdown_missing"
    if total_ms <= 0:
        return "none"
    known_nested_ms = layer_tree_ms + query_ms + fold_ms
    unit_build_residual_ms = max(unit_build_ms - known_nested_ms, 0)
    nested_name, nested_ms = max(
        [
            ("layer_tree", layer_tree_ms),
            ("query", query_ms),
            ("fold", fold_ms),
        ],
        key=lambda item: item[1],
    )

    residual_pct = unit_build_residual_ms * 100.0 / total_ms
    nested_pct = nested_ms * 100.0 / total_ms
    unit_scope_pct = unit_build_ms * 100.0 / total_ms
    if residual_pct >= 30.0 and residual_pct >= nested_pct:
        return "profile_fri_opening_unit_build_residual"
    if nested_ms > 0 and nested_pct >= 30.0:
        return f"profile_fri_opening_nested_{nested_name}"
    if unit_scope_pct >= 50.0:
        return "fri_opening_unit_build_scope_dominant"
    if unit_build_ms <= 0 and known_nested_ms <= 0:
        return "none"
    return "fri_opening_balanced"


def final_proof_timing_hint(
    total_ms: int,
    fri_opening_ms: int,
    fri_transcript_unit_build_ms: int,
    contribution_total_ms: int,
) -> str:
    scopes = [
        ("fri_opening", fri_opening_ms),
        ("fri_transcript", fri_transcript_unit_build_ms),
        ("contribution", contribution_total_ms),
    ]
    scope_name, scope_ms = max(scopes, key=lambda item: item[1])
    if scope_ms <= 0:
        return "none"
    if total_ms <= 0:
        return f"profile_final_proof_{scope_name}"
    scope_pct = scope_ms * 100.0 / total_ms
    if scope_pct < 10.0:
        return "final_proof_not_dominant"
    return f"profile_final_proof_{scope_name}"


def opening_source_rebuild_hint(
    external_source_count: int,
    retained_source_count: int,
    source_retention_metrics_present: bool,
    source_retention_attempts: int,
    source_retention_retained: int,
    source_retention_rejected: int,
    source_retention_limit_bytes: int,
) -> str:
    if external_source_count <= 0:
        return "none"
    if (
        source_retention_metrics_present
        and source_retention_retained == 0
        and source_retention_limit_bytes == 0
        and retained_source_count == 0
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


def opening_external_source_descriptor_action_hint(
    external_source_count: int,
    descriptor_upload_ms: int,
    descriptor_upload_bytes: int,
    descriptor_retention_rejected: int,
    descriptor_retention_limit_bytes: int,
) -> str:
    if external_source_count <= 0 or descriptor_upload_bytes <= 0:
        return "none"
    if descriptor_retention_rejected > 0:
        return "opening_descriptor_reupload_after_retention_reject"
    if descriptor_retention_limit_bytes == 0:
        return "opening_descriptor_reupload_without_retention_budget"
    if descriptor_upload_ms > 0:
        return "opening_descriptor_reupload"
    return "opening_descriptor_reupload_bytes_only"


def data_residency_action_hint(
    source_rebuild_hint: str,
    cuda_transfer_hint: str,
    source_retention_rejected_bytes: int,
    segment_commit_cuda_memory_total_bytes: int,
    trace_descriptor_residency_pipeline: bool,
    copy_summary_gpu_residency_hint: str,
) -> str:
    if (
        source_rebuild_hint
        in {
            "retained_source_disabled_external_rebuild",
            "retained_source_budget_rejected_external_rebuild",
            "partial_retained_source_external_rebuild",
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
    if copy_summary_gpu_residency_hint not in {"", "none"}:
        return copy_summary_gpu_residency_hint
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
    external_descriptor_upload_rows: int,
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
        selected_row_baseline = max(source_rows, query_units)
        if (
            external_descriptor_upload_rows >= SELECTED_DESCRIPTOR_ROW_REBUILD_MIN_ROWS
            and selected_row_baseline > 0
            and external_descriptor_upload_rows
            >= selected_row_baseline * SELECTED_DESCRIPTOR_ROW_REBUILD_RATIO
        ):
            return "select_descriptor_rows_for_external_source_openings"
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
    row_validation_hotspot_name: str,
    row_validation_residual_pct: float,
    source_values_residual_pct: float,
    visit_descriptor_pct: float,
    trace_shape_hint: str,
) -> str:
    if hotspot_name == "none" or hotspot_pct <= 0.0:
        return "none"
    if hotspot_name == "row_validation":
        if row_validation_residual_pct >= 50.0:
            if trace_shape_hint == "shape_timing_missing_for_detail_profile":
                return "enable_shape_timing_for_row_validation_residual"
            return "split_row_validation_residual_timers"
        if row_validation_hotspot_name == "timer_bookkeeping":
            return "detail_timing_bookkeeping_overhead"
        if (
            row_validation_hotspot_name == "source_value_record"
            and row_validation_residual_pct >= 25.0
        ):
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
    if (
        values.get(TRACE_SHAPE_SAMPLES_KEY, 0) > 0
        or values.get(TRACE_SHAPE_SAMPLE_ROWS_KEY, 0) > 0
    ):
        return "shape_timing_sampled"
    if any(values.get(key, 0) > 0 for key in TRACE_SHAPE_KEYS):
        return "shape_timing_enabled"
    if (
        values.get(TRACE_REPORT_DETAIL_SAMPLES_KEY, 0) > 0
        and values.get(TRACE_REPORT_ROW_VALIDATION_SAMPLED_NS_KEY, 0) > 0
    ):
        return "shape_timing_missing_for_detail_profile"
    return "shape_timing_disabled_or_zero"


def trace_precompile_action_hint(
    trace_shape_hint: str,
    precompile_rows: int,
    profiled_rows: int,
) -> str:
    if not trace_shape_profile_available(trace_shape_hint):
        if profiled_rows > 0:
            return "enable_shape_timing_for_precompile_rows"
        return "none"
    if profiled_rows <= 0:
        return "none"
    if precompile_rows <= 0:
        return "skip_precompile_microprobes"
    precompile_row_pct = precompile_rows * 100.0 / profiled_rows
    if precompile_row_pct < 1.0:
        return "precompile_rows_secondary"
    return "precompile_rows_present"


def trace_shape_profile_available(trace_shape_hint: str) -> bool:
    return trace_shape_hint in {"shape_timing_enabled", "shape_timing_sampled"}


def trace_shape_row_denominator(
    values: dict[str, int],
    trace_shape_hint: str,
    trace_report_rows: int,
) -> int:
    if trace_shape_hint == "shape_timing_sampled":
        sample_rows = values.get(TRACE_SHAPE_SAMPLE_ROWS_KEY, 0)
        if sample_rows > 0:
            return sample_rows
        return trace_report_rows
    return trace_report_rows


def trace_shape_row_mix_hint(
    trace_shape_hint: str,
    external_op_row_pct: float,
    copy_row_pct: float,
    indirect_memory_row_pct: float,
) -> str:
    if not trace_shape_profile_available(trace_shape_hint):
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


def trace_copy_action_hint(
    copy_shape_hint: str,
    copy_memory_source_row_pct: float,
    copy_indirect_memory_row_pct: float,
    copy_no_memory_row_pct: float,
) -> str:
    if copy_shape_hint == "none":
        return "none"
    if copy_shape_hint == "copy_no_memory_fast_path_candidate":
        return "target_copy_no_memory_fast_path"
    if copy_memory_source_row_pct >= 50.0 or copy_indirect_memory_row_pct >= 30.0:
        return "target_copy_memory_source_and_indirect_validation"
    if copy_no_memory_row_pct >= 30.0:
        return "measure_copy_no_memory_before_optimizing"
    return "measure_copy_shape_before_optimizing"


def trace_copy_source_action_hint(
    copy_source_memory_read_sampled_ns: int,
    copy_source_indirect_read_sampled_ns: int,
    copy_source_memory_read_ms: int,
    copy_source_indirect_read_ms: int,
    copy_source_memory_reads: int,
    copy_source_indirect_reads: int,
    source_values_lookup_pct: float,
    source_values_residual_pct: float,
) -> str:
    if source_values_residual_pct >= 50.0 and 0.0 < source_values_lookup_pct < 25.0:
        return "target_copy_source_values_residual"
    sampled_ns = copy_source_memory_read_sampled_ns + copy_source_indirect_read_sampled_ns
    if sampled_ns > 0:
        if copy_source_indirect_read_sampled_ns * 100 >= sampled_ns * 60:
            return "target_copy_indirect_source_lookup"
        if copy_source_memory_read_sampled_ns * 100 >= sampled_ns * 60:
            return "target_copy_memory_source_lookup"
        return "measure_copy_source_lookup_split"
    exact_ms = copy_source_memory_read_ms + copy_source_indirect_read_ms
    if exact_ms > 0:
        if copy_source_indirect_read_ms * 100 >= exact_ms * 60:
            return "target_copy_indirect_source_lookup"
        if copy_source_memory_read_ms * 100 >= exact_ms * 60:
            return "target_copy_memory_source_lookup"
        return "measure_copy_source_lookup_split"
    if copy_source_memory_reads + copy_source_indirect_reads > 0:
        return "enable_detail_timing_for_copy_source_reads"
    return "none"


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
    if trace_shape_hint in {"shape_timing_enabled", "shape_timing_sampled"}:
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
    ("source_value_record", TRACE_REPORT_SOURCE_VALUE_RECORD_SAMPLED_NS_KEY),
    ("instruction_result", TRACE_REPORT_INSTRUCTION_RESULT_SAMPLED_NS_KEY),
    ("next_pc", TRACE_REPORT_NEXT_PC_SAMPLED_NS_KEY),
    ("register_access", TRACE_REPORT_REGISTER_ACCESS_SAMPLED_NS_KEY),
    ("memory_access", TRACE_REPORT_MEMORY_ACCESS_SAMPLED_NS_KEY),
    ("store_apply", TRACE_REPORT_STORE_APPLY_SAMPLED_NS_KEY),
    (
        "row_validation_timer_bookkeeping",
        TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_SAMPLED_NS_KEY,
    ),
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
    ("source_value_record", TRACE_REPORT_SOURCE_VALUE_RECORD_MS_KEY),
    ("instruction_result", TRACE_REPORT_INSTRUCTION_RESULT_MS_KEY),
    ("next_pc", TRACE_REPORT_NEXT_PC_MS_KEY),
    ("register_access", TRACE_REPORT_REGISTER_ACCESS_MS_KEY),
    ("memory_access", TRACE_REPORT_MEMORY_ACCESS_MS_KEY),
    ("store_apply", TRACE_REPORT_STORE_APPLY_MS_KEY),
    (
        "row_validation_timer_bookkeeping",
        TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_MS_KEY,
    ),
    ("precompile_memory", TRACE_REPORT_PRECOMPILE_MEMORY_MS_KEY),
    ("memory_columns", TRACE_REPORT_MEMORY_COLUMNS_MS_KEY),
]

SOURCE_VALUE_DETAIL_HOTSPOT_KEYS = [
    ("source_a_value", TRACE_REPORT_SOURCE_A_VALUE_SAMPLED_NS_KEY),
    ("source_b_value", TRACE_REPORT_SOURCE_B_VALUE_SAMPLED_NS_KEY),
    ("source_value_record", TRACE_REPORT_SOURCE_VALUE_RECORD_SAMPLED_NS_KEY),
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
    (
        "timer_bookkeeping",
        TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_SAMPLED_NS_KEY,
    ),
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


RUNNER_DETAIL_HOTSPOT_KEYS = [
    ("prepare_instruction", RUNNER_PREPARE_INSTRUCTION_SAMPLED_NS_KEY),
    ("pre_boundary", RUNNER_PRE_BOUNDARY_SAMPLED_NS_KEY),
    ("row_plan", RUNNER_ROW_PLAN_SAMPLED_NS_KEY),
    ("cache_policy", RUNNER_CACHE_POLICY_SAMPLED_NS_KEY),
    ("advance", RUNNER_ADVANCE_SAMPLED_NS_KEY),
    ("cache_update", RUNNER_CACHE_UPDATE_SAMPLED_NS_KEY),
    ("row_count", RUNNER_ROW_COUNT_SAMPLED_NS_KEY),
    ("post_boundary", RUNNER_POST_BOUNDARY_SAMPLED_NS_KEY),
    ("counter_update", RUNNER_COUNTER_UPDATE_SAMPLED_NS_KEY),
    ("detail_recording", RUNNER_TIMER_BOOKKEEPING_SAMPLED_NS_KEY),
]


def trace_runner_detail_hotspot(values: dict[str, int]) -> tuple[int, str, float, float]:
    samples = values.get(RUNNER_DETAIL_SAMPLES_KEY, 0)
    sampled_ns = values.get(RUNNER_DETAIL_SAMPLED_NS_KEY, 0)
    if samples <= 0 or sampled_ns <= 0:
        return (0, "none", 0.0, 0.0)
    avg_ns = sampled_ns // samples
    hotspot_name = "none"
    hotspot_ns = 0
    explained_ns = 0
    for name, key in RUNNER_DETAIL_HOTSPOT_KEYS:
        value = values.get(key, 0)
        explained_ns += value
        if value > hotspot_ns:
            hotspot_name = name
            hotspot_ns = value
    hotspot_pct = hotspot_ns * 100.0 / sampled_ns if hotspot_ns else 0.0
    residual_pct = max(sampled_ns - explained_ns, 0) * 100.0 / sampled_ns
    return (avg_ns, hotspot_name, hotspot_pct, residual_pct)


def trace_runner_detail_action_hint(
    hotspot_name: str, hotspot_pct: float, residual_pct: float, has_sampled_detail: bool
) -> str:
    if not has_sampled_detail:
        return "enable_runner_detail_timing"
    if residual_pct >= 35.0:
        return "profile_runner_unattributed_work"
    if hotspot_name == "prepare_instruction" and hotspot_pct > 0.0:
        return "profile_instruction_cache_prepare"
    if hotspot_name in {"pre_boundary", "post_boundary"} and hotspot_pct > 0.0:
        return "profile_runner_boundary_snapshot"
    if hotspot_name == "cache_policy" and hotspot_pct > 0.0:
        return "profile_runner_cache_policy"
    if hotspot_name == "advance" and hotspot_pct > 0.0:
        return "profile_guest_machine_advance"
    if hotspot_name == "cache_update" and hotspot_pct > 0.0:
        return "profile_instruction_cache_invalidation"
    if hotspot_name in {"row_plan", "row_count", "counter_update"} and hotspot_pct > 0.0:
        return "profile_runner_row_accounting"
    if hotspot_name == "detail_recording" and hotspot_pct > 0.0:
        return "reduce_runner_detail_recording_overhead"
    return "runner_detail_balanced"


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
    detail_samples = values.get(TRACE_REPORT_DETAIL_SAMPLES_KEY, 0)
    trace_reports = values.get(TRACE_REPORTS_KEY, 0)
    if 0 < detail_samples < trace_reports:
        return (
            "none",
            0.0,
            trace_report_exact_action_hint("none", 0.0, True),
        )
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
) -> tuple[float, float, float]:
    source_values_ns = values.get(TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY, 0)
    if source_values_ns <= 0:
        return (0.0, 0.0, 0.0)
    lookup_ns = trace_report_source_lookup_sampled_ns(values)
    record_ns = trace_report_source_value_record_sampled_ns(values)
    residual_ns = max(source_values_ns - lookup_ns - record_ns, 0)
    return (
        lookup_ns * 100.0 / source_values_ns,
        record_ns * 100.0 / source_values_ns,
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


def trace_report_source_value_record_sampled_ns(values: dict[str, int]) -> int:
    return values.get(TRACE_REPORT_SOURCE_VALUE_RECORD_SAMPLED_NS_KEY, 0)


def trace_report_source_values_residual_sampled_ns(values: dict[str, int]) -> int:
    return max(
        values.get(TRACE_REPORT_SOURCE_VALUES_SAMPLED_NS_KEY, 0)
        - trace_report_source_lookup_sampled_ns(values)
        - trace_report_source_value_record_sampled_ns(values),
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


def seed_direct_lift_action_hint(
    attempts: int,
    successes: int,
    dominant_reason: str,
    trace_pipeline_hint: str = "none",
) -> str:
    if attempts <= 0:
        if trace_pipeline_hint in {
            "trace_generation_and_commit_pipeline_candidate",
            "parallel_segment_reexecution_candidate",
            "parallel_segment_reexecution_authorization_required",
            "parallel_trace_lowering_candidate",
            "trace_generation_parallelism_candidate",
        }:
            return "profile_runner_seed_snapshot"
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
    parallel_lower_result_receive_wait_ms: int,
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
    result_receive_wait_ratio = (
        parallel_lower_result_receive_wait_ms / stream_elapsed_ms
        if stream_elapsed_ms
        else 0.0
    )
    runner_stream_ratio = runner_ms / stream_elapsed_ms if stream_elapsed_ms else 0.0
    leaf_ratio = leaf_kernel_ms / trace_ms if trace_ms else 0.0
    trace_total_ratio = trace_ms / total_ms if total_ms else 0.0
    if parallel_lower_workers > 0:
        if pending_receive_wait_ratio >= 0.5 and runner_stream_ratio >= 0.75:
            return "parallel_lower_runner_bound"
        if result_receive_wait_ratio >= 0.5:
            return "parallel_lower_worker_result_bound"
        if receive_wait_ratio >= 0.5:
            return "parallel_lower_waiting"
        return "parallel_lower_active"
    if trace_total_ratio >= 0.6 and receive_wait_ratio >= 0.5 and leaf_ratio <= 0.2:
        return "trace_stream_cpu_floor"
    if trace_total_ratio >= 0.6:
        return "cpu_trace_dominant"
    return "none"


def trace_structure_hint_from_values(values: dict[str, int]) -> str:
    if parallel_lower_replay_duplicate_work_from_values(values):
        return "parallel_lower_replay_duplicate_work"
    return trace_structure_hint(
        values.get(TOTAL_MS_KEY, 0),
        values.get(RUNNER_MS_KEY, 0),
        values.get(LOWERER_MS_KEY, 0),
        values.get(STREAM_ELAPSED_MS_KEY, 0),
        values.get(SEGMENT_RECEIVE_WAIT_MS_KEY, 0),
        values.get(PENDING_RECEIVE_WAIT_MS_KEY, 0),
        values.get(PARALLEL_LOWER_RESULT_RECEIVE_WAIT_MS_KEY, 0),
        values.get(PARALLEL_LOWER_WORKERS_KEY, 0),
        values.get(LEAF_KERNEL_MS_KEY, 0),
    )


def parallel_lower_replay_duplicate_work_from_values(values: dict[str, int]) -> bool:
    return (
        values.get(PARALLEL_LOWER_WORKERS_KEY, 0) > 1
        and values.get(PARALLEL_LOWER_SNAPSHOT_REPLAY_KEY, 0) > 0
        and values.get(PARALLEL_LOWER_SNAPSHOT_REPLAY_MS_KEY, 0) > 0
        and values.get(PARALLEL_LOWER_REPORT_ELIDED_KEY, 0) > 0
    )


def trace_row_shape_pattern_description(pattern: int) -> str:
    if pattern == 0:
        return "none"
    op_code = (pattern >> 1) & 0xFF
    source_a_code = (pattern >> 9) & 0x7
    source_b_code = (pattern >> 12) & 0x7
    store_code = (pattern >> 15) & 0x3
    ind_width = (pattern >> 17) & 0xFF
    store_pc = (pattern >> 25) & 0x1
    set_pc = (pattern >> 26) & 0x1
    m32 = (pattern >> 27) & 0x1
    external = (pattern >> 28) & 0x1
    precompiled = (pattern >> 29) & 0x1
    op = TRACE_ROW_SHAPE_OP_NAMES.get(op_code, f"op{op_code}")
    source_a = TRACE_ROW_SHAPE_SOURCE_NAMES.get(source_a_code, f"source{source_a_code}")
    source_b = TRACE_ROW_SHAPE_SOURCE_NAMES.get(source_b_code, f"source{source_b_code}")
    store = TRACE_ROW_SHAPE_STORE_NAMES.get(store_code, f"store{store_code}")
    return (
        f"op={op};a={source_a};b={source_b};store={store};"
        f"ind_width={ind_width};store_pc={store_pc};set_pc={set_pc};"
        f"m32={m32};external={external};precompiled={precompiled}"
    )


def runner_advance_shape_pattern_description(pattern: int) -> str:
    if pattern == 0:
        return "none"
    kind_code = (pattern >> 1) & 0x7F
    memory_write = (pattern >> 8) & 0x1
    kind = RUNNER_ADVANCE_SHAPE_KIND_NAMES.get(kind_code, f"kind{kind_code}")
    return f"instruction={kind};memory_write={memory_write}"


def summarize_profile_values(
    label: str,
    values: dict[str, int],
    perf_hotspots: dict[str, float] | None = None,
) -> str:
    input_bytes = values.get(INPUT_BYTES_KEY, 0)
    total_ms = values.get(TOTAL_MS_KEY, 0)
    catalog_ms = values.get(CATALOG_MS_KEY, 0)
    eth_input_ms = values.get(ETH_INPUT_MS_KEY, 0)
    public_inputs_ms = values.get(PUBLIC_INPUTS_MS_KEY, 0)
    plan_ms = values.get(PLAN_MS_KEY, 0)
    framed_guest_input_ms = values.get(FRAMED_GUEST_INPUT_MS_KEY, 0)
    gpu_memory_preflight_ms = values.get(GPU_MEMORY_PREFLIGHT_MS_KEY, 0)
    gpu_setup_ms = values.get(GPU_SETUP_MS_KEY, 0)
    auxiliary_inputs_ms = values.get(AUXILIARY_INPUTS_MS_KEY, 0)
    trace_inputs_ms = values.get(TRACE_INPUTS_MS_KEY, 0)
    witness_ms = values.get(WITNESS_MS_KEY, 0)
    proof_ms = values.get(PROOF_MS_KEY, 0)
    output_write_ms = values.get(OUTPUT_WRITE_MS_KEY, 0)
    summary_ms = values.get(SUMMARY_MS_KEY, 0)
    accounted_top_level_ms = (
        catalog_ms
        + eth_input_ms
        + public_inputs_ms
        + plan_ms
        + framed_guest_input_ms
        + gpu_memory_preflight_ms
        + gpu_setup_ms
        + auxiliary_inputs_ms
        + trace_inputs_ms
        + witness_ms
        + proof_ms
        + output_write_ms
        + summary_ms
    )
    top_level_unattributed_ms = max(total_ms - accounted_top_level_ms, 0)
    gpu_memory_preflight_pct = (
        gpu_memory_preflight_ms * 100.0 / total_ms if total_ms else 0.0
    )
    gpu_setup_pct = gpu_setup_ms * 100.0 / total_ms if total_ms else 0.0
    top_level_hint = top_level_bottleneck(
        total_ms,
        catalog_ms,
        eth_input_ms,
        public_inputs_ms,
        plan_ms,
        framed_guest_input_ms,
        gpu_memory_preflight_ms,
        gpu_setup_ms,
        auxiliary_inputs_ms,
        trace_inputs_ms,
        witness_ms,
        proof_ms,
        output_write_ms,
        summary_ms,
        top_level_unattributed_ms,
    )
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
    runner_advance_fast_paths = values.get(RUNNER_ADVANCE_FAST_PATHS_KEY, 0)
    runner_advance_generic_fallbacks = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACKS_KEY, 0
    )
    runner_advance_total = (
        runner_advance_fast_paths + runner_advance_generic_fallbacks
    )
    runner_advance_fast_path_pct = (
        runner_advance_fast_paths * 100.0 / runner_advance_total
        if runner_advance_total
        else 0.0
    )
    runner_advance_fallback_shape_top_1_pattern = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_1_PATTERN_KEY, 0
    )
    runner_advance_fallback_shape_top_1_count = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_1_COUNT_KEY, 0
    )
    runner_advance_fallback_shape_top_2_pattern = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_2_PATTERN_KEY, 0
    )
    runner_advance_fallback_shape_top_2_count = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_2_COUNT_KEY, 0
    )
    runner_advance_fallback_shape_top_3_pattern = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_3_PATTERN_KEY, 0
    )
    runner_advance_fallback_shape_top_3_count = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_3_COUNT_KEY, 0
    )
    runner_advance_fallback_shape_top_4_pattern = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_4_PATTERN_KEY, 0
    )
    runner_advance_fallback_shape_top_4_count = values.get(
        RUNNER_ADVANCE_GENERIC_FALLBACK_SHAPE_TOP_4_COUNT_KEY, 0
    )
    runner_advance_fallback_shape_top_1_shape = (
        runner_advance_shape_pattern_description(
            runner_advance_fallback_shape_top_1_pattern
        )
    )
    runner_advance_fallback_shape_top_2_shape = (
        runner_advance_shape_pattern_description(
            runner_advance_fallback_shape_top_2_pattern
        )
    )
    runner_advance_fallback_shape_top_3_shape = (
        runner_advance_shape_pattern_description(
            runner_advance_fallback_shape_top_3_pattern
        )
    )
    runner_advance_fallback_shape_top_4_shape = (
        runner_advance_shape_pattern_description(
            runner_advance_fallback_shape_top_4_pattern
        )
    )
    runner_cache_hits = values.get(RUNNER_CACHE_HITS_KEY, 0)
    runner_cache_misses = values.get(RUNNER_CACHE_MISSES_KEY, 0)
    runner_cache_total = runner_cache_hits + runner_cache_misses
    runner_cache_hit_pct = (
        runner_cache_hits * 100.0 / runner_cache_total
        if runner_cache_total
        else 0.0
    )
    runner_cache_clears = values.get(RUNNER_CACHE_CLEARS_KEY, 0)
    runner_cache_fcall_clears = values.get(RUNNER_CACHE_FCALL_CLEARS_KEY, 0)
    runner_cache_dma_clears = values.get(RUNNER_CACHE_DMA_CLEARS_KEY, 0)
    runner_cache_invalidation_ranges = values.get(
        RUNNER_CACHE_INVALIDATION_RANGES_KEY, 0
    )
    runner_cache_invalidation_skipped_ranges = values.get(
        RUNNER_CACHE_INVALIDATION_SKIPPED_RANGES_KEY, 0
    )
    runner_cache_invalidation_skip_pct = (
        runner_cache_invalidation_skipped_ranges
        * 100.0
        / runner_cache_invalidation_ranges
        if runner_cache_invalidation_ranges
        else 0.0
    )
    runner_cache_invalidation_probes = values.get(
        RUNNER_CACHE_INVALIDATION_PROBES_KEY, 0
    )
    runner_cache_invalidated_entries = values.get(
        RUNNER_CACHE_INVALIDATED_ENTRIES_KEY, 0
    )
    trace_runner_detail_samples = values.get(RUNNER_DETAIL_SAMPLES_KEY, 0)
    trace_reports_for_runner_detail = values.get(TRACE_REPORTS_KEY, 0)
    trace_runner_detail_sample_pct = (
        trace_runner_detail_samples * 100.0 / trace_reports_for_runner_detail
        if trace_reports_for_runner_detail
        else 0.0
    )
    trace_runner_prepare_instruction_sampled_ns = values.get(
        RUNNER_PREPARE_INSTRUCTION_SAMPLED_NS_KEY, 0
    )
    trace_runner_pre_boundary_sampled_ns = values.get(RUNNER_PRE_BOUNDARY_SAMPLED_NS_KEY, 0)
    trace_runner_row_plan_sampled_ns = values.get(RUNNER_ROW_PLAN_SAMPLED_NS_KEY, 0)
    trace_runner_cache_policy_sampled_ns = values.get(RUNNER_CACHE_POLICY_SAMPLED_NS_KEY, 0)
    trace_runner_advance_sampled_ns = values.get(RUNNER_ADVANCE_SAMPLED_NS_KEY, 0)
    trace_runner_advance_setup_sampled_ns = values.get(
        RUNNER_ADVANCE_SETUP_SAMPLED_NS_KEY, 0
    )
    trace_runner_advance_execute_sampled_ns = values.get(
        RUNNER_ADVANCE_EXECUTE_SAMPLED_NS_KEY, 0
    )
    trace_runner_advance_report_sampled_ns = values.get(
        RUNNER_ADVANCE_REPORT_SAMPLED_NS_KEY, 0
    )
    trace_runner_cache_update_sampled_ns = values.get(
        RUNNER_CACHE_UPDATE_SAMPLED_NS_KEY, 0
    )
    trace_runner_row_count_sampled_ns = values.get(RUNNER_ROW_COUNT_SAMPLED_NS_KEY, 0)
    trace_runner_post_boundary_sampled_ns = values.get(RUNNER_POST_BOUNDARY_SAMPLED_NS_KEY, 0)
    trace_runner_counter_update_sampled_ns = values.get(
        RUNNER_COUNTER_UPDATE_SAMPLED_NS_KEY, 0
    )
    trace_runner_timer_bookkeeping_sampled_ns = values.get(
        RUNNER_TIMER_BOOKKEEPING_SAMPLED_NS_KEY, 0
    )
    (
        trace_runner_detail_avg_ns,
        trace_runner_detail_hotspot_name,
        trace_runner_detail_hotspot_pct,
        trace_runner_detail_residual_pct,
    ) = trace_runner_detail_hotspot(values)
    trace_runner_detail_action = trace_runner_detail_action_hint(
        trace_runner_detail_hotspot_name,
        trace_runner_detail_hotspot_pct,
        trace_runner_detail_residual_pct,
        trace_runner_detail_samples > 0,
    )
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
    segment_input_gap_ms = values.get(SEGMENT_INPUT_GAP_MS_KEY, 0)
    segment_input_gap_max_ms = values.get(SEGMENT_INPUT_GAP_MAX_MS_KEY, 0)
    segment_input_gap_count = values.get(SEGMENT_INPUT_GAP_COUNT_KEY, 0)
    segment_input_gap_avg_ms = (
        segment_input_gap_ms / segment_input_gap_count
        if segment_input_gap_count > 0
        else 0.0
    )
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
    segment_commit_cuda_memory_sample_ms = values.get(
        SEGMENT_COMMIT_CUDA_MEMORY_SAMPLE_MS_KEY, 0
    )
    segment_commit_cuda_memory_sample_count = values.get(
        SEGMENT_COMMIT_CUDA_MEMORY_SAMPLE_COUNT_KEY, 0
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
    segment_commit_memory_diagnostic = segment_commit_memory_diagnostic_hint(
        segment_commit_ms, segment_commit_memory_hint
    )
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
    parallel_lower_snapshot_replay_ms = values.get(
        PARALLEL_LOWER_SNAPSHOT_REPLAY_MS_KEY, 0
    )
    parallel_lower_report_elided = values.get(PARALLEL_LOWER_REPORT_ELIDED_KEY, 0)
    parallel_lower_stream_segments = values.get(
        PARALLEL_LOWER_STREAM_SEGMENTS_KEY, 0
    )
    parallel_lower_stream_chunks = values.get(PARALLEL_LOWER_STREAM_CHUNKS_KEY, 0)
    parallel_lower_stream_fallbacks = values.get(
        PARALLEL_LOWER_STREAM_FALLBACKS_KEY, 0
    )
    parallel_lower_stream_retained_reports = values.get(
        PARALLEL_LOWER_STREAM_RETAINED_REPORTS_KEY, 0
    )
    owned_streaming_lower_segments = values.get(
        OWNED_STREAMING_LOWER_SEGMENTS_KEY, 0
    )
    parallel_lower_dispatch_wait_ms = values.get(PARALLEL_LOWER_DISPATCH_WAIT_MS_KEY, 0)
    parallel_lower_stream_start_dispatch_wait_ms = values.get(
        PARALLEL_LOWER_STREAM_START_DISPATCH_WAIT_MS_KEY, 0
    )
    parallel_lower_stream_chunk_dispatch_wait_ms = values.get(
        PARALLEL_LOWER_STREAM_CHUNK_DISPATCH_WAIT_MS_KEY, 0
    )
    parallel_lower_stream_chunk_process_ms = values.get(
        PARALLEL_LOWER_STREAM_CHUNK_PROCESS_MS_KEY, 0
    )
    parallel_lower_job_receive_wait_ms = values.get(
        PARALLEL_LOWER_JOB_RECEIVE_WAIT_MS_KEY, 0
    )
    parallel_lower_result_send_wait_ms = values.get(
        PARALLEL_LOWER_RESULT_SEND_WAIT_MS_KEY, 0
    )
    parallel_lower_stream_segment_dispatch_wait_ms = values.get(
        PARALLEL_LOWER_STREAM_SEGMENT_DISPATCH_WAIT_MS_KEY, 0
    )
    parallel_lower_stream_finish_dispatch_wait_ms = values.get(
        PARALLEL_LOWER_STREAM_FINISH_DISPATCH_WAIT_MS_KEY, 0
    )
    parallel_lower_result_receive_wait_ms = values.get(
        PARALLEL_LOWER_RESULT_RECEIVE_WAIT_MS_KEY, 0
    )
    parallel_lower_dispatch_blocked = values.get(PARALLEL_LOWER_DISPATCH_BLOCKED_KEY, 0)
    segment_replay_count = values.get(SEGMENT_REPLAY_COUNT_KEY, 0)
    trace_reports = values.get(TRACE_REPORTS_KEY, 0)
    trace_report_rows = values.get(TRACE_REPORT_ROWS_KEY, 0)
    main_report_fast_paths = values.get(MAIN_REPORT_FAST_PATHS_KEY, 0)
    main_report_generic_fallbacks = values.get(MAIN_REPORT_GENERIC_FALLBACKS_KEY, 0)
    main_report_fcall_result_fast_paths = values.get(
        MAIN_REPORT_FCALL_RESULT_FAST_PATHS_KEY, 0
    )
    main_report_load_copy_fast_paths = values.get(
        MAIN_REPORT_LOAD_COPY_FAST_PATHS_KEY, 0
    )
    main_report_load_sign_extend_fast_paths = values.get(
        MAIN_REPORT_LOAD_SIGN_EXTEND_FAST_PATHS_KEY, 0
    )
    main_report_no_memory_fast_paths = values.get(
        MAIN_REPORT_NO_MEMORY_FAST_PATHS_KEY, 0
    )
    main_report_store_copy_fast_paths = values.get(
        MAIN_REPORT_STORE_COPY_FAST_PATHS_KEY, 0
    )
    main_report_simple_copy_fast_paths = values.get(
        MAIN_REPORT_SIMPLE_COPY_FAST_PATHS_KEY, 0
    )
    main_report_jump_fast_paths = values.get(MAIN_REPORT_JUMP_FAST_PATHS_KEY, 0)
    main_report_fallback_shape_top_1_pattern = values.get(
        MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_1_PATTERN_KEY, 0
    )
    main_report_fallback_shape_top_1_count = values.get(
        MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_1_COUNT_KEY, 0
    )
    main_report_fallback_shape_top_2_pattern = values.get(
        MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_2_PATTERN_KEY, 0
    )
    main_report_fallback_shape_top_2_count = values.get(
        MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_2_COUNT_KEY, 0
    )
    main_report_fallback_shape_top_3_pattern = values.get(
        MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_3_PATTERN_KEY, 0
    )
    main_report_fallback_shape_top_3_count = values.get(
        MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_3_COUNT_KEY, 0
    )
    main_report_fallback_shape_top_4_pattern = values.get(
        MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_4_PATTERN_KEY, 0
    )
    main_report_fallback_shape_top_4_count = values.get(
        MAIN_REPORT_GENERIC_FALLBACK_SHAPE_TOP_4_COUNT_KEY, 0
    )
    main_report_fallback_shape_top_1_shape = trace_row_shape_pattern_description(
        main_report_fallback_shape_top_1_pattern
    )
    main_report_fallback_shape_top_2_shape = trace_row_shape_pattern_description(
        main_report_fallback_shape_top_2_pattern
    )
    main_report_fallback_shape_top_3_shape = trace_row_shape_pattern_description(
        main_report_fallback_shape_top_3_pattern
    )
    main_report_fallback_shape_top_4_shape = trace_row_shape_pattern_description(
        main_report_fallback_shape_top_4_pattern
    )
    main_report_total_fast_path_attempts = (
        main_report_fast_paths + main_report_generic_fallbacks
    )
    main_report_fast_path_pct = (
        main_report_fast_paths * 100.0 / main_report_total_fast_path_attempts
        if main_report_total_fast_path_attempts
        else 0.0
    )
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
    copy_source_memory_read_ms = values.get(TRACE_COPY_SOURCE_MEMORY_READ_MS_KEY, 0)
    copy_source_indirect_read_ms = values.get(TRACE_COPY_SOURCE_INDIRECT_READ_MS_KEY, 0)
    copy_source_memory_reads = values.get(TRACE_COPY_SOURCE_MEMORY_READS_KEY, 0)
    copy_source_indirect_reads = values.get(TRACE_COPY_SOURCE_INDIRECT_READS_KEY, 0)
    copy_source_memory_read_sampled_ns = values.get(
        TRACE_COPY_SOURCE_MEMORY_READ_SAMPLED_NS_KEY, 0
    )
    copy_source_indirect_read_sampled_ns = values.get(
        TRACE_COPY_SOURCE_INDIRECT_READ_SAMPLED_NS_KEY, 0
    )
    copy_source_memory_read_avg_sample_ns = values.get(
        TRACE_COPY_SOURCE_MEMORY_READ_AVG_SAMPLE_NS_KEY,
        copy_source_memory_read_sampled_ns // copy_source_memory_reads
        if copy_source_memory_reads
        else 0,
    )
    copy_source_indirect_read_avg_sample_ns = values.get(
        TRACE_COPY_SOURCE_INDIRECT_READ_AVG_SAMPLE_NS_KEY,
        copy_source_indirect_read_sampled_ns // copy_source_indirect_reads
        if copy_source_indirect_reads
        else 0,
    )
    external_op_runs = values.get(TRACE_EXTERNAL_OP_RUNS_KEY, 0)
    external_op_max_run = values.get(TRACE_EXTERNAL_OP_MAX_RUN_KEY, 0)
    copy_runs = values.get(TRACE_COPY_RUNS_KEY, 0)
    copy_max_run = values.get(TRACE_COPY_MAX_RUN_KEY, 0)
    flag_rows = values.get(TRACE_FLAG_ROWS_KEY, 0)
    precompile_rows = values.get(TRACE_PRECOMPILE_ROWS_KEY, 0)
    indirect_memory_rows = values.get(TRACE_INDIRECT_MEMORY_ROWS_KEY, 0)
    trace_shape_hint = trace_shape_sample_hint(values, trace_report_rows)
    profiled_shape_rows = trace_shape_row_denominator(
        values,
        trace_shape_hint,
        trace_report_rows,
    )
    register_source_reads = values.get(TRACE_REGISTER_SOURCE_READS_KEY, 0)
    memory_source_reads = values.get(TRACE_MEMORY_SOURCE_READS_KEY, 0)
    register_store_rows = values.get(TRACE_REGISTER_STORE_ROWS_KEY, 0)
    memory_store_rows = values.get(TRACE_MEMORY_STORE_ROWS_KEY, 0)
    no_store_rows = values.get(TRACE_NO_STORE_ROWS_KEY, 0)
    row_shape_top_1_pattern = values.get(TRACE_ROW_SHAPE_TOP_1_PATTERN_KEY, 0)
    row_shape_top_1_count = values.get(TRACE_ROW_SHAPE_TOP_1_COUNT_KEY, 0)
    row_shape_top_2_pattern = values.get(TRACE_ROW_SHAPE_TOP_2_PATTERN_KEY, 0)
    row_shape_top_2_count = values.get(TRACE_ROW_SHAPE_TOP_2_COUNT_KEY, 0)
    row_shape_top_3_pattern = values.get(TRACE_ROW_SHAPE_TOP_3_PATTERN_KEY, 0)
    row_shape_top_3_count = values.get(TRACE_ROW_SHAPE_TOP_3_COUNT_KEY, 0)
    row_shape_top_4_pattern = values.get(TRACE_ROW_SHAPE_TOP_4_PATTERN_KEY, 0)
    row_shape_top_4_count = values.get(TRACE_ROW_SHAPE_TOP_4_COUNT_KEY, 0)
    row_shape_top_1_shape = trace_row_shape_pattern_description(row_shape_top_1_pattern)
    row_shape_top_2_shape = trace_row_shape_pattern_description(row_shape_top_2_pattern)
    row_shape_top_3_shape = trace_row_shape_pattern_description(row_shape_top_3_pattern)
    row_shape_top_4_shape = trace_row_shape_pattern_description(row_shape_top_4_pattern)
    indirect_memory_row_pct = (
        indirect_memory_rows * 100.0 / profiled_shape_rows
        if profiled_shape_rows
        else 0.0
    )
    memory_source_read_pct = (
        memory_source_reads * 100.0 / profiled_shape_rows
        if profiled_shape_rows
        else 0.0
    )
    memory_store_row_pct = (
        memory_store_rows * 100.0 / profiled_shape_rows
        if profiled_shape_rows
        else 0.0
    )
    no_store_row_pct = (
        no_store_rows * 100.0 / profiled_shape_rows
        if profiled_shape_rows
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
    copy_action_hint = trace_copy_action_hint(
        copy_shape_hint,
        copy_memory_source_row_pct,
        copy_indirect_memory_row_pct,
        copy_no_memory_row_pct,
    )
    copy_source_read_ms = copy_source_memory_read_ms + copy_source_indirect_read_ms
    copy_source_memory_read_pct = (
        copy_source_memory_read_ms * 100.0 / copy_source_read_ms
        if copy_source_read_ms
        else 0.0
    )
    copy_source_indirect_read_pct = (
        copy_source_indirect_read_ms * 100.0 / copy_source_read_ms
        if copy_source_read_ms
        else 0.0
    )
    trace_precompile_action = trace_precompile_action_hint(
        trace_shape_hint,
        precompile_rows,
        profiled_shape_rows,
    )
    external_op_row_pct = (
        external_op_rows * 100.0 / profiled_shape_rows
        if profiled_shape_rows
        else 0.0
    )
    copy_row_pct = (
        copy_rows * 100.0 / profiled_shape_rows
        if profiled_shape_rows
        else 0.0
    )
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
    trace_report_apply_ms = values.get(TRACE_REPORT_APPLY_MS_KEY, 0)
    trace_unit_summary_ms = values.get(TRACE_UNIT_SUMMARY_MS_KEY, 0)
    trace_report_emit_ms = values.get(TRACE_REPORT_EMIT_MS_KEY, 0)
    trace_descriptor_ms = values.get(TRACE_DESCRIPTOR_MS_KEY, 0)
    trace_report_lowering_ms = values.get(TRACE_REPORT_LOWERING_MS_KEY, 0)
    trace_report_row_validation_ms = values.get(TRACE_REPORT_ROW_VALIDATION_MS_KEY, 0)
    trace_report_row_validation_timer_bookkeeping_ms = values.get(
        TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_MS_KEY, 0
    )
    trace_report_memory_columns_ms = values.get(TRACE_REPORT_MEMORY_COLUMNS_MS_KEY, 0)
    trace_report_source_values_ms = values.get(TRACE_REPORT_SOURCE_VALUES_MS_KEY, 0)
    trace_report_source_a_value_ms = values.get(TRACE_REPORT_SOURCE_A_VALUE_MS_KEY, 0)
    trace_report_source_b_value_ms = values.get(TRACE_REPORT_SOURCE_B_VALUE_MS_KEY, 0)
    trace_report_source_value_record_ms = values.get(
        TRACE_REPORT_SOURCE_VALUE_RECORD_MS_KEY, 0
    )
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
    trace_report_source_value_record_share_ms = (
        trace_report_sampled_ns_lowerer_share_ms(
            values,
            trace_report_source_value_record_sampled_ns(values),
            trace_lowerer_share_scale_ms,
        )
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
    trace_report_row_validation_timer_bookkeeping_share_ms = (
        trace_report_detail_lowerer_share_ms(
            values,
            TRACE_REPORT_ROW_VALIDATION_TIMER_BOOKKEEPING_SAMPLED_NS_KEY,
            trace_lowerer_share_scale_ms,
        )
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
    trace_report_source_value_record_ns_per_row = ns_per_row_from_ms(
        trace_report_source_value_record_share_ms,
        trace_report_rows,
    )
    trace_report_source_values_residual_ns_per_row = ns_per_row_from_ms(
        trace_report_source_values_residual_share_ms,
        trace_report_rows,
    )
    trace_report_row_validation_timer_bookkeeping_ns_per_row = ns_per_row_from_ms(
        trace_report_row_validation_timer_bookkeeping_share_ms,
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
        trace_report_source_values_record_pct,
        trace_report_source_values_residual_pct,
    ) = trace_report_source_values_lookup_coverage(values)
    copy_source_action_hint = trace_copy_source_action_hint(
        copy_source_memory_read_sampled_ns,
        copy_source_indirect_read_sampled_ns,
        copy_source_memory_read_ms,
        copy_source_indirect_read_ms,
        copy_source_memory_reads,
        copy_source_indirect_reads,
        trace_report_source_values_lookup_pct,
        trace_report_source_values_residual_pct,
    )
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
        trace_report_row_validation_hotspot_name,
        trace_report_row_validation_residual_pct,
        trace_report_source_values_residual_pct,
        trace_report_visit_descriptor_pct,
        trace_shape_hint,
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
    parallel_lower_stream_chunks_per_segment = (
        parallel_lower_stream_chunks / parallel_lower_stream_segments
        if parallel_lower_stream_segments
        else 0.0
    )
    parallel_lower_stream_reports_per_chunk = (
        trace_report_chunk_reports / parallel_lower_stream_chunks
        if parallel_lower_stream_chunks
        else 0.0
    )
    parallel_lower_stream_shape = parallel_lower_stream_shape_hint(
        parallel_lower_stream_segments,
        parallel_lower_stream_chunks,
        parallel_lower_stream_fallbacks,
        parallel_lower_stream_retained_reports,
    )
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
    device_source_build_ms = values.get(DEVICE_SOURCE_BUILD_MS_KEY, 0)
    descriptor_upload_ms = values.get(DESCRIPTOR_UPLOAD_MS_KEY, 0)
    device_source_trace_expand_ms = values.get(DEVICE_SOURCE_TRACE_EXPAND_MS_KEY, 0)
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
    trace_pipeline_hint = trace_pipeline_action_hint_from_values(values)
    seed_direct_lift_action = seed_direct_lift_action_hint(
        seed_direct_lift_attempts,
        seed_direct_lift_successes,
        seed_direct_lift_dominant_miss,
        trace_pipeline_hint,
    )
    seed_direct_lift_ms = values.get(SEED_DIRECT_LIFT_MS_KEY, 0)
    seed_full_advance_ms = values.get(SEED_FULL_ADVANCE_MS_KEY, 0)
    seed_full_advances = values.get(SEED_FULL_ADVANCES_KEY, 0)
    seed_snapshot_runtime = seed_snapshot_runtime_hint(
        seed_direct_lift_action,
        seed_full_advances,
        parallel_lower_workers,
    )
    finish_opening_ms = values.get(FINISH_OPENING_MS_KEY, 0)
    opening_external_source_ms = values.get(OPENING_EXTERNAL_SOURCE_MS_KEY, 0)
    opening_external_source_descriptor_upload_ms = values.get(
        OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_MS_KEY, 0
    )
    opening_external_source_descriptor_upload_bytes = values.get(
        OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_BYTES_KEY, 0
    )
    opening_external_source_descriptor_upload_words = values.get(
        OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_WORDS_KEY, 0
    )
    opening_external_source_descriptor_upload_rows = values.get(
        OPENING_EXTERNAL_SOURCE_DESCRIPTOR_UPLOAD_ROWS_KEY, 0
    )
    opening_external_source_trace_expand_ms = values.get(
        OPENING_EXTERNAL_SOURCE_TRACE_EXPAND_MS_KEY, 0
    )
    opening_queries = values.get(OPENING_QUERY_COUNT_KEY, 0)
    opening_query_units = values.get(OPENING_QUERY_UNITS_KEY, 0)
    opening_single_query_units = values.get(OPENING_SINGLE_QUERY_UNITS_KEY, 0)
    opening_max_queries_per_unit = values.get(OPENING_MAX_QUERIES_PER_UNIT_KEY, 0)
    opening_stage_count = values.get(OPENING_STAGE_COUNT_KEY, 0)
    fri_opening_ms = values.get(FRI_OPENING_MS_KEY, 0)
    fri_opening_unit_build_ms = values.get(FRI_OPENING_UNIT_BUILD_MS_KEY, 0)
    fri_opening_layer_tree_ms = values.get(FRI_OPENING_LAYER_TREE_MS_KEY, 0)
    fri_opening_query_ms = values.get(FRI_OPENING_QUERY_MS_KEY, 0)
    fri_opening_fold_ms = values.get(FRI_OPENING_FOLD_MS_KEY, 0)
    fri_opening_duration_breakdown_present = all(
        key in values
        for key in (
            FRI_OPENING_MS_KEY,
            FRI_OPENING_UNIT_BUILD_MS_KEY,
            FRI_OPENING_LAYER_TREE_MS_KEY,
            FRI_OPENING_QUERY_MS_KEY,
            FRI_OPENING_FOLD_MS_KEY,
        )
    )
    fri_opening_units = values.get(FRI_OPENING_UNIT_COUNT_KEY, 0)
    fri_opening_layers = values.get(FRI_OPENING_LAYER_COUNT_KEY, 0)
    fri_opening_queries = values.get(FRI_OPENING_QUERY_COUNT_KEY, 0)
    fri_layers_per_unit = (
        fri_opening_layers / fri_opening_units if fri_opening_units else 0.0
    )
    fri_queries_per_unit = (
        fri_opening_queries / fri_opening_units if fri_opening_units else 0.0
    )
    fri_transcript_unit_build_ms = values.get(FRI_TRANSCRIPT_UNIT_BUILD_MS_KEY, 0)
    fri_transcript_layer_tree_ms = values.get(FRI_TRANSCRIPT_LAYER_TREE_MS_KEY, 0)
    fri_transcript_fold_ms = values.get(FRI_TRANSCRIPT_FOLD_MS_KEY, 0)
    fri_transcript_units = values.get(FRI_TRANSCRIPT_UNIT_COUNT_KEY, 0)
    fri_transcript_layers = values.get(FRI_TRANSCRIPT_LAYER_COUNT_KEY, 0)
    fri_transcript_layers_per_unit = (
        fri_transcript_layers / fri_transcript_units
        if fri_transcript_units
        else 0.0
    )
    contribution_segment_ms = values.get(CONTRIBUTION_SEGMENT_MS_KEY, 0)
    contribution_verify_ms = values.get(CONTRIBUTION_VERIFY_MS_KEY, 0)
    contribution_challenge_ms = values.get(CONTRIBUTION_CHALLENGE_MS_KEY, 0)
    contribution_total_ms = (
        contribution_segment_ms + contribution_verify_ms + contribution_challenge_ms
    )
    fri_opening_total_pct = (
        fri_opening_ms * 100.0 / total_ms if total_ms else 0.0
    )
    fri_transcript_unit_build_total_pct = (
        fri_transcript_unit_build_ms * 100.0 / total_ms if total_ms else 0.0
    )
    contribution_total_pct = (
        contribution_total_ms * 100.0 / total_ms if total_ms else 0.0
    )
    final_proof_hint = final_proof_timing_hint(
        total_ms,
        fri_opening_ms,
        fri_transcript_unit_build_ms,
        contribution_total_ms,
    )
    fri_opening_unit_build_scope_pct = (
        fri_opening_unit_build_ms * 100.0 / fri_opening_ms
        if fri_opening_ms
        else 0.0
    )
    fri_opening_layer_tree_nested_pct = (
        fri_opening_layer_tree_ms * 100.0 / fri_opening_ms
        if fri_opening_ms
        else 0.0
    )
    fri_opening_query_nested_pct = (
        fri_opening_query_ms * 100.0 / fri_opening_ms if fri_opening_ms else 0.0
    )
    fri_opening_fold_nested_pct = (
        fri_opening_fold_ms * 100.0 / fri_opening_ms if fri_opening_ms else 0.0
    )
    fri_opening_known_nested_ms = (
        fri_opening_layer_tree_ms + fri_opening_query_ms + fri_opening_fold_ms
    )
    fri_opening_known_nested_pct = (
        fri_opening_known_nested_ms * 100.0 / fri_opening_ms
        if fri_opening_ms
        else 0.0
    )
    fri_opening_unit_build_residual_ms = max(
        fri_opening_unit_build_ms - fri_opening_known_nested_ms,
        0,
    )
    fri_opening_unit_build_residual_pct = (
        fri_opening_unit_build_residual_ms * 100.0 / fri_opening_ms
        if fri_opening_ms
        else 0.0
    )
    fri_opening_scope_hint = fri_opening_scope_action_hint(
        fri_opening_ms,
        fri_opening_unit_build_ms,
        fri_opening_layer_tree_ms,
        fri_opening_query_ms,
        fri_opening_fold_ms,
        fri_opening_duration_breakdown_present,
        fri_opening_units,
    )
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
    stage_source_upload_ms = values.get(STAGE_SOURCE_UPLOAD_MS_KEY, 0)
    retained_trace_artifact_ms = values.get(RETAINED_TRACE_ARTIFACT_MS_KEY, 0)
    opening_row_value_device_rows = values.get(OPENING_ROW_VALUE_DEVICE_ROWS_KEY, 0)
    opening_row_value_source_rows = values.get(OPENING_ROW_VALUE_SOURCE_ROWS_KEY, 0)
    opening_row_value_words = values.get(OPENING_ROW_VALUE_WORDS_KEY, 0)
    opening_row_value_bytes = values.get(OPENING_ROW_VALUE_BYTES_KEY, 0)
    opening_row_value_source_extend_calls = values.get(
        OPENING_ROW_VALUE_SOURCE_EXTEND_CALLS_KEY, 0
    )
    opening_row_value_source_extend_max_rows = values.get(
        OPENING_ROW_VALUE_SOURCE_EXTEND_MAX_ROWS_KEY, 0
    )
    opening_row_value_source_extend_rows_per_call = (
        opening_row_value_source_rows / opening_row_value_source_extend_calls
        if opening_row_value_source_extend_calls
        else 0.0
    )
    opening_row_value_source_extend_ms = values.get(
        OPENING_ROW_VALUE_SOURCE_EXTEND_MS_KEY, 0
    )
    opening_row_value_source_download_ms = values.get(
        OPENING_ROW_VALUE_SOURCE_DOWNLOAD_MS_KEY, 0
    )
    opening_row_value_device_download_ms = values.get(
        OPENING_ROW_VALUE_DEVICE_DOWNLOAD_MS_KEY, 0
    )
    opening_row_value_source_extend_ms_per_call = (
        opening_row_value_source_extend_ms / opening_row_value_source_extend_calls
        if opening_row_value_source_extend_calls
        else 0.0
    )
    opening_row_value_source_extend_pct = (
        opening_row_value_source_extend_ms * 100.0 / total_ms if total_ms else 0.0
    )
    opening_row_dedup_input_rows = values.get(OPENING_ROW_DEDUP_INPUT_ROWS_KEY, 0)
    opening_row_dedup_unique_rows = values.get(OPENING_ROW_DEDUP_UNIQUE_ROWS_KEY, 0)
    opening_row_dedup_elided_rows = values.get(OPENING_ROW_DEDUP_ELIDED_ROWS_KEY, 0)
    opening_row_dedup_elided_pct = (
        opening_row_dedup_elided_rows * 100.0 / opening_row_dedup_input_rows
        if opening_row_dedup_input_rows
        else 0.0
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
            total_ms,
            retained_parent_checkpoint_openings,
            retained_parent_checkpoint_rows,
            retained_parent_checkpoint_all_single_row_value,
            opening_external_source_count,
            opening_query_units,
            opening_single_query_units,
            opening_row_value_source_rows,
            opening_row_value_source_extend_ms,
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
    (
        opening_row_value_device_batch_stage_count,
        opening_row_value_device_batch_max_stage,
        opening_row_value_device_batch_stage_sum,
    ) = opening_device_batch_stage_shape(values)
    opening_row_value_device_batch_unattributed = max(
        opening_row_value_device_download_batches - opening_row_value_device_batch_stage_sum,
        0,
    )
    opening_row_value_device_single_downloads = values.get(
        OPENING_ROW_VALUE_DEVICE_SINGLE_DOWNLOADS_KEY, 0
    )
    (
        opening_row_value_device_single_stage_count,
        opening_row_value_device_single_max_stage,
        opening_row_value_device_cross_unit_batch_savings,
    ) = opening_device_single_stage_shape(values)
    root_count = values.get(ROOT_COUNT_KEY, 0)
    groups = values.get(ROOT_GROUPS_KEY, 0)
    max_group_size = values.get(ROOT_MAX_GROUP_KEY, 0)
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
    cuda_setup_init_calls = values.get(CUDA_SETUP_INIT_CALLS_KEY, 0)
    cuda_setup_init_wait_ms = values.get(CUDA_SETUP_INIT_WAIT_NS_KEY, 0) / 1_000_000.0
    cuda_setup_init_max_wait_ms = (
        values.get(CUDA_SETUP_INIT_MAX_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_setup_cache_hits = values.get(CUDA_SETUP_CACHE_HITS_KEY, 0)
    cuda_setup_cache_hit_wait_ms = (
        values.get(CUDA_SETUP_CACHE_HIT_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_setup_cache_hit_max_wait_ms = (
        values.get(CUDA_SETUP_CACHE_HIT_MAX_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_setup_native_init_calls = values.get(CUDA_SETUP_NATIVE_INIT_CALLS_KEY, 0)
    cuda_setup_native_init_wait_ms = (
        values.get(CUDA_SETUP_NATIVE_INIT_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_setup_native_init_max_wait_ms = (
        values.get(CUDA_SETUP_NATIVE_INIT_MAX_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_current_device_calls = values.get(CUDA_CURRENT_DEVICE_CALLS_KEY, 0)
    cuda_current_device_wait_ms = (
        values.get(CUDA_CURRENT_DEVICE_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_current_device_max_wait_ms = (
        values.get(CUDA_CURRENT_DEVICE_MAX_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_memory_info_calls = values.get(CUDA_MEMORY_INFO_CALLS_KEY, 0)
    cuda_memory_info_wait_ms = (
        values.get(CUDA_MEMORY_INFO_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_memory_info_max_wait_ms = (
        values.get(CUDA_MEMORY_INFO_MAX_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_malloc_calls = values.get(CUDA_MALLOC_CALLS_KEY, 0)
    cuda_allocator_malloc_wait_ms = values.get(CUDA_MALLOC_WAIT_NS_KEY, 0) / 1_000_000.0
    cuda_allocator_malloc_max_wait_ms = (
        values.get(CUDA_MALLOC_MAX_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_h2d_wait_ms = (
        values.get(CUDA_COPY_H2D_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_h2d_hot_bytes = values.get(CUDA_COPY_H2D_HOT_BYTES_KEY, 0)
    cuda_allocator_h2d_hot_count = values.get(CUDA_COPY_H2D_HOT_COUNT_KEY, 0)
    cuda_allocator_h2d_hot_wait_ms = (
        values.get(CUDA_COPY_H2D_HOT_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_h2d_hot_wait_pct = (
        cuda_allocator_h2d_hot_wait_ms * 100.0 / cuda_allocator_h2d_wait_ms
        if cuda_allocator_h2d_wait_ms
        else 0.0
    )
    cuda_allocator_h2d_second_hot_bytes = values.get(
        CUDA_COPY_H2D_SECOND_HOT_BYTES_KEY, 0
    )
    cuda_allocator_h2d_second_hot_count = values.get(
        CUDA_COPY_H2D_SECOND_HOT_COUNT_KEY, 0
    )
    cuda_allocator_h2d_second_hot_wait_ms = (
        values.get(CUDA_COPY_H2D_SECOND_HOT_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_h2d_second_hot_wait_pct = (
        cuda_allocator_h2d_second_hot_wait_ms * 100.0 / cuda_allocator_h2d_wait_ms
        if cuda_allocator_h2d_wait_ms
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
    cuda_allocator_event_sync_calls = values.get(CUDA_EVENT_SYNC_CALLS_KEY, 0)
    cuda_allocator_event_sync_bytes = values.get(CUDA_EVENT_SYNC_BYTES_KEY, 0)
    cuda_allocator_event_sync_max_bytes = values.get(CUDA_EVENT_SYNC_MAX_BYTES_KEY, 0)
    cuda_allocator_event_sync_wait_ms = (
        values.get(CUDA_EVENT_SYNC_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_event_sync_max_wait_ms = (
        values.get(CUDA_EVENT_SYNC_MAX_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_event_sync_hot_bytes = values.get(CUDA_EVENT_SYNC_HOT_BYTES_KEY, 0)
    cuda_allocator_event_sync_hot_count = values.get(CUDA_EVENT_SYNC_HOT_COUNT_KEY, 0)
    cuda_allocator_event_sync_hot_wait_ms = (
        values.get(CUDA_EVENT_SYNC_HOT_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_allocator_event_sync_hot_wait_pct = (
        cuda_allocator_event_sync_hot_wait_ms
        * 100.0
        / cuda_allocator_event_sync_wait_ms
        if cuda_allocator_event_sync_wait_ms
        else 0.0
    )
    cuda_allocator_cached_reuse_count = values.get(CUDA_CACHED_REUSE_COUNT_KEY, 0)
    cuda_allocator_pending_reuse_count = values.get(CUDA_PENDING_REUSE_COUNT_KEY, 0)
    cuda_allocator_no_wait_bypass_count = values.get(CUDA_NO_WAIT_BYPASS_COUNT_KEY, 0)
    cuda_allocator_no_wait_bypass_bytes = values.get(CUDA_NO_WAIT_BYPASS_BYTES_KEY, 0)
    cuda_allocator_reuse_hint = allocator_reuse_action_hint(
        cuda_allocator_event_sync_wait_ms,
        cuda_allocator_event_sync_hot_wait_pct,
        cuda_allocator_pending_reuse_count,
        cuda_allocator_no_wait_bypass_count,
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
    cuda_host_unregister_wait_ms = (
        values.get(CUDA_HOST_UNREGISTER_WAIT_NS_KEY, 0) / 1_000_000.0
    )
    cuda_host_registration_total_wait_ms = (
        cuda_host_register_wait_ms + cuda_host_unregister_wait_ms
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
    source_retention_metrics_present = any(
        key in values
        for key in (
            SOURCE_RETENTION_ATTEMPTS_KEY,
            SOURCE_RETENTION_RETAINED_KEY,
            SOURCE_RETENTION_REJECTED_KEY,
            SOURCE_RETENTION_LIMIT_BYTES_KEY,
        )
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
        source_retention_metrics_present,
        source_retention_attempts,
        source_retention_retained,
        source_retention_rejected,
        source_retention_limit_bytes,
    )
    opening_external_source_descriptor_action = (
        opening_external_source_descriptor_action_hint(
            opening_external_source_count,
            opening_external_source_descriptor_upload_ms,
            opening_external_source_descriptor_upload_bytes,
            descriptor_retention_rejected,
            descriptor_retention_limit_bytes,
        )
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
        total_ms,
        opening_external_source_count,
        opening_query_units,
        opening_single_query_units,
        opening_row_value_source_rows,
        opening_row_value_source_extend_ms,
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
        parallel_lower_result_receive_wait_ms,
        parallel_lower_workers,
        leaf_kernel_ms,
    )
    if parallel_lower_replay_duplicate_work_from_values(values):
        trace_hint = "parallel_lower_replay_duplicate_work"
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
    if perf_hotspots is None:
        perf_hotspots = parse_perf_self_hotspots("")
    cpu_report_storage_hint = cpu_trace_report_storage_action_hint(values, perf_hotspots)
    lowerer_hint = cpu_trace_lowerer_action_hint(perf_hotspots, trace_report_detail_action)
    performance_focus = performance_focus_hint(
        trace_pipeline_hint,
        retained_parent_checkpoint_action_hint,
        seed_direct_lift_action,
        seed_full_advances,
        seed_snapshot_runtime,
        cpu_report_storage_hint,
        lowerer_hint,
        trace_report_detail_action != "none",
        trace_shape_points_to_segment_reexecution(values),
    )
    opening_source_row_value_hint = opening_source_row_value_action_hint(
        total_ms,
        opening_row_value_source_extend_ms,
        opening_row_value_source_rows,
        opening_external_source_count,
        opening_query_units,
        opening_single_query_units,
        opening_external_source_descriptor_upload_rows,
        trace_pipeline_hint,
    )
    cuda_transfer_hint = cuda_transfer_action_hint_from_values(values)
    cuda_copy_site_hint = cuda_copy_site_action_hint(values)
    copy_summary_gpu_residency_hint = str(
        values.get(NSYS_COPY_GPU_RESIDENCY_HINT_KEY, "none")
    )
    data_residency_hint = data_residency_action_hint(
        source_rebuild_hint,
        cuda_transfer_hint,
        source_retention_rejected_bytes,
        segment_commit_cuda_memory_total_bytes,
        values.get(NSYS_COPY_TRACE_DESCRIPTOR_RESIDENCY_PIPELINE_KEY, 0) > 0,
        copy_summary_gpu_residency_hint,
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
    copy_summary_host_registration_api_ms = str(
        values.get(NSYS_COPY_HOST_REGISTRATION_API_MS_KEY, "0.000")
    )
    copy_summary_host_registration_hint = str(
        values.get(NSYS_COPY_HOST_REGISTRATION_HINT_KEY, "none")
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
    ncu_top_kernel_separation_hint = str(
        values.get(NCU_TOP_KERNEL_SEPARATION_HINT_KEY, "unknown")
    )
    ncu_descriptor_expansion_hint = str(
        values.get(NCU_DESCRIPTOR_EXPANSION_HINT_KEY, "none")
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
    cpu_report_storage_memcpy_pct = perf_hotspots.get(
        CPU_TRACE_MEMCPY_REPORT_STORAGE_HINT_PCT_KEY, 0.0
    )
    cpu_report_storage_memcpy_total_pct = (
        memmove_pct * cpu_report_storage_memcpy_pct / 100.0
    )
    live_stream_message_pct = perf_hotspots.get(
        PERF_LIVE_STREAM_MESSAGE_SELF_PCT_KEY, 0.0
    )
    live_stream_hint = cpu_trace_live_stream_action_hint(values, perf_hotspots)
    append_descriptor_pct = perf_hotspots.get(
        PERF_APPEND_DESCRIPTOR_SELF_PCT_KEY, 0.0
    )
    source_value_pct = perf_hotspots.get(PERF_SOURCE_VALUE_SELF_PCT_KEY, 0.0)
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
        f"{csv_cell(label)},{input_bytes},{total_ms},"
        f"{catalog_ms},{eth_input_ms},{public_inputs_ms},"
        f"{plan_ms},{framed_guest_input_ms},{gpu_memory_preflight_ms},{gpu_setup_ms},"
        f"{auxiliary_inputs_ms},{trace_inputs_ms},{witness_ms},"
        f"{proof_ms},{output_write_ms},{summary_ms},"
        f"{top_level_unattributed_ms},{gpu_memory_preflight_pct:.3f},{gpu_setup_pct:.3f},{top_level_hint},"
        f"{constant_material_elapsed_ms},{constant_material_join_wait_ms},"
        f"{constant_material_hint},{runner_ms},"
        f"{runner_advance_fast_paths},{runner_advance_generic_fallbacks},"
        f"{runner_advance_fast_path_pct:.3f},"
        f"{csv_cell(runner_advance_fallback_shape_top_1_pattern)},"
        f"{runner_advance_fallback_shape_top_1_count},"
        f"{runner_advance_fallback_shape_top_1_shape},"
        f"{csv_cell(runner_advance_fallback_shape_top_2_pattern)},"
        f"{runner_advance_fallback_shape_top_2_count},"
        f"{runner_advance_fallback_shape_top_2_shape},"
        f"{csv_cell(runner_advance_fallback_shape_top_3_pattern)},"
        f"{runner_advance_fallback_shape_top_3_count},"
        f"{runner_advance_fallback_shape_top_3_shape},"
        f"{csv_cell(runner_advance_fallback_shape_top_4_pattern)},"
        f"{runner_advance_fallback_shape_top_4_count},"
        f"{runner_advance_fallback_shape_top_4_shape},"
        f"{runner_cache_hits},{runner_cache_misses},"
        f"{runner_cache_hit_pct:.3f},{runner_cache_clears},"
        f"{runner_cache_fcall_clears},{runner_cache_dma_clears},"
        f"{runner_cache_invalidation_ranges},"
        f"{runner_cache_invalidation_skipped_ranges},"
        f"{runner_cache_invalidation_skip_pct:.3f},"
        f"{runner_cache_invalidation_probes},"
        f"{runner_cache_invalidated_entries},"
        f"{trace_runner_detail_samples},{trace_runner_detail_sample_pct:.3f},"
        f"{trace_runner_detail_avg_ns},"
        f"{trace_runner_prepare_instruction_sampled_ns},"
        f"{trace_runner_pre_boundary_sampled_ns},"
        f"{trace_runner_row_plan_sampled_ns},"
        f"{trace_runner_cache_policy_sampled_ns},"
        f"{trace_runner_advance_sampled_ns},"
        f"{trace_runner_advance_setup_sampled_ns},"
        f"{trace_runner_advance_execute_sampled_ns},"
        f"{trace_runner_advance_report_sampled_ns},"
        f"{trace_runner_cache_update_sampled_ns},"
        f"{trace_runner_row_count_sampled_ns},"
        f"{trace_runner_post_boundary_sampled_ns},"
        f"{trace_runner_counter_update_sampled_ns},"
        f"{trace_runner_timer_bookkeeping_sampled_ns},"
        f"{csv_cell(trace_runner_detail_hotspot_name)},"
        f"{trace_runner_detail_hotspot_pct:.3f},"
        f"{trace_runner_detail_residual_pct:.3f},"
        f"{trace_runner_detail_action},{lowerer_ms},"
        f"{trace_lower_ms},{trace_report_ms},"
        f"{trace_report_apply_ms},{trace_unit_summary_ms},{trace_non_report_ms},"
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
        f"{segment_input_gap_ms},{segment_input_gap_max_ms},"
        f"{segment_input_gap_count},{segment_input_gap_avg_ms:.3f},"
        f"{stream_commit_residual_ms},{segment_receive_wait_ms},"
        f"{pending_receive_wait_ms},{pending_send_wait_ms},"
        f"{parallel_lower_workers},{parallel_lower_dispatched},"
        f"{parallel_lower_received},{parallel_lower_emitted},"
        f"{parallel_lower_max_reorder},{parallel_lower_snapshot_replay},"
        f"{parallel_lower_snapshot_replay_ms},"
        f"{parallel_lower_report_elided},"
        f"{parallel_lower_stream_segments},{parallel_lower_stream_chunks},"
        f"{parallel_lower_stream_fallbacks},{parallel_lower_stream_retained_reports},"
        f"{owned_streaming_lower_segments},"
        f"{parallel_lower_stream_chunks_per_segment:.3f},"
        f"{parallel_lower_stream_reports_per_chunk:.3f},"
        f"{parallel_lower_stream_shape},"
        f"{parallel_lower_dispatch_wait_ms},"
        f"{parallel_lower_stream_start_dispatch_wait_ms},"
        f"{parallel_lower_stream_chunk_dispatch_wait_ms},"
        f"{parallel_lower_stream_chunk_process_ms},"
        f"{parallel_lower_job_receive_wait_ms},"
        f"{parallel_lower_result_send_wait_ms},"
        f"{parallel_lower_stream_segment_dispatch_wait_ms},"
        f"{parallel_lower_stream_finish_dispatch_wait_ms},"
        f"{parallel_lower_result_receive_wait_ms},"
        f"{parallel_lower_dispatch_blocked},{segment_replay_count},{trace_reports},"
        f"{trace_report_rows},{main_report_fast_paths},"
        f"{main_report_generic_fallbacks},{main_report_fast_path_pct:.3f},"
        f"{main_report_fcall_result_fast_paths},"
        f"{main_report_load_copy_fast_paths},"
        f"{main_report_load_sign_extend_fast_paths},"
        f"{main_report_no_memory_fast_paths},"
        f"{main_report_store_copy_fast_paths},"
        f"{main_report_simple_copy_fast_paths},"
        f"{main_report_jump_fast_paths},"
        f"{csv_cell(main_report_fallback_shape_top_1_pattern)},"
        f"{main_report_fallback_shape_top_1_count},"
        f"{main_report_fallback_shape_top_1_shape},"
        f"{csv_cell(main_report_fallback_shape_top_2_pattern)},"
        f"{main_report_fallback_shape_top_2_count},"
        f"{main_report_fallback_shape_top_2_shape},"
        f"{csv_cell(main_report_fallback_shape_top_3_pattern)},"
        f"{main_report_fallback_shape_top_3_count},"
        f"{main_report_fallback_shape_top_3_shape},"
        f"{csv_cell(main_report_fallback_shape_top_4_pattern)},"
        f"{main_report_fallback_shape_top_4_count},"
        f"{main_report_fallback_shape_top_4_shape},"
        f"{trace_rows_per_report:.3f},"
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
        f"{device_source_build_ms},{descriptor_upload_ms},{device_source_trace_expand_ms},"
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
        f"{seed_direct_lift_boundary_c_unavailable},{seed_direct_lift_ms},"
        f"{seed_full_advance_ms},{seed_full_advances},"
        f"{seed_snapshot_runtime},"
        f"{finish_opening_ms},{opening_external_source_ms},"
        f"{opening_external_source_descriptor_upload_ms},"
        f"{opening_external_source_descriptor_upload_bytes},"
        f"{opening_external_source_descriptor_upload_words},"
        f"{opening_external_source_descriptor_upload_rows},"
        f"{opening_external_source_trace_expand_ms},"
        f"{opening_query_units},{opening_single_query_units},"
        f"{opening_queries},{opening_max_queries_per_unit},{opening_stage_count},"
        f"{fri_opening_ms},{fri_opening_unit_build_ms},{fri_opening_layer_tree_ms},"
        f"{fri_opening_query_ms},{fri_opening_fold_ms},"
        f"{fri_opening_units},{fri_opening_layers},{fri_opening_queries},"
        f"{fri_layers_per_unit:.3f},{fri_queries_per_unit:.3f},"
        f"{fri_transcript_unit_build_ms},{fri_transcript_layer_tree_ms},"
        f"{fri_transcript_fold_ms},{fri_transcript_units},"
        f"{fri_transcript_layers},{fri_transcript_layers_per_unit:.3f},"
        f"{contribution_segment_ms},{contribution_verify_ms},"
        f"{contribution_challenge_ms},{contribution_total_ms},"
        f"{fri_opening_total_pct:.3f},{fri_transcript_unit_build_total_pct:.3f},"
        f"{contribution_total_pct:.3f},{final_proof_hint},"
        f"{opening_source_hint},"
        f"{stage_source_upload_ms},{retained_trace_artifact_ms},"
        f"{source_retention_attempts},{source_retention_retained},"
        f"{source_retention_rejected},{source_retention_retained_bytes},"
        f"{source_retention_rejected_bytes},{source_retention_max_retained_bytes},"
        f"{source_retention_max_rejected_bytes},{source_retention_limit_bytes},"
        f"{source_retention_total_exceeds_device_memory},"
        f"{source_retention_max_exceeds_device_memory},"
        f"{source_rebuild_hint},{opening_external_source_descriptor_action},"
        f"{opening_row_value_device_rows},"
        f"{opening_row_value_source_rows},{opening_row_value_words},"
        f"{opening_row_value_bytes},{opening_row_value_source_extend_calls},"
        f"{opening_row_value_source_extend_max_rows},"
        f"{opening_row_value_source_extend_rows_per_call:.3f},"
        f"{opening_row_value_source_extend_ms_per_call:.3f},"
        f"{opening_row_value_source_extend_ms},"
        f"{opening_row_value_source_download_ms},"
        f"{opening_row_value_device_download_ms},"
        f"{opening_row_value_source_extend_pct:.3f},{opening_source_row_value_hint},"
        f"{opening_row_dedup_input_rows},{opening_row_dedup_unique_rows},"
        f"{opening_row_dedup_elided_rows},{opening_row_dedup_elided_pct:.3f},"
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
        f"{opening_row_value_device_batch_stage_count},"
        f"{opening_row_value_device_batch_max_stage},"
        f"{opening_row_value_device_batch_stage_sum},"
        f"{opening_row_value_device_batch_unattributed},"
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
        f"{live_stream_message_pct:.3f},{live_stream_hint},"
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
        f"{csv_cell(row_shape_top_1_pattern)},{row_shape_top_1_count},{row_shape_top_1_shape},"
        f"{csv_cell(row_shape_top_2_pattern)},{row_shape_top_2_count},{row_shape_top_2_shape},"
        f"{csv_cell(row_shape_top_3_pattern)},{row_shape_top_3_count},{row_shape_top_3_shape},"
        f"{csv_cell(row_shape_top_4_pattern)},{row_shape_top_4_count},{row_shape_top_4_shape},"
        f"{trace_precompile_action},"
        f"{copy_memory_source_rows},{copy_memory_source_row_pct:.3f},"
        f"{copy_indirect_memory_rows},{copy_indirect_memory_row_pct:.3f},"
        f"{copy_register_store_rows},{copy_memory_store_rows},"
        f"{copy_no_store_rows},{copy_no_memory_rows},"
        f"{copy_no_memory_row_pct:.3f},{copy_shape_hint},{copy_action_hint},"
        f"{copy_source_memory_read_ms},{copy_source_indirect_read_ms},"
        f"{copy_source_memory_read_pct:.3f},{copy_source_indirect_read_pct:.3f},"
        f"{copy_source_memory_reads},{copy_source_indirect_reads},"
        f"{copy_source_memory_read_sampled_ns},{copy_source_indirect_read_sampled_ns},"
        f"{copy_source_memory_read_avg_sample_ns},{copy_source_indirect_read_avg_sample_ns},"
        f"{copy_source_action_hint},"
        f"{trace_report_validation_ms},{trace_report_emit_ms},{trace_descriptor_ms},"
        f"{trace_report_lowering_ms},{trace_report_row_validation_ms},"
        f"{trace_report_row_validation_timer_bookkeeping_ms},"
        f"{trace_report_memory_columns_ms},{trace_report_source_values_ms},"
        f"{trace_report_source_a_value_ms},{trace_report_source_b_value_ms},"
        f"{trace_report_source_value_record_ms},"
        f"{trace_report_precompile_memory_ms},"
        f"{trace_report_instruction_result_ms},{trace_report_next_pc_ms},"
        f"{trace_report_register_access_ms},{trace_report_memory_access_ms},"
        f"{trace_report_store_apply_ms},{trace_report_visit_ms},"
        f"{csv_cell(trace_report_exact_hotspot_name)},"
        f"{trace_report_exact_hotspot_pct:.3f},{trace_report_exact_action},"
        f"{trace_report_detail_samples},{trace_report_detail_sample_pct:.3f},"
        f"{trace_report_detail_sample_ppm:.3f},{trace_report_detail_hint},"
        f"{trace_report_detail_avg_ns},"
        f"{trace_report_detail_share_ms:.3f},"
        f"{trace_report_row_validation_share_ms:.3f},"
        f"{trace_report_memory_columns_share_ms:.3f},"
        f"{trace_report_source_values_share_ms:.3f},"
        f"{trace_report_source_lookup_share_ms:.3f},"
        f"{trace_report_source_value_record_share_ms:.3f},"
        f"{trace_report_source_values_residual_share_ms:.3f},"
        f"{trace_report_precompile_memory_share_ms:.3f},"
        f"{trace_report_instruction_result_share_ms:.3f},"
        f"{trace_report_next_pc_share_ms:.3f},"
        f"{trace_report_register_access_share_ms:.3f},"
        f"{trace_report_memory_access_share_ms:.3f},"
        f"{trace_report_store_apply_share_ms:.3f},"
        f"{trace_report_row_validation_timer_bookkeeping_share_ms:.3f},"
        f"{trace_report_row_validation_residual_share_ms:.3f},"
        f"{trace_report_visit_share_ms:.3f},"
        f"{trace_report_descriptor_share_ms:.3f},"
        f"{csv_cell(trace_report_detail_hotspot_name)},{trace_report_detail_hotspot_pct:.3f},"
        f"{trace_report_detail_action},"
        f"{csv_cell(trace_report_row_validation_hotspot_name)},"
        f"{trace_report_row_validation_hotspot_pct:.3f},"
        f"{trace_report_row_validation_explained_pct:.3f},"
        f"{trace_report_row_validation_residual_pct:.3f},"
        f"{trace_report_source_values_lookup_pct:.3f},"
        f"{trace_report_source_values_record_pct:.3f},"
        f"{trace_report_source_values_residual_pct:.3f},"
        f"{source_immediate_reads},{source_immediate_read_pct:.3f},"
        f"{source_register_reads},{source_register_read_pct:.3f},"
        f"{source_memory_reads},{source_memory_read_pct:.3f},"
        f"{source_indirect_reads},{source_indirect_read_pct:.3f},"
        f"{source_last_c_reads},{source_last_c_read_pct:.3f},"
        f"{csv_cell(trace_report_source_kind_hotspot)},"
        f"{trace_report_source_kind_hotspot_pct:.3f},"
        f"{trace_report_source_kind_coverage_pct:.3f},"
        f"{trace_report_source_kind_residual_pct:.3f},"
        f"{trace_report_detail_visit_pct:.3f},"
        f"{trace_report_visit_descriptor_pct:.3f},"
        f"{trace_report_visit_residual_pct:.3f},"
        f"{direct_d2h_hot_bytes},{direct_d2h_hot_count},"
        f"{direct_d2h_hot_wait_ms:.3f},{direct_d2h_hot_wait_pct:.3f},"
        f"{direct_d2h_hint},"
        f"{cuda_setup_init_calls},{cuda_setup_init_wait_ms:.3f},"
        f"{cuda_setup_init_max_wait_ms:.3f},{cuda_setup_cache_hits},"
        f"{cuda_setup_cache_hit_wait_ms:.3f},"
        f"{cuda_setup_cache_hit_max_wait_ms:.3f},"
        f"{cuda_setup_native_init_calls},{cuda_setup_native_init_wait_ms:.3f},"
        f"{cuda_setup_native_init_max_wait_ms:.3f},"
        f"{cuda_current_device_calls},{cuda_current_device_wait_ms:.3f},"
        f"{cuda_current_device_max_wait_ms:.3f},"
        f"{cuda_memory_info_calls},{cuda_memory_info_wait_ms:.3f},"
        f"{cuda_memory_info_max_wait_ms:.3f},"
        f"{cuda_allocator_malloc_calls},{cuda_allocator_malloc_wait_ms:.3f},"
        f"{cuda_allocator_malloc_max_wait_ms:.3f},"
        f"{cuda_allocator_h2d_hot_bytes},{cuda_allocator_h2d_hot_count},"
        f"{cuda_allocator_h2d_hot_wait_ms:.3f},"
        f"{cuda_allocator_h2d_hot_wait_pct:.3f},"
        f"{cuda_allocator_h2d_second_hot_bytes},"
        f"{cuda_allocator_h2d_second_hot_count},"
        f"{cuda_allocator_h2d_second_hot_wait_ms:.3f},"
        f"{cuda_allocator_h2d_second_hot_wait_pct:.3f},"
        f"{cuda_allocator_d2h_bytes},{cuda_allocator_d2h_wait_ms:.3f},"
        f"{cuda_allocator_d2h_hot_bytes},{cuda_allocator_d2h_hot_count},"
        f"{cuda_allocator_d2h_hot_wait_ms:.3f},"
        f"{cuda_allocator_d2h_hot_wait_pct:.3f},{cuda_allocator_d2h_hint},"
        f"{cuda_allocator_event_sync_calls},{cuda_allocator_event_sync_bytes},"
        f"{cuda_allocator_event_sync_max_bytes},"
        f"{cuda_allocator_event_sync_wait_ms:.3f},"
        f"{cuda_allocator_event_sync_max_wait_ms:.3f},"
        f"{cuda_allocator_event_sync_hot_bytes},"
        f"{cuda_allocator_event_sync_hot_count},"
        f"{cuda_allocator_event_sync_hot_wait_ms:.3f},"
        f"{cuda_allocator_event_sync_hot_wait_pct:.3f},"
        f"{cuda_allocator_cached_reuse_count},"
        f"{cuda_allocator_pending_reuse_count},"
        f"{cuda_allocator_no_wait_bypass_count},"
        f"{cuda_allocator_no_wait_bypass_bytes},{cuda_allocator_reuse_hint},"
        f"{cuda_host_register_wait_ms:.3f},{cuda_host_unregister_wait_ms:.3f},"
        f"{cuda_host_registration_total_wait_ms:.3f},{cuda_h2d_bytes},"
        f"{cuda_copy_site_summary_fields(values)},{cuda_copy_site_hint},"
        f"{cuda_transfer_hint},"
        f"{data_residency_hint},"
        f"{copy_summary_gpu_residency_hint},{copy_summary_h2d_bulk_app_frame_hint},"
        f"{copy_summary_small_d2h_batching_hint},"
        f"{copy_summary_cuda_api_backtrace_hint},"
        f"{copy_summary_host_registration_api_ms},{copy_summary_host_registration_hint},"
        f"{kernel_graph_fusion_priority_hint},{kernel_next_action_hint},"
        f"{kernel_graph_fusion_upper_bound_ms},"
        f"{kernel_top_stream_idle_ms},{kernel_separation_hint},"
        f"{csv_cell(kernel_top_stream_idle_gap_previous)},"
        f"{csv_cell(kernel_top_stream_idle_gap_next)},"
        f"{kernel_top_stream_idle_gap_calls},{kernel_top_stream_idle_gap_ms},"
        f"{kernel_stream_idle_boundary},"
        f"{ncu_metric_collection_hint},{csv_cell(ncu_top_kernel)},"
        f"{ncu_top_kernel_duration_ms},{ncu_top_kernel_sm_throughput_pct},"
        f"{ncu_top_kernel_dram_throughput_pct},"
        f"{ncu_top_kernel_registers_per_thread},"
        f"{csv_cell(ncu_top_kernel_limiting_factors)},"
        f"{ncu_top_kernel_separation_hint},{ncu_descriptor_expansion_hint},"
        f"{segment_commit_cuda_memory_total_bytes},"
        f"{segment_commit_cuda_memory_initial_free_bytes},"
        f"{segment_commit_cuda_memory_effective_free_bytes},"
        f"{segment_commit_cuda_memory_min_free_bytes},"
        f"{segment_commit_cuda_memory_sample_ms},"
        f"{segment_commit_cuda_memory_sample_count},"
        f"{segment_commit_cuda_allocator_initial_cached_bytes},"
        f"{segment_commit_cuda_allocator_effective_cached_bytes},"
        f"{segment_commit_cuda_memory_min_free_pct:.3f},"
        f"{segment_commit_memory_hint},"
        f"{segment_commit_memory_diagnostic},"
        f"{descriptor_retention_attempts},{descriptor_retention_retained},"
        f"{descriptor_retention_rejected},{descriptor_retention_retained_bytes},"
        f"{descriptor_retention_rejected_bytes},{descriptor_retention_limit_bytes},"
        f"{external_op_row_pct:.3f},{copy_row_pct:.3f},{trace_shape_row_mix},"
        f"{external_op_row_lower_ms},{copy_row_lower_ms},"
        f"{external_op_row_lower_ns_per_row:.3f},"
        f"{copy_row_lower_ns_per_row:.3f},"
        f"{external_op_row_lower_pct:.3f},{copy_row_lower_pct:.3f},"
        f"{trace_shape_duration},{trace_shape_unit_cost},"
        f"{trace_report_source_value_record_ns_per_row:.3f},"
        f"{trace_report_source_values_residual_ns_per_row:.3f},"
        f"{trace_report_row_validation_timer_bookkeeping_ns_per_row:.3f},"
        f"{trace_report_row_validation_residual_ns_per_row:.3f},"
        f"{trace_report_visit_residual_ns_per_row:.3f},"
        f"{trace_report_descriptor_ns_per_row:.3f},"
        f"{external_op_runs},{external_op_avg_run:.3f},"
        f"{external_op_max_run},{copy_runs},{copy_avg_run:.3f},"
        f"{copy_max_run},{trace_shape_run},"
        f"{trace_pipeline_hint},{performance_focus},{trace_shape_profile},"
        f"{fri_opening_unit_build_scope_pct:.3f},"
        f"{fri_opening_layer_tree_nested_pct:.3f},"
        f"{fri_opening_query_nested_pct:.3f},"
        f"{fri_opening_fold_nested_pct:.3f},"
        f"{fri_opening_known_nested_ms},{fri_opening_known_nested_pct:.3f},"
        f"{fri_opening_unit_build_residual_ms},"
        f"{fri_opening_unit_build_residual_pct:.3f},{fri_opening_scope_hint}"
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


PROFILE_HEADER_FIELDS = tuple(next(csv.reader([HEADER])))


def profile_summary_map(row: str) -> dict[str, str]:
    fields = next(csv.reader([row]))
    if len(fields) != len(PROFILE_HEADER_FIELDS):
        raise ValueError("profile summary row does not match the header")
    return dict(zip(PROFILE_HEADER_FIELDS, fields))


def aggregate_mean_suffix(profile_rows: list[dict[str, str]]) -> str:
    if not profile_rows:
        return "," + ",".join("0.000" for _ in AGGREGATE_MEAN_PROFILE_COLUMNS)
    means: list[str] = []
    for column in AGGREGATE_MEAN_PROFILE_COLUMNS:
        total = sum(float(row.get(column, "0") or 0.0) for row in profile_rows)
        means.append(f"{total / len(profile_rows):.3f}")
    return "," + ",".join(means)


def summarize_total_samples(
    parsed_inputs: list[tuple[str, dict[str, int], dict[str, str]]],
) -> str:
    total_count = len(parsed_inputs)
    valid_inputs = [
        (label, values, profile_row)
        for label, values, profile_row in parsed_inputs
        if values.get(TOTAL_MS_KEY, 0) > 0 and not is_diagnostic_shape_profile(values)
    ]
    totals = [
        values[TOTAL_MS_KEY]
        for _, values, _ in valid_inputs
    ]
    valid_total_count = len(totals)
    if not totals:
        return (
            f"aggregate,{total_count},0,0,0.000,0.000,0,0.000,no,no,"
            "none,no,none,no,none,no,none,no,none,no,none,no,none,no"
            f"{aggregate_mean_suffix([])}"
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
        for _, values, _ in valid_inputs
    ]
    dominant_action_hint, action_consensus = dominant_hint_and_consensus(action_hints)
    trace_structure_hints = [
        trace_structure_hint_from_values(values)
        for _, values, _ in valid_inputs
    ]
    dominant_trace_structure_hint, trace_structure_consensus = (
        dominant_hint_and_consensus(trace_structure_hints)
    )
    transfer_hints = [
        cuda_transfer_action_hint_from_values(values)
        for _, values, _ in valid_inputs
    ]
    dominant_transfer_hint, transfer_consensus = dominant_hint_and_consensus(transfer_hints)
    segment_memory_hints = [
        segment_commit_memory_pressure_hint_from_values(values)
        for _, values, _ in valid_inputs
    ]
    dominant_segment_memory_hint, segment_memory_consensus = (
        dominant_hint_and_consensus(segment_memory_hints)
    )
    segment_memory_diagnostic_hints = [
        segment_commit_memory_diagnostic_hint(
            values.get(SEGMENT_COMMIT_MS_KEY, 0),
            segment_commit_memory_pressure_hint_from_values(values),
        )
        for _, values, _ in valid_inputs
    ]
    dominant_segment_memory_diagnostic, segment_memory_diagnostic_consensus = (
        dominant_hint_and_consensus(segment_memory_diagnostic_hints)
    )
    runner_detail_hotspots = [
        profile_row.get("trace_runner_detail_hotspot", "none") or "none"
        for _, _, profile_row in valid_inputs
    ]
    dominant_runner_detail_hotspot, runner_detail_hotspot_consensus = (
        dominant_hint_and_consensus(runner_detail_hotspots)
    )
    runner_detail_action_hints = [
        profile_row.get("trace_runner_detail_action_hint", "none") or "none"
        for _, _, profile_row in valid_inputs
    ]
    dominant_runner_detail_action, runner_detail_action_consensus = (
        dominant_hint_and_consensus(runner_detail_action_hints)
    )
    mean_suffix = aggregate_mean_suffix([profile_row for _, _, profile_row in valid_inputs])
    return (
        f"aggregate,{total_count},{valid_total_count},{total_min_ms},"
        f"{total_mean_ms:.3f},{total_median_ms:.3f},{total_max_ms},"
        f"{sample_spread_pct:.3f},{close_samples},{max_outlier},"
        f"{dominant_action_hint},{action_consensus},"
        f"{dominant_trace_structure_hint},{trace_structure_consensus},"
        f"{dominant_transfer_hint},{transfer_consensus},"
        f"{dominant_segment_memory_hint},{segment_memory_consensus},"
        f"{dominant_segment_memory_diagnostic},{segment_memory_diagnostic_consensus},"
        f"{dominant_runner_detail_hotspot},{runner_detail_hotspot_consensus},"
        f"{dominant_runner_detail_action},{runner_detail_action_consensus}"
        f"{mean_suffix}"
    )


def summarize_total_samples_by_input_bytes(
    input_bytes: int,
    parsed_inputs: list[tuple[str, dict[str, int], dict[str, str]]],
) -> str:
    summary = summarize_total_samples(parsed_inputs)
    return f"aggregate_by_input_bytes,{input_bytes},{summary.split(',', 1)[1]}"


def grouped_total_samples_by_input_bytes(
    parsed_inputs: list[tuple[str, dict[str, int], dict[str, str]]],
) -> list[tuple[int, list[tuple[str, dict[str, int], dict[str, str]]]]]:
    groups: dict[int, list[tuple[str, dict[str, int], dict[str, str]]]] = {}
    order: list[int] = []
    for label, values, profile_row in parsed_inputs:
        input_bytes = values.get(INPUT_BYTES_KEY, 0)
        if input_bytes not in groups:
            groups[input_bytes] = []
            order.append(input_bytes)
        groups[input_bytes].append((label, values, profile_row))
    return [(input_bytes, groups[input_bytes]) for input_bytes in order]


def print_summary(inputs: list[tuple[str, str]]) -> None:
    parsed_inputs = [
        (label, parse_timing_log(text), parse_perf_self_hotspots(text))
        for label, text in inputs
    ]
    print(HEADER)
    aggregate_inputs = []
    for label, values, perf_hotspots in parsed_inputs:
        row = summarize_profile_values(label, values, perf_hotspots)
        print(row)
        aggregate_inputs.append((label, values, profile_summary_map(row)))
    if len(parsed_inputs) > 1:
        print(AGGREGATE_HEADER)
        print(summarize_total_samples(aggregate_inputs))
        grouped_inputs = grouped_total_samples_by_input_bytes(aggregate_inputs)
        if len(grouped_inputs) > 1:
            print(AGGREGATE_BY_INPUT_BYTES_HEADER)
            for input_bytes, group in grouped_inputs:
                print(summarize_total_samples_by_input_bytes(input_bytes, group))


def stable_summary_aggregate(text: str) -> tuple[list[str], list[str]] | None:
    try:
        rows = list(csv.reader(text.splitlines()))
    except csv.Error:
        return None
    if len(rows) < 3:
        return None
    header = rows[0]
    if not header or header[0] != "profile":
        return None
    aggregate_rows = [row for row in rows if row and row[0] == "aggregate"]
    if len(aggregate_rows) < 2:
        return None
    names = aggregate_rows[-2][1:]
    values = aggregate_rows[-1][1:]
    if not names or len(names) != len(values) or any(not name for name in names):
        return None
    return names, values


def print_stable_summary_aggregates(inputs: list[tuple[str, str]]) -> bool:
    aggregate_names: list[str] | None = None
    aggregate_rows: list[tuple[str, list[str]]] = []
    for label, text in inputs:
        aggregate = stable_summary_aggregate(text)
        if aggregate is None:
            return False
        names, values = aggregate
        if aggregate_names is None:
            aggregate_names = names
        elif aggregate_names != names:
            raise SystemExit("stable summary aggregate columns differ")
        aggregate_rows.append((label, values))

    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(["profile", *(aggregate_names or [])])
    for label, values in aggregate_rows:
        writer.writerow([label, *values])
    return True


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
                        f"{SEGMENT_INPUT_GAP_MS_KEY}=1234",
                        f"{SEGMENT_INPUT_GAP_MAX_MS_KEY}=800",
                        f"{SEGMENT_INPUT_GAP_COUNT_KEY}=22",
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
                        f"{MAIN_REPORT_FAST_PATHS_KEY}=90000000",
                        f"{MAIN_REPORT_GENERIC_FALLBACKS_KEY}=3843537",
                        f"{MAIN_REPORT_FCALL_RESULT_FAST_PATHS_KEY}=1000",
                        f"{MAIN_REPORT_LOAD_COPY_FAST_PATHS_KEY}=30000000",
                        f"{MAIN_REPORT_LOAD_SIGN_EXTEND_FAST_PATHS_KEY}=2000000",
                        f"{MAIN_REPORT_NO_MEMORY_FAST_PATHS_KEY}=40000000",
                        f"{MAIN_REPORT_STORE_COPY_FAST_PATHS_KEY}=10000000",
                        f"{MAIN_REPORT_SIMPLE_COPY_FAST_PATHS_KEY}=5000000",
                        f"{MAIN_REPORT_JUMP_FAST_PATHS_KEY}=2999000",
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
                        "timing_cuda_copy_site_h2d_top_1_load_alpha_rs_12_calls=4",
                        "timing_cuda_copy_site_h2d_top_1_load_alpha_rs_12_bytes=4096",
                        "timing_cuda_copy_site_h2d_top_1_load_alpha_rs_12_max_bytes=1024",
                        "timing_cuda_copy_site_h2d_top_1_load_alpha_rs_12_wait_ns=8000000",
                        "timing_cuda_copy_site_h2d_top_1_load_alpha_rs_12_max_wait_ns=3000000",
                        "timing_cuda_copy_site_h2d_top_1_load_alpha_rs_12_avg_wait_per_call_ns=2000000",
                        "timing_cuda_copy_site_d2h_top_1_store_beta_rs_34_calls=2",
                        "timing_cuda_copy_site_d2h_top_1_store_beta_rs_34_bytes=2048",
                        "timing_cuda_copy_site_d2h_top_1_store_beta_rs_34_max_bytes=1024",
                        "timing_cuda_copy_site_d2h_top_1_store_beta_rs_34_wait_ns=6000000",
                        "timing_cuda_copy_site_d2h_top_1_store_beta_rs_34_max_wait_ns=4000000",
                        "timing_cuda_copy_site_d2h_top_1_store_beta_rs_34_avg_wait_per_call_ns=3000000",
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
        input_path.with_suffix(".cpu-summary.csv"),
        input_path.with_suffix(".cpu.txt"),
        input_path.with_suffix(".cpu.csv"),
        input_path.with_name(f"{input_path.name}.cpu-summary.txt"),
        input_path.with_name(f"{input_path.name}.cpu-summary.csv"),
        input_path.with_name(f"{input_path.name}.cpu.txt"),
        input_path.with_name(f"{input_path.name}.cpu.csv"),
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
    inputs = [read_input(path, extra_reports) for path in args.logs]
    if not extra_reports and print_stable_summary_aggregates(inputs):
        return
    print_summary(inputs)


if __name__ == "__main__":
    main()

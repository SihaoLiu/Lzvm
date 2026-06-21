use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::mem::size_of;
use std::sync::mpsc;
#[cfg(feature = "cuda")]
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::guest_instruction::{
    RiscvAmoKind, RiscvAmoWidth, RiscvDmaKind, RiscvInstruction, RiscvOpImmKind, RiscvOpKind,
    RiscvPrecompileKind,
};
use crate::guest_machine::{
    advance_guest_machine_with_prepared_fcalls, decode_current_guest_instruction,
    prepare_current_guest_instruction, run_guest_machine_trace_with_fcalls,
    run_guest_machine_with_fcalls, GuestDmaProofValueFlags, GuestFcallHandler, GuestMachineHalt,
    GuestMachineMemory, GuestMachineReport, GuestMachineRunError, GuestMachineState,
    GuestMachineTraceSliceStatus, GuestMemoryAccess, GuestMemoryAccessKind, GuestMemoryAccessList,
    GuestPrecompileMemoryAccessList, GuestRegisterWrite, GuestRegisterWriteList,
};
use crate::guest_memory::{load_guest_memory_image, GuestMemoryError};
use crate::witness_layout::{ResolvedTraceColumn, WitnessTraceBuildError, WitnessTraceLayout};
use crate::witness_loader::{
    WitnessBackend, WitnessCallError, WitnessComputeContext, WitnessTraceBuffers,
    WitnessTraceOutput, WitnessTraceProofValue, WitnessTraceUnitValue,
};
use crate::witness_runner::{WitnessTraceRequest, WitnessTraceRunError, WitnessTraceRunOutput};
use crate::witness_trace::WitnessTraceBuffer;
use crate::zisk_main::{
    lower_guest_report, ZiskMainInstruction, ZiskMainLowerError, ZiskMainOp, ZiskMainSource,
    ZiskMainStore, ZISK_EXTRA_PARAMS_ADDRESS,
};
#[cfg(feature = "cuda")]
use lzvm_accel::{CudaDeviceBuffer, CudaStream};
use lzvm_field::{Felt, FieldError};

use crate::zisk_fcalls::{ZiskInputFcallError, ZiskInputFcallHandler, ZISK_INPUT_ADDRESS};

mod precompile_memory_trace;

const ZISK_RAM_ADDRESS: u64 = 0xa000_0000;
const ZISK_RAM_SIZE: u64 = 0x2000_0000;
const ZISK_MAIN_REGISTER_START: usize = 1;
const ZISK_MAIN_REGISTER_COUNT: usize = 31;
const ZISK_MAIN_RESERVED_MEM_STEPS: u64 = 1;
const ZISK_MAIN_MEM_STEPS_PER_ROW: u64 = 4;
const ZISK_MAIN_A_MEM_STEP_OFFSET: u64 = 0;
const ZISK_MAIN_B_MEM_STEP_OFFSET: u64 = 1;
const ZISK_MAIN_STORE_MEM_STEP_OFFSET: u64 = 2;
const ZISK_MAIN_SPECIAL_MEM_STEP_OFFSET: u64 = 3;
const ZISK_AMO_TEMP_REGISTER: u8 = 32;
pub(crate) const ZISK_MAIN_ROW_SHAPE_TOP_PATTERN_COUNT: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestPcTraceBackend {
    instruction_limit: u64,
}

impl GuestPcTraceBackend {
    pub fn new(instruction_limit: u64) -> Self {
        Self { instruction_limit }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestPcTraceSegmentRunOutput {
    trace_instance_index: u32,
    trace_source_prefix_rows: usize,
    #[cfg(feature = "cuda")]
    device_segment_material: Option<GuestPcTraceDeviceSegmentMaterial>,
    trace: Option<WitnessTraceBuffer>,
    unit_values: Vec<WitnessTraceUnitValue>,
    proof_values: Vec<WitnessTraceProofValue>,
}

impl GuestPcTraceSegmentRunOutput {
    fn from_segment_trace(segment: GuestPcTraceSegmentTrace) -> Self {
        Self {
            trace_instance_index: segment.trace_instance_index,
            trace_source_prefix_rows: segment.trace_source_prefix_rows,
            #[cfg(feature = "cuda")]
            device_segment_material: segment.device_segment_material,
            trace: segment.trace,
            unit_values: segment.unit_values,
            proof_values: segment.proof_values,
        }
    }

    pub fn trace_instance_index(&self) -> u32 {
        self.trace_instance_index
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn trace_source_prefix_rows(&self) -> usize {
        self.trace_source_prefix_rows
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn device_trace_descriptors(&self) -> Option<&ZiskMainDeviceTraceDescriptors> {
        self.device_segment_material
            .as_ref()
            .map(|material| &material.device_trace_descriptors)
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn device_segment_material(&self) -> Option<&GuestPcTraceDeviceSegmentMaterial> {
        self.device_segment_material.as_ref()
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn into_trace_and_device_material(
        self,
    ) -> (
        Option<WitnessTraceBuffer>,
        Option<GuestPcTraceDeviceSegmentMaterial>,
    ) {
        (self.trace, self.device_segment_material)
    }

    pub fn trace_if_available(&self) -> Option<&WitnessTraceBuffer> {
        self.trace.as_ref()
    }

    pub fn trace(&self) -> &WitnessTraceBuffer {
        self.trace
            .as_ref()
            .expect("guest PC segment host trace is not available")
    }

    pub fn unit_values(&self) -> &[WitnessTraceUnitValue] {
        &self.unit_values
    }

    pub fn proof_values(&self) -> &[WitnessTraceProofValue] {
        &self.proof_values
    }

    pub fn into_trace(self) -> Option<WitnessTraceBuffer> {
        self.trace
    }

    pub fn into_output(self) -> Option<WitnessTraceRunOutput> {
        Some(WitnessTraceRunOutput::from_parts(
            self.trace?,
            self.unit_values,
            self.proof_values,
        ))
    }
}

pub fn run_guest_pc_trace_segments_with_context(
    backend: &GuestPcTraceBackend,
    context: WitnessComputeContext<'_>,
    request: WitnessTraceRequest<'_>,
) -> Result<Vec<GuestPcTraceSegmentRunOutput>, WitnessTraceRunError> {
    let layout = context
        .trace_layout
        .ok_or_else(|| WitnessCallError::Backend {
            message: "guest PC trace segmented backend requires trace layout".to_owned(),
        })?;
    if request.rows != layout.row_count() || request.columns != layout.column_count() {
        return Err(WitnessCallError::Backend {
            message: format!(
                "guest PC trace segmented request shape mismatch: layout {}x{}, request {}x{}",
                layout.row_count(),
                layout.column_count(),
                request.rows,
                request.columns
            ),
        }
        .into());
    }
    let segments =
        compute_guest_pc_trace_segments(backend.instruction_limit, context, request.input.as_ref())
            .map_err(WitnessCallError::from)?;
    let mut out = Vec::with_capacity(segments.len());
    for segment in segments {
        out.push(GuestPcTraceSegmentRunOutput::from_segment_trace(segment));
    }
    Ok(out)
}

pub(crate) enum GuestPcTraceSegmentStreamError<E> {
    Trace(WitnessTraceRunError),
    Emit(E),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GuestPcTraceStreamTiming {
    runner_duration: Duration,
    lowerer_duration: Duration,
    trace_lower_duration: Duration,
    trace_report_duration: Duration,
    trace_report_apply_duration: Duration,
    trace_unit_summary_duration: Duration,
    trace_report_sample_duration: Duration,
    trace_single_row_report_duration: Duration,
    trace_multi_row_report_duration: Duration,
    trace_pending_dma_report_duration: Duration,
    trace_amo_report_duration: Duration,
    trace_store_conditional_report_duration: Duration,
    trace_external_op_row_duration: Duration,
    trace_copy_row_duration: Duration,
    trace_report_lowering_duration: Duration,
    trace_report_row_validation_duration: Duration,
    trace_report_memory_columns_duration: Duration,
    trace_report_source_values_duration: Duration,
    trace_report_source_a_value_duration: Duration,
    trace_report_source_b_value_duration: Duration,
    trace_report_source_value_record_duration: Duration,
    trace_report_source_immediate_read_duration: Duration,
    trace_report_source_register_read_duration: Duration,
    trace_report_source_memory_read_duration: Duration,
    trace_report_source_indirect_read_duration: Duration,
    trace_report_source_last_c_read_duration: Duration,
    trace_copy_source_memory_read_duration: Duration,
    trace_copy_source_indirect_read_duration: Duration,
    trace_report_precompile_memory_duration: Duration,
    trace_report_instruction_result_duration: Duration,
    trace_report_next_pc_duration: Duration,
    trace_report_register_access_duration: Duration,
    trace_report_memory_access_duration: Duration,
    trace_report_store_apply_duration: Duration,
    trace_report_visit_duration: Duration,
    trace_emit_duration: Duration,
    trace_descriptor_duration: Duration,
    trace_report_detail_sample_count: usize,
    trace_report_source_immediate_read_count: usize,
    trace_report_source_register_read_count: usize,
    trace_report_source_memory_read_count: usize,
    trace_report_source_indirect_read_count: usize,
    trace_report_source_last_c_read_count: usize,
    trace_copy_source_memory_read_count: usize,
    trace_copy_source_indirect_read_count: usize,
    segment_replay_count: usize,
    seed_direct_lift_duration: Duration,
    seed_full_advance_duration: Duration,
    pending_send_wait_duration: Duration,
    pending_receive_wait_duration: Duration,
    segment_send_wait_duration: Duration,
    segment_receive_wait_duration: Duration,
    parallel_lower_worker_count: usize,
    parallel_lower_dispatched_count: usize,
    parallel_lower_received_count: usize,
    parallel_lower_emitted_count: usize,
    parallel_lower_max_reorder_count: usize,
    parallel_lower_snapshot_replay_count: usize,
    parallel_lower_snapshot_replay_duration: Duration,
    parallel_lower_report_elided_count: usize,
    parallel_lower_stream_segment_count: usize,
    parallel_lower_stream_chunk_count: usize,
    parallel_lower_stream_fallback_count: usize,
    parallel_lower_stream_retained_report_count: usize,
    parallel_lower_stream_chunk_process_duration: Duration,
    parallel_lower_job_receive_wait_duration: Duration,
    parallel_lower_result_send_wait_duration: Duration,
    parallel_lower_dispatch_wait_duration: Duration,
    parallel_lower_stream_start_dispatch_wait_duration: Duration,
    parallel_lower_stream_chunk_dispatch_wait_duration: Duration,
    parallel_lower_stream_segment_dispatch_wait_duration: Duration,
    parallel_lower_stream_finish_dispatch_wait_duration: Duration,
    parallel_lower_result_receive_wait_duration: Duration,
    parallel_lower_dispatch_blocked_count: usize,
    seed_direct_lift_attempt_count: usize,
    seed_direct_lift_success_count: usize,
    seed_direct_lift_empty_segment_count: usize,
    seed_direct_lift_pending_dma_single_report_count: usize,
    seed_direct_lift_amo_boundary_count: usize,
    seed_direct_lift_store_conditional_boundary_count: usize,
    seed_direct_lift_dma_prepare_missing_lookahead_count: usize,
    seed_direct_lift_boundary_c_unavailable_count: usize,
    seed_full_advance_count: usize,
    trace_report_count: usize,
    trace_report_row_count: usize,
    trace_stream_start_sent_count: usize,
    trace_report_chunk_sent_count: usize,
    trace_report_chunk_received_count: usize,
    trace_report_chunk_report_count: usize,
    trace_report_chunk_row_count: usize,
    trace_report_chunk_max_queued_count: usize,
    trace_runner_report_buffer_capacity: usize,
    trace_runner_report_buffer_max_capacity: usize,
    trace_runner_report_buffer_excess_capacity: usize,
    trace_report_buffer_capacity: usize,
    trace_report_buffer_max_capacity: usize,
    trace_report_buffer_excess_capacity: usize,
    trace_descriptor_row_count: usize,
    trace_descriptor_compact_row_count: usize,
    trace_descriptor_wide_row_count: usize,
    trace_descriptor_unpaired_value_count: usize,
    trace_descriptor_unpaired_high32_nonzero_count: usize,
    trace_descriptor_unpaired_high32_nonzero_row_count: usize,
    trace_descriptor_high32_field_counts: [u32; ZISK_MAIN_UNPAIRED_DESCRIPTOR_FIELD_COUNT],
    trace_descriptor_high32_row_field_histogram:
        [u32; ZISK_MAIN_UNPAIRED_DESCRIPTOR_HIGH32_HISTOGRAM_BUCKETS],
    trace_single_row_report_count: usize,
    trace_multi_row_report_count: usize,
    trace_pending_dma_report_count: usize,
    trace_amo_report_count: usize,
    trace_store_conditional_report_count: usize,
    trace_external_op_row_count: usize,
    trace_external_op_run_count: usize,
    trace_external_op_current_run_count: usize,
    trace_external_op_max_run_count: usize,
    trace_copy_row_count: usize,
    trace_copy_run_count: usize,
    trace_copy_current_run_count: usize,
    trace_copy_max_run_count: usize,
    trace_copy_memory_source_row_count: usize,
    trace_copy_indirect_memory_row_count: usize,
    trace_copy_register_store_row_count: usize,
    trace_copy_memory_store_row_count: usize,
    trace_copy_no_store_row_count: usize,
    trace_copy_no_memory_row_count: usize,
    trace_flag_row_count: usize,
    trace_precompile_row_count: usize,
    trace_indirect_memory_row_count: usize,
    trace_register_source_read_count: usize,
    trace_memory_source_read_count: usize,
    trace_register_store_row_count: usize,
    trace_memory_store_row_count: usize,
    trace_no_store_row_count: usize,
    trace_row_shape_pattern_counts: BTreeMap<u64, usize>,
}

impl GuestPcTraceStreamTiming {
    fn add(&mut self, other: Self) {
        self.runner_duration += other.runner_duration;
        self.lowerer_duration += other.lowerer_duration;
        self.trace_lower_duration += other.trace_lower_duration;
        self.trace_report_duration += other.trace_report_duration;
        self.trace_report_apply_duration += other.trace_report_apply_duration;
        self.trace_unit_summary_duration += other.trace_unit_summary_duration;
        self.trace_report_sample_duration += other.trace_report_sample_duration;
        self.trace_single_row_report_duration += other.trace_single_row_report_duration;
        self.trace_multi_row_report_duration += other.trace_multi_row_report_duration;
        self.trace_pending_dma_report_duration += other.trace_pending_dma_report_duration;
        self.trace_amo_report_duration += other.trace_amo_report_duration;
        self.trace_store_conditional_report_duration +=
            other.trace_store_conditional_report_duration;
        self.trace_external_op_row_duration += other.trace_external_op_row_duration;
        self.trace_copy_row_duration += other.trace_copy_row_duration;
        self.trace_report_lowering_duration += other.trace_report_lowering_duration;
        self.trace_report_row_validation_duration += other.trace_report_row_validation_duration;
        self.trace_report_memory_columns_duration += other.trace_report_memory_columns_duration;
        self.trace_report_source_values_duration += other.trace_report_source_values_duration;
        self.trace_report_source_a_value_duration += other.trace_report_source_a_value_duration;
        self.trace_report_source_b_value_duration += other.trace_report_source_b_value_duration;
        self.trace_report_source_value_record_duration +=
            other.trace_report_source_value_record_duration;
        self.trace_report_source_immediate_read_duration +=
            other.trace_report_source_immediate_read_duration;
        self.trace_report_source_register_read_duration +=
            other.trace_report_source_register_read_duration;
        self.trace_report_source_memory_read_duration +=
            other.trace_report_source_memory_read_duration;
        self.trace_report_source_indirect_read_duration +=
            other.trace_report_source_indirect_read_duration;
        self.trace_report_source_last_c_read_duration +=
            other.trace_report_source_last_c_read_duration;
        self.trace_copy_source_memory_read_duration += other.trace_copy_source_memory_read_duration;
        self.trace_copy_source_indirect_read_duration +=
            other.trace_copy_source_indirect_read_duration;
        self.trace_report_precompile_memory_duration +=
            other.trace_report_precompile_memory_duration;
        self.trace_report_instruction_result_duration +=
            other.trace_report_instruction_result_duration;
        self.trace_report_next_pc_duration += other.trace_report_next_pc_duration;
        self.trace_report_register_access_duration += other.trace_report_register_access_duration;
        self.trace_report_memory_access_duration += other.trace_report_memory_access_duration;
        self.trace_report_store_apply_duration += other.trace_report_store_apply_duration;
        self.trace_report_visit_duration += other.trace_report_visit_duration;
        self.trace_emit_duration += other.trace_emit_duration;
        self.trace_descriptor_duration += other.trace_descriptor_duration;
        self.trace_report_detail_sample_count += other.trace_report_detail_sample_count;
        self.trace_report_source_immediate_read_count +=
            other.trace_report_source_immediate_read_count;
        self.trace_report_source_register_read_count +=
            other.trace_report_source_register_read_count;
        self.trace_report_source_memory_read_count += other.trace_report_source_memory_read_count;
        self.trace_report_source_indirect_read_count +=
            other.trace_report_source_indirect_read_count;
        self.trace_report_source_last_c_read_count += other.trace_report_source_last_c_read_count;
        self.trace_copy_source_memory_read_count += other.trace_copy_source_memory_read_count;
        self.trace_copy_source_indirect_read_count += other.trace_copy_source_indirect_read_count;
        self.segment_replay_count += other.segment_replay_count;
        self.seed_direct_lift_duration += other.seed_direct_lift_duration;
        self.seed_full_advance_duration += other.seed_full_advance_duration;
        self.pending_send_wait_duration += other.pending_send_wait_duration;
        self.pending_receive_wait_duration += other.pending_receive_wait_duration;
        self.segment_send_wait_duration += other.segment_send_wait_duration;
        self.segment_receive_wait_duration += other.segment_receive_wait_duration;
        self.parallel_lower_worker_count = self
            .parallel_lower_worker_count
            .max(other.parallel_lower_worker_count);
        self.parallel_lower_dispatched_count += other.parallel_lower_dispatched_count;
        self.parallel_lower_received_count += other.parallel_lower_received_count;
        self.parallel_lower_emitted_count += other.parallel_lower_emitted_count;
        self.parallel_lower_max_reorder_count = self
            .parallel_lower_max_reorder_count
            .max(other.parallel_lower_max_reorder_count);
        self.parallel_lower_snapshot_replay_count += other.parallel_lower_snapshot_replay_count;
        self.parallel_lower_snapshot_replay_duration +=
            other.parallel_lower_snapshot_replay_duration;
        self.parallel_lower_report_elided_count += other.parallel_lower_report_elided_count;
        self.parallel_lower_stream_segment_count += other.parallel_lower_stream_segment_count;
        self.parallel_lower_stream_chunk_count += other.parallel_lower_stream_chunk_count;
        self.parallel_lower_stream_fallback_count += other.parallel_lower_stream_fallback_count;
        self.parallel_lower_stream_retained_report_count +=
            other.parallel_lower_stream_retained_report_count;
        self.parallel_lower_stream_chunk_process_duration +=
            other.parallel_lower_stream_chunk_process_duration;
        self.parallel_lower_job_receive_wait_duration +=
            other.parallel_lower_job_receive_wait_duration;
        self.parallel_lower_result_send_wait_duration +=
            other.parallel_lower_result_send_wait_duration;
        self.parallel_lower_dispatch_wait_duration += other.parallel_lower_dispatch_wait_duration;
        self.parallel_lower_stream_start_dispatch_wait_duration +=
            other.parallel_lower_stream_start_dispatch_wait_duration;
        self.parallel_lower_stream_chunk_dispatch_wait_duration +=
            other.parallel_lower_stream_chunk_dispatch_wait_duration;
        self.parallel_lower_stream_segment_dispatch_wait_duration +=
            other.parallel_lower_stream_segment_dispatch_wait_duration;
        self.parallel_lower_stream_finish_dispatch_wait_duration +=
            other.parallel_lower_stream_finish_dispatch_wait_duration;
        self.parallel_lower_result_receive_wait_duration +=
            other.parallel_lower_result_receive_wait_duration;
        self.parallel_lower_dispatch_blocked_count += other.parallel_lower_dispatch_blocked_count;
        self.seed_direct_lift_attempt_count += other.seed_direct_lift_attempt_count;
        self.seed_direct_lift_success_count += other.seed_direct_lift_success_count;
        self.seed_direct_lift_empty_segment_count += other.seed_direct_lift_empty_segment_count;
        self.seed_direct_lift_pending_dma_single_report_count +=
            other.seed_direct_lift_pending_dma_single_report_count;
        self.seed_direct_lift_amo_boundary_count += other.seed_direct_lift_amo_boundary_count;
        self.seed_direct_lift_store_conditional_boundary_count +=
            other.seed_direct_lift_store_conditional_boundary_count;
        self.seed_direct_lift_dma_prepare_missing_lookahead_count +=
            other.seed_direct_lift_dma_prepare_missing_lookahead_count;
        self.seed_direct_lift_boundary_c_unavailable_count +=
            other.seed_direct_lift_boundary_c_unavailable_count;
        self.seed_full_advance_count += other.seed_full_advance_count;
        self.trace_report_count += other.trace_report_count;
        self.trace_report_row_count += other.trace_report_row_count;
        self.trace_stream_start_sent_count += other.trace_stream_start_sent_count;
        self.trace_report_chunk_sent_count += other.trace_report_chunk_sent_count;
        self.trace_report_chunk_received_count += other.trace_report_chunk_received_count;
        self.trace_report_chunk_report_count += other.trace_report_chunk_report_count;
        self.trace_report_chunk_row_count += other.trace_report_chunk_row_count;
        self.trace_report_chunk_max_queued_count = self
            .trace_report_chunk_max_queued_count
            .max(other.trace_report_chunk_max_queued_count);
        self.trace_runner_report_buffer_capacity += other.trace_runner_report_buffer_capacity;
        self.trace_runner_report_buffer_max_capacity = self
            .trace_runner_report_buffer_max_capacity
            .max(other.trace_runner_report_buffer_max_capacity);
        self.trace_runner_report_buffer_excess_capacity +=
            other.trace_runner_report_buffer_excess_capacity;
        self.trace_report_buffer_capacity += other.trace_report_buffer_capacity;
        self.trace_report_buffer_max_capacity = self
            .trace_report_buffer_max_capacity
            .max(other.trace_report_buffer_max_capacity);
        self.trace_report_buffer_excess_capacity += other.trace_report_buffer_excess_capacity;
        self.trace_descriptor_row_count += other.trace_descriptor_row_count;
        self.trace_descriptor_compact_row_count += other.trace_descriptor_compact_row_count;
        self.trace_descriptor_wide_row_count += other.trace_descriptor_wide_row_count;
        self.trace_descriptor_unpaired_value_count += other.trace_descriptor_unpaired_value_count;
        self.trace_descriptor_unpaired_high32_nonzero_count +=
            other.trace_descriptor_unpaired_high32_nonzero_count;
        self.trace_descriptor_unpaired_high32_nonzero_row_count +=
            other.trace_descriptor_unpaired_high32_nonzero_row_count;
        for (field_count, other_count) in self
            .trace_descriptor_high32_field_counts
            .iter_mut()
            .zip(other.trace_descriptor_high32_field_counts)
        {
            *field_count = field_count.saturating_add(other_count);
        }
        for (bucket_count, other_count) in self
            .trace_descriptor_high32_row_field_histogram
            .iter_mut()
            .zip(other.trace_descriptor_high32_row_field_histogram)
        {
            *bucket_count = bucket_count.saturating_add(other_count);
        }
        self.trace_single_row_report_count += other.trace_single_row_report_count;
        self.trace_multi_row_report_count += other.trace_multi_row_report_count;
        self.trace_pending_dma_report_count += other.trace_pending_dma_report_count;
        self.trace_amo_report_count += other.trace_amo_report_count;
        self.trace_store_conditional_report_count += other.trace_store_conditional_report_count;
        self.trace_external_op_row_count += other.trace_external_op_row_count;
        self.trace_external_op_run_count += other.trace_external_op_run_count;
        self.trace_external_op_current_run_count = self
            .trace_external_op_current_run_count
            .max(other.trace_external_op_current_run_count);
        self.trace_external_op_max_run_count = self
            .trace_external_op_max_run_count
            .max(other.trace_external_op_max_run_count);
        self.trace_copy_row_count += other.trace_copy_row_count;
        self.trace_copy_run_count += other.trace_copy_run_count;
        self.trace_copy_current_run_count = self
            .trace_copy_current_run_count
            .max(other.trace_copy_current_run_count);
        self.trace_copy_max_run_count = self
            .trace_copy_max_run_count
            .max(other.trace_copy_max_run_count);
        self.trace_copy_memory_source_row_count += other.trace_copy_memory_source_row_count;
        self.trace_copy_indirect_memory_row_count += other.trace_copy_indirect_memory_row_count;
        self.trace_copy_register_store_row_count += other.trace_copy_register_store_row_count;
        self.trace_copy_memory_store_row_count += other.trace_copy_memory_store_row_count;
        self.trace_copy_no_store_row_count += other.trace_copy_no_store_row_count;
        self.trace_copy_no_memory_row_count += other.trace_copy_no_memory_row_count;
        self.trace_flag_row_count += other.trace_flag_row_count;
        self.trace_precompile_row_count += other.trace_precompile_row_count;
        self.trace_indirect_memory_row_count += other.trace_indirect_memory_row_count;
        self.trace_register_source_read_count += other.trace_register_source_read_count;
        self.trace_memory_source_read_count += other.trace_memory_source_read_count;
        self.trace_register_store_row_count += other.trace_register_store_row_count;
        self.trace_memory_store_row_count += other.trace_memory_store_row_count;
        self.trace_no_store_row_count += other.trace_no_store_row_count;
        for (id, count) in other.trace_row_shape_pattern_counts {
            self.record_trace_row_shape_pattern_count(id, count);
        }
    }

    pub fn runner_duration(&self) -> Duration {
        self.runner_duration
    }

    pub fn lowerer_duration(&self) -> Duration {
        self.lowerer_duration
    }

    pub fn trace_lower_duration(&self) -> Duration {
        self.trace_lower_duration
    }

    pub fn trace_report_duration(&self) -> Duration {
        self.trace_report_duration
    }

    pub fn trace_report_apply_duration(&self) -> Duration {
        self.trace_report_apply_duration
    }

    pub fn trace_unit_summary_duration(&self) -> Duration {
        self.trace_unit_summary_duration
    }

    pub fn trace_report_sample_duration(&self) -> Duration {
        self.trace_report_sample_duration
    }

    pub fn trace_single_row_report_duration(&self) -> Duration {
        self.trace_single_row_report_duration
    }

    pub fn trace_multi_row_report_duration(&self) -> Duration {
        self.trace_multi_row_report_duration
    }

    pub fn trace_pending_dma_report_duration(&self) -> Duration {
        self.trace_pending_dma_report_duration
    }

    pub fn trace_amo_report_duration(&self) -> Duration {
        self.trace_amo_report_duration
    }

    pub fn trace_store_conditional_report_duration(&self) -> Duration {
        self.trace_store_conditional_report_duration
    }

    pub fn trace_external_op_row_duration(&self) -> Duration {
        self.trace_external_op_row_duration
    }

    pub fn trace_copy_row_duration(&self) -> Duration {
        self.trace_copy_row_duration
    }

    pub fn trace_report_lowering_duration(&self) -> Duration {
        self.trace_report_lowering_duration
    }

    pub fn trace_report_row_validation_duration(&self) -> Duration {
        self.trace_report_row_validation_duration
    }

    pub fn trace_report_memory_columns_duration(&self) -> Duration {
        self.trace_report_memory_columns_duration
    }

    pub fn trace_report_source_values_duration(&self) -> Duration {
        self.trace_report_source_values_duration
    }

    pub fn trace_report_source_a_value_duration(&self) -> Duration {
        self.trace_report_source_a_value_duration
    }

    pub fn trace_report_source_b_value_duration(&self) -> Duration {
        self.trace_report_source_b_value_duration
    }

    pub fn trace_report_source_value_record_duration(&self) -> Duration {
        self.trace_report_source_value_record_duration
    }

    pub fn trace_report_source_immediate_read_duration(&self) -> Duration {
        self.trace_report_source_immediate_read_duration
    }

    pub fn trace_report_source_register_read_duration(&self) -> Duration {
        self.trace_report_source_register_read_duration
    }

    pub fn trace_report_source_memory_read_duration(&self) -> Duration {
        self.trace_report_source_memory_read_duration
    }

    pub fn trace_report_source_indirect_read_duration(&self) -> Duration {
        self.trace_report_source_indirect_read_duration
    }

    pub fn trace_report_source_last_c_read_duration(&self) -> Duration {
        self.trace_report_source_last_c_read_duration
    }

    pub fn trace_copy_source_memory_read_duration(&self) -> Duration {
        self.trace_copy_source_memory_read_duration
    }

    pub fn trace_copy_source_indirect_read_duration(&self) -> Duration {
        self.trace_copy_source_indirect_read_duration
    }

    pub fn trace_report_precompile_memory_duration(&self) -> Duration {
        self.trace_report_precompile_memory_duration
    }

    pub fn trace_report_instruction_result_duration(&self) -> Duration {
        self.trace_report_instruction_result_duration
    }

    pub fn trace_report_next_pc_duration(&self) -> Duration {
        self.trace_report_next_pc_duration
    }

    pub fn trace_report_register_access_duration(&self) -> Duration {
        self.trace_report_register_access_duration
    }

    pub fn trace_report_memory_access_duration(&self) -> Duration {
        self.trace_report_memory_access_duration
    }

    pub fn trace_report_store_apply_duration(&self) -> Duration {
        self.trace_report_store_apply_duration
    }

    pub fn trace_report_visit_duration(&self) -> Duration {
        self.trace_report_visit_duration
    }

    pub fn trace_emit_duration(&self) -> Duration {
        self.trace_emit_duration
    }

    pub fn trace_descriptor_duration(&self) -> Duration {
        self.trace_descriptor_duration
    }

    pub fn trace_report_detail_sample_count(&self) -> usize {
        self.trace_report_detail_sample_count
    }

    pub fn trace_report_source_immediate_read_count(&self) -> usize {
        self.trace_report_source_immediate_read_count
    }

    pub fn trace_report_source_register_read_count(&self) -> usize {
        self.trace_report_source_register_read_count
    }

    pub fn trace_report_source_memory_read_count(&self) -> usize {
        self.trace_report_source_memory_read_count
    }

    pub fn trace_report_source_indirect_read_count(&self) -> usize {
        self.trace_report_source_indirect_read_count
    }

    pub fn trace_report_source_last_c_read_count(&self) -> usize {
        self.trace_report_source_last_c_read_count
    }

    pub fn trace_copy_source_memory_read_count(&self) -> usize {
        self.trace_copy_source_memory_read_count
    }

    pub fn trace_copy_source_indirect_read_count(&self) -> usize {
        self.trace_copy_source_indirect_read_count
    }

    pub fn segment_replay_count(&self) -> usize {
        self.segment_replay_count
    }

    pub fn seed_direct_lift_duration(&self) -> Duration {
        self.seed_direct_lift_duration
    }

    pub fn seed_full_advance_duration(&self) -> Duration {
        self.seed_full_advance_duration
    }

    pub fn pending_send_wait_duration(&self) -> Duration {
        self.pending_send_wait_duration
    }

    pub fn pending_receive_wait_duration(&self) -> Duration {
        self.pending_receive_wait_duration
    }

    pub fn segment_send_wait_duration(&self) -> Duration {
        self.segment_send_wait_duration
    }

    pub fn segment_receive_wait_duration(&self) -> Duration {
        self.segment_receive_wait_duration
    }

    pub fn parallel_lower_worker_count(&self) -> usize {
        self.parallel_lower_worker_count
    }

    pub fn parallel_lower_dispatched_count(&self) -> usize {
        self.parallel_lower_dispatched_count
    }

    pub fn parallel_lower_received_count(&self) -> usize {
        self.parallel_lower_received_count
    }

    pub fn parallel_lower_emitted_count(&self) -> usize {
        self.parallel_lower_emitted_count
    }

    pub fn parallel_lower_max_reorder_count(&self) -> usize {
        self.parallel_lower_max_reorder_count
    }

    pub fn parallel_lower_snapshot_replay_count(&self) -> usize {
        self.parallel_lower_snapshot_replay_count
    }

    pub fn parallel_lower_snapshot_replay_duration(&self) -> Duration {
        self.parallel_lower_snapshot_replay_duration
    }

    pub fn parallel_lower_report_elided_count(&self) -> usize {
        self.parallel_lower_report_elided_count
    }

    pub fn parallel_lower_stream_segment_count(&self) -> usize {
        self.parallel_lower_stream_segment_count
    }

    pub fn parallel_lower_stream_chunk_count(&self) -> usize {
        self.parallel_lower_stream_chunk_count
    }

    pub fn parallel_lower_stream_fallback_count(&self) -> usize {
        self.parallel_lower_stream_fallback_count
    }

    pub fn parallel_lower_stream_retained_report_count(&self) -> usize {
        self.parallel_lower_stream_retained_report_count
    }

    pub fn parallel_lower_stream_chunk_process_duration(&self) -> Duration {
        self.parallel_lower_stream_chunk_process_duration
    }

    pub fn parallel_lower_job_receive_wait_duration(&self) -> Duration {
        self.parallel_lower_job_receive_wait_duration
    }

    pub fn parallel_lower_result_send_wait_duration(&self) -> Duration {
        self.parallel_lower_result_send_wait_duration
    }

    pub fn parallel_lower_dispatch_wait_duration(&self) -> Duration {
        self.parallel_lower_dispatch_wait_duration
    }

    pub fn parallel_lower_stream_start_dispatch_wait_duration(&self) -> Duration {
        self.parallel_lower_stream_start_dispatch_wait_duration
    }

    pub fn parallel_lower_stream_chunk_dispatch_wait_duration(&self) -> Duration {
        self.parallel_lower_stream_chunk_dispatch_wait_duration
    }

    pub fn parallel_lower_stream_segment_dispatch_wait_duration(&self) -> Duration {
        self.parallel_lower_stream_segment_dispatch_wait_duration
    }

    pub fn parallel_lower_stream_finish_dispatch_wait_duration(&self) -> Duration {
        self.parallel_lower_stream_finish_dispatch_wait_duration
    }

    pub fn parallel_lower_result_receive_wait_duration(&self) -> Duration {
        self.parallel_lower_result_receive_wait_duration
    }

    pub fn parallel_lower_dispatch_blocked_count(&self) -> usize {
        self.parallel_lower_dispatch_blocked_count
    }

    pub fn seed_direct_lift_attempt_count(&self) -> usize {
        self.seed_direct_lift_attempt_count
    }

    pub fn seed_direct_lift_success_count(&self) -> usize {
        self.seed_direct_lift_success_count
    }

    fn record_seed_direct_lift_miss(&mut self, reason: ZiskMainDirectSeedLiftMissReason) {
        match reason {
            ZiskMainDirectSeedLiftMissReason::EmptySegment => {
                self.seed_direct_lift_empty_segment_count += 1;
            }
            ZiskMainDirectSeedLiftMissReason::PendingDmaSingleReport => {
                self.seed_direct_lift_pending_dma_single_report_count += 1;
            }
            ZiskMainDirectSeedLiftMissReason::AmoBoundary => {
                self.seed_direct_lift_amo_boundary_count += 1;
            }
            ZiskMainDirectSeedLiftMissReason::StoreConditionalBoundary => {
                self.seed_direct_lift_store_conditional_boundary_count += 1;
            }
            ZiskMainDirectSeedLiftMissReason::DmaPrepareMissingLookahead => {
                self.seed_direct_lift_dma_prepare_missing_lookahead_count += 1;
            }
            ZiskMainDirectSeedLiftMissReason::BoundaryCUnavailable => {
                self.seed_direct_lift_boundary_c_unavailable_count += 1;
            }
        }
    }

    pub fn seed_direct_lift_empty_segment_count(&self) -> usize {
        self.seed_direct_lift_empty_segment_count
    }

    pub fn seed_direct_lift_pending_dma_single_report_count(&self) -> usize {
        self.seed_direct_lift_pending_dma_single_report_count
    }

    pub fn seed_direct_lift_amo_boundary_count(&self) -> usize {
        self.seed_direct_lift_amo_boundary_count
    }

    pub fn seed_direct_lift_store_conditional_boundary_count(&self) -> usize {
        self.seed_direct_lift_store_conditional_boundary_count
    }

    pub fn seed_direct_lift_dma_prepare_missing_lookahead_count(&self) -> usize {
        self.seed_direct_lift_dma_prepare_missing_lookahead_count
    }

    pub fn seed_direct_lift_boundary_c_unavailable_count(&self) -> usize {
        self.seed_direct_lift_boundary_c_unavailable_count
    }

    pub fn seed_full_advance_count(&self) -> usize {
        self.seed_full_advance_count
    }

    pub fn trace_report_count(&self) -> usize {
        self.trace_report_count
    }

    pub fn trace_report_row_count(&self) -> usize {
        self.trace_report_row_count
    }

    pub fn trace_stream_start_sent_count(&self) -> usize {
        self.trace_stream_start_sent_count
    }

    pub fn trace_report_chunk_sent_count(&self) -> usize {
        self.trace_report_chunk_sent_count
    }

    pub fn trace_report_chunk_received_count(&self) -> usize {
        self.trace_report_chunk_received_count
    }

    pub fn trace_report_chunk_report_count(&self) -> usize {
        self.trace_report_chunk_report_count
    }

    pub fn trace_report_chunk_row_count(&self) -> usize {
        self.trace_report_chunk_row_count
    }

    pub fn trace_report_chunk_max_queued_count(&self) -> usize {
        self.trace_report_chunk_max_queued_count
    }

    pub fn trace_runner_report_buffer_capacity(&self) -> usize {
        self.trace_runner_report_buffer_capacity
    }

    pub fn trace_runner_report_buffer_max_capacity(&self) -> usize {
        self.trace_runner_report_buffer_max_capacity
    }

    pub fn trace_runner_report_buffer_excess_capacity(&self) -> usize {
        self.trace_runner_report_buffer_excess_capacity
    }

    pub fn trace_report_buffer_capacity(&self) -> usize {
        self.trace_report_buffer_capacity
    }

    pub fn trace_report_buffer_max_capacity(&self) -> usize {
        self.trace_report_buffer_max_capacity
    }

    pub fn trace_report_buffer_excess_capacity(&self) -> usize {
        self.trace_report_buffer_excess_capacity
    }

    pub fn trace_report_record_size_bytes(&self) -> usize {
        size_of::<GuestMachineReport>()
    }

    pub fn trace_report_instruction_size_bytes(&self) -> usize {
        size_of::<RiscvInstruction>()
    }

    pub fn trace_report_register_write_list_size_bytes(&self) -> usize {
        size_of::<GuestRegisterWriteList>()
    }

    pub fn trace_report_memory_access_list_size_bytes(&self) -> usize {
        size_of::<GuestMemoryAccessList>()
    }

    pub fn trace_report_precompile_access_list_size_bytes(&self) -> usize {
        size_of::<GuestPrecompileMemoryAccessList>()
    }

    pub fn trace_report_storage_bytes(&self) -> usize {
        self.trace_report_count
            .saturating_mul(self.trace_report_record_size_bytes())
    }

    pub fn trace_report_buffer_capacity_bytes(&self) -> usize {
        self.trace_report_buffer_capacity
            .saturating_mul(self.trace_report_record_size_bytes())
    }

    pub fn trace_runner_report_buffer_capacity_bytes(&self) -> usize {
        self.trace_runner_report_buffer_capacity
            .saturating_mul(self.trace_report_record_size_bytes())
    }

    pub fn trace_report_buffer_excess_bytes(&self) -> usize {
        self.trace_report_buffer_excess_capacity
            .saturating_mul(self.trace_report_record_size_bytes())
    }

    pub fn trace_runner_report_buffer_excess_bytes(&self) -> usize {
        self.trace_runner_report_buffer_excess_capacity
            .saturating_mul(self.trace_report_record_size_bytes())
    }

    pub fn trace_descriptor_row_count(&self) -> usize {
        self.trace_descriptor_row_count
    }

    pub fn trace_descriptor_compact_row_count(&self) -> usize {
        self.trace_descriptor_compact_row_count
    }

    pub fn trace_descriptor_wide_row_count(&self) -> usize {
        self.trace_descriptor_wide_row_count
    }

    pub fn trace_descriptor_unpaired_value_count(&self) -> usize {
        self.trace_descriptor_unpaired_value_count
    }

    pub fn trace_descriptor_unpaired_high32_nonzero_count(&self) -> usize {
        self.trace_descriptor_unpaired_high32_nonzero_count
    }

    pub fn trace_descriptor_unpaired_high32_nonzero_row_count(&self) -> usize {
        self.trace_descriptor_unpaired_high32_nonzero_row_count
    }

    pub fn trace_descriptor_high32_field_counts(
        &self,
    ) -> [usize; ZISK_MAIN_UNPAIRED_DESCRIPTOR_FIELD_COUNT] {
        self.trace_descriptor_high32_field_counts
            .map(|count| count as usize)
    }

    pub fn trace_descriptor_high32_row_field_histogram(
        &self,
    ) -> [usize; ZISK_MAIN_UNPAIRED_DESCRIPTOR_HIGH32_HISTOGRAM_BUCKETS] {
        self.trace_descriptor_high32_row_field_histogram
            .map(|count| count as usize)
    }

    pub fn trace_single_row_report_count(&self) -> usize {
        self.trace_single_row_report_count
    }

    pub fn trace_multi_row_report_count(&self) -> usize {
        self.trace_multi_row_report_count
    }

    pub fn trace_pending_dma_report_count(&self) -> usize {
        self.trace_pending_dma_report_count
    }

    pub fn trace_amo_report_count(&self) -> usize {
        self.trace_amo_report_count
    }

    pub fn trace_store_conditional_report_count(&self) -> usize {
        self.trace_store_conditional_report_count
    }

    pub fn trace_external_op_row_count(&self) -> usize {
        self.trace_external_op_row_count
    }

    pub fn trace_external_op_run_count(&self) -> usize {
        self.trace_external_op_run_count
    }

    pub fn trace_external_op_max_run_count(&self) -> usize {
        self.trace_external_op_max_run_count
    }

    pub fn trace_copy_row_count(&self) -> usize {
        self.trace_copy_row_count
    }

    pub fn trace_copy_run_count(&self) -> usize {
        self.trace_copy_run_count
    }

    pub fn trace_copy_max_run_count(&self) -> usize {
        self.trace_copy_max_run_count
    }

    pub fn trace_copy_memory_source_row_count(&self) -> usize {
        self.trace_copy_memory_source_row_count
    }

    pub fn trace_copy_indirect_memory_row_count(&self) -> usize {
        self.trace_copy_indirect_memory_row_count
    }

    pub fn trace_copy_register_store_row_count(&self) -> usize {
        self.trace_copy_register_store_row_count
    }

    pub fn trace_copy_memory_store_row_count(&self) -> usize {
        self.trace_copy_memory_store_row_count
    }

    pub fn trace_copy_no_store_row_count(&self) -> usize {
        self.trace_copy_no_store_row_count
    }

    pub fn trace_copy_no_memory_row_count(&self) -> usize {
        self.trace_copy_no_memory_row_count
    }

    pub fn trace_flag_row_count(&self) -> usize {
        self.trace_flag_row_count
    }

    pub fn trace_precompile_row_count(&self) -> usize {
        self.trace_precompile_row_count
    }

    pub fn trace_indirect_memory_row_count(&self) -> usize {
        self.trace_indirect_memory_row_count
    }

    pub fn trace_register_source_read_count(&self) -> usize {
        self.trace_register_source_read_count
    }

    pub fn trace_memory_source_read_count(&self) -> usize {
        self.trace_memory_source_read_count
    }

    pub fn trace_register_store_row_count(&self) -> usize {
        self.trace_register_store_row_count
    }

    pub fn trace_memory_store_row_count(&self) -> usize {
        self.trace_memory_store_row_count
    }

    pub fn trace_no_store_row_count(&self) -> usize {
        self.trace_no_store_row_count
    }

    fn record_trace_row_shape_pattern(&mut self, id: u64) {
        self.record_trace_row_shape_pattern_count(id, 1);
    }

    fn record_trace_row_shape_pattern_count(&mut self, id: u64, count: usize) {
        if id == 0 || count == 0 {
            return;
        }
        let entry = self.trace_row_shape_pattern_counts.entry(id).or_default();
        *entry = entry.saturating_add(count);
    }

    pub fn trace_row_shape_top_patterns(
        &self,
    ) -> [(u64, usize); ZISK_MAIN_ROW_SHAPE_TOP_PATTERN_COUNT] {
        let mut pairs = self
            .trace_row_shape_pattern_counts
            .iter()
            .map(|(&id, &count)| (id, count))
            .collect::<Vec<_>>();
        pairs.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

        let mut out = [(0_u64, 0_usize); ZISK_MAIN_ROW_SHAPE_TOP_PATTERN_COUNT];
        for (slot, pair) in out.iter_mut().zip(pairs.into_iter()) {
            *slot = pair;
        }
        out
    }
}

struct DurationTimer<'a> {
    target: Option<&'a mut Duration>,
    started: Option<Instant>,
}

impl<'a> DurationTimer<'a> {
    fn new(target: Option<&'a mut Duration>) -> Self {
        let started = target.as_ref().map(|_| Instant::now());
        Self { target, started }
    }
}

impl Drop for DurationTimer<'_> {
    fn drop(&mut self) {
        if let (Some(target), Some(started)) = (self.target.as_deref_mut(), self.started) {
            *target += started.elapsed();
        }
    }
}

fn reborrow_trace_timing<'a>(
    timing: &'a mut Option<&mut GuestPcTraceStreamTiming>,
) -> Option<&'a mut GuestPcTraceStreamTiming> {
    match timing {
        Some(timing) => Some(&mut **timing),
        None => None,
    }
}

fn detail_duration_started(
    timing: &Option<&mut GuestPcTraceStreamTiming>,
    detail_timing: bool,
) -> Option<Instant> {
    timing
        .as_ref()
        .filter(|_| detail_timing)
        .map(|_| Instant::now())
}

fn record_detail_duration(
    started: Option<Instant>,
    timing: &mut Option<&mut GuestPcTraceStreamTiming>,
    target: fn(&mut GuestPcTraceStreamTiming) -> &mut Duration,
) {
    if let (Some(started), Some(timing)) = (started, timing.as_mut()) {
        *target(timing) += started.elapsed();
    }
}

fn env_flag_enabled(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| match value.as_str() {
            "0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF" => false,
            "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON" => true,
            _ => default,
        })
        .unwrap_or(default)
}

pub(crate) struct GuestPcTraceStreamResult {
    pub(crate) proof_values: Vec<WitnessTraceProofValue>,
    pub(crate) timing: GuestPcTraceStreamTiming,
}

pub(crate) fn run_guest_pc_trace_runtime_proof_values_with_context(
    backend: &GuestPcTraceBackend,
    context: WitnessComputeContext<'_>,
    input: &[u8],
) -> Result<Vec<WitnessTraceProofValue>, WitnessTraceRunError> {
    let (mut memory, mut state, mut fcall_handler) = load_guest_pc_trace_machine(context, input)
        .map_err(WitnessCallError::from)
        .map_err(WitnessTraceRunError::from)?;
    let run = run_guest_machine_with_fcalls(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        backend.instruction_limit,
    )
    .map_err(GuestPcTraceBackendError::GuestRun)
    .map_err(WitnessCallError::from)
    .map_err(WitnessTraceRunError::from)?;
    Ok(zisk_runtime_proof_values(
        run.executed_instructions != 0,
        fcall_handler.input_data_was_mapped(),
        state.dma_proof_value_flags(),
    ))
}

pub(crate) fn for_each_guest_pc_trace_segment_with_context<E>(
    backend: &GuestPcTraceBackend,
    context: WitnessComputeContext<'_>,
    request: WitnessTraceRequest<'_>,
    proof_values: &[WitnessTraceProofValue],
    mut emit: impl FnMut(GuestPcTraceSegmentRunOutput) -> Result<(), E>,
) -> Result<GuestPcTraceStreamTiming, GuestPcTraceSegmentStreamError<E>> {
    let layout = context
        .trace_layout
        .ok_or_else(|| WitnessCallError::Backend {
            message: "guest PC trace segmented backend requires trace layout".to_owned(),
        })
        .map_err(WitnessTraceRunError::from)
        .map_err(GuestPcTraceSegmentStreamError::Trace)?;
    if request.rows != layout.row_count() || request.columns != layout.column_count() {
        return Err(GuestPcTraceSegmentStreamError::Trace(
            WitnessCallError::Backend {
                message: format!(
                    "guest PC trace segmented request shape mismatch: layout {}x{}, request {}x{}",
                    layout.row_count(),
                    layout.column_count(),
                    request.rows,
                    request.columns
                ),
            }
            .into(),
        ));
    }

    for_each_guest_pc_trace_segment(
        backend.instruction_limit,
        context,
        request.input.as_ref(),
        Some(proof_values),
        |segment| {
            emit(GuestPcTraceSegmentRunOutput::from_segment_trace(segment))
                .map_err(GuestPcTraceSegmentStreamError::Emit)
        },
    )
    .map(|stream| stream.timing)
}

pub(crate) fn for_each_guest_pc_trace_segment_collecting_proof_values_with_context<E>(
    backend: &GuestPcTraceBackend,
    context: WitnessComputeContext<'_>,
    request: WitnessTraceRequest<'_>,
    mut emit: impl FnMut(GuestPcTraceSegmentRunOutput) -> Result<(), E>,
) -> Result<GuestPcTraceStreamResult, GuestPcTraceSegmentStreamError<E>> {
    let layout = context
        .trace_layout
        .ok_or_else(|| WitnessCallError::Backend {
            message: "guest PC trace segmented backend requires trace layout".to_owned(),
        })
        .map_err(WitnessTraceRunError::from)
        .map_err(GuestPcTraceSegmentStreamError::Trace)?;
    if request.rows != layout.row_count() || request.columns != layout.column_count() {
        return Err(GuestPcTraceSegmentStreamError::Trace(
            WitnessCallError::Backend {
                message: format!(
                    "guest PC trace segmented request shape mismatch: layout {}x{}, request {}x{}",
                    layout.row_count(),
                    layout.column_count(),
                    request.rows,
                    request.columns
                ),
            }
            .into(),
        ));
    }

    for_each_guest_pc_trace_segment(
        backend.instruction_limit,
        context,
        request.input.as_ref(),
        None,
        |segment| {
            emit(GuestPcTraceSegmentRunOutput::from_segment_trace(segment))
                .map_err(GuestPcTraceSegmentStreamError::Emit)
        },
    )
}

struct GuestPcTraceSegmentTrace {
    trace_instance_index: u32,
    trace_source_prefix_rows: usize,
    #[cfg(feature = "cuda")]
    device_segment_material: Option<GuestPcTraceDeviceSegmentMaterial>,
    trace: Option<WitnessTraceBuffer>,
    unit_values: Vec<WitnessTraceUnitValue>,
    proof_values: Vec<WitnessTraceProofValue>,
}

struct GuestPcTraceSegmentSlice {
    executed_instructions: u64,
    trace_rows: usize,
    status: GuestMachineTraceSliceStatus,
    report_count: usize,
    report_capacity: usize,
    last_report: Option<GuestMachineReport>,
    reports: Vec<GuestMachineReport>,
}

trait GuestPcTraceReplayHandler: GuestFcallHandler + Any + Send + Sync {
    fn clone_box(&self) -> Box<dyn GuestPcTraceReplayHandler>;
    fn equals_any(&self, other: &dyn Any) -> bool;
}

impl<H> GuestPcTraceReplayHandler for H
where
    H: GuestFcallHandler + Clone + PartialEq + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn GuestPcTraceReplayHandler> {
        Box::new(self.clone())
    }

    fn equals_any(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<H>() == Some(self)
    }
}

impl Clone for Box<dyn GuestPcTraceReplayHandler> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Clone)]
struct GuestPcTraceSegmentReplaySnapshot {
    memory: GuestMachineMemory,
    state: GuestMachineState,
    fcall_handler: Box<dyn GuestPcTraceReplayHandler>,
}

impl GuestPcTraceSegmentReplaySnapshot {
    fn capture<H>(memory: &GuestMachineMemory, state: &GuestMachineState, fcall_handler: &H) -> Self
    where
        H: GuestPcTraceReplayHandler,
    {
        Self {
            memory: memory.clone(),
            state: state.clone(),
            fcall_handler: fcall_handler.clone_box(),
        }
    }
}

struct GuestPcTraceSegmentReplay {
    slice: GuestPcTraceSegmentSlice,
    memory: GuestMachineMemory,
    state: GuestMachineState,
    fcall_handler: Box<dyn GuestPcTraceReplayHandler>,
}

struct GuestPcTracePendingSegmentSlice {
    trace_instance_index: u32,
    executed_instruction_count: u64,
    trace_row_count: usize,
    runner_remaining_instruction_limit: u64,
    report_count: usize,
    report_capacity: usize,
    reports: Vec<GuestMachineReport>,
    reports_elided: bool,
    terminal_pc: u64,
    lookahead_instruction: Option<RiscvInstruction>,
    is_last_segment: bool,
    seed: Option<Box<ZiskMainSegmentSeed>>,
    #[cfg_attr(not(test), allow(dead_code))]
    replay_snapshot: Option<GuestPcTraceSegmentReplaySnapshot>,
}

struct GuestPcTracePendingReportChunk {
    trace_instance_index: u32,
    reports: Vec<GuestMachineReport>,
}

#[cfg_attr(not(test), allow(dead_code))]
struct GuestPcTracePendingSegmentStreamStart {
    trace_instance_index: u32,
    runner_remaining_instruction_limit: u64,
    seed: Option<Box<ZiskMainSegmentSeed>>,
    #[allow(dead_code)]
    replay_snapshot: Option<GuestPcTraceSegmentReplaySnapshot>,
}

struct GuestPcTracePendingSegmentFinish {
    trace_instance_index: u32,
}

#[derive(Default)]
struct GuestPcTracePendingReportChunkGroup {
    reports: Vec<GuestMachineReport>,
    chunk_count: usize,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
struct GuestPcTraceRunnerStreamingSegment {
    slice: GuestPcTraceSegmentSlice,
    terminal_pc: u64,
    lookahead_instruction: Option<RiscvInstruction>,
    device_build: GuestPcTraceDeviceSegmentBuild,
    next_seed: ZiskMainSegmentSeed,
}

enum GuestPcTraceSegmentStreamMessage {
    Segment(Box<GuestPcTraceSegmentTrace>),
    Complete(Box<GuestPcTraceStreamResult>),
    Error(GuestPcTraceBackendError),
}

enum GuestPcTracePendingSegmentMessage {
    Segment(Box<GuestPcTracePendingSegmentSlice>),
    #[cfg_attr(not(test), allow(dead_code))]
    SegmentStreamStarted(Box<GuestPcTracePendingSegmentStreamStart>),
    SegmentStarted(Box<GuestPcTracePendingSegmentSlice>),
    ReportChunk(Box<GuestPcTracePendingReportChunk>),
    SegmentFinished(Box<GuestPcTracePendingSegmentFinish>),
    Complete(Box<GuestPcTraceStreamResult>),
    Error(GuestPcTraceBackendError),
}

pub fn is_guest_pc_trace_layout_supported(layout: &WitnessTraceLayout) -> bool {
    layout_trace_capacity(Some(layout)).is_ok()
}

pub fn is_guest_pc_trace_segmented_layout_supported(layout: &WitnessTraceLayout) -> bool {
    matches!(zisk_main_trace_columns(layout), Ok(Some(_)))
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
pub(crate) struct GuestPcTraceDeviceTraceBuilder {
    trace: Arc<CudaDeviceBuffer>,
    device_trace_descriptor_buffer: Option<Arc<CudaDeviceBuffer>>,
    stages: Vec<GuestPcTraceDeviceTraceStage>,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GuestPcDeviceSourceBuildTiming {
    descriptor_upload_duration: Duration,
    descriptor_upload_byte_count: usize,
    descriptor_upload_word_count: usize,
    descriptor_upload_row_count: usize,
    descriptor_stream_ingress_count: usize,
    trace_expand_duration: Duration,
}

#[cfg(feature = "cuda")]
impl GuestPcDeviceSourceBuildTiming {
    pub(crate) fn descriptor_upload_duration(&self) -> Duration {
        self.descriptor_upload_duration
    }

    pub(crate) fn descriptor_upload_byte_count(&self) -> usize {
        self.descriptor_upload_byte_count
    }

    pub(crate) fn descriptor_upload_word_count(&self) -> usize {
        self.descriptor_upload_word_count
    }

    pub(crate) fn descriptor_upload_row_count(&self) -> usize {
        self.descriptor_upload_row_count
    }

    pub(crate) fn descriptor_stream_ingress_count(&self) -> usize {
        self.descriptor_stream_ingress_count
    }

    pub(crate) fn trace_expand_duration(&self) -> Duration {
        self.trace_expand_duration
    }
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZiskMainDeviceTraceDescriptors {
    descriptor_words: usize,
    descriptor_rows: usize,
    unpaired_value_count: usize,
    unpaired_high32_nonzero_count: usize,
    unpaired_high32_nonzero_row_count: usize,
    unpaired_high32_nonzero_field_counts: [u32; ZISK_MAIN_UNPAIRED_DESCRIPTOR_FIELD_COUNT],
    unpaired_high32_nonzero_row_field_histogram:
        [u32; ZISK_MAIN_UNPAIRED_DESCRIPTOR_HIGH32_HISTOGRAM_BUCKETS],
    record_unpaired_high32_stats_enabled: bool,
    row_count: usize,
    column_count: usize,
    terminal_pc: u64,
    words: Vec<u64>,
    sparse_high_words: Vec<u64>,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GuestPcTraceDeviceTraceStage {
    stage_index: usize,
    row_count: usize,
    column_count: usize,
    row_stride: usize,
    column_offset: usize,
    known_zero: bool,
}

#[cfg(feature = "cuda")]
impl ZiskMainDeviceTraceDescriptors {
    #[cfg(test)]
    fn new(row_count: usize, column_count: usize, terminal_pc: u64) -> Self {
        Self::new_with_descriptor_words_and_stats(
            row_count,
            column_count,
            terminal_pc,
            ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS,
            true,
        )
    }

    fn new_with_descriptor_words(
        row_count: usize,
        column_count: usize,
        terminal_pc: u64,
        descriptor_words: usize,
    ) -> Self {
        Self::new_with_descriptor_words_and_stats(
            row_count,
            column_count,
            terminal_pc,
            descriptor_words,
            guest_pc_trace_descriptor_high32_stats_enabled(),
        )
    }

    fn new_with_descriptor_words_and_stats(
        row_count: usize,
        column_count: usize,
        terminal_pc: u64,
        descriptor_words: usize,
        record_unpaired_high32_stats_enabled: bool,
    ) -> Self {
        debug_assert!(
            descriptor_words == ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS
                || descriptor_words == ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS
                || descriptor_words == ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS
        );
        let word_capacity = row_count.checked_mul(descriptor_words).unwrap_or(0);
        Self {
            descriptor_words,
            descriptor_rows: 0,
            unpaired_value_count: 0,
            unpaired_high32_nonzero_count: 0,
            unpaired_high32_nonzero_row_count: 0,
            unpaired_high32_nonzero_field_counts: [0; ZISK_MAIN_UNPAIRED_DESCRIPTOR_FIELD_COUNT],
            unpaired_high32_nonzero_row_field_histogram: [0;
                ZISK_MAIN_UNPAIRED_DESCRIPTOR_HIGH32_HISTOGRAM_BUCKETS],
            record_unpaired_high32_stats_enabled,
            row_count,
            column_count,
            terminal_pc,
            words: Vec::with_capacity(word_capacity),
            sparse_high_words: Vec::new(),
        }
    }

    pub(crate) fn descriptor_rows(&self) -> usize {
        self.descriptor_rows
    }

    pub(crate) fn descriptor_word_count(&self) -> usize {
        self.descriptor_words
    }

    pub(crate) fn unpaired_value_count(&self) -> usize {
        self.unpaired_value_count
    }

    pub(crate) fn unpaired_high32_nonzero_count(&self) -> usize {
        self.unpaired_high32_nonzero_count
    }

    pub(crate) fn unpaired_high32_nonzero_row_count(&self) -> usize {
        self.unpaired_high32_nonzero_row_count
    }

    pub(crate) fn unpaired_high32_nonzero_field_counts(
        &self,
    ) -> [u32; ZISK_MAIN_UNPAIRED_DESCRIPTOR_FIELD_COUNT] {
        self.unpaired_high32_nonzero_field_counts
    }

    pub(crate) fn unpaired_high32_nonzero_row_field_histogram(
        &self,
    ) -> [u32; ZISK_MAIN_UNPAIRED_DESCRIPTOR_HIGH32_HISTOGRAM_BUCKETS] {
        self.unpaired_high32_nonzero_row_field_histogram
    }

    pub(crate) fn row_count(&self) -> usize {
        self.row_count
    }

    pub(crate) fn column_count(&self) -> usize {
        self.column_count
    }

    pub(crate) fn terminal_pc(&self) -> u64 {
        self.terminal_pc
    }

    pub(crate) fn words(&self) -> &[u64] {
        &self.words
    }

    pub(crate) fn sparse_high_words(&self) -> &[u64] {
        &self.sparse_high_words
    }

    pub(crate) fn is_sparse(&self) -> bool {
        self.descriptor_words == ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS
    }

    pub(crate) fn upload_word_count(&self) -> usize {
        self.words
            .len()
            .saturating_add(self.sparse_high_words.len())
    }

    fn record_unpaired_high32_stats(&mut self, values: [u64; 7]) {
        let high32_nonzero_count = values
            .iter()
            .filter(|&&value| zisk_main_high32_nonzero(value))
            .count();
        self.unpaired_value_count += values.len();
        self.unpaired_high32_nonzero_count += high32_nonzero_count;
        if high32_nonzero_count != 0 {
            self.unpaired_high32_nonzero_row_count += 1;
        }
        if let Some(bucket_count) = self
            .unpaired_high32_nonzero_row_field_histogram
            .get_mut(high32_nonzero_count)
        {
            *bucket_count = bucket_count.saturating_add(1);
        }
        for (field_count, value) in self
            .unpaired_high32_nonzero_field_counts
            .iter_mut()
            .zip(values)
        {
            if zisk_main_high32_nonzero(value) {
                *field_count = field_count.saturating_add(1);
            }
        }
    }
}

#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_COLUMNS: usize = 39;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS: usize = 11;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS: usize = 9;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS: usize = 14;
#[cfg(feature = "cuda")]
const ZISK_MAIN_SPARSE_DESCRIPTOR_MAX_HIGH_WORDS: usize =
    ZISK_MAIN_UNPAIRED_DESCRIPTOR_FIELD_COUNT.div_ceil(2);
pub(crate) const ZISK_MAIN_UNPAIRED_DESCRIPTOR_FIELD_COUNT: usize = 7;
pub(crate) const ZISK_MAIN_UNPAIRED_DESCRIPTOR_HIGH32_HISTOGRAM_BUCKETS: usize =
    ZISK_MAIN_UNPAIRED_DESCRIPTOR_FIELD_COUNT + 1;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_SOURCE_MEMORY: u64 = 1;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_SOURCE_IMMEDIATE: u64 = 2;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_SOURCE_REGISTER: u64 = 3;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_SOURCE_INDIRECT: u64 = 4;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_STORE_MEMORY: u64 = 1;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_STORE_REGISTER: u64 = 2;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_STORE_INDIRECT: u64 = 3;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_A_KIND_SHIFT: u64 = 32;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_B_KIND_SHIFT: u64 = 35;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_STORE_KIND_SHIFT: u64 = 38;

#[cfg(feature = "cuda")]
fn main_device_trace_descriptors(
    layout: &WitnessTraceLayout,
    columns: &ZiskMainTraceColumns<'_>,
    terminal_pc: u64,
    segment: ZiskMainTraceSegmentInfo,
) -> Option<ZiskMainDeviceTraceDescriptors> {
    if !guest_pc_device_trace_source_enabled()
        || !main_device_trace_layout_supported(layout, columns)
    {
        return None;
    }
    Some(ZiskMainDeviceTraceDescriptors::new_with_descriptor_words(
        layout.row_count(),
        layout.column_count(),
        terminal_pc,
        main_segment_descriptor_words(layout.row_count(), segment.trace_instance_index),
    ))
}

#[cfg(feature = "cuda")]
fn main_segment_descriptor_words(row_count: usize, trace_instance_index: u32) -> usize {
    if main_segment_mem_steps_fit_compact(row_count, trace_instance_index) {
        if guest_pc_trace_sparse_high32_descriptors_enabled() {
            ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS
        } else {
            ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS
        }
    } else {
        ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS
    }
}

#[cfg(feature = "cuda")]
fn main_segment_mem_steps_fit_compact(row_count: usize, trace_instance_index: u32) -> bool {
    let Some(row_count) = u64::try_from(row_count).ok() else {
        return false;
    };
    if row_count == 0 {
        return true;
    }
    let Some(exclusive_end) = u64::from(trace_instance_index)
        .checked_add(1)
        .and_then(|next| next.checked_mul(row_count))
    else {
        return false;
    };
    let Some(main_step) = exclusive_end.checked_sub(1) else {
        return true;
    };
    let Some(max_step) = ZISK_MAIN_MEM_STEPS_PER_ROW
        .checked_mul(main_step)
        .and_then(|base| base.checked_add(ZISK_MAIN_RESERVED_MEM_STEPS))
        .and_then(|base| base.checked_add(ZISK_MAIN_SPECIAL_MEM_STEP_OFFSET))
    else {
        return false;
    };
    max_step <= u64::from(u32::MAX)
}

#[cfg(feature = "cuda")]
fn main_device_trace_layout_supported(
    layout: &WitnessTraceLayout,
    columns: &ZiskMainTraceColumns<'_>,
) -> bool {
    layout.column_count() == ZISK_MAIN_DEVICE_TRACE_COLUMNS
        && trace_target_at(&columns.a, 0)
        && trace_target_at(&columns.b, 2)
        && trace_target_at(&columns.c, 4)
        && trace_target_at(&columns.flag, 6)
        && trace_target_at(&columns.pc, 7)
        && optional_trace_target_at(&columns.a_src_imm, 8)
        && optional_trace_target_at(&columns.a_src_mem, 9)
        && optional_trace_target_at(&columns.a_offset_imm0, 10)
        && optional_trace_target_at(&columns.a_imm1, 11)
        && optional_trace_target_at(&columns.is_precompiled, 12)
        && optional_trace_target_at(&columns.b_src_imm, 13)
        && optional_trace_target_at(&columns.b_src_mem, 14)
        && optional_trace_target_at(&columns.b_offset_imm0, 15)
        && optional_trace_target_at(&columns.b_imm1, 16)
        && optional_trace_target_at(&columns.b_src_ind, 17)
        && optional_trace_target_at(&columns.ind_width, 18)
        && optional_trace_target_at(&columns.is_external_op, 19)
        && trace_target_at(&columns.op, 20)
        && optional_trace_target_at(&columns.store_pc, 21)
        && optional_trace_target_at(&columns.store_mem, 22)
        && optional_trace_target_at(&columns.store_ind, 23)
        && optional_trace_target_at(&columns.store_offset, 24)
        && optional_trace_target_at(&columns.set_pc, 25)
        && optional_trace_target_at(&columns.jmp_offset1, 26)
        && optional_trace_target_at(&columns.jmp_offset2, 27)
        && optional_trace_target_at(&columns.m32, 28)
        && optional_trace_target_at(&columns.addr1, 29)
        && optional_trace_target_at(&columns.a_reg_prev_mem_step, 30)
        && optional_trace_target_at(&columns.b_reg_prev_mem_step, 31)
        && optional_trace_target_at(&columns.store_reg_prev_mem_step, 32)
        && optional_trace_target_at(&columns.store_reg_prev_value, 33)
        && optional_trace_target_at(&columns.a_src_reg, 35)
        && optional_trace_target_at(&columns.b_src_reg, 36)
        && optional_trace_target_at(&columns.store_reg, 37)
        && columns.addr2.is_none()
}

#[cfg(feature = "cuda")]
fn trace_target_at(target: &TraceColumnTarget<'_>, trace_column: usize) -> bool {
    target.trace_column() == trace_column
}

#[cfg(feature = "cuda")]
fn optional_trace_target_at(target: &Option<TraceColumnTarget<'_>>, trace_column: usize) -> bool {
    target
        .as_ref()
        .is_some_and(|target| trace_target_at(target, trace_column))
}

#[cfg(feature = "cuda")]
fn append_main_device_trace_descriptor(
    descriptors: &mut ZiskMainDeviceTraceDescriptors,
    values: &ZiskMainReportTraceValues,
) -> Result<(), GuestPcTraceBackendError> {
    if descriptors.descriptor_rows >= descriptors.row_count {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main device trace descriptor rows exceed layout rows".to_owned(),
        });
    }
    let instruction = &values.instruction;
    let (a_kind, a_payload) = zisk_main_device_trace_source_descriptor(instruction.a);
    let (b_kind, b_payload) = zisk_main_device_trace_source_descriptor(instruction.b);
    let (store_kind, store_payload) = zisk_main_device_trace_store_descriptor(&instruction.store);
    let control = u64::from(instruction.op.code())
        | (u64::from(values.flag) << 8)
        | (u64::from(instruction.store_pc) << 9)
        | (u64::from(instruction.set_pc) << 10)
        | (u64::from(instruction.m32) << 11)
        | (u64::from(instruction.is_external_op) << 12)
        | (u64::from(instruction.is_precompiled) << 13)
        | (instruction.ind_width << 16)
        | (a_kind << ZISK_MAIN_DEVICE_TRACE_A_KIND_SHIFT)
        | (b_kind << ZISK_MAIN_DEVICE_TRACE_B_KIND_SHIFT)
        | (store_kind << ZISK_MAIN_DEVICE_TRACE_STORE_KIND_SHIFT);
    let a_prev_mem_step = values.register_accesses.a_prev_mem_step.unwrap_or(0);
    let b_prev_mem_step = values.register_accesses.b_prev_mem_step.unwrap_or(0);
    let store_prev_mem_step = values.register_accesses.store_prev_mem_step.unwrap_or(0);
    let store_prev_value = values.register_accesses.store_prev_value.unwrap_or(0);
    if descriptors.record_unpaired_high32_stats_enabled {
        descriptors.record_unpaired_high32_stats(zisk_main_unpaired_descriptor_values(
            values,
            a_payload,
            b_payload,
            store_payload,
            store_prev_value,
        ));
    }
    if descriptors.descriptor_words == ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS {
        if let Some(sparse) = zisk_main_sparse_device_trace_descriptor_words(
            values,
            a_payload,
            b_payload,
            store_payload,
            control,
            a_prev_mem_step,
            b_prev_mem_step,
            store_prev_mem_step,
            store_prev_value,
            descriptors.sparse_high_words.len(),
        ) {
            descriptors.words.extend_from_slice(&sparse.words);
            descriptors
                .sparse_high_words
                .extend_from_slice(&sparse.high_words[..sparse.high_word_count]);
            descriptors.descriptor_rows = descriptors.descriptor_rows.checked_add(1).ok_or(
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "Zisk Main device trace descriptor row count overflow".to_owned(),
                },
            )?;
            return Ok(());
        }
        convert_zisk_main_sparse_descriptors_to_wide(descriptors);
    }
    if descriptors.descriptor_words == ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS {
        if let Some(compact_words) = zisk_main_compact_device_trace_descriptor_words(
            values,
            a_payload,
            b_payload,
            store_payload,
            control,
            a_prev_mem_step,
            b_prev_mem_step,
            store_prev_mem_step,
            store_prev_value,
        ) {
            descriptors.words.extend_from_slice(&compact_words);
            descriptors.descriptor_rows = descriptors.descriptor_rows.checked_add(1).ok_or(
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "Zisk Main device trace descriptor row count overflow".to_owned(),
                },
            )?;
            return Ok(());
        }
        convert_zisk_main_compact_descriptors_to_wide(descriptors);
    }
    descriptors.words.extend_from_slice(&[
        values.a,
        values.b,
        values.c,
        instruction.pc,
        a_payload,
        b_payload,
        store_payload,
        control,
        instruction.jmp_offset1 as u64,
        instruction.jmp_offset2 as u64,
        a_prev_mem_step,
        b_prev_mem_step,
        store_prev_mem_step,
        store_prev_value,
    ]);
    descriptors.descriptor_rows = descriptors.descriptor_rows.checked_add(1).ok_or(
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main device trace descriptor row count overflow".to_owned(),
        },
    )?;
    Ok(())
}

#[cfg(feature = "cuda")]
fn zisk_main_unpaired_descriptor_values(
    values: &ZiskMainReportTraceValues,
    a_payload: u64,
    b_payload: u64,
    store_payload: u64,
    store_prev_value: u64,
) -> [u64; 7] {
    [
        values.a,
        values.b,
        values.c,
        a_payload,
        b_payload,
        store_payload,
        store_prev_value,
    ]
}

#[cfg(feature = "cuda")]
fn zisk_main_high32_nonzero(value: u64) -> bool {
    value >> 32 != 0
}

#[cfg(feature = "cuda")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ZiskMainSparseDeviceTraceDescriptorWords {
    words: [u64; ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS],
    high_words: [u64; ZISK_MAIN_SPARSE_DESCRIPTOR_MAX_HIGH_WORDS],
    high_word_count: usize,
}

#[cfg(feature = "cuda")]
#[allow(clippy::too_many_arguments)]
fn zisk_main_sparse_device_trace_descriptor_words(
    values: &ZiskMainReportTraceValues,
    a_payload: u64,
    b_payload: u64,
    store_payload: u64,
    control: u64,
    a_prev_mem_step: u64,
    b_prev_mem_step: u64,
    store_prev_mem_step: u64,
    store_prev_value: u64,
    sparse_high_word_offset: usize,
) -> Option<ZiskMainSparseDeviceTraceDescriptorWords> {
    let pc_and_store_step = zisk_main_pack_u32_pair(values.instruction.pc, store_prev_mem_step)?;
    let jump_offsets = zisk_main_pack_i32_pair(
        values.instruction.jmp_offset1,
        values.instruction.jmp_offset2,
    )?;
    let register_mem_steps = zisk_main_pack_u32_pair(a_prev_mem_step, b_prev_mem_step)?;
    let unpaired_values = zisk_main_unpaired_descriptor_values(
        values,
        a_payload,
        b_payload,
        store_payload,
        store_prev_value,
    );
    let mut high_mask = 0_u64;
    let mut high_words = [0_u64; ZISK_MAIN_SPARSE_DESCRIPTOR_MAX_HIGH_WORDS];
    let mut high_value_count = 0_usize;
    for (index, value) in unpaired_values.into_iter().enumerate() {
        let high = value >> 32;
        if high != 0 {
            high_mask |= 1_u64 << index;
            let high_word_index = high_value_count / 2;
            if high_value_count.is_multiple_of(2) {
                high_words[high_word_index] = high;
            } else {
                high_words[high_word_index] |= high << 32;
            }
            high_value_count += 1;
        }
    }
    Some(ZiskMainSparseDeviceTraceDescriptorWords {
        words: [
            zisk_main_low32_pair(values.a, values.b),
            zisk_main_low32_pair(values.c, a_payload),
            zisk_main_low32_pair(b_payload, store_payload),
            control,
            pc_and_store_step,
            jump_offsets,
            register_mem_steps,
            (store_prev_value & 0xffff_ffff) | (high_mask << 32),
            u64::try_from(sparse_high_word_offset).ok()?,
        ],
        high_words,
        high_word_count: high_value_count.div_ceil(2),
    })
}

#[cfg(feature = "cuda")]
fn zisk_main_low32_pair(lhs: u64, rhs: u64) -> u64 {
    (lhs & 0xffff_ffff) | ((rhs & 0xffff_ffff) << 32)
}

#[cfg(feature = "cuda")]
fn zisk_main_sparse_descriptor_high32(
    low32_high_mask: u64,
    high_words: &[u64],
    high_word_offset: usize,
    field_index: usize,
) -> u64 {
    let high_mask = low32_high_mask >> 32;
    if high_mask & (1_u64 << field_index) == 0 {
        return 0;
    }
    let preceding_mask = (1_u64 << field_index).saturating_sub(1);
    let high_position = (high_mask & preceding_mask).count_ones() as usize;
    let word_index = high_word_offset.saturating_add(high_position / 2);
    let packed = high_words.get(word_index).copied().unwrap_or(0);
    if high_position.is_multiple_of(2) {
        packed & 0xffff_ffff
    } else {
        packed >> 32
    }
}

#[cfg(feature = "cuda")]
fn zisk_main_sparse_descriptor_value(
    low32: u64,
    low32_high_mask: u64,
    high_words: &[u64],
    high_word_offset: usize,
    field_index: usize,
) -> u64 {
    (low32 & 0xffff_ffff)
        | (zisk_main_sparse_descriptor_high32(
            low32_high_mask,
            high_words,
            high_word_offset,
            field_index,
        ) << 32)
}

#[cfg(feature = "cuda")]
#[allow(clippy::too_many_arguments)]
fn zisk_main_compact_device_trace_descriptor_words(
    values: &ZiskMainReportTraceValues,
    a_payload: u64,
    b_payload: u64,
    store_payload: u64,
    control: u64,
    a_prev_mem_step: u64,
    b_prev_mem_step: u64,
    store_prev_mem_step: u64,
    store_prev_value: u64,
) -> Option<[u64; ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS]> {
    Some([
        values.a,
        values.b,
        values.c,
        a_payload,
        b_payload,
        store_payload,
        control,
        zisk_main_pack_u32_pair(values.instruction.pc, store_prev_mem_step)?,
        zisk_main_pack_i32_pair(
            values.instruction.jmp_offset1,
            values.instruction.jmp_offset2,
        )?,
        zisk_main_pack_u32_pair(a_prev_mem_step, b_prev_mem_step)?,
        store_prev_value,
    ])
}

#[cfg(feature = "cuda")]
fn zisk_main_pack_i32_pair(lhs: i64, rhs: i64) -> Option<u64> {
    let lhs = i32::try_from(lhs).ok()? as u32;
    let rhs = i32::try_from(rhs).ok()? as u32;
    Some(u64::from(lhs) | (u64::from(rhs) << 32))
}

#[cfg(feature = "cuda")]
fn zisk_main_pack_u32_pair(lhs: u64, rhs: u64) -> Option<u64> {
    Some(u64::from(zisk_main_pack_u32(lhs)?) | (u64::from(zisk_main_pack_u32(rhs)?) << 32))
}

#[cfg(feature = "cuda")]
fn zisk_main_pack_u32(value: u64) -> Option<u32> {
    u32::try_from(value).ok()
}

#[cfg(feature = "cuda")]
fn convert_zisk_main_sparse_descriptors_to_wide(descriptors: &mut ZiskMainDeviceTraceDescriptors) {
    debug_assert_eq!(
        descriptors.descriptor_words,
        ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS
    );
    let mut wide_words = Vec::with_capacity(
        descriptors
            .descriptor_rows
            .saturating_mul(ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS),
    );
    for sparse in descriptors
        .words
        .chunks_exact(ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS)
    {
        let ab = sparse[0];
        let c_and_a_payload = sparse[1];
        let b_and_store_payload = sparse[2];
        let pc_and_store_step = sparse[4];
        let jump_offsets = sparse[5];
        let register_mem_steps = sparse[6];
        let store_prev_and_mask = sparse[7];
        let high_word_offset = usize::try_from(sparse[8]).unwrap_or(usize::MAX);
        let a = zisk_main_sparse_descriptor_value(
            ab,
            store_prev_and_mask,
            &descriptors.sparse_high_words,
            high_word_offset,
            0,
        );
        let b = zisk_main_sparse_descriptor_value(
            ab >> 32,
            store_prev_and_mask,
            &descriptors.sparse_high_words,
            high_word_offset,
            1,
        );
        let c = zisk_main_sparse_descriptor_value(
            c_and_a_payload,
            store_prev_and_mask,
            &descriptors.sparse_high_words,
            high_word_offset,
            2,
        );
        let a_payload = zisk_main_sparse_descriptor_value(
            c_and_a_payload >> 32,
            store_prev_and_mask,
            &descriptors.sparse_high_words,
            high_word_offset,
            3,
        );
        let b_payload = zisk_main_sparse_descriptor_value(
            b_and_store_payload,
            store_prev_and_mask,
            &descriptors.sparse_high_words,
            high_word_offset,
            4,
        );
        let store_payload = zisk_main_sparse_descriptor_value(
            b_and_store_payload >> 32,
            store_prev_and_mask,
            &descriptors.sparse_high_words,
            high_word_offset,
            5,
        );
        let store_prev_value = zisk_main_sparse_descriptor_value(
            store_prev_and_mask,
            store_prev_and_mask,
            &descriptors.sparse_high_words,
            high_word_offset,
            6,
        );
        wide_words.extend_from_slice(&[
            a,
            b,
            c,
            pc_and_store_step & 0xffff_ffff,
            a_payload,
            b_payload,
            store_payload,
            sparse[3],
            zisk_main_unpack_i32_low(jump_offsets),
            zisk_main_unpack_i32_high(jump_offsets),
            register_mem_steps & 0xffff_ffff,
            register_mem_steps >> 32,
            pc_and_store_step >> 32,
            store_prev_value,
        ]);
    }
    descriptors.words = wide_words;
    descriptors.sparse_high_words.clear();
    descriptors.descriptor_words = ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS;
}

#[cfg(feature = "cuda")]
fn convert_zisk_main_compact_descriptors_to_wide(descriptors: &mut ZiskMainDeviceTraceDescriptors) {
    debug_assert_eq!(
        descriptors.descriptor_words,
        ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS
    );
    let mut wide_words = Vec::with_capacity(
        descriptors
            .descriptor_rows
            .saturating_mul(ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS),
    );
    for compact in descriptors
        .words
        .chunks_exact(ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS)
    {
        let pc_and_store_step = compact[7];
        let jump_offsets = compact[8];
        let register_mem_steps = compact[9];
        wide_words.extend_from_slice(&[
            compact[0],
            compact[1],
            compact[2],
            pc_and_store_step & 0xffff_ffff,
            compact[3],
            compact[4],
            compact[5],
            compact[6],
            zisk_main_unpack_i32_low(jump_offsets),
            zisk_main_unpack_i32_high(jump_offsets),
            register_mem_steps & 0xffff_ffff,
            register_mem_steps >> 32,
            pc_and_store_step >> 32,
            compact[10],
        ]);
    }
    descriptors.words = wide_words;
    descriptors.descriptor_words = ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS;
}

#[cfg(feature = "cuda")]
fn zisk_main_unpack_i32_low(value: u64) -> u64 {
    i64::from(value as u32 as i32) as u64
}

#[cfg(feature = "cuda")]
fn zisk_main_unpack_i32_high(value: u64) -> u64 {
    i64::from((value >> 32) as u32 as i32) as u64
}

#[cfg(feature = "cuda")]
fn zisk_main_device_trace_source_descriptor(source: ZiskMainSource) -> (u64, u64) {
    match source {
        ZiskMainSource::LastC => (0, 0),
        ZiskMainSource::Memory(value) => (ZISK_MAIN_DEVICE_TRACE_SOURCE_MEMORY, value),
        ZiskMainSource::Immediate(value) => (ZISK_MAIN_DEVICE_TRACE_SOURCE_IMMEDIATE, value),
        ZiskMainSource::Register(index) => {
            (ZISK_MAIN_DEVICE_TRACE_SOURCE_REGISTER, u64::from(index))
        }
        ZiskMainSource::Indirect(offset) => (ZISK_MAIN_DEVICE_TRACE_SOURCE_INDIRECT, offset as u64),
    }
}

#[cfg(feature = "cuda")]
fn zisk_main_device_trace_store_descriptor(store: &ZiskMainStore) -> (u64, u64) {
    match store {
        ZiskMainStore::None => (0, 0),
        ZiskMainStore::Memory(address) => (ZISK_MAIN_DEVICE_TRACE_STORE_MEMORY, *address as u64),
        ZiskMainStore::Register(index) => {
            (ZISK_MAIN_DEVICE_TRACE_STORE_REGISTER, u64::from(*index))
        }
        ZiskMainStore::Indirect(offset) => (ZISK_MAIN_DEVICE_TRACE_STORE_INDIRECT, *offset as u64),
    }
}

#[cfg(feature = "cuda")]
impl GuestPcTraceDeviceTraceBuilder {
    pub(crate) fn trace(&self) -> &Arc<CudaDeviceBuffer> {
        &self.trace
    }

    pub(crate) fn device_trace_descriptor_buffer(&self) -> Option<&Arc<CudaDeviceBuffer>> {
        self.device_trace_descriptor_buffer.as_ref()
    }

    pub(crate) fn stages(&self) -> &[GuestPcTraceDeviceTraceStage] {
        &self.stages
    }
}

#[cfg(feature = "cuda")]
impl GuestPcTraceDeviceTraceStage {
    pub(crate) fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub(crate) fn row_count(&self) -> usize {
        self.row_count
    }

    pub(crate) fn column_count(&self) -> usize {
        self.column_count
    }

    pub(crate) fn row_stride(&self) -> usize {
        self.row_stride
    }

    pub(crate) fn column_offset(&self) -> usize {
        self.column_offset
    }

    pub(crate) fn is_known_zero(&self) -> bool {
        self.known_zero
    }
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) fn build_guest_pc_trace_stage_source_devices_from_device_material(
    layout: &WitnessTraceLayout,
    material: &GuestPcTraceDeviceSegmentMaterial,
) -> Result<GuestPcTraceDeviceTraceBuilder, WitnessTraceRunError> {
    build_guest_pc_trace_stage_source_devices_from_device_material_timing(layout, material, None)
}

#[cfg(feature = "cuda")]
pub(crate) fn build_guest_pc_trace_stage_source_devices_from_device_material_timing(
    layout: &WitnessTraceLayout,
    material: &GuestPcTraceDeviceSegmentMaterial,
    mut timing: Option<&mut GuestPcDeviceSourceBuildTiming>,
) -> Result<GuestPcTraceDeviceTraceBuilder, WitnessTraceRunError> {
    if !is_guest_pc_trace_segmented_layout_supported(layout) {
        return Err(guest_pc_device_trace_source_error(
            "guest PC device material requires a supported segmented layout",
        ));
    }
    let descriptors = &material.device_trace_descriptors;
    if descriptors.row_count() != layout.row_count()
        || descriptors.column_count() != layout.column_count()
        || descriptors.descriptor_rows() != material.trace_source_prefix_rows
    {
        return Err(guest_pc_device_trace_source_error(
            "device material descriptor shape does not match guest PC layout",
        ));
    }

    let descriptor_upload_word_count = descriptors.upload_word_count();
    let descriptor_upload_byte_count =
        descriptor_upload_word_count.saturating_mul(std::mem::size_of::<u64>());
    let descriptor_upload_row_count = descriptors.descriptor_rows();

    if descriptors.is_sparse() {
        let trace_device = record_device_source_build_duration(
            timing
                .as_mut()
                .map(|timing| &mut timing.descriptor_upload_duration),
            || {
                CudaDeviceBuffer::from_sparse_zisk_main_trace_descriptors(
                    descriptors.words(),
                    descriptors.sparse_high_words(),
                    descriptors.descriptor_rows(),
                    descriptors.row_count(),
                    descriptors.column_count(),
                    descriptors.terminal_pc(),
                )
                .map_err(|error| {
                    guest_pc_device_trace_source_error(format!(
                        "CUDA sparse trace descriptor expansion failed: {error}"
                    ))
                })
            },
        )?;
        if let Some(timing) = timing.as_mut() {
            timing.descriptor_upload_byte_count += descriptor_upload_byte_count;
            timing.descriptor_upload_word_count += descriptor_upload_word_count;
            timing.descriptor_upload_row_count += descriptor_upload_row_count;
        }
        let builder = guest_pc_device_trace_builder_from_layout_with_descriptor_source(
            layout,
            trace_device,
            None,
            true,
        );
        validate_guest_pc_trace_device_source_matches_layout(layout, &builder)?;
        return Ok(builder);
    }

    if guest_pc_descriptor_stream_ingress_enabled() {
        let stream = CudaStream::new().map_err(|error| {
            guest_pc_device_trace_source_error(format!(
                "CUDA trace descriptor stream creation failed: {error}"
            ))
        })?;
        let pending = record_device_source_build_duration(
            timing
                .as_mut()
                .map(|timing| &mut timing.descriptor_upload_duration),
            || unsafe {
                CudaDeviceBuffer::begin_trace_descriptor_expansion_on_stream(
                    descriptors.words(),
                    descriptors.descriptor_word_count(),
                    descriptors.descriptor_rows(),
                    descriptors.row_count(),
                    descriptors.column_count(),
                    descriptors.terminal_pc(),
                    &stream,
                )
                .map_err(|error| {
                    guest_pc_device_trace_source_error(format!(
                        "CUDA trace descriptor stream ingress failed: {error}"
                    ))
                })
            },
        )?;
        record_device_source_build_duration(
            timing
                .as_mut()
                .map(|timing| &mut timing.trace_expand_duration),
            || {
                stream.synchronize().map_err(|error| {
                    guest_pc_device_trace_source_error(format!(
                        "CUDA trace descriptor stream synchronization failed: {error}"
                    ))
                })
            },
        )?;
        let (descriptor_buffer, trace_device) = pending.into_parts();
        let descriptor_buffer = Arc::new(descriptor_buffer);
        if let Some(timing) = timing.as_mut() {
            timing.descriptor_upload_byte_count += descriptor_upload_byte_count;
            timing.descriptor_upload_word_count += descriptor_upload_word_count;
            timing.descriptor_upload_row_count += descriptor_upload_row_count;
            timing.descriptor_stream_ingress_count += 1;
        }
        let builder = guest_pc_device_trace_builder_from_layout_with_descriptor_source(
            layout,
            trace_device,
            Some(descriptor_buffer),
            true,
        );
        validate_guest_pc_trace_device_source_matches_layout(layout, &builder)?;
        return Ok(builder);
    }

    let descriptor_buffer = Arc::new(record_device_source_build_duration(
        timing
            .as_mut()
            .map(|timing| &mut timing.descriptor_upload_duration),
        || {
            CudaDeviceBuffer::from_u64_words(descriptors.words()).map_err(|error| {
                guest_pc_device_trace_source_error(format!(
                    "CUDA trace descriptor upload failed: {error}"
                ))
            })
        },
    )?);
    if let Some(timing) = timing.as_mut() {
        timing.descriptor_upload_byte_count += descriptor_upload_byte_count;
        timing.descriptor_upload_word_count += descriptor_upload_word_count;
        timing.descriptor_upload_row_count += descriptor_upload_row_count;
    }
    let mut builder = build_guest_pc_trace_stage_source_devices_from_device_descriptors_timing(
        layout,
        material,
        descriptor_buffer.as_ref(),
        timing,
    )?;
    builder.device_trace_descriptor_buffer = Some(descriptor_buffer);
    Ok(builder)
}

#[cfg(feature = "cuda")]
fn guest_pc_descriptor_stream_ingress_enabled() -> bool {
    env_flag_enabled("LZVM_CUDA_GUEST_PC_DESCRIPTOR_STREAM_INGRESS", false)
}

#[cfg(feature = "cuda")]
pub(crate) fn guest_pc_trace_descriptor_high32_stats_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_DESCRIPTOR_HIGH32_STATS", false)
}

#[cfg(feature = "cuda")]
fn guest_pc_trace_sparse_high32_descriptors_enabled() -> bool {
    env_flag_enabled("LZVM_CUDA_GUEST_PC_SPARSE_HIGH32_DESCRIPTORS", false)
}

#[cfg(feature = "cuda")]
fn record_device_source_build_duration<T>(
    duration: Option<&mut Duration>,
    f: impl FnOnce() -> Result<T, WitnessTraceRunError>,
) -> Result<T, WitnessTraceRunError> {
    if let Some(duration) = duration {
        let started = Instant::now();
        let result = f();
        *duration += started.elapsed();
        return result;
    }
    f()
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) fn build_guest_pc_trace_stage_source_devices_from_device_descriptors(
    layout: &WitnessTraceLayout,
    material: &GuestPcTraceDeviceSegmentMaterial,
    device_trace_descriptor_buffer: &CudaDeviceBuffer,
) -> Result<GuestPcTraceDeviceTraceBuilder, WitnessTraceRunError> {
    build_guest_pc_trace_stage_source_devices_from_device_descriptors_timing(
        layout,
        material,
        device_trace_descriptor_buffer,
        None,
    )
}

#[cfg(feature = "cuda")]
pub(crate) fn build_guest_pc_trace_stage_source_devices_from_device_descriptors_timing(
    layout: &WitnessTraceLayout,
    material: &GuestPcTraceDeviceSegmentMaterial,
    device_trace_descriptor_buffer: &CudaDeviceBuffer,
    timing: Option<&mut GuestPcDeviceSourceBuildTiming>,
) -> Result<GuestPcTraceDeviceTraceBuilder, WitnessTraceRunError> {
    if !is_guest_pc_trace_segmented_layout_supported(layout) {
        return Err(guest_pc_device_trace_source_error(
            "guest PC device descriptors require a supported segmented layout",
        ));
    }
    let descriptors = &material.device_trace_descriptors;
    if descriptors.row_count() != layout.row_count()
        || descriptors.column_count() != layout.column_count()
        || descriptors.descriptor_rows() != material.trace_source_prefix_rows
    {
        return Err(guest_pc_device_trace_source_error(
            "device descriptor shape does not match guest PC layout",
        ));
    }

    let trace_device = record_device_source_build_duration(
        timing.map(|timing| &mut timing.trace_expand_duration),
        || {
            CudaDeviceBuffer::from_zisk_main_trace_descriptors_device(
                device_trace_descriptor_buffer,
                descriptors.descriptor_word_count(),
                descriptors.descriptor_rows(),
                descriptors.row_count(),
                descriptors.column_count(),
                descriptors.terminal_pc(),
            )
            .map_err(|error| {
                guest_pc_device_trace_source_error(format!(
                    "CUDA trace descriptor expansion failed: {error}"
                ))
            })
        },
    )?;
    let builder = guest_pc_device_trace_builder_from_layout_with_descriptor_source(
        layout,
        trace_device,
        None,
        true,
    );
    validate_guest_pc_trace_device_source_matches_layout(layout, &builder)?;
    Ok(builder)
}

#[cfg(feature = "cuda")]
pub(crate) fn build_guest_pc_trace_stage_source_devices(
    layout: &WitnessTraceLayout,
    trace: &WitnessTraceBuffer,
    trace_source_prefix_rows: usize,
    device_trace_descriptors: Option<&ZiskMainDeviceTraceDescriptors>,
) -> Result<Option<GuestPcTraceDeviceTraceBuilder>, WitnessTraceRunError> {
    if !guest_pc_device_trace_source_enabled() {
        return Ok(None);
    }
    if !is_guest_pc_trace_segmented_layout_supported(layout) {
        return Ok(None);
    }
    if trace.row_count() != layout.row_count() || trace.column_count() != layout.column_count() {
        return Err(guest_pc_device_trace_source_error(
            "trace shape does not match guest PC layout",
        ));
    }

    let row_width = trace.column_count();
    if let Some(descriptors) = device_trace_descriptors {
        if descriptors.row_count() != trace.row_count()
            || descriptors.column_count() != row_width
            || descriptors.descriptor_rows() != trace_source_prefix_rows
        {
            return Err(guest_pc_device_trace_source_error(
                "device trace descriptor shape does not match guest PC trace",
            ));
        }
        let trace_device = if descriptors.is_sparse() {
            CudaDeviceBuffer::from_sparse_zisk_main_trace_descriptors(
                descriptors.words(),
                descriptors.sparse_high_words(),
                descriptors.descriptor_rows(),
                descriptors.row_count(),
                descriptors.column_count(),
                descriptors.terminal_pc(),
            )
            .map_err(|error| {
                guest_pc_device_trace_source_error(format!(
                    "CUDA sparse trace descriptor expansion failed: {error}"
                ))
            })?
        } else {
            CudaDeviceBuffer::from_zisk_main_trace_descriptors(
                descriptors.words(),
                descriptors.descriptor_word_count(),
                descriptors.descriptor_rows(),
                descriptors.row_count(),
                descriptors.column_count(),
                descriptors.terminal_pc(),
            )
            .map_err(|error| {
                guest_pc_device_trace_source_error(format!(
                    "CUDA trace descriptor expansion failed: {error}"
                ))
            })?
        };
        let builder = guest_pc_device_trace_builder_from_layout_with_descriptor_source(
            layout,
            trace_device,
            None,
            true,
        );
        validate_guest_pc_trace_device_source_matches_trace(layout, trace, &builder)?;
        return Ok(Some(builder));
    }

    if trace_source_prefix_rows >= trace.row_count() {
        return Ok(None);
    }

    let trace_words = Felt::as_u64_slice(trace.values());
    let prefix_words = trace_source_prefix_rows
        .checked_mul(row_width)
        .ok_or_else(|| guest_pc_device_trace_source_error("trace prefix word count overflow"))?;
    if prefix_words
        .checked_add(row_width)
        .is_none_or(|end| end > trace_words.len())
    {
        return Err(guest_pc_device_trace_source_error(
            "terminal row exceeds trace words",
        ));
    }
    let terminal_row = &trace_words[prefix_words..prefix_words + row_width];
    for row in trace_words[prefix_words..].chunks_exact(row_width) {
        if row != terminal_row {
            return Ok(None);
        }
    }

    let trace_device = CudaDeviceBuffer::from_row_major_u64_prefix_and_suffix_row(
        &trace_words[..prefix_words],
        terminal_row,
        trace.row_count(),
        row_width,
        trace_source_prefix_rows,
    )
    .map_err(|error| {
        guest_pc_device_trace_source_error(format!("CUDA trace source build failed: {error}"))
    })?;
    let builder = guest_pc_device_trace_builder(layout, trace, trace_device);
    validate_guest_pc_trace_device_source_matches_trace(layout, trace, &builder)?;
    Ok(Some(builder))
}

#[cfg(feature = "cuda")]
fn guest_pc_device_trace_builder(
    layout: &WitnessTraceLayout,
    trace: &WitnessTraceBuffer,
    trace_device: CudaDeviceBuffer,
) -> GuestPcTraceDeviceTraceBuilder {
    debug_assert_eq!(trace.row_count(), layout.row_count());
    debug_assert_eq!(trace.column_count(), layout.column_count());
    guest_pc_device_trace_builder_from_layout(layout, trace_device)
}

#[cfg(feature = "cuda")]
fn guest_pc_device_trace_builder_from_layout(
    layout: &WitnessTraceLayout,
    trace_device: CudaDeviceBuffer,
) -> GuestPcTraceDeviceTraceBuilder {
    guest_pc_device_trace_builder_from_layout_with_descriptors(layout, trace_device, None)
}

#[cfg(feature = "cuda")]
fn guest_pc_device_trace_builder_from_layout_with_descriptors(
    layout: &WitnessTraceLayout,
    trace_device: CudaDeviceBuffer,
    device_trace_descriptor_buffer: Option<Arc<CudaDeviceBuffer>>,
) -> GuestPcTraceDeviceTraceBuilder {
    let has_descriptor_source = device_trace_descriptor_buffer.is_some();
    guest_pc_device_trace_builder_from_layout_with_descriptor_source(
        layout,
        trace_device,
        device_trace_descriptor_buffer,
        has_descriptor_source,
    )
}

#[cfg(feature = "cuda")]
fn guest_pc_device_trace_builder_from_layout_with_descriptor_source(
    layout: &WitnessTraceLayout,
    trace_device: CudaDeviceBuffer,
    device_trace_descriptor_buffer: Option<Arc<CudaDeviceBuffer>>,
    has_descriptor_source: bool,
) -> GuestPcTraceDeviceTraceBuilder {
    let stages = layout
        .stages()
        .iter()
        .map(|stage| GuestPcTraceDeviceTraceStage {
            stage_index: stage.stage_index,
            row_count: layout.row_count(),
            column_count: stage.width,
            row_stride: layout.column_count(),
            column_offset: stage.start_column,
            known_zero: has_descriptor_source
                && layout.column_count() == ZISK_MAIN_DEVICE_TRACE_COLUMNS
                && stage.width == 1
                && stage.start_column == ZISK_MAIN_DEVICE_TRACE_COLUMNS - 1,
        })
        .collect::<Vec<_>>();
    GuestPcTraceDeviceTraceBuilder {
        trace: Arc::new(trace_device),
        device_trace_descriptor_buffer,
        stages,
    }
}

#[cfg(feature = "cuda")]
fn validate_guest_pc_trace_device_source_matches_layout(
    layout: &WitnessTraceLayout,
    builder: &GuestPcTraceDeviceTraceBuilder,
) -> Result<(), WitnessTraceRunError> {
    if builder.stages().len() != layout.stages().len() {
        return Err(guest_pc_device_trace_source_error(
            "device trace source stage count mismatch",
        ));
    }
    for (stage, source) in layout.stages().iter().zip(builder.stages()) {
        if source.stage_index() != stage.stage_index
            || source.row_count() != layout.row_count()
            || source.column_count() != stage.width
            || source.row_stride() != layout.column_count()
            || source.column_offset() != stage.start_column
        {
            return Err(guest_pc_device_trace_source_error(
                "device trace source stage shape mismatch",
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "cuda")]
pub(crate) fn validate_guest_pc_trace_device_source_matches_trace(
    layout: &WitnessTraceLayout,
    trace: &WitnessTraceBuffer,
    builder: &GuestPcTraceDeviceTraceBuilder,
) -> Result<(), WitnessTraceRunError> {
    validate_guest_pc_trace_device_source_matches_layout(layout, builder)?;
    if trace.row_count() != layout.row_count() || trace.column_count() != layout.column_count() {
        return Err(guest_pc_device_trace_source_error(
            "trace shape does not match guest PC layout",
        ));
    }
    if guest_pc_device_trace_source_deep_validation_enabled() {
        let actual = builder.trace().to_u64_words().map_err(|error| {
            guest_pc_device_trace_source_error(format!(
                "CUDA trace source validation download failed: {error}"
            ))
        })?;
        if actual != Felt::as_u64_slice(trace.values()) {
            return Err(guest_pc_device_trace_source_error(
                "device trace source values mismatch host trace",
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "cuda")]
fn guest_pc_device_trace_source_enabled() -> bool {
    env_flag_enabled("LZVM_CUDA_GUEST_PC_DEVICE_TRACE_SOURCE", true)
}

#[cfg(feature = "cuda")]
fn guest_pc_device_trace_source_deep_validation_enabled() -> bool {
    env_flag_enabled("LZVM_CUDA_VALIDATE_GUEST_PC_DEVICE_TRACE_SOURCE", false)
}

#[cfg(feature = "cuda")]
fn guest_pc_device_trace_source_error(message: impl Into<String>) -> WitnessTraceRunError {
    WitnessTraceRunError::from(WitnessCallError::Backend {
        message: format!("guest PC CUDA trace source failed: {}", message.into()),
    })
}

impl WitnessBackend for GuestPcTraceBackend {
    fn compute(
        &self,
        buffers: &mut WitnessTraceBuffers<'_>,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        self.compute_with_context(WitnessComputeContext::empty(), buffers)
    }

    fn compute_with_context(
        &self,
        context: WitnessComputeContext<'_>,
        buffers: &mut WitnessTraceBuffers<'_>,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        compute_guest_pc_trace(self.instruction_limit, context, buffers)
            .map_err(WitnessCallError::from)
    }
}

#[derive(Debug)]
enum GuestPcTraceBackendError {
    MissingGuestImage,
    MissingGuestImageInfo,
    GuestImageIo(std::io::Error),
    GuestMemory(GuestMemoryError),
    ZiskInput(ZiskInputFcallError),
    GuestRun(GuestMachineRunError),
    TraceBuild(WitnessTraceBuildError),
    ZiskMainLower {
        row: usize,
        source: ZiskMainLowerError,
    },
    InvalidPcTraceLayout {
        message: String,
    },
    UnsupportedZiskMainSource {
        row: usize,
    },
    UnsupportedZiskMainStore {
        row: usize,
    },
    ZiskMainEffectMismatch {
        row: usize,
        message: String,
    },
    UnmappedTraceLayout,
    TooManyRegisterWrites {
        row: usize,
        found: usize,
    },
    TooManyMemoryAccesses {
        row: usize,
        kind: GuestMemoryAccessKind,
        found: usize,
    },
    NonCanonicalTraceValue {
        row: usize,
        column: String,
        value: u64,
    },
    TraceCapacityExceeded {
        rows: usize,
        row_width: usize,
        required_rows: usize,
        required_trace_instances: usize,
    },
    OutputOverflow {
        produced_len: usize,
        output_len: usize,
    },
}

impl fmt::Display for GuestPcTraceBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingGuestImage => write!(f, "guest PC trace backend missing guest image path"),
            Self::MissingGuestImageInfo => {
                write!(f, "guest PC trace backend missing guest image metadata")
            }
            Self::GuestImageIo(error) => {
                write!(f, "guest PC trace backend guest image read failed: {error}")
            }
            Self::GuestMemory(error) => {
                write!(f, "guest PC trace backend guest memory load failed: {error}")
            }
            Self::ZiskInput(error) => {
                write!(f, "guest PC trace backend Zisk input setup failed: {error}")
            }
            Self::GuestRun(error) => write!(f, "guest PC trace backend guest run failed: {error}"),
            Self::TraceBuild(error) => {
                write!(f, "guest PC trace backend layout trace build failed: {error}")
            }
            Self::ZiskMainLower { row, source } => write!(
                f,
                "guest PC trace backend row {row} Zisk Main lowering failed: {source}"
            ),
            Self::InvalidPcTraceLayout { message } => {
                write!(f, "guest PC trace backend layout is invalid: {message}")
            }
            Self::UnsupportedZiskMainSource { row } => write!(
                f,
                "guest PC trace backend row {row} uses an unsupported Zisk Main source"
            ),
            Self::UnsupportedZiskMainStore { row } => write!(
                f,
                "guest PC trace backend row {row} uses an unsupported Zisk Main store"
            ),
            Self::ZiskMainEffectMismatch { row, message } => write!(
                f,
                "guest PC trace backend row {row} Zisk Main effects are inconsistent: {message}"
            ),
            Self::UnmappedTraceLayout => write!(
                f,
                "guest PC trace backend layout does not expose guest trace columns"
            ),
            Self::TooManyRegisterWrites { row, found } => write!(
                f,
                "guest PC trace backend row {row} has too many register writes: {found}"
            ),
            Self::TooManyMemoryAccesses { row, kind, found } => write!(
                f,
                "guest PC trace backend row {row} has too many {kind:?} memory accesses: {found}"
            ),
            Self::NonCanonicalTraceValue { row, column, value } => write!(
                f,
                "guest PC trace backend value is non-canonical at row {row} column {column}: {value}"
            ),
            Self::TraceCapacityExceeded {
                rows,
                row_width,
                required_rows,
                required_trace_instances,
            } => write!(
                f,
                "guest PC trace backend exceeded trace layout capacity: rows {rows}, row width {row_width}, required rows at least {required_rows}, required same-capacity trace instances at least {required_trace_instances}"
            ),
            Self::OutputOverflow {
                produced_len,
                output_len,
            } => write!(
                f,
                "guest PC trace backend trace exceeds output buffer: produced {produced_len}, output {output_len}"
            ),
        }
    }
}

impl std::error::Error for GuestPcTraceBackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::GuestImageIo(error) => Some(error),
            Self::GuestMemory(error) => Some(error),
            Self::ZiskInput(error) => Some(error),
            Self::GuestRun(error) => Some(error),
            Self::TraceBuild(error) => Some(error),
            Self::ZiskMainLower { source, .. } => Some(source),
            Self::MissingGuestImage
            | Self::MissingGuestImageInfo
            | Self::InvalidPcTraceLayout { .. }
            | Self::UnsupportedZiskMainSource { .. }
            | Self::UnsupportedZiskMainStore { .. }
            | Self::ZiskMainEffectMismatch { .. }
            | Self::UnmappedTraceLayout
            | Self::TooManyRegisterWrites { .. }
            | Self::TooManyMemoryAccesses { .. }
            | Self::NonCanonicalTraceValue { .. }
            | Self::TraceCapacityExceeded { .. }
            | Self::OutputOverflow { .. } => None,
        }
    }
}

impl From<GuestPcTraceBackendError> for WitnessCallError {
    fn from(error: GuestPcTraceBackendError) -> Self {
        match error {
            GuestPcTraceBackendError::OutputOverflow {
                produced_len,
                output_len,
            } => Self::OutputOverflow {
                produced_len,
                output_len,
            },
            other => Self::Backend {
                message: other.to_string(),
            },
        }
    }
}

fn compute_guest_pc_trace(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    buffers: &mut WitnessTraceBuffers<'_>,
) -> Result<WitnessTraceOutput, GuestPcTraceBackendError> {
    let layout_capacity = layout_trace_capacity(context.trace_layout)?;
    let run_instruction_limit = layout_capacity
        .map(|capacity| instruction_limit.min(capacity.instruction_limit))
        .unwrap_or(instruction_limit);
    let (mut memory, mut state, mut fcall_handler) =
        load_guest_pc_trace_machine(context, buffers.input())?;
    let trace = run_guest_machine_trace_with_fcalls(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        run_instruction_limit,
    );
    let trace = match trace {
        Ok(trace) => trace,
        Err(error) => {
            if let Some(error) =
                layout_capacity_error(layout_capacity, run_instruction_limit, &error)
            {
                return Err(error);
            }
            return Err(GuestPcTraceBackendError::GuestRun(error));
        }
    };
    let proof_values = zisk_runtime_proof_values(
        !trace.reports.is_empty(),
        fcall_handler.input_data_was_mapped(),
        state.dma_proof_value_flags(),
    );
    if let Some(layout) = context.trace_layout {
        if let Some(output) = write_layout_zisk_main_trace(
            layout,
            &trace.reports,
            guest_machine_halt_pc(&trace.run.halt),
            buffers.output_mut(),
        )? {
            let mut output = output;
            output.proof_values = proof_values;
            return Ok(output);
        }
        if let Some(produced_len) = precompile_memory_trace::write_layout_precompile_memory_trace(
            layout,
            &trace.reports,
            buffers.output_mut(),
        )? {
            return Ok(WitnessTraceOutput::with_values(
                produced_len,
                Vec::new(),
                proof_values,
            ));
        }
        if let Some(produced_len) =
            write_layout_pc_trace(layout, &trace.reports, buffers.output_mut())?
        {
            return Ok(WitnessTraceOutput::with_values(
                produced_len,
                Vec::new(),
                proof_values,
            ));
        }
    }
    let produced_len =
        trace
            .reports
            .len()
            .checked_mul(16)
            .ok_or(GuestPcTraceBackendError::OutputOverflow {
                produced_len: usize::MAX,
                output_len: buffers.output().len(),
            })?;
    if produced_len > buffers.output().len() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len,
            output_len: buffers.output().len(),
        });
    }

    let output = buffers.output_mut();
    for (index, report) in trace.reports.iter().enumerate() {
        let offset = index * 16;
        output[offset..offset + 8].copy_from_slice(&report.address.to_le_bytes());
        output[offset + 8..offset + 16].copy_from_slice(&report.next_pc.to_le_bytes());
    }
    Ok(WitnessTraceOutput::with_values(
        produced_len,
        Vec::new(),
        proof_values,
    ))
}

fn load_guest_pc_trace_machine(
    context: WitnessComputeContext<'_>,
    input: &[u8],
) -> Result<(GuestMachineMemory, GuestMachineState, ZiskInputFcallHandler), GuestPcTraceBackendError>
{
    let guest_image = context
        .guest_image
        .ok_or(GuestPcTraceBackendError::MissingGuestImage)?;
    let guest_image_info = context
        .guest_image_info
        .ok_or(GuestPcTraceBackendError::MissingGuestImageInfo)?;
    let guest_image_bytes =
        std::fs::read(guest_image).map_err(GuestPcTraceBackendError::GuestImageIo)?;
    let memory_image = load_guest_memory_image(&guest_image_bytes, guest_image_info)
        .map_err(GuestPcTraceBackendError::GuestMemory)?;
    let mut memory = GuestMachineMemory::from_image(&memory_image);
    memory
        .map_zeroed_gap_range(ZISK_RAM_ADDRESS, ZISK_RAM_SIZE)
        .map_err(GuestPcTraceBackendError::GuestMemory)?;
    let state = GuestMachineState::new(memory.entry_address());
    let fcall_handler =
        ZiskInputFcallHandler::new(input).map_err(GuestPcTraceBackendError::ZiskInput)?;
    Ok((memory, state, fcall_handler))
}

fn run_guest_pc_trace_segment_slice(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    instruction_limit: u64,
    row_limit: usize,
) -> Result<GuestPcTraceSegmentSlice, GuestPcTraceBackendError> {
    run_guest_pc_trace_segment_slice_inner::<false, true>(
        memory,
        state,
        handler,
        instruction_limit,
        row_limit,
        None,
    )
}

fn replay_guest_pc_trace_segment_from_snapshot(
    snapshot: GuestPcTraceSegmentReplaySnapshot,
    instruction_limit: u64,
    row_limit: usize,
) -> Result<GuestPcTraceSegmentReplay, GuestPcTraceBackendError> {
    let GuestPcTraceSegmentReplaySnapshot {
        mut memory,
        mut state,
        mut fcall_handler,
    } = snapshot;
    let slice = run_guest_pc_trace_segment_slice(
        &mut memory,
        &mut state,
        &mut *fcall_handler,
        instruction_limit,
        row_limit,
    )?;
    Ok(GuestPcTraceSegmentReplay {
        slice,
        memory,
        state,
        fcall_handler,
    })
}

fn replay_guest_pc_trace_segment_reports_for_seed_advance(
    snapshot: GuestPcTraceSegmentReplaySnapshot,
    instruction_limit: u64,
    row_limit: usize,
    slice: &GuestPcTraceSegmentSlice,
    trace_instance_index: u32,
) -> Result<Vec<GuestMachineReport>, GuestPcTraceBackendError> {
    let replay =
        replay_guest_pc_trace_segment_from_snapshot(snapshot, instruction_limit, row_limit)?;
    if replay.slice.executed_instructions != slice.executed_instructions
        || replay.slice.trace_rows != slice.trace_rows
        || replay.slice.status != slice.status
        || replay.slice.report_count != slice.report_count
        || (!slice.reports.is_empty() && replay.slice.reports != slice.reports)
    {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "guest PC trace seed validation replay diverged for segment {trace_instance_index}"
            ),
        });
    }
    Ok(replay.slice.reports)
}

fn run_guest_pc_trace_segment_slice_with_boundary_snapshot(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    instruction_limit: u64,
    row_limit: usize,
    boundary_snapshot: &mut ZiskMainRunnerBoundarySnapshot,
) -> Result<GuestPcTraceSegmentSlice, GuestPcTraceBackendError> {
    run_guest_pc_trace_segment_slice_inner::<true, true>(
        memory,
        state,
        handler,
        instruction_limit,
        row_limit,
        Some(boundary_snapshot),
    )
}

fn run_guest_pc_trace_segment_slice_with_elided_reports_and_boundary_snapshot(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    instruction_limit: u64,
    row_limit: usize,
    boundary_snapshot: &mut ZiskMainRunnerBoundarySnapshot,
) -> Result<GuestPcTraceSegmentSlice, GuestPcTraceBackendError> {
    run_guest_pc_trace_segment_slice_inner::<true, false>(
        memory,
        state,
        handler,
        instruction_limit,
        row_limit,
        Some(boundary_snapshot),
    )
}

#[allow(dead_code)]
fn run_guest_pc_trace_segment_slice_with_live_report_chunks(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    instruction_limit: u64,
    row_limit: usize,
    mut boundary_snapshot: Option<&mut ZiskMainRunnerBoundarySnapshot>,
    mut emit_report: impl FnMut(GuestMachineReport) -> Result<(), GuestPcTraceBackendError>,
) -> Result<GuestPcTraceSegmentSlice, GuestPcTraceBackendError> {
    let mut pending_report = None;
    let mut report_count = 0_usize;
    let mut executed_instructions = 0_u64;
    let mut trace_rows = 0_usize;
    loop {
        let pc = state.pc();
        let prepared = prepare_current_guest_instruction(memory, pc)
            .map_err(GuestMachineRunError::from)
            .map_err(GuestPcTraceBackendError::GuestRun)?;
        let current = prepared.instruction();
        if let (Some(snapshot), Some(report)) =
            (boundary_snapshot.as_deref_mut(), pending_report.as_ref())
        {
            snapshot.record_report(report, Some(current), state.registers())?;
        }
        if current == RiscvInstruction::Ecall {
            return finish_guest_pc_trace_live_report_chunk_segment_slice(
                pending_report,
                report_count,
                &mut emit_report,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Halted(GuestMachineHalt::Ecall { address: pc }),
            );
        }
        if executed_instructions == instruction_limit {
            return finish_guest_pc_trace_live_report_chunk_segment_slice(
                pending_report,
                report_count,
                &mut emit_report,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                },
            );
        }
        let max_rows = zisk_main_instruction_max_rows(current);
        if max_rows > row_limit {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main layout cannot fit the next guest instruction".to_owned(),
            });
        }
        let required_rows = trace_rows.checked_add(max_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main row count overflow".to_owned(),
            }
        })?;
        if trace_rows != 0 && required_rows > row_limit {
            return finish_guest_pc_trace_live_report_chunk_segment_slice(
                pending_report,
                report_count,
                &mut emit_report,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                },
            );
        }
        let report = advance_guest_machine_with_prepared_fcalls(memory, state, handler, prepared)
            .map_err(GuestMachineRunError::from)
            .map_err(GuestPcTraceBackendError::GuestRun)?;
        let report_rows = zisk_main_report_row_count(report_count, &report)?;
        let next_trace_rows = trace_rows.checked_add(report_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main row count overflow".to_owned(),
            }
        })?;
        if next_trace_rows > row_limit {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main report rows exceed layout rows".to_owned(),
            });
        }
        trace_rows = next_trace_rows;
        if let Some(previous) = pending_report.replace(report) {
            emit_report(previous)?;
        }
        report_count = report_count.checked_add(1).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest PC trace report count overflow".to_owned(),
            }
        })?;
        executed_instructions += 1;
        if trace_rows == row_limit {
            let pc = state.pc();
            let current = decode_current_guest_instruction(memory, pc)
                .map_err(GuestMachineRunError::from)
                .map_err(GuestPcTraceBackendError::GuestRun)?;
            let lookahead_instruction = (current != RiscvInstruction::Ecall).then_some(current);
            if let (Some(snapshot), Some(report)) =
                (boundary_snapshot.as_deref_mut(), pending_report.as_ref())
            {
                snapshot.record_report(report, lookahead_instruction, state.registers())?;
            }
            let status = if current == RiscvInstruction::Ecall {
                GuestMachineTraceSliceStatus::Halted(GuestMachineHalt::Ecall { address: pc })
            } else {
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                }
            };
            return finish_guest_pc_trace_live_report_chunk_segment_slice(
                pending_report,
                report_count,
                &mut emit_report,
                executed_instructions,
                trace_rows,
                status,
            );
        }
    }
}

#[allow(dead_code)]
fn finish_guest_pc_trace_live_report_chunk_segment_slice(
    pending_report: Option<GuestMachineReport>,
    report_count: usize,
    emit_report: &mut impl FnMut(GuestMachineReport) -> Result<(), GuestPcTraceBackendError>,
    executed_instructions: u64,
    trace_rows: usize,
    status: GuestMachineTraceSliceStatus,
) -> Result<GuestPcTraceSegmentSlice, GuestPcTraceBackendError> {
    let last_report = match pending_report {
        Some(report) => {
            let last_report = report.clone();
            emit_report(report)?;
            Some(last_report)
        }
        None => None,
    };
    Ok(GuestPcTraceSegmentSlice {
        executed_instructions,
        trace_rows,
        status,
        report_count,
        report_capacity: 0,
        last_report,
        reports: Vec::new(),
    })
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn run_guest_pc_trace_segment_slice_with_streaming_device_material(
    layout: &WitnessTraceLayout,
    initial_state: &ZiskMainTraceState,
    segment: ZiskMainTraceSegmentInfo,
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut ZiskInputFcallHandler,
    instruction_limit: u64,
    row_limit: usize,
) -> Result<Option<GuestPcTraceRunnerStreamingSegment>, GuestPcTraceBackendError> {
    if row_limit != layout.row_count() {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "streaming device material runner requires the full segment row limit"
                .to_owned(),
        });
    }
    let Some(mut builder) =
        ZiskMainStreamingDeviceSegmentBuilder::new(layout, initial_state, segment)?
    else {
        return Ok(None);
    };

    let timing_config = ZiskMainTraceLowerTimingConfig::from_env();
    let mut reports = Vec::new();
    let mut pending_report = None;
    let mut next_report_index = 0_usize;
    let mut executed_instructions = 0_u64;
    let mut trace_rows = 0_usize;

    loop {
        let pc = state.pc();
        let prepared = prepare_current_guest_instruction(memory, pc)
            .map_err(GuestMachineRunError::from)
            .map_err(GuestPcTraceBackendError::GuestRun)?;
        let current = prepared.instruction();
        if current == RiscvInstruction::Ecall {
            return finish_guest_pc_trace_streaming_device_segment(
                builder,
                reports,
                &mut pending_report,
                &mut next_report_index,
                timing_config,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Halted(GuestMachineHalt::Ecall { address: pc }),
                pc,
                None,
            )
            .map(Some);
        }
        if executed_instructions == instruction_limit {
            return finish_guest_pc_trace_streaming_device_segment(
                builder,
                reports,
                &mut pending_report,
                &mut next_report_index,
                timing_config,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                },
                pc,
                Some(current),
            )
            .map(Some);
        }
        let max_rows = zisk_main_instruction_max_rows(current);
        if max_rows > row_limit {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "main trace layout cannot fit the next guest instruction".to_owned(),
            });
        }
        let required_rows = trace_rows.checked_add(max_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "main trace row count overflow".to_owned(),
            }
        })?;
        if trace_rows != 0 && required_rows > row_limit {
            return finish_guest_pc_trace_streaming_device_segment(
                builder,
                reports,
                &mut pending_report,
                &mut next_report_index,
                timing_config,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                },
                pc,
                Some(current),
            )
            .map(Some);
        }
        let report = advance_guest_machine_with_prepared_fcalls(memory, state, handler, prepared)
            .map_err(GuestMachineRunError::from)
            .map_err(GuestPcTraceBackendError::GuestRun)?;
        let report_rows = zisk_main_report_row_count(reports.len(), &report)?;
        let next_trace_rows = trace_rows.checked_add(report_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "main trace row count overflow".to_owned(),
            }
        })?;
        if next_trace_rows > row_limit {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "main trace report rows exceed layout rows".to_owned(),
            });
        }
        trace_rows = next_trace_rows;
        push_guest_pc_trace_streaming_device_report(
            &mut builder,
            &mut reports,
            &mut pending_report,
            &mut next_report_index,
            timing_config,
            report,
        )?;
        executed_instructions += 1;
        if trace_rows == row_limit {
            let pc = state.pc();
            let current = decode_current_guest_instruction(memory, pc)
                .map_err(GuestMachineRunError::from)
                .map_err(GuestPcTraceBackendError::GuestRun)?;
            let lookahead_instruction = (current != RiscvInstruction::Ecall).then_some(current);
            let status = if current == RiscvInstruction::Ecall {
                GuestMachineTraceSliceStatus::Halted(GuestMachineHalt::Ecall { address: pc })
            } else {
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                }
            };
            return finish_guest_pc_trace_streaming_device_segment(
                builder,
                reports,
                &mut pending_report,
                &mut next_report_index,
                timing_config,
                executed_instructions,
                trace_rows,
                status,
                pc,
                lookahead_instruction,
            )
            .map(Some);
        }
    }
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
fn push_guest_pc_trace_streaming_device_report(
    builder: &mut ZiskMainStreamingDeviceSegmentBuilder,
    reports: &mut Vec<GuestMachineReport>,
    pending_report: &mut Option<GuestMachineReport>,
    next_report_index: &mut usize,
    timing_config: ZiskMainTraceLowerTimingConfig,
    report: GuestMachineReport,
) -> Result<(), GuestPcTraceBackendError> {
    if let Some(pending) = pending_report.take() {
        let next_instruction = report.instruction;
        builder.push_report_at(
            *next_report_index,
            &pending,
            || Some(next_instruction),
            timing_config,
            None,
        )?;
        *next_report_index = next_report_index.checked_add(1).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "main trace report index overflow".to_owned(),
            }
        })?;
        reports.push(pending);
    }
    *pending_report = Some(report);
    Ok(())
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn finish_guest_pc_trace_streaming_device_segment(
    mut builder: ZiskMainStreamingDeviceSegmentBuilder,
    mut reports: Vec<GuestMachineReport>,
    pending_report: &mut Option<GuestMachineReport>,
    next_report_index: &mut usize,
    timing_config: ZiskMainTraceLowerTimingConfig,
    executed_instructions: u64,
    trace_rows: usize,
    status: GuestMachineTraceSliceStatus,
    terminal_pc: u64,
    lookahead_instruction: Option<RiscvInstruction>,
) -> Result<GuestPcTraceRunnerStreamingSegment, GuestPcTraceBackendError> {
    if let Some(pending) = pending_report.take() {
        builder.push_report_at(
            *next_report_index,
            &pending,
            || lookahead_instruction,
            timing_config,
            None,
        )?;
        *next_report_index = next_report_index.checked_add(1).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "main trace report index overflow".to_owned(),
            }
        })?;
        reports.push(pending);
    }
    let report_capacity = reports.capacity();
    let report_count = reports.len();
    let last_report = reports.last().cloned();
    let device_build = builder.finish(terminal_pc, None)?;
    let next_seed = ZiskMainSegmentSeed {
        initial_state: device_build.continuation_state.clone(),
        previous_c: device_build.final_state.last_c,
    };
    Ok(GuestPcTraceRunnerStreamingSegment {
        slice: GuestPcTraceSegmentSlice {
            executed_instructions,
            trace_rows,
            status,
            report_count,
            report_capacity,
            last_report,
            reports,
        },
        terminal_pc,
        lookahead_instruction,
        device_build,
        next_seed,
    })
}

fn finish_guest_pc_trace_segment_slice(
    reports: Vec<GuestMachineReport>,
    last_report: Option<GuestMachineReport>,
    report_count: usize,
    retain_reports: bool,
    executed_instructions: u64,
    trace_rows: usize,
    status: GuestMachineTraceSliceStatus,
) -> GuestPcTraceSegmentSlice {
    let last_report = if retain_reports {
        reports.last().cloned()
    } else {
        last_report
    };
    GuestPcTraceSegmentSlice {
        executed_instructions,
        trace_rows,
        status,
        report_count,
        report_capacity: if retain_reports {
            reports.capacity()
        } else {
            0
        },
        last_report,
        reports,
    }
}

fn run_guest_pc_trace_segment_slice_inner<
    const TRACK_BOUNDARY: bool,
    const RETAIN_REPORTS: bool,
>(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    instruction_limit: u64,
    row_limit: usize,
    mut boundary_snapshot: Option<&mut ZiskMainRunnerBoundarySnapshot>,
) -> Result<GuestPcTraceSegmentSlice, GuestPcTraceBackendError> {
    let mut reports = Vec::new();
    let mut last_report = None;
    let mut report_count = 0_usize;
    let mut executed_instructions = 0_u64;
    let mut trace_rows = 0_usize;
    loop {
        let pc = state.pc();
        let prepared = prepare_current_guest_instruction(memory, pc)
            .map_err(GuestMachineRunError::from)
            .map_err(GuestPcTraceBackendError::GuestRun)?;
        let current = prepared.instruction();
        if TRACK_BOUNDARY {
            let boundary_report = if RETAIN_REPORTS {
                reports.last()
            } else {
                last_report.as_ref()
            };
            if let (Some(snapshot), Some(report)) =
                (boundary_snapshot.as_deref_mut(), boundary_report)
            {
                snapshot.record_report(report, Some(current), state.registers())?;
            }
        }
        if current == RiscvInstruction::Ecall {
            return Ok(finish_guest_pc_trace_segment_slice(
                reports,
                last_report,
                report_count,
                RETAIN_REPORTS,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Halted(GuestMachineHalt::Ecall { address: pc }),
            ));
        }
        if executed_instructions == instruction_limit {
            return Ok(finish_guest_pc_trace_segment_slice(
                reports,
                last_report,
                report_count,
                RETAIN_REPORTS,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                },
            ));
        }
        let max_rows = zisk_main_instruction_max_rows(current);
        if max_rows > row_limit {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main layout cannot fit the next guest instruction".to_owned(),
            });
        }
        let required_rows = trace_rows.checked_add(max_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main row count overflow".to_owned(),
            }
        })?;
        if trace_rows != 0 && required_rows > row_limit {
            return Ok(finish_guest_pc_trace_segment_slice(
                reports,
                last_report,
                report_count,
                RETAIN_REPORTS,
                executed_instructions,
                trace_rows,
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                },
            ));
        }
        let report = advance_guest_machine_with_prepared_fcalls(memory, state, handler, prepared)
            .map_err(GuestMachineRunError::from)
            .map_err(GuestPcTraceBackendError::GuestRun)?;
        let report_rows = zisk_main_report_row_count(report_count, &report)?;
        let next_trace_rows = trace_rows.checked_add(report_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main row count overflow".to_owned(),
            }
        })?;
        if next_trace_rows > row_limit {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main report rows exceed layout rows".to_owned(),
            });
        }
        trace_rows = next_trace_rows;
        if RETAIN_REPORTS {
            reports.push(report);
        } else {
            last_report = Some(report);
        }
        report_count = report_count.checked_add(1).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest PC trace report count overflow".to_owned(),
            }
        })?;
        executed_instructions += 1;
        if trace_rows == row_limit {
            let pc = state.pc();
            let current = decode_current_guest_instruction(memory, pc)
                .map_err(GuestMachineRunError::from)
                .map_err(GuestPcTraceBackendError::GuestRun)?;
            let lookahead_instruction = (current != RiscvInstruction::Ecall).then_some(current);
            if TRACK_BOUNDARY {
                let boundary_report = if RETAIN_REPORTS {
                    reports.last()
                } else {
                    last_report.as_ref()
                };
                if let (Some(snapshot), Some(report)) =
                    (boundary_snapshot.as_deref_mut(), boundary_report)
                {
                    snapshot.record_report(report, lookahead_instruction, state.registers())?;
                }
            }
            let status = if current == RiscvInstruction::Ecall {
                GuestMachineTraceSliceStatus::Halted(GuestMachineHalt::Ecall { address: pc })
            } else {
                GuestMachineTraceSliceStatus::Paused {
                    pc,
                    instruction: current,
                }
            };
            return Ok(finish_guest_pc_trace_segment_slice(
                reports,
                last_report,
                report_count,
                RETAIN_REPORTS,
                executed_instructions,
                trace_rows,
                status,
            ));
        }
    }
}

fn zisk_main_instruction_max_rows(instruction: RiscvInstruction) -> usize {
    match instruction {
        RiscvInstruction::Amo {
            kind: RiscvAmoKind::Add,
            rd,
            rs1,
            rs2,
            ..
        } => amo_add_row_count(rd, rs1, rs2),
        RiscvInstruction::StoreConditional { rd, .. } if rd != 0 => 2,
        _ => 1,
    }
}

fn zisk_main_report_row_count(
    row: usize,
    report: &GuestMachineReport,
) -> Result<usize, GuestPcTraceBackendError> {
    match report.instruction {
        RiscvInstruction::Amo {
            kind: RiscvAmoKind::Add,
            rd,
            rs1,
            rs2,
            ..
        } => Ok(amo_add_row_count(rd, rs1, rs2)),
        RiscvInstruction::StoreConditional { rd, .. } => {
            if !report
                .memory_accesses
                .iter()
                .any(|access| access.kind == GuestMemoryAccessKind::Write)
            {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message:
                        "StoreConditional without a memory write is not supported by Zisk Main lowering"
                            .to_owned(),
                });
            }
            Ok(if rd == 0 { 1 } else { 2 })
        }
        _ => Ok(1),
    }
}

fn amo_add_row_count(rd: u8, rs1: u8, rs2: u8) -> usize {
    if rd != 0 && (rd == rs1 || rd == rs2) {
        4
    } else {
        3
    }
}

fn compute_guest_pc_trace_segments(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    input: &[u8],
) -> Result<Vec<GuestPcTraceSegmentTrace>, GuestPcTraceBackendError> {
    let layout = context
        .trace_layout
        .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
    if zisk_main_trace_columns(layout)?.is_none() {
        return Err(GuestPcTraceBackendError::UnmappedTraceLayout);
    }
    let row_count = layout.row_count();
    if row_count == 0 {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main layout has zero rows".to_owned(),
        });
    }

    let (mut memory, mut state, mut fcall_handler) = load_guest_pc_trace_machine(context, input)?;
    let mut trace_state = ZiskMainTraceState::new();
    let mut previous_c = 0_u64;
    let mut executed_instructions = 0_u64;
    let mut outputs = Vec::new();
    loop {
        let remaining_limit = instruction_limit.saturating_sub(executed_instructions);
        let slice = run_guest_pc_trace_segment_slice(
            &mut memory,
            &mut state,
            &mut fcall_handler,
            remaining_limit,
            row_count,
        )?;
        executed_instructions = executed_instructions
            .checked_add(slice.executed_instructions)
            .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest instruction count overflow".to_owned(),
            })?;
        let (halted, terminal_pc, lookahead_instruction) = match &slice.status {
            GuestMachineTraceSliceStatus::Halted(halt) => (true, guest_machine_halt_pc(halt), None),
            GuestMachineTraceSliceStatus::Paused { pc, instruction } => {
                (false, *pc, Some(*instruction))
            }
        };
        let needs_terminal_segment = halted && slice.trace_rows == row_count;
        let is_last_segment = halted && !needs_terminal_segment;
        if !is_last_segment && slice.trace_rows < row_count {
            return Err(GuestPcTraceBackendError::GuestRun(
                GuestMachineRunError::InstructionLimitExceeded {
                    instruction_limit,
                    pc: terminal_pc,
                },
            ));
        }
        let trace_instance_index = u32::try_from(outputs.len()).map_err(|_| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main trace instance index is too large".to_owned(),
            }
        })?;
        let written = build_layout_zisk_main_trace_segment_for_segment_output(
            layout,
            &slice.reports,
            terminal_pc,
            &trace_state,
            lookahead_instruction,
            ZiskMainTraceSegmentInfo {
                trace_instance_index,
                is_last_segment,
                previous_c,
            },
            None,
        )?
        .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
        previous_c = written.final_state.last_c;
        trace_state = written.continuation_state;
        outputs.push(GuestPcTraceSegmentTrace {
            trace_instance_index,
            trace_source_prefix_rows: written.trace_source_prefix_rows,
            #[cfg(feature = "cuda")]
            device_segment_material: written.device_segment_material,
            trace: written.trace,
            unit_values: written.output.unit_values,
            proof_values: Vec::new(),
        });
        if is_last_segment {
            break;
        }
        if needs_terminal_segment {
            continue;
        }
        if executed_instructions == instruction_limit {
            return Err(GuestPcTraceBackendError::GuestRun(
                GuestMachineRunError::InstructionLimitExceeded {
                    instruction_limit,
                    pc: terminal_pc,
                },
            ));
        }
    }

    let proof_values = zisk_runtime_proof_values(
        executed_instructions != 0,
        fcall_handler.input_data_was_mapped(),
        state.dma_proof_value_flags(),
    );
    for output in &mut outputs {
        output.proof_values = proof_values.clone();
    }
    Ok(outputs)
}

fn stream_backend_error<E>(error: GuestPcTraceBackendError) -> GuestPcTraceSegmentStreamError<E> {
    GuestPcTraceSegmentStreamError::Trace(WitnessTraceRunError::from(WitnessCallError::from(error)))
}

fn for_each_guest_pc_trace_segment<E>(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    input: &[u8],
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    mut emit: impl FnMut(GuestPcTraceSegmentTrace) -> Result<(), GuestPcTraceSegmentStreamError<E>>,
) -> Result<GuestPcTraceStreamResult, GuestPcTraceSegmentStreamError<E>> {
    let (sender, receiver) = mpsc::sync_channel(guest_pc_trace_segment_queue_capacity());
    thread::scope(|scope| {
        let producer = spawn_guest_pc_trace_thread(scope, "lzvm-gp-lower", move || {
            let mut segment_send_wait_duration = Duration::ZERO;
            let produced = produce_guest_pc_trace_segments(
                instruction_limit,
                context,
                input,
                expected_proof_values,
                |segment| {
                    let send_started = Instant::now();
                    sender
                        .send(GuestPcTraceSegmentStreamMessage::Segment(Box::new(segment)))
                        .map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
                            message: "guest PC trace segment consumer stopped".to_owned(),
                        })?;
                    segment_send_wait_duration += send_started.elapsed();
                    Ok(())
                },
            );
            let message = match produced {
                Ok(mut stream) => {
                    stream.timing.segment_send_wait_duration += segment_send_wait_duration;
                    GuestPcTraceSegmentStreamMessage::Complete(Box::new(stream))
                }
                Err(error) => GuestPcTraceSegmentStreamMessage::Error(error),
            };
            let _ = sender.send(message);
        });

        let mut emit_error = None;
        let mut stream_result: Option<
            Result<GuestPcTraceStreamResult, GuestPcTraceSegmentStreamError<E>>,
        > = None;
        let mut segment_receive_wait_duration = Duration::ZERO;
        loop {
            let receive_started = Instant::now();
            let message = match receiver.recv() {
                Ok(message) => message,
                Err(_) => break,
            };
            segment_receive_wait_duration += receive_started.elapsed();
            match message {
                GuestPcTraceSegmentStreamMessage::Segment(segment) => {
                    if emit_error.is_none() {
                        if let Err(error) = emit(*segment) {
                            emit_error = Some(error);
                        }
                    }
                }
                GuestPcTraceSegmentStreamMessage::Complete(mut stream) => {
                    stream.timing.segment_receive_wait_duration += segment_receive_wait_duration;
                    stream_result = Some(Ok(*stream));
                    break;
                }
                GuestPcTraceSegmentStreamMessage::Error(error) => {
                    stream_result = Some(Err(stream_backend_error::<E>(error)));
                    break;
                }
            }
        }
        if let Err(payload) = producer.join() {
            std::panic::resume_unwind(payload);
        }
        if let Some(error) = emit_error {
            return Err(error);
        }
        stream_result.unwrap_or_else(|| {
            Err(stream_backend_error::<E>(
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace segment producer stopped".to_owned(),
                },
            ))
        })
    })
}

fn guest_pc_trace_segment_queue_capacity() -> usize {
    std::env::var("LZVM_GUEST_PC_TRACE_SEGMENT_QUEUE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
}

fn guest_pc_trace_report_chunks_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_REPORT_CHUNKS", false)
}

fn guest_pc_trace_report_chunk_capacity() -> usize {
    std::env::var("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(4096)
}

fn spawn_guest_pc_trace_thread<'scope, 'env, F, T>(
    scope: &'scope thread::Scope<'scope, 'env>,
    name: &'static str,
    f: F,
) -> thread::ScopedJoinHandle<'scope, T>
where
    F: FnOnce() -> T + Send + 'scope,
    T: Send + 'scope,
{
    thread::Builder::new()
        .name(name.to_owned())
        .spawn_scoped(scope, f)
        .expect("guest PC trace thread should spawn")
}

fn produce_guest_pc_trace_segments(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    input: &[u8],
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    mut emit: impl FnMut(GuestPcTraceSegmentTrace) -> Result<(), GuestPcTraceBackendError>,
) -> Result<GuestPcTraceStreamResult, GuestPcTraceBackendError> {
    let layout = context
        .trace_layout
        .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
    if zisk_main_trace_columns(layout)?.is_none() {
        return Err(GuestPcTraceBackendError::UnmappedTraceLayout);
    }
    let row_count = layout.row_count();
    if row_count == 0 {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main layout has zero rows".to_owned(),
        });
    }

    let (pending_sender, pending_receiver) =
        mpsc::sync_channel(guest_pc_trace_segment_queue_capacity());
    thread::scope(|scope| {
        let runner = spawn_guest_pc_trace_thread(scope, "lzvm-gp-runner", move || {
            let runner_started = Instant::now();
            let mut pending_send_wait_duration = Duration::ZERO;
            let mut report_chunk_timing = GuestPcTraceStreamTiming::default();
            let report_chunks_enabled = guest_pc_trace_report_chunks_enabled();
            let live_report_chunks_enabled = guest_pc_trace_live_report_chunks_enabled();
            let report_chunk_capacity = guest_pc_trace_report_chunk_capacity();
            let mut send_message =
                |message: GuestPcTracePendingSegmentMessage| -> Result<
                    (),
                    GuestPcTraceBackendError,
                > {
                    let send_started = Instant::now();
                    pending_sender.send(message).map_err(|_| {
                        GuestPcTraceBackendError::InvalidPcTraceLayout {
                            message: "guest PC trace pending segment consumer stopped".to_owned(),
                        }
                    })?;
                    pending_send_wait_duration += send_started.elapsed();
                    Ok(())
                };
            let produced = if live_report_chunks_enabled {
                produce_guest_pc_trace_live_pending_messages(
                    instruction_limit,
                    context,
                    input,
                    row_count,
                    report_chunk_capacity,
                    &mut send_message,
                )
            } else {
                produce_guest_pc_trace_pending_slices(
                    instruction_limit,
                    context,
                    input,
                    row_count,
                    |mut pending| {
                        if report_chunks_enabled
                            && !pending.reports_elided
                            && !pending.reports.is_empty()
                        {
                            let trace_instance_index = pending.trace_instance_index;
                            let trace_row_count = pending.trace_row_count;
                            let reports = std::mem::take(&mut pending.reports);
                            send_message(GuestPcTracePendingSegmentMessage::SegmentStarted(
                                Box::new(pending),
                            ))?;
                            let mut reports = reports.into_iter();
                            loop {
                                let mut chunk_reports = Vec::with_capacity(report_chunk_capacity);
                                for _ in 0..report_chunk_capacity {
                                    let Some(report) = reports.next() else {
                                        break;
                                    };
                                    chunk_reports.push(report);
                                }
                                if chunk_reports.is_empty() {
                                    break;
                                }
                                let chunk_report_count = chunk_reports.len();
                                report_chunk_timing.trace_report_chunk_sent_count =
                                    report_chunk_timing
                                        .trace_report_chunk_sent_count
                                        .saturating_add(1);
                                report_chunk_timing.trace_report_chunk_report_count =
                                    report_chunk_timing
                                        .trace_report_chunk_report_count
                                        .saturating_add(chunk_report_count);
                                send_message(GuestPcTracePendingSegmentMessage::ReportChunk(
                                    Box::new(GuestPcTracePendingReportChunk {
                                        trace_instance_index,
                                        reports: chunk_reports,
                                    }),
                                ))?;
                            }
                            report_chunk_timing.trace_report_chunk_row_count = report_chunk_timing
                                .trace_report_chunk_row_count
                                .saturating_add(trace_row_count);
                            send_message(GuestPcTracePendingSegmentMessage::SegmentFinished(
                                Box::new(GuestPcTracePendingSegmentFinish {
                                    trace_instance_index,
                                }),
                            ))?;
                            return Ok(());
                        }
                        send_message(GuestPcTracePendingSegmentMessage::Segment(Box::new(
                            pending,
                        )))
                    },
                )
            };
            let message = match produced {
                Ok(produced) => {
                    let mut timing = produced.timing;
                    timing.add(report_chunk_timing);
                    timing.runner_duration += runner_started.elapsed();
                    timing.pending_send_wait_duration += pending_send_wait_duration;
                    GuestPcTracePendingSegmentMessage::Complete(Box::new(
                        GuestPcTraceStreamResult {
                            proof_values: produced.proof_values,
                            timing,
                        },
                    ))
                }
                Err(error) => GuestPcTracePendingSegmentMessage::Error(error),
            };
            let _ = pending_sender.send(message);
        });

        let lowerer_started = Instant::now();
        let mut timing = GuestPcTraceStreamTiming::default();
        let result = lower_guest_pc_trace_pending_segments(
            layout,
            pending_receiver,
            expected_proof_values,
            &mut timing,
            &mut emit,
        );
        timing.lowerer_duration += lowerer_started.elapsed();
        if let Err(payload) = runner.join() {
            std::panic::resume_unwind(payload);
        }
        let mut stream = result?;
        stream.timing.add(timing);
        if expected_proof_values.is_some_and(|expected| stream.proof_values.as_slice() != expected)
        {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest PC trace runtime proof values changed between passes".to_owned(),
            });
        }
        Ok(stream)
    })
}

struct GuestPcTracePendingSliceProduction {
    proof_values: Vec<WitnessTraceProofValue>,
    timing: GuestPcTraceStreamTiming,
}

struct GuestPcTraceLivePendingSegmentEmission {
    executed_instruction_count: u64,
    trace_row_count: usize,
    report_count: usize,
    stream_start_count: usize,
    report_chunk_count: usize,
    #[allow(dead_code)]
    status: GuestMachineTraceSliceStatus,
    #[allow(dead_code)]
    last_report: Option<GuestMachineReport>,
    #[allow(dead_code)]
    lookahead_instruction: Option<RiscvInstruction>,
    terminal_pc: u64,
    is_last_segment: bool,
    needs_terminal_segment: bool,
}

#[allow(clippy::too_many_arguments)]
fn emit_guest_pc_trace_live_pending_segment_messages(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    trace_instance_index: u32,
    runner_remaining_instruction_limit: u64,
    row_count: usize,
    seed: Option<Box<ZiskMainSegmentSeed>>,
    replay_snapshot: Option<&GuestPcTraceSegmentReplaySnapshot>,
    boundary_snapshot: Option<&mut ZiskMainRunnerBoundarySnapshot>,
    emit_stream_start_before_chunks: bool,
    report_chunk_capacity: usize,
    mut emit_message: impl FnMut(
        GuestPcTracePendingSegmentMessage,
    ) -> Result<(), GuestPcTraceBackendError>,
) -> Result<GuestPcTraceLivePendingSegmentEmission, GuestPcTraceBackendError> {
    let report_chunk_capacity = report_chunk_capacity.max(1);
    let mut chunk_reports = Vec::with_capacity(report_chunk_capacity);
    let mut stream_start_count = 0_usize;
    let mut report_chunk_count = 0_usize;
    if emit_stream_start_before_chunks {
        emit_message(GuestPcTracePendingSegmentMessage::SegmentStreamStarted(
            Box::new(GuestPcTracePendingSegmentStreamStart {
                trace_instance_index,
                runner_remaining_instruction_limit,
                seed: seed.clone(),
                replay_snapshot: None,
            }),
        ))?;
        stream_start_count = 1;
    }
    let slice = run_guest_pc_trace_segment_slice_with_live_report_chunks(
        memory,
        state,
        handler,
        runner_remaining_instruction_limit,
        row_count,
        boundary_snapshot,
        |report| {
            chunk_reports.push(report);
            if chunk_reports.len() == report_chunk_capacity {
                let reports = std::mem::take(&mut chunk_reports);
                emit_message(GuestPcTracePendingSegmentMessage::ReportChunk(Box::new(
                    GuestPcTracePendingReportChunk {
                        trace_instance_index,
                        reports,
                    },
                )))?;
                report_chunk_count = report_chunk_count.saturating_add(1);
                chunk_reports = Vec::with_capacity(report_chunk_capacity);
            }
            Ok(())
        },
    )?;
    if !chunk_reports.is_empty() {
        emit_message(GuestPcTracePendingSegmentMessage::ReportChunk(Box::new(
            GuestPcTracePendingReportChunk {
                trace_instance_index,
                reports: chunk_reports,
            },
        )))?;
        report_chunk_count = report_chunk_count.saturating_add(1);
    }

    let status = slice.status.clone();
    let (halted, terminal_pc, lookahead_instruction) = match &status {
        GuestMachineTraceSliceStatus::Halted(halt) => (true, guest_machine_halt_pc(halt), None),
        GuestMachineTraceSliceStatus::Paused { pc, instruction } => {
            (false, *pc, Some(*instruction))
        }
    };
    let needs_terminal_segment = halted && slice.trace_rows == row_count;
    let is_last_segment = halted && !needs_terminal_segment;
    if !is_last_segment && slice.trace_rows < row_count {
        return Err(GuestPcTraceBackendError::GuestRun(
            GuestMachineRunError::InstructionLimitExceeded {
                instruction_limit: runner_remaining_instruction_limit,
                pc: terminal_pc,
            },
        ));
    }
    let pending = GuestPcTracePendingSegmentSlice {
        trace_instance_index,
        executed_instruction_count: slice.executed_instructions,
        trace_row_count: slice.trace_rows,
        runner_remaining_instruction_limit,
        report_count: slice.report_count,
        report_capacity: slice.report_capacity,
        reports: Vec::new(),
        reports_elided: false,
        terminal_pc,
        lookahead_instruction,
        is_last_segment,
        seed,
        replay_snapshot: is_last_segment.then(|| replay_snapshot.cloned()).flatten(),
    };
    if pending.report_count == 0 {
        emit_message(GuestPcTracePendingSegmentMessage::Segment(Box::new(
            pending,
        )))?;
    } else {
        emit_message(GuestPcTracePendingSegmentMessage::SegmentStarted(Box::new(
            pending,
        )))?;
        emit_message(GuestPcTracePendingSegmentMessage::SegmentFinished(
            Box::new(GuestPcTracePendingSegmentFinish {
                trace_instance_index,
            }),
        ))?;
    }

    Ok(GuestPcTraceLivePendingSegmentEmission {
        executed_instruction_count: slice.executed_instructions,
        trace_row_count: slice.trace_rows,
        report_count: slice.report_count,
        stream_start_count,
        report_chunk_count,
        status,
        last_report: slice.last_report,
        lookahead_instruction,
        terminal_pc,
        is_last_segment,
        needs_terminal_segment,
    })
}

fn guest_pc_trace_live_report_chunks_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS", false)
}

fn guest_pc_trace_live_stream_start_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_LIVE_STREAM_START", false)
}

fn validate_guest_pc_trace_live_report_chunk_mode() -> Result<(), GuestPcTraceBackendError> {
    if guest_pc_trace_segment_replay_enabled()
        || guest_pc_trace_segment_replay_snapshot_enabled()
        || guest_pc_trace_parallel_lower_report_elision_enabled()
    {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "live guest PC report chunks do not support replay or report elision"
                .to_owned(),
        });
    }
    if guest_pc_trace_seed_mirror_enabled() {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "live guest PC report chunks require runner seed snapshots for seeds"
                .to_owned(),
        });
    }
    if guest_pc_trace_runner_seed_snapshot_enabled()
        && !guest_pc_trace_runner_seed_snapshot_trusted_enabled()
    {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "live guest PC report chunks require trusted runner seed snapshots".to_owned(),
        });
    }
    Ok(())
}

fn produce_guest_pc_trace_live_pending_messages(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    input: &[u8],
    row_count: usize,
    report_chunk_capacity: usize,
    mut emit_message: impl FnMut(
        GuestPcTracePendingSegmentMessage,
    ) -> Result<(), GuestPcTraceBackendError>,
) -> Result<GuestPcTracePendingSliceProduction, GuestPcTraceBackendError> {
    validate_guest_pc_trace_live_report_chunk_mode()?;
    let layout = context
        .trace_layout
        .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
    let (mut memory, mut state, mut fcall_handler) = load_guest_pc_trace_machine(context, input)?;
    let mut executed_instructions = 0_u64;
    let mut trace_instance_count = 0_usize;
    let mut timing = GuestPcTraceStreamTiming::default();
    let runner_seed_snapshot = guest_pc_trace_runner_seed_snapshot_enabled();
    let runner_seed_snapshot_trusted =
        runner_seed_snapshot && guest_pc_trace_runner_seed_snapshot_trusted_enabled();
    let emit_stream_start_before_chunks = guest_pc_trace_live_stream_start_enabled();
    let validate_runner_seed_snapshot =
        runner_seed_snapshot && guest_pc_trace_runner_seed_snapshot_validation_enabled();
    let mut seed_mirror = runner_seed_snapshot.then(ZiskMainSegmentSeed::new);
    loop {
        let remaining_limit = instruction_limit.saturating_sub(executed_instructions);
        let trace_instance_index = u32::try_from(trace_instance_count).map_err(|_| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main trace instance index is too large".to_owned(),
            }
        })?;
        let seed = seed_mirror.clone();
        let replay_snapshot = (runner_seed_snapshot && seed.is_some())
            .then(|| GuestPcTraceSegmentReplaySnapshot::capture(&memory, &state, &fcall_handler));
        let mut runner_boundary_snapshot = if runner_seed_snapshot {
            let seed =
                seed.as_ref()
                    .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace live runner seed snapshot missing current seed"
                            .to_owned(),
                    })?;
            Some(ZiskMainRunnerBoundarySnapshot::new(seed))
        } else {
            None
        };
        let emitted = emit_guest_pc_trace_live_pending_segment_messages(
            &mut memory,
            &mut state,
            &mut fcall_handler,
            trace_instance_index,
            remaining_limit,
            row_count,
            seed.clone().map(Box::new),
            replay_snapshot.as_ref(),
            runner_boundary_snapshot.as_mut(),
            emit_stream_start_before_chunks,
            report_chunk_capacity,
            &mut emit_message,
        )?;
        executed_instructions = executed_instructions
            .checked_add(emitted.executed_instruction_count)
            .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest instruction count overflow".to_owned(),
            })?;
        timing.trace_report_chunk_sent_count = timing
            .trace_report_chunk_sent_count
            .saturating_add(emitted.report_chunk_count);
        timing.trace_stream_start_sent_count = timing
            .trace_stream_start_sent_count
            .saturating_add(emitted.stream_start_count);
        timing.trace_report_chunk_report_count = timing
            .trace_report_chunk_report_count
            .saturating_add(emitted.report_count);
        timing.trace_report_chunk_row_count = timing
            .trace_report_chunk_row_count
            .saturating_add(emitted.trace_row_count);
        let segment = ZiskMainTraceSegmentInfo {
            trace_instance_index,
            is_last_segment: emitted.is_last_segment,
            previous_c: seed
                .as_ref()
                .map(|seed| seed.previous_c)
                .unwrap_or_default(),
        };
        let runner_direct_next_seed = match (runner_seed_snapshot, segment.is_last_segment) {
            (true, false) => {
                let current_seed = seed.as_ref().ok_or_else(|| {
                    GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace live runner seed snapshot missing current seed"
                            .to_owned(),
                    }
                })?;
                let boundary_snapshot = runner_boundary_snapshot.as_ref().ok_or_else(|| {
                    GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace live runner boundary snapshot missing".to_owned(),
                    }
                })?;
                let direct_lift_started = Instant::now();
                timing.seed_direct_lift_attempt_count += 1;
                let next_seed = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
                    row_count,
                    segment,
                    ZiskMainRunnerBoundarySeedInput {
                        reports: &[],
                        report_count: emitted.report_count,
                        last_report: emitted.last_report.as_ref(),
                        lookahead_instruction: emitted.lookahead_instruction,
                        runner_state: &state,
                        current_seed,
                        boundary_snapshot,
                    },
                )?;
                timing.seed_direct_lift_duration += direct_lift_started.elapsed();
                match next_seed {
                    Ok(next_seed) => {
                        timing.seed_direct_lift_success_count += 1;
                        Some(next_seed)
                    }
                    Err(reason) => {
                        timing.record_seed_direct_lift_miss(reason);
                        None
                    }
                }
            }
            _ => None,
        };
        let needs_full_seed_advance = guest_pc_trace_needs_full_seed_advance(
            seed.is_some(),
            runner_seed_snapshot_trusted,
            validate_runner_seed_snapshot,
            segment.is_last_segment,
            runner_direct_next_seed.is_some(),
        );
        let full_next_seed = if needs_full_seed_advance {
            let seed =
                seed.as_ref()
                    .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace live seed advancement missing current seed"
                            .to_owned(),
                    })?;
            let replay_snapshot = replay_snapshot.clone().ok_or_else(|| {
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace live seed advancement replay snapshot missing"
                        .to_owned(),
                }
            })?;
            let full_advance_started = Instant::now();
            timing.seed_full_advance_count += 1;
            let replay = replay_guest_pc_trace_segment_from_snapshot(
                replay_snapshot,
                remaining_limit,
                row_count,
            )?;
            if replay.slice.executed_instructions != emitted.executed_instruction_count
                || replay.slice.trace_rows != emitted.trace_row_count
                || replay.slice.status != emitted.status
                || replay.slice.report_count != emitted.report_count
                || replay.memory != memory
                || replay.state != state
                || !replay.fcall_handler.equals_any(&fcall_handler)
            {
                return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace live seed validation replay diverged".to_owned(),
                });
            }
            let next_seed = advance_zisk_main_segment_seed(
                layout,
                &replay.slice.reports,
                emitted.terminal_pc,
                seed,
                emitted.lookahead_instruction,
                segment,
            )?
            .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
            timing.seed_full_advance_duration += full_advance_started.elapsed();
            Some(next_seed)
        } else {
            None
        };
        let next_seed = if runner_seed_snapshot_trusted && !segment.is_last_segment {
            runner_direct_next_seed.clone().or(full_next_seed.clone())
        } else {
            full_next_seed.clone()
        };
        if runner_seed_snapshot && !segment.is_last_segment && full_next_seed.is_some() {
            let current_seed =
                seed.as_ref()
                    .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace live runner seed snapshot missing current seed"
                            .to_owned(),
                    })?;
            let expected_next_seed = full_next_seed.as_ref().ok_or_else(|| {
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace live runner seed snapshot missing mirror seed"
                        .to_owned(),
                }
            })?;
            let runner_next_seed = match runner_direct_next_seed {
                Some(seed) => seed,
                None => {
                    let boundary_snapshot = runner_boundary_snapshot.as_ref().ok_or_else(|| {
                        GuestPcTraceBackendError::InvalidPcTraceLayout {
                            message: "guest PC trace live runner boundary snapshot missing"
                                .to_owned(),
                        }
                    })?;
                    lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
                        row_count,
                        segment,
                        ZiskMainRunnerBoundarySeedInput {
                            reports: &[],
                            report_count: emitted.report_count,
                            last_report: emitted.last_report.as_ref(),
                            lookahead_instruction: emitted.lookahead_instruction,
                            runner_state: &state,
                            current_seed,
                            boundary_snapshot,
                        },
                        expected_next_seed.previous_c,
                    )?
                }
            };
            if runner_next_seed != *expected_next_seed {
                return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: format!(
                        "guest PC trace live runner seed snapshot mismatch after segment {trace_instance_index}"
                    ),
                });
            }
        }
        if runner_seed_snapshot && !segment.is_last_segment && next_seed.is_none() {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "guest PC trace live runner seed snapshot missed segment {trace_instance_index}"
                ),
            });
        }
        if let Some(next_seed) = next_seed {
            seed_mirror = Some(next_seed);
        }
        trace_instance_count = trace_instance_count.checked_add(1).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main trace instance count overflow".to_owned(),
            }
        })?;
        if emitted.is_last_segment {
            break;
        }
        if emitted.needs_terminal_segment {
            continue;
        }
        if executed_instructions == instruction_limit {
            return Err(GuestPcTraceBackendError::GuestRun(
                GuestMachineRunError::InstructionLimitExceeded {
                    instruction_limit,
                    pc: emitted.terminal_pc,
                },
            ));
        }
    }

    Ok(GuestPcTracePendingSliceProduction {
        proof_values: zisk_runtime_proof_values(
            executed_instructions != 0,
            fcall_handler.input_data_was_mapped(),
            state.dma_proof_value_flags(),
        ),
        timing,
    })
}

fn produce_guest_pc_trace_pending_slices(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    input: &[u8],
    row_count: usize,
    mut emit: impl FnMut(GuestPcTracePendingSegmentSlice) -> Result<(), GuestPcTraceBackendError>,
) -> Result<GuestPcTracePendingSliceProduction, GuestPcTraceBackendError> {
    let layout = context
        .trace_layout
        .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
    let (mut memory, mut state, mut fcall_handler) = load_guest_pc_trace_machine(context, input)?;
    let mut executed_instructions = 0_u64;
    let mut trace_instance_count = 0_usize;
    let mut timing = GuestPcTraceStreamTiming::default();
    let runner_seed_snapshot = guest_pc_trace_runner_seed_snapshot_enabled();
    let segment_replay = guest_pc_trace_segment_replay_enabled();
    let report_elision = guest_pc_trace_parallel_lower_report_elision_enabled()
        && guest_pc_trace_parallel_lower_worker_count().is_some_and(|count| count > 1);
    let carry_replay_snapshot = guest_pc_trace_segment_replay_snapshot_enabled() || report_elision;
    let mut seed_mirror = (guest_pc_trace_seed_mirror_enabled() || runner_seed_snapshot)
        .then(ZiskMainSegmentSeed::new);
    loop {
        let remaining_limit = instruction_limit.saturating_sub(executed_instructions);
        let replay_snapshot = (segment_replay || carry_replay_snapshot)
            .then(|| GuestPcTraceSegmentReplaySnapshot::capture(&memory, &state, &fcall_handler));
        let mut runner_boundary_snapshot = if runner_seed_snapshot {
            let seed = seed_mirror.as_ref().ok_or_else(|| {
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace runner seed snapshot missing current seed".to_owned(),
                }
            })?;
            Some(ZiskMainRunnerBoundarySnapshot::new(seed))
        } else {
            None
        };
        let slice = if let Some(snapshot) = runner_boundary_snapshot.as_mut() {
            if report_elision {
                run_guest_pc_trace_segment_slice_with_elided_reports_and_boundary_snapshot(
                    &mut memory,
                    &mut state,
                    &mut fcall_handler,
                    remaining_limit,
                    row_count,
                    snapshot,
                )?
            } else {
                run_guest_pc_trace_segment_slice_with_boundary_snapshot(
                    &mut memory,
                    &mut state,
                    &mut fcall_handler,
                    remaining_limit,
                    row_count,
                    snapshot,
                )?
            }
        } else {
            run_guest_pc_trace_segment_slice(
                &mut memory,
                &mut state,
                &mut fcall_handler,
                remaining_limit,
                row_count,
            )?
        };
        timing.trace_runner_report_buffer_capacity += slice.report_capacity;
        timing.trace_runner_report_buffer_max_capacity = timing
            .trace_runner_report_buffer_max_capacity
            .max(slice.report_capacity);
        timing.trace_runner_report_buffer_excess_capacity +=
            slice.report_capacity.saturating_sub(slice.reports.len());
        if segment_replay {
            let replay_snapshot = replay_snapshot.clone().ok_or_else(|| {
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace segment replay snapshot missing".to_owned(),
                }
            })?;
            let replay = replay_guest_pc_trace_segment_from_snapshot(
                replay_snapshot,
                remaining_limit,
                row_count,
            )?;
            if replay.slice.executed_instructions != slice.executed_instructions
                || replay.slice.trace_rows != slice.trace_rows
                || replay.slice.status != slice.status
                || replay.slice.report_count != slice.report_count
                || (!slice.reports.is_empty() && replay.slice.reports != slice.reports)
                || replay.memory != memory
                || replay.state != state
                || !replay.fcall_handler.equals_any(&fcall_handler)
            {
                return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace segment replay diverged from serial runner".to_owned(),
                });
            }
            timing.segment_replay_count += 1;
        }
        executed_instructions = executed_instructions
            .checked_add(slice.executed_instructions)
            .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest instruction count overflow".to_owned(),
            })?;
        let (halted, terminal_pc, lookahead_instruction) = match &slice.status {
            GuestMachineTraceSliceStatus::Halted(halt) => (true, guest_machine_halt_pc(halt), None),
            GuestMachineTraceSliceStatus::Paused { pc, instruction } => {
                (false, *pc, Some(*instruction))
            }
        };
        let needs_terminal_segment = halted && slice.trace_rows == row_count;
        let is_last_segment = halted && !needs_terminal_segment;
        if !is_last_segment && slice.trace_rows < row_count {
            return Err(GuestPcTraceBackendError::GuestRun(
                GuestMachineRunError::InstructionLimitExceeded {
                    instruction_limit,
                    pc: terminal_pc,
                },
            ));
        }
        let trace_instance_index = u32::try_from(trace_instance_count).map_err(|_| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main trace instance index is too large".to_owned(),
            }
        })?;
        let segment = ZiskMainTraceSegmentInfo {
            trace_instance_index,
            is_last_segment,
            previous_c: seed_mirror
                .as_ref()
                .map(|seed| seed.previous_c)
                .unwrap_or_default(),
        };
        let seed = seed_mirror.clone();
        let runner_seed_snapshot_trusted =
            runner_seed_snapshot && guest_pc_trace_runner_seed_snapshot_trusted_enabled();
        let validate_runner_seed_snapshot =
            runner_seed_snapshot && guest_pc_trace_runner_seed_snapshot_validation_enabled();
        let runner_direct_next_seed = match (runner_seed_snapshot, segment.is_last_segment) {
            (true, false) => {
                let current_seed = seed.as_ref().ok_or_else(|| {
                    GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace runner seed snapshot missing current seed"
                            .to_owned(),
                    }
                })?;
                let boundary_snapshot = runner_boundary_snapshot.as_ref().ok_or_else(|| {
                    GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace runner boundary snapshot missing".to_owned(),
                    }
                })?;
                let direct_lift_started = Instant::now();
                timing.seed_direct_lift_attempt_count += 1;
                let next_seed = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
                    row_count,
                    segment,
                    ZiskMainRunnerBoundarySeedInput {
                        reports: &slice.reports,
                        report_count: slice.report_count,
                        last_report: slice.last_report.as_ref(),
                        lookahead_instruction,
                        runner_state: &state,
                        current_seed,
                        boundary_snapshot,
                    },
                )?;
                timing.seed_direct_lift_duration += direct_lift_started.elapsed();
                match next_seed {
                    Ok(next_seed) => {
                        timing.seed_direct_lift_success_count += 1;
                        Some(next_seed)
                    }
                    Err(reason) => {
                        timing.record_seed_direct_lift_miss(reason);
                        None
                    }
                }
            }
            _ => None,
        };
        let needs_full_seed_advance = guest_pc_trace_needs_full_seed_advance(
            seed.is_some(),
            runner_seed_snapshot_trusted,
            validate_runner_seed_snapshot,
            segment.is_last_segment,
            runner_direct_next_seed.is_some(),
        );
        let full_next_seed = if needs_full_seed_advance {
            let seed =
                seed.as_ref()
                    .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace seed advancement missing current seed".to_owned(),
                    })?;
            let full_advance_started = Instant::now();
            timing.seed_full_advance_count += 1;
            let replayed_reports;
            let seed_reports = if slice.reports.len() == slice.report_count {
                slice.reports.as_slice()
            } else {
                let replay_snapshot = replay_snapshot.clone().ok_or_else(|| {
                    GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace seed validation replay snapshot missing"
                            .to_owned(),
                    }
                })?;
                replayed_reports = replay_guest_pc_trace_segment_reports_for_seed_advance(
                    replay_snapshot,
                    remaining_limit,
                    row_count,
                    &slice,
                    trace_instance_index,
                )?;
                replayed_reports.as_slice()
            };
            let next_seed = advance_zisk_main_segment_seed(
                layout,
                seed_reports,
                terminal_pc,
                seed,
                lookahead_instruction,
                segment,
            )?
            .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
            timing.seed_full_advance_duration += full_advance_started.elapsed();
            Some(next_seed)
        } else {
            None
        };
        let next_seed = if runner_seed_snapshot_trusted && !segment.is_last_segment {
            runner_direct_next_seed.clone().or(full_next_seed.clone())
        } else {
            full_next_seed.clone()
        };
        if runner_seed_snapshot && !segment.is_last_segment && full_next_seed.is_some() {
            let current_seed =
                seed.as_ref()
                    .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace runner seed snapshot missing current seed"
                            .to_owned(),
                    })?;
            let expected_next_seed = full_next_seed.as_ref().ok_or_else(|| {
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace runner seed snapshot missing mirror seed".to_owned(),
                }
            })?;
            let runner_next_seed = match runner_direct_next_seed {
                Some(seed) => seed,
                None => {
                    let boundary_snapshot = runner_boundary_snapshot.as_ref().ok_or_else(|| {
                        GuestPcTraceBackendError::InvalidPcTraceLayout {
                            message: "guest PC trace runner boundary snapshot missing".to_owned(),
                        }
                    })?;
                    lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
                        row_count,
                        segment,
                        ZiskMainRunnerBoundarySeedInput {
                            reports: &slice.reports,
                            report_count: slice.report_count,
                            last_report: slice.last_report.as_ref(),
                            lookahead_instruction,
                            runner_state: &state,
                            current_seed,
                            boundary_snapshot,
                        },
                        expected_next_seed.previous_c,
                    )?
                }
            };
            if runner_next_seed != *expected_next_seed {
                return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: format!(
                        "guest PC trace runner seed snapshot mismatch after segment {trace_instance_index}"
                    ),
                });
            }
        }
        let report_count = slice.report_count;
        let report_capacity = slice.report_capacity;
        let reports_elided = report_elision;
        let reports = if reports_elided {
            timing.parallel_lower_report_elided_count =
                timing.parallel_lower_report_elided_count.saturating_add(1);
            Vec::new()
        } else {
            slice.reports
        };
        emit(GuestPcTracePendingSegmentSlice {
            trace_instance_index,
            executed_instruction_count: slice.executed_instructions,
            trace_row_count: slice.trace_rows,
            runner_remaining_instruction_limit: remaining_limit,
            report_count,
            report_capacity,
            reports,
            reports_elided,
            terminal_pc,
            lookahead_instruction,
            is_last_segment,
            seed: seed.map(Box::new),
            replay_snapshot: carry_replay_snapshot.then_some(replay_snapshot).flatten(),
        })?;
        if let Some(next_seed) = next_seed {
            seed_mirror = Some(next_seed);
        }
        trace_instance_count = trace_instance_count.checked_add(1).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main trace instance count overflow".to_owned(),
            }
        })?;
        if is_last_segment {
            break;
        }
        if needs_terminal_segment {
            continue;
        }
        if executed_instructions == instruction_limit {
            return Err(GuestPcTraceBackendError::GuestRun(
                GuestMachineRunError::InstructionLimitExceeded {
                    instruction_limit,
                    pc: terminal_pc,
                },
            ));
        }
    }

    Ok(GuestPcTracePendingSliceProduction {
        proof_values: zisk_runtime_proof_values(
            executed_instructions != 0,
            fcall_handler.input_data_was_mapped(),
            state.dma_proof_value_flags(),
        ),
        timing,
    })
}

struct GuestPcTraceLoweredSegment {
    segment: GuestPcTraceSegmentTrace,
    next_seed: ZiskMainSegmentSeed,
}

struct GuestPcTraceSeededLoweredSegment {
    seed: ZiskMainSegmentSeed,
    lowered: GuestPcTraceLoweredSegment,
}

fn lower_guest_pc_trace_seeded_pending_segment(
    layout: &WitnessTraceLayout,
    pending: &GuestPcTracePendingSegmentSlice,
    seed: &ZiskMainSegmentSeed,
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<GuestPcTraceLoweredSegment, GuestPcTraceBackendError> {
    let lower_started = Instant::now();
    let written = build_layout_zisk_main_trace_segment_for_segment_output(
        layout,
        &pending.reports,
        pending.terminal_pc,
        &seed.initial_state,
        pending.lookahead_instruction,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: pending.trace_instance_index,
            is_last_segment: pending.is_last_segment,
            previous_c: seed.previous_c,
        },
        timing.as_deref_mut(),
    )?
    .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
    if let Some(timing) = timing {
        timing.trace_lower_duration += lower_started.elapsed();
    }
    let next_seed = ZiskMainSegmentSeed {
        initial_state: written.continuation_state,
        previous_c: written.final_state.last_c,
    };
    Ok(GuestPcTraceLoweredSegment {
        segment: GuestPcTraceSegmentTrace {
            trace_instance_index: pending.trace_instance_index,
            trace_source_prefix_rows: written.trace_source_prefix_rows,
            #[cfg(feature = "cuda")]
            device_segment_material: written.device_segment_material,
            trace: written.trace,
            unit_values: written.output.unit_values,
            proof_values: expected_proof_values.unwrap_or_default().to_vec(),
        },
        next_seed,
    })
}

#[cfg(feature = "cuda")]
fn lower_guest_pc_trace_owned_streaming_pending_segment(
    layout: &WitnessTraceLayout,
    mut pending: GuestPcTracePendingSegmentSlice,
    seed: &ZiskMainSegmentSeed,
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<GuestPcTraceLoweredSegment, GuestPcTraceBackendError> {
    if pending.reports.len() > layout.row_count() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len: layout_trace_byte_len(pending.reports.len(), layout.column_count()),
            output_len: layout_trace_byte_len(layout.row_count(), layout.column_count()),
        });
    }

    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: pending.trace_instance_index,
        is_last_segment: pending.is_last_segment,
        previous_c: seed.previous_c,
    };
    let Some(mut builder) =
        ZiskMainStreamingDeviceSegmentBuilder::new(layout, &seed.initial_state, segment)?
    else {
        return lower_guest_pc_trace_seeded_pending_segment(
            layout,
            &pending,
            seed,
            expected_proof_values,
            timing,
        );
    };

    let lower_started = Instant::now();
    let timing_config = ZiskMainTraceLowerTimingConfig::from_env();
    let mut feeder = ZiskMainOwnedStreamingDeviceReportFeeder::new(timing_config);
    let aggregate_report_started = timing.as_ref().map(|_| Instant::now());
    for report in std::mem::take(&mut pending.reports) {
        feeder.push_report(&mut builder, report, timing.as_deref_mut())?;
    }
    feeder.finish(
        &mut builder,
        pending.lookahead_instruction,
        timing.as_deref_mut(),
    )?;
    record_aggregate_trace_report_duration(&mut timing, aggregate_report_started);
    let GuestPcTraceDeviceSegmentBuild {
        device_segment_material,
        unit_values,
        final_state,
        continuation_state,
    } = builder.finish(pending.terminal_pc, timing.as_deref_mut())?;
    if let Some(timing) = timing {
        timing.trace_lower_duration += lower_started.elapsed();
    }
    let next_seed = ZiskMainSegmentSeed {
        initial_state: continuation_state,
        previous_c: final_state.last_c,
    };
    Ok(GuestPcTraceLoweredSegment {
        segment: GuestPcTraceSegmentTrace {
            trace_instance_index: pending.trace_instance_index,
            trace_source_prefix_rows: device_segment_material.trace_source_prefix_rows,
            device_segment_material: Some(device_segment_material),
            trace: None,
            unit_values,
            proof_values: expected_proof_values.unwrap_or_default().to_vec(),
        },
        next_seed,
    })
}

#[cfg(test)]
fn lower_guest_pc_trace_seeded_pending_segments_with_workers(
    layout: &WitnessTraceLayout,
    pending: Vec<GuestPcTracePendingSegmentSlice>,
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    worker_count: usize,
) -> Result<Vec<GuestPcTraceLoweredSegment>, GuestPcTraceBackendError> {
    lower_guest_pc_trace_seeded_pending_segments_with_timing(
        layout,
        pending,
        expected_proof_values,
        worker_count,
        None,
    )
}

#[cfg(test)]
fn lower_guest_pc_trace_seeded_pending_segments_with_timing(
    layout: &WitnessTraceLayout,
    pending: Vec<GuestPcTracePendingSegmentSlice>,
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    worker_count: usize,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<Vec<GuestPcTraceLoweredSegment>, GuestPcTraceBackendError> {
    if pending.is_empty() {
        return Ok(Vec::new());
    }

    let worker_count = worker_count.max(1).min(pending.len());
    let mut lowered = Vec::with_capacity(pending.len());
    if worker_count == 1 {
        for pending in &pending {
            let seed = pending.seed.as_deref().ok_or_else(|| {
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: format!(
                        "parallel guest PC trace lower requires seed for segment {}",
                        pending.trace_instance_index
                    ),
                }
            })?;
            lowered.push(GuestPcTraceSeededLoweredSegment {
                seed: seed.clone(),
                lowered: lower_guest_pc_trace_seeded_pending_segment(
                    layout,
                    pending,
                    seed,
                    expected_proof_values,
                    timing.as_deref_mut(),
                )?,
            });
        }
    } else {
        let chunk_size = pending.len().div_ceil(worker_count);
        thread::scope(|scope| {
            let mut handles = Vec::new();
            for chunk in pending.chunks(chunk_size) {
                handles.push(scope.spawn(move || {
                    let mut chunk_timing = GuestPcTraceStreamTiming::default();
                    let mut chunk_out = Vec::with_capacity(chunk.len());
                    for pending in chunk {
                        let seed = pending.seed.as_deref().ok_or_else(|| {
                            GuestPcTraceBackendError::InvalidPcTraceLayout {
                                message: format!(
                                    "parallel guest PC trace lower requires seed for segment {}",
                                    pending.trace_instance_index
                                ),
                            }
                        })?;
                        chunk_out.push(GuestPcTraceSeededLoweredSegment {
                            seed: seed.clone(),
                            lowered: lower_guest_pc_trace_seeded_pending_segment(
                                layout,
                                pending,
                                seed,
                                expected_proof_values,
                                Some(&mut chunk_timing),
                            )?,
                        });
                    }
                    Ok::<_, GuestPcTraceBackendError>((chunk_out, chunk_timing))
                }));
            }

            for handle in handles {
                let (chunk, chunk_timing) = handle.join().map_err(|_| {
                    GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "parallel guest PC trace lower worker panicked".to_owned(),
                    }
                })??;
                if let Some(timing) = &mut timing {
                    (**timing).add(chunk_timing);
                }
                lowered.extend(chunk);
            }
            Ok::<(), GuestPcTraceBackendError>(())
        })?;
    }

    lowered.sort_by_key(|entry| entry.lowered.segment.trace_instance_index);
    let mut current_seed = ZiskMainSegmentSeed::new();
    for entry in &lowered {
        validate_guest_pc_trace_pending_segment_seed(
            entry.lowered.segment.trace_instance_index,
            Some(&entry.seed),
            &current_seed.initial_state,
            current_seed.previous_c,
        )?;
        current_seed = entry.lowered.next_seed.clone();
    }
    Ok(lowered.into_iter().map(|entry| entry.lowered).collect())
}

fn receive_guest_pc_trace_pending_report_chunk(
    chunk: GuestPcTracePendingReportChunk,
    pending_chunks: &mut BTreeMap<u32, GuestPcTracePendingReportChunkGroup>,
    timing: &mut GuestPcTraceStreamTiming,
) {
    timing.trace_report_chunk_received_count =
        timing.trace_report_chunk_received_count.saturating_add(1);
    let group = pending_chunks
        .entry(chunk.trace_instance_index)
        .or_default();
    group.chunk_count = group.chunk_count.saturating_add(1);
    group.reports.extend(chunk.reports);
    let queued_chunks = pending_chunks
        .values()
        .map(|group| group.chunk_count)
        .sum::<usize>();
    timing.trace_report_chunk_max_queued_count = timing
        .trace_report_chunk_max_queued_count
        .max(queued_chunks);
}

#[cfg(feature = "cuda")]
struct GuestPcTraceActiveChunkedSegment {
    pending: GuestPcTracePendingSegmentSlice,
    seed: ZiskMainSegmentSeed,
    builder: ZiskMainStreamingDeviceSegmentBuilder,
    feeder: ZiskMainOwnedStreamingDeviceReportFeeder,
    aggregate_report_started: Option<Instant>,
}

#[cfg(feature = "cuda")]
impl GuestPcTraceActiveChunkedSegment {
    fn new(
        layout: &WitnessTraceLayout,
        pending: GuestPcTracePendingSegmentSlice,
        seed: ZiskMainSegmentSeed,
        timing_enabled: bool,
    ) -> Result<Result<Self, GuestPcTracePendingSegmentSlice>, GuestPcTraceBackendError> {
        let segment = ZiskMainTraceSegmentInfo {
            trace_instance_index: pending.trace_instance_index,
            is_last_segment: pending.is_last_segment,
            previous_c: seed.previous_c,
        };
        let Some(builder) =
            ZiskMainStreamingDeviceSegmentBuilder::new(layout, &seed.initial_state, segment)?
        else {
            return Ok(Err(pending));
        };
        Ok(Ok(Self {
            pending,
            seed,
            builder,
            feeder: ZiskMainOwnedStreamingDeviceReportFeeder::new(
                ZiskMainTraceLowerTimingConfig::from_env(),
            ),
            aggregate_report_started: timing_enabled.then(Instant::now),
        }))
    }

    fn trace_instance_index(&self) -> u32 {
        self.pending.trace_instance_index
    }

    fn push_chunk(
        &mut self,
        chunk: GuestPcTracePendingReportChunk,
        timing: &mut GuestPcTraceStreamTiming,
    ) -> Result<(), GuestPcTraceBackendError> {
        if chunk.trace_instance_index != self.pending.trace_instance_index {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "guest PC trace report chunk for segment {} reached active segment {}",
                    chunk.trace_instance_index, self.pending.trace_instance_index
                ),
            });
        }
        timing.trace_report_chunk_received_count =
            timing.trace_report_chunk_received_count.saturating_add(1);
        timing.trace_report_chunk_max_queued_count =
            timing.trace_report_chunk_max_queued_count.max(1);
        for report in chunk.reports {
            self.feeder
                .push_report(&mut self.builder, report, Some(timing))?;
        }
        Ok(())
    }

    fn finish(
        mut self,
        finish: GuestPcTracePendingSegmentFinish,
        expected_proof_values: Option<&[WitnessTraceProofValue]>,
        timing: &mut GuestPcTraceStreamTiming,
    ) -> Result<GuestPcTraceSeededLoweredSegment, GuestPcTraceBackendError> {
        if finish.trace_instance_index != self.pending.trace_instance_index {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "guest PC trace segment finish {} reached active segment {}",
                    finish.trace_instance_index, self.pending.trace_instance_index
                ),
            });
        }
        self.feeder.finish(
            &mut self.builder,
            self.pending.lookahead_instruction,
            Some(timing),
        )?;
        let mut aggregate_timing = Some(&mut *timing);
        record_aggregate_trace_report_duration(
            &mut aggregate_timing,
            self.aggregate_report_started,
        );
        let GuestPcTraceDeviceSegmentBuild {
            device_segment_material,
            unit_values,
            final_state,
            continuation_state,
        } = self
            .builder
            .finish(self.pending.terminal_pc, Some(timing))?;
        let next_seed = ZiskMainSegmentSeed {
            initial_state: continuation_state.clone(),
            previous_c: final_state.last_c,
        };
        Ok(GuestPcTraceSeededLoweredSegment {
            seed: self.seed,
            lowered: GuestPcTraceLoweredSegment {
                segment: GuestPcTraceSegmentTrace {
                    trace_instance_index: self.pending.trace_instance_index,
                    trace_source_prefix_rows: device_segment_material.trace_source_prefix_rows,
                    device_segment_material: Some(device_segment_material),
                    trace: None,
                    unit_values,
                    proof_values: expected_proof_values.unwrap_or_default().to_vec(),
                },
                next_seed,
            },
        })
    }
}

fn finish_guest_pc_trace_chunked_pending_segment(
    mut pending: GuestPcTracePendingSegmentSlice,
    pending_chunks: &mut BTreeMap<u32, GuestPcTracePendingReportChunkGroup>,
) -> Result<GuestPcTracePendingSegmentSlice, GuestPcTraceBackendError> {
    let Some(group) = pending_chunks.remove(&pending.trace_instance_index) else {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "guest PC trace report chunks missing for segment {}",
                pending.trace_instance_index
            ),
        });
    };
    if !pending.reports.is_empty() {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "guest PC trace report chunks overlap carried reports for segment {}",
                pending.trace_instance_index
            ),
        });
    }
    pending.reports = group.reports;
    if pending.reports.len() != pending.report_count {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "guest PC trace report chunks lost reports for segment {}",
                pending.trace_instance_index
            ),
        });
    }
    Ok(pending)
}

fn validate_guest_pc_trace_no_pending_report_chunks(
    pending_chunks: &BTreeMap<u32, GuestPcTracePendingReportChunkGroup>,
) -> Result<(), GuestPcTraceBackendError> {
    if pending_chunks.is_empty() {
        return Ok(());
    }
    Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "guest PC trace report chunks missing segment finish".to_owned(),
    })
}

fn lower_guest_pc_trace_pending_segments(
    layout: &WitnessTraceLayout,
    pending_receiver: mpsc::Receiver<GuestPcTracePendingSegmentMessage>,
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    timing: &mut GuestPcTraceStreamTiming,
    emit: &mut impl FnMut(GuestPcTraceSegmentTrace) -> Result<(), GuestPcTraceBackendError>,
) -> Result<GuestPcTraceStreamResult, GuestPcTraceBackendError> {
    if let Some(worker_count) =
        guest_pc_trace_parallel_lower_worker_count().filter(|count| *count > 1)
    {
        return lower_guest_pc_trace_pending_segments_parallel(
            layout,
            pending_receiver,
            expected_proof_values,
            timing,
            emit,
            worker_count,
        );
    }

    let mut current_seed = ZiskMainSegmentSeed::new();
    let mut pending_chunks = BTreeMap::new();
    let mut pending_chunked_segments = BTreeMap::new();
    #[cfg(feature = "cuda")]
    let mut active_chunked_segment: Option<GuestPcTraceActiveChunkedSegment> = None;
    loop {
        let receive_started = Instant::now();
        let message = match pending_receiver.recv() {
            Ok(message) => message,
            Err(_) => break,
        };
        timing.pending_receive_wait_duration += receive_started.elapsed();
        let pending = match message {
            GuestPcTracePendingSegmentMessage::Segment(pending) => *pending,
            GuestPcTracePendingSegmentMessage::SegmentStreamStarted(_) => {
                continue;
            }
            GuestPcTracePendingSegmentMessage::SegmentStarted(pending) => {
                let pending = *pending;
                #[cfg(feature = "cuda")]
                let pending = {
                    if guest_pc_trace_less_segment_output_enabled()
                        && !pending_chunks.contains_key(&pending.trace_instance_index)
                    {
                        if active_chunked_segment.is_some() {
                            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                                message: "guest PC trace report chunks overlapped active segment"
                                    .to_owned(),
                            });
                        }
                        validate_guest_pc_trace_pending_segment_seed(
                            pending.trace_instance_index,
                            pending.seed.as_deref(),
                            &current_seed.initial_state,
                            current_seed.previous_c,
                        )?;
                        let segment_seed = pending.seed.as_deref().unwrap_or(&current_seed).clone();
                        match GuestPcTraceActiveChunkedSegment::new(
                            layout,
                            pending,
                            segment_seed,
                            true,
                        )? {
                            Ok(active) => {
                                active_chunked_segment = Some(active);
                                continue;
                            }
                            Err(returned_pending) => returned_pending,
                        }
                    } else {
                        pending
                    }
                };
                if pending_chunked_segments
                    .insert(pending.trace_instance_index, pending)
                    .is_some()
                {
                    return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace duplicate chunked segment start".to_owned(),
                    });
                }
                continue;
            }
            GuestPcTracePendingSegmentMessage::ReportChunk(chunk) => {
                #[cfg(feature = "cuda")]
                if let Some(active) = active_chunked_segment.as_mut() {
                    if active.trace_instance_index() != chunk.trace_instance_index {
                        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                            message: format!(
                                "guest PC trace report chunk for segment {} arrived while segment {} is active",
                                chunk.trace_instance_index,
                                active.trace_instance_index()
                            ),
                        });
                    }
                    active.push_chunk(*chunk, timing)?;
                    continue;
                }
                receive_guest_pc_trace_pending_report_chunk(*chunk, &mut pending_chunks, timing);
                continue;
            }
            GuestPcTracePendingSegmentMessage::SegmentFinished(finish) => {
                #[cfg(feature = "cuda")]
                if let Some(active) = active_chunked_segment.take() {
                    if active.trace_instance_index() != finish.trace_instance_index {
                        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                            message: format!(
                                "guest PC trace segment finish {} arrived while segment {} is active",
                                finish.trace_instance_index,
                                active.trace_instance_index()
                            ),
                        });
                    }
                    let lowered = active.finish(*finish, expected_proof_values, timing)?;
                    current_seed = lowered.lowered.next_seed.clone();
                    let emit_started = Instant::now();
                    emit(lowered.lowered.segment)?;
                    timing.trace_emit_duration += emit_started.elapsed();
                    continue;
                }
                let pending = pending_chunked_segments
                    .remove(&finish.trace_instance_index)
                    .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: format!(
                            "guest PC trace chunked segment {} finished before start",
                            finish.trace_instance_index
                        ),
                    })?;
                finish_guest_pc_trace_chunked_pending_segment(pending, &mut pending_chunks)?
            }
            GuestPcTracePendingSegmentMessage::Complete(stream) => {
                validate_guest_pc_trace_no_pending_report_chunks(&pending_chunks)?;
                if !pending_chunked_segments.is_empty() {
                    return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace chunked segment missing finish".to_owned(),
                    });
                }
                #[cfg(feature = "cuda")]
                if active_chunked_segment.is_some() {
                    return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "guest PC trace active chunked segment missing finish".to_owned(),
                    });
                }
                return Ok(*stream);
            }
            GuestPcTracePendingSegmentMessage::Error(error) => return Err(error),
        };
        timing.trace_report_buffer_capacity += pending.report_capacity;
        timing.trace_report_buffer_max_capacity = timing
            .trace_report_buffer_max_capacity
            .max(pending.report_capacity);
        timing.trace_report_buffer_excess_capacity += pending
            .report_capacity
            .saturating_sub(pending.reports.len());
        validate_guest_pc_trace_pending_segment_seed(
            pending.trace_instance_index,
            pending.seed.as_deref(),
            &current_seed.initial_state,
            current_seed.previous_c,
        )?;
        let segment_seed = pending.seed.as_deref().unwrap_or(&current_seed);
        #[cfg(feature = "cuda")]
        let lowered = if guest_pc_trace_owned_streaming_lower_enabled() {
            let segment_seed = segment_seed.clone();
            lower_guest_pc_trace_owned_streaming_pending_segment(
                layout,
                pending,
                &segment_seed,
                expected_proof_values,
                Some(timing),
            )?
        } else {
            lower_guest_pc_trace_seeded_pending_segment(
                layout,
                &pending,
                segment_seed,
                expected_proof_values,
                Some(timing),
            )?
        };
        #[cfg(not(feature = "cuda"))]
        let lowered = lower_guest_pc_trace_seeded_pending_segment(
            layout,
            &pending,
            segment_seed,
            expected_proof_values,
            Some(timing),
        )?;
        current_seed = lowered.next_seed;
        let emit_started = Instant::now();
        emit(lowered.segment)?;
        timing.trace_emit_duration += emit_started.elapsed();
    }
    Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "guest PC trace pending segment runner stopped".to_owned(),
    })
}

enum GuestPcTraceParallelLowerMessage {
    Segment {
        trace_instance_index: u32,
        result: Box<Result<GuestPcTraceSeededLoweredSegment, GuestPcTraceBackendError>>,
        timing: GuestPcTraceStreamTiming,
    },
    Complete {
        stream: Box<GuestPcTraceStreamResult>,
        dispatched_count: usize,
        timing: GuestPcTraceStreamTiming,
    },
    Error {
        error: GuestPcTraceBackendError,
        dispatched_count: usize,
        timing: GuestPcTraceStreamTiming,
    },
}

fn send_guest_pc_trace_parallel_lower_segment_result(
    result_sender: &mpsc::SyncSender<GuestPcTraceParallelLowerMessage>,
    trace_instance_index: u32,
    result: Result<GuestPcTraceSeededLoweredSegment, GuestPcTraceBackendError>,
    timing: GuestPcTraceStreamTiming,
) -> bool {
    let send_started = Instant::now();
    let mut message = GuestPcTraceParallelLowerMessage::Segment {
        trace_instance_index,
        result: Box::new(result),
        timing,
    };
    loop {
        if let GuestPcTraceParallelLowerMessage::Segment { timing, .. } = &mut message {
            timing.parallel_lower_result_send_wait_duration = send_started.elapsed();
        }
        match result_sender.try_send(message) {
            Ok(()) => return true,
            Err(mpsc::TrySendError::Full(returned_message)) => {
                message = returned_message;
                std::thread::yield_now();
            }
            Err(mpsc::TrySendError::Disconnected(_)) => return false,
        }
    }
}

enum GuestPcTraceParallelLowerJob {
    Segment(Box<GuestPcTracePendingSegmentSlice>),
    #[cfg(feature = "cuda")]
    StreamStart(Box<GuestPcTracePendingSegmentStreamStart>),
    #[cfg(feature = "cuda")]
    StreamChunk(Box<GuestPcTracePendingReportChunk>),
    #[cfg(feature = "cuda")]
    StreamSegment(Box<GuestPcTracePendingSegmentSlice>),
    #[cfg(feature = "cuda")]
    StreamFinish(Box<GuestPcTracePendingSegmentFinish>),
}

trait GuestPcTraceParallelLowerDispatchClass {
    fn stream_dispatch_kind(&self) -> Option<u8>;
}

impl GuestPcTraceParallelLowerDispatchClass for GuestPcTraceParallelLowerJob {
    fn stream_dispatch_kind(&self) -> Option<u8> {
        match self {
            #[cfg(feature = "cuda")]
            GuestPcTraceParallelLowerJob::StreamStart(_) => Some(1),
            #[cfg(feature = "cuda")]
            GuestPcTraceParallelLowerJob::StreamChunk(_) => Some(2),
            #[cfg(feature = "cuda")]
            GuestPcTraceParallelLowerJob::StreamSegment(_) => Some(3),
            #[cfg(feature = "cuda")]
            GuestPcTraceParallelLowerJob::StreamFinish(_) => Some(4),
            GuestPcTraceParallelLowerJob::Segment(_) => None,
        }
    }
}

#[cfg(test)]
impl GuestPcTraceParallelLowerDispatchClass for u32 {
    fn stream_dispatch_kind(&self) -> Option<u8> {
        None
    }
}

fn record_guest_pc_trace_parallel_lower_dispatch_wait(
    timing: &mut GuestPcTraceStreamTiming,
    stream_kind: Option<u8>,
    elapsed: Duration,
) {
    timing.parallel_lower_dispatch_wait_duration += elapsed;
    match stream_kind {
        #[cfg(feature = "cuda")]
        Some(1) => {
            timing.parallel_lower_stream_start_dispatch_wait_duration += elapsed;
        }
        #[cfg(feature = "cuda")]
        Some(2) => {
            timing.parallel_lower_stream_chunk_dispatch_wait_duration += elapsed;
        }
        #[cfg(feature = "cuda")]
        Some(3) => {
            timing.parallel_lower_stream_segment_dispatch_wait_duration += elapsed;
        }
        #[cfg(feature = "cuda")]
        Some(4) => {
            timing.parallel_lower_stream_finish_dispatch_wait_duration += elapsed;
        }
        _ => {}
    }
}

fn replay_guest_pc_trace_pending_segment_reports(
    layout: &WitnessTraceLayout,
    pending: &mut GuestPcTracePendingSegmentSlice,
    timing: &mut GuestPcTraceStreamTiming,
) -> Result<(), GuestPcTraceBackendError> {
    let snapshot = pending.replay_snapshot.take().ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "parallel guest PC trace lower replay snapshot missing for segment {}",
                pending.trace_instance_index
            ),
        }
    })?;
    let replay_started = Instant::now();
    let replay = replay_guest_pc_trace_segment_from_snapshot(
        snapshot,
        pending.runner_remaining_instruction_limit,
        layout.row_count(),
    )?;
    timing.parallel_lower_snapshot_replay_duration += replay_started.elapsed();
    let (is_last_segment, terminal_pc, lookahead_instruction) = match replay.slice.status {
        GuestMachineTraceSliceStatus::Halted(halt) => (true, guest_machine_halt_pc(&halt), None),
        GuestMachineTraceSliceStatus::Paused { pc, instruction } => (false, pc, Some(instruction)),
    };
    if replay.slice.executed_instructions != pending.executed_instruction_count
        || replay.slice.trace_rows != pending.trace_row_count
        || replay.slice.report_count != pending.report_count
        || terminal_pc != pending.terminal_pc
        || lookahead_instruction != pending.lookahead_instruction
        || is_last_segment != pending.is_last_segment
        || (!pending.reports_elided && replay.slice.reports != pending.reports)
    {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "parallel guest PC trace lower replay diverged for segment {}",
                pending.trace_instance_index
            ),
        });
    }
    pending.reports = replay.slice.reports;
    timing.parallel_lower_snapshot_replay_count = timing
        .parallel_lower_snapshot_replay_count
        .saturating_add(1);
    Ok(())
}

fn lower_guest_pc_trace_parallel_pending_job(
    layout: &WitnessTraceLayout,
    mut pending: GuestPcTracePendingSegmentSlice,
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    timing: &mut GuestPcTraceStreamTiming,
) -> Result<GuestPcTraceSeededLoweredSegment, GuestPcTraceBackendError> {
    if guest_pc_trace_parallel_lower_replay_snapshot_enabled() {
        replay_guest_pc_trace_pending_segment_reports(layout, &mut pending, timing)?;
    }
    let trace_instance_index = pending.trace_instance_index;
    let seed =
        pending
            .seed
            .as_deref()
            .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                "parallel guest PC trace lower requires seed for segment {trace_instance_index}"
            ),
            })?;
    #[cfg(feature = "cuda")]
    if guest_pc_trace_owned_streaming_lower_enabled() {
        let seed = seed.clone();
        return Ok(GuestPcTraceSeededLoweredSegment {
            seed: seed.clone(),
            lowered: lower_guest_pc_trace_owned_streaming_pending_segment(
                layout,
                pending,
                &seed,
                expected_proof_values,
                Some(timing),
            )?,
        });
    }
    Ok(GuestPcTraceSeededLoweredSegment {
        seed: seed.clone(),
        lowered: lower_guest_pc_trace_seeded_pending_segment(
            layout,
            &pending,
            seed,
            expected_proof_values,
            Some(timing),
        )?,
    })
}

#[cfg(feature = "cuda")]
struct GuestPcTraceParallelStreamedSegment {
    trace_instance_index: u32,
    seed: ZiskMainSegmentSeed,
    builder: Option<ZiskMainStreamingDeviceSegmentBuilder>,
    feeder: ZiskMainOwnedStreamingDeviceReportFeeder,
    pending: Option<GuestPcTracePendingSegmentSlice>,
    reports: Option<Vec<GuestMachineReport>>,
    report_count: usize,
    aggregate_report_started: Option<Instant>,
}

#[cfg(feature = "cuda")]
impl GuestPcTraceParallelStreamedSegment {
    fn new(
        layout: &WitnessTraceLayout,
        start: GuestPcTracePendingSegmentStreamStart,
        timing_enabled: bool,
    ) -> Result<Self, GuestPcTraceBackendError> {
        let seed = start.seed.map(|seed| *seed).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "parallel guest PC trace stream start missing seed for segment {}",
                    start.trace_instance_index
                ),
            }
        })?;
        let segment = ZiskMainTraceSegmentInfo {
            trace_instance_index: start.trace_instance_index,
            is_last_segment: false,
            previous_c: seed.previous_c,
        };
        let builder =
            ZiskMainStreamingDeviceSegmentBuilder::new(layout, &seed.initial_state, segment)?;
        let retain_reports = builder.is_none();
        let aggregate_report_started = (timing_enabled && builder.is_some()).then(Instant::now);
        Ok(Self {
            trace_instance_index: start.trace_instance_index,
            seed,
            builder,
            feeder: ZiskMainOwnedStreamingDeviceReportFeeder::new(
                ZiskMainTraceLowerTimingConfig::from_env(),
            ),
            pending: None,
            reports: retain_reports.then(Vec::new),
            report_count: 0,
            aggregate_report_started,
        })
    }

    fn push_chunk(
        &mut self,
        chunk: GuestPcTracePendingReportChunk,
        timing: &mut GuestPcTraceStreamTiming,
    ) -> Result<(), GuestPcTraceBackendError> {
        if chunk.trace_instance_index != self.trace_instance_index {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "parallel guest PC trace stream chunk for segment {} reached segment {}",
                    chunk.trace_instance_index, self.trace_instance_index
                ),
            });
        }
        timing.trace_report_chunk_received_count =
            timing.trace_report_chunk_received_count.saturating_add(1);
        timing.trace_report_chunk_max_queued_count =
            timing.trace_report_chunk_max_queued_count.max(1);
        timing.parallel_lower_stream_chunk_count =
            timing.parallel_lower_stream_chunk_count.saturating_add(1);
        self.report_count = self.report_count.saturating_add(chunk.reports.len());
        for report in chunk.reports {
            if let Some(builder) = self.builder.as_mut() {
                self.feeder.push_report(builder, report, Some(timing))?;
            } else if let Some(reports) = self.reports.as_mut() {
                reports.push(report);
                timing.parallel_lower_stream_retained_report_count = timing
                    .parallel_lower_stream_retained_report_count
                    .saturating_add(1);
            }
        }
        Ok(())
    }

    fn set_pending(
        &mut self,
        pending: GuestPcTracePendingSegmentSlice,
    ) -> Result<(), GuestPcTraceBackendError> {
        if pending.trace_instance_index != self.trace_instance_index {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "parallel guest PC trace stream segment {} reached segment {}",
                    pending.trace_instance_index, self.trace_instance_index
                ),
            });
        }
        if self.pending.is_some() {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "parallel guest PC trace stream segment {} duplicated metadata",
                    self.trace_instance_index
                ),
            });
        }
        self.pending = Some(pending);
        Ok(())
    }

    fn finish(
        mut self,
        layout: &WitnessTraceLayout,
        finish: GuestPcTracePendingSegmentFinish,
        expected_proof_values: Option<&[WitnessTraceProofValue]>,
        timing: &mut GuestPcTraceStreamTiming,
    ) -> Result<GuestPcTraceSeededLoweredSegment, GuestPcTraceBackendError> {
        if finish.trace_instance_index != self.trace_instance_index {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "parallel guest PC trace stream finish {} reached segment {}",
                    finish.trace_instance_index, self.trace_instance_index
                ),
            });
        }
        let mut pending =
            self.pending
                .take()
                .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: format!(
                        "parallel guest PC trace stream segment {} finished before metadata",
                        self.trace_instance_index
                    ),
                })?;
        if !pending.reports.is_empty() {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "parallel guest PC trace stream segment {} overlapped carried reports",
                    self.trace_instance_index
                ),
            });
        }
        if pending.report_count != self.report_count {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "parallel guest PC trace stream segment {} lost reports",
                    self.trace_instance_index
                ),
            });
        }
        if let Some(reports) = self.reports {
            pending.reports = reports;
        }
        if pending.is_last_segment || self.builder.is_none() {
            timing.parallel_lower_stream_fallback_count = timing
                .parallel_lower_stream_fallback_count
                .saturating_add(1);
            if pending.reports.is_empty() && pending.report_count != 0 {
                pending.reports_elided = true;
                replay_guest_pc_trace_pending_segment_reports(layout, &mut pending, timing)?;
            }
            return lower_guest_pc_trace_parallel_pending_job(
                layout,
                pending,
                expected_proof_values,
                timing,
            );
        }
        let mut builder = self.builder.take().expect("builder presence checked");
        self.feeder
            .finish(&mut builder, pending.lookahead_instruction, Some(timing))?;
        let mut aggregate_timing = Some(&mut *timing);
        record_aggregate_trace_report_duration(
            &mut aggregate_timing,
            self.aggregate_report_started,
        );
        let GuestPcTraceDeviceSegmentBuild {
            device_segment_material,
            unit_values,
            final_state,
            continuation_state,
        } = builder.finish(pending.terminal_pc, Some(timing))?;
        let next_seed = ZiskMainSegmentSeed {
            initial_state: continuation_state,
            previous_c: final_state.last_c,
        };
        timing.parallel_lower_stream_segment_count =
            timing.parallel_lower_stream_segment_count.saturating_add(1);
        Ok(GuestPcTraceSeededLoweredSegment {
            seed: self.seed,
            lowered: GuestPcTraceLoweredSegment {
                segment: GuestPcTraceSegmentTrace {
                    trace_instance_index: pending.trace_instance_index,
                    trace_source_prefix_rows: device_segment_material.trace_source_prefix_rows,
                    device_segment_material: Some(device_segment_material),
                    trace: None,
                    unit_values,
                    proof_values: expected_proof_values.unwrap_or_default().to_vec(),
                },
                next_seed,
            },
        })
    }
}

fn dispatch_guest_pc_trace_parallel_lower_job<T: GuestPcTraceParallelLowerDispatchClass>(
    job_senders: &[mpsc::SyncSender<T>],
    next_worker: &mut usize,
    mut job: T,
    timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<usize, T> {
    let worker_count = job_senders.len();
    if worker_count == 0 {
        return Err(job);
    }
    let start = *next_worker % worker_count;
    let mut saw_full_queue = false;
    for offset in 0..worker_count {
        let worker = (start + offset) % worker_count;
        match job_senders[worker].try_send(job) {
            Ok(()) => {
                *next_worker = (worker + 1) % worker_count;
                return Ok(worker);
            }
            Err(mpsc::TrySendError::Full(returned_job)) => {
                saw_full_queue = true;
                job = returned_job;
            }
            Err(mpsc::TrySendError::Disconnected(returned_job)) => {
                job = returned_job;
            }
        }
    }
    if saw_full_queue {
        let stream_kind = job.stream_dispatch_kind();
        let send_started = Instant::now();
        let result = job_senders[start].send(job).map_err(|error| error.0);
        if let Some(timing) = timing {
            record_guest_pc_trace_parallel_lower_dispatch_wait(
                timing,
                stream_kind,
                send_started.elapsed(),
            );
            timing.parallel_lower_dispatch_blocked_count = timing
                .parallel_lower_dispatch_blocked_count
                .saturating_add(1);
        }
        result?;
    } else {
        job_senders[start].send(job).map_err(|error| error.0)?;
    }
    *next_worker = (start + 1) % worker_count;
    Ok(start)
}

#[cfg(feature = "cuda")]
fn send_guest_pc_trace_parallel_lower_job_to_worker(
    job_senders: &[mpsc::SyncSender<GuestPcTraceParallelLowerJob>],
    worker: usize,
    job: GuestPcTraceParallelLowerJob,
    timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<(), GuestPcTraceParallelLowerJob> {
    let Some(sender) = job_senders.get(worker) else {
        return Err(job);
    };
    let stream_kind = job.stream_dispatch_kind();
    match sender.try_send(job) {
        Ok(()) => Ok(()),
        Err(mpsc::TrySendError::Full(job)) => {
            let send_started = Instant::now();
            let result = sender.send(job).map_err(|error| error.0);
            if let Some(timing) = timing {
                record_guest_pc_trace_parallel_lower_dispatch_wait(
                    timing,
                    stream_kind,
                    send_started.elapsed(),
                );
                timing.parallel_lower_dispatch_blocked_count = timing
                    .parallel_lower_dispatch_blocked_count
                    .saturating_add(1);
            }
            result
        }
        Err(mpsc::TrySendError::Disconnected(job)) => Err(job),
    }
}

fn lower_guest_pc_trace_pending_segments_parallel(
    layout: &WitnessTraceLayout,
    pending_receiver: mpsc::Receiver<GuestPcTracePendingSegmentMessage>,
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    timing: &mut GuestPcTraceStreamTiming,
    emit: &mut impl FnMut(GuestPcTraceSegmentTrace) -> Result<(), GuestPcTraceBackendError>,
    worker_count: usize,
) -> Result<GuestPcTraceStreamResult, GuestPcTraceBackendError> {
    thread::scope(|scope| {
        let worker_count = worker_count.max(2);
        timing.parallel_lower_worker_count = timing.parallel_lower_worker_count.max(worker_count);
        let (result_sender, result_receiver) = mpsc::sync_channel(
            guest_pc_trace_parallel_lower_result_queue_capacity(worker_count),
        );
        let mut worker_handles = Vec::with_capacity(worker_count);
        let mut job_senders = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let (job_sender, job_receiver) = mpsc::sync_channel::<GuestPcTraceParallelLowerJob>(1);
            job_senders.push(job_sender);
            let result_sender = result_sender.clone();
            worker_handles.push(spawn_guest_pc_trace_thread(
                scope,
                "lzvm-gp-plow",
                move || {
                    #[cfg(feature = "cuda")]
                    let mut active_stream: Option<GuestPcTraceParallelStreamedSegment> = None;
                    #[cfg(feature = "cuda")]
                    let mut active_stream_timing = GuestPcTraceStreamTiming::default();
                    loop {
                        let job_receive_started = Instant::now();
                        let job = match job_receiver.recv() {
                            Ok(job) => job,
                            Err(_) => break,
                        };
                        let job_receive_elapsed = job_receive_started.elapsed();
                        let mut worker_timing = GuestPcTraceStreamTiming::default();
                        let (trace_instance_index, result) = match job {
                            GuestPcTraceParallelLowerJob::Segment(pending) => {
                                worker_timing.parallel_lower_job_receive_wait_duration +=
                                    job_receive_elapsed;
                                let pending = *pending;
                                let trace_instance_index = pending.trace_instance_index;
                                let result = lower_guest_pc_trace_parallel_pending_job(
                                    layout,
                                    pending,
                                    expected_proof_values,
                                    &mut worker_timing,
                                );
                                (trace_instance_index, Some(result))
                            }
                            #[cfg(feature = "cuda")]
                            GuestPcTraceParallelLowerJob::StreamStart(start) => {
                                let trace_instance_index = start.trace_instance_index;
                                let result = if active_stream.is_some() {
                                    Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                                        message:
                                            "parallel guest PC trace stream overlapped active segment"
                                                .to_owned(),
                                    })
                                } else {
                                    GuestPcTraceParallelStreamedSegment::new(
                                        layout,
                                        *start,
                                        true,
                                    )
                                    .map(|stream| {
                                        active_stream = Some(stream);
                                    })
                                };
                                match result {
                                    Ok(()) => {
                                        active_stream_timing = GuestPcTraceStreamTiming::default();
                                        active_stream_timing
                                            .parallel_lower_job_receive_wait_duration +=
                                            job_receive_elapsed;
                                        (trace_instance_index, None)
                                    }
                                    Err(error) => {
                                        worker_timing.parallel_lower_job_receive_wait_duration +=
                                            job_receive_elapsed;
                                        (trace_instance_index, Some(Err(error)))
                                    }
                                }
                            }
                            #[cfg(feature = "cuda")]
                            GuestPcTraceParallelLowerJob::StreamChunk(chunk) => {
                                let trace_instance_index = chunk.trace_instance_index;
                                active_stream_timing
                                    .parallel_lower_job_receive_wait_duration +=
                                    job_receive_elapsed;
                                let chunk_process_started = Instant::now();
                                let result = active_stream
                                    .as_mut()
                                    .ok_or_else(|| {
                                        GuestPcTraceBackendError::InvalidPcTraceLayout {
                                            message: format!(
                                                "parallel guest PC trace stream chunk {} arrived before start",
                                                trace_instance_index
                                            ),
                                        }
                                    })
                                    .and_then(|stream| {
                                        stream.push_chunk(*chunk, &mut active_stream_timing)
                                    });
                                active_stream_timing.parallel_lower_stream_chunk_process_duration +=
                                    chunk_process_started.elapsed();
                                match result {
                                    Ok(()) => (trace_instance_index, None),
                                    Err(error) => {
                                        worker_timing.add(std::mem::take(
                                            &mut active_stream_timing,
                                        ));
                                        (trace_instance_index, Some(Err(error)))
                                    }
                                }
                            }
                            #[cfg(feature = "cuda")]
                            GuestPcTraceParallelLowerJob::StreamSegment(pending) => {
                                let trace_instance_index = pending.trace_instance_index;
                                active_stream_timing
                                    .parallel_lower_job_receive_wait_duration +=
                                    job_receive_elapsed;
                                let result = active_stream
                                    .as_mut()
                                    .ok_or_else(|| {
                                        GuestPcTraceBackendError::InvalidPcTraceLayout {
                                            message: format!(
                                                "parallel guest PC trace stream segment {} arrived before start",
                                                trace_instance_index
                                            ),
                                        }
                                    })
                                    .and_then(|stream| stream.set_pending(*pending));
                                match result {
                                    Ok(()) => (trace_instance_index, None),
                                    Err(error) => {
                                        worker_timing.add(std::mem::take(
                                            &mut active_stream_timing,
                                        ));
                                        (trace_instance_index, Some(Err(error)))
                                    }
                                }
                            }
                            #[cfg(feature = "cuda")]
                            GuestPcTraceParallelLowerJob::StreamFinish(finish) => {
                                let trace_instance_index = finish.trace_instance_index;
                                active_stream_timing
                                    .parallel_lower_job_receive_wait_duration +=
                                    job_receive_elapsed;
                                let result = active_stream
                                    .take()
                                    .ok_or_else(|| {
                                        GuestPcTraceBackendError::InvalidPcTraceLayout {
                                            message: format!(
                                                "parallel guest PC trace stream finish {} arrived before start",
                                                trace_instance_index
                                            ),
                                        }
                                    })
                                    .and_then(|stream| {
                                        stream.finish(
                                            layout,
                                            *finish,
                                            expected_proof_values,
                                            &mut active_stream_timing,
                                        )
                                    });
                                worker_timing.add(std::mem::take(&mut active_stream_timing));
                                (trace_instance_index, Some(result))
                            }
                        };
                        if let Some(result) = result {
                            if !send_guest_pc_trace_parallel_lower_segment_result(
                                &result_sender,
                                trace_instance_index,
                                result,
                                worker_timing,
                            ) {
                                break;
                            }
                        }
                    }
                },
            ));
        }

        let dispatcher_sender = result_sender.clone();
        let dispatcher_handle = spawn_guest_pc_trace_thread(scope, "lzvm-gp-pdisp", move || {
            let mut dispatcher_timing = GuestPcTraceStreamTiming::default();
            let mut dispatched_count = 0_usize;
            let mut next_worker = 0_usize;
            let mut pending_chunks = BTreeMap::new();
            let mut pending_chunked_segments = BTreeMap::new();
            #[cfg(feature = "cuda")]
            let mut pending_stream_segments = BTreeMap::<u32, usize>::new();
            loop {
                let receive_started = Instant::now();
                let message = match pending_receiver.recv() {
                    Ok(message) => message,
                    Err(_) => {
                        let _ = dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Error {
                            error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                message: "guest PC trace pending segment runner stopped".to_owned(),
                            },
                            dispatched_count,
                            timing: dispatcher_timing,
                        });
                        break;
                    }
                };
                dispatcher_timing.pending_receive_wait_duration += receive_started.elapsed();
                match message {
                    GuestPcTracePendingSegmentMessage::Segment(pending) => {
                        let pending = *pending;
                        dispatcher_timing.trace_report_buffer_capacity += pending.report_capacity;
                        dispatcher_timing.trace_report_buffer_max_capacity = dispatcher_timing
                            .trace_report_buffer_max_capacity
                            .max(pending.report_capacity);
                        dispatcher_timing.trace_report_buffer_excess_capacity += pending
                            .report_capacity
                            .saturating_sub(pending.reports.len());
                        if dispatch_guest_pc_trace_parallel_lower_job(
                            &job_senders,
                            &mut next_worker,
                            GuestPcTraceParallelLowerJob::Segment(Box::new(pending)),
                            Some(&mut dispatcher_timing),
                        )
                        .is_err()
                        {
                            let _ =
                                dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Error {
                                    error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                        message: "parallel guest PC trace lower worker stopped"
                                            .to_owned(),
                                    },
                                    dispatched_count,
                                    timing: dispatcher_timing,
                                });
                            break;
                        }
                        dispatched_count = dispatched_count.saturating_add(1);
                    }
                    GuestPcTracePendingSegmentMessage::SegmentStreamStarted(start) => {
                        #[cfg(feature = "cuda")]
                        if guest_pc_trace_parallel_stream_chunks_enabled() {
                            let trace_instance_index = start.trace_instance_index;
                            if pending_stream_segments.contains_key(&trace_instance_index) {
                                let _ = dispatcher_sender.send(
                                    GuestPcTraceParallelLowerMessage::Error {
                                        error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                            message:
                                                "guest PC trace duplicate stream segment start"
                                                    .to_owned(),
                                        },
                                        dispatched_count,
                                        timing: dispatcher_timing,
                                    },
                                );
                                break;
                            }
                            match dispatch_guest_pc_trace_parallel_lower_job(
                                &job_senders,
                                &mut next_worker,
                                GuestPcTraceParallelLowerJob::StreamStart(start),
                                Some(&mut dispatcher_timing),
                            ) {
                                Ok(worker) => {
                                    pending_stream_segments.insert(trace_instance_index, worker);
                                    dispatched_count = dispatched_count.saturating_add(1);
                                }
                                Err(_) => {
                                    let _ =
                                        dispatcher_sender
                                            .send(GuestPcTraceParallelLowerMessage::Error {
                                            error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                                message:
                                                    "parallel guest PC trace lower worker stopped"
                                                        .to_owned(),
                                            },
                                            dispatched_count,
                                            timing: dispatcher_timing,
                                        });
                                    break;
                                }
                            }
                        }
                        #[cfg(not(feature = "cuda"))]
                        {
                            let _ = start;
                        }
                    }
                    GuestPcTracePendingSegmentMessage::SegmentStarted(pending) => {
                        let pending = *pending;
                        #[cfg(feature = "cuda")]
                        if let Some(&worker) =
                            pending_stream_segments.get(&pending.trace_instance_index)
                        {
                            if send_guest_pc_trace_parallel_lower_job_to_worker(
                                &job_senders,
                                worker,
                                GuestPcTraceParallelLowerJob::StreamSegment(Box::new(pending)),
                                Some(&mut dispatcher_timing),
                            )
                            .is_err()
                            {
                                let _ = dispatcher_sender.send(
                                    GuestPcTraceParallelLowerMessage::Error {
                                        error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                            message: "parallel guest PC trace lower worker stopped"
                                                .to_owned(),
                                        },
                                        dispatched_count,
                                        timing: dispatcher_timing,
                                    },
                                );
                                break;
                            }
                            continue;
                        }
                        if pending_chunked_segments
                            .insert(pending.trace_instance_index, pending)
                            .is_some()
                        {
                            let _ =
                                dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Error {
                                    error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                        message: "guest PC trace duplicate chunked segment start"
                                            .to_owned(),
                                    },
                                    dispatched_count,
                                    timing: dispatcher_timing,
                                });
                            break;
                        }
                    }
                    GuestPcTracePendingSegmentMessage::ReportChunk(chunk) => {
                        #[cfg(feature = "cuda")]
                        if let Some(&worker) =
                            pending_stream_segments.get(&chunk.trace_instance_index)
                        {
                            if send_guest_pc_trace_parallel_lower_job_to_worker(
                                &job_senders,
                                worker,
                                GuestPcTraceParallelLowerJob::StreamChunk(chunk),
                                Some(&mut dispatcher_timing),
                            )
                            .is_err()
                            {
                                let _ = dispatcher_sender.send(
                                    GuestPcTraceParallelLowerMessage::Error {
                                        error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                            message: "parallel guest PC trace lower worker stopped"
                                                .to_owned(),
                                        },
                                        dispatched_count,
                                        timing: dispatcher_timing,
                                    },
                                );
                                break;
                            }
                            continue;
                        }
                        receive_guest_pc_trace_pending_report_chunk(
                            *chunk,
                            &mut pending_chunks,
                            &mut dispatcher_timing,
                        );
                    }
                    GuestPcTracePendingSegmentMessage::SegmentFinished(finish) => {
                        #[cfg(feature = "cuda")]
                        if let Some(worker) =
                            pending_stream_segments.remove(&finish.trace_instance_index)
                        {
                            if send_guest_pc_trace_parallel_lower_job_to_worker(
                                &job_senders,
                                worker,
                                GuestPcTraceParallelLowerJob::StreamFinish(finish),
                                Some(&mut dispatcher_timing),
                            )
                            .is_err()
                            {
                                let _ = dispatcher_sender.send(
                                    GuestPcTraceParallelLowerMessage::Error {
                                        error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                            message: "parallel guest PC trace lower worker stopped"
                                                .to_owned(),
                                        },
                                        dispatched_count,
                                        timing: dispatcher_timing,
                                    },
                                );
                                break;
                            }
                            continue;
                        }
                        let pending = match pending_chunked_segments
                            .remove(&finish.trace_instance_index)
                            .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                                message: format!(
                                    "guest PC trace chunked segment {} finished before start",
                                    finish.trace_instance_index
                                ),
                            })
                            .and_then(|pending| {
                                finish_guest_pc_trace_chunked_pending_segment(
                                    pending,
                                    &mut pending_chunks,
                                )
                            }) {
                            Ok(pending) => pending,
                            Err(error) => {
                                let _ = dispatcher_sender.send(
                                    GuestPcTraceParallelLowerMessage::Error {
                                        error,
                                        dispatched_count,
                                        timing: dispatcher_timing,
                                    },
                                );
                                break;
                            }
                        };
                        dispatcher_timing.trace_report_buffer_capacity += pending.report_capacity;
                        dispatcher_timing.trace_report_buffer_max_capacity = dispatcher_timing
                            .trace_report_buffer_max_capacity
                            .max(pending.report_capacity);
                        dispatcher_timing.trace_report_buffer_excess_capacity += pending
                            .report_capacity
                            .saturating_sub(pending.reports.len());
                        if dispatch_guest_pc_trace_parallel_lower_job(
                            &job_senders,
                            &mut next_worker,
                            GuestPcTraceParallelLowerJob::Segment(Box::new(pending)),
                            Some(&mut dispatcher_timing),
                        )
                        .is_err()
                        {
                            let _ =
                                dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Error {
                                    error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                        message: "parallel guest PC trace lower worker stopped"
                                            .to_owned(),
                                    },
                                    dispatched_count,
                                    timing: dispatcher_timing,
                                });
                            break;
                        }
                        dispatched_count = dispatched_count.saturating_add(1);
                    }
                    GuestPcTracePendingSegmentMessage::Complete(stream) => {
                        if let Err(error) =
                            validate_guest_pc_trace_no_pending_report_chunks(&pending_chunks)
                        {
                            let _ =
                                dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Error {
                                    error,
                                    dispatched_count,
                                    timing: dispatcher_timing,
                                });
                            break;
                        }
                        if !pending_chunked_segments.is_empty() {
                            let _ =
                                dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Error {
                                    error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                        message: "guest PC trace chunked segment missing finish"
                                            .to_owned(),
                                    },
                                    dispatched_count,
                                    timing: dispatcher_timing,
                                });
                            break;
                        }
                        #[cfg(feature = "cuda")]
                        if !pending_stream_segments.is_empty() {
                            let _ =
                                dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Error {
                                    error: GuestPcTraceBackendError::InvalidPcTraceLayout {
                                        message: "guest PC trace stream segment missing finish"
                                            .to_owned(),
                                    },
                                    dispatched_count,
                                    timing: dispatcher_timing,
                                });
                            break;
                        }
                        dispatcher_timing.parallel_lower_worker_count = dispatcher_timing
                            .parallel_lower_worker_count
                            .max(worker_count);
                        dispatcher_timing.parallel_lower_dispatched_count = dispatcher_timing
                            .parallel_lower_dispatched_count
                            .saturating_add(dispatched_count);
                        let _ =
                            dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Complete {
                                stream,
                                dispatched_count,
                                timing: dispatcher_timing,
                            });
                        break;
                    }
                    GuestPcTracePendingSegmentMessage::Error(error) => {
                        dispatcher_timing.parallel_lower_worker_count = dispatcher_timing
                            .parallel_lower_worker_count
                            .max(worker_count);
                        dispatcher_timing.parallel_lower_dispatched_count = dispatcher_timing
                            .parallel_lower_dispatched_count
                            .saturating_add(dispatched_count);
                        let _ = dispatcher_sender.send(GuestPcTraceParallelLowerMessage::Error {
                            error,
                            dispatched_count,
                            timing: dispatcher_timing,
                        });
                        break;
                    }
                }
            }
        });
        drop(result_sender);

        let mut next_emit_index = 0_u32;
        let mut current_seed = ZiskMainSegmentSeed::new();
        let mut reorder = BTreeMap::<u32, GuestPcTraceSeededLoweredSegment>::new();
        let mut received_count = 0_usize;
        let mut emitted_count = 0_usize;
        let mut dispatched_count = None;
        let mut stream_result = None;
        let mut first_error = None;

        while stream_result.is_none()
            || dispatched_count.is_none_or(|count| received_count < count)
            || (first_error.is_none() && dispatched_count.is_none_or(|count| emitted_count < count))
        {
            let result_receive_started = Instant::now();
            let message = match result_receiver.recv() {
                Ok(message) => {
                    timing.parallel_lower_result_receive_wait_duration +=
                        result_receive_started.elapsed();
                    message
                }
                Err(_) => {
                    timing.parallel_lower_result_receive_wait_duration +=
                        result_receive_started.elapsed();
                    break;
                }
            };
            match message {
                GuestPcTraceParallelLowerMessage::Segment {
                    trace_instance_index,
                    result,
                    timing: worker_timing,
                } => {
                    timing.add(worker_timing);
                    received_count = received_count.saturating_add(1);
                    timing.parallel_lower_received_count =
                        timing.parallel_lower_received_count.saturating_add(1);
                    match *result {
                        Ok(lowered) if first_error.is_none() => {
                            reorder.insert(trace_instance_index, lowered);
                            timing.parallel_lower_max_reorder_count =
                                timing.parallel_lower_max_reorder_count.max(reorder.len());
                            while let Some(lowered) = reorder.remove(&next_emit_index) {
                                if let Err(error) = validate_guest_pc_trace_pending_segment_seed(
                                    lowered.lowered.segment.trace_instance_index,
                                    Some(&lowered.seed),
                                    &current_seed.initial_state,
                                    current_seed.previous_c,
                                ) {
                                    first_error.get_or_insert(error);
                                    break;
                                }
                                current_seed = lowered.lowered.next_seed.clone();
                                let emit_started = Instant::now();
                                if let Err(error) = emit(lowered.lowered.segment) {
                                    first_error.get_or_insert(error);
                                    break;
                                }
                                timing.trace_emit_duration += emit_started.elapsed();
                                emitted_count = emitted_count.saturating_add(1);
                                timing.parallel_lower_emitted_count =
                                    timing.parallel_lower_emitted_count.saturating_add(1);
                                next_emit_index =
                                    next_emit_index.checked_add(1).ok_or_else(|| {
                                        GuestPcTraceBackendError::InvalidPcTraceLayout {
                                        message:
                                            "parallel guest PC trace lower segment index overflow"
                                                .to_owned(),
                                    }
                                    })?;
                            }
                        }
                        Ok(_) => {}
                        Err(error) => {
                            first_error.get_or_insert(error);
                        }
                    }
                }
                GuestPcTraceParallelLowerMessage::Complete {
                    stream,
                    dispatched_count: count,
                    timing: dispatcher_timing,
                } => {
                    timing.add(dispatcher_timing);
                    dispatched_count = Some(count);
                    stream_result = Some(Ok(*stream));
                }
                GuestPcTraceParallelLowerMessage::Error {
                    error,
                    dispatched_count: count,
                    timing: dispatcher_timing,
                } => {
                    timing.add(dispatcher_timing);
                    dispatched_count = Some(count);
                    stream_result = Some(Err(error));
                }
            }
        }

        if dispatcher_handle.join().is_err() {
            first_error.get_or_insert(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "parallel guest PC trace lower dispatcher panicked".to_owned(),
            });
        }
        for handle in worker_handles {
            if handle.join().is_err() {
                first_error.get_or_insert(GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "parallel guest PC trace lower worker panicked".to_owned(),
                });
            }
        }

        if let Some(error) = first_error {
            return Err(error);
        }
        match stream_result {
            Some(Ok(stream)) => Ok(stream),
            Some(Err(error)) => Err(error),
            None => Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "parallel guest PC trace lower stopped before completion".to_owned(),
            }),
        }
    })
}

fn validate_guest_pc_trace_pending_segment_seed(
    trace_instance_index: u32,
    seed: Option<&ZiskMainSegmentSeed>,
    trace_state: &ZiskMainTraceState,
    previous_c: u64,
) -> Result<(), GuestPcTraceBackendError> {
    let Some(seed) = seed else {
        return Ok(());
    };
    if seed.previous_c != previous_c || seed.initial_state != *trace_state {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "guest PC trace seed mirror mismatch at segment {trace_instance_index}"
            ),
        });
    }
    Ok(())
}

fn guest_pc_trace_seed_mirror_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_SEED_MIRROR", false)
}

fn guest_pc_trace_segment_replay_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY", false)
}

fn guest_pc_trace_segment_replay_snapshot_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY_SNAPSHOT", false)
}

fn guest_pc_trace_parallel_lower_replay_snapshot_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_SNAPSHOT", false)
        || guest_pc_trace_parallel_lower_report_elision_enabled()
}

fn guest_pc_trace_parallel_lower_report_elision_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY", false)
}

fn guest_pc_trace_runner_seed_snapshot_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT", false)
        || guest_pc_trace_parallel_lower_enabled()
}

fn guest_pc_trace_runner_seed_snapshot_trusted_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED", false)
        || guest_pc_trace_parallel_lower_enabled()
}

fn guest_pc_trace_runner_seed_snapshot_validation_enabled() -> bool {
    cfg!(debug_assertions)
        || env_flag_enabled("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_VALIDATE", false)
}

fn guest_pc_trace_parallel_lower_worker_count() -> Option<usize> {
    if !guest_pc_trace_parallel_lower_enabled() {
        return None;
    }
    let configured = std::env::var("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0);
    let worker_count = configured.unwrap_or_else(|| {
        thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
    });
    Some(worker_count.max(1))
}

fn guest_pc_trace_parallel_lower_result_queue_capacity(worker_count: usize) -> usize {
    worker_count.max(1)
}

fn guest_pc_trace_parallel_lower_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER", false)
}

#[cfg(feature = "cuda")]
fn guest_pc_trace_parallel_stream_chunks_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_PC_TRACE_PARALLEL_STREAM_CHUNKS", false)
}

#[cfg(feature = "cuda")]
fn guest_pc_trace_owned_streaming_lower_enabled() -> bool {
    env_flag_enabled("LZVM_CUDA_GUEST_PC_OWNED_STREAMING_LOWER", false)
}

fn guest_pc_trace_needs_full_seed_advance(
    seed_present: bool,
    runner_seed_snapshot_trusted: bool,
    validate_runner_seed_snapshot: bool,
    is_last_segment: bool,
    runner_direct_next_seed_present: bool,
) -> bool {
    seed_present
        && (!runner_seed_snapshot_trusted
            || validate_runner_seed_snapshot
            || is_last_segment
            || !runner_direct_next_seed_present)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayoutTraceCapacity {
    row_count: usize,
    row_width: usize,
    instruction_limit: u64,
}

fn layout_trace_capacity(
    layout: Option<&WitnessTraceLayout>,
) -> Result<Option<LayoutTraceCapacity>, GuestPcTraceBackendError> {
    let Some(layout) = layout else {
        return Ok(None);
    };
    let (row_width, instruction_limit) = if zisk_main_trace_columns(layout)?.is_some() {
        (
            layout.column_count(),
            u64::try_from(layout.row_count()).unwrap_or(u64::MAX),
        )
    } else if precompile_memory_trace::precompile_memory_trace_columns(layout)?.is_some() {
        (layout.column_count(), u64::MAX)
    } else {
        match guest_trace_columns(layout)? {
            Some(_) => (
                layout.column_count(),
                u64::try_from(layout.row_count()).unwrap_or(u64::MAX),
            ),
            None if is_raw_pc_pair_layout(layout) => {
                (2, u64::try_from(layout.row_count()).unwrap_or(u64::MAX))
            }
            None => return Err(GuestPcTraceBackendError::UnmappedTraceLayout),
        }
    };
    Ok(Some(LayoutTraceCapacity {
        row_count: layout.row_count(),
        row_width,
        instruction_limit,
    }))
}

fn layout_capacity_error(
    capacity: Option<LayoutTraceCapacity>,
    run_instruction_limit: u64,
    error: &GuestMachineRunError,
) -> Option<GuestPcTraceBackendError> {
    let capacity = capacity?;
    match error {
        GuestMachineRunError::InstructionLimitExceeded {
            instruction_limit, ..
        } if *instruction_limit == run_instruction_limit
            && run_instruction_limit == capacity.instruction_limit =>
        {
            let required_rows = capacity.row_count.saturating_add(1);
            Some(GuestPcTraceBackendError::TraceCapacityExceeded {
                rows: capacity.row_count,
                row_width: capacity.row_width,
                required_rows,
                required_trace_instances: required_trace_instances(
                    required_rows,
                    capacity.row_count,
                ),
            })
        }
        _ => None,
    }
}

fn required_trace_instances(required_rows: usize, rows_per_instance: usize) -> usize {
    if required_rows == 0 {
        0
    } else if rows_per_instance == 0 {
        usize::MAX
    } else {
        required_rows.div_ceil(rows_per_instance)
    }
}

fn layout_trace_byte_len(row_count: usize, column_count: usize) -> usize {
    row_count
        .checked_mul(column_count)
        .and_then(|count| count.checked_mul(8))
        .unwrap_or(usize::MAX)
}

fn is_raw_pc_pair_layout(layout: &WitnessTraceLayout) -> bool {
    layout.column_count() == 2 && layout.columns().is_empty()
}

fn guest_machine_halt_pc(halt: &GuestMachineHalt) -> u64 {
    match halt {
        GuestMachineHalt::Ecall { address } => *address,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraceColumnTarget<'a> {
    column: ResolvedTraceColumn<'a>,
}

impl<'a> TraceColumnTarget<'a> {
    fn name(&self) -> &str {
        self.column.name()
    }

    fn trace_column(&self) -> usize {
        self.column.trace_column()
    }

    fn resolved(&self) -> &ResolvedTraceColumn<'a> {
        &self.column
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PcTraceColumns<'a> {
    pc: TraceColumnTarget<'a>,
    next_pc: TraceColumnTarget<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegisterWriteColumns<'a> {
    index: TraceColumnTarget<'a>,
    value: TraceColumnTarget<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryAccessColumns<'a> {
    address: TraceColumnTarget<'a>,
    value: TraceColumnTarget<'a>,
    byte_len: TraceColumnTarget<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestTraceColumns<'a> {
    pc: Option<PcTraceColumns<'a>>,
    register_write: Option<RegisterWriteColumns<'a>>,
    memory_read: Option<MemoryAccessColumns<'a>>,
    memory_write: Option<MemoryAccessColumns<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZiskMainTraceColumns<'a> {
    a: TraceColumnTarget<'a>,
    b: TraceColumnTarget<'a>,
    c: TraceColumnTarget<'a>,
    flag: TraceColumnTarget<'a>,
    pc: TraceColumnTarget<'a>,
    a_src_imm: Option<TraceColumnTarget<'a>>,
    a_offset_imm0: Option<TraceColumnTarget<'a>>,
    a_imm1: Option<TraceColumnTarget<'a>>,
    b_src_imm: Option<TraceColumnTarget<'a>>,
    b_imm1: Option<TraceColumnTarget<'a>>,
    a_src_reg: Option<TraceColumnTarget<'a>>,
    b_src_reg: Option<TraceColumnTarget<'a>>,
    a_src_mem: Option<TraceColumnTarget<'a>>,
    b_src_mem: Option<TraceColumnTarget<'a>>,
    b_src_ind: Option<TraceColumnTarget<'a>>,
    b_offset_imm0: Option<TraceColumnTarget<'a>>,
    addr1: Option<TraceColumnTarget<'a>>,
    addr2: Option<TraceColumnTarget<'a>>,
    store_reg: Option<TraceColumnTarget<'a>>,
    store_mem: Option<TraceColumnTarget<'a>>,
    store_ind: Option<TraceColumnTarget<'a>>,
    store_offset: Option<TraceColumnTarget<'a>>,
    store_pc: Option<TraceColumnTarget<'a>>,
    set_pc: Option<TraceColumnTarget<'a>>,
    op: TraceColumnTarget<'a>,
    jmp_offset1: Option<TraceColumnTarget<'a>>,
    jmp_offset2: Option<TraceColumnTarget<'a>>,
    ind_width: Option<TraceColumnTarget<'a>>,
    m32: Option<TraceColumnTarget<'a>>,
    is_external_op: Option<TraceColumnTarget<'a>>,
    is_precompiled: Option<TraceColumnTarget<'a>>,
    a_reg_prev_mem_step: Option<TraceColumnTarget<'a>>,
    b_reg_prev_mem_step: Option<TraceColumnTarget<'a>>,
    store_reg_prev_mem_step: Option<TraceColumnTarget<'a>>,
    store_reg_prev_value: Option<TraceColumnTarget<'a>>,
}

impl ZiskMainTraceColumns<'_> {
    fn has_required_indirect_memory_columns(&self) -> bool {
        self.b_src_ind.is_some()
            && self.b_offset_imm0.is_some()
            && self.ind_width.is_some()
            && self.store_ind.is_some()
            && self.store_offset.is_some()
            && self.store_mem.is_some()
    }

    fn has_required_a_memory_source_columns(&self) -> bool {
        self.a_src_mem.is_some() && self.a_offset_imm0.is_some()
    }

    fn has_required_b_memory_source_columns(&self) -> bool {
        self.b_src_mem.is_some() && self.b_offset_imm0.is_some()
    }

    fn has_required_memory_store_columns(&self) -> bool {
        self.store_mem.is_some() && self.store_offset.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExpectedMemoryAccess {
    kind: GuestMemoryAccessKind,
    address: u64,
    byte_len: usize,
    value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZiskMainTraceState {
    registers: [u64; 32],
    internal_memory: ZiskMainInternalMemory,
    register_mem_steps: [u64; 32],
    pending_dma: Option<ZiskMainPendingDma>,
    last_c: u64,
    next_pc: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ZiskMainInternalMemory {
    extra_params: Option<u64>,
    amo_temp: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZiskMainInternalMemoryAddressError;

impl ZiskMainInternalMemory {
    fn new() -> Self {
        Self::default()
    }

    fn get(&self, address: u64) -> Option<u64> {
        match address {
            ZISK_EXTRA_PARAMS_ADDRESS => self.extra_params,
            address if address == zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER) => {
                self.amo_temp
            }
            _ => None,
        }
    }

    fn insert(
        &mut self,
        address: u64,
        value: u64,
    ) -> Result<(), ZiskMainInternalMemoryAddressError> {
        match address {
            ZISK_EXTRA_PARAMS_ADDRESS => {
                self.extra_params = Some(value);
                Ok(())
            }
            address if address == zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER) => {
                self.amo_temp = Some(value);
                Ok(())
            }
            _ => Err(ZiskMainInternalMemoryAddressError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZiskMainPendingDma {
    kind: RiscvDmaKind,
    first_arg_reg: u8,
}

impl ZiskMainTraceState {
    fn new() -> Self {
        Self {
            registers: [0; 32],
            internal_memory: ZiskMainInternalMemory::new(),
            register_mem_steps: [0; 32],
            pending_dma: None,
            last_c: 0,
            next_pc: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZiskMainSegmentSeed {
    initial_state: ZiskMainTraceState,
    previous_c: u64,
}

impl ZiskMainSegmentSeed {
    fn new() -> Self {
        Self {
            initial_state: ZiskMainTraceState::new(),
            previous_c: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZiskMainRunnerBoundarySnapshot {
    internal_memory: ZiskMainInternalMemory,
}

impl ZiskMainRunnerBoundarySnapshot {
    fn new(seed: &ZiskMainSegmentSeed) -> Self {
        Self {
            internal_memory: seed.initial_state.internal_memory,
        }
    }

    fn record_report(
        &mut self,
        report: &GuestMachineReport,
        next_instruction: Option<RiscvInstruction>,
        registers: &[u64; 32],
    ) -> Result<(), GuestPcTraceBackendError> {
        record_zisk_main_runner_scratch_update(
            &mut self.internal_memory,
            registers,
            report,
            next_instruction,
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct ZiskMainRunnerBoundarySeedInput<'a> {
    reports: &'a [GuestMachineReport],
    report_count: usize,
    last_report: Option<&'a GuestMachineReport>,
    lookahead_instruction: Option<RiscvInstruction>,
    runner_state: &'a GuestMachineState,
    current_seed: &'a ZiskMainSegmentSeed,
    boundary_snapshot: &'a ZiskMainRunnerBoundarySnapshot,
}

impl<'a> ZiskMainRunnerBoundarySeedInput<'a> {
    #[cfg(test)]
    fn from_reports(
        reports: &'a [GuestMachineReport],
        lookahead_instruction: Option<RiscvInstruction>,
        runner_state: &'a GuestMachineState,
        current_seed: &'a ZiskMainSegmentSeed,
        boundary_snapshot: &'a ZiskMainRunnerBoundarySnapshot,
    ) -> Self {
        Self {
            reports,
            report_count: reports.len(),
            last_report: reports.last(),
            lookahead_instruction,
            runner_state,
            current_seed,
            boundary_snapshot,
        }
    }

    fn report_count(self) -> usize {
        self.report_count
    }

    fn last_report(self) -> Option<&'a GuestMachineReport> {
        self.reports.last().or(self.last_report)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZiskMainDirectSeedLiftMissReason {
    EmptySegment,
    PendingDmaSingleReport,
    AmoBoundary,
    StoreConditionalBoundary,
    DmaPrepareMissingLookahead,
    BoundaryCUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZiskMainTraceSegmentInfo {
    trace_instance_index: u32,
    is_last_segment: bool,
    previous_c: u64,
}

struct ZiskMainTraceSegmentWrite {
    trace: Option<WitnessTraceBuffer>,
    trace_source_prefix_rows: usize,
    #[cfg(feature = "cuda")]
    device_segment_material: Option<GuestPcTraceDeviceSegmentMaterial>,
    output: WitnessTraceOutput,
    final_state: ZiskMainTraceState,
    continuation_state: ZiskMainTraceState,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuestPcTraceDeviceSegmentMaterial {
    trace_source_prefix_rows: usize,
    device_trace_descriptors: ZiskMainDeviceTraceDescriptors,
}

#[cfg(feature = "cuda")]
struct GuestPcTraceDeviceSegmentBuild {
    device_segment_material: GuestPcTraceDeviceSegmentMaterial,
    unit_values: Vec<WitnessTraceUnitValue>,
    final_state: ZiskMainTraceState,
    continuation_state: ZiskMainTraceState,
}

#[cfg(feature = "cuda")]
struct ZiskMainStreamingDeviceSegmentBuilder {
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
    context: ZiskMainReportValidationContext<'static>,
    unit_value_summary: ZiskMainSegmentUnitValueSummary,
    device_trace_descriptors: ZiskMainDeviceTraceDescriptors,
    state: ZiskMainTraceState,
    output_row: usize,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Copy)]
struct ZiskMainTraceLowerTimingConfig {
    detail_timing: bool,
    shape_timing: bool,
    detail_sample_stride: usize,
    row_timing_enabled: bool,
}

#[cfg(feature = "cuda")]
impl ZiskMainTraceLowerTimingConfig {
    fn from_env() -> Self {
        let detail_timing = guest_pc_trace_lower_detail_timing_enabled();
        let shape_timing = guest_pc_trace_shape_timing_enabled();
        Self {
            detail_timing,
            shape_timing,
            detail_sample_stride: guest_pc_trace_detail_timing_sample_stride(),
            row_timing_enabled: detail_timing || shape_timing,
        }
    }
}

#[cfg(feature = "cuda")]
struct ZiskMainStreamingDeviceReportFeeder<'a> {
    pending_report: Option<&'a GuestMachineReport>,
    next_report_index: usize,
    timing_config: ZiskMainTraceLowerTimingConfig,
}

#[cfg(feature = "cuda")]
impl<'a> ZiskMainStreamingDeviceReportFeeder<'a> {
    fn new(timing_config: ZiskMainTraceLowerTimingConfig) -> Self {
        Self {
            pending_report: None,
            next_report_index: 0,
            timing_config,
        }
    }

    fn push_report(
        &mut self,
        builder: &mut ZiskMainStreamingDeviceSegmentBuilder,
        report: &'a GuestMachineReport,
        timing: Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<(), GuestPcTraceBackendError> {
        if let Some(pending) = self.pending_report.take() {
            let next_instruction = report.instruction;
            builder.push_report_at(
                self.next_report_index,
                pending,
                || Some(next_instruction),
                self.timing_config,
                timing,
            )?;
            self.next_report_index = self.next_report_index.checked_add(1).ok_or_else(|| {
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace report index overflow".to_owned(),
                }
            })?;
        }
        self.pending_report = Some(report);
        Ok(())
    }

    fn finish(
        mut self,
        builder: &mut ZiskMainStreamingDeviceSegmentBuilder,
        lookahead_instruction: Option<RiscvInstruction>,
        timing: Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<(), GuestPcTraceBackendError> {
        if let Some(pending) = self.pending_report.take() {
            builder.push_report_at(
                self.next_report_index,
                pending,
                || lookahead_instruction,
                self.timing_config,
                timing,
            )?;
        }
        Ok(())
    }
}

#[cfg(feature = "cuda")]
struct ZiskMainOwnedStreamingDeviceReportFeeder {
    pending_report: Option<GuestMachineReport>,
    next_report_index: usize,
    timing_config: ZiskMainTraceLowerTimingConfig,
}

#[cfg(feature = "cuda")]
impl ZiskMainOwnedStreamingDeviceReportFeeder {
    fn new(timing_config: ZiskMainTraceLowerTimingConfig) -> Self {
        Self {
            pending_report: None,
            next_report_index: 0,
            timing_config,
        }
    }

    fn push_report(
        &mut self,
        builder: &mut ZiskMainStreamingDeviceSegmentBuilder,
        report: GuestMachineReport,
        timing: Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<(), GuestPcTraceBackendError> {
        if let Some(pending) = self.pending_report.take() {
            let next_instruction = report.instruction;
            builder.push_report_at(
                self.next_report_index,
                &pending,
                || Some(next_instruction),
                self.timing_config,
                timing,
            )?;
            self.next_report_index = self.next_report_index.checked_add(1).ok_or_else(|| {
                GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "guest PC trace report index overflow".to_owned(),
                }
            })?;
        }
        self.pending_report = Some(report);
        Ok(())
    }

    fn finish(
        mut self,
        builder: &mut ZiskMainStreamingDeviceSegmentBuilder,
        lookahead_instruction: Option<RiscvInstruction>,
        timing: Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<(), GuestPcTraceBackendError> {
        if let Some(pending) = self.pending_report.take() {
            builder.push_report_at(
                self.next_report_index,
                &pending,
                || lookahead_instruction,
                self.timing_config,
                timing,
            )?;
        }
        Ok(())
    }
}

#[cfg(feature = "cuda")]
impl ZiskMainStreamingDeviceSegmentBuilder {
    fn new(
        layout: &WitnessTraceLayout,
        initial_state: &ZiskMainTraceState,
        segment: ZiskMainTraceSegmentInfo,
    ) -> Result<Option<Self>, GuestPcTraceBackendError> {
        let Some(columns) = zisk_main_trace_columns(layout)? else {
            return Ok(None);
        };
        let Some(device_trace_descriptors) =
            main_device_trace_descriptors(layout, &columns, 0, segment)
        else {
            return Ok(None);
        };
        Ok(Some(Self {
            row_count: layout.row_count(),
            segment,
            context: ZiskMainReportValidationContext::new(None, layout.row_count(), segment)?,
            unit_value_summary: ZiskMainSegmentUnitValueSummary::new(),
            device_trace_descriptors,
            state: initial_state.clone(),
            output_row: 0,
        }))
    }

    #[inline(always)]
    fn push_report_at(
        &mut self,
        report_index: usize,
        report: &GuestMachineReport,
        next_instruction: impl FnMut() -> Option<RiscvInstruction>,
        timing_config: ZiskMainTraceLowerTimingConfig,
        mut timing: Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<usize, GuestPcTraceBackendError> {
        let report_detail_timing = timing_config.detail_timing
            && report_index.is_multiple_of(timing_config.detail_sample_stride);
        let report_started = timing
            .as_ref()
            .filter(|_| report_detail_timing)
            .map(|_| Instant::now());
        let descriptor_rows_before = self.device_trace_descriptors.descriptor_rows();
        let pending_report = self.state.pending_dma.is_some();
        let report_apply_started = timing
            .as_ref()
            .filter(|_| timing_config.detail_timing)
            .map(|_| Instant::now());
        let written_rows = validate_and_apply_zisk_main_report(
            self.output_row,
            report,
            next_instruction,
            &mut self.state,
            &mut self.context,
            if timing_config.row_timing_enabled
                && (report_detail_timing || timing_config.shape_timing)
            {
                timing.as_deref_mut()
            } else {
                None
            },
            report_detail_timing,
            timing_config.shape_timing,
            |_, values, mut visit_timing| {
                if timing_config.shape_timing {
                    if let Some(timing) = visit_timing.as_deref_mut() {
                        record_trace_lowered_row_shape(timing, &values.instruction);
                    }
                }
                if report_detail_timing {
                    let _descriptor_timer = DurationTimer::new(
                        visit_timing
                            .as_deref_mut()
                            .map(|timing| &mut timing.trace_descriptor_duration),
                    );
                    append_main_device_trace_descriptor(&mut self.device_trace_descriptors, &values)
                } else {
                    append_main_device_trace_descriptor(&mut self.device_trace_descriptors, &values)
                }
            },
        )?;
        if let (Some(timing), Some(started)) = (timing.as_deref_mut(), report_apply_started) {
            timing.trace_report_apply_duration += started.elapsed();
        }
        let unit_summary_started = timing
            .as_ref()
            .filter(|_| timing_config.detail_timing)
            .map(|_| Instant::now());
        self.unit_value_summary.push_report(report);
        if let (Some(timing), Some(started)) = (timing.as_deref_mut(), unit_summary_started) {
            timing.trace_unit_summary_duration += started.elapsed();
        }
        if let Some(timing) = timing {
            timing.trace_report_count += 1;
            timing.trace_report_row_count += written_rows;
            timing.trace_descriptor_row_count += self
                .device_trace_descriptors
                .descriptor_rows()
                .saturating_sub(descriptor_rows_before);
            if timing_config.shape_timing {
                record_trace_report_shape(timing, report, pending_report, written_rows);
            }
            if let Some(started) = report_started {
                timing.trace_report_detail_sample_count += 1;
                let duration = started.elapsed();
                timing.trace_report_sample_duration += duration;
                record_trace_report_duration(
                    timing,
                    report,
                    pending_report,
                    written_rows,
                    duration,
                );
            }
        }
        self.output_row = self.output_row.checked_add(written_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main row index overflow".to_owned(),
            }
        })?;
        Ok(written_rows)
    }

    fn finish(
        mut self,
        terminal_pc: u64,
        mut timing: Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<GuestPcTraceDeviceSegmentBuild, GuestPcTraceBackendError> {
        self.device_trace_descriptors.terminal_pc = terminal_pc;
        record_trace_descriptor_width_counts(&mut timing, &self.device_trace_descriptors);
        if self.output_row < self.row_count {
            if !self.segment.is_last_segment {
                return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "non-final Zisk Main segment does not fill layout rows".to_owned(),
                });
            }
            validate_zisk_main_halt_pc(self.output_row, &self.state, terminal_pc)?;
        }
        let continuation_state =
            zisk_main_continuation_state(self.row_count, &self.state, self.segment)?;
        let unit_values = self.unit_value_summary.unit_values(
            self.row_count,
            self.output_row,
            terminal_pc,
            &self.state,
            self.segment,
        );
        Ok(GuestPcTraceDeviceSegmentBuild {
            device_segment_material: GuestPcTraceDeviceSegmentMaterial {
                trace_source_prefix_rows: self.output_row,
                device_trace_descriptors: self.device_trace_descriptors,
            },
            unit_values,
            final_state: self.state,
            continuation_state,
        })
    }
}

struct ZiskMainReportTraceValues {
    instruction: ZiskMainInstruction,
    a: u64,
    b: u64,
    c: u64,
    flag: bool,
    register_accesses: ZiskMainRegisterAccessValues,
}

#[derive(Debug, Clone, Copy)]
struct ZiskMainReportEffects<'a> {
    register_writes: &'a [GuestRegisterWrite],
    memory_accesses: &'a [GuestMemoryAccess],
    precompile_memory_accesses: &'a [GuestMemoryAccess],
    precompile_result: Option<u64>,
}

impl<'a> ZiskMainReportEffects<'a> {
    fn empty() -> Self {
        Self {
            register_writes: &[],
            memory_accesses: &[],
            precompile_memory_accesses: &[],
            precompile_result: None,
        }
    }

    fn from_report(report: &'a GuestMachineReport) -> Self {
        Self {
            register_writes: &report.register_writes,
            memory_accesses: &report.memory_accesses,
            precompile_memory_accesses: report.precompile_memory_accesses(),
            precompile_result: report.precompile_result(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ZiskMainSourceValueResult {
    value: u64,
    memory_access_count: u8,
    register_index: Option<u8>,
}

#[derive(Debug, Clone)]
struct ZiskMainLoweredReportRow<'a> {
    instruction: ZiskMainInstruction,
    effects: ZiskMainReportEffects<'a>,
    expected_next_pc: u64,
}

struct ZiskMainReportWindow<'a> {
    current: &'a GuestMachineReport,
    next_instruction: &'a mut dyn FnMut() -> Option<RiscvInstruction>,
}

#[derive(Debug, Clone)]
struct GuestPcTraceRowMemStepCursor {
    segment_base: u64,
    current_row: usize,
    current_base: u64,
}

impl GuestPcTraceRowMemStepCursor {
    fn new(row_count: usize, trace_instance_index: u32) -> Result<Self, GuestPcTraceBackendError> {
        let segment_base = zisk_main_segment_mem_step_base(row_count, trace_instance_index)?;
        Ok(Self {
            segment_base,
            current_row: 0,
            current_base: segment_base,
        })
    }

    fn advance_to(&mut self, row: usize) -> Result<(), GuestPcTraceBackendError> {
        if row == self.current_row {
            return Ok(());
        }
        if row < self.current_row {
            self.current_base =
                zisk_main_row_mem_step_base_from_segment_base(self.segment_base, row)?;
            self.current_row = row;
            return Ok(());
        }
        let delta = row - self.current_row;
        let row_offset = u64::try_from(delta)
            .ok()
            .and_then(|delta| ZISK_MAIN_MEM_STEPS_PER_ROW.checked_mul(delta))
            .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest PC trace step is too large".to_owned(),
            })?;
        self.current_base = self.current_base.checked_add(row_offset).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest PC trace step is too large".to_owned(),
            }
        })?;
        self.current_row = row;
        Ok(())
    }

    fn base(&self) -> u64 {
        self.current_base
    }

    #[cfg(test)]
    fn step(&self, offset: u64) -> Result<u64, GuestPcTraceBackendError> {
        zisk_main_mem_step_from_base(self.current_base, offset)
    }
}

#[derive(Debug, Clone)]
struct ZiskMainReportValidationContext<'a> {
    columns: Option<&'a ZiskMainTraceColumns<'a>>,
    row_count: usize,
    row_mem_step_cursor: GuestPcTraceRowMemStepCursor,
}

impl<'a> ZiskMainReportValidationContext<'a> {
    fn new(
        columns: Option<&'a ZiskMainTraceColumns<'a>>,
        row_count: usize,
        segment: ZiskMainTraceSegmentInfo,
    ) -> Result<Self, GuestPcTraceBackendError> {
        Ok(Self {
            columns,
            row_count,
            row_mem_step_cursor: GuestPcTraceRowMemStepCursor::new(
                row_count,
                segment.trace_instance_index,
            )?,
        })
    }

    fn row_mem_step_base(&mut self, row: usize) -> Result<u64, GuestPcTraceBackendError> {
        self.row_mem_step_cursor.advance_to(row)?;
        Ok(self.row_mem_step_cursor.base())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZiskMainRegisterAccessValues {
    a_prev_mem_step: Option<u64>,
    b_prev_mem_step: Option<u64>,
    store_prev_mem_step: Option<u64>,
    store_prev_value: Option<u64>,
}

#[allow(clippy::too_many_arguments)]
fn validate_and_apply_zisk_main_report(
    row: usize,
    report: &GuestMachineReport,
    mut next_instruction: impl FnMut() -> Option<RiscvInstruction>,
    state: &mut ZiskMainTraceState,
    context: &mut ZiskMainReportValidationContext<'_>,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
    detail_timing: bool,
    shape_timing: bool,
    mut visit: impl FnMut(
        usize,
        ZiskMainReportTraceValues,
        Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<(), GuestPcTraceBackendError>,
) -> Result<usize, GuestPcTraceBackendError> {
    if let Some(pending) = state.pending_dma {
        validate_zisk_main_report_row_capacity(row, 1, context.row_count)?;
        let lowering_started = detail_duration_started(&timing, detail_timing);
        let lowered_row = ZiskMainLoweredReportRow {
            instruction: lower_pending_dma_report(row, report, pending)?,
            effects: ZiskMainReportEffects::from_report(report),
            expected_next_pc: report.next_pc,
        };
        record_detail_duration(lowering_started, &mut timing, |timing| {
            &mut timing.trace_report_lowering_duration
        });
        apply_zisk_main_lowered_report_row(
            row,
            report,
            lowered_row,
            state,
            context,
            timing,
            detail_timing,
            shape_timing,
            &mut visit,
        )?;
        state.pending_dma = None;
        return Ok(1);
    }

    if let RiscvInstruction::StoreConditional {
        width,
        rd,
        rs1,
        rs2,
        ..
    } = report.instruction
    {
        let lowering_started = detail_duration_started(&timing, detail_timing);
        let lowered = lower_store_conditional_report_rows(row, report, width, rd, rs1, rs2)?;
        record_detail_duration(lowering_started, &mut timing, |timing| {
            &mut timing.trace_report_lowering_duration
        });
        let produced_rows = validate_and_apply_zisk_main_lowered_report_rows(
            row,
            report,
            lowered,
            state,
            context,
            timing,
            detail_timing,
            shape_timing,
            visit,
        )?;
        state.pending_dma = zisk_main_pending_dma(report);
        return Ok(produced_rows);
    }

    if let RiscvInstruction::Amo {
        kind,
        width,
        rd,
        rs1,
        rs2,
        ..
    } = report.instruction
    {
        let lowering_started = detail_duration_started(&timing, detail_timing);
        let lowered = lower_amo_report_rows(row, report, kind, width, rd, rs1, rs2)?;
        record_detail_duration(lowering_started, &mut timing, |timing| {
            &mut timing.trace_report_lowering_duration
        });
        let produced_rows = validate_and_apply_zisk_main_lowered_report_rows(
            row,
            report,
            lowered,
            state,
            context,
            timing,
            detail_timing,
            shape_timing,
            visit,
        )?;
        state.pending_dma = zisk_main_pending_dma(report);
        return Ok(produced_rows);
    }

    validate_zisk_main_report_row_capacity(row, 1, context.row_count)?;
    let lowering_started = detail_duration_started(&timing, detail_timing);
    let lowered_row = lower_single_zisk_main_report_row(row, report, &mut next_instruction)?;
    record_detail_duration(lowering_started, &mut timing, |timing| {
        &mut timing.trace_report_lowering_duration
    });
    apply_zisk_main_lowered_report_row(
        row,
        report,
        lowered_row,
        state,
        context,
        timing,
        detail_timing,
        shape_timing,
        &mut visit,
    )?;
    state.pending_dma = zisk_main_pending_dma(report);
    Ok(1)
}

#[allow(clippy::too_many_arguments)]
fn validate_and_apply_zisk_main_lowered_report_rows(
    row: usize,
    report: &GuestMachineReport,
    lowered: Vec<ZiskMainLoweredReportRow<'_>>,
    state: &mut ZiskMainTraceState,
    context: &mut ZiskMainReportValidationContext<'_>,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
    detail_timing: bool,
    shape_timing: bool,
    mut visit: impl FnMut(
        usize,
        ZiskMainReportTraceValues,
        Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<(), GuestPcTraceBackendError>,
) -> Result<usize, GuestPcTraceBackendError> {
    let exclusive_end = row.checked_add(lowered.len()).ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main row index overflow".to_owned(),
        }
    })?;
    if exclusive_end > context.row_count {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main report rows exceed layout rows".to_owned(),
        });
    }
    let produced_rows = lowered.len();
    for (offset, lowered_row) in lowered.into_iter().enumerate() {
        let output_row = row.checked_add(offset).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main row index overflow".to_owned(),
            }
        })?;
        apply_zisk_main_lowered_report_row(
            output_row,
            report,
            lowered_row,
            state,
            &mut *context,
            reborrow_trace_timing(&mut timing),
            detail_timing,
            shape_timing,
            &mut visit,
        )?;
    }
    Ok(produced_rows)
}

fn validate_zisk_main_report_row_capacity(
    row: usize,
    produced_rows: usize,
    row_count: usize,
) -> Result<(), GuestPcTraceBackendError> {
    let exclusive_end = row.checked_add(produced_rows).ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main row index overflow".to_owned(),
        }
    })?;
    if exclusive_end > row_count {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main report rows exceed layout rows".to_owned(),
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_zisk_main_lowered_report_row(
    output_row: usize,
    report: &GuestMachineReport,
    lowered_row: ZiskMainLoweredReportRow<'_>,
    state: &mut ZiskMainTraceState,
    context: &mut ZiskMainReportValidationContext<'_>,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
    detail_timing: bool,
    shape_timing: bool,
    visit: &mut impl FnMut(
        usize,
        ZiskMainReportTraceValues,
        Option<&mut GuestPcTraceStreamTiming>,
    ) -> Result<(), GuestPcTraceBackendError>,
) -> Result<(), GuestPcTraceBackendError> {
    let shape_started = timing
        .as_ref()
        .filter(|_| shape_timing)
        .map(|_| Instant::now());
    let validation_started = detail_duration_started(&timing, detail_timing);
    let instruction = lowered_row.instruction;
    let is_external_op = instruction.is_external_op;
    let is_copy = matches!(instruction.op, ZiskMainOp::CopyB);
    if let Some(columns) = context.columns {
        let memory_columns_started = detail_duration_started(&timing, detail_timing);
        validate_zisk_main_memory_columns(output_row, &instruction, columns)?;
        record_detail_duration(memory_columns_started, &mut timing, |timing| {
            &mut timing.trace_report_memory_columns_duration
        });
    }
    let source_values_started = detail_duration_started(&timing, detail_timing);
    let source_a_value_started = detail_duration_started(&timing, detail_timing);
    let a_value = zisk_main_source_value(
        output_row,
        instruction.a,
        state,
        report,
        lowered_row.effects,
        None,
        0,
        0,
    )?;
    let a = a_value.value;
    let source_a_record_started = detail_duration_started(&timing, detail_timing);
    record_trace_report_source_value_duration(
        source_a_value_started,
        &mut timing,
        instruction.a,
        is_copy,
        |timing| &mut timing.trace_report_source_a_value_duration,
    );
    record_detail_duration(source_a_record_started, &mut timing, |timing| {
        &mut timing.trace_report_source_value_record_duration
    });
    let source_b_value_started = detail_duration_started(&timing, detail_timing);
    let b_memory_access_index = usize::from(a_value.memory_access_count);
    let b_value = zisk_main_source_value(
        output_row,
        instruction.b,
        state,
        report,
        lowered_row.effects,
        Some(a),
        instruction.ind_width,
        b_memory_access_index,
    )?;
    let b = b_value.value;
    let source_b_record_started = detail_duration_started(&timing, detail_timing);
    record_trace_report_source_value_duration(
        source_b_value_started,
        &mut timing,
        instruction.b,
        is_copy,
        |timing| &mut timing.trace_report_source_b_value_duration,
    );
    record_detail_duration(source_b_record_started, &mut timing, |timing| {
        &mut timing.trace_report_source_value_record_duration
    });
    record_detail_duration(source_values_started, &mut timing, |timing| {
        &mut timing.trace_report_source_values_duration
    });

    let precompile_memory_started = detail_duration_started(&timing, detail_timing);
    if zisk_main_precompile_memory_validation_required(&instruction, lowered_row.effects) {
        validate_zisk_main_precompile_memory_accesses(output_row, report, lowered_row.effects, b)?;
    }
    record_detail_duration(precompile_memory_started, &mut timing, |timing| {
        &mut timing.trace_report_precompile_memory_duration
    });

    let instruction_result_started = detail_duration_started(&timing, detail_timing);
    let (c, flag) =
        zisk_main_instruction_result(output_row, &instruction, a, b, lowered_row.effects)?;
    record_detail_duration(instruction_result_started, &mut timing, |timing| {
        &mut timing.trace_report_instruction_result_duration
    });

    let next_pc_started = detail_duration_started(&timing, detail_timing);
    validate_zisk_main_next_pc(
        output_row,
        &instruction,
        lowered_row.expected_next_pc,
        c,
        flag,
    )?;
    record_detail_duration(next_pc_started, &mut timing, |timing| {
        &mut timing.trace_report_next_pc_duration
    });

    let register_access_started = detail_duration_started(&timing, detail_timing);
    let row_mem_step_base = context.row_mem_step_base(output_row)?;
    let register_accesses = apply_zisk_main_register_access_values(
        output_row,
        &instruction,
        state,
        row_mem_step_base,
        a_value.register_index,
        b_value.register_index,
    )?;
    record_detail_duration(register_access_started, &mut timing, |timing| {
        &mut timing.trace_report_register_access_duration
    });

    let memory_access_started = detail_duration_started(&timing, detail_timing);
    let validated_source_access_count =
        b_memory_access_index + usize::from(b_value.memory_access_count);
    validate_zisk_main_memory_accesses_after_source_values(
        output_row,
        &instruction,
        lowered_row.effects,
        a,
        c,
        validated_source_access_count,
    )?;
    record_detail_duration(memory_access_started, &mut timing, |timing| {
        &mut timing.trace_report_memory_access_duration
    });

    let store_apply_started = detail_duration_started(&timing, detail_timing);
    apply_zisk_main_store(
        output_row,
        &instruction,
        c,
        lowered_row.effects,
        lowered_row.expected_next_pc,
        state,
    )?;
    record_detail_duration(store_apply_started, &mut timing, |timing| {
        &mut timing.trace_report_store_apply_duration
    });
    record_detail_duration(validation_started, &mut timing, |timing| {
        &mut timing.trace_report_row_validation_duration
    });
    let values = ZiskMainReportTraceValues {
        instruction,
        a,
        b,
        c,
        flag,
        register_accesses,
    };
    let visit_started = detail_duration_started(&timing, detail_timing);
    let result = visit(output_row, values, timing.as_deref_mut());
    record_detail_duration(visit_started, &mut timing, |timing| {
        &mut timing.trace_report_visit_duration
    });
    if let (Some(timing), Some(started)) = (timing, shape_started) {
        record_trace_lowered_row_duration(timing, is_external_op, is_copy, started.elapsed());
    }
    result
}

fn record_trace_report_shape(
    timing: &mut GuestPcTraceStreamTiming,
    report: &GuestMachineReport,
    pending_report: bool,
    written_rows: usize,
) {
    if written_rows <= 1 {
        timing.trace_single_row_report_count += 1;
    } else {
        timing.trace_multi_row_report_count += 1;
    }
    if pending_report {
        timing.trace_pending_dma_report_count += 1;
    }
    match report.instruction {
        RiscvInstruction::Amo { .. } => {
            timing.trace_amo_report_count += 1;
        }
        RiscvInstruction::StoreConditional { .. } => {
            timing.trace_store_conditional_report_count += 1;
        }
        _ => {}
    }
}

fn record_trace_report_duration(
    timing: &mut GuestPcTraceStreamTiming,
    report: &GuestMachineReport,
    pending_report: bool,
    written_rows: usize,
    duration: Duration,
) {
    if written_rows <= 1 {
        timing.trace_single_row_report_duration += duration;
    } else {
        timing.trace_multi_row_report_duration += duration;
    }
    if pending_report {
        timing.trace_pending_dma_report_duration += duration;
    }
    match report.instruction {
        RiscvInstruction::Amo { .. } => {
            timing.trace_amo_report_duration += duration;
        }
        RiscvInstruction::StoreConditional { .. } => {
            timing.trace_store_conditional_report_duration += duration;
        }
        _ => {}
    }
}

fn record_aggregate_trace_report_duration(
    timing: &mut Option<&mut GuestPcTraceStreamTiming>,
    started: Option<Instant>,
) {
    if let (Some(timing), Some(started)) = (timing.as_mut(), started) {
        let timing = &mut **timing;
        timing.trace_report_duration += started.elapsed();
    }
}

#[cfg(feature = "cuda")]
fn record_trace_descriptor_width_counts(
    timing: &mut Option<&mut GuestPcTraceStreamTiming>,
    descriptors: &ZiskMainDeviceTraceDescriptors,
) {
    let Some(timing) = timing.as_mut() else {
        return;
    };
    let timing = &mut **timing;
    match descriptors.descriptor_word_count() {
        ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS
        | ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS => {
            timing.trace_descriptor_compact_row_count += descriptors.descriptor_rows();
        }
        ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS => {
            timing.trace_descriptor_wide_row_count += descriptors.descriptor_rows();
        }
        _ => {}
    }
    timing.trace_descriptor_unpaired_value_count += descriptors.unpaired_value_count();
    timing.trace_descriptor_unpaired_high32_nonzero_count +=
        descriptors.unpaired_high32_nonzero_count();
    timing.trace_descriptor_unpaired_high32_nonzero_row_count +=
        descriptors.unpaired_high32_nonzero_row_count();
    for (field_count, descriptor_count) in timing
        .trace_descriptor_high32_field_counts
        .iter_mut()
        .zip(descriptors.unpaired_high32_nonzero_field_counts())
    {
        *field_count += descriptor_count;
    }
    for (bucket_count, descriptor_count) in timing
        .trace_descriptor_high32_row_field_histogram
        .iter_mut()
        .zip(descriptors.unpaired_high32_nonzero_row_field_histogram())
    {
        *bucket_count += descriptor_count;
    }
}

fn record_trace_lowered_row_shape(
    timing: &mut GuestPcTraceStreamTiming,
    instruction: &ZiskMainInstruction,
) {
    timing.record_trace_row_shape_pattern(zisk_main_row_shape_pattern_id(instruction));
    let (register_a_sources, memory_a_sources) = source_shape_count(instruction.a);
    let (register_b_sources, memory_b_sources) = source_shape_count(instruction.b);
    let memory_source_count = memory_a_sources + memory_b_sources;
    timing.trace_register_source_read_count += register_a_sources + register_b_sources;
    timing.trace_memory_source_read_count += memory_source_count;
    if instruction.is_external_op {
        timing.trace_external_op_row_count += 1;
    }
    record_trace_shape_run(
        instruction.is_external_op,
        &mut timing.trace_external_op_run_count,
        &mut timing.trace_external_op_current_run_count,
        &mut timing.trace_external_op_max_run_count,
    );
    let is_copy = matches!(instruction.op, ZiskMainOp::CopyB);
    record_trace_shape_run(
        is_copy,
        &mut timing.trace_copy_run_count,
        &mut timing.trace_copy_current_run_count,
        &mut timing.trace_copy_max_run_count,
    );
    let uses_indirect_memory_row = matches!(instruction.b, ZiskMainSource::Indirect(_))
        || matches!(instruction.store, ZiskMainStore::Indirect(_));
    match instruction.op {
        ZiskMainOp::CopyB => {
            timing.trace_copy_row_count += 1;
            if memory_source_count > 0 {
                timing.trace_copy_memory_source_row_count += 1;
            }
            if uses_indirect_memory_row {
                timing.trace_copy_indirect_memory_row_count += 1;
            }
            match instruction.store {
                ZiskMainStore::Register(_) => timing.trace_copy_register_store_row_count += 1,
                ZiskMainStore::Memory(_) | ZiskMainStore::Indirect(_) => {
                    timing.trace_copy_memory_store_row_count += 1;
                }
                ZiskMainStore::None => timing.trace_copy_no_store_row_count += 1,
            }
            if memory_source_count == 0
                && !matches!(
                    instruction.store,
                    ZiskMainStore::Memory(_) | ZiskMainStore::Indirect(_)
                )
            {
                timing.trace_copy_no_memory_row_count += 1;
            }
        }
        ZiskMainOp::Flag => timing.trace_flag_row_count += 1,
        _ => {}
    }
    if instruction.is_precompiled {
        timing.trace_precompile_row_count += 1;
    }
    if uses_indirect_memory_row {
        timing.trace_indirect_memory_row_count += 1;
    }
    match instruction.store {
        ZiskMainStore::Register(_) => timing.trace_register_store_row_count += 1,
        ZiskMainStore::Memory(_) | ZiskMainStore::Indirect(_) => {
            timing.trace_memory_store_row_count += 1;
        }
        ZiskMainStore::None => timing.trace_no_store_row_count += 1,
    }
}

fn record_trace_shape_run(
    is_active: bool,
    run_count: &mut usize,
    current_run_count: &mut usize,
    max_run_count: &mut usize,
) {
    if is_active {
        if *current_run_count == 0 {
            *run_count += 1;
        }
        *current_run_count += 1;
        *max_run_count = (*max_run_count).max(*current_run_count);
    } else {
        *current_run_count = 0;
    }
}

fn record_trace_lowered_row_duration(
    timing: &mut GuestPcTraceStreamTiming,
    is_external_op: bool,
    is_copy: bool,
    duration: Duration,
) {
    if is_external_op {
        timing.trace_external_op_row_duration += duration;
    }
    if is_copy {
        timing.trace_copy_row_duration += duration;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceSourceKind {
    Immediate,
    Register,
    Memory,
    Indirect,
    LastC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceStoreKind {
    None,
    Register,
    Memory,
    Indirect,
}

fn trace_source_kind(source: ZiskMainSource) -> TraceSourceKind {
    match source {
        ZiskMainSource::Immediate(_) => TraceSourceKind::Immediate,
        ZiskMainSource::Register(_) => TraceSourceKind::Register,
        ZiskMainSource::Memory(_) => TraceSourceKind::Memory,
        ZiskMainSource::Indirect(_) => TraceSourceKind::Indirect,
        ZiskMainSource::LastC => TraceSourceKind::LastC,
    }
}

fn trace_source_kind_code(source: ZiskMainSource) -> u64 {
    match trace_source_kind(source) {
        TraceSourceKind::Immediate => 0,
        TraceSourceKind::Register => 1,
        TraceSourceKind::Memory => 2,
        TraceSourceKind::Indirect => 3,
        TraceSourceKind::LastC => 4,
    }
}

fn trace_store_kind(store: &ZiskMainStore) -> TraceStoreKind {
    match store {
        ZiskMainStore::None => TraceStoreKind::None,
        ZiskMainStore::Register(_) => TraceStoreKind::Register,
        ZiskMainStore::Memory(_) => TraceStoreKind::Memory,
        ZiskMainStore::Indirect(_) => TraceStoreKind::Indirect,
    }
}

fn trace_store_kind_code(store: &ZiskMainStore) -> u64 {
    match trace_store_kind(store) {
        TraceStoreKind::None => 0,
        TraceStoreKind::Register => 1,
        TraceStoreKind::Memory => 2,
        TraceStoreKind::Indirect => 3,
    }
}

fn zisk_main_row_shape_pattern_id(instruction: &ZiskMainInstruction) -> u64 {
    1 | (u64::from(instruction.op.code()) << 1)
        | (trace_source_kind_code(instruction.a) << 9)
        | (trace_source_kind_code(instruction.b) << 12)
        | (trace_store_kind_code(&instruction.store) << 15)
        | ((instruction.ind_width & 0xff) << 17)
        | (u64::from(instruction.store_pc) << 25)
        | (u64::from(instruction.set_pc) << 26)
        | (u64::from(instruction.m32) << 27)
        | (u64::from(instruction.is_external_op) << 28)
        | (u64::from(instruction.is_precompiled) << 29)
}

fn record_trace_report_source_read_timing(
    timing: &mut GuestPcTraceStreamTiming,
    source: ZiskMainSource,
    duration: Duration,
    is_copy: bool,
) {
    match trace_source_kind(source) {
        TraceSourceKind::Immediate => {
            timing.trace_report_source_immediate_read_count += 1;
            timing.trace_report_source_immediate_read_duration += duration;
        }
        TraceSourceKind::Register => {
            timing.trace_report_source_register_read_count += 1;
            timing.trace_report_source_register_read_duration += duration;
        }
        TraceSourceKind::Memory => {
            timing.trace_report_source_memory_read_count += 1;
            timing.trace_report_source_memory_read_duration += duration;
            if is_copy {
                timing.trace_copy_source_memory_read_count += 1;
                timing.trace_copy_source_memory_read_duration += duration;
            }
        }
        TraceSourceKind::Indirect => {
            timing.trace_report_source_indirect_read_count += 1;
            timing.trace_report_source_indirect_read_duration += duration;
            if is_copy {
                timing.trace_copy_source_indirect_read_count += 1;
                timing.trace_copy_source_indirect_read_duration += duration;
            }
        }
        TraceSourceKind::LastC => {
            timing.trace_report_source_last_c_read_count += 1;
            timing.trace_report_source_last_c_read_duration += duration;
        }
    }
}

fn record_trace_report_source_value_duration(
    started: Option<Instant>,
    timing: &mut Option<&mut GuestPcTraceStreamTiming>,
    source: ZiskMainSource,
    is_copy: bool,
    target: fn(&mut GuestPcTraceStreamTiming) -> &mut Duration,
) {
    if let (Some(timing), Some(started)) = (timing.as_mut(), started) {
        let timing = &mut **timing;
        let duration = started.elapsed();
        *target(timing) += duration;
        record_trace_report_source_read_timing(timing, source, duration, is_copy);
    }
}

fn source_shape_count(source: ZiskMainSource) -> (usize, usize) {
    match source {
        ZiskMainSource::Register(_) => (1, 0),
        ZiskMainSource::Memory(_) | ZiskMainSource::Indirect(_) => (0, 1),
        ZiskMainSource::LastC | ZiskMainSource::Immediate(_) => (0, 0),
    }
}

fn lower_single_zisk_main_report_row<'a>(
    row: usize,
    report: &'a GuestMachineReport,
    mut next_instruction: impl FnMut() -> Option<RiscvInstruction>,
) -> Result<ZiskMainLoweredReportRow<'a>, GuestPcTraceBackendError> {
    let instruction = lower_guest_report(report)
        .map_err(|source| GuestPcTraceBackendError::ZiskMainLower { row, source })?;
    let instruction = if let RiscvInstruction::ZiskDmaPrepare { kind, .. } = report.instruction {
        lower_dma_prepare_report(row, instruction, kind, next_instruction())?
    } else {
        instruction
    };
    Ok(ZiskMainLoweredReportRow {
        instruction,
        effects: ZiskMainReportEffects::from_report(report),
        expected_next_pc: report.next_pc,
    })
}

fn lower_amo_report_rows(
    row: usize,
    report: &GuestMachineReport,
    kind: RiscvAmoKind,
    width: RiscvAmoWidth,
    rd: u8,
    rs1: u8,
    rs2: u8,
) -> Result<Vec<ZiskMainLoweredReportRow<'_>>, GuestPcTraceBackendError> {
    if kind != RiscvAmoKind::Add {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!("unsupported AMO operation {kind:?}"),
        });
    }
    let [read_access, write_access] = report.memory_accesses.as_slice() else {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "AMO row reported {} memory accesses",
                report.memory_accesses.len()
            ),
        });
    };
    if read_access.kind != GuestMemoryAccessKind::Read
        || write_access.kind != GuestMemoryAccessKind::Write
    {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: "AMO row must report one memory read followed by one memory write".to_owned(),
        });
    }

    let (load_op, ind_width) = amo_load_op_width(width);
    let compute_op = amo_add_op(width);
    let _ = zisk_main_report_instruction_size(row, report)?;
    let load_pc = report.address;
    let compute_pc = report.address.checked_add(1).ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "AMO compute pc overflow".to_owned(),
        }
    })?;
    let store_pc = report.address.checked_add(2).ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "AMO store pc overflow".to_owned(),
        }
    })?;
    let aliases_result = rd != 0 && (rd == rs1 || rd == rs2);
    let register_pc = report.address.checked_add(3).ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "AMO register pc overflow".to_owned(),
        }
    })?;
    let store_jump = if aliases_result {
        1
    } else {
        zisk_main_pc_delta(row, store_pc, report.next_pc)?
    };

    let mut load_row = zisk_main_base_instruction(
        load_pc,
        zisk_main_register_source(rs1),
        ZiskMainSource::Indirect(0),
        load_op,
        if aliases_result {
            ZiskMainStore::Memory(zisk_internal_register_address(ZISK_AMO_TEMP_REGISTER)?)
        } else {
            zisk_main_register_store(rd)
        },
        1,
    );
    load_row.ind_width = ind_width;
    let compute_row = zisk_main_base_instruction(
        compute_pc,
        ZiskMainSource::LastC,
        zisk_main_register_source(rs2),
        compute_op,
        ZiskMainStore::None,
        1,
    );
    let mut store_row = zisk_main_base_instruction(
        store_pc,
        zisk_main_register_source(rs1),
        ZiskMainSource::LastC,
        ZiskMainOp::CopyB,
        ZiskMainStore::Indirect(0),
        store_jump,
    );
    store_row.ind_width = ind_width;

    let mut load_effects = ZiskMainReportEffects::empty();
    load_effects.memory_accesses = &report.memory_accesses[..1];
    let compute_effects = ZiskMainReportEffects::empty();
    let mut store_effects = ZiskMainReportEffects::empty();
    store_effects.memory_accesses = &report.memory_accesses[1..2];

    if aliases_result {
        let register_jump = zisk_main_pc_delta(row, register_pc, report.next_pc)?;
        let register_row = zisk_main_base_instruction(
            register_pc,
            ZiskMainSource::Immediate(0),
            ZiskMainSource::Memory(zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER)),
            ZiskMainOp::CopyB,
            zisk_main_register_store(rd),
            register_jump,
        );
        let mut register_effects = ZiskMainReportEffects::empty();
        register_effects.register_writes = &report.register_writes;
        return Ok(vec![
            ZiskMainLoweredReportRow {
                instruction: load_row,
                effects: load_effects,
                expected_next_pc: compute_pc,
            },
            ZiskMainLoweredReportRow {
                instruction: compute_row,
                effects: compute_effects,
                expected_next_pc: store_pc,
            },
            ZiskMainLoweredReportRow {
                instruction: store_row,
                effects: store_effects,
                expected_next_pc: register_pc,
            },
            ZiskMainLoweredReportRow {
                instruction: register_row,
                effects: register_effects,
                expected_next_pc: report.next_pc,
            },
        ]);
    }

    load_effects.register_writes = &report.register_writes;

    Ok(vec![
        ZiskMainLoweredReportRow {
            instruction: load_row,
            effects: load_effects,
            expected_next_pc: compute_pc,
        },
        ZiskMainLoweredReportRow {
            instruction: compute_row,
            effects: compute_effects,
            expected_next_pc: store_pc,
        },
        ZiskMainLoweredReportRow {
            instruction: store_row,
            effects: store_effects,
            expected_next_pc: report.next_pc,
        },
    ])
}

fn amo_add_op(width: RiscvAmoWidth) -> ZiskMainOp {
    match width {
        RiscvAmoWidth::Word => ZiskMainOp::AddW,
        RiscvAmoWidth::Doubleword => ZiskMainOp::Add,
    }
}

fn amo_load_op_width(width: RiscvAmoWidth) -> (ZiskMainOp, u64) {
    match width {
        RiscvAmoWidth::Word => (ZiskMainOp::SignExtendW, 4),
        RiscvAmoWidth::Doubleword => (ZiskMainOp::CopyB, 8),
    }
}

fn zisk_internal_register_address(index: u8) -> Result<i64, GuestPcTraceBackendError> {
    i64::try_from(zisk_internal_register_address_u64(index)).map_err(|_| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk internal register address is out of range".to_owned(),
        }
    })
}

fn zisk_internal_register_address_u64(index: u8) -> u64 {
    ZISK_RAM_ADDRESS + u64::from(index) * 8
}

fn zisk_internal_memory_address(address: u64) -> bool {
    address == ZISK_EXTRA_PARAMS_ADDRESS
        || address == zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER)
}

fn lower_store_conditional_report_rows(
    row: usize,
    report: &GuestMachineReport,
    width: RiscvAmoWidth,
    rd: u8,
    rs1: u8,
    rs2: u8,
) -> Result<Vec<ZiskMainLoweredReportRow<'_>>, GuestPcTraceBackendError> {
    if !report
        .memory_accesses
        .iter()
        .any(|access| access.kind == GuestMemoryAccessKind::Write)
    {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message:
                "StoreConditional without a memory write is not supported by Zisk Main lowering"
                    .to_owned(),
        });
    }
    let instruction_size = zisk_main_report_instruction_size(row, report)?;
    let ind_width = store_conditional_width_bytes(width);
    let mut store_row = zisk_main_base_instruction(
        report.address,
        zisk_main_register_source(rs1),
        zisk_main_register_source(rs2),
        ZiskMainOp::CopyB,
        ZiskMainStore::Indirect(0),
        instruction_size,
    );
    store_row.ind_width = ind_width;

    let mut memory_effects = ZiskMainReportEffects::empty();
    memory_effects.memory_accesses = &report.memory_accesses;
    if rd == 0 {
        return Ok(vec![ZiskMainLoweredReportRow {
            instruction: store_row,
            effects: memory_effects,
            expected_next_pc: report.next_pc,
        }]);
    }

    let register_pc = report.address.checked_add(1).ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "StoreConditional internal pc overflow".to_owned(),
        }
    })?;
    store_row.jmp_offset1 = 1;
    store_row.jmp_offset2 = 1;
    let register_jump = zisk_main_pc_delta(row, register_pc, report.next_pc)?;
    let register_row = zisk_main_base_instruction(
        register_pc,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Immediate(0),
        ZiskMainOp::CopyB,
        zisk_main_register_store(rd),
        register_jump,
    );
    let mut register_effects = ZiskMainReportEffects::empty();
    register_effects.register_writes = &report.register_writes;
    Ok(vec![
        ZiskMainLoweredReportRow {
            instruction: store_row,
            effects: memory_effects,
            expected_next_pc: register_pc,
        },
        ZiskMainLoweredReportRow {
            instruction: register_row,
            effects: register_effects,
            expected_next_pc: report.next_pc,
        },
    ])
}

fn store_conditional_width_bytes(width: RiscvAmoWidth) -> u64 {
    match width {
        RiscvAmoWidth::Word => 4,
        RiscvAmoWidth::Doubleword => 8,
    }
}

fn zisk_main_pc_delta(row: usize, from: u64, to: u64) -> Result<i64, GuestPcTraceBackendError> {
    let delta = i128::from(to) - i128::from(from);
    i64::try_from(delta).map_err(|_| GuestPcTraceBackendError::ZiskMainEffectMismatch {
        row,
        message: format!("Zisk Main pc delta from {from} to {to} is out of range"),
    })
}

fn lower_dma_prepare_report(
    row: usize,
    mut instruction: ZiskMainInstruction,
    kind: RiscvDmaKind,
    next_instruction: Option<RiscvInstruction>,
) -> Result<ZiskMainInstruction, GuestPcTraceBackendError> {
    if matches!(kind, RiscvDmaKind::Memcpy | RiscvDmaKind::Memcmp) {
        if let Some(RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rs2,
            ..
        }) = next_instruction
        {
            instruction.a = ZiskMainSource::Immediate(0);
            instruction.b = zisk_main_register_source(rs2);
            instruction.store = ZiskMainStore::Memory(ZISK_EXTRA_PARAMS_ADDRESS as i64);
            instruction.jmp_offset1 = 0;
            return Ok(instruction);
        }
    }
    if next_instruction.is_none() {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: "DMA prepare row is missing the following execute row".to_owned(),
        });
    }
    Ok(instruction)
}

fn lower_pending_dma_report(
    row: usize,
    report: &GuestMachineReport,
    pending: ZiskMainPendingDma,
) -> Result<ZiskMainInstruction, GuestPcTraceBackendError> {
    let instruction_size = zisk_main_report_instruction_size(row, report)?;
    match report.instruction {
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd,
            rs1,
            rs2,
        } => Ok(lower_pending_dma_add(
            report.address,
            instruction_size,
            pending,
            rd,
            rs1,
            rs2,
        )),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd,
            rs1,
            immediate,
        } => Ok(lower_pending_dma_addi(
            report.address,
            instruction_size,
            pending,
            rd,
            rs1,
            immediate,
        )),
        instruction => Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!("DMA prepare is followed by unsupported instruction {instruction:?}"),
        }),
    }
}

fn lower_pending_dma_add(
    pc: u64,
    instruction_size: i64,
    pending: ZiskMainPendingDma,
    rd: u8,
    rs1: u8,
    rs2: u8,
) -> ZiskMainInstruction {
    let mut instruction = zisk_main_base_instruction(
        pc,
        zisk_main_register_source(rs1),
        dma_register_b_source(pending, rs2),
        dma_register_op(pending.kind),
        zisk_main_register_store(rd),
        instruction_size,
    );
    if pending.kind == RiscvDmaKind::Memset {
        instruction.jmp_offset1 = 0;
    }
    instruction
}

fn lower_pending_dma_addi(
    pc: u64,
    instruction_size: i64,
    pending: ZiskMainPendingDma,
    rd: u8,
    rs1: u8,
    immediate: i64,
) -> ZiskMainInstruction {
    let (a, b, op, jmp_offset1) = match pending.kind {
        RiscvDmaKind::Memcpy => (
            zisk_main_register_source(rs1),
            zisk_main_register_source(pending.first_arg_reg),
            ZiskMainOp::DmaXMemCpy,
            immediate,
        ),
        RiscvDmaKind::Memcmp => (
            zisk_main_register_source(rs1),
            zisk_main_register_source(pending.first_arg_reg),
            ZiskMainOp::DmaXMemCmp,
            immediate,
        ),
        RiscvDmaKind::Inputcpy => (
            zisk_main_register_source(rs1),
            ZiskMainSource::Immediate(immediate as u64),
            ZiskMainOp::DmaInputCpy,
            instruction_size,
        ),
        RiscvDmaKind::Memset => (
            zisk_main_register_source(pending.first_arg_reg),
            zisk_main_register_source(rs1),
            ZiskMainOp::DmaXMemSet,
            i64::from(immediate as u8),
        ),
    };
    let mut instruction =
        zisk_main_base_instruction(pc, a, b, op, zisk_main_register_store(rd), instruction_size);
    instruction.jmp_offset1 = jmp_offset1;
    instruction
}

fn dma_register_b_source(pending: ZiskMainPendingDma, count_reg: u8) -> ZiskMainSource {
    match pending.kind {
        RiscvDmaKind::Memcpy | RiscvDmaKind::Memcmp => {
            zisk_main_register_source(pending.first_arg_reg)
        }
        RiscvDmaKind::Inputcpy | RiscvDmaKind::Memset => zisk_main_register_source(count_reg),
    }
}

fn dma_register_op(kind: RiscvDmaKind) -> ZiskMainOp {
    match kind {
        RiscvDmaKind::Memcpy => ZiskMainOp::DmaMemCpy,
        RiscvDmaKind::Memcmp => ZiskMainOp::DmaMemCmp,
        RiscvDmaKind::Inputcpy => ZiskMainOp::DmaInputCpy,
        RiscvDmaKind::Memset => ZiskMainOp::DmaXMemSet,
    }
}

fn guest_report_next_instruction(
    reports: &[GuestMachineReport],
    report_index: usize,
    lookahead_instruction: Option<RiscvInstruction>,
) -> Option<RiscvInstruction> {
    reports
        .get(report_index + 1)
        .map(|next| next.instruction)
        .or_else(|| {
            (report_index + 1 == reports.len())
                .then_some(lookahead_instruction)
                .flatten()
        })
}

fn zisk_main_pending_dma(report: &GuestMachineReport) -> Option<ZiskMainPendingDma> {
    match report.instruction {
        RiscvInstruction::ZiskDmaPrepare { kind, rs1 } => Some(ZiskMainPendingDma {
            kind,
            first_arg_reg: rs1,
        }),
        _ => None,
    }
}

fn zisk_main_report_instruction_size(
    row: usize,
    report: &GuestMachineReport,
) -> Result<i64, GuestPcTraceBackendError> {
    match report.instruction_byte_len {
        2 | 4 => Ok(report.instruction_byte_len as i64),
        byte_len => Err(GuestPcTraceBackendError::ZiskMainLower {
            row,
            source: ZiskMainLowerError::InvalidInstructionByteLen {
                pc: report.address,
                byte_len: usize::from(byte_len),
            },
        }),
    }
}

fn zisk_main_base_instruction(
    pc: u64,
    a: ZiskMainSource,
    b: ZiskMainSource,
    op: ZiskMainOp,
    store: ZiskMainStore,
    instruction_size: i64,
) -> ZiskMainInstruction {
    ZiskMainInstruction {
        pc,
        a,
        b,
        op,
        store,
        store_pc: false,
        set_pc: false,
        jmp_offset1: instruction_size,
        jmp_offset2: instruction_size,
        ind_width: 0,
        m32: false,
        is_external_op: !matches!(op, ZiskMainOp::Flag | ZiskMainOp::CopyB),
        is_precompiled: false,
    }
}

fn zisk_main_register_source(index: u8) -> ZiskMainSource {
    if index == 0 {
        ZiskMainSource::Immediate(0)
    } else {
        ZiskMainSource::Register(index)
    }
}

fn zisk_main_register_store(index: u8) -> ZiskMainStore {
    if index == 0 {
        ZiskMainStore::None
    } else {
        ZiskMainStore::Register(index)
    }
}

fn apply_zisk_main_register_access_values(
    row: usize,
    instruction: &ZiskMainInstruction,
    state: &mut ZiskMainTraceState,
    row_mem_step_base: u64,
    a_index: Option<u8>,
    b_index: Option<u8>,
) -> Result<ZiskMainRegisterAccessValues, GuestPcTraceBackendError> {
    let store_index = zisk_main_store_register_index(row, instruction.store)?;
    let mut values = ZiskMainRegisterAccessValues {
        a_prev_mem_step: None,
        b_prev_mem_step: None,
        store_prev_mem_step: None,
        store_prev_value: None,
    };
    if a_index.is_none() && b_index.is_none() && store_index.is_none() {
        return Ok(values);
    }
    if let Some(index) = a_index {
        let next_step =
            zisk_main_mem_step_from_base(row_mem_step_base, ZISK_MAIN_A_MEM_STEP_OFFSET)?;
        values.a_prev_mem_step = Some(read_then_update_register_mem_step(
            &mut state.register_mem_steps,
            index,
            next_step,
        ));
    }
    if let Some(index) = b_index {
        let next_step =
            zisk_main_mem_step_from_base(row_mem_step_base, ZISK_MAIN_B_MEM_STEP_OFFSET)?;
        values.b_prev_mem_step = Some(read_then_update_register_mem_step(
            &mut state.register_mem_steps,
            index,
            next_step,
        ));
    }
    if let Some(index) = store_index {
        values.store_prev_value = Some(state.registers[usize::from(index)]);
        let next_step =
            zisk_main_mem_step_from_base(row_mem_step_base, ZISK_MAIN_STORE_MEM_STEP_OFFSET)?;
        values.store_prev_mem_step = Some(read_then_update_register_mem_step(
            &mut state.register_mem_steps,
            index,
            next_step,
        ));
    }

    Ok(values)
}

fn read_then_update_register_mem_step(
    register_mem_steps: &mut [u64; 32],
    index: u8,
    value: u64,
) -> u64 {
    let index = usize::from(index);
    let previous = register_mem_steps[index];
    register_mem_steps[index] = value;
    previous
}

fn zisk_main_store_register_index(
    row: usize,
    store: ZiskMainStore,
) -> Result<Option<u8>, GuestPcTraceBackendError> {
    match store {
        ZiskMainStore::Register(index) => zisk_main_register_index(index)
            .map(Some)
            .map_err(|()| GuestPcTraceBackendError::UnsupportedZiskMainStore { row }),
        _ => Ok(None),
    }
}

fn zisk_main_register_index(index: u8) -> Result<u8, ()> {
    if (ZISK_MAIN_REGISTER_START..ZISK_MAIN_REGISTER_START + ZISK_MAIN_REGISTER_COUNT)
        .contains(&usize::from(index))
    {
        Ok(index)
    } else {
        Err(())
    }
}

#[cfg(test)]
fn zisk_main_row_mem_step(
    row_count: usize,
    trace_instance_index: u32,
    row: usize,
    offset: u64,
) -> Result<u64, GuestPcTraceBackendError> {
    let base = zisk_main_row_mem_step_base(row_count, trace_instance_index, row)?;
    zisk_main_mem_step_from_base(base, offset)
}

#[cfg(test)]
fn zisk_main_row_mem_step_base(
    row_count: usize,
    trace_instance_index: u32,
    row: usize,
) -> Result<u64, GuestPcTraceBackendError> {
    let segment_base = zisk_main_segment_mem_step_base(row_count, trace_instance_index)?;
    zisk_main_row_mem_step_base_from_segment_base(segment_base, row)
}

fn zisk_main_segment_mem_step_base(
    row_count: usize,
    trace_instance_index: u32,
) -> Result<u64, GuestPcTraceBackendError> {
    let row_count =
        u64::try_from(row_count).map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main row count is too large".to_owned(),
        })?;
    let main_step_base = u64::from(trace_instance_index)
        .checked_mul(row_count)
        .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main step is too large".to_owned(),
        })?;
    zisk_main_mem_step(main_step_base, ZISK_MAIN_A_MEM_STEP_OFFSET)
}

fn zisk_main_row_mem_step_base_from_segment_base(
    segment_base: u64,
    row: usize,
) -> Result<u64, GuestPcTraceBackendError> {
    let row = u64::try_from(row).map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "Zisk Main row index is too large".to_owned(),
    })?;
    ZISK_MAIN_MEM_STEPS_PER_ROW
        .checked_mul(row)
        .and_then(|row_offset| segment_base.checked_add(row_offset))
        .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main step is too large".to_owned(),
        })
}

fn zisk_main_mem_step_from_base(base: u64, offset: u64) -> Result<u64, GuestPcTraceBackendError> {
    base.checked_add(offset)
        .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main memory step is too large".to_owned(),
        })
}

fn zisk_main_last_segment_reg_mem_step(
    row_count: usize,
    trace_instance_index: u32,
) -> Result<u64, GuestPcTraceBackendError> {
    if row_count == 0 {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main layout has zero rows".to_owned(),
        });
    }
    let row_count =
        u64::try_from(row_count).map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main row count is too large".to_owned(),
        })?;
    let main_step = u64::from(trace_instance_index)
        .checked_add(1)
        .and_then(|next_segment| next_segment.checked_mul(row_count))
        .and_then(|exclusive_end| exclusive_end.checked_sub(1))
        .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main final register reload step is too large".to_owned(),
        })?;
    zisk_main_mem_step(main_step, ZISK_MAIN_SPECIAL_MEM_STEP_OFFSET)
}

fn zisk_main_mem_step(main_step: u64, offset: u64) -> Result<u64, GuestPcTraceBackendError> {
    ZISK_MAIN_MEM_STEPS_PER_ROW
        .checked_mul(main_step)
        .and_then(|base| base.checked_add(ZISK_MAIN_RESERVED_MEM_STEPS))
        .and_then(|base| base.checked_add(offset))
        .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main memory step is too large".to_owned(),
        })
}

fn write_layout_zisk_main_trace(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    halt_pc: u64,
    output: &mut [u8],
) -> Result<Option<WitnessTraceOutput>, GuestPcTraceBackendError> {
    let Some(written) = write_layout_zisk_main_trace_segment(
        layout,
        reports,
        halt_pc,
        output,
        &ZiskMainTraceState::new(),
        None,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: true,
            previous_c: 0,
        },
    )?
    else {
        return Ok(None);
    };
    Ok(Some(written.output))
}

fn write_layout_zisk_main_trace_segment(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    output: &mut [u8],
    initial_state: &ZiskMainTraceState,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
) -> Result<Option<ZiskMainTraceSegmentWrite>, GuestPcTraceBackendError> {
    let Some(written) = build_layout_zisk_main_trace_segment(
        layout,
        reports,
        terminal_pc,
        initial_state,
        lookahead_instruction,
        segment,
        None,
    )?
    else {
        return Ok(None);
    };
    let trace =
        written
            .trace
            .as_ref()
            .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "host trace is unavailable for output serialization".to_owned(),
            })?;
    serialize_trace_to_output(trace, written.output.produced_len, output)?;
    Ok(Some(written))
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
fn build_layout_zisk_main_trace_segment_device_material(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    initial_state: &ZiskMainTraceState,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<Option<GuestPcTraceDeviceSegmentBuild>, GuestPcTraceBackendError> {
    if reports.len() > layout.row_count() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len: layout_trace_byte_len(reports.len(), layout.column_count()),
            output_len: layout_trace_byte_len(layout.row_count(), layout.column_count()),
        });
    }
    let Some(mut builder) =
        ZiskMainStreamingDeviceSegmentBuilder::new(layout, initial_state, segment)?
    else {
        return Ok(None);
    };

    let timing_config = ZiskMainTraceLowerTimingConfig::from_env();
    let mut feeder = ZiskMainStreamingDeviceReportFeeder::new(timing_config);
    let aggregate_report_started = timing.as_ref().map(|_| Instant::now());
    for report in reports {
        feeder.push_report(&mut builder, report, timing.as_deref_mut())?;
    }
    feeder.finish(&mut builder, lookahead_instruction, timing.as_deref_mut())?;
    record_aggregate_trace_report_duration(&mut timing, aggregate_report_started);
    builder.finish(terminal_pc, timing).map(Some)
}

#[cfg(feature = "cuda")]
fn build_layout_zisk_main_trace_segment_from_device_material(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    initial_state: &ZiskMainTraceState,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
    timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<Option<ZiskMainTraceSegmentWrite>, GuestPcTraceBackendError> {
    let Some(material) = build_layout_zisk_main_trace_segment_device_material(
        layout,
        reports,
        terminal_pc,
        initial_state,
        lookahead_instruction,
        segment,
        timing,
    )?
    else {
        return Ok(None);
    };
    let produced_len = layout_trace_byte_len(layout.row_count(), layout.column_count());
    let GuestPcTraceDeviceSegmentBuild {
        device_segment_material,
        unit_values,
        final_state,
        continuation_state,
    } = material;
    let trace_source_prefix_rows = device_segment_material.trace_source_prefix_rows;
    Ok(Some(ZiskMainTraceSegmentWrite {
        trace: None,
        trace_source_prefix_rows,
        device_segment_material: Some(device_segment_material),
        output: WitnessTraceOutput::with_unit_values(produced_len, unit_values),
        final_state,
        continuation_state,
    }))
}

fn build_layout_zisk_main_trace_segment_for_segment_output(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    initial_state: &ZiskMainTraceState,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
    timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<Option<ZiskMainTraceSegmentWrite>, GuestPcTraceBackendError> {
    #[cfg(feature = "cuda")]
    let mut timing = timing;

    #[cfg(feature = "cuda")]
    {
        if guest_pc_trace_less_segment_output_enabled() {
            if let Some(written) = build_layout_zisk_main_trace_segment_from_device_material(
                layout,
                reports,
                terminal_pc,
                initial_state,
                lookahead_instruction,
                segment,
                timing.as_deref_mut(),
            )? {
                return Ok(Some(written));
            }
        }
    }
    build_layout_zisk_main_trace_segment(
        layout,
        reports,
        terminal_pc,
        initial_state,
        lookahead_instruction,
        segment,
        timing,
    )
}

fn advance_zisk_main_segment_seed(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    seed: &ZiskMainSegmentSeed,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
) -> Result<Option<ZiskMainSegmentSeed>, GuestPcTraceBackendError> {
    if zisk_main_trace_columns(layout)?.is_none() {
        return Ok(None);
    }
    if seed.previous_c != segment.previous_c {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "Zisk Main segment seed previous_c {} does not match segment previous_c {}",
                seed.previous_c, segment.previous_c
            ),
        });
    }

    let mut state = seed.initial_state.clone();
    let mut output_row = 0_usize;
    let mut context = ZiskMainReportValidationContext::new(None, layout.row_count(), segment)?;
    for (report_index, report) in reports.iter().enumerate() {
        let mut next_instruction =
            || guest_report_next_instruction(reports, report_index, lookahead_instruction);
        let written_rows = validate_and_apply_zisk_main_report(
            output_row,
            report,
            &mut next_instruction,
            &mut state,
            &mut context,
            None,
            false,
            false,
            |_, _, _| Ok(()),
        )?;
        output_row = output_row.checked_add(written_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main row index overflow".to_owned(),
            }
        })?;
    }

    if output_row < layout.row_count() {
        if !segment.is_last_segment {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "non-final Zisk Main segment does not fill layout rows".to_owned(),
            });
        }
        validate_zisk_main_halt_pc(output_row, &state, terminal_pc)?;
    }

    let continuation_state = zisk_main_continuation_state(layout.row_count(), &state, segment)?;
    Ok(Some(ZiskMainSegmentSeed {
        initial_state: continuation_state,
        previous_c: state.last_c,
    }))
}

#[cfg(test)]
fn try_lift_zisk_main_next_segment_seed_from_runner_boundary(
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
    reports: &[GuestMachineReport],
    lookahead_instruction: Option<RiscvInstruction>,
    runner_state: &GuestMachineState,
    current_seed: &ZiskMainSegmentSeed,
) -> Result<Result<ZiskMainSegmentSeed, ZiskMainDirectSeedLiftMissReason>, GuestPcTraceBackendError>
{
    let boundary_snapshot = zisk_main_runner_boundary_snapshot_from_reports(
        reports,
        lookahead_instruction,
        runner_state,
        current_seed,
    )?;
    try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        row_count,
        segment,
        ZiskMainRunnerBoundarySeedInput::from_reports(
            reports,
            lookahead_instruction,
            runner_state,
            current_seed,
            &boundary_snapshot,
        ),
    )
}

fn try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
    input: ZiskMainRunnerBoundarySeedInput<'_>,
) -> Result<Result<ZiskMainSegmentSeed, ZiskMainDirectSeedLiftMissReason>, GuestPcTraceBackendError>
{
    let next_previous_c = match direct_zisk_main_segment_boundary_c_from_tail(
        input.report_count(),
        input.last_report(),
        input.lookahead_instruction,
        input.current_seed,
        Some(input.runner_state.registers()),
    )? {
        Ok(next_previous_c) => next_previous_c,
        Err(reason) => return Ok(Err(reason)),
    };
    lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        row_count,
        segment,
        input,
        next_previous_c,
    )
    .map(Ok)
}

#[cfg(test)]
fn lift_zisk_main_next_segment_seed_from_runner_boundary(
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
    reports: &[GuestMachineReport],
    lookahead_instruction: Option<RiscvInstruction>,
    runner_state: &GuestMachineState,
    current_seed: &ZiskMainSegmentSeed,
    next_previous_c: u64,
) -> Result<ZiskMainSegmentSeed, GuestPcTraceBackendError> {
    let boundary_snapshot = zisk_main_runner_boundary_snapshot_from_reports(
        reports,
        lookahead_instruction,
        runner_state,
        current_seed,
    )?;
    lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        row_count,
        segment,
        ZiskMainRunnerBoundarySeedInput::from_reports(
            reports,
            lookahead_instruction,
            runner_state,
            current_seed,
            &boundary_snapshot,
        ),
        next_previous_c,
    )
}

fn lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
    input: ZiskMainRunnerBoundarySeedInput<'_>,
    next_previous_c: u64,
) -> Result<ZiskMainSegmentSeed, GuestPcTraceBackendError> {
    if segment.is_last_segment {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "runner boundary seed snapshot is only defined for non-final segments"
                .to_owned(),
        });
    }

    if let Ok(direct_c) = direct_zisk_main_segment_boundary_c_from_tail(
        input.report_count(),
        input.last_report(),
        input.lookahead_instruction,
        input.current_seed,
        Some(input.runner_state.registers()),
    )? {
        if direct_c != next_previous_c {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: format!(
                    "runner boundary direct boundary c {direct_c} does not match mirror c {next_previous_c} after segment {}",
                    segment.trace_instance_index
                ),
            });
        }
    }

    let mut register_mem_steps = [0; 32];
    let final_reload_step =
        zisk_main_last_segment_reg_mem_step(row_count, segment.trace_instance_index)?;
    for step in register_mem_steps
        .iter_mut()
        .skip(ZISK_MAIN_REGISTER_START)
        .take(ZISK_MAIN_REGISTER_COUNT)
    {
        *step = final_reload_step;
    }

    Ok(ZiskMainSegmentSeed {
        initial_state: ZiskMainTraceState {
            registers: *input.runner_state.registers(),
            internal_memory: input.boundary_snapshot.internal_memory,
            register_mem_steps,
            pending_dma: input.last_report().and_then(zisk_main_pending_dma),
            last_c: next_previous_c,
            next_pc: input.runner_state.pc(),
        },
        previous_c: next_previous_c,
    })
}

#[cfg(test)]
fn zisk_main_runner_boundary_snapshot_from_reports(
    reports: &[GuestMachineReport],
    lookahead_instruction: Option<RiscvInstruction>,
    runner_state: &GuestMachineState,
    current_seed: &ZiskMainSegmentSeed,
) -> Result<ZiskMainRunnerBoundarySnapshot, GuestPcTraceBackendError> {
    let mut snapshot = ZiskMainRunnerBoundarySnapshot::new(current_seed);
    let mut registers = current_seed.initial_state.registers;
    for (report_index, report) in reports.iter().enumerate() {
        for write in &report.register_writes {
            if write.index != 0 {
                registers[usize::from(write.index)] = write.value;
            }
        }
        if report_index + 1 == reports.len() {
            registers = *runner_state.registers();
        }
        let next_instruction =
            guest_report_next_instruction(reports, report_index, lookahead_instruction);
        snapshot.record_report(report, next_instruction, &registers)?;
    }
    Ok(snapshot)
}

fn record_zisk_main_runner_scratch_update(
    internal_memory: &mut ZiskMainInternalMemory,
    registers: &[u64; 32],
    report: &GuestMachineReport,
    next_instruction: Option<RiscvInstruction>,
) -> Result<(), GuestPcTraceBackendError> {
    if let RiscvInstruction::Amo {
        kind: RiscvAmoKind::Add,
        width,
        rd,
        rs1,
        rs2,
        ..
    } = report.instruction
    {
        if rd != 0 && (rd == rs1 || rd == rs2) {
            let Some(read_access) = report
                .memory_accesses
                .iter()
                .find(|access| access.kind == GuestMemoryAccessKind::Read)
            else {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row: 0,
                    message: "AMO scratch snapshot is missing the load access".to_owned(),
                });
            };
            let value = match width {
                RiscvAmoWidth::Word => sign_extend_word(read_access.value as u32),
                RiscvAmoWidth::Doubleword => read_access.value,
            };
            internal_memory
                .insert(
                    zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER),
                    value,
                )
                .map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: "unsupported Zisk Main internal scratch address".to_owned(),
                })?;
        }
    }

    if let RiscvInstruction::ZiskDmaPrepare { kind, .. } = report.instruction {
        if matches!(kind, RiscvDmaKind::Memcpy | RiscvDmaKind::Memcmp) {
            if let Some(RiscvInstruction::Op {
                kind: RiscvOpKind::Add,
                rs2,
                ..
            }) = next_instruction
            {
                internal_memory
                    .insert(ZISK_EXTRA_PARAMS_ADDRESS, registers[usize::from(rs2)])
                    .map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
                        message: "unsupported Zisk Main internal scratch address".to_owned(),
                    })?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "cuda")]
fn guest_pc_trace_less_segment_output_enabled() -> bool {
    env_flag_enabled("LZVM_CUDA_GUEST_PC_TRACELESS_SEGMENT_OUTPUT", true)
}

fn guest_pc_trace_lower_detail_timing_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_TRACE_DETAIL_TIMING", false)
}

fn guest_pc_trace_detail_timing_sample_stride() -> usize {
    std::env::var("LZVM_GUEST_TRACE_DETAIL_TIMING_SAMPLE_STRIDE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|&stride| stride != 0)
        .unwrap_or(1)
}

fn guest_pc_trace_shape_timing_enabled() -> bool {
    env_flag_enabled("LZVM_GUEST_TRACE_SHAPE_TIMING", false)
}

fn build_layout_zisk_main_trace_segment(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    initial_state: &ZiskMainTraceState,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
) -> Result<Option<ZiskMainTraceSegmentWrite>, GuestPcTraceBackendError> {
    let Some(columns) = zisk_main_trace_columns(layout)? else {
        return Ok(None);
    };
    if reports.len() > layout.row_count() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len: layout_trace_byte_len(reports.len(), layout.column_count()),
            output_len: layout_trace_byte_len(layout.row_count(), layout.column_count()),
        });
    }

    let mut builder = layout
        .trace_builder()
        .map_err(GuestPcTraceBackendError::TraceBuild)?;
    #[cfg(feature = "cuda")]
    let mut device_trace_descriptors =
        main_device_trace_descriptors(layout, &columns, terminal_pc, segment);
    let mut state = initial_state.clone();
    let mut output_row = 0_usize;
    let detail_timing = guest_pc_trace_lower_detail_timing_enabled();
    let shape_timing = guest_pc_trace_shape_timing_enabled();
    let detail_sample_stride = guest_pc_trace_detail_timing_sample_stride();
    let aggregate_report_started = timing.as_ref().map(|_| Instant::now());
    for (report_index, report) in reports.iter().enumerate() {
        let report_detail_timing = detail_timing && report_index % detail_sample_stride == 0;
        let report_started = timing
            .as_ref()
            .filter(|_| report_detail_timing)
            .map(|_| Instant::now());
        let pending_report = state.pending_dma.is_some();
        let mut next_instruction =
            || guest_report_next_instruction(reports, report_index, lookahead_instruction);
        let row_timing = if report_detail_timing || shape_timing {
            timing.as_deref_mut()
        } else {
            None
        };
        let written_rows = write_zisk_main_report_columns(
            &mut builder,
            output_row,
            ZiskMainReportWindow {
                current: report,
                next_instruction: &mut next_instruction,
            },
            &columns,
            &mut state,
            layout.row_count(),
            segment,
            #[cfg(feature = "cuda")]
            &mut device_trace_descriptors,
            row_timing,
            report_detail_timing,
            shape_timing,
        )?;
        if let Some(timing) = timing.as_deref_mut() {
            timing.trace_report_count += 1;
            timing.trace_report_row_count += written_rows;
            if shape_timing {
                record_trace_report_shape(timing, report, pending_report, written_rows);
            }
            if let Some(started) = report_started {
                timing.trace_report_detail_sample_count += 1;
                let duration = started.elapsed();
                timing.trace_report_sample_duration += duration;
                record_trace_report_duration(
                    timing,
                    report,
                    pending_report,
                    written_rows,
                    duration,
                );
            }
        }
        output_row = output_row.checked_add(written_rows).ok_or_else(|| {
            GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "Zisk Main row index overflow".to_owned(),
            }
        })?;
    }
    record_aggregate_trace_report_duration(&mut timing, aggregate_report_started);
    #[cfg(feature = "cuda")]
    if let Some(descriptors) = device_trace_descriptors.as_ref() {
        record_trace_descriptor_width_counts(&mut timing, descriptors);
    }
    if output_row < layout.row_count() {
        if !segment.is_last_segment {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "non-final Zisk Main segment does not fill layout rows".to_owned(),
            });
        }
        validate_zisk_main_halt_pc(output_row, &state, terminal_pc)?;
        for row in output_row..layout.row_count() {
            write_zisk_main_terminal_row(&mut builder, row, &columns, terminal_pc)?;
        }
    }
    let trace = builder.build();
    let produced_len =
        trace
            .values()
            .len()
            .checked_mul(8)
            .ok_or(GuestPcTraceBackendError::OutputOverflow {
                produced_len: usize::MAX,
                output_len: usize::MAX,
            })?;
    let continuation_state = zisk_main_continuation_state(layout.row_count(), &state, segment)?;
    let unit_values = ZiskMainSegmentUnitValueSummary::from_reports(reports).unit_values(
        layout.row_count(),
        output_row,
        terminal_pc,
        &state,
        segment,
    );
    #[cfg(feature = "cuda")]
    let device_segment_material = device_trace_descriptors.map(|device_trace_descriptors| {
        GuestPcTraceDeviceSegmentMaterial {
            trace_source_prefix_rows: output_row,
            device_trace_descriptors,
        }
    });
    Ok(Some(ZiskMainTraceSegmentWrite {
        trace: Some(trace),
        trace_source_prefix_rows: output_row,
        #[cfg(feature = "cuda")]
        device_segment_material,
        output: WitnessTraceOutput::with_unit_values(produced_len, unit_values),
        final_state: state,
        continuation_state,
    }))
}

fn serialize_trace_to_output(
    trace: &WitnessTraceBuffer,
    produced_len: usize,
    output: &mut [u8],
) -> Result<(), GuestPcTraceBackendError> {
    debug_assert_eq!(
        produced_len,
        layout_trace_byte_len(trace.row_count(), trace.column_count())
    );
    if produced_len > output.len() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len,
            output_len: output.len(),
        });
    }
    for (index, value) in trace.values().iter().copied().enumerate() {
        let offset = index * 8;
        output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    Ok(())
}

fn zisk_main_continuation_state(
    row_count: usize,
    state: &ZiskMainTraceState,
    segment: ZiskMainTraceSegmentInfo,
) -> Result<ZiskMainTraceState, GuestPcTraceBackendError> {
    let mut continuation_state = state.clone();
    if !segment.is_last_segment {
        let final_reload_step =
            zisk_main_last_segment_reg_mem_step(row_count, segment.trace_instance_index)?;
        for index in ZISK_MAIN_REGISTER_START..ZISK_MAIN_REGISTER_START + ZISK_MAIN_REGISTER_COUNT {
            continuation_state.register_mem_steps[index] = final_reload_step;
        }
    }
    Ok(continuation_state)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ZiskMainSegmentUnitValueSummary {
    segment_initial_pc: Option<u64>,
}

impl ZiskMainSegmentUnitValueSummary {
    fn new() -> Self {
        Self::default()
    }

    fn from_reports(reports: &[GuestMachineReport]) -> Self {
        let mut summary = Self::new();
        for report in reports {
            summary.push_report(report);
        }
        summary
    }

    fn push_report(&mut self, report: &GuestMachineReport) {
        self.segment_initial_pc.get_or_insert(report.address);
    }

    fn unit_values(
        self,
        row_count: usize,
        written_rows: usize,
        terminal_pc: u64,
        state: &ZiskMainTraceState,
        segment: ZiskMainTraceSegmentInfo,
    ) -> Vec<WitnessTraceUnitValue> {
        zisk_main_unit_values(row_count, written_rows, self, terminal_pc, state, segment)
    }
}

fn zisk_main_unit_values(
    row_count: usize,
    written_rows: usize,
    summary: ZiskMainSegmentUnitValueSummary,
    terminal_pc: u64,
    state: &ZiskMainTraceState,
    segment: ZiskMainTraceSegmentInfo,
) -> Vec<WitnessTraceUnitValue> {
    let segment_initial_pc = summary.segment_initial_pc.unwrap_or(terminal_pc);
    let segment_last_c = if segment.is_last_segment && written_rows < row_count {
        0
    } else {
        state.last_c
    };

    let mut last_reg_value = Vec::with_capacity(ZISK_MAIN_REGISTER_COUNT * 2);
    let mut last_reg_mem_step = Vec::with_capacity(ZISK_MAIN_REGISTER_COUNT);
    for index in ZISK_MAIN_REGISTER_START..ZISK_MAIN_REGISTER_START + ZISK_MAIN_REGISTER_COUNT {
        last_reg_value.extend(felt_limbs_u64(state.registers[index]));
        last_reg_mem_step.push(Felt::from_u64(state.register_mem_steps[index]));
    }

    vec![
        WitnessTraceUnitValue::new(
            "main_last_segment",
            vec![if segment.is_last_segment {
                Felt::ONE
            } else {
                Felt::ZERO
            }],
        ),
        WitnessTraceUnitValue::new(
            "main_segment",
            vec![Felt::from_u64(u64::from(segment.trace_instance_index))],
        ),
        WitnessTraceUnitValue::new(
            "segment_initial_pc",
            vec![Felt::from_u64(segment_initial_pc)],
        ),
        WitnessTraceUnitValue::new(
            "segment_previous_c",
            felt_limbs_u64(segment.previous_c).to_vec(),
        ),
        WitnessTraceUnitValue::new("segment_next_pc", vec![Felt::from_u64(terminal_pc)]),
        WitnessTraceUnitValue::new("segment_last_c", felt_limbs_u64(segment_last_c).to_vec()),
        WitnessTraceUnitValue::new("last_reg_value", last_reg_value),
        WitnessTraceUnitValue::new("last_reg_mem_step", last_reg_mem_step),
    ]
}

fn zisk_runtime_proof_values(
    enable_rom_data: bool,
    enable_input_data: bool,
    dma: GuestDmaProofValueFlags,
) -> Vec<WitnessTraceProofValue> {
    vec![
        proof_value_bool("enable_input_data", enable_input_data),
        proof_value_bool("enable_rom_data", enable_rom_data),
        proof_value_bool("enable_dma_64_aligned", false),
        proof_value_bool(
            "enable_dma_64_aligned_inputcpy",
            dma.enable_dma_64_aligned_inputcpy,
        ),
        proof_value_bool("enable_dma_64_aligned_mem", dma.enable_dma_64_aligned_mem),
        proof_value_bool(
            "enable_dma_64_aligned_memcpy",
            dma.enable_dma_64_aligned_memcpy,
        ),
        proof_value_bool(
            "enable_dma_64_aligned_memset",
            dma.enable_dma_64_aligned_memset,
        ),
        proof_value_bool("enable_dma_unaligned", dma.enable_dma_unaligned),
    ]
}

fn proof_value_bool(name: &str, enabled: bool) -> WitnessTraceProofValue {
    WitnessTraceProofValue::new(name, vec![if enabled { Felt::ONE } else { Felt::ZERO }])
}

#[allow(clippy::too_many_arguments)]
fn write_zisk_main_report_columns(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    reports: ZiskMainReportWindow<'_>,
    columns: &ZiskMainTraceColumns<'_>,
    state: &mut ZiskMainTraceState,
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
    #[cfg(feature = "cuda")] device_trace_descriptors: &mut Option<ZiskMainDeviceTraceDescriptors>,
    mut timing: Option<&mut GuestPcTraceStreamTiming>,
    _detail_timing: bool,
    shape_timing: bool,
) -> Result<usize, GuestPcTraceBackendError> {
    #[cfg(feature = "cuda")]
    let descriptor_rows_before = device_trace_descriptors
        .as_ref()
        .map(ZiskMainDeviceTraceDescriptors::descriptor_rows)
        .unwrap_or(0);
    let mut context = ZiskMainReportValidationContext::new(Some(columns), row_count, segment)?;
    validate_and_apply_zisk_main_report(
        row,
        reports.current,
        reports.next_instruction,
        state,
        &mut context,
        reborrow_trace_timing(&mut timing),
        _detail_timing,
        shape_timing,
        |output_row, values, mut visit_timing| {
            #[cfg(feature = "cuda")]
            if let Some(descriptors) = device_trace_descriptors.as_mut() {
                let _descriptor_timer = DurationTimer::new(
                    visit_timing
                        .as_deref_mut()
                        .filter(|_| _detail_timing)
                        .map(|timing| &mut timing.trace_descriptor_duration),
                );
                append_main_device_trace_descriptor(descriptors, &values)?;
            }
            if shape_timing {
                if let Some(timing) = visit_timing.as_deref_mut() {
                    record_trace_lowered_row_shape(timing, &values.instruction);
                }
            }
            let _emit_timer = DurationTimer::new(
                visit_timing
                    .as_deref_mut()
                    .map(|timing| &mut timing.trace_emit_duration),
            );
            write_zisk_main_row_columns(builder, output_row, values, columns)
        },
    )
    .inspect(|_| {
        #[cfg(feature = "cuda")]
        if let Some(timing) = timing.as_mut() {
            let timing = &mut **timing;
            let descriptor_rows_after = device_trace_descriptors
                .as_ref()
                .map(ZiskMainDeviceTraceDescriptors::descriptor_rows)
                .unwrap_or(0);
            timing.trace_descriptor_row_count +=
                descriptor_rows_after.saturating_sub(descriptor_rows_before);
        }
    })
}

fn write_zisk_main_row_columns(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    values: ZiskMainReportTraceValues,
    columns: &ZiskMainTraceColumns<'_>,
) -> Result<(), GuestPcTraceBackendError> {
    let instruction = values.instruction;

    write_wide_column(builder, row, &columns.a, values.a)?;
    write_wide_column(builder, row, &columns.b, values.b)?;
    write_wide_column(builder, row, &columns.c, values.c)?;
    write_column(builder, row, &columns.flag, u64::from(values.flag))?;
    write_column(builder, row, &columns.pc, instruction.pc)?;
    write_optional_column(
        builder,
        row,
        &columns.a_src_imm,
        u64::from(matches!(instruction.a, ZiskMainSource::Immediate(_))),
    )?;
    write_optional_signed_column(
        builder,
        row,
        &columns.a_offset_imm0,
        zisk_main_source_offset(row, instruction.a)?,
    )?;
    write_optional_column(
        builder,
        row,
        &columns.a_imm1,
        zisk_main_source_high_limb(instruction.a),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.b_src_imm,
        u64::from(matches!(instruction.b, ZiskMainSource::Immediate(_))),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.b_imm1,
        zisk_main_source_high_limb(instruction.b),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.a_src_reg,
        u64::from(matches!(instruction.a, ZiskMainSource::Register(_))),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.b_src_reg,
        u64::from(matches!(instruction.b, ZiskMainSource::Register(_))),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.a_src_mem,
        u64::from(matches!(instruction.a, ZiskMainSource::Memory(_))),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.b_src_mem,
        u64::from(matches!(instruction.b, ZiskMainSource::Memory(_))),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.b_src_ind,
        u64::from(matches!(instruction.b, ZiskMainSource::Indirect(_))),
    )?;
    write_optional_signed_column(
        builder,
        row,
        &columns.b_offset_imm0,
        zisk_main_source_offset(row, instruction.b)?,
    )?;
    write_optional_signed_column(
        builder,
        row,
        &columns.addr1,
        zisk_main_b_address(row, instruction.b, values.a)?,
    )?;
    write_optional_column(
        builder,
        row,
        &columns.store_reg,
        u64::from(matches!(instruction.store, ZiskMainStore::Register(_))),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.store_mem,
        u64::from(matches!(instruction.store, ZiskMainStore::Memory(_))),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.store_ind,
        u64::from(matches!(instruction.store, ZiskMainStore::Indirect(_))),
    )?;
    write_optional_signed_column(
        builder,
        row,
        &columns.store_offset,
        zisk_main_store_offset(row, &instruction.store)?,
    )?;
    write_optional_signed_column(
        builder,
        row,
        &columns.addr2,
        zisk_main_store_address(row, &instruction.store, values.a)?,
    )?;
    write_optional_column(
        builder,
        row,
        &columns.store_pc,
        u64::from(instruction.store_pc),
    )?;
    write_optional_column(builder, row, &columns.set_pc, u64::from(instruction.set_pc))?;
    write_column(builder, row, &columns.op, u64::from(instruction.op.code()))?;
    write_optional_signed_column(builder, row, &columns.jmp_offset1, instruction.jmp_offset1)?;
    write_optional_signed_column(builder, row, &columns.jmp_offset2, instruction.jmp_offset2)?;
    write_optional_column(builder, row, &columns.ind_width, instruction.ind_width)?;
    write_optional_column(builder, row, &columns.m32, u64::from(instruction.m32))?;
    write_optional_column(
        builder,
        row,
        &columns.is_external_op,
        u64::from(instruction.is_external_op),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.is_precompiled,
        u64::from(instruction.is_precompiled),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.a_reg_prev_mem_step,
        values.register_accesses.a_prev_mem_step.unwrap_or(0),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.b_reg_prev_mem_step,
        values.register_accesses.b_prev_mem_step.unwrap_or(0),
    )?;
    write_optional_column(
        builder,
        row,
        &columns.store_reg_prev_mem_step,
        values.register_accesses.store_prev_mem_step.unwrap_or(0),
    )?;
    write_optional_wide_column(
        builder,
        row,
        &columns.store_reg_prev_value,
        values.register_accesses.store_prev_value.unwrap_or(0),
    )
}

fn validate_zisk_main_halt_pc(
    written_rows: usize,
    state: &ZiskMainTraceState,
    halt_pc: u64,
) -> Result<(), GuestPcTraceBackendError> {
    if written_rows != 0 && state.next_pc != halt_pc {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row: written_rows - 1,
            message: format!(
                "last next pc {} does not match halt pc {}",
                state.next_pc, halt_pc
            ),
        });
    }
    Ok(())
}

fn write_zisk_main_terminal_row(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    columns: &ZiskMainTraceColumns<'_>,
    halt_pc: u64,
) -> Result<(), GuestPcTraceBackendError> {
    write_wide_column(builder, row, &columns.a, 0)?;
    write_wide_column(builder, row, &columns.b, 0)?;
    write_wide_column(builder, row, &columns.c, 0)?;
    write_column(builder, row, &columns.flag, 0)?;
    write_column(builder, row, &columns.pc, halt_pc)?;
    write_optional_column(builder, row, &columns.a_src_imm, 1)?;
    write_optional_column(builder, row, &columns.b_src_imm, 1)?;
    write_optional_column(builder, row, &columns.a_src_mem, 0)?;
    write_optional_column(builder, row, &columns.b_src_mem, 0)?;
    write_column(
        builder,
        row,
        &columns.op,
        u64::from(ZiskMainOp::CopyB.code()),
    )
}

fn validate_zisk_main_memory_columns(
    row: usize,
    instruction: &ZiskMainInstruction,
    columns: &ZiskMainTraceColumns<'_>,
) -> Result<(), GuestPcTraceBackendError> {
    if matches!(instruction.a, ZiskMainSource::Memory(_))
        && !columns.has_required_a_memory_source_columns()
    {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "Zisk Main memory source rows require a_src_mem and a_offset_imm0 columns at row {row}"
            ),
        });
    }
    if matches!(instruction.b, ZiskMainSource::Memory(_))
        && !columns.has_required_b_memory_source_columns()
    {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "Zisk Main memory source rows require b_src_mem and b_offset_imm0 columns at row {row}"
            ),
        });
    }
    if matches!(instruction.store, ZiskMainStore::Memory(_))
        && !columns.has_required_memory_store_columns()
    {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "Zisk Main memory store rows require store_mem and store_offset columns at row {row}"
            ),
        });
    }
    let uses_indirect_memory_row = matches!(instruction.b, ZiskMainSource::Indirect(_))
        || matches!(instruction.store, ZiskMainStore::Indirect(_));
    if uses_indirect_memory_row && !columns.has_required_indirect_memory_columns() {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "Zisk Main memory rows require b_src_ind, b_offset_imm0, ind_width, store_ind, store_offset, and store_mem columns at row {row}"
            ),
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn zisk_main_source_value(
    row: usize,
    source: ZiskMainSource,
    state: &ZiskMainTraceState,
    report: &GuestMachineReport,
    effects: ZiskMainReportEffects<'_>,
    base: Option<u64>,
    ind_width: u64,
    memory_access_index: usize,
) -> Result<ZiskMainSourceValueResult, GuestPcTraceBackendError> {
    match source {
        ZiskMainSource::LastC => Ok(ZiskMainSourceValueResult {
            value: state.last_c,
            memory_access_count: 0,
            register_index: None,
        }),
        ZiskMainSource::Immediate(value) => Ok(ZiskMainSourceValueResult {
            value,
            memory_access_count: 0,
            register_index: None,
        }),
        ZiskMainSource::Register(index) => {
            let index = zisk_main_register_index(index)
                .map_err(|()| GuestPcTraceBackendError::UnsupportedZiskMainSource { row })?;
            Ok(ZiskMainSourceValueResult {
                value: state.registers[usize::from(index)],
                memory_access_count: 0,
                register_index: Some(index),
            })
        }
        ZiskMainSource::Indirect(offset) => {
            let Some(base) = base else {
                return Err(GuestPcTraceBackendError::UnsupportedZiskMainSource { row });
            };
            let byte_len = usize::try_from(ind_width)
                .map_err(|_| GuestPcTraceBackendError::UnsupportedZiskMainSource { row })?;
            let address = base.wrapping_add_signed(offset);
            let value = ordered_memory_access_value(
                row,
                effects,
                memory_access_index,
                GuestMemoryAccessKind::Read,
                address,
                byte_len,
            )?;
            Ok(ZiskMainSourceValueResult {
                value,
                memory_access_count: 1,
                register_index: None,
            })
        }
        ZiskMainSource::Memory(address) => {
            zisk_main_memory_source_value(row, address, state, report, effects, memory_access_index)
        }
    }
}

#[inline(always)]
fn zisk_main_memory_source_value(
    row: usize,
    address: u64,
    state: &ZiskMainTraceState,
    report: &GuestMachineReport,
    effects: ZiskMainReportEffects<'_>,
    memory_access_index: usize,
) -> Result<ZiskMainSourceValueResult, GuestPcTraceBackendError> {
    if address == ZISK_INPUT_ADDRESS {
        if let RiscvInstruction::ZiskFcallResult { rd } = report.instruction {
            let value = zisk_main_fcall_result_value(row, rd, effects)?;
            return Ok(ZiskMainSourceValueResult {
                value,
                memory_access_count: 0,
                register_index: None,
            });
        }
    }
    if zisk_internal_memory_address(address) && effects.memory_accesses.is_empty() {
        let Some(value) = state.internal_memory.get(address) else {
            return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                row,
                message: format!("missing internal memory value at {address}"),
            });
        };
        return Ok(ZiskMainSourceValueResult {
            value,
            memory_access_count: 0,
            register_index: None,
        });
    }
    let value = ordered_memory_access_value(
        row,
        effects,
        memory_access_index,
        GuestMemoryAccessKind::Read,
        address,
        8,
    )?;
    Ok(ZiskMainSourceValueResult {
        value,
        memory_access_count: 1,
        register_index: None,
    })
}

fn zisk_main_fcall_result_value(
    row: usize,
    rd: u8,
    effects: ZiskMainReportEffects<'_>,
) -> Result<u64, GuestPcTraceBackendError> {
    let [write] = effects.register_writes else {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "free-call result row reported {} register writes",
                effects.register_writes.len()
            ),
        });
    };
    if write.index != rd {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!("expected free-call result in x{rd}, found x{}", write.index),
        });
    }
    Ok(write.value)
}

#[inline(always)]
fn ordered_memory_access_value(
    row: usize,
    effects: ZiskMainReportEffects<'_>,
    access_index: usize,
    kind: GuestMemoryAccessKind,
    address: u64,
    byte_len: usize,
) -> Result<u64, GuestPcTraceBackendError> {
    let Some(access) = effects.memory_accesses.get(access_index) else {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!("missing {kind:?} access at {address} with byte length {byte_len}"),
        });
    };
    validate_memory_access_fields(row, access, kind, address, byte_len, access.value)?;
    Ok(access.value)
}

#[cfg(test)]
fn matching_memory_access(
    row: usize,
    effects: ZiskMainReportEffects<'_>,
    kind: GuestMemoryAccessKind,
    address: u64,
    byte_len: usize,
) -> Result<ExpectedMemoryAccess, GuestPcTraceBackendError> {
    let mut matching = None;
    for access in effects.memory_accesses {
        if access.kind == kind
            && access.address == address
            && usize::from(access.byte_len) == byte_len
        {
            if matching.is_some() {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!(
                        "multiple {kind:?} accesses at {address} with byte length {byte_len}"
                    ),
                });
            }
            matching = Some(access);
        }
    }
    match matching {
        Some(access) => Ok(ExpectedMemoryAccess {
            kind,
            address,
            byte_len: usize::from(access.byte_len),
            value: access.value,
        }),
        None => Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!("missing {kind:?} access at {address} with byte length {byte_len}"),
        }),
    }
}

#[cfg(test)]
fn validate_zisk_main_memory_accesses(
    row: usize,
    instruction: &ZiskMainInstruction,
    effects: ZiskMainReportEffects<'_>,
    a: u64,
    c: u64,
    a_access: Option<ExpectedMemoryAccess>,
    b_access: Option<ExpectedMemoryAccess>,
) -> Result<(), GuestPcTraceBackendError> {
    let store_access = zisk_main_store_memory_access(row, instruction, a, c)?;
    let expected_len = usize::from(a_access.is_some())
        + usize::from(b_access.is_some())
        + usize::from(store_access.is_some());
    if effects.memory_accesses.len() != expected_len {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "expected {} memory accesses, found {}",
                expected_len,
                effects.memory_accesses.len()
            ),
        });
    }

    let mut access_index = 0;
    if let Some(expected) = a_access {
        validate_expected_memory_access(row, effects.memory_accesses[access_index], expected)?;
        access_index += 1;
    }
    if let Some(expected) = b_access {
        validate_expected_memory_access(row, effects.memory_accesses[access_index], expected)?;
        access_index += 1;
    }
    if let Some(expected) = store_access {
        validate_expected_memory_access(row, effects.memory_accesses[access_index], expected)?;
        access_index += 1;
    }
    debug_assert_eq!(access_index, expected_len);
    Ok(())
}

fn validate_zisk_main_memory_accesses_after_source_values(
    row: usize,
    instruction: &ZiskMainInstruction,
    effects: ZiskMainReportEffects<'_>,
    a: u64,
    c: u64,
    validated_source_access_count: usize,
) -> Result<(), GuestPcTraceBackendError> {
    if !matches!(instruction.store, ZiskMainStore::Indirect(_)) {
        if effects.memory_accesses.len() != validated_source_access_count {
            return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                row,
                message: format!(
                    "expected {} memory accesses, found {}",
                    validated_source_access_count,
                    effects.memory_accesses.len()
                ),
            });
        }
        return Ok(());
    }
    let store_access = zisk_main_store_memory_access(row, instruction, a, c)?;
    let expected_len = validated_source_access_count + usize::from(store_access.is_some());
    if effects.memory_accesses.len() != expected_len {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "expected {} memory accesses, found {}",
                expected_len,
                effects.memory_accesses.len()
            ),
        });
    }
    if let Some(expected) = store_access {
        validate_expected_memory_access(
            row,
            effects.memory_accesses[validated_source_access_count],
            expected,
        )?;
    }
    Ok(())
}

fn zisk_main_store_memory_access(
    row: usize,
    instruction: &ZiskMainInstruction,
    a: u64,
    c: u64,
) -> Result<Option<ExpectedMemoryAccess>, GuestPcTraceBackendError> {
    let store_value = zisk_main_store_value(instruction, c);
    if let ZiskMainStore::Indirect(offset) = instruction.store {
        let byte_len = usize::try_from(instruction.ind_width)
            .map_err(|_| GuestPcTraceBackendError::UnsupportedZiskMainStore { row })?;
        Ok(Some(ExpectedMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: a.wrapping_add_signed(offset),
            byte_len,
            value: low_bytes_value(store_value, byte_len),
        }))
    } else {
        Ok(None)
    }
}

fn validate_expected_memory_access(
    row: usize,
    found: GuestMemoryAccess,
    expected: ExpectedMemoryAccess,
) -> Result<(), GuestPcTraceBackendError> {
    validate_memory_access_fields(
        row,
        &found,
        expected.kind,
        expected.address,
        expected.byte_len,
        expected.value,
    )
}

#[inline(always)]
fn validate_memory_access_fields(
    row: usize,
    found: &GuestMemoryAccess,
    expected_kind: GuestMemoryAccessKind,
    expected_address: u64,
    expected_byte_len: usize,
    expected_value: u64,
) -> Result<(), GuestPcTraceBackendError> {
    if found.kind != expected_kind
        || found.address != expected_address
        || usize::from(found.byte_len) != expected_byte_len
        || found.value != expected_value
    {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "expected {:?} at {} byte length {} value {}, found {:?} at {} byte length {} value {}",
                expected_kind,
                expected_address,
                expected_byte_len,
                expected_value,
                found.kind,
                found.address,
                found.byte_len,
                found.value
            ),
        });
    }
    Ok(())
}

fn validate_zisk_main_precompile_memory_accesses(
    row: usize,
    report: &GuestMachineReport,
    effects: ZiskMainReportEffects<'_>,
    operand_address: u64,
) -> Result<(), GuestPcTraceBackendError> {
    let RiscvInstruction::ZiskPrecompile { kind, .. } = report.instruction else {
        if effects.precompile_memory_accesses.is_empty() {
            return Ok(());
        }
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "non-precompile row reported {} precompile memory accesses",
                effects.precompile_memory_accesses.len()
            ),
        });
    };

    let mut cursor = PrecompileMemoryAccessCursor {
        row,
        accesses: effects.precompile_memory_accesses,
        offset: 0,
    };
    match kind {
        RiscvPrecompileKind::Keccak => {
            cursor.expect_reads::<25>(operand_address)?;
            cursor.expect_writes::<25>(operand_address)?;
        }
        RiscvPrecompileKind::Arith256 => {
            let params = cursor.expect_read_values::<5>(operand_address)?;
            cursor.expect_reads::<4>(params[0])?;
            cursor.expect_reads::<4>(params[1])?;
            cursor.expect_reads::<4>(params[2])?;
            cursor.expect_writes::<4>(params[3])?;
            cursor.expect_writes::<4>(params[4])?;
        }
        RiscvPrecompileKind::Arith256Mod => {
            let params = cursor.expect_read_values::<5>(operand_address)?;
            cursor.expect_reads::<4>(params[0])?;
            cursor.expect_reads::<4>(params[1])?;
            cursor.expect_reads::<4>(params[2])?;
            cursor.expect_reads::<4>(params[3])?;
            cursor.expect_writes::<4>(params[4])?;
        }
        RiscvPrecompileKind::Secp256k1Add => {
            let params = cursor.expect_read_values::<2>(operand_address)?;
            cursor.expect_reads::<8>(params[0])?;
            cursor.expect_reads::<8>(params[1])?;
            cursor.expect_writes::<8>(params[0])?;
        }
        RiscvPrecompileKind::Secp256k1Dbl => {
            cursor.expect_reads::<8>(operand_address)?;
            cursor.expect_writes::<8>(operand_address)?;
        }
        RiscvPrecompileKind::Add256 => {
            let params = cursor.expect_read_values::<4>(operand_address)?;
            cursor.expect_reads::<4>(params[0])?;
            cursor.expect_reads::<4>(params[1])?;
            cursor.expect_writes::<4>(params[3])?;
        }
    }
    cursor.finish()
}

#[cfg(test)]
fn validate_zisk_main_precompile_memory_accesses_if_required(
    row: usize,
    report: &GuestMachineReport,
    instruction: &ZiskMainInstruction,
    effects: ZiskMainReportEffects<'_>,
    operand_address: u64,
) -> Result<(), GuestPcTraceBackendError> {
    if !zisk_main_precompile_memory_validation_required(instruction, effects) {
        return Ok(());
    }
    validate_zisk_main_precompile_memory_accesses(row, report, effects, operand_address)
}

fn zisk_main_precompile_memory_validation_required(
    instruction: &ZiskMainInstruction,
    effects: ZiskMainReportEffects<'_>,
) -> bool {
    instruction.is_precompiled || !effects.precompile_memory_accesses.is_empty()
}

struct PrecompileMemoryAccessCursor<'a> {
    row: usize,
    accesses: &'a [GuestMemoryAccess],
    offset: usize,
}

impl PrecompileMemoryAccessCursor<'_> {
    fn expect_read_values<const N: usize>(
        &mut self,
        base_address: u64,
    ) -> Result<[u64; N], GuestPcTraceBackendError> {
        let mut values = [0_u64; N];
        for (index, value) in values.iter_mut().enumerate() {
            *value =
                self.expect_access(GuestMemoryAccessKind::Read, base_address + index as u64 * 8)?;
        }
        Ok(values)
    }

    fn expect_reads<const N: usize>(
        &mut self,
        base_address: u64,
    ) -> Result<(), GuestPcTraceBackendError> {
        self.expect_read_values::<N>(base_address).map(|_| ())
    }

    fn expect_writes<const N: usize>(
        &mut self,
        base_address: u64,
    ) -> Result<(), GuestPcTraceBackendError> {
        for index in 0..N {
            self.expect_access(
                GuestMemoryAccessKind::Write,
                base_address + index as u64 * 8,
            )?;
        }
        Ok(())
    }

    fn expect_access(
        &mut self,
        kind: GuestMemoryAccessKind,
        address: u64,
    ) -> Result<u64, GuestPcTraceBackendError> {
        let Some(access) = self.accesses.get(self.offset) else {
            return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                row: self.row,
                message: format!("missing precompile memory access {}", self.offset),
            });
        };
        if access.kind != kind || access.address != address || access.byte_len != 8 {
            return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                row: self.row,
                message: format!(
                    "expected precompile memory access {} as {:?} at {} byte length 8, found {:?} at {} byte length {}",
                    self.offset, kind, address, access.kind, access.address, access.byte_len
                ),
            });
        }
        self.offset += 1;
        Ok(access.value)
    }

    fn finish(self) -> Result<(), GuestPcTraceBackendError> {
        if self.offset != self.accesses.len() {
            return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                row: self.row,
                message: format!(
                    "expected {} precompile memory accesses, found {}",
                    self.offset,
                    self.accesses.len()
                ),
            });
        }
        Ok(())
    }
}

fn zisk_main_store_offset(
    row: usize,
    store: &ZiskMainStore,
) -> Result<i64, GuestPcTraceBackendError> {
    match store {
        ZiskMainStore::None => Ok(0),
        ZiskMainStore::Register(index) => Ok(i64::from(*index)),
        ZiskMainStore::Indirect(offset) => Ok(*offset),
        ZiskMainStore::Memory(address) => {
            let Ok(address_u64) = u64::try_from(*address) else {
                return Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row });
            };
            if !zisk_internal_memory_address(address_u64) {
                return Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row });
            }
            Ok(*address)
        }
    }
}

fn zisk_main_source_offset(
    _row: usize,
    source: ZiskMainSource,
) -> Result<i64, GuestPcTraceBackendError> {
    match source {
        ZiskMainSource::LastC => Ok(0),
        ZiskMainSource::Immediate(value) => Ok((value & 0xffff_ffff) as i64),
        ZiskMainSource::Register(index) => Ok(i64::from(index)),
        ZiskMainSource::Indirect(offset) => Ok(offset),
        ZiskMainSource::Memory(address) => Ok((address & 0xffff_ffff) as i64),
    }
}

fn zisk_main_source_high_limb(source: ZiskMainSource) -> u64 {
    match source {
        ZiskMainSource::Immediate(value) | ZiskMainSource::Memory(value) => value >> 32,
        ZiskMainSource::LastC | ZiskMainSource::Register(_) | ZiskMainSource::Indirect(_) => 0,
    }
}

fn zisk_main_b_address(
    row: usize,
    source: ZiskMainSource,
    a: u64,
) -> Result<i64, GuestPcTraceBackendError> {
    let offset = zisk_main_source_offset(row, source)?;
    if matches!(source, ZiskMainSource::Indirect(_)) {
        return zisk_main_indirect_address(offset, a)
            .ok_or(GuestPcTraceBackendError::UnsupportedZiskMainSource { row });
    }
    Ok(offset)
}

fn zisk_main_store_address(
    row: usize,
    store: &ZiskMainStore,
    a: u64,
) -> Result<i64, GuestPcTraceBackendError> {
    let offset = zisk_main_store_offset(row, store)?;
    if matches!(store, ZiskMainStore::Indirect(_)) {
        return zisk_main_indirect_address(offset, a)
            .ok_or(GuestPcTraceBackendError::UnsupportedZiskMainStore { row });
    }
    Ok(offset)
}

fn zisk_main_indirect_address(offset: i64, base: u64) -> Option<i64> {
    offset.checked_add((base & 0xffff_ffff) as i64)
}

fn low_bytes_value(value: u64, byte_len: usize) -> u64 {
    if byte_len >= 8 {
        value
    } else {
        value & ((1_u64 << (byte_len * 8)) - 1)
    }
}

fn zisk_main_op_result(op: ZiskMainOp, a: u64, b: u64) -> (u64, bool) {
    match op {
        ZiskMainOp::Flag => (0, true),
        ZiskMainOp::CopyB => (b, false),
        ZiskMainOp::Ltu => {
            if a < b {
                (1, true)
            } else {
                (0, false)
            }
        }
        ZiskMainOp::Lt => {
            if (a as i64) < (b as i64) {
                (1, true)
            } else {
                (0, false)
            }
        }
        ZiskMainOp::Eq => {
            if a == b {
                (1, true)
            } else {
                (0, false)
            }
        }
        ZiskMainOp::Add => (a.wrapping_add(b), false),
        ZiskMainOp::Sub => (a.wrapping_sub(b), false),
        ZiskMainOp::Mul => (a.wrapping_mul(b), false),
        ZiskMainOp::Mulh => (
            (((a as i64 as i128) * (b as i64 as i128)) >> 64) as u64,
            false,
        ),
        ZiskMainOp::Mulhsu => ((((a as i64 as i128) * (b as i128)) >> 64) as u64, false),
        ZiskMainOp::Mulhu => ((((a as u128) * (b as u128)) >> 64) as u64, false),
        ZiskMainOp::Div => signed_divide_result(a as i64, b as i64),
        ZiskMainOp::Divu => unsigned_divide_result(a, b),
        ZiskMainOp::Rem => signed_remainder_result(a as i64, b as i64),
        ZiskMainOp::Remu => unsigned_remainder_result(a, b),
        ZiskMainOp::AddW => (sign_extend_word((a as u32).wrapping_add(b as u32)), false),
        ZiskMainOp::SubW => (sign_extend_word((a as u32).wrapping_sub(b as u32)), false),
        ZiskMainOp::MulW => (sign_extend_word((a as u32).wrapping_mul(b as u32)), false),
        ZiskMainOp::DivW => signed_divide_word_result(a as u32 as i32, b as u32 as i32),
        ZiskMainOp::DivuW => unsigned_divide_word_result(a as u32, b as u32),
        ZiskMainOp::RemW => signed_remainder_word_result(a as u32 as i32, b as u32 as i32),
        ZiskMainOp::RemuW => unsigned_remainder_word_result(a as u32, b as u32),
        ZiskMainOp::And => (a & b, false),
        ZiskMainOp::Or => (a | b, false),
        ZiskMainOp::Xor => (a ^ b, false),
        ZiskMainOp::Sll => (a.wrapping_shl((b as u32) & 0x3f), false),
        ZiskMainOp::Srl => (a.wrapping_shr((b as u32) & 0x3f), false),
        ZiskMainOp::Sra => (((a as i64) >> ((b as u32) & 0x3f)) as u64, false),
        ZiskMainOp::SllW => (
            sign_extend_word((a as u32).wrapping_shl((b as u32) & 0x1f)),
            false,
        ),
        ZiskMainOp::SrlW => (
            sign_extend_word((a as u32).wrapping_shr((b as u32) & 0x1f)),
            false,
        ),
        ZiskMainOp::SraW => (
            sign_extend_word(((a as u32 as i32) >> ((b as u32) & 0x1f)) as u32),
            false,
        ),
        ZiskMainOp::SignExtendB => ((b as i8) as u64, false),
        ZiskMainOp::SignExtendH => ((b as i16) as u64, false),
        ZiskMainOp::SignExtendW => ((b as i32) as u64, false),
        ZiskMainOp::DmaMemCpy
        | ZiskMainOp::DmaMemCmp
        | ZiskMainOp::DmaInputCpy
        | ZiskMainOp::DmaXMemCpy
        | ZiskMainOp::DmaXMemCmp
        | ZiskMainOp::DmaXMemSet
        | ZiskMainOp::Add256
        | ZiskMainOp::Keccak
        | ZiskMainOp::Arith256
        | ZiskMainOp::Arith256Mod
        | ZiskMainOp::Secp256k1Add
        | ZiskMainOp::Secp256k1Dbl => (0, false),
    }
}

fn zisk_main_instruction_result(
    row: usize,
    instruction: &ZiskMainInstruction,
    a: u64,
    b: u64,
    effects: ZiskMainReportEffects<'_>,
) -> Result<(u64, bool), GuestPcTraceBackendError> {
    match instruction.op {
        ZiskMainOp::DmaMemCpy
        | ZiskMainOp::DmaInputCpy
        | ZiskMainOp::DmaXMemCpy
        | ZiskMainOp::DmaXMemSet => zisk_main_dma_result(row, instruction, effects, Some(a)),
        ZiskMainOp::DmaMemCmp | ZiskMainOp::DmaXMemCmp => {
            zisk_main_dma_result(row, instruction, effects, None)
        }
        ZiskMainOp::Add256 if instruction.is_precompiled => {
            zisk_main_add256_result(row, instruction, effects)
        }
        ZiskMainOp::Keccak
        | ZiskMainOp::Arith256
        | ZiskMainOp::Arith256Mod
        | ZiskMainOp::Secp256k1Add
        | ZiskMainOp::Secp256k1Dbl
            if instruction.is_precompiled =>
        {
            Ok((0, false))
        }
        _ => Ok(zisk_main_op_result(instruction.op, a, b)),
    }
}

fn zisk_main_dma_result(
    row: usize,
    instruction: &ZiskMainInstruction,
    effects: ZiskMainReportEffects<'_>,
    expected_value: Option<u64>,
) -> Result<(u64, bool), GuestPcTraceBackendError> {
    match instruction.store {
        ZiskMainStore::Register(index) => {
            let [write] = effects.register_writes else {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!(
                        "DMA row reported {} register writes",
                        effects.register_writes.len()
                    ),
                });
            };
            if write.index != index {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!("expected DMA result in x{index}, found x{}", write.index),
                });
            }
            if let Some(expected_value) = expected_value {
                if write.value != expected_value {
                    return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                        row,
                        message: format!(
                            "expected DMA result {expected_value}, found {}",
                            write.value
                        ),
                    });
                }
            }
            Ok((write.value, false))
        }
        ZiskMainStore::None => {
            if !effects.register_writes.is_empty() {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: "DMA row with no store reported register writes".to_owned(),
                });
            }
            let Some(value) = expected_value else {
                return Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row });
            };
            Ok((value, false))
        }
        ZiskMainStore::Indirect(_) | ZiskMainStore::Memory(_) => {
            Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row })
        }
    }
}

fn zisk_main_add256_result(
    row: usize,
    instruction: &ZiskMainInstruction,
    effects: ZiskMainReportEffects<'_>,
) -> Result<(u64, bool), GuestPcTraceBackendError> {
    let Some(result) = effects.precompile_result else {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: "Add256 row missing precompile result".to_owned(),
        });
    };
    match instruction.store {
        ZiskMainStore::None => Ok((result, false)),
        ZiskMainStore::Register(index) => {
            let [write] = effects.register_writes else {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!(
                        "Add256 row reported {} register writes",
                        effects.register_writes.len()
                    ),
                });
            };
            if write.index != index {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!("expected Add256 result in x{index}, found x{}", write.index),
                });
            }
            Ok((result, false))
        }
        ZiskMainStore::Indirect(_) | ZiskMainStore::Memory(_) => {
            Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row })
        }
    }
}

fn sign_extend_word(value: u32) -> u64 {
    (value as i32 as i64) as u64
}

fn unsigned_divide_result(dividend: u64, divisor: u64) -> (u64, bool) {
    if divisor == 0 {
        (u64::MAX, true)
    } else {
        (dividend / divisor, false)
    }
}

fn unsigned_remainder_result(dividend: u64, divisor: u64) -> (u64, bool) {
    if divisor == 0 {
        (dividend, true)
    } else {
        (dividend % divisor, false)
    }
}

fn signed_divide_result(dividend: i64, divisor: i64) -> (u64, bool) {
    if divisor == 0 {
        (-1_i64 as u64, true)
    } else if dividend == i64::MIN && divisor == -1 {
        (i64::MIN as u64, false)
    } else {
        ((dividend / divisor) as u64, false)
    }
}

fn signed_remainder_result(dividend: i64, divisor: i64) -> (u64, bool) {
    if divisor == 0 {
        (dividend as u64, true)
    } else if dividend == i64::MIN && divisor == -1 {
        (0, false)
    } else {
        ((dividend % divisor) as u64, false)
    }
}

fn unsigned_divide_word_result(dividend: u32, divisor: u32) -> (u64, bool) {
    if divisor == 0 {
        (u64::MAX, true)
    } else {
        (sign_extend_word(dividend / divisor), false)
    }
}

fn unsigned_remainder_word_result(dividend: u32, divisor: u32) -> (u64, bool) {
    if divisor == 0 {
        (sign_extend_word(dividend), true)
    } else {
        (sign_extend_word(dividend % divisor), false)
    }
}

fn signed_divide_word_result(dividend: i32, divisor: i32) -> (u64, bool) {
    if divisor == 0 {
        (u64::MAX, true)
    } else if dividend == i32::MIN && divisor == -1 {
        (sign_extend_word(dividend as u32), false)
    } else {
        (sign_extend_word((dividend / divisor) as u32), false)
    }
}

fn signed_remainder_word_result(dividend: i32, divisor: i32) -> (u64, bool) {
    if divisor == 0 {
        (sign_extend_word(dividend as u32), true)
    } else if dividend == i32::MIN && divisor == -1 {
        (0, false)
    } else {
        (sign_extend_word((dividend % divisor) as u32), false)
    }
}

fn validate_zisk_main_next_pc(
    row: usize,
    instruction: &ZiskMainInstruction,
    expected_report_next_pc: u64,
    c: u64,
    flag: bool,
) -> Result<(), GuestPcTraceBackendError> {
    let expected_next_pc = if instruction.set_pc {
        c.wrapping_add_signed(instruction.jmp_offset1)
    } else if flag {
        instruction.pc.wrapping_add_signed(instruction.jmp_offset1)
    } else {
        instruction.pc.wrapping_add_signed(instruction.jmp_offset2)
    };
    if expected_report_next_pc != expected_next_pc {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "expected next pc {expected_next_pc}, found {}",
                expected_report_next_pc
            ),
        });
    }
    Ok(())
}

fn apply_zisk_main_store(
    row: usize,
    instruction: &ZiskMainInstruction,
    c: u64,
    effects: ZiskMainReportEffects<'_>,
    expected_next_pc: u64,
    state: &mut ZiskMainTraceState,
) -> Result<(), GuestPcTraceBackendError> {
    match instruction.store {
        ZiskMainStore::None => {
            if !effects.register_writes.is_empty() {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: "store none row reported register writes".to_owned(),
                });
            }
        }
        ZiskMainStore::Register(index) => {
            let store_value = zisk_main_store_value(instruction, c);
            let [write] = effects.register_writes else {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!(
                        "store register row reported {} register writes",
                        effects.register_writes.len()
                    ),
                });
            };
            if write.index != index || write.value != store_value {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!(
                        "expected x{index} = {store_value}, found x{} = {}",
                        write.index, write.value
                    ),
                });
            }
            state.registers[usize::from(index)] = store_value;
        }
        ZiskMainStore::Indirect(_) => {
            if !effects.register_writes.is_empty() {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: "store indirect row reported register writes".to_owned(),
                });
            }
        }
        ZiskMainStore::Memory(address) => {
            let store_value = zisk_main_store_value(instruction, c);
            let Ok(address) = u64::try_from(address) else {
                return Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row });
            };
            if !zisk_internal_memory_address(address) {
                return Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row });
            }
            if !effects.register_writes.is_empty() {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: "store memory row reported register writes".to_owned(),
                });
            }
            state
                .internal_memory
                .insert(address, store_value)
                .map_err(|_| GuestPcTraceBackendError::UnsupportedZiskMainStore { row })?;
        }
    }
    state.last_c = c;
    state.next_pc = expected_next_pc;
    Ok(())
}

fn zisk_main_store_value(instruction: &ZiskMainInstruction, c: u64) -> u64 {
    if instruction.store_pc {
        instruction.pc.wrapping_add_signed(instruction.jmp_offset2)
    } else {
        c
    }
}

#[cfg(test)]
fn direct_zisk_main_segment_boundary_c(
    reports: &[GuestMachineReport],
    lookahead_instruction: Option<RiscvInstruction>,
    current_seed: &ZiskMainSegmentSeed,
    boundary_registers: Option<&[u64; 32]>,
) -> Result<Result<u64, ZiskMainDirectSeedLiftMissReason>, GuestPcTraceBackendError> {
    direct_zisk_main_segment_boundary_c_from_tail(
        reports.len(),
        reports.last(),
        lookahead_instruction,
        current_seed,
        boundary_registers,
    )
}

fn direct_zisk_main_segment_boundary_c_from_tail(
    report_count: usize,
    last_report: Option<&GuestMachineReport>,
    lookahead_instruction: Option<RiscvInstruction>,
    current_seed: &ZiskMainSegmentSeed,
    boundary_registers: Option<&[u64; 32]>,
) -> Result<Result<u64, ZiskMainDirectSeedLiftMissReason>, GuestPcTraceBackendError> {
    let Some(report) = last_report else {
        return Ok(Err(ZiskMainDirectSeedLiftMissReason::EmptySegment));
    };
    if current_seed.initial_state.pending_dma.is_some() && report_count == 1 {
        return Ok(Err(
            ZiskMainDirectSeedLiftMissReason::PendingDmaSingleReport,
        ));
    }
    if matches!(report.instruction, RiscvInstruction::Amo { .. }) {
        return Ok(Err(ZiskMainDirectSeedLiftMissReason::AmoBoundary));
    }
    if matches!(
        report.instruction,
        RiscvInstruction::StoreConditional { .. }
    ) {
        return Ok(Err(
            ZiskMainDirectSeedLiftMissReason::StoreConditionalBoundary,
        ));
    }
    if matches!(report.instruction, RiscvInstruction::ZiskDmaPrepare { .. })
        && lookahead_instruction.is_none()
    {
        return Ok(Err(
            ZiskMainDirectSeedLiftMissReason::DmaPrepareMissingLookahead,
        ));
    }
    if let Some(boundary_c) = direct_zisk_main_store_boundary_c(report, boundary_registers) {
        return Ok(Ok(boundary_c));
    }

    let lowered = lower_single_zisk_main_report_row(0, report, || lookahead_instruction)?;
    Ok(
        direct_zisk_main_report_boundary_c(report, &lowered.instruction)
            .ok_or(ZiskMainDirectSeedLiftMissReason::BoundaryCUnavailable),
    )
}

fn direct_zisk_main_store_boundary_c(
    report: &GuestMachineReport,
    boundary_registers: Option<&[u64; 32]>,
) -> Option<u64> {
    let RiscvInstruction::Store { rs2, .. } = report.instruction else {
        return None;
    };
    if rs2 == 0 {
        return Some(0);
    }
    boundary_registers.map(|registers| registers[usize::from(rs2)])
}

fn direct_zisk_main_report_boundary_c(
    report: &GuestMachineReport,
    instruction: &ZiskMainInstruction,
) -> Option<u64> {
    if instruction.store_pc || matches!(instruction.store, ZiskMainStore::None) {
        return direct_zisk_main_report_result_c(report, instruction);
    }
    if let ZiskMainStore::Register(index) = instruction.store {
        let [write] = report.register_writes.as_slice() else {
            return None;
        };
        return (write.index == index).then_some(write.value);
    }
    direct_zisk_main_full_width_memory_store_c(report, instruction)
}

fn direct_zisk_main_report_result_c(
    report: &GuestMachineReport,
    instruction: &ZiskMainInstruction,
) -> Option<u64> {
    if instruction.set_pc {
        return Some(wrapping_sub_signed(report.next_pc, instruction.jmp_offset1));
    }
    if matches!(
        instruction.op,
        ZiskMainOp::Eq | ZiskMainOp::Lt | ZiskMainOp::Ltu
    ) {
        return direct_zisk_main_branch_c(report, instruction);
    }
    match instruction.op {
        ZiskMainOp::Flag => Some(0),
        ZiskMainOp::CopyB => direct_zisk_main_source_value_without_state(instruction.b),
        ZiskMainOp::Add256 if instruction.is_precompiled => report.precompile_result(),
        ZiskMainOp::Keccak
        | ZiskMainOp::Arith256
        | ZiskMainOp::Arith256Mod
        | ZiskMainOp::Secp256k1Add
        | ZiskMainOp::Secp256k1Dbl
            if instruction.is_precompiled =>
        {
            Some(0)
        }
        _ => None,
    }
}

fn direct_zisk_main_branch_c(
    report: &GuestMachineReport,
    instruction: &ZiskMainInstruction,
) -> Option<u64> {
    let flag_next_pc = instruction.pc.wrapping_add_signed(instruction.jmp_offset1);
    let fallthrough_next_pc = instruction.pc.wrapping_add_signed(instruction.jmp_offset2);
    if flag_next_pc == fallthrough_next_pc {
        return None;
    }
    if report.next_pc == flag_next_pc {
        Some(1)
    } else if report.next_pc == fallthrough_next_pc {
        Some(0)
    } else {
        None
    }
}

fn direct_zisk_main_source_value_without_state(source: ZiskMainSource) -> Option<u64> {
    match source {
        ZiskMainSource::Immediate(value) => Some(value),
        ZiskMainSource::LastC
        | ZiskMainSource::Memory(_)
        | ZiskMainSource::Register(_)
        | ZiskMainSource::Indirect(_) => None,
    }
}

fn direct_zisk_main_full_width_memory_store_c(
    report: &GuestMachineReport,
    instruction: &ZiskMainInstruction,
) -> Option<u64> {
    if !matches!(instruction.store, ZiskMainStore::Indirect(_)) || instruction.ind_width != 8 {
        return None;
    }
    let mut writes = report
        .memory_accesses
        .iter()
        .filter(|access| access.kind == GuestMemoryAccessKind::Write && access.byte_len == 8);
    let write = writes.next()?;
    if writes.next().is_some() {
        return None;
    }
    Some(write.value)
}

fn wrapping_sub_signed(value: u64, offset: i64) -> u64 {
    if offset >= 0 {
        value.wrapping_sub(offset as u64)
    } else {
        value.wrapping_add(offset.unsigned_abs())
    }
}

fn write_layout_pc_trace(
    layout: &WitnessTraceLayout,
    reports: &[crate::guest_machine::GuestMachineReport],
    output: &mut [u8],
) -> Result<Option<usize>, GuestPcTraceBackendError> {
    let Some(columns) = guest_trace_columns(layout)? else {
        return Ok(None);
    };
    if reports.len() > layout.row_count() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len: layout_trace_byte_len(reports.len(), layout.column_count()),
            output_len: output.len(),
        });
    }

    let mut builder = layout
        .trace_builder()
        .map_err(GuestPcTraceBackendError::TraceBuild)?;
    for (row, report) in reports.iter().enumerate() {
        write_report_columns(&mut builder, row, report, &columns)?;
    }
    let trace = builder.build();
    let produced_len =
        trace
            .values()
            .len()
            .checked_mul(8)
            .ok_or(GuestPcTraceBackendError::OutputOverflow {
                produced_len: usize::MAX,
                output_len: output.len(),
            })?;
    if produced_len > output.len() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len,
            output_len: output.len(),
        });
    }
    for (index, value) in trace.values().iter().copied().enumerate() {
        let offset = index * 8;
        output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    Ok(Some(produced_len))
}

fn write_report_columns(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    report: &GuestMachineReport,
    columns: &GuestTraceColumns<'_>,
) -> Result<(), GuestPcTraceBackendError> {
    if let Some(pc_columns) = &columns.pc {
        write_column(builder, row, &pc_columns.pc, report.address)?;
        write_column(builder, row, &pc_columns.next_pc, report.next_pc)?;
    }
    if let Some(register_write_columns) = &columns.register_write {
        write_register_columns(
            builder,
            row,
            &report.register_writes,
            register_write_columns,
        )?;
    }
    if let Some(memory_read_columns) = &columns.memory_read {
        write_memory_columns(
            builder,
            row,
            &report.memory_accesses,
            GuestMemoryAccessKind::Read,
            memory_read_columns,
        )?;
    }
    if let Some(memory_write_columns) = &columns.memory_write {
        write_memory_columns(
            builder,
            row,
            &report.memory_accesses,
            GuestMemoryAccessKind::Write,
            memory_write_columns,
        )?;
    }
    Ok(())
}

fn write_register_columns(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    register_writes: &[GuestRegisterWrite],
    columns: &RegisterWriteColumns<'_>,
) -> Result<(), GuestPcTraceBackendError> {
    match register_writes {
        [] => Ok(()),
        [write] => {
            write_column(builder, row, &columns.index, u64::from(write.index))?;
            write_column(builder, row, &columns.value, write.value)
        }
        writes => Err(GuestPcTraceBackendError::TooManyRegisterWrites {
            row,
            found: writes.len(),
        }),
    }
}

fn write_memory_columns(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    memory_accesses: &[GuestMemoryAccess],
    kind: GuestMemoryAccessKind,
    columns: &MemoryAccessColumns<'_>,
) -> Result<(), GuestPcTraceBackendError> {
    let mut matching = memory_accesses.iter().filter(|access| access.kind == kind);
    let Some(access) = matching.next() else {
        return Ok(());
    };
    if matching.next().is_some() {
        return Err(GuestPcTraceBackendError::TooManyMemoryAccesses {
            row,
            kind,
            found: memory_accesses
                .iter()
                .filter(|access| access.kind == kind)
                .count(),
        });
    }
    write_column(builder, row, &columns.address, access.address)?;
    write_column(builder, row, &columns.value, access.value)?;
    write_column(builder, row, &columns.byte_len, u64::from(access.byte_len))
}

fn write_column(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    column: &TraceColumnTarget<'_>,
    value: u64,
) -> Result<(), GuestPcTraceBackendError> {
    let value = canonical_trace_value(row, column.name(), value)?;
    builder
        .write_trusted_resolved_scalar_value(row, column.resolved(), value)
        .map_err(GuestPcTraceBackendError::TraceBuild)
}

fn write_optional_column(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    column: &Option<TraceColumnTarget<'_>>,
    value: u64,
) -> Result<(), GuestPcTraceBackendError> {
    if let Some(column) = column {
        write_column(builder, row, column, value)?;
    }
    Ok(())
}

fn write_wide_column(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    column: &TraceColumnTarget<'_>,
    value: u64,
) -> Result<(), GuestPcTraceBackendError> {
    let values = [
        canonical_trace_value(row, column.name(), value & 0xffff_ffff)?,
        canonical_trace_value(row, column.name(), value >> 32)?,
    ];
    builder
        .write_trusted_resolved_pair_values(row, column.resolved(), values)
        .map_err(GuestPcTraceBackendError::TraceBuild)
}

fn write_optional_wide_column(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    column: &Option<TraceColumnTarget<'_>>,
    value: u64,
) -> Result<(), GuestPcTraceBackendError> {
    if let Some(column) = column {
        write_wide_column(builder, row, column, value)?;
    }
    Ok(())
}

fn felt_limbs_u64(value: u64) -> [Felt; 2] {
    [
        Felt::from_u64(value & 0xffff_ffff),
        Felt::from_u64(value >> 32),
    ]
}

fn write_optional_signed_column(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    column: &Option<TraceColumnTarget<'_>>,
    value: i64,
) -> Result<(), GuestPcTraceBackendError> {
    let Some(column) = column else {
        return Ok(());
    };
    let value = signed_trace_value(row, column.name(), value)?;
    builder
        .write_trusted_resolved_scalar_value(row, column.resolved(), value)
        .map_err(GuestPcTraceBackendError::TraceBuild)
}

fn guest_trace_columns<'a>(
    layout: &'a WitnessTraceLayout,
) -> Result<Option<GuestTraceColumns<'a>>, GuestPcTraceBackendError> {
    let columns = GuestTraceColumns {
        pc: pc_trace_columns(layout)?,
        register_write: register_write_columns(layout)?,
        memory_read: memory_access_columns(
            layout,
            "mem_read_address",
            "mem_read_value",
            "mem_read_byte_len",
        )?,
        memory_write: memory_access_columns(
            layout,
            "mem_write_address",
            "mem_write_value",
            "mem_write_byte_len",
        )?,
    };
    if columns.pc.is_none()
        && columns.register_write.is_none()
        && columns.memory_read.is_none()
        && columns.memory_write.is_none()
    {
        Ok(None)
    } else {
        Ok(Some(columns))
    }
}

fn zisk_main_trace_columns<'a>(
    layout: &'a WitnessTraceLayout,
) -> Result<Option<ZiskMainTraceColumns<'a>>, GuestPcTraceBackendError> {
    if !is_zisk_main_trace_layout(layout) {
        return Ok(None);
    }
    Ok(Some(ZiskMainTraceColumns {
        a: required_vector_trace_column_target(layout, "a", 2)?,
        b: required_vector_trace_column_target(layout, "b", 2)?,
        c: required_vector_trace_column_target(layout, "c", 2)?,
        flag: required_trace_column_target(layout, "flag")?,
        pc: required_trace_column_target(layout, "pc")?,
        a_src_imm: trace_column_target(layout, "a_src_imm")?,
        a_offset_imm0: trace_column_target(layout, "a_offset_imm0")?,
        a_imm1: trace_column_target_aliases(layout, &["a_imm1", "air.a_imm1"])?,
        b_src_imm: trace_column_target(layout, "b_src_imm")?,
        b_imm1: trace_column_target_aliases(layout, &["b_imm1", "air.b_imm1"])?,
        a_src_reg: trace_column_target(layout, "a_src_reg")?,
        b_src_reg: trace_column_target(layout, "b_src_reg")?,
        a_src_mem: trace_column_target(layout, "a_src_mem")?,
        b_src_mem: trace_column_target(layout, "b_src_mem")?,
        b_src_ind: trace_column_target(layout, "b_src_ind")?,
        b_offset_imm0: trace_column_target(layout, "b_offset_imm0")?,
        addr1: trace_column_target_aliases(layout, &["addr1", "air.addr1"])?,
        addr2: trace_column_target_aliases(layout, &["addr2", "air.addr2"])?,
        store_reg: trace_column_target(layout, "store_reg")?,
        store_mem: trace_column_target(layout, "store_mem")?,
        store_ind: trace_column_target(layout, "store_ind")?,
        store_offset: trace_column_target(layout, "store_offset")?,
        store_pc: trace_column_target(layout, "store_pc")?,
        set_pc: trace_column_target(layout, "set_pc")?,
        op: required_trace_column_target(layout, "op")?,
        jmp_offset1: trace_column_target(layout, "jmp_offset1")?,
        jmp_offset2: trace_column_target(layout, "jmp_offset2")?,
        ind_width: trace_column_target(layout, "ind_width")?,
        m32: trace_column_target(layout, "m32")?,
        is_external_op: trace_column_target(layout, "is_external_op")?,
        is_precompiled: trace_column_target(layout, "is_precompiled")?,
        a_reg_prev_mem_step: trace_column_target(layout, "a_reg_prev_mem_step")?,
        b_reg_prev_mem_step: trace_column_target(layout, "b_reg_prev_mem_step")?,
        store_reg_prev_mem_step: trace_column_target(layout, "store_reg_prev_mem_step")?,
        store_reg_prev_value: vector_trace_column_target(layout, "store_reg_prev_value", 2)?,
    }))
}

fn is_zisk_main_trace_layout(layout: &WitnessTraceLayout) -> bool {
    [
        "a",
        "b",
        "c",
        "pc",
        "op",
        "store_pc",
        "set_pc",
        "a_src_reg",
        "b_src_reg",
        "store_reg",
    ]
    .iter()
    .all(|name| has_trace_column(layout, name))
}

fn has_trace_column(layout: &WitnessTraceLayout, name: &str) -> bool {
    layout.columns().iter().any(|column| column.name() == name)
}

fn pc_trace_columns<'a>(
    layout: &'a WitnessTraceLayout,
) -> Result<Option<PcTraceColumns<'a>>, GuestPcTraceBackendError> {
    let pc = trace_column_target(layout, "pc")?;
    let next_pc = trace_column_target(layout, "next_pc")?;
    match (pc, next_pc) {
        (None, None) => Ok(None),
        (Some(_), None) => Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "missing next_pc column".to_owned(),
        }),
        (None, Some(_)) => Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "missing pc column".to_owned(),
        }),
        (Some(pc), Some(next_pc)) => {
            if pc.trace_column() == next_pc.trace_column() {
                return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: format!(
                        "pc and next_pc columns share trace column {}",
                        pc.trace_column()
                    ),
                });
            }
            Ok(Some(PcTraceColumns { pc, next_pc }))
        }
    }
}

fn register_write_columns<'a>(
    layout: &'a WitnessTraceLayout,
) -> Result<Option<RegisterWriteColumns<'a>>, GuestPcTraceBackendError> {
    let index = trace_column_target(layout, "reg_write_index")?;
    let value = trace_column_target(layout, "reg_write_value")?;
    match (index, value) {
        (None, None) => Ok(None),
        (Some(_), None) => Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "missing reg_write_value column".to_owned(),
        }),
        (None, Some(_)) => Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "missing reg_write_index column".to_owned(),
        }),
        (Some(index), Some(value)) => Ok(Some(RegisterWriteColumns { index, value })),
    }
}

fn memory_access_columns<'a>(
    layout: &'a WitnessTraceLayout,
    address_name: &str,
    value_name: &str,
    byte_len_name: &str,
) -> Result<Option<MemoryAccessColumns<'a>>, GuestPcTraceBackendError> {
    let address = trace_column_target(layout, address_name)?;
    let value = trace_column_target(layout, value_name)?;
    let byte_len = trace_column_target(layout, byte_len_name)?;
    if address.is_none() && value.is_none() && byte_len.is_none() {
        return Ok(None);
    }
    let address = address.ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: format!("missing {address_name} column"),
    })?;
    let value = value.ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: format!("missing {value_name} column"),
    })?;
    let byte_len = byte_len.ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: format!("missing {byte_len_name} column"),
    })?;
    Ok(Some(MemoryAccessColumns {
        address,
        value,
        byte_len,
    }))
}

fn required_trace_column_target<'a>(
    layout: &'a WitnessTraceLayout,
    name: &str,
) -> Result<TraceColumnTarget<'a>, GuestPcTraceBackendError> {
    trace_column_target(layout, name)?.ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!("missing {name} column"),
        }
    })
}

fn required_vector_trace_column_target<'a>(
    layout: &'a WitnessTraceLayout,
    name: &str,
    dimension: usize,
) -> Result<TraceColumnTarget<'a>, GuestPcTraceBackendError> {
    vector_trace_column_target(layout, name, dimension)?.ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!("missing {name} column"),
        }
    })
}

fn vector_trace_column_target<'a>(
    layout: &'a WitnessTraceLayout,
    name: &str,
    dimension: usize,
) -> Result<Option<TraceColumnTarget<'a>>, GuestPcTraceBackendError> {
    let mut matches = layout
        .columns()
        .iter()
        .filter(|column| column.name() == name);
    let Some(column) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!("column {name} is ambiguous"),
        });
    }
    if column.dimension() != dimension {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "column {name} must have dimension {dimension}, found {}",
                column.dimension()
            ),
        });
    }
    Ok(Some(TraceColumnTarget {
        column: layout.resolved_column(column),
    }))
}

fn trace_column_target<'a>(
    layout: &'a WitnessTraceLayout,
    name: &str,
) -> Result<Option<TraceColumnTarget<'a>>, GuestPcTraceBackendError> {
    let mut matches = layout
        .columns()
        .iter()
        .filter(|column| column.name() == name);
    let Some(column) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!("column {name} is ambiguous"),
        });
    }
    if column.dimension() != 1 {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "column {name} must have dimension 1, found {}",
                column.dimension()
            ),
        });
    }
    Ok(Some(TraceColumnTarget {
        column: layout.resolved_column(column),
    }))
}

fn trace_column_target_aliases<'a>(
    layout: &'a WitnessTraceLayout,
    names: &[&str],
) -> Result<Option<TraceColumnTarget<'a>>, GuestPcTraceBackendError> {
    let mut matches = layout
        .columns()
        .iter()
        .filter(|column| names.iter().any(|name| column.name() == *name));
    let Some(column) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!("column {} is ambiguous", names.join("/")),
        });
    }
    if column.dimension() != 1 {
        return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!(
                "column {} must have dimension 1, found {}",
                names.join("/"),
                column.dimension()
            ),
        });
    }
    Ok(Some(TraceColumnTarget {
        column: layout.resolved_column(column),
    }))
}

fn canonical_trace_value(
    row: usize,
    column: &str,
    value: u64,
) -> Result<Felt, GuestPcTraceBackendError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => GuestPcTraceBackendError::NonCanonicalTraceValue {
            row,
            column: column.to_owned(),
            value,
        },
    })
}

fn signed_trace_value(
    row: usize,
    column: &str,
    value: i64,
) -> Result<Felt, GuestPcTraceBackendError> {
    if value >= 0 {
        canonical_trace_value(row, column, value as u64)
    } else {
        Ok(-canonical_trace_value(row, column, value.unsigned_abs())?)
    }
}

#[cfg(test)]
mod tests;

#[cfg(feature = "cuda")]
use std::collections::HashSet;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(feature = "cuda")]
use lzvm_accel::CudaDeviceBuffer;
use lzvm_artifacts::fixed::FixedColumns;
use lzvm_artifacts::global_info::{GlobalInfo, NamedStageValue};
use lzvm_artifacts::hint_program::{
    source_unimplemented_hint_name, HintProgram, SOURCE_ASSIGNMENT_CHECK_HINT,
};
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::public_values::{read_public_values_file, PublicValues, PublicValuesError};
use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::trace_bundle::TraceBundleSource;
use lzvm_field::{Ext3, Felt, FieldError};

use crate::fixed_material::{
    load_execution_unit_fixed_columns_material, FixedColumnsMaterialError,
};
use crate::fri_polynomial::{
    build_fri_domain_points, FriPolynomialError, FriPolynomialZerofierTable,
};
use crate::global_constraints::GlobalConstraintInputs;
#[cfg(feature = "cuda")]
use crate::guest_pc_trace_backend::{
    build_guest_pc_trace_stage_source_devices,
    build_guest_pc_trace_stage_source_devices_from_device_material_timing,
    GuestPcDeviceSourceBuildTiming, GuestPcTraceDeviceSegmentMaterial,
    GuestPcTraceDeviceTraceBuilder,
};
use crate::guest_pc_trace_backend::{
    for_each_guest_pc_trace_segment_collecting_proof_values_with_context,
    for_each_guest_pc_trace_segment_with_context,
    run_guest_pc_trace_runtime_proof_values_with_context, run_guest_pc_trace_segments_with_context,
    GuestPcTraceBackend, GuestPcTraceSegmentRunOutput, GuestPcTraceSegmentStreamError,
    GuestPcTraceStreamTiming,
};
use crate::hint_eval::{
    regular_hint_input_requirements, resolve_global_hint_program,
    resolve_regular_hint_program_for_row, HintEvalError,
};
#[cfg(not(feature = "cuda"))]
use crate::regular_constraints::evaluate_regular_constraints_first_violations_with_acceleration;
#[cfg(feature = "cuda")]
use crate::regular_constraints::evaluate_regular_constraints_first_violations_with_cuda_fixed_values;
#[cfg(feature = "cuda")]
use crate::regular_constraints::try_evaluate_regular_constraints_cuda_base;
use crate::regular_constraints::{
    RegularColumnMatrix, RegularConstraintEvalError, RegularConstraintInputs, RegularStageColumns,
};
use crate::source_assignment_hints::validate_source_assignment_hints;
use crate::source_lookup_hints::{SourceLookupBalance, SourceLookupHintError};
#[cfg(feature = "cuda")]
use crate::witness_commitment::{
    commit_witness_stage_source_devices_and_indexed_timing_external_source_with_leaf_workspace_cache,
    commit_witness_stage_source_devices_and_indexed_timing_with_leaf_workspace_cache,
    commit_witness_stage_values_with_source_devices_and_indexed_timing,
    commit_witness_stage_values_with_source_devices_and_workers,
    commit_witness_stage_values_with_source_devices_reusing_cached_stages_and_indexed_timing,
    commit_witness_stage_values_with_source_devices_reusing_cached_stages_and_workers,
    retain_device_buffer, retained_source_device_limit, WitnessRetainedDeviceBuffer,
    WitnessStageCommitmentError, WitnessStageCommitmentReuseCache, WitnessStageLeafError,
    WitnessStageLeafWorkspaceCache, WitnessStageRetainedSourceDevice, WitnessStageSourceDevice,
    WitnessStageSourceDeviceView,
};
#[cfg(not(feature = "cuda"))]
use crate::witness_commitment::{
    commit_witness_stage_values_with_workers,
    commit_witness_stage_values_with_workers_and_indexed_timing,
};
use crate::witness_commitment::{
    commit_witness_trace_stages_with_workers, WitnessIndexedStageCommitTiming,
    WitnessStageCommitTiming, WitnessTraceCommitmentError, WitnessTraceCommitments,
};
use crate::witness_layout::{
    derive_witness_trace_layout, WitnessTraceLayout, WitnessTraceLayoutError,
    WitnessTraceStageValues,
};
use crate::witness_loader::{
    load_witness_library, WitnessBackend, WitnessCallError, WitnessComputeContext,
    WitnessLoadError, WitnessTraceProofValue, WitnessTraceUnitValue,
};
use crate::witness_runner::{
    run_witness_trace_output_with_context, trace_output_byte_len, WitnessTraceRunError,
};
use crate::witness_trace::{parse_witness_trace, WitnessTraceBuffer};
use crate::{ProveExecutionPlan, ProveExecutionUnitArtifacts, ProvePassRequest, ProveUnitSchedule};

mod proof_value_dependency;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProveTraceIdentity {
    unit_index: usize,
    trace_instance_index: u32,
}

impl ProveTraceIdentity {
    fn new(unit_index: usize, trace_instance_index: u32) -> Self {
        Self {
            unit_index,
            trace_instance_index,
        }
    }

    fn unit_index(&self) -> usize {
        self.unit_index
    }

    fn trace_instance_index(&self) -> u32 {
        self.trace_instance_index
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveWitnessCommitments {
    identity: ProveTraceIdentity,
    input_byte_count: usize,
    trace_rows: usize,
    trace_columns: usize,
    stage_commitments: WitnessTraceCommitments,
}

impl ProveWitnessCommitments {
    pub fn unit_index(&self) -> usize {
        self.identity.unit_index()
    }

    pub fn trace_instance_index(&self) -> u32 {
        self.identity.trace_instance_index()
    }

    pub fn input_byte_count(&self) -> usize {
        self.input_byte_count
    }

    pub fn trace_row_count(&self) -> usize {
        self.trace_rows
    }

    pub fn trace_column_count(&self) -> usize {
        self.trace_columns
    }

    pub fn stage_commitments(&self) -> &WitnessTraceCommitments {
        &self.stage_commitments
    }

    pub fn with_trace_instance_index(mut self, trace_instance_index: u32) -> Self {
        self.identity.trace_instance_index = trace_instance_index;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ProveWitnessTraceCommitments {
    commitments: ProveWitnessCommitments,
    trace: Option<WitnessTraceBuffer>,
    trace_constraint_checks: ProveWitnessTraceConstraintChecks,
    #[cfg(feature = "cuda")]
    stage_source_devices: Vec<WitnessStageRetainedSourceDevice>,
    #[cfg(feature = "cuda")]
    guest_pc_device_descriptor_buffer: Option<WitnessRetainedDeviceBuffer>,
    #[cfg(feature = "cuda")]
    guest_pc_device_segment_material: Option<GuestPcTraceDeviceSegmentMaterial>,
    publics: Vec<Felt>,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
}

impl PartialEq for ProveWitnessTraceCommitments {
    fn eq(&self, other: &Self) -> bool {
        let base_equal = self.commitments == other.commitments
            && self.trace == other.trace
            && self.trace_constraint_checks == other.trace_constraint_checks
            && self.publics == other.publics
            && self.auxiliary_inputs == other.auxiliary_inputs;
        #[cfg(feature = "cuda")]
        {
            base_equal
                && self.guest_pc_device_segment_material == other.guest_pc_device_segment_material
        }
        #[cfg(not(feature = "cuda"))]
        {
            base_equal
        }
    }
}

impl Eq for ProveWitnessTraceCommitments {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProveWitnessTraceConstraintChecks {
    regular_constraint_count: usize,
    trace_extracted: bool,
    regular_constraints_evaluated: bool,
    witness_values_committed: bool,
    constraint_checker_conformant: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProveWitnessTraceConstraintEvidence {
    unit_index: usize,
    trace_instance_index: u32,
    trace_row_count: usize,
    trace_column_count: usize,
    checks: ProveWitnessTraceConstraintChecks,
}

impl ProveWitnessTraceConstraintEvidence {
    pub fn unit_index(&self) -> usize {
        self.unit_index
    }

    pub fn trace_instance_index(&self) -> u32 {
        self.trace_instance_index
    }

    pub fn trace_row_count(&self) -> usize {
        self.trace_row_count
    }

    pub fn trace_column_count(&self) -> usize {
        self.trace_column_count
    }

    pub fn regular_constraint_count(&self) -> usize {
        self.checks.regular_constraint_count
    }

    pub fn trace_extracted(&self) -> bool {
        self.checks.trace_extracted
    }

    pub fn regular_constraints_evaluated(&self) -> bool {
        self.checks.regular_constraints_evaluated
    }

    pub fn witness_values_committed(&self) -> bool {
        self.checks.witness_values_committed
    }

    pub fn constraint_checker_conformant(&self) -> bool {
        self.checks.constraint_checker_conformant
    }
}

impl ProveWitnessTraceCommitments {
    pub fn commitments(&self) -> &ProveWitnessCommitments {
        &self.commitments
    }

    pub fn trace(&self) -> &WitnessTraceBuffer {
        self.trace
            .as_ref()
            .expect("witness trace was not retained for this output")
    }

    pub fn trace_if_available(&self) -> Option<&WitnessTraceBuffer> {
        self.trace.as_ref()
    }

    pub fn retains_trace(&self) -> bool {
        self.trace.is_some()
    }

    pub fn trace_constraint_evidence(&self) -> ProveWitnessTraceConstraintEvidence {
        ProveWitnessTraceConstraintEvidence {
            unit_index: self.commitments.unit_index(),
            trace_instance_index: self.commitments.trace_instance_index(),
            trace_row_count: self.commitments.trace_row_count(),
            trace_column_count: self.commitments.trace_column_count(),
            checks: self.trace_constraint_checks,
        }
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn stage_source_devices_if_available(
        &self,
    ) -> Option<&[WitnessStageRetainedSourceDevice]> {
        (!self.stage_source_devices.is_empty()).then_some(self.stage_source_devices.as_slice())
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn stage_source_device_view(
        &self,
        stage_index: usize,
    ) -> Option<&WitnessStageSourceDeviceView> {
        self.stage_source_devices
            .iter()
            .find(|source| source.stage_index() == stage_index)
            .map(WitnessStageRetainedSourceDevice::source_view)
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn guest_pc_device_descriptor_buffer(&self) -> Option<&CudaDeviceBuffer> {
        self.guest_pc_device_descriptor_buffer
            .as_ref()
            .map(WitnessRetainedDeviceBuffer::buffer)
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn guest_pc_device_segment_material(
        &self,
    ) -> Option<&GuestPcTraceDeviceSegmentMaterial> {
        self.guest_pc_device_segment_material.as_ref()
    }

    pub fn publics(&self) -> &[Felt] {
        &self.publics
    }

    pub fn auxiliary_inputs(&self) -> &ProveWitnessAuxiliaryInputs {
        self.auxiliary_inputs.as_ref()
    }

    pub fn into_commitments(self) -> ProveWitnessCommitments {
        self.commitments
    }

    fn without_trace(mut self) -> Self {
        self.trace = None;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProveWitnessAuxiliaryInputs {
    pub unit_values: Vec<Felt>,
    pub proof_values: Vec<Felt>,
    pub group_values: Vec<Ext3>,
    pub challenges: Vec<Ext3>,
    pub evaluations: Vec<Ext3>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveWitnessGuestPcTraceTiming {
    segment_count: usize,
    guest_trace_stream_elapsed_duration: Duration,
    guest_trace_stream_duration: Duration,
    guest_segment_commit_duration: Duration,
    guest_trace_runner_duration: Duration,
    guest_trace_lowerer_duration: Duration,
    guest_trace_lower_duration: Duration,
    guest_trace_report_duration: Duration,
    guest_trace_single_row_report_duration: Duration,
    guest_trace_multi_row_report_duration: Duration,
    guest_trace_pending_dma_report_duration: Duration,
    guest_trace_amo_report_duration: Duration,
    guest_trace_store_conditional_report_duration: Duration,
    guest_trace_report_lowering_duration: Duration,
    guest_trace_report_row_validation_duration: Duration,
    guest_trace_report_source_values_duration: Duration,
    guest_trace_report_precompile_memory_duration: Duration,
    guest_trace_report_instruction_result_duration: Duration,
    guest_trace_report_next_pc_duration: Duration,
    guest_trace_report_register_access_duration: Duration,
    guest_trace_report_memory_access_duration: Duration,
    guest_trace_report_store_apply_duration: Duration,
    guest_trace_report_visit_duration: Duration,
    guest_trace_emit_duration: Duration,
    guest_trace_descriptor_duration: Duration,
    guest_trace_report_detail_sample_count: usize,
    guest_trace_seed_direct_lift_duration: Duration,
    guest_trace_seed_full_advance_duration: Duration,
    guest_trace_pending_send_wait_duration: Duration,
    guest_trace_pending_receive_wait_duration: Duration,
    guest_trace_segment_send_wait_duration: Duration,
    guest_trace_segment_receive_wait_duration: Duration,
    guest_trace_seed_direct_lift_attempt_count: usize,
    guest_trace_seed_direct_lift_success_count: usize,
    guest_trace_seed_direct_lift_empty_segment_count: usize,
    guest_trace_seed_direct_lift_pending_dma_single_report_count: usize,
    guest_trace_seed_direct_lift_amo_boundary_count: usize,
    guest_trace_seed_direct_lift_store_conditional_boundary_count: usize,
    guest_trace_seed_direct_lift_dma_prepare_missing_lookahead_count: usize,
    guest_trace_seed_direct_lift_boundary_c_unavailable_count: usize,
    guest_trace_seed_full_advance_count: usize,
    guest_trace_report_count: usize,
    guest_trace_report_row_count: usize,
    guest_trace_report_buffer_capacity: usize,
    guest_trace_report_buffer_max_capacity: usize,
    guest_trace_report_buffer_excess_capacity: usize,
    guest_trace_descriptor_row_count: usize,
    guest_trace_descriptor_compact_row_count: usize,
    guest_trace_descriptor_wide_row_count: usize,
    guest_trace_single_row_report_count: usize,
    guest_trace_multi_row_report_count: usize,
    guest_trace_pending_dma_report_count: usize,
    guest_trace_amo_report_count: usize,
    guest_trace_store_conditional_report_count: usize,
    guest_trace_external_op_row_count: usize,
    guest_trace_copy_row_count: usize,
    guest_trace_flag_row_count: usize,
    guest_trace_precompile_row_count: usize,
    guest_trace_indirect_memory_row_count: usize,
    guest_trace_register_source_read_count: usize,
    guest_trace_memory_source_read_count: usize,
    guest_trace_register_store_row_count: usize,
    guest_trace_memory_store_row_count: usize,
    guest_trace_no_store_row_count: usize,
    guest_device_source_build_duration: Duration,
    guest_device_source_descriptor_upload_duration: Duration,
    guest_device_source_descriptor_upload_byte_count: usize,
    guest_device_source_descriptor_upload_word_count: usize,
    guest_device_source_descriptor_upload_row_count: usize,
    guest_device_source_trace_expand_duration: Duration,
    guest_stage_source_retention_attempt_count: usize,
    guest_stage_source_retention_retained_count: usize,
    guest_stage_source_retention_rejected_count: usize,
    guest_stage_source_retention_retained_byte_count: usize,
    guest_stage_source_retention_rejected_byte_count: usize,
    guest_stage_source_retention_limit_byte_count: usize,
    guest_descriptor_buffer_retention_attempt_count: usize,
    guest_descriptor_buffer_retention_retained_count: usize,
    guest_descriptor_buffer_retention_rejected_count: usize,
    guest_descriptor_buffer_retention_retained_byte_count: usize,
    guest_descriptor_buffer_retention_rejected_byte_count: usize,
    guest_regular_constraint_duration: Duration,
    guest_regular_hint_duration: Duration,
    guest_stage_commit_duration: Duration,
    guest_stage_trace_extract_duration: Duration,
    guest_stage_leaf_extend_work_duration: Duration,
    guest_stage_leaf_setup_work_duration: Duration,
    guest_stage_leaf_setup_prepare_duration: Duration,
    guest_stage_leaf_setup_output_alloc_duration: Duration,
    guest_stage_leaf_setup_workspace_alloc_duration: Duration,
    guest_stage_leaf_setup_output_alloc_byte_count: usize,
    guest_stage_leaf_setup_workspace_alloc_byte_count: usize,
    guest_stage_leaf_setup_output_alloc_count: usize,
    guest_stage_leaf_output_cache_hit_count: usize,
    guest_stage_leaf_output_cache_miss_count: usize,
    guest_stage_leaf_setup_workspace_alloc_count: usize,
    guest_stage_leaf_upload_work_duration: Duration,
    guest_stage_leaf_kernel_work_duration: Duration,
    guest_stage_leaf_download_work_duration: Duration,
    guest_stage_leaf_validate_work_duration: Duration,
    guest_stage_leaf_hash_work_duration: Duration,
    guest_stage_leaf_hash_row_count: usize,
    guest_stage_leaf_hash_byte_count: usize,
    guest_stage_leaf_hash_arity2_row_count: usize,
    guest_stage_leaf_hash_arity2_byte_count: usize,
    guest_stage_leaf_hash_arity4_row_count: usize,
    guest_stage_leaf_hash_arity4_byte_count: usize,
    guest_stage_leaf_coset_extend_call_count: usize,
    guest_stage_leaf_coset_extend_output_byte_count: usize,
    guest_stage_leaf_coset_extend_column_count: usize,
    guest_stage_leaf_coset_extend_max_column_count: usize,
    guest_stage_leaf_coset_extend_ntt_launch_count: usize,
    guest_stage_leaf_coset_extend_bit_reverse_launch_count: usize,
    guest_stage_leaf_coset_extend_ntt_stage_launch_count: usize,
    guest_stage_leaf_coset_extend_ntt_block_twiddle_launch_count: usize,
    guest_stage_leaf_coset_extend_normalize_launch_count: usize,
    guest_stage_leaf_coset_extend_pack_launch_count: usize,
    guest_stage_leaf_coset_extend_unpack_launch_count: usize,
    guest_stage_tree_commit_work_duration: Duration,
    guest_stage_tree_commit_checkpoint_work_duration: Duration,
    guest_stage_tree_commit_root_work_duration: Duration,
    guest_stage_tree_commit_root_count: usize,
    guest_stage_tree_commit_root_byte_count: usize,
    guest_stage_tree_commit_retain_work_duration: Duration,
    guest_stage_timings: Vec<ProveWitnessGuestStageTiming>,
}

impl ProveWitnessGuestPcTraceTiming {
    fn new(
        segment_count: usize,
        guest_trace_stream_elapsed_duration: Duration,
        guest_trace_stream_duration: Duration,
        guest_segment_commit_duration: Duration,
        stream_timing: GuestPcTraceStreamTiming,
        trace_timing: ProveWitnessTraceTimingAccumulator,
    ) -> Self {
        Self {
            segment_count,
            guest_trace_stream_elapsed_duration,
            guest_trace_stream_duration,
            guest_segment_commit_duration,
            guest_trace_runner_duration: stream_timing.runner_duration(),
            guest_trace_lowerer_duration: stream_timing.lowerer_duration(),
            guest_trace_lower_duration: stream_timing.trace_lower_duration(),
            guest_trace_report_duration: stream_timing.trace_report_duration(),
            guest_trace_single_row_report_duration: stream_timing
                .trace_single_row_report_duration(),
            guest_trace_multi_row_report_duration: stream_timing.trace_multi_row_report_duration(),
            guest_trace_pending_dma_report_duration: stream_timing
                .trace_pending_dma_report_duration(),
            guest_trace_amo_report_duration: stream_timing.trace_amo_report_duration(),
            guest_trace_store_conditional_report_duration: stream_timing
                .trace_store_conditional_report_duration(),
            guest_trace_report_lowering_duration: stream_timing.trace_report_lowering_duration(),
            guest_trace_report_row_validation_duration: stream_timing
                .trace_report_row_validation_duration(),
            guest_trace_report_source_values_duration: stream_timing
                .trace_report_source_values_duration(),
            guest_trace_report_precompile_memory_duration: stream_timing
                .trace_report_precompile_memory_duration(),
            guest_trace_report_instruction_result_duration: stream_timing
                .trace_report_instruction_result_duration(),
            guest_trace_report_next_pc_duration: stream_timing.trace_report_next_pc_duration(),
            guest_trace_report_register_access_duration: stream_timing
                .trace_report_register_access_duration(),
            guest_trace_report_memory_access_duration: stream_timing
                .trace_report_memory_access_duration(),
            guest_trace_report_store_apply_duration: stream_timing
                .trace_report_store_apply_duration(),
            guest_trace_report_visit_duration: stream_timing.trace_report_visit_duration(),
            guest_trace_emit_duration: stream_timing.trace_emit_duration(),
            guest_trace_descriptor_duration: stream_timing.trace_descriptor_duration(),
            guest_trace_report_detail_sample_count: stream_timing
                .trace_report_detail_sample_count(),
            guest_trace_seed_direct_lift_duration: stream_timing.seed_direct_lift_duration(),
            guest_trace_seed_full_advance_duration: stream_timing.seed_full_advance_duration(),
            guest_trace_pending_send_wait_duration: stream_timing.pending_send_wait_duration(),
            guest_trace_pending_receive_wait_duration: stream_timing
                .pending_receive_wait_duration(),
            guest_trace_segment_send_wait_duration: stream_timing.segment_send_wait_duration(),
            guest_trace_segment_receive_wait_duration: stream_timing
                .segment_receive_wait_duration(),
            guest_trace_seed_direct_lift_attempt_count: stream_timing
                .seed_direct_lift_attempt_count(),
            guest_trace_seed_direct_lift_success_count: stream_timing
                .seed_direct_lift_success_count(),
            guest_trace_seed_direct_lift_empty_segment_count: stream_timing
                .seed_direct_lift_empty_segment_count(),
            guest_trace_seed_direct_lift_pending_dma_single_report_count: stream_timing
                .seed_direct_lift_pending_dma_single_report_count(),
            guest_trace_seed_direct_lift_amo_boundary_count: stream_timing
                .seed_direct_lift_amo_boundary_count(),
            guest_trace_seed_direct_lift_store_conditional_boundary_count: stream_timing
                .seed_direct_lift_store_conditional_boundary_count(),
            guest_trace_seed_direct_lift_dma_prepare_missing_lookahead_count: stream_timing
                .seed_direct_lift_dma_prepare_missing_lookahead_count(),
            guest_trace_seed_direct_lift_boundary_c_unavailable_count: stream_timing
                .seed_direct_lift_boundary_c_unavailable_count(),
            guest_trace_seed_full_advance_count: stream_timing.seed_full_advance_count(),
            guest_trace_report_count: stream_timing.trace_report_count(),
            guest_trace_report_row_count: stream_timing.trace_report_row_count(),
            guest_trace_report_buffer_capacity: stream_timing.trace_report_buffer_capacity(),
            guest_trace_report_buffer_max_capacity: stream_timing
                .trace_report_buffer_max_capacity(),
            guest_trace_report_buffer_excess_capacity: stream_timing
                .trace_report_buffer_excess_capacity(),
            guest_trace_descriptor_row_count: stream_timing.trace_descriptor_row_count(),
            guest_trace_descriptor_compact_row_count: stream_timing
                .trace_descriptor_compact_row_count(),
            guest_trace_descriptor_wide_row_count: stream_timing.trace_descriptor_wide_row_count(),
            guest_trace_single_row_report_count: stream_timing.trace_single_row_report_count(),
            guest_trace_multi_row_report_count: stream_timing.trace_multi_row_report_count(),
            guest_trace_pending_dma_report_count: stream_timing.trace_pending_dma_report_count(),
            guest_trace_amo_report_count: stream_timing.trace_amo_report_count(),
            guest_trace_store_conditional_report_count: stream_timing
                .trace_store_conditional_report_count(),
            guest_trace_external_op_row_count: stream_timing.trace_external_op_row_count(),
            guest_trace_copy_row_count: stream_timing.trace_copy_row_count(),
            guest_trace_flag_row_count: stream_timing.trace_flag_row_count(),
            guest_trace_precompile_row_count: stream_timing.trace_precompile_row_count(),
            guest_trace_indirect_memory_row_count: stream_timing.trace_indirect_memory_row_count(),
            guest_trace_register_source_read_count: stream_timing
                .trace_register_source_read_count(),
            guest_trace_memory_source_read_count: stream_timing.trace_memory_source_read_count(),
            guest_trace_register_store_row_count: stream_timing.trace_register_store_row_count(),
            guest_trace_memory_store_row_count: stream_timing.trace_memory_store_row_count(),
            guest_trace_no_store_row_count: stream_timing.trace_no_store_row_count(),
            guest_device_source_build_duration: trace_timing.device_source_build_duration,
            guest_device_source_descriptor_upload_duration: trace_timing
                .device_source_descriptor_upload_duration,
            guest_device_source_descriptor_upload_byte_count: trace_timing
                .device_source_descriptor_upload_byte_count,
            guest_device_source_descriptor_upload_word_count: trace_timing
                .device_source_descriptor_upload_word_count,
            guest_device_source_descriptor_upload_row_count: trace_timing
                .device_source_descriptor_upload_row_count,
            guest_device_source_trace_expand_duration: trace_timing
                .device_source_trace_expand_duration,
            guest_stage_source_retention_attempt_count: trace_timing
                .stage_source_retention_attempt_count,
            guest_stage_source_retention_retained_count: trace_timing
                .stage_source_retention_retained_count,
            guest_stage_source_retention_rejected_count: trace_timing
                .stage_source_retention_rejected_count,
            guest_stage_source_retention_retained_byte_count: trace_timing
                .stage_source_retention_retained_byte_count,
            guest_stage_source_retention_rejected_byte_count: trace_timing
                .stage_source_retention_rejected_byte_count,
            guest_stage_source_retention_limit_byte_count: trace_timing
                .stage_source_retention_limit_byte_count,
            guest_descriptor_buffer_retention_attempt_count: trace_timing
                .descriptor_buffer_retention_attempt_count,
            guest_descriptor_buffer_retention_retained_count: trace_timing
                .descriptor_buffer_retention_retained_count,
            guest_descriptor_buffer_retention_rejected_count: trace_timing
                .descriptor_buffer_retention_rejected_count,
            guest_descriptor_buffer_retention_retained_byte_count: trace_timing
                .descriptor_buffer_retention_retained_byte_count,
            guest_descriptor_buffer_retention_rejected_byte_count: trace_timing
                .descriptor_buffer_retention_rejected_byte_count,
            guest_regular_constraint_duration: trace_timing.regular_constraint_duration,
            guest_regular_hint_duration: trace_timing.regular_hint_duration,
            guest_stage_commit_duration: trace_timing.stage_commit_duration,
            guest_stage_trace_extract_duration: trace_timing.stage_trace_extract_duration,
            guest_stage_leaf_extend_work_duration: trace_timing.stage_leaf_extend_work_duration,
            guest_stage_leaf_setup_work_duration: trace_timing.stage_leaf_setup_work_duration,
            guest_stage_leaf_setup_prepare_duration: trace_timing.stage_leaf_setup_prepare_duration,
            guest_stage_leaf_setup_output_alloc_duration: trace_timing
                .stage_leaf_setup_output_alloc_duration,
            guest_stage_leaf_setup_workspace_alloc_duration: trace_timing
                .stage_leaf_setup_workspace_alloc_duration,
            guest_stage_leaf_setup_output_alloc_byte_count: trace_timing
                .stage_leaf_setup_output_alloc_byte_count,
            guest_stage_leaf_setup_workspace_alloc_byte_count: trace_timing
                .stage_leaf_setup_workspace_alloc_byte_count,
            guest_stage_leaf_setup_output_alloc_count: trace_timing
                .stage_leaf_setup_output_alloc_count,
            guest_stage_leaf_output_cache_hit_count: trace_timing.stage_leaf_output_cache_hit_count,
            guest_stage_leaf_output_cache_miss_count: trace_timing
                .stage_leaf_output_cache_miss_count,
            guest_stage_leaf_setup_workspace_alloc_count: trace_timing
                .stage_leaf_setup_workspace_alloc_count,
            guest_stage_leaf_upload_work_duration: trace_timing.stage_leaf_upload_work_duration,
            guest_stage_leaf_kernel_work_duration: trace_timing.stage_leaf_kernel_work_duration,
            guest_stage_leaf_download_work_duration: trace_timing.stage_leaf_download_work_duration,
            guest_stage_leaf_validate_work_duration: trace_timing.stage_leaf_validate_work_duration,
            guest_stage_leaf_hash_work_duration: trace_timing.stage_leaf_hash_work_duration,
            guest_stage_leaf_hash_row_count: trace_timing.stage_leaf_hash_row_count,
            guest_stage_leaf_hash_byte_count: trace_timing.stage_leaf_hash_byte_count,
            guest_stage_leaf_hash_arity2_row_count: trace_timing.stage_leaf_hash_arity2_row_count,
            guest_stage_leaf_hash_arity2_byte_count: trace_timing.stage_leaf_hash_arity2_byte_count,
            guest_stage_leaf_hash_arity4_row_count: trace_timing.stage_leaf_hash_arity4_row_count,
            guest_stage_leaf_hash_arity4_byte_count: trace_timing.stage_leaf_hash_arity4_byte_count,
            guest_stage_leaf_coset_extend_call_count: trace_timing
                .stage_leaf_coset_extend_call_count,
            guest_stage_leaf_coset_extend_output_byte_count: trace_timing
                .stage_leaf_coset_extend_output_byte_count,
            guest_stage_leaf_coset_extend_column_count: trace_timing
                .stage_leaf_coset_extend_column_count,
            guest_stage_leaf_coset_extend_max_column_count: trace_timing
                .stage_leaf_coset_extend_max_column_count,
            guest_stage_leaf_coset_extend_ntt_launch_count: trace_timing
                .stage_leaf_coset_extend_ntt_launch_count,
            guest_stage_leaf_coset_extend_bit_reverse_launch_count: trace_timing
                .stage_leaf_coset_extend_bit_reverse_launch_count,
            guest_stage_leaf_coset_extend_ntt_stage_launch_count: trace_timing
                .stage_leaf_coset_extend_ntt_stage_launch_count,
            guest_stage_leaf_coset_extend_ntt_block_twiddle_launch_count: trace_timing
                .stage_leaf_coset_extend_ntt_block_twiddle_launch_count,
            guest_stage_leaf_coset_extend_normalize_launch_count: trace_timing
                .stage_leaf_coset_extend_normalize_launch_count,
            guest_stage_leaf_coset_extend_pack_launch_count: trace_timing
                .stage_leaf_coset_extend_pack_launch_count,
            guest_stage_leaf_coset_extend_unpack_launch_count: trace_timing
                .stage_leaf_coset_extend_unpack_launch_count,
            guest_stage_tree_commit_work_duration: trace_timing.stage_tree_commit_work_duration,
            guest_stage_tree_commit_checkpoint_work_duration: trace_timing
                .stage_tree_commit_checkpoint_work_duration,
            guest_stage_tree_commit_root_work_duration: trace_timing
                .stage_tree_commit_root_work_duration,
            guest_stage_tree_commit_root_count: trace_timing.stage_tree_commit_root_count,
            guest_stage_tree_commit_root_byte_count: trace_timing.stage_tree_commit_root_byte_count,
            guest_stage_tree_commit_retain_work_duration: trace_timing
                .stage_tree_commit_retain_work_duration,
            guest_stage_timings: trace_timing.stage_timings,
        }
    }

    pub fn segment_count(&self) -> usize {
        self.segment_count
    }

    pub fn guest_trace_stream_elapsed_duration(&self) -> Duration {
        self.guest_trace_stream_elapsed_duration
    }

    pub fn guest_trace_stream_duration(&self) -> Duration {
        self.guest_trace_stream_duration
    }

    pub fn guest_segment_commit_duration(&self) -> Duration {
        self.guest_segment_commit_duration
    }

    pub fn guest_trace_runner_duration(&self) -> Duration {
        self.guest_trace_runner_duration
    }

    pub fn guest_trace_lowerer_duration(&self) -> Duration {
        self.guest_trace_lowerer_duration
    }

    pub fn guest_trace_lower_duration(&self) -> Duration {
        self.guest_trace_lower_duration
    }

    pub fn guest_trace_report_duration(&self) -> Duration {
        self.guest_trace_report_duration
    }

    pub fn guest_trace_report_validation_duration(&self) -> Duration {
        self.guest_trace_report_duration
            .saturating_sub(self.guest_trace_emit_duration)
            .saturating_sub(self.guest_trace_descriptor_duration)
    }

    pub fn guest_trace_single_row_report_duration(&self) -> Duration {
        self.guest_trace_single_row_report_duration
    }

    pub fn guest_trace_multi_row_report_duration(&self) -> Duration {
        self.guest_trace_multi_row_report_duration
    }

    pub fn guest_trace_pending_dma_report_duration(&self) -> Duration {
        self.guest_trace_pending_dma_report_duration
    }

    pub fn guest_trace_amo_report_duration(&self) -> Duration {
        self.guest_trace_amo_report_duration
    }

    pub fn guest_trace_store_conditional_report_duration(&self) -> Duration {
        self.guest_trace_store_conditional_report_duration
    }

    pub fn guest_trace_report_lowering_duration(&self) -> Duration {
        self.guest_trace_report_lowering_duration
    }

    pub fn guest_trace_report_row_validation_duration(&self) -> Duration {
        self.guest_trace_report_row_validation_duration
    }

    pub fn guest_trace_report_source_values_duration(&self) -> Duration {
        self.guest_trace_report_source_values_duration
    }

    pub fn guest_trace_report_precompile_memory_duration(&self) -> Duration {
        self.guest_trace_report_precompile_memory_duration
    }

    pub fn guest_trace_report_instruction_result_duration(&self) -> Duration {
        self.guest_trace_report_instruction_result_duration
    }

    pub fn guest_trace_report_next_pc_duration(&self) -> Duration {
        self.guest_trace_report_next_pc_duration
    }

    pub fn guest_trace_report_register_access_duration(&self) -> Duration {
        self.guest_trace_report_register_access_duration
    }

    pub fn guest_trace_report_memory_access_duration(&self) -> Duration {
        self.guest_trace_report_memory_access_duration
    }

    pub fn guest_trace_report_store_apply_duration(&self) -> Duration {
        self.guest_trace_report_store_apply_duration
    }

    pub fn guest_trace_report_visit_duration(&self) -> Duration {
        self.guest_trace_report_visit_duration
    }

    pub fn guest_trace_emit_duration(&self) -> Duration {
        self.guest_trace_emit_duration
    }

    pub fn guest_trace_descriptor_duration(&self) -> Duration {
        self.guest_trace_descriptor_duration
    }

    pub fn guest_trace_report_detail_sample_count(&self) -> usize {
        self.guest_trace_report_detail_sample_count
    }

    pub fn guest_trace_seed_direct_lift_duration(&self) -> Duration {
        self.guest_trace_seed_direct_lift_duration
    }

    pub fn guest_trace_seed_full_advance_duration(&self) -> Duration {
        self.guest_trace_seed_full_advance_duration
    }

    pub fn guest_trace_pending_send_wait_duration(&self) -> Duration {
        self.guest_trace_pending_send_wait_duration
    }

    pub fn guest_trace_pending_receive_wait_duration(&self) -> Duration {
        self.guest_trace_pending_receive_wait_duration
    }

    pub fn guest_trace_segment_send_wait_duration(&self) -> Duration {
        self.guest_trace_segment_send_wait_duration
    }

    pub fn guest_trace_segment_receive_wait_duration(&self) -> Duration {
        self.guest_trace_segment_receive_wait_duration
    }

    pub fn guest_trace_seed_direct_lift_attempt_count(&self) -> usize {
        self.guest_trace_seed_direct_lift_attempt_count
    }

    pub fn guest_trace_seed_direct_lift_success_count(&self) -> usize {
        self.guest_trace_seed_direct_lift_success_count
    }

    pub fn guest_trace_seed_direct_lift_empty_segment_count(&self) -> usize {
        self.guest_trace_seed_direct_lift_empty_segment_count
    }

    pub fn guest_trace_seed_direct_lift_pending_dma_single_report_count(&self) -> usize {
        self.guest_trace_seed_direct_lift_pending_dma_single_report_count
    }

    pub fn guest_trace_seed_direct_lift_amo_boundary_count(&self) -> usize {
        self.guest_trace_seed_direct_lift_amo_boundary_count
    }

    pub fn guest_trace_seed_direct_lift_store_conditional_boundary_count(&self) -> usize {
        self.guest_trace_seed_direct_lift_store_conditional_boundary_count
    }

    pub fn guest_trace_seed_direct_lift_dma_prepare_missing_lookahead_count(&self) -> usize {
        self.guest_trace_seed_direct_lift_dma_prepare_missing_lookahead_count
    }

    pub fn guest_trace_seed_direct_lift_boundary_c_unavailable_count(&self) -> usize {
        self.guest_trace_seed_direct_lift_boundary_c_unavailable_count
    }

    pub fn guest_trace_seed_full_advance_count(&self) -> usize {
        self.guest_trace_seed_full_advance_count
    }

    pub fn guest_trace_report_count(&self) -> usize {
        self.guest_trace_report_count
    }

    pub fn guest_trace_report_row_count(&self) -> usize {
        self.guest_trace_report_row_count
    }

    pub fn guest_trace_report_buffer_capacity(&self) -> usize {
        self.guest_trace_report_buffer_capacity
    }

    pub fn guest_trace_report_buffer_max_capacity(&self) -> usize {
        self.guest_trace_report_buffer_max_capacity
    }

    pub fn guest_trace_report_buffer_excess_capacity(&self) -> usize {
        self.guest_trace_report_buffer_excess_capacity
    }

    pub fn guest_trace_descriptor_row_count(&self) -> usize {
        self.guest_trace_descriptor_row_count
    }

    pub fn guest_trace_descriptor_compact_row_count(&self) -> usize {
        self.guest_trace_descriptor_compact_row_count
    }

    pub fn guest_trace_descriptor_wide_row_count(&self) -> usize {
        self.guest_trace_descriptor_wide_row_count
    }

    pub fn guest_trace_single_row_report_count(&self) -> usize {
        self.guest_trace_single_row_report_count
    }

    pub fn guest_trace_multi_row_report_count(&self) -> usize {
        self.guest_trace_multi_row_report_count
    }

    pub fn guest_trace_pending_dma_report_count(&self) -> usize {
        self.guest_trace_pending_dma_report_count
    }

    pub fn guest_trace_amo_report_count(&self) -> usize {
        self.guest_trace_amo_report_count
    }

    pub fn guest_trace_store_conditional_report_count(&self) -> usize {
        self.guest_trace_store_conditional_report_count
    }

    pub fn guest_trace_external_op_row_count(&self) -> usize {
        self.guest_trace_external_op_row_count
    }

    pub fn guest_trace_copy_row_count(&self) -> usize {
        self.guest_trace_copy_row_count
    }

    pub fn guest_trace_flag_row_count(&self) -> usize {
        self.guest_trace_flag_row_count
    }

    pub fn guest_trace_precompile_row_count(&self) -> usize {
        self.guest_trace_precompile_row_count
    }

    pub fn guest_trace_indirect_memory_row_count(&self) -> usize {
        self.guest_trace_indirect_memory_row_count
    }

    pub fn guest_trace_register_source_read_count(&self) -> usize {
        self.guest_trace_register_source_read_count
    }

    pub fn guest_trace_memory_source_read_count(&self) -> usize {
        self.guest_trace_memory_source_read_count
    }

    pub fn guest_trace_register_store_row_count(&self) -> usize {
        self.guest_trace_register_store_row_count
    }

    pub fn guest_trace_memory_store_row_count(&self) -> usize {
        self.guest_trace_memory_store_row_count
    }

    pub fn guest_trace_no_store_row_count(&self) -> usize {
        self.guest_trace_no_store_row_count
    }

    pub fn guest_device_source_build_duration(&self) -> Duration {
        self.guest_device_source_build_duration
    }

    pub fn guest_device_source_descriptor_upload_duration(&self) -> Duration {
        self.guest_device_source_descriptor_upload_duration
    }

    pub fn guest_device_source_descriptor_upload_byte_count(&self) -> usize {
        self.guest_device_source_descriptor_upload_byte_count
    }

    pub fn guest_device_source_descriptor_upload_word_count(&self) -> usize {
        self.guest_device_source_descriptor_upload_word_count
    }

    pub fn guest_device_source_descriptor_upload_row_count(&self) -> usize {
        self.guest_device_source_descriptor_upload_row_count
    }

    pub fn guest_device_source_trace_expand_duration(&self) -> Duration {
        self.guest_device_source_trace_expand_duration
    }

    pub fn guest_stage_source_retention_attempt_count(&self) -> usize {
        self.guest_stage_source_retention_attempt_count
    }

    pub fn guest_stage_source_retention_retained_count(&self) -> usize {
        self.guest_stage_source_retention_retained_count
    }

    pub fn guest_stage_source_retention_rejected_count(&self) -> usize {
        self.guest_stage_source_retention_rejected_count
    }

    pub fn guest_stage_source_retention_retained_byte_count(&self) -> usize {
        self.guest_stage_source_retention_retained_byte_count
    }

    pub fn guest_stage_source_retention_rejected_byte_count(&self) -> usize {
        self.guest_stage_source_retention_rejected_byte_count
    }

    pub fn guest_stage_source_retention_limit_byte_count(&self) -> usize {
        self.guest_stage_source_retention_limit_byte_count
    }

    pub fn guest_descriptor_buffer_retention_attempt_count(&self) -> usize {
        self.guest_descriptor_buffer_retention_attempt_count
    }

    pub fn guest_descriptor_buffer_retention_retained_count(&self) -> usize {
        self.guest_descriptor_buffer_retention_retained_count
    }

    pub fn guest_descriptor_buffer_retention_rejected_count(&self) -> usize {
        self.guest_descriptor_buffer_retention_rejected_count
    }

    pub fn guest_descriptor_buffer_retention_retained_byte_count(&self) -> usize {
        self.guest_descriptor_buffer_retention_retained_byte_count
    }

    pub fn guest_descriptor_buffer_retention_rejected_byte_count(&self) -> usize {
        self.guest_descriptor_buffer_retention_rejected_byte_count
    }

    pub fn guest_regular_constraint_duration(&self) -> Duration {
        self.guest_regular_constraint_duration
    }

    pub fn guest_regular_hint_duration(&self) -> Duration {
        self.guest_regular_hint_duration
    }

    pub fn guest_stage_commit_duration(&self) -> Duration {
        self.guest_stage_commit_duration
    }

    pub fn guest_stage_trace_extract_duration(&self) -> Duration {
        self.guest_stage_trace_extract_duration
    }

    pub fn guest_stage_leaf_extend_work_duration(&self) -> Duration {
        self.guest_stage_leaf_extend_work_duration
    }

    pub fn guest_stage_leaf_setup_work_duration(&self) -> Duration {
        self.guest_stage_leaf_setup_work_duration
    }

    pub fn guest_stage_leaf_setup_prepare_duration(&self) -> Duration {
        self.guest_stage_leaf_setup_prepare_duration
    }

    pub fn guest_stage_leaf_setup_output_alloc_duration(&self) -> Duration {
        self.guest_stage_leaf_setup_output_alloc_duration
    }

    pub fn guest_stage_leaf_setup_workspace_alloc_duration(&self) -> Duration {
        self.guest_stage_leaf_setup_workspace_alloc_duration
    }

    pub fn guest_stage_leaf_setup_output_alloc_byte_count(&self) -> usize {
        self.guest_stage_leaf_setup_output_alloc_byte_count
    }

    pub fn guest_stage_leaf_setup_workspace_alloc_byte_count(&self) -> usize {
        self.guest_stage_leaf_setup_workspace_alloc_byte_count
    }

    pub fn guest_stage_leaf_setup_output_alloc_count(&self) -> usize {
        self.guest_stage_leaf_setup_output_alloc_count
    }

    pub fn guest_stage_leaf_output_cache_hit_count(&self) -> usize {
        self.guest_stage_leaf_output_cache_hit_count
    }

    pub fn guest_stage_leaf_output_cache_miss_count(&self) -> usize {
        self.guest_stage_leaf_output_cache_miss_count
    }

    pub fn guest_stage_leaf_setup_workspace_alloc_count(&self) -> usize {
        self.guest_stage_leaf_setup_workspace_alloc_count
    }

    pub fn guest_stage_leaf_upload_work_duration(&self) -> Duration {
        self.guest_stage_leaf_upload_work_duration
    }

    pub fn guest_stage_leaf_kernel_work_duration(&self) -> Duration {
        self.guest_stage_leaf_kernel_work_duration
    }

    pub fn guest_stage_leaf_download_work_duration(&self) -> Duration {
        self.guest_stage_leaf_download_work_duration
    }

    pub fn guest_stage_leaf_validate_work_duration(&self) -> Duration {
        self.guest_stage_leaf_validate_work_duration
    }

    pub fn guest_stage_leaf_hash_work_duration(&self) -> Duration {
        self.guest_stage_leaf_hash_work_duration
    }

    pub fn guest_stage_leaf_hash_row_count(&self) -> usize {
        self.guest_stage_leaf_hash_row_count
    }

    pub fn guest_stage_leaf_hash_byte_count(&self) -> usize {
        self.guest_stage_leaf_hash_byte_count
    }

    pub fn guest_stage_leaf_hash_arity2_row_count(&self) -> usize {
        self.guest_stage_leaf_hash_arity2_row_count
    }

    pub fn guest_stage_leaf_hash_arity2_byte_count(&self) -> usize {
        self.guest_stage_leaf_hash_arity2_byte_count
    }

    pub fn guest_stage_leaf_hash_arity4_row_count(&self) -> usize {
        self.guest_stage_leaf_hash_arity4_row_count
    }

    pub fn guest_stage_leaf_hash_arity4_byte_count(&self) -> usize {
        self.guest_stage_leaf_hash_arity4_byte_count
    }

    pub fn guest_stage_leaf_coset_extend_call_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_call_count
    }

    pub fn guest_stage_leaf_coset_extend_output_byte_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_output_byte_count
    }

    pub fn guest_stage_leaf_coset_extend_column_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_column_count
    }

    pub fn guest_stage_leaf_coset_extend_max_column_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_max_column_count
    }

    pub fn guest_stage_leaf_coset_extend_ntt_launch_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_ntt_launch_count
    }

    pub fn guest_stage_leaf_coset_extend_bit_reverse_launch_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_bit_reverse_launch_count
    }

    pub fn guest_stage_leaf_coset_extend_ntt_stage_launch_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_ntt_stage_launch_count
    }

    pub fn guest_stage_leaf_coset_extend_ntt_block_twiddle_launch_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_ntt_block_twiddle_launch_count
    }

    pub fn guest_stage_leaf_coset_extend_normalize_launch_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_normalize_launch_count
    }

    pub fn guest_stage_leaf_coset_extend_pack_launch_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_pack_launch_count
    }

    pub fn guest_stage_leaf_coset_extend_unpack_launch_count(&self) -> usize {
        self.guest_stage_leaf_coset_extend_unpack_launch_count
    }

    pub fn guest_stage_tree_commit_work_duration(&self) -> Duration {
        self.guest_stage_tree_commit_work_duration
    }

    pub fn guest_stage_tree_commit_checkpoint_work_duration(&self) -> Duration {
        self.guest_stage_tree_commit_checkpoint_work_duration
    }

    pub fn guest_stage_tree_commit_root_work_duration(&self) -> Duration {
        self.guest_stage_tree_commit_root_work_duration
    }

    pub fn guest_stage_tree_commit_root_count(&self) -> usize {
        self.guest_stage_tree_commit_root_count
    }

    pub fn guest_stage_tree_commit_root_byte_count(&self) -> usize {
        self.guest_stage_tree_commit_root_byte_count
    }

    pub fn guest_stage_tree_commit_retain_work_duration(&self) -> Duration {
        self.guest_stage_tree_commit_retain_work_duration
    }

    pub fn guest_stage_timings(&self) -> &[ProveWitnessGuestStageTiming] {
        &self.guest_stage_timings
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProveWitnessGuestStageTiming {
    stage_index: usize,
    leaf_extend_work_duration: Duration,
    leaf_setup_work_duration: Duration,
    leaf_setup_prepare_duration: Duration,
    leaf_setup_output_alloc_duration: Duration,
    leaf_setup_workspace_alloc_duration: Duration,
    leaf_setup_output_alloc_byte_count: usize,
    leaf_setup_workspace_alloc_byte_count: usize,
    leaf_setup_output_alloc_count: usize,
    leaf_output_cache_hit_count: usize,
    leaf_output_cache_miss_count: usize,
    leaf_setup_workspace_alloc_count: usize,
    leaf_upload_work_duration: Duration,
    leaf_kernel_work_duration: Duration,
    leaf_download_work_duration: Duration,
    leaf_validate_work_duration: Duration,
    leaf_hash_work_duration: Duration,
    leaf_hash_row_count: usize,
    leaf_hash_byte_count: usize,
    leaf_hash_arity2_row_count: usize,
    leaf_hash_arity2_byte_count: usize,
    leaf_hash_arity4_row_count: usize,
    leaf_hash_arity4_byte_count: usize,
    leaf_coset_extend_call_count: usize,
    leaf_coset_extend_output_byte_count: usize,
    leaf_coset_extend_column_count: usize,
    leaf_coset_extend_max_column_count: usize,
    leaf_coset_extend_ntt_launch_count: usize,
    leaf_coset_extend_bit_reverse_launch_count: usize,
    leaf_coset_extend_ntt_stage_launch_count: usize,
    leaf_coset_extend_ntt_block_twiddle_launch_count: usize,
    leaf_coset_extend_normalize_launch_count: usize,
    leaf_coset_extend_pack_launch_count: usize,
    leaf_coset_extend_unpack_launch_count: usize,
    tree_commit_work_duration: Duration,
    tree_commit_checkpoint_duration: Duration,
    tree_commit_root_duration: Duration,
    tree_commit_root_count: usize,
    tree_commit_root_byte_count: usize,
    tree_commit_retain_duration: Duration,
}

impl ProveWitnessGuestStageTiming {
    fn from_witness_stage_timing(timing: WitnessIndexedStageCommitTiming) -> Self {
        let timing_value = timing.timing();
        Self {
            stage_index: timing.stage_index(),
            leaf_extend_work_duration: timing_value.leaf_extend_duration(),
            leaf_setup_work_duration: timing_value.leaf_setup_duration(),
            leaf_setup_prepare_duration: timing_value.leaf_setup_prepare_duration(),
            leaf_setup_output_alloc_duration: timing_value.leaf_setup_output_alloc_duration(),
            leaf_setup_workspace_alloc_duration: timing_value.leaf_setup_workspace_alloc_duration(),
            leaf_setup_output_alloc_byte_count: timing_value.leaf_setup_output_alloc_byte_count(),
            leaf_setup_workspace_alloc_byte_count: timing_value
                .leaf_setup_workspace_alloc_byte_count(),
            leaf_setup_output_alloc_count: timing_value.leaf_setup_output_alloc_count(),
            leaf_output_cache_hit_count: timing_value.leaf_output_cache_hit_count(),
            leaf_output_cache_miss_count: timing_value.leaf_output_cache_miss_count(),
            leaf_setup_workspace_alloc_count: timing_value.leaf_setup_workspace_alloc_count(),
            leaf_upload_work_duration: timing_value.leaf_upload_duration(),
            leaf_kernel_work_duration: timing_value.leaf_kernel_duration(),
            leaf_download_work_duration: timing_value.leaf_download_duration(),
            leaf_validate_work_duration: timing_value.leaf_validate_duration(),
            leaf_hash_work_duration: timing_value.leaf_hash_duration(),
            leaf_hash_row_count: timing_value.leaf_hash_row_count(),
            leaf_hash_byte_count: timing_value.leaf_hash_byte_count(),
            leaf_hash_arity2_row_count: timing_value.leaf_hash_arity2_row_count(),
            leaf_hash_arity2_byte_count: timing_value.leaf_hash_arity2_byte_count(),
            leaf_hash_arity4_row_count: timing_value.leaf_hash_arity4_row_count(),
            leaf_hash_arity4_byte_count: timing_value.leaf_hash_arity4_byte_count(),
            leaf_coset_extend_call_count: timing_value.leaf_coset_extend_call_count(),
            leaf_coset_extend_output_byte_count: timing_value.leaf_coset_extend_output_byte_count(),
            leaf_coset_extend_column_count: timing_value.leaf_coset_extend_column_count(),
            leaf_coset_extend_max_column_count: timing_value.leaf_coset_extend_max_column_count(),
            leaf_coset_extend_ntt_launch_count: timing_value.leaf_coset_extend_ntt_launch_count(),
            leaf_coset_extend_bit_reverse_launch_count: timing_value
                .leaf_coset_extend_bit_reverse_launch_count(),
            leaf_coset_extend_ntt_stage_launch_count: timing_value
                .leaf_coset_extend_ntt_stage_launch_count(),
            leaf_coset_extend_ntt_block_twiddle_launch_count: timing_value
                .leaf_coset_extend_ntt_block_twiddle_launch_count(),
            leaf_coset_extend_normalize_launch_count: timing_value
                .leaf_coset_extend_normalize_launch_count(),
            leaf_coset_extend_pack_launch_count: timing_value.leaf_coset_extend_pack_launch_count(),
            leaf_coset_extend_unpack_launch_count: timing_value
                .leaf_coset_extend_unpack_launch_count(),
            tree_commit_work_duration: timing_value.tree_commit_duration(),
            tree_commit_checkpoint_duration: timing_value.tree_commit_checkpoint_duration(),
            tree_commit_root_duration: timing_value.tree_commit_root_duration(),
            tree_commit_root_count: timing_value.tree_commit_root_count(),
            tree_commit_root_byte_count: timing_value.tree_commit_root_byte_count(),
            tree_commit_retain_duration: timing_value.tree_commit_retain_duration(),
        }
    }

    fn accumulate(&mut self, other: Self) {
        self.leaf_extend_work_duration += other.leaf_extend_work_duration;
        self.leaf_setup_work_duration += other.leaf_setup_work_duration;
        self.leaf_setup_prepare_duration += other.leaf_setup_prepare_duration;
        self.leaf_setup_output_alloc_duration += other.leaf_setup_output_alloc_duration;
        self.leaf_setup_workspace_alloc_duration += other.leaf_setup_workspace_alloc_duration;
        self.leaf_setup_output_alloc_byte_count += other.leaf_setup_output_alloc_byte_count;
        self.leaf_setup_workspace_alloc_byte_count += other.leaf_setup_workspace_alloc_byte_count;
        self.leaf_setup_output_alloc_count += other.leaf_setup_output_alloc_count;
        self.leaf_output_cache_hit_count += other.leaf_output_cache_hit_count;
        self.leaf_output_cache_miss_count += other.leaf_output_cache_miss_count;
        self.leaf_setup_workspace_alloc_count += other.leaf_setup_workspace_alloc_count;
        self.leaf_upload_work_duration += other.leaf_upload_work_duration;
        self.leaf_kernel_work_duration += other.leaf_kernel_work_duration;
        self.leaf_download_work_duration += other.leaf_download_work_duration;
        self.leaf_validate_work_duration += other.leaf_validate_work_duration;
        self.leaf_hash_work_duration += other.leaf_hash_work_duration;
        self.leaf_hash_row_count += other.leaf_hash_row_count;
        self.leaf_hash_byte_count += other.leaf_hash_byte_count;
        self.leaf_hash_arity2_row_count += other.leaf_hash_arity2_row_count;
        self.leaf_hash_arity2_byte_count += other.leaf_hash_arity2_byte_count;
        self.leaf_hash_arity4_row_count += other.leaf_hash_arity4_row_count;
        self.leaf_hash_arity4_byte_count += other.leaf_hash_arity4_byte_count;
        self.leaf_coset_extend_call_count += other.leaf_coset_extend_call_count;
        self.leaf_coset_extend_output_byte_count += other.leaf_coset_extend_output_byte_count;
        self.leaf_coset_extend_column_count += other.leaf_coset_extend_column_count;
        self.leaf_coset_extend_max_column_count = self
            .leaf_coset_extend_max_column_count
            .max(other.leaf_coset_extend_max_column_count);
        self.leaf_coset_extend_ntt_launch_count += other.leaf_coset_extend_ntt_launch_count;
        self.leaf_coset_extend_bit_reverse_launch_count +=
            other.leaf_coset_extend_bit_reverse_launch_count;
        self.leaf_coset_extend_ntt_stage_launch_count +=
            other.leaf_coset_extend_ntt_stage_launch_count;
        self.leaf_coset_extend_ntt_block_twiddle_launch_count +=
            other.leaf_coset_extend_ntt_block_twiddle_launch_count;
        self.leaf_coset_extend_normalize_launch_count +=
            other.leaf_coset_extend_normalize_launch_count;
        self.leaf_coset_extend_pack_launch_count += other.leaf_coset_extend_pack_launch_count;
        self.leaf_coset_extend_unpack_launch_count += other.leaf_coset_extend_unpack_launch_count;
        self.tree_commit_work_duration += other.tree_commit_work_duration;
        self.tree_commit_checkpoint_duration += other.tree_commit_checkpoint_duration;
        self.tree_commit_root_duration += other.tree_commit_root_duration;
        self.tree_commit_root_count += other.tree_commit_root_count;
        self.tree_commit_root_byte_count += other.tree_commit_root_byte_count;
        self.tree_commit_retain_duration += other.tree_commit_retain_duration;
    }

    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn leaf_extend_work_duration(&self) -> Duration {
        self.leaf_extend_work_duration
    }

    pub fn leaf_setup_work_duration(&self) -> Duration {
        self.leaf_setup_work_duration
    }

    pub fn leaf_setup_prepare_duration(&self) -> Duration {
        self.leaf_setup_prepare_duration
    }

    pub fn leaf_setup_output_alloc_duration(&self) -> Duration {
        self.leaf_setup_output_alloc_duration
    }

    pub fn leaf_setup_workspace_alloc_duration(&self) -> Duration {
        self.leaf_setup_workspace_alloc_duration
    }

    pub fn leaf_setup_output_alloc_byte_count(&self) -> usize {
        self.leaf_setup_output_alloc_byte_count
    }

    pub fn leaf_setup_workspace_alloc_byte_count(&self) -> usize {
        self.leaf_setup_workspace_alloc_byte_count
    }

    pub fn leaf_setup_output_alloc_count(&self) -> usize {
        self.leaf_setup_output_alloc_count
    }

    pub fn leaf_output_cache_hit_count(&self) -> usize {
        self.leaf_output_cache_hit_count
    }

    pub fn leaf_output_cache_miss_count(&self) -> usize {
        self.leaf_output_cache_miss_count
    }

    pub fn leaf_setup_workspace_alloc_count(&self) -> usize {
        self.leaf_setup_workspace_alloc_count
    }

    pub fn leaf_upload_work_duration(&self) -> Duration {
        self.leaf_upload_work_duration
    }

    pub fn leaf_kernel_work_duration(&self) -> Duration {
        self.leaf_kernel_work_duration
    }

    pub fn leaf_download_work_duration(&self) -> Duration {
        self.leaf_download_work_duration
    }

    pub fn leaf_validate_work_duration(&self) -> Duration {
        self.leaf_validate_work_duration
    }

    pub fn leaf_hash_work_duration(&self) -> Duration {
        self.leaf_hash_work_duration
    }

    pub fn leaf_hash_row_count(&self) -> usize {
        self.leaf_hash_row_count
    }

    pub fn leaf_hash_byte_count(&self) -> usize {
        self.leaf_hash_byte_count
    }

    pub fn leaf_hash_arity2_row_count(&self) -> usize {
        self.leaf_hash_arity2_row_count
    }

    pub fn leaf_hash_arity2_byte_count(&self) -> usize {
        self.leaf_hash_arity2_byte_count
    }

    pub fn leaf_hash_arity4_row_count(&self) -> usize {
        self.leaf_hash_arity4_row_count
    }

    pub fn leaf_hash_arity4_byte_count(&self) -> usize {
        self.leaf_hash_arity4_byte_count
    }

    pub fn leaf_coset_extend_call_count(&self) -> usize {
        self.leaf_coset_extend_call_count
    }

    pub fn leaf_coset_extend_output_byte_count(&self) -> usize {
        self.leaf_coset_extend_output_byte_count
    }

    pub fn leaf_coset_extend_column_count(&self) -> usize {
        self.leaf_coset_extend_column_count
    }

    pub fn leaf_coset_extend_max_column_count(&self) -> usize {
        self.leaf_coset_extend_max_column_count
    }

    pub fn leaf_coset_extend_ntt_launch_count(&self) -> usize {
        self.leaf_coset_extend_ntt_launch_count
    }

    pub fn leaf_coset_extend_bit_reverse_launch_count(&self) -> usize {
        self.leaf_coset_extend_bit_reverse_launch_count
    }

    pub fn leaf_coset_extend_ntt_stage_launch_count(&self) -> usize {
        self.leaf_coset_extend_ntt_stage_launch_count
    }

    pub fn leaf_coset_extend_ntt_block_twiddle_launch_count(&self) -> usize {
        self.leaf_coset_extend_ntt_block_twiddle_launch_count
    }

    pub fn leaf_coset_extend_normalize_launch_count(&self) -> usize {
        self.leaf_coset_extend_normalize_launch_count
    }

    pub fn leaf_coset_extend_pack_launch_count(&self) -> usize {
        self.leaf_coset_extend_pack_launch_count
    }

    pub fn leaf_coset_extend_unpack_launch_count(&self) -> usize {
        self.leaf_coset_extend_unpack_launch_count
    }

    pub fn tree_commit_work_duration(&self) -> Duration {
        self.tree_commit_work_duration
    }

    pub fn tree_commit_checkpoint_work_duration(&self) -> Duration {
        self.tree_commit_checkpoint_duration
    }

    pub fn tree_commit_root_work_duration(&self) -> Duration {
        self.tree_commit_root_duration
    }

    pub fn tree_commit_root_count(&self) -> usize {
        self.tree_commit_root_count
    }

    pub fn tree_commit_root_byte_count(&self) -> usize {
        self.tree_commit_root_byte_count
    }

    pub fn tree_commit_retain_work_duration(&self) -> Duration {
        self.tree_commit_retain_duration
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ProveWitnessTraceTimingAccumulator {
    device_source_build_duration: Duration,
    device_source_descriptor_upload_duration: Duration,
    device_source_descriptor_upload_byte_count: usize,
    device_source_descriptor_upload_word_count: usize,
    device_source_descriptor_upload_row_count: usize,
    device_source_trace_expand_duration: Duration,
    stage_source_retention_attempt_count: usize,
    stage_source_retention_retained_count: usize,
    stage_source_retention_rejected_count: usize,
    stage_source_retention_retained_byte_count: usize,
    stage_source_retention_rejected_byte_count: usize,
    stage_source_retention_limit_byte_count: usize,
    descriptor_buffer_retention_attempt_count: usize,
    descriptor_buffer_retention_retained_count: usize,
    descriptor_buffer_retention_rejected_count: usize,
    descriptor_buffer_retention_retained_byte_count: usize,
    descriptor_buffer_retention_rejected_byte_count: usize,
    regular_constraint_duration: Duration,
    regular_hint_duration: Duration,
    stage_commit_duration: Duration,
    stage_trace_extract_duration: Duration,
    stage_leaf_extend_work_duration: Duration,
    stage_leaf_setup_work_duration: Duration,
    stage_leaf_setup_prepare_duration: Duration,
    stage_leaf_setup_output_alloc_duration: Duration,
    stage_leaf_setup_workspace_alloc_duration: Duration,
    stage_leaf_setup_output_alloc_byte_count: usize,
    stage_leaf_setup_workspace_alloc_byte_count: usize,
    stage_leaf_setup_output_alloc_count: usize,
    stage_leaf_output_cache_hit_count: usize,
    stage_leaf_output_cache_miss_count: usize,
    stage_leaf_setup_workspace_alloc_count: usize,
    stage_leaf_upload_work_duration: Duration,
    stage_leaf_kernel_work_duration: Duration,
    stage_leaf_download_work_duration: Duration,
    stage_leaf_validate_work_duration: Duration,
    stage_leaf_hash_work_duration: Duration,
    stage_leaf_hash_row_count: usize,
    stage_leaf_hash_byte_count: usize,
    stage_leaf_hash_arity2_row_count: usize,
    stage_leaf_hash_arity2_byte_count: usize,
    stage_leaf_hash_arity4_row_count: usize,
    stage_leaf_hash_arity4_byte_count: usize,
    stage_leaf_coset_extend_call_count: usize,
    stage_leaf_coset_extend_output_byte_count: usize,
    stage_leaf_coset_extend_column_count: usize,
    stage_leaf_coset_extend_max_column_count: usize,
    stage_leaf_coset_extend_ntt_launch_count: usize,
    stage_leaf_coset_extend_bit_reverse_launch_count: usize,
    stage_leaf_coset_extend_ntt_stage_launch_count: usize,
    stage_leaf_coset_extend_ntt_block_twiddle_launch_count: usize,
    stage_leaf_coset_extend_normalize_launch_count: usize,
    stage_leaf_coset_extend_pack_launch_count: usize,
    stage_leaf_coset_extend_unpack_launch_count: usize,
    stage_tree_commit_work_duration: Duration,
    stage_tree_commit_checkpoint_work_duration: Duration,
    stage_tree_commit_root_work_duration: Duration,
    stage_tree_commit_root_count: usize,
    stage_tree_commit_root_byte_count: usize,
    stage_tree_commit_retain_work_duration: Duration,
    stage_timings: Vec<ProveWitnessGuestStageTiming>,
}

impl ProveWitnessTraceTimingAccumulator {
    fn accumulate(&mut self, other: Self) {
        self.device_source_build_duration += other.device_source_build_duration;
        self.device_source_descriptor_upload_duration +=
            other.device_source_descriptor_upload_duration;
        self.device_source_descriptor_upload_byte_count +=
            other.device_source_descriptor_upload_byte_count;
        self.device_source_descriptor_upload_word_count +=
            other.device_source_descriptor_upload_word_count;
        self.device_source_descriptor_upload_row_count +=
            other.device_source_descriptor_upload_row_count;
        self.device_source_trace_expand_duration += other.device_source_trace_expand_duration;
        self.stage_source_retention_attempt_count += other.stage_source_retention_attempt_count;
        self.stage_source_retention_retained_count += other.stage_source_retention_retained_count;
        self.stage_source_retention_rejected_count += other.stage_source_retention_rejected_count;
        self.stage_source_retention_retained_byte_count +=
            other.stage_source_retention_retained_byte_count;
        self.stage_source_retention_rejected_byte_count +=
            other.stage_source_retention_rejected_byte_count;
        self.stage_source_retention_limit_byte_count = self
            .stage_source_retention_limit_byte_count
            .max(other.stage_source_retention_limit_byte_count);
        self.descriptor_buffer_retention_attempt_count +=
            other.descriptor_buffer_retention_attempt_count;
        self.descriptor_buffer_retention_retained_count +=
            other.descriptor_buffer_retention_retained_count;
        self.descriptor_buffer_retention_rejected_count +=
            other.descriptor_buffer_retention_rejected_count;
        self.descriptor_buffer_retention_retained_byte_count +=
            other.descriptor_buffer_retention_retained_byte_count;
        self.descriptor_buffer_retention_rejected_byte_count +=
            other.descriptor_buffer_retention_rejected_byte_count;
        self.regular_constraint_duration += other.regular_constraint_duration;
        self.regular_hint_duration += other.regular_hint_duration;
        self.stage_commit_duration += other.stage_commit_duration;
        self.stage_trace_extract_duration += other.stage_trace_extract_duration;
        self.stage_leaf_extend_work_duration += other.stage_leaf_extend_work_duration;
        self.stage_leaf_setup_work_duration += other.stage_leaf_setup_work_duration;
        self.stage_leaf_setup_prepare_duration += other.stage_leaf_setup_prepare_duration;
        self.stage_leaf_setup_output_alloc_duration += other.stage_leaf_setup_output_alloc_duration;
        self.stage_leaf_setup_workspace_alloc_duration +=
            other.stage_leaf_setup_workspace_alloc_duration;
        self.stage_leaf_setup_output_alloc_byte_count +=
            other.stage_leaf_setup_output_alloc_byte_count;
        self.stage_leaf_setup_workspace_alloc_byte_count +=
            other.stage_leaf_setup_workspace_alloc_byte_count;
        self.stage_leaf_setup_output_alloc_count += other.stage_leaf_setup_output_alloc_count;
        self.stage_leaf_output_cache_hit_count += other.stage_leaf_output_cache_hit_count;
        self.stage_leaf_output_cache_miss_count += other.stage_leaf_output_cache_miss_count;
        self.stage_leaf_setup_workspace_alloc_count += other.stage_leaf_setup_workspace_alloc_count;
        self.stage_leaf_upload_work_duration += other.stage_leaf_upload_work_duration;
        self.stage_leaf_kernel_work_duration += other.stage_leaf_kernel_work_duration;
        self.stage_leaf_download_work_duration += other.stage_leaf_download_work_duration;
        self.stage_leaf_validate_work_duration += other.stage_leaf_validate_work_duration;
        self.stage_leaf_hash_work_duration += other.stage_leaf_hash_work_duration;
        self.stage_leaf_hash_row_count += other.stage_leaf_hash_row_count;
        self.stage_leaf_hash_byte_count += other.stage_leaf_hash_byte_count;
        self.stage_leaf_hash_arity2_row_count += other.stage_leaf_hash_arity2_row_count;
        self.stage_leaf_hash_arity2_byte_count += other.stage_leaf_hash_arity2_byte_count;
        self.stage_leaf_hash_arity4_row_count += other.stage_leaf_hash_arity4_row_count;
        self.stage_leaf_hash_arity4_byte_count += other.stage_leaf_hash_arity4_byte_count;
        self.stage_leaf_coset_extend_call_count += other.stage_leaf_coset_extend_call_count;
        self.stage_leaf_coset_extend_output_byte_count +=
            other.stage_leaf_coset_extend_output_byte_count;
        self.stage_leaf_coset_extend_column_count += other.stage_leaf_coset_extend_column_count;
        self.stage_leaf_coset_extend_max_column_count = self
            .stage_leaf_coset_extend_max_column_count
            .max(other.stage_leaf_coset_extend_max_column_count);
        self.stage_leaf_coset_extend_ntt_launch_count +=
            other.stage_leaf_coset_extend_ntt_launch_count;
        self.stage_leaf_coset_extend_bit_reverse_launch_count +=
            other.stage_leaf_coset_extend_bit_reverse_launch_count;
        self.stage_leaf_coset_extend_ntt_stage_launch_count +=
            other.stage_leaf_coset_extend_ntt_stage_launch_count;
        self.stage_leaf_coset_extend_ntt_block_twiddle_launch_count +=
            other.stage_leaf_coset_extend_ntt_block_twiddle_launch_count;
        self.stage_leaf_coset_extend_normalize_launch_count +=
            other.stage_leaf_coset_extend_normalize_launch_count;
        self.stage_leaf_coset_extend_pack_launch_count +=
            other.stage_leaf_coset_extend_pack_launch_count;
        self.stage_leaf_coset_extend_unpack_launch_count +=
            other.stage_leaf_coset_extend_unpack_launch_count;
        self.stage_tree_commit_work_duration += other.stage_tree_commit_work_duration;
        self.stage_tree_commit_checkpoint_work_duration +=
            other.stage_tree_commit_checkpoint_work_duration;
        self.stage_tree_commit_root_work_duration += other.stage_tree_commit_root_work_duration;
        self.stage_tree_commit_root_count += other.stage_tree_commit_root_count;
        self.stage_tree_commit_root_byte_count += other.stage_tree_commit_root_byte_count;
        self.stage_tree_commit_retain_work_duration += other.stage_tree_commit_retain_work_duration;
        for stage_timing in other.stage_timings {
            self.accumulate_stage_timing(stage_timing);
        }
    }

    fn accumulate_indexed_stage_timing(&mut self, timing: WitnessIndexedStageCommitTiming) {
        self.accumulate_stage_timing(ProveWitnessGuestStageTiming::from_witness_stage_timing(
            timing,
        ));
    }

    fn accumulate_stage_timing(&mut self, stage_timing: ProveWitnessGuestStageTiming) {
        if let Some(existing) = self
            .stage_timings
            .iter_mut()
            .find(|existing| existing.stage_index == stage_timing.stage_index)
        {
            existing.accumulate(stage_timing);
            return;
        }
        self.stage_timings.push(stage_timing);
        self.stage_timings
            .sort_by_key(|stage_timing| stage_timing.stage_index);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    fn add_stage_source_retention(
        &mut self,
        attempt_count: usize,
        retained_count: usize,
        rejected_count: usize,
        retained_byte_count: usize,
        rejected_byte_count: usize,
        limit_byte_count: usize,
    ) {
        self.stage_source_retention_attempt_count += attempt_count;
        self.stage_source_retention_retained_count += retained_count;
        self.stage_source_retention_rejected_count += rejected_count;
        self.stage_source_retention_retained_byte_count = self
            .stage_source_retention_retained_byte_count
            .saturating_add(retained_byte_count);
        self.stage_source_retention_rejected_byte_count += rejected_byte_count;
        self.stage_source_retention_limit_byte_count = self
            .stage_source_retention_limit_byte_count
            .max(limit_byte_count);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    fn add_descriptor_buffer_retention(&mut self, retained_byte_len: usize, retained: bool) {
        self.descriptor_buffer_retention_attempt_count += 1;
        if retained {
            self.descriptor_buffer_retention_retained_count += 1;
            self.descriptor_buffer_retention_retained_byte_count = self
                .descriptor_buffer_retention_retained_byte_count
                .saturating_add(retained_byte_len);
        } else {
            self.descriptor_buffer_retention_rejected_count += 1;
            self.descriptor_buffer_retention_rejected_byte_count = self
                .descriptor_buffer_retention_rejected_byte_count
                .saturating_add(retained_byte_len);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProveWitnessAuxiliaryInputSlices<'a> {
    pub(crate) unit_values: &'a [Felt],
    pub(crate) proof_values: &'a [Felt],
    pub(crate) group_values: &'a [Ext3],
    pub(crate) challenges: &'a [Ext3],
    pub(crate) evaluations: &'a [Ext3],
}

impl<'a> From<&'a ProveWitnessAuxiliaryInputs> for ProveWitnessAuxiliaryInputSlices<'a> {
    fn from(inputs: &'a ProveWitnessAuxiliaryInputs) -> Self {
        Self {
            unit_values: &inputs.unit_values,
            proof_values: &inputs.proof_values,
            group_values: &inputs.group_values,
            challenges: &inputs.challenges,
            evaluations: &inputs.evaluations,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WitnessSharedInputs {
    input: Vec<u8>,
    publics: Vec<Felt>,
}

enum WitnessRegularHintMode<'a> {
    Balanced(&'a mut SourceLookupBalance),
    AssignmentsOnly,
}

#[derive(Debug, Clone, Copy)]
struct WitnessProofInputs<'a> {
    publics: &'a [Felt],
    auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
}

#[derive(Debug, Clone, Copy)]
struct WitnessRegularHintProgramInputs<'a> {
    program: &'a HintProgram,
    proof_inputs: WitnessProofInputs<'a>,
}

struct WitnessRegularTraceInputs<'a, L>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    layout: &'a WitnessTraceLayout,
    trace: Option<&'a WitnessTraceBuffer>,
    fixed_columns: WitnessFixedColumnsSource<'a, L>,
    stage_traces: &'a mut WitnessStageTraceCache,
    #[cfg(feature = "cuda")]
    stage_source_devices: Option<&'a WitnessStageSourceDeviceCache>,
}

struct WitnessTraceCommitmentInput<'a> {
    unit: &'a ProveUnitSchedule,
    layout: WitnessTraceLayout,
    trace: Option<WitnessTraceBuffer>,
    #[cfg(feature = "cuda")]
    terminal_trace_source_prefix_rows: Option<usize>,
    #[cfg(feature = "cuda")]
    stage_source_devices: Option<WitnessStageSourceDeviceCache>,
    #[cfg(feature = "cuda")]
    guest_pc_device_segment_material: Option<GuestPcTraceDeviceSegmentMaterial>,
}

struct ProveWitnessTraceRunObservers<'a> {
    fixed_columns_cache: Option<&'a mut WitnessFixedColumnsCache>,
    #[cfg(feature = "cuda")]
    stage_commitment_reuse_cache: Option<&'a mut WitnessStageCommitmentReuseCache>,
    #[cfg(feature = "cuda")]
    leaf_workspace_cache: Option<&'a mut WitnessStageLeafWorkspaceCache>,
    timing: Option<&'a mut ProveWitnessTraceTimingAccumulator>,
}

type WitnessFixedColumnsLoadResult =
    Result<crate::FixedColumnsMaterial, ProveWitnessCommitmentError>;
type WitnessFixedColumnsLoader =
    fn(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult;

enum WitnessFixedColumnsSource<'a, L = WitnessFixedColumnsLoader>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    Cache(&'a mut WitnessFixedColumnsCache<L>),
    #[allow(dead_code)]
    Material(&'a crate::FixedColumnsMaterial),
}

impl<L> WitnessFixedColumnsSource<'_, L>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    fn get_or_load(
        &mut self,
        unit_index: usize,
        plan_unit: &ProveExecutionUnitArtifacts,
        layout: &WitnessTraceLayout,
    ) -> Result<&crate::FixedColumnsMaterial, ProveWitnessCommitmentError> {
        match self {
            Self::Cache(cache) => cache.get_or_load(unit_index, plan_unit, layout),
            Self::Material(material) => {
                validate_fixed_columns_shape(
                    &material.fixed_columns,
                    plan_unit.fixed_column_count,
                    layout.row_count(),
                    unit_index,
                    &plan_unit.fixed_columns,
                )?;
                Ok(material)
            }
        }
    }
}

struct WitnessFixedColumnsCache<L = WitnessFixedColumnsLoader>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    material: Option<crate::FixedColumnsMaterial>,
    loader: L,
}

impl WitnessFixedColumnsCache<WitnessFixedColumnsLoader> {
    fn new() -> Self {
        Self::with_loader(load_witness_fixed_columns_material)
    }
}

impl<L> WitnessFixedColumnsCache<L>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    fn with_loader(loader: L) -> Self {
        Self {
            material: None,
            loader,
        }
    }

    fn get_or_load(
        &mut self,
        unit_index: usize,
        plan_unit: &ProveExecutionUnitArtifacts,
        layout: &WitnessTraceLayout,
    ) -> Result<&crate::FixedColumnsMaterial, ProveWitnessCommitmentError> {
        if self.material.is_none() {
            let material = (self.loader)(unit_index, plan_unit)?;
            validate_fixed_columns_shape(
                &material.fixed_columns,
                plan_unit.fixed_column_count,
                layout.row_count(),
                unit_index,
                &plan_unit.fixed_columns,
            )?;
            self.material = Some(material);
        }
        Ok(self
            .material
            .as_ref()
            .expect("fixed columns material should be cached after load"))
    }
}

#[derive(Default)]
struct WitnessStageTraceCache {
    stages: Option<Vec<WitnessTraceStageValues>>,
}

impl WitnessStageTraceCache {
    fn get_or_extract(
        &mut self,
        layout: &WitnessTraceLayout,
        trace: &WitnessTraceBuffer,
    ) -> Result<&[WitnessTraceStageValues], ProveWitnessCommitmentError> {
        if self.stages.is_none() {
            let stages = layout
                .stages()
                .iter()
                .map(|stage| layout.stage_trace(trace, stage.stage_index))
                .collect::<Result<Vec<_>, _>>()?;
            self.stages = Some(stages);
        }
        Ok(self
            .stages
            .as_deref()
            .expect("stage traces should be cached after extraction"))
    }

    fn get_or_extract_optional(
        &mut self,
        layout: &WitnessTraceLayout,
        trace: Option<&WitnessTraceBuffer>,
        consumer: &'static str,
    ) -> Result<&[WitnessTraceStageValues], ProveWitnessCommitmentError> {
        let trace = require_host_trace(trace, consumer)?;
        self.get_or_extract(layout, trace)
    }

    fn is_extracted(&self) -> bool {
        self.stages.is_some()
    }
}

#[cfg(feature = "cuda")]
#[derive(Default)]
struct WitnessStageSourceDeviceCache {
    trace: Option<Arc<CudaDeviceBuffer>>,
    guest_pc_device_descriptor_buffer: Option<Arc<CudaDeviceBuffer>>,
    stages: Vec<(usize, usize, usize, usize, usize, bool)>,
}

#[cfg(feature = "cuda")]
impl WitnessStageSourceDeviceCache {
    fn from_guest_pc_device_trace_builder(builder: GuestPcTraceDeviceTraceBuilder) -> Self {
        let stages = builder
            .stages()
            .iter()
            .map(|stage| {
                (
                    stage.stage_index(),
                    stage.row_count(),
                    stage.column_count(),
                    stage.row_stride(),
                    stage.column_offset(),
                    stage.is_known_zero(),
                )
            })
            .collect();
        Self {
            trace: Some(Arc::clone(builder.trace())),
            guest_pc_device_descriptor_buffer: builder.device_trace_descriptor_buffer().cloned(),
            stages,
        }
    }

    fn upload_from_trace_or_preloaded_if_empty(
        &mut self,
        layout: &WitnessTraceLayout,
        trace: Option<&WitnessTraceBuffer>,
        preloaded: Option<WitnessStageSourceDeviceCache>,
        terminal_trace_source_prefix_rows: Option<usize>,
    ) -> Result<(), ProveWitnessCommitmentError> {
        if let Some(preloaded) = preloaded {
            preloaded.validate_layout(layout)?;
            *self = preloaded;
            return Ok(());
        }
        let trace = require_host_trace(trace, "CUDA stage source upload")?;
        if terminal_sparse_trace_source_enabled() {
            if let Some(prefix_rows) = terminal_trace_source_prefix_rows {
                if prefix_rows < layout.row_count() {
                    return self.upload_from_trace_prefix_and_terminal_fill_if_empty(
                        layout,
                        trace,
                        prefix_rows,
                    );
                }
            }
        }
        if sparse_trace_source_enabled()
            && self.upload_from_trace_sparse_if_profitable_if_empty(layout, trace)?
        {
            return Ok(());
        }
        self.upload_from_trace_if_empty(layout, trace)
    }

    fn upload_from_trace_sparse_if_profitable_if_empty(
        &mut self,
        layout: &WitnessTraceLayout,
        trace: &WitnessTraceBuffer,
    ) -> Result<bool, ProveWitnessCommitmentError> {
        if self.trace.is_some() {
            return Ok(true);
        }
        validate_trace_shape(layout, trace)?;
        let trace_words = Felt::as_u64_slice(trace.values());
        let max_percent = sparse_trace_source_max_percent();
        let max_nonzero_words = trace_words.len().saturating_mul(max_percent) / 100;
        let mut nonzero_count = 0_usize;
        for word in trace_words {
            if *word != 0 {
                nonzero_count += 1;
                if nonzero_count > max_nonzero_words {
                    return Ok(false);
                }
            }
        }

        let mut indices = Vec::with_capacity(nonzero_count);
        let mut values = Vec::with_capacity(nonzero_count);
        for (index, word) in trace_words.iter().copied().enumerate() {
            if word != 0 {
                indices.push(u64::try_from(index).map_err(|_| {
                    ProveWitnessCommitmentError::PreloadedStageSource {
                        message: "sparse trace word index does not fit u64".to_owned(),
                    }
                })?);
                values.push(word);
            }
        }
        let trace_device = CudaDeviceBuffer::from_sparse_u64_words(
            trace_words.len(),
            indices.as_slice(),
            values.as_slice(),
        )
        .map_err(|source| {
            ProveWitnessCommitmentError::Commit(WitnessTraceCommitmentError::from(
                WitnessStageLeafError::from(source),
            ))
        })?;
        self.trace = Some(Arc::new(trace_device));
        self.guest_pc_device_descriptor_buffer = None;
        self.record_layout_stages(layout);
        if debug_sparse_trace_source_enabled() {
            eprintln!(
                "lzvm_cuda_sparse_trace_source_words={} nonzero={} max_percent={}",
                trace_words.len(),
                nonzero_count,
                max_percent
            );
        }
        Ok(true)
    }

    fn upload_from_trace_if_empty(
        &mut self,
        layout: &WitnessTraceLayout,
        trace: &WitnessTraceBuffer,
    ) -> Result<(), ProveWitnessCommitmentError> {
        if self.trace.is_some() {
            return Ok(());
        }
        validate_trace_shape(layout, trace)?;
        let trace_words = Felt::as_u64_slice(trace.values());
        let trace_device = CudaDeviceBuffer::from_u64_words(trace_words).map_err(|source| {
            ProveWitnessCommitmentError::Commit(WitnessTraceCommitmentError::from(
                WitnessStageLeafError::from(source),
            ))
        })?;
        self.trace = Some(Arc::new(trace_device));
        self.guest_pc_device_descriptor_buffer = None;
        self.record_layout_stages(layout);
        Ok(())
    }

    fn upload_from_trace_prefix_and_terminal_fill_if_empty(
        &mut self,
        layout: &WitnessTraceLayout,
        trace: &WitnessTraceBuffer,
        prefix_rows: usize,
    ) -> Result<(), ProveWitnessCommitmentError> {
        if self.trace.is_some() {
            return Ok(());
        }
        validate_trace_shape(layout, trace)?;
        if prefix_rows > trace.row_count() {
            return Err(ProveWitnessCommitmentError::PreloadedStageSource {
                message: format!(
                    "terminal prefix row count {prefix_rows} exceeds trace rows {}",
                    trace.row_count()
                ),
            });
        }
        if prefix_rows == trace.row_count() {
            return self.upload_from_trace_if_empty(layout, trace);
        }
        let row_width = trace.column_count();
        let trace_words = Felt::as_u64_slice(trace.values());
        let prefix_words = prefix_rows.checked_mul(row_width).ok_or(
            ProveWitnessCommitmentError::PreloadedStageSource {
                message: "terminal prefix word count overflow".to_owned(),
            },
        )?;
        let terminal_row = &trace_words[prefix_words..prefix_words + row_width];
        for row in trace_words[prefix_words..].chunks_exact(row_width) {
            if row != terminal_row {
                return self.upload_from_trace_if_empty(layout, trace);
            }
        }

        let trace_device = CudaDeviceBuffer::from_row_major_u64_prefix_and_suffix_row(
            &trace_words[..prefix_words],
            terminal_row,
            trace.row_count(),
            row_width,
            prefix_rows,
        )
        .map_err(|source| {
            ProveWitnessCommitmentError::Commit(WitnessTraceCommitmentError::from(
                WitnessStageLeafError::from(source),
            ))
        })?;
        self.trace = Some(Arc::new(trace_device));
        self.guest_pc_device_descriptor_buffer = None;
        self.record_layout_stages(layout);
        Ok(())
    }

    fn record_layout_stages(&mut self, layout: &WitnessTraceLayout) {
        self.stages.clear();
        self.stages.reserve(layout.stages().len());
        for stage in layout.stages() {
            self.stages.push((
                stage.stage_index,
                layout.row_count(),
                stage.width,
                layout.column_count(),
                stage.start_column,
                false,
            ));
        }
    }

    fn validate_layout(
        &self,
        layout: &WitnessTraceLayout,
    ) -> Result<(), ProveWitnessCommitmentError> {
        for stage in layout.stages() {
            let Some((row_count, column_count, row_stride, column_offset, _, _)) =
                self.get_stage(stage.stage_index)
            else {
                return Err(ProveWitnessCommitmentError::PreloadedStageSource {
                    message: format!(
                        "missing preloaded CUDA source for stage {}",
                        stage.stage_index
                    ),
                });
            };
            if row_count != layout.row_count()
                || column_count != stage.width
                || row_stride != layout.column_count()
                || column_offset != stage.start_column
            {
                return Err(ProveWitnessCommitmentError::PreloadedStageSource {
                    message: format!(
                        "preloaded CUDA source shape mismatch for stage {}",
                        stage.stage_index
                    ),
                });
            }
        }
        Ok(())
    }

    fn descriptors(&self) -> Vec<WitnessStageSourceDevice> {
        let Some(trace) = self.trace.as_ref() else {
            return Vec::new();
        };
        self.stages
            .iter()
            .map(
                |(stage_index, row_count, column_count, row_stride, column_offset, known_zero)| {
                    WitnessStageSourceDevice::from_row_major_column_window_with_known_zero(
                        *stage_index,
                        *row_count,
                        *column_count,
                        *row_stride,
                        *column_offset,
                        *known_zero,
                        trace,
                    )
                },
            )
            .collect()
    }

    fn retained_descriptors(
        &self,
        timing: Option<&mut ProveWitnessTraceTimingAccumulator>,
    ) -> Vec<WitnessStageRetainedSourceDevice> {
        let retention_limit = retained_source_device_limit();
        let mut attempt_count = 0usize;
        let mut rejected_count = 0usize;
        let mut retained_byte_count = 0usize;
        let mut rejected_byte_count = 0usize;
        let mut retained_buffer_keys = HashSet::new();
        let mut retained = Vec::new();
        for source_device in self.descriptors() {
            attempt_count += 1;
            let retained_byte_len = source_device.retained_byte_len();
            let retained_buffer_key = source_device.retained_buffer_key();
            if let Some(source_device) = source_device.retain() {
                if retained_buffer_keys.insert(retained_buffer_key) {
                    retained_byte_count = retained_byte_count.saturating_add(retained_byte_len);
                }
                retained.push(source_device);
            } else {
                rejected_count += 1;
                rejected_byte_count = rejected_byte_count.saturating_add(retained_byte_len);
            }
        }
        if let Some(timing) = timing {
            timing.add_stage_source_retention(
                attempt_count,
                retained.len(),
                rejected_count,
                retained_byte_count,
                rejected_byte_count,
                retention_limit,
            );
        }
        if debug_fri_stage_source_devices() {
            eprintln!(
                "lzvm_cuda_fri_stage_source_retained={} attempts={attempt_count} retained_bytes={retained_byte_count} rejected={rejected_count} rejected_bytes={rejected_byte_count} limit_bytes={retention_limit}",
                retained.len(),
            );
        }
        retained
    }

    fn stage_count(&self) -> usize {
        self.stages.len()
    }

    fn retained_guest_pc_device_descriptor_buffer(
        &self,
        timing: Option<&mut ProveWitnessTraceTimingAccumulator>,
    ) -> Option<WitnessRetainedDeviceBuffer> {
        let buffer = self.guest_pc_device_descriptor_buffer.as_ref()?;
        let retained_descriptor_buffer_byte_len = buffer.len();
        let retained = retain_device_buffer(buffer);
        if let Some(timing) = timing {
            timing.add_descriptor_buffer_retention(
                retained_descriptor_buffer_byte_len,
                retained.is_some(),
            );
        }
        retained
    }

    fn get(&self, stage_index: usize) -> Option<(usize, usize, usize, &CudaDeviceBuffer)> {
        let trace = self.trace.as_ref()?;
        self.stages
            .iter()
            .find(|(index, _, _, _, _, _)| *index == stage_index)
            .map(|(_, _, column_count, row_stride, column_offset, _)| {
                (*column_count, *row_stride, *column_offset, trace.as_ref())
            })
    }

    fn get_stage(
        &self,
        stage_index: usize,
    ) -> Option<(usize, usize, usize, usize, bool, &CudaDeviceBuffer)> {
        let trace = self.trace.as_ref()?;
        self.stages
            .iter()
            .find(|(index, _, _, _, _, _)| *index == stage_index)
            .map(
                |(_, row_count, column_count, row_stride, column_offset, known_zero)| {
                    (
                        *row_count,
                        *column_count,
                        *row_stride,
                        *column_offset,
                        *known_zero,
                        trace.as_ref(),
                    )
                },
            )
    }
}

#[cfg(feature = "cuda")]
fn validate_trace_shape(
    layout: &WitnessTraceLayout,
    trace: &WitnessTraceBuffer,
) -> Result<(), ProveWitnessCommitmentError> {
    if trace.row_count() != layout.row_count() || trace.column_count() != layout.column_count() {
        return Err(ProveWitnessCommitmentError::Layout(
            WitnessTraceLayoutError::TraceShapeMismatch {
                expected_rows: layout.row_count(),
                expected_columns: layout.column_count(),
                found_rows: trace.row_count(),
                found_columns: trace.column_count(),
            },
        ));
    }
    Ok(())
}

fn require_host_trace<'a>(
    trace: Option<&'a WitnessTraceBuffer>,
    consumer: &'static str,
) -> Result<&'a WitnessTraceBuffer, ProveWitnessCommitmentError> {
    trace.ok_or_else(|| ProveWitnessCommitmentError::PreloadedStageSource {
        message: format!("host trace is unavailable for {consumer}"),
    })
}

#[cfg(feature = "cuda")]
fn terminal_sparse_trace_source_enabled() -> bool {
    std::env::var("LZVM_CUDA_TERMINAL_SPARSE_TRACE_SOURCE")
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

#[cfg(feature = "cuda")]
fn sparse_trace_source_enabled() -> bool {
    std::env::var("LZVM_CUDA_SPARSE_TRACE_SOURCE")
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

#[cfg(feature = "cuda")]
fn sparse_trace_source_max_percent() -> usize {
    std::env::var("LZVM_CUDA_SPARSE_TRACE_SOURCE_MAX_PERCENT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|percent| (1..50).contains(percent))
        .unwrap_or(45)
}

#[cfg(feature = "cuda")]
fn debug_sparse_trace_source_enabled() -> bool {
    matches!(
        std::env::var("LZVM_CUDA_SPARSE_TRACE_SOURCE_DEBUG").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveWitnessCommitmentError {
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    InputData {
        path: PathBuf,
        message: String,
    },
    MissingWitnessLibrary,
    PublicInputs {
        path: PathBuf,
        source: PublicValuesError,
    },
    PublicInputsSetupHashMismatch,
    PublicInputNonCanonical {
        index: usize,
        value: u64,
    },
    WitnessLoad(WitnessLoadError),
    Layout(WitnessTraceLayoutError),
    WitnessRun(WitnessTraceRunError),
    BackendUnitValue {
        unit_index: usize,
        message: String,
    },
    BackendProofValue {
        unit_index: usize,
        message: String,
    },
    FixedColumns {
        unit_index: usize,
        path: PathBuf,
        source: Box<FixedColumnsMaterialError>,
    },
    FixedRowCountTooLarge {
        unit_index: usize,
        path: PathBuf,
        rows: u64,
    },
    FixedRowCountMismatch {
        unit_index: usize,
        path: PathBuf,
        expected: usize,
        found: usize,
    },
    FixedColumnCountMismatch {
        unit_index: usize,
        path: PathBuf,
        expected: usize,
        found: usize,
    },
    FixedColumnValueCountMismatch {
        unit_index: usize,
        path: PathBuf,
        column: String,
        expected: usize,
        found: usize,
    },
    FixedColumnValueCountOverflow {
        unit_index: usize,
        path: PathBuf,
    },
    FixedColumnNonCanonical {
        unit_index: usize,
        path: PathBuf,
        index: usize,
        value: u64,
    },
    StageIndexTooLarge {
        unit_index: usize,
        stage_index: usize,
    },
    MissingRegularConstraintInput {
        unit_index: usize,
        buffer: &'static str,
    },
    RegularConstraintDomainHelper {
        unit_index: usize,
        source: FriPolynomialError,
    },
    RegularConstraintEval(RegularConstraintEvalError),
    MissingRegularHintInput {
        unit_index: usize,
        source: &'static str,
    },
    RegularHintEval {
        unit_index: usize,
        source: HintEvalError,
    },
    GlobalHintEval {
        source: HintEvalError,
    },
    UnsupportedRegularHint {
        unit_index: usize,
        name: String,
    },
    SourceLookup {
        unit_index: usize,
        message: String,
    },
    SourceAssignment {
        unit_index: usize,
        message: String,
    },
    SourceLookupSet {
        message: String,
    },
    PreloadedStageSource {
        message: String,
    },
    SegmentCommitOutputOrder {
        message: String,
    },
    RegularConstraintViolation {
        unit_index: usize,
        constraint_index: usize,
        row: usize,
        value: [u64; 3],
    },
    Commit(WitnessTraceCommitmentError),
}

impl fmt::Display for ProveWitnessCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove witness commitment unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::InputData { path, message } => write!(
                f,
                "prove witness commitment input-data read failed: {}: {message}",
                path.display()
            ),
            Self::MissingWitnessLibrary => {
                write!(f, "prove witness commitment missing witness library")
            }
            Self::PublicInputs { path, source } => {
                write!(f, "read public inputs failed: {}: {source}", path.display())
            }
            Self::PublicInputsSetupHashMismatch => {
                write!(f, "public inputs setup hash mismatch")
            }
            Self::PublicInputNonCanonical { index, value } => write!(
                f,
                "public input field {index} is non-canonical: {value}"
            ),
            Self::WitnessLoad(error) => {
                write!(f, "prove witness commitment library load failed: {error}")
            }
            Self::Layout(error) => write!(f, "prove witness commitment layout failed: {error}"),
            Self::WitnessRun(error) => write!(f, "prove witness commitment run failed: {error}"),
            Self::BackendUnitValue {
                unit_index,
                message,
            } => write!(
                f,
                "prove witness commitment backend unit values failed for unit {unit_index}: {message}"
            ),
            Self::BackendProofValue {
                unit_index,
                message,
            } => write!(
                f,
                "prove witness commitment backend proof values failed for unit {unit_index}: {message}"
            ),
            Self::FixedColumns {
                unit_index,
                path,
                source,
            } => write!(
                f,
                "prove witness commitment fixed columns failed for unit {unit_index}: {}: {source}",
                path.display()
            ),
            Self::FixedRowCountTooLarge {
                unit_index,
                path,
                rows,
            } => write!(
                f,
                "prove witness commitment fixed-column row count is too large for unit {unit_index}: {}: {rows}",
                path.display()
            ),
            Self::FixedRowCountMismatch {
                unit_index,
                path,
                expected,
                found,
            } => write!(
                f,
                "prove witness commitment fixed-column row count mismatch for unit {unit_index}: {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnCountMismatch {
                unit_index,
                path,
                expected,
                found,
            } => write!(
                f,
                "prove witness commitment fixed-column count mismatch for unit {unit_index}: {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnValueCountMismatch {
                unit_index,
                path,
                column,
                expected,
                found,
            } => write!(
                f,
                "prove witness commitment fixed-column value count mismatch for unit {unit_index}: {}: {column}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnValueCountOverflow { unit_index, path } => write!(
                f,
                "prove witness commitment fixed-column value count overflow for unit {unit_index}: {}",
                path.display()
            ),
            Self::FixedColumnNonCanonical {
                unit_index,
                path,
                index,
                value,
            } => write!(
                f,
                "prove witness commitment fixed-column value is non-canonical for unit {unit_index}: {}: index {index}: {value}",
                path.display()
            ),
            Self::StageIndexTooLarge {
                unit_index,
                stage_index,
            } => write!(
                f,
                "prove witness commitment stage index does not fit u16 for unit {unit_index}: {stage_index}"
            ),
            Self::MissingRegularConstraintInput { unit_index, buffer } => write!(
                f,
                "missing regular constraint {buffer} input for prove witness commitment unit {unit_index}"
            ),
            Self::RegularConstraintDomainHelper { unit_index, source } => write!(
                f,
                "prove witness commitment regular constraint domain helper build failed for unit {unit_index}: {source}"
            ),
            Self::RegularConstraintEval(error) => {
                write!(f, "prove witness commitment regular constraint evaluation failed: {error}")
            }
            Self::MissingRegularHintInput { unit_index, source } => write!(
                f,
                "missing regular hint {source} input for prove witness commitment unit {unit_index}"
            ),
            Self::RegularHintEval { unit_index, source } => write!(
                f,
                "prove witness commitment regular hint evaluation failed for unit {unit_index}: {source}"
            ),
            Self::GlobalHintEval { source } => {
                write!(f, "prove witness commitment global hint evaluation failed: {source}")
            }
            Self::UnsupportedRegularHint { unit_index, name } => write!(
                f,
                "unsupported regular hint {name} for prove witness commitment unit {unit_index}"
            ),
            Self::SourceLookup {
                unit_index,
                message,
            } => write!(
                f,
                "source lookup validation failed for prove witness commitment unit {unit_index}: {message}"
            ),
            Self::SourceAssignment {
                unit_index,
                message,
            } => write!(
                f,
                "source assignment validation failed for prove witness commitment unit {unit_index}: {message}"
            ),
            Self::SourceLookupSet { message } => write!(
                f,
                "source lookup validation failed for prove witness commitment set: {message}"
            ),
            Self::PreloadedStageSource { message } => write!(
                f,
                "preloaded CUDA stage source failed for prove witness commitment: {message}"
            ),
            Self::SegmentCommitOutputOrder { message } => write!(
                f,
                "guest PC segment commit output ordering failed: {message}"
            ),
            Self::RegularConstraintViolation {
                unit_index,
                constraint_index,
                row,
                value,
            } => write!(
                f,
                "prove witness commitment regular constraint {constraint_index} failed for unit {unit_index} at row {row}: {value:?}"
            ),
            Self::Commit(error) => write!(f, "prove witness commitment failed: {error}"),
        }
    }
}

impl std::error::Error for ProveWitnessCommitmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::WitnessLoad(error) => Some(error),
            Self::Layout(error) => Some(error),
            Self::WitnessRun(error) => Some(error),
            Self::PublicInputs { source, .. } => Some(source),
            Self::FixedColumns { source, .. } => Some(source),
            Self::RegularConstraintDomainHelper { source, .. } => Some(source),
            Self::RegularConstraintEval(error) => Some(error),
            Self::RegularHintEval { source, .. } => Some(source),
            Self::GlobalHintEval { source } => Some(source),
            Self::Commit(error) => Some(error),
            Self::UnitIndexOutOfRange { .. }
            | Self::InputData { .. }
            | Self::MissingWitnessLibrary
            | Self::PublicInputsSetupHashMismatch
            | Self::PublicInputNonCanonical { .. }
            | Self::BackendUnitValue { .. }
            | Self::BackendProofValue { .. }
            | Self::FixedRowCountTooLarge { .. }
            | Self::FixedRowCountMismatch { .. }
            | Self::FixedColumnCountMismatch { .. }
            | Self::FixedColumnValueCountMismatch { .. }
            | Self::FixedColumnValueCountOverflow { .. }
            | Self::FixedColumnNonCanonical { .. }
            | Self::StageIndexTooLarge { .. }
            | Self::MissingRegularConstraintInput { .. }
            | Self::MissingRegularHintInput { .. }
            | Self::UnsupportedRegularHint { .. }
            | Self::SourceLookup { .. }
            | Self::SourceAssignment { .. }
            | Self::SourceLookupSet { .. }
            | Self::PreloadedStageSource { .. }
            | Self::SegmentCommitOutputOrder { .. }
            | Self::RegularConstraintViolation { .. } => None,
        }
    }
}

impl From<WitnessLoadError> for ProveWitnessCommitmentError {
    fn from(error: WitnessLoadError) -> Self {
        Self::WitnessLoad(error)
    }
}

impl From<WitnessTraceLayoutError> for ProveWitnessCommitmentError {
    fn from(error: WitnessTraceLayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<WitnessTraceRunError> for ProveWitnessCommitmentError {
    fn from(error: WitnessTraceRunError) -> Self {
        Self::WitnessRun(error)
    }
}

impl From<RegularConstraintEvalError> for ProveWitnessCommitmentError {
    fn from(error: RegularConstraintEvalError) -> Self {
        Self::RegularConstraintEval(error)
    }
}

impl From<WitnessTraceCommitmentError> for ProveWitnessCommitmentError {
    fn from(error: WitnessTraceCommitmentError) -> Self {
        Self::Commit(error)
    }
}

impl From<SourceLookupHintError> for ProveWitnessCommitmentError {
    fn from(error: SourceLookupHintError) -> Self {
        match error {
            SourceLookupHintError::Unit {
                unit_index,
                message,
            } => Self::SourceLookup {
                unit_index,
                message,
            },
            SourceLookupHintError::Set { message } => Self::SourceLookupSet { message },
        }
    }
}

pub fn run_prove_witness_commitments(
    plan: &ProveExecutionPlan,
    unit_index: usize,
) -> Result<ProveWitnessCommitments, ProveWitnessCommitmentError> {
    run_prove_witness_commitments_with_auxiliary_inputs(
        plan,
        unit_index,
        ProveWitnessAuxiliaryInputs::default(),
    )
}

pub fn run_prove_witness_commitments_with_auxiliary_inputs(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
) -> Result<ProveWitnessCommitments, ProveWitnessCommitmentError> {
    run_prove_witness_commitments_with_trace(plan, unit_index, auxiliary_inputs)
        .map(ProveWitnessTraceCommitments::into_commitments)
}

pub fn run_prove_witness_commitments_with_trace(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let Some(witness_library) = &plan.inputs.witness_library else {
        return Err(ProveWitnessCommitmentError::MissingWitnessLibrary);
    };
    let library = load_witness_library(witness_library)?;
    run_prove_witness_commitments_with_trace_backend(plan, unit_index, auxiliary_inputs, &library)
}

/// Runs witness commitments with a caller-supplied witness backend.
pub fn run_prove_witness_commitments_with_trace_backend<B: WitnessBackend + ?Sized>(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    backend: &B,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let mut source_lookup_balance = SourceLookupBalance::default();
    validate_witness_unit_index(plan, unit_index)?;
    let shared_inputs = load_witness_shared_inputs(plan)?;
    let auxiliary_inputs = Arc::new(auxiliary_inputs);
    let defer_cross_unit_source_lookup = should_defer_cross_unit_source_lookup(plan, unit_index);
    let output = if defer_cross_unit_source_lookup {
        run_prove_witness_commitments_with_trace_backend_inner(
            plan,
            unit_index,
            &shared_inputs,
            Arc::clone(&auxiliary_inputs),
            backend,
            WitnessRegularHintMode::AssignmentsOnly,
        )?
    } else {
        let output = run_prove_witness_commitments_with_trace_backend_inner(
            plan,
            unit_index,
            &shared_inputs,
            Arc::clone(&auxiliary_inputs),
            backend,
            WitnessRegularHintMode::Balanced(&mut source_lookup_balance),
        )?;
        accumulate_witness_global_hints(
            plan,
            &shared_inputs.publics,
            output.auxiliary_inputs(),
            &mut source_lookup_balance,
        )?;
        source_lookup_balance.validate_all_units()?;
        output
    };
    Ok(output)
}

pub fn run_prove_witness_commitments_with_guest_pc_trace_segments(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    instruction_limit: u64,
) -> Result<Vec<ProveWitnessTraceCommitments>, ProveWitnessCommitmentError> {
    let mut source_lookup_balance = SourceLookupBalance::default();
    validate_witness_unit_index(plan, unit_index)?;
    let shared_inputs = load_witness_shared_inputs(plan)?;
    let auxiliary_inputs = Arc::new(auxiliary_inputs);
    let defer_cross_unit_source_lookup = should_defer_cross_unit_source_lookup(plan, unit_index);
    let outputs = if defer_cross_unit_source_lookup {
        run_prove_witness_commitments_with_guest_pc_trace_segments_inner(
            plan,
            unit_index,
            &shared_inputs,
            Arc::clone(&auxiliary_inputs),
            instruction_limit,
            None,
        )?
    } else {
        let outputs = run_prove_witness_commitments_with_guest_pc_trace_segments_inner(
            plan,
            unit_index,
            &shared_inputs,
            Arc::clone(&auxiliary_inputs),
            instruction_limit,
            Some(&mut source_lookup_balance),
        )?;
        let global_auxiliary_inputs =
            global_auxiliary_inputs_from_outputs(auxiliary_inputs.as_ref(), &outputs)?;
        accumulate_witness_global_hints(
            plan,
            &shared_inputs.publics,
            &global_auxiliary_inputs,
            &mut source_lookup_balance,
        )?;
        source_lookup_balance.validate_all_units()?;
        outputs
    };
    Ok(outputs)
}

pub fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    instruction_limit: u64,
) -> Result<Vec<ProveWitnessTraceCommitments>, ProveWitnessCommitmentError> {
    run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_optional_timings(
        plan,
        unit_index,
        auxiliary_inputs,
        instruction_limit,
        None,
    )
}

pub fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_with_timings(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    instruction_limit: u64,
    timing_observer: &mut dyn FnMut(ProveWitnessGuestPcTraceTiming),
) -> Result<Vec<ProveWitnessTraceCommitments>, ProveWitnessCommitmentError> {
    run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_optional_timings(
        plan,
        unit_index,
        auxiliary_inputs,
        instruction_limit,
        Some(timing_observer),
    )
}

fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_optional_timings(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    instruction_limit: u64,
    timing_observer: Option<&mut dyn FnMut(ProveWitnessGuestPcTraceTiming)>,
) -> Result<Vec<ProveWitnessTraceCommitments>, ProveWitnessCommitmentError> {
    validate_witness_unit_index(plan, unit_index)?;
    let shared_inputs = load_witness_shared_inputs(plan)?;
    let auxiliary_inputs = Arc::new(auxiliary_inputs);
    let mut timing_observer = timing_observer;
    let result = run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_attempt(
        plan,
        unit_index,
        &shared_inputs,
        Arc::clone(&auxiliary_inputs),
        instruction_limit,
        timing_observer
            .as_mut()
            .map(|observer| &mut **observer as &mut dyn FnMut(ProveWitnessGuestPcTraceTiming)),
        None,
    );
    match result {
        Ok(outputs) => Ok(outputs),
        Err(error)
            if should_retry_guest_pc_segment_commit_with_serial_worker(
                shared_inputs.input.len(),
                None,
                &error,
            ) =>
        {
            run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_attempt(
                plan,
                unit_index,
                &shared_inputs,
                auxiliary_inputs,
                instruction_limit,
                timing_observer.as_mut().map(|observer| {
                    &mut **observer as &mut dyn FnMut(ProveWitnessGuestPcTraceTiming)
                }),
                Some(1),
            )
        }
        Err(error) => Err(error),
    }
}

fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_attempt(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    shared_inputs: &WitnessSharedInputs,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    instruction_limit: u64,
    timing_observer: Option<&mut dyn FnMut(ProveWitnessGuestPcTraceTiming)>,
    segment_commit_worker_count_override: Option<usize>,
) -> Result<Vec<ProveWitnessTraceCommitments>, ProveWitnessCommitmentError> {
    let mut source_lookup_balance = SourceLookupBalance::default();
    let defer_cross_unit_source_lookup = should_defer_cross_unit_source_lookup(plan, unit_index);
    let outputs = if defer_cross_unit_source_lookup {
        run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner(
            plan,
            unit_index,
            Arc::clone(&auxiliary_inputs),
            instruction_limit,
            GuestPcTraceSegmentCommitRunOptions {
                shared_inputs,
                source_lookup_balance: None,
                timing_observer,
                segment_commit_worker_count_override,
            },
        )?
    } else {
        let outputs = run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner(
            plan,
            unit_index,
            Arc::clone(&auxiliary_inputs),
            instruction_limit,
            GuestPcTraceSegmentCommitRunOptions {
                shared_inputs,
                source_lookup_balance: Some(&mut source_lookup_balance),
                timing_observer,
                segment_commit_worker_count_override,
            },
        )?;
        let global_auxiliary_inputs =
            global_auxiliary_inputs_from_outputs(auxiliary_inputs.as_ref(), &outputs)?;
        accumulate_witness_global_hints(
            plan,
            &shared_inputs.publics,
            &global_auxiliary_inputs,
            &mut source_lookup_balance,
        )?;
        source_lookup_balance.validate_all_units()?;
        outputs
    };
    Ok(outputs)
}

fn global_auxiliary_inputs_from_outputs(
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    outputs: &[ProveWitnessTraceCommitments],
) -> Result<ProveWitnessAuxiliaryInputs, ProveWitnessCommitmentError> {
    let mut merged = auxiliary_inputs.clone();
    if !merged.proof_values.is_empty() {
        return Ok(merged);
    }

    let mut proof_values = None;
    for output in outputs {
        let candidate = output.auxiliary_inputs().proof_values.as_slice();
        if candidate.is_empty() {
            continue;
        }
        if let Some(existing) = proof_values {
            if existing != candidate {
                return Err(ProveWitnessCommitmentError::BackendProofValue {
                    unit_index: output.commitments().unit_index(),
                    message: "backend proof values conflict across witness outputs".to_owned(),
                });
            }
        } else {
            proof_values = Some(candidate);
        }
    }
    if let Some(proof_values) = proof_values {
        merged.proof_values = proof_values.to_vec();
    }
    Ok(merged)
}

pub fn run_prove_witness_commitments_with_trace_bytes(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: ProveWitnessAuxiliaryInputs,
    trace_bytes: &[u8],
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let mut source_lookup_balance = SourceLookupBalance::default();
    validate_witness_unit_index(plan, unit_index)?;
    let shared_inputs = load_witness_shared_inputs(plan)?;
    let auxiliary_inputs = Arc::new(auxiliary_inputs);
    let defer_cross_unit_source_lookup = should_defer_cross_unit_source_lookup(plan, unit_index);
    let output = if defer_cross_unit_source_lookup {
        run_prove_witness_commitments_with_trace_bytes_inner(
            plan,
            unit_index,
            &shared_inputs,
            Arc::clone(&auxiliary_inputs),
            trace_bytes,
            WitnessRegularHintMode::AssignmentsOnly,
        )?
    } else {
        let output = run_prove_witness_commitments_with_trace_bytes_inner(
            plan,
            unit_index,
            &shared_inputs,
            Arc::clone(&auxiliary_inputs),
            trace_bytes,
            WitnessRegularHintMode::Balanced(&mut source_lookup_balance),
        )?;
        accumulate_witness_global_hints(
            plan,
            &shared_inputs.publics,
            auxiliary_inputs.as_ref(),
            &mut source_lookup_balance,
        )?;
        source_lookup_balance.validate_all_units()?;
        output
    };
    Ok(output)
}

fn should_defer_cross_unit_source_lookup(plan: &ProveExecutionPlan, unit_index: usize) -> bool {
    let Some(unit) = plan.run_plan.schedule.units.get(unit_index) else {
        return false;
    };
    unit.kind == KeyUnitKind::Basic
        && plan
            .run_plan
            .schedule
            .units
            .iter()
            .filter(|unit| unit.kind == KeyUnitKind::Basic)
            .count()
            > 1
}

fn run_prove_witness_commitments_with_trace_backend_inner<B: WitnessBackend + ?Sized>(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    shared_inputs: &WitnessSharedInputs,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    backend: &B,
    regular_hint_mode: WitnessRegularHintMode<'_>,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let unit_count = plan.run_plan.schedule.units.len();
    let unit = plan.run_plan.schedule.units.get(unit_index).ok_or(
        ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index,
            unit_count,
        },
    )?;
    let layout = derive_witness_trace_layout(unit)?;
    let trace_output = run_witness_trace_output_with_context(
        backend,
        WitnessComputeContext {
            guest_image: Some(&plan.inputs.guest_image),
            guest_image_info: Some(&plan.guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(&shared_inputs.input[..]),
    )?;
    let auxiliary_inputs = merge_backend_unit_values(
        unit_index,
        unit,
        auxiliary_inputs,
        trace_output.unit_values(),
    )?;
    let auxiliary_inputs = merge_backend_proof_values(
        unit_index,
        &plan.global_info,
        auxiliary_inputs,
        trace_output.proof_values(),
    )?;
    let trace = trace_output.into_trace();
    run_prove_witness_commitments_from_trace_inner(
        plan,
        unit_index,
        shared_inputs,
        auxiliary_inputs,
        WitnessTraceCommitmentInput {
            unit,
            layout,
            trace: Some(trace),
            #[cfg(feature = "cuda")]
            terminal_trace_source_prefix_rows: None,
            #[cfg(feature = "cuda")]
            stage_source_devices: None,
            #[cfg(feature = "cuda")]
            guest_pc_device_segment_material: None,
        },
        regular_hint_mode,
        ProveWitnessTraceRunObservers {
            fixed_columns_cache: None,
            #[cfg(feature = "cuda")]
            stage_commitment_reuse_cache: None,
            #[cfg(feature = "cuda")]
            leaf_workspace_cache: None,
            timing: None,
        },
    )
}

#[cfg(feature = "cuda")]
fn build_preloaded_guest_pc_trace_stage_source_devices(
    layout: &WitnessTraceLayout,
    segment_output: &GuestPcTraceSegmentRunOutput,
    timing: Option<&mut ProveWitnessTraceTimingAccumulator>,
) -> Result<Option<WitnessStageSourceDeviceCache>, ProveWitnessCommitmentError> {
    if let Some(material) = segment_output.device_segment_material() {
        let mut source_timing = GuestPcDeviceSourceBuildTiming::default();
        let builder = build_guest_pc_trace_stage_source_devices_from_device_material_timing(
            layout,
            material,
            timing.as_ref().map(|_| &mut source_timing),
        )?;
        if let Some(timing) = timing {
            timing.device_source_descriptor_upload_duration +=
                source_timing.descriptor_upload_duration();
            timing.device_source_descriptor_upload_byte_count +=
                source_timing.descriptor_upload_byte_count();
            timing.device_source_descriptor_upload_word_count +=
                source_timing.descriptor_upload_word_count();
            timing.device_source_descriptor_upload_row_count +=
                source_timing.descriptor_upload_row_count();
            timing.device_source_trace_expand_duration += source_timing.trace_expand_duration();
        }
        return Ok(Some(
            WitnessStageSourceDeviceCache::from_guest_pc_device_trace_builder(builder),
        ));
    }

    let trace = segment_output.trace_if_available().ok_or_else(|| {
        ProveWitnessCommitmentError::PreloadedStageSource {
            message: "guest PC segment host trace is unavailable for CUDA source fallback"
                .to_owned(),
        }
    })?;
    build_guest_pc_trace_stage_source_devices(
        layout,
        trace,
        segment_output.trace_source_prefix_rows(),
        segment_output.device_trace_descriptors(),
    )
    .map(|source| source.map(WitnessStageSourceDeviceCache::from_guest_pc_device_trace_builder))
    .map_err(ProveWitnessCommitmentError::from)
}

#[cfg(feature = "cuda")]
fn require_guest_pc_segment_host_trace(
    trace: Option<WitnessTraceBuffer>,
) -> Result<WitnessTraceBuffer, ProveWitnessCommitmentError> {
    trace.ok_or_else(|| ProveWitnessCommitmentError::PreloadedStageSource {
        message: "guest PC segment host trace is unavailable for commitment input".to_owned(),
    })
}

#[cfg(not(feature = "cuda"))]
fn guest_pc_segment_commitment_trace(
    segment_output: GuestPcTraceSegmentRunOutput,
    has_external_device_source_material: bool,
) -> Result<Option<WitnessTraceBuffer>, ProveWitnessCommitmentError> {
    if has_external_device_source_material && guest_pc_trace_less_commitment_input_enabled() {
        return Ok(None);
    }
    segment_output
        .into_trace()
        .ok_or_else(|| ProveWitnessCommitmentError::PreloadedStageSource {
            message: "guest PC segment host trace is unavailable for commitment input".to_owned(),
        })
        .map(Some)
}

#[cfg(feature = "cuda")]
fn guest_pc_segment_commitment_trace(
    segment_output: GuestPcTraceSegmentRunOutput,
    has_external_device_source_material: bool,
) -> Result<
    (
        Option<WitnessTraceBuffer>,
        Option<GuestPcTraceDeviceSegmentMaterial>,
    ),
    ProveWitnessCommitmentError,
> {
    let (trace, device_segment_material) = segment_output.into_trace_and_device_material();
    if has_external_device_source_material && guest_pc_trace_less_commitment_input_enabled() {
        return Ok((None, device_segment_material));
    }
    require_guest_pc_segment_host_trace(trace)
        .map(Some)
        .map(|trace| (trace, device_segment_material))
}

#[cfg(feature = "cuda")]
fn guest_pc_trace_less_commitment_input_enabled() -> bool {
    std::env::var("LZVM_CUDA_GUEST_PC_TRACELESS_COMMITMENT_INPUT")
        .map(|value| {
            !matches!(
                value.as_str(),
                "0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF"
            )
        })
        .unwrap_or(true)
}

#[cfg(not(feature = "cuda"))]
fn guest_pc_trace_less_commitment_input_enabled() -> bool {
    false
}

fn run_prove_witness_commitments_with_guest_pc_trace_segments_inner(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    shared_inputs: &WitnessSharedInputs,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    instruction_limit: u64,
    source_lookup_balance: Option<&mut SourceLookupBalance>,
) -> Result<Vec<ProveWitnessTraceCommitments>, ProveWitnessCommitmentError> {
    let unit_count = plan.run_plan.schedule.units.len();
    let unit = plan.run_plan.schedule.units.get(unit_index).ok_or(
        ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index,
            unit_count,
        },
    )?;
    let layout = derive_witness_trace_layout(unit)?;
    let backend = GuestPcTraceBackend::new(instruction_limit);
    let trace_outputs = run_guest_pc_trace_segments_with_context(
        &backend,
        WitnessComputeContext {
            guest_image: Some(&plan.inputs.guest_image),
            guest_image_info: Some(&plan.guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(&shared_inputs.input[..]),
    )?;
    let mut outputs = Vec::with_capacity(trace_outputs.len());
    let mut source_lookup_balance = source_lookup_balance;
    let mut fixed_columns_cache = WitnessFixedColumnsCache::new();
    #[cfg(feature = "cuda")]
    let mut leaf_workspace_cache = WitnessStageLeafWorkspaceCache::default();
    for segment_output in trace_outputs {
        let trace_instance_index = segment_output.trace_instance_index();
        #[cfg(feature = "cuda")]
        let trace_source_prefix_rows = segment_output.trace_source_prefix_rows();
        #[cfg(feature = "cuda")]
        let preloaded_stage_source_devices =
            build_preloaded_guest_pc_trace_stage_source_devices(&layout, &segment_output, None)?;
        #[cfg(feature = "cuda")]
        let has_preloaded_stage_source_devices = preloaded_stage_source_devices.is_some();
        #[cfg(feature = "cuda")]
        let has_external_device_source_material = has_preloaded_stage_source_devices
            && segment_output.device_segment_material().is_some();
        #[cfg(not(feature = "cuda"))]
        let has_external_device_source_material = false;
        let merged_inputs = merge_backend_unit_values(
            unit_index,
            unit,
            Arc::clone(&auxiliary_inputs),
            segment_output.unit_values(),
        )?;
        let merged_inputs = merge_backend_proof_values(
            unit_index,
            &plan.global_info,
            merged_inputs,
            segment_output.proof_values(),
        )?;
        let regular_hint_mode = match source_lookup_balance {
            Some(ref mut balance) => WitnessRegularHintMode::Balanced(balance),
            None => WitnessRegularHintMode::AssignmentsOnly,
        };
        #[cfg(feature = "cuda")]
        let (trace, guest_pc_device_segment_material) =
            guest_pc_segment_commitment_trace(segment_output, has_external_device_source_material)?;
        #[cfg(not(feature = "cuda"))]
        let trace =
            guest_pc_segment_commitment_trace(segment_output, has_external_device_source_material)?;
        let mut output = run_prove_witness_commitments_from_trace_inner(
            plan,
            unit_index,
            shared_inputs,
            merged_inputs,
            WitnessTraceCommitmentInput {
                unit,
                layout: layout.clone(),
                trace,
                #[cfg(feature = "cuda")]
                terminal_trace_source_prefix_rows: Some(trace_source_prefix_rows),
                #[cfg(feature = "cuda")]
                stage_source_devices: preloaded_stage_source_devices,
                #[cfg(feature = "cuda")]
                guest_pc_device_segment_material,
            },
            regular_hint_mode,
            ProveWitnessTraceRunObservers {
                fixed_columns_cache: Some(&mut fixed_columns_cache),
                #[cfg(feature = "cuda")]
                stage_commitment_reuse_cache: None,
                #[cfg(feature = "cuda")]
                leaf_workspace_cache: Some(&mut leaf_workspace_cache),
                timing: None,
            },
        )?;
        output.commitments.identity.trace_instance_index = trace_instance_index;
        outputs.push(output);
    }
    Ok(outputs)
}

#[derive(Clone, Copy)]
struct GuestPcTraceSegmentCommitContext<'a> {
    plan: &'a ProveExecutionPlan,
    unit_index: usize,
    unit: &'a ProveUnitSchedule,
    layout: &'a WitnessTraceLayout,
    shared_inputs: &'a WitnessSharedInputs,
}

struct GuestPcTraceSegmentCommitScratch {
    fixed_columns_cache: WitnessFixedColumnsCache,
    #[cfg(feature = "cuda")]
    stage_commitment_reuse_cache: WitnessStageCommitmentReuseCache,
    #[cfg(feature = "cuda")]
    leaf_workspace_cache: WitnessStageLeafWorkspaceCache,
}

impl GuestPcTraceSegmentCommitScratch {
    fn new() -> Self {
        Self {
            fixed_columns_cache: WitnessFixedColumnsCache::new(),
            #[cfg(feature = "cuda")]
            stage_commitment_reuse_cache: WitnessStageCommitmentReuseCache::default(),
            #[cfg(feature = "cuda")]
            leaf_workspace_cache: WitnessStageLeafWorkspaceCache::default(),
        }
    }
}

struct GuestPcTraceSegmentCommitOutputCollector {
    outputs: Vec<ProveWitnessTraceCommitments>,
    pending_outputs: BTreeMap<u32, ProveWitnessTraceCommitments>,
    next_output_trace_instance_index: u32,
}

impl GuestPcTraceSegmentCommitOutputCollector {
    fn new() -> Self {
        Self {
            outputs: Vec::new(),
            pending_outputs: BTreeMap::new(),
            next_output_trace_instance_index: 0,
        }
    }

    fn collect_committed_segment(
        &mut self,
        output: ProveWitnessTraceCommitments,
    ) -> Result<(), ProveWitnessCommitmentError> {
        let trace_instance_index = output.commitments().trace_instance_index();
        if trace_instance_index < self.next_output_trace_instance_index {
            return Err(ProveWitnessCommitmentError::SegmentCommitOutputOrder {
                message: format!(
                    "duplicate or stale trace instance {trace_instance_index}; next expected {}",
                    self.next_output_trace_instance_index
                ),
            });
        }
        if self
            .pending_outputs
            .insert(trace_instance_index, output.without_trace())
            .is_some()
        {
            return Err(ProveWitnessCommitmentError::SegmentCommitOutputOrder {
                message: format!("duplicate trace instance {trace_instance_index}"),
            });
        }
        self.drain_ordered_outputs()
    }

    fn drain_ordered_outputs(&mut self) -> Result<(), ProveWitnessCommitmentError> {
        while let Some(output) = self
            .pending_outputs
            .remove(&self.next_output_trace_instance_index)
        {
            self.outputs.push(output);
            self.next_output_trace_instance_index = self
                .next_output_trace_instance_index
                .checked_add(1)
                .ok_or_else(|| ProveWitnessCommitmentError::SegmentCommitOutputOrder {
                    message: "trace instance index overflow".to_owned(),
                })?;
        }
        Ok(())
    }

    fn finish(self) -> Result<Vec<ProveWitnessTraceCommitments>, ProveWitnessCommitmentError> {
        if let Some((&trace_instance_index, _)) = self.pending_outputs.first_key_value() {
            return Err(ProveWitnessCommitmentError::SegmentCommitOutputOrder {
                message: format!(
                    "missing trace instance {} before pending trace instance {trace_instance_index}",
                    self.next_output_trace_instance_index
                ),
            });
        }
        Ok(self.outputs)
    }
}

struct GuestPcTraceSegmentCommitResult {
    output: ProveWitnessTraceCommitments,
    source_lookup_balance: SourceLookupBalance,
    trace_timing: Option<ProveWitnessTraceTimingAccumulator>,
    guest_segment_commit_duration: Option<Duration>,
}

struct GuestPcTraceSegmentCommitWorkerState {
    scratch: GuestPcTraceSegmentCommitScratch,
}

impl GuestPcTraceSegmentCommitWorkerState {
    fn new() -> Self {
        Self {
            scratch: GuestPcTraceSegmentCommitScratch::new(),
        }
    }

    fn commit_segment(
        &mut self,
        context: GuestPcTraceSegmentCommitContext<'_>,
        auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
        segment_output: GuestPcTraceSegmentRunOutput,
        use_source_lookup_balance: bool,
        collect_timing: bool,
    ) -> Result<GuestPcTraceSegmentCommitResult, ProveWitnessCommitmentError> {
        commit_guest_pc_trace_segment_with_scratch(
            context,
            auxiliary_inputs,
            segment_output,
            use_source_lookup_balance,
            collect_timing,
            &mut self.scratch,
        )
    }
}

type GuestPcTraceSegmentCommitWorkerHandle<'scope> = thread::ScopedJoinHandle<
    'scope,
    Result<GuestPcTraceSegmentCommitResult, ProveWitnessCommitmentError>,
>;

struct GuestPcTraceSegmentCommitWorkerPool<'scope, 'env> {
    scope: &'scope thread::Scope<'scope, 'env>,
    worker_count: usize,
    worker_state: GuestPcTraceSegmentCommitWorkerState,
    pending_workers: VecDeque<GuestPcTraceSegmentCommitWorkerHandle<'scope>>,
}

impl<'scope, 'env> GuestPcTraceSegmentCommitWorkerPool<'scope, 'env> {
    fn new(
        scope: &'scope thread::Scope<'scope, 'env>,
        input_byte_count: usize,
        worker_count_override: Option<usize>,
    ) -> Self {
        Self {
            scope,
            worker_count: guest_pc_trace_segment_commit_worker_count_for_input_with_override(
                input_byte_count,
                worker_count_override,
            ),
            worker_state: GuestPcTraceSegmentCommitWorkerState::new(),
            pending_workers: VecDeque::new(),
        }
    }

    fn submit_segment(
        &mut self,
        context: GuestPcTraceSegmentCommitContext<'env>,
        auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
        segment_output: GuestPcTraceSegmentRunOutput,
        use_source_lookup_balance: bool,
        collect_timing: bool,
    ) -> Result<Vec<GuestPcTraceSegmentCommitResult>, ProveWitnessCommitmentError> {
        if self.worker_count <= 1 {
            let result = self.worker_state.commit_segment(
                context,
                auxiliary_inputs,
                segment_output,
                use_source_lookup_balance,
                collect_timing,
            )?;
            return Ok(vec![result]);
        }

        let mut ready_results = Vec::new();
        while self.pending_workers.len() >= self.worker_count {
            let handle = self.pending_workers.pop_front().ok_or_else(|| {
                ProveWitnessCommitmentError::SegmentCommitOutputOrder {
                    message: "segment commit worker queue unexpectedly empty".to_owned(),
                }
            })?;
            match join_guest_pc_trace_segment_commit_worker(handle) {
                Ok(result) => ready_results.push(result),
                Err(error) => {
                    let _ = self.finish();
                    return Err(error);
                }
            }
        }

        self.pending_workers.push_back(self.scope.spawn(move || {
            let mut worker_state = GuestPcTraceSegmentCommitWorkerState::new();
            worker_state.commit_segment(
                context,
                auxiliary_inputs,
                segment_output,
                use_source_lookup_balance,
                collect_timing,
            )
        }));
        Ok(ready_results)
    }

    fn finish(
        &mut self,
    ) -> Result<Vec<GuestPcTraceSegmentCommitResult>, ProveWitnessCommitmentError> {
        let mut ready_results = Vec::new();
        let mut first_error = None;
        while let Some(handle) = self.pending_workers.pop_front() {
            match join_guest_pc_trace_segment_commit_worker(handle) {
                Ok(result) => ready_results.push(result),
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(ready_results)
    }
}

fn default_guest_pc_trace_segment_commit_worker_count_for_input(_input_byte_count: usize) -> usize {
    1
}

fn guest_pc_trace_segment_commit_worker_count_for_input(input_byte_count: usize) -> usize {
    std::env::var("LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|count| *count > 0)
        .unwrap_or_else(|| {
            default_guest_pc_trace_segment_commit_worker_count_for_input(input_byte_count)
        })
}

fn guest_pc_trace_segment_commit_worker_count_for_input_with_override(
    input_byte_count: usize,
    worker_count_override: Option<usize>,
) -> usize {
    worker_count_override
        .filter(|count| *count > 0)
        .unwrap_or_else(|| guest_pc_trace_segment_commit_worker_count_for_input(input_byte_count))
}

fn should_retry_guest_pc_segment_commit_with_serial_worker(
    input_byte_count: usize,
    worker_count_override: Option<usize>,
    error: &ProveWitnessCommitmentError,
) -> bool {
    guest_pc_trace_segment_commit_worker_count_for_input_with_override(
        input_byte_count,
        worker_count_override,
    ) > 1
        && prove_witness_commitment_error_is_cuda_out_of_memory(error)
}

#[cfg(feature = "cuda")]
fn prove_witness_commitment_error_is_cuda_out_of_memory(
    error: &ProveWitnessCommitmentError,
) -> bool {
    const CUDA_ERROR_OUT_OF_MEMORY: i32 = 2;
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(error) = source {
        if matches!(
            error.downcast_ref::<lzvm_accel::AccelError>(),
            Some(lzvm_accel::AccelError::Cuda { code }) if *code == CUDA_ERROR_OUT_OF_MEMORY
        ) {
            return true;
        }
        source = error.source();
    }
    false
}

#[cfg(not(feature = "cuda"))]
fn prove_witness_commitment_error_is_cuda_out_of_memory(
    _error: &ProveWitnessCommitmentError,
) -> bool {
    false
}

fn join_guest_pc_trace_segment_commit_worker(
    handle: GuestPcTraceSegmentCommitWorkerHandle<'_>,
) -> Result<GuestPcTraceSegmentCommitResult, ProveWitnessCommitmentError> {
    handle.join().map_err(|_| {
        ProveWitnessCommitmentError::Commit(WitnessTraceCommitmentError::WorkerPanic)
    })?
}

struct GuestPcTraceSegmentCommitDriver<'scope, 'env, 'b> {
    context: GuestPcTraceSegmentCommitContext<'env>,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    source_lookup_balance: Option<&'b mut SourceLookupBalance>,
    collect_timing: bool,
    output_collector: GuestPcTraceSegmentCommitOutputCollector,
    guest_segment_commit_duration: Duration,
    trace_timing: ProveWitnessTraceTimingAccumulator,
    segment_count: usize,
    worker_pool: GuestPcTraceSegmentCommitWorkerPool<'scope, 'env>,
}

struct GuestPcTraceSegmentCommitDriverOutput {
    outputs: Vec<ProveWitnessTraceCommitments>,
    guest_segment_commit_duration: Duration,
    trace_timing: ProveWitnessTraceTimingAccumulator,
    segment_count: usize,
}

impl<'scope, 'env, 'b> GuestPcTraceSegmentCommitDriver<'scope, 'env, 'b> {
    fn new(
        scope: &'scope thread::Scope<'scope, 'env>,
        context: GuestPcTraceSegmentCommitContext<'env>,
        auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
        input_byte_count: usize,
        source_lookup_balance: Option<&'b mut SourceLookupBalance>,
        collect_timing: bool,
        segment_commit_worker_count_override: Option<usize>,
    ) -> Self {
        Self {
            context,
            auxiliary_inputs,
            source_lookup_balance,
            collect_timing,
            output_collector: GuestPcTraceSegmentCommitOutputCollector::new(),
            guest_segment_commit_duration: Duration::ZERO,
            trace_timing: ProveWitnessTraceTimingAccumulator::default(),
            segment_count: 0,
            worker_pool: GuestPcTraceSegmentCommitWorkerPool::new(
                scope,
                input_byte_count,
                segment_commit_worker_count_override,
            ),
        }
    }

    fn commit_segment(
        &mut self,
        segment_output: GuestPcTraceSegmentRunOutput,
    ) -> Result<(), ProveWitnessCommitmentError> {
        let ready_results = self.worker_pool.submit_segment(
            self.context,
            Arc::clone(&self.auxiliary_inputs),
            segment_output,
            self.source_lookup_balance.is_some(),
            self.collect_timing,
        )?;
        for result in ready_results {
            self.collect_committed_segment_result(result)?;
        }
        Ok(())
    }

    fn collect_committed_segment_result(
        &mut self,
        result: GuestPcTraceSegmentCommitResult,
    ) -> Result<(), ProveWitnessCommitmentError> {
        if let Some(balance) = self.source_lookup_balance.as_deref_mut() {
            balance.merge(result.source_lookup_balance);
        }
        if let Some(trace_timing) = result.trace_timing {
            self.trace_timing.accumulate(trace_timing);
        }
        self.output_collector
            .collect_committed_segment(result.output)?;
        if let Some(duration) = result.guest_segment_commit_duration {
            self.guest_segment_commit_duration += duration;
            self.segment_count += 1;
        }
        Ok(())
    }

    fn finish(
        mut self,
    ) -> Result<GuestPcTraceSegmentCommitDriverOutput, ProveWitnessCommitmentError> {
        let pending_results = self.worker_pool.finish()?;
        for result in pending_results {
            self.collect_committed_segment_result(result)?;
        }
        Ok(GuestPcTraceSegmentCommitDriverOutput {
            outputs: self.output_collector.finish()?,
            guest_segment_commit_duration: self.guest_segment_commit_duration,
            trace_timing: self.trace_timing,
            segment_count: self.segment_count,
        })
    }
}

fn commit_guest_pc_trace_segment_with_scratch(
    context: GuestPcTraceSegmentCommitContext<'_>,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    segment_output: GuestPcTraceSegmentRunOutput,
    use_source_lookup_balance: bool,
    collect_timing: bool,
    scratch: &mut GuestPcTraceSegmentCommitScratch,
) -> Result<GuestPcTraceSegmentCommitResult, ProveWitnessCommitmentError> {
    let guest_segment_commit_started = collect_timing.then(Instant::now);
    let mut trace_timing = collect_timing.then(ProveWitnessTraceTimingAccumulator::default);
    let mut segment_source_lookup_balance = SourceLookupBalance::default();
    let regular_hint_mode = if use_source_lookup_balance {
        WitnessRegularHintMode::Balanced(&mut segment_source_lookup_balance)
    } else {
        WitnessRegularHintMode::AssignmentsOnly
    };
    let output = commit_guest_pc_trace_segment_output(GuestPcTraceSegmentCommitRequest {
        context,
        auxiliary_inputs,
        segment_output,
        regular_hint_mode,
        scratch,
        timing: trace_timing.as_mut(),
    })?;
    Ok(GuestPcTraceSegmentCommitResult {
        output,
        source_lookup_balance: segment_source_lookup_balance,
        trace_timing,
        guest_segment_commit_duration: guest_segment_commit_started
            .map(|started| started.elapsed()),
    })
}

struct GuestPcTraceSegmentCommitRequest<'a, 'b> {
    context: GuestPcTraceSegmentCommitContext<'a>,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    segment_output: GuestPcTraceSegmentRunOutput,
    regular_hint_mode: WitnessRegularHintMode<'b>,
    scratch: &'b mut GuestPcTraceSegmentCommitScratch,
    timing: Option<&'b mut ProveWitnessTraceTimingAccumulator>,
}

fn commit_guest_pc_trace_segment_output(
    request: GuestPcTraceSegmentCommitRequest<'_, '_>,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let GuestPcTraceSegmentCommitRequest {
        context,
        auxiliary_inputs,
        segment_output,
        regular_hint_mode,
        scratch,
        timing,
    } = request;
    let GuestPcTraceSegmentCommitContext {
        plan,
        unit_index,
        unit,
        layout,
        shared_inputs,
    } = context;
    #[cfg(feature = "cuda")]
    let mut timing = timing;
    let trace_instance_index = segment_output.trace_instance_index();
    #[cfg(feature = "cuda")]
    let trace_source_prefix_rows = segment_output.trace_source_prefix_rows();
    #[cfg(feature = "cuda")]
    let preloaded_stage_source_devices = match timing.as_mut() {
        Some(timing) => {
            let timing = &mut **timing;
            let started = Instant::now();
            let result = build_preloaded_guest_pc_trace_stage_source_devices(
                layout,
                &segment_output,
                Some(timing),
            );
            timing.device_source_build_duration += started.elapsed();
            result
        }
        None => build_preloaded_guest_pc_trace_stage_source_devices(layout, &segment_output, None),
    }?;
    #[cfg(feature = "cuda")]
    let has_preloaded_stage_source_devices = preloaded_stage_source_devices.is_some();
    #[cfg(feature = "cuda")]
    let has_external_device_source_material =
        has_preloaded_stage_source_devices && segment_output.device_segment_material().is_some();
    #[cfg(not(feature = "cuda"))]
    let has_external_device_source_material = false;
    let merged_inputs = merge_backend_unit_values(
        unit_index,
        unit,
        auxiliary_inputs,
        segment_output.unit_values(),
    )?;
    #[cfg(feature = "cuda")]
    let (trace, guest_pc_device_segment_material) =
        guest_pc_segment_commitment_trace(segment_output, has_external_device_source_material)?;
    #[cfg(not(feature = "cuda"))]
    let trace =
        guest_pc_segment_commitment_trace(segment_output, has_external_device_source_material)?;
    let mut output = run_prove_witness_commitments_from_trace_inner(
        plan,
        unit_index,
        shared_inputs,
        merged_inputs,
        WitnessTraceCommitmentInput {
            unit,
            layout: layout.clone(),
            trace,
            #[cfg(feature = "cuda")]
            terminal_trace_source_prefix_rows: Some(trace_source_prefix_rows),
            #[cfg(feature = "cuda")]
            stage_source_devices: preloaded_stage_source_devices,
            #[cfg(feature = "cuda")]
            guest_pc_device_segment_material,
        },
        regular_hint_mode,
        ProveWitnessTraceRunObservers {
            fixed_columns_cache: Some(&mut scratch.fixed_columns_cache),
            #[cfg(feature = "cuda")]
            stage_commitment_reuse_cache: Some(&mut scratch.stage_commitment_reuse_cache),
            #[cfg(feature = "cuda")]
            leaf_workspace_cache: Some(&mut scratch.leaf_workspace_cache),
            timing,
        },
    )?;
    output.commitments.identity.trace_instance_index = trace_instance_index;
    Ok(output)
}

struct GuestPcTraceSegmentCommitRunOptions<'shared, 'balance, 'timing> {
    shared_inputs: &'shared WitnessSharedInputs,
    source_lookup_balance: Option<&'balance mut SourceLookupBalance>,
    timing_observer: Option<&'timing mut dyn FnMut(ProveWitnessGuestPcTraceTiming)>,
    segment_commit_worker_count_override: Option<usize>,
}

fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    instruction_limit: u64,
    options: GuestPcTraceSegmentCommitRunOptions<'_, '_, '_>,
) -> Result<Vec<ProveWitnessTraceCommitments>, ProveWitnessCommitmentError> {
    let GuestPcTraceSegmentCommitRunOptions {
        shared_inputs,
        source_lookup_balance,
        mut timing_observer,
        segment_commit_worker_count_override,
    } = options;
    let unit_count = plan.run_plan.schedule.units.len();
    let unit = plan.run_plan.schedule.units.get(unit_index).ok_or(
        ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index,
            unit_count,
        },
    )?;
    let execution_unit =
        plan.units
            .get(unit_index)
            .ok_or(ProveWitnessCommitmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: plan.units.len(),
            })?;
    let layout = derive_witness_trace_layout(unit)?;
    let backend = GuestPcTraceBackend::new(instruction_limit);
    let context = WitnessComputeContext {
        guest_image: Some(&plan.inputs.guest_image),
        guest_image_info: Some(&plan.guest_image_info),
        trace_layout: Some(&layout),
    };
    let segment_commit_context = GuestPcTraceSegmentCommitContext {
        plan,
        unit_index,
        unit,
        layout: &layout,
        shared_inputs,
    };
    if auxiliary_inputs.proof_values.is_empty()
        && !proof_value_dependency::regular_program_uses_proof_values(
            execution_unit.stage_count,
            &execution_unit.regular_constraints,
            &execution_unit.regular_hints,
        )
    {
        return thread::scope(|scope| {
            let collect_timing = timing_observer.is_some();
            let mut commit_driver = GuestPcTraceSegmentCommitDriver::new(
                scope,
                segment_commit_context,
                auxiliary_inputs,
                shared_inputs.input.len(),
                source_lookup_balance,
                collect_timing,
                segment_commit_worker_count_override,
            );
            let guest_trace_stream_started = collect_timing.then(Instant::now);
            let stream_result =
                for_each_guest_pc_trace_segment_collecting_proof_values_with_context(
                    &backend,
                    context,
                    layout.request(&shared_inputs.input[..]),
                    |segment_output| commit_driver.commit_segment(segment_output),
                )
                .map_err(|error| match error {
                    GuestPcTraceSegmentStreamError::Trace(error) => {
                        ProveWitnessCommitmentError::from(error)
                    }
                    GuestPcTraceSegmentStreamError::Emit(error) => error,
                })?;
            let commit_output = commit_driver.finish()?;
            let proof_values = stream_result.proof_values;
            if let Some(started) = guest_trace_stream_started {
                let guest_trace_stream_elapsed_duration = started.elapsed();
                let guest_trace_stream_duration = guest_trace_stream_elapsed_duration
                    .saturating_sub(commit_output.guest_segment_commit_duration);
                if let Some(observer) = timing_observer.as_deref_mut() {
                    observer(ProveWitnessGuestPcTraceTiming::new(
                        commit_output.segment_count,
                        guest_trace_stream_elapsed_duration,
                        guest_trace_stream_duration,
                        commit_output.guest_segment_commit_duration,
                        stream_result.timing,
                        commit_output.trace_timing,
                    ));
                }
            }
            let mut outputs = commit_output.outputs;
            for output in &mut outputs {
                output.auxiliary_inputs = merge_backend_proof_values(
                    unit_index,
                    &plan.global_info,
                    Arc::clone(&output.auxiliary_inputs),
                    &proof_values,
                )?;
            }
            Ok(outputs)
        });
    }
    let proof_values = run_guest_pc_trace_runtime_proof_values_with_context(
        &backend,
        context,
        &shared_inputs.input,
    )?;
    let auxiliary_inputs = merge_backend_proof_values(
        unit_index,
        &plan.global_info,
        auxiliary_inputs,
        &proof_values,
    )?;
    thread::scope(|scope| {
        let collect_timing = timing_observer.is_some();
        let mut commit_driver = GuestPcTraceSegmentCommitDriver::new(
            scope,
            segment_commit_context,
            auxiliary_inputs,
            shared_inputs.input.len(),
            source_lookup_balance,
            collect_timing,
            segment_commit_worker_count_override,
        );
        let guest_trace_stream_started = collect_timing.then(Instant::now);
        let stream_timing = for_each_guest_pc_trace_segment_with_context(
            &backend,
            context,
            layout.request(&shared_inputs.input[..]),
            &proof_values,
            |segment_output| commit_driver.commit_segment(segment_output),
        )
        .map_err(|error| match error {
            GuestPcTraceSegmentStreamError::Trace(error) => {
                ProveWitnessCommitmentError::from(error)
            }
            GuestPcTraceSegmentStreamError::Emit(error) => error,
        })?;
        let commit_output = commit_driver.finish()?;
        if let Some(started) = guest_trace_stream_started {
            let guest_trace_stream_elapsed_duration = started.elapsed();
            let guest_trace_stream_duration = guest_trace_stream_elapsed_duration
                .saturating_sub(commit_output.guest_segment_commit_duration);
            if let Some(observer) = timing_observer {
                observer(ProveWitnessGuestPcTraceTiming::new(
                    commit_output.segment_count,
                    guest_trace_stream_elapsed_duration,
                    guest_trace_stream_duration,
                    commit_output.guest_segment_commit_duration,
                    stream_timing,
                    commit_output.trace_timing,
                ));
            }
        }
        Ok(commit_output.outputs)
    })
}

fn merge_backend_unit_values(
    unit_index: usize,
    unit: &ProveUnitSchedule,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    backend_unit_values: &[WitnessTraceUnitValue],
) -> Result<Arc<ProveWitnessAuxiliaryInputs>, ProveWitnessCommitmentError> {
    if backend_unit_values.is_empty() || unit.unit_value_map.is_empty() {
        return Ok(auxiliary_inputs);
    }

    let packed_values =
        pack_backend_unit_values(unit_index, &unit.unit_value_map, backend_unit_values)?;
    let mut merged = auxiliary_inputs.as_ref().clone();
    merged.unit_values = packed_values;
    Ok(Arc::new(merged))
}

fn merge_backend_proof_values(
    unit_index: usize,
    global_info: &GlobalInfo,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    backend_proof_values: &[WitnessTraceProofValue],
) -> Result<Arc<ProveWitnessAuxiliaryInputs>, ProveWitnessCommitmentError> {
    if backend_proof_values.is_empty() || global_info.proof_values_map.is_empty() {
        return Ok(auxiliary_inputs);
    }

    let packed_values = pack_backend_proof_values(unit_index, global_info, backend_proof_values)?;
    if auxiliary_inputs.proof_values.is_empty() {
        let mut merged = auxiliary_inputs.as_ref().clone();
        merged.proof_values = packed_values;
        return Ok(Arc::new(merged));
    }
    if auxiliary_inputs.proof_values == packed_values {
        return Ok(auxiliary_inputs);
    }
    Err(ProveWitnessCommitmentError::BackendProofValue {
        unit_index,
        message: "backend proof values conflict with provided proof values".to_owned(),
    })
}

fn pack_backend_proof_values(
    unit_index: usize,
    global_info: &GlobalInfo,
    backend_proof_values: &[WitnessTraceProofValue],
) -> Result<Vec<Felt>, ProveWitnessCommitmentError> {
    let mut packed_values = Vec::new();
    for entry in &global_info.proof_values_map {
        let mut matches = backend_proof_values
            .iter()
            .filter(|value| value.name() == entry.name);
        let Some(value) = matches.next() else {
            return Err(ProveWitnessCommitmentError::BackendProofValue {
                unit_index,
                message: format!("missing {}", entry.name),
            });
        };
        if matches.next().is_some() {
            return Err(ProveWitnessCommitmentError::BackendProofValue {
                unit_index,
                message: format!("duplicate {}", entry.name),
            });
        }
        let expected = named_stage_value_packed_field_count(entry).map_err(|message| {
            ProveWitnessCommitmentError::BackendProofValue {
                unit_index,
                message,
            }
        })?;
        if value.values().len() != expected {
            return Err(ProveWitnessCommitmentError::BackendProofValue {
                unit_index,
                message: format!(
                    "{} value count mismatch: expected {}, found {}",
                    entry.name,
                    expected,
                    value.values().len()
                ),
            });
        }
        packed_values.extend_from_slice(value.values());
    }
    Ok(packed_values)
}

fn pack_backend_unit_values(
    unit_index: usize,
    unit_value_map: &[StageValue],
    backend_unit_values: &[WitnessTraceUnitValue],
) -> Result<Vec<Felt>, ProveWitnessCommitmentError> {
    let mut packed_values = Vec::new();
    for entry in unit_value_map {
        let mut matches = backend_unit_values
            .iter()
            .filter(|value| value.name() == entry.name);
        let Some(value) = matches.next() else {
            return Err(ProveWitnessCommitmentError::BackendUnitValue {
                unit_index,
                message: format!("missing {}", entry.name),
            });
        };
        if matches.next().is_some() {
            return Err(ProveWitnessCommitmentError::BackendUnitValue {
                unit_index,
                message: format!("duplicate {}", entry.name),
            });
        }
        let expected = stage_value_packed_field_count(entry).map_err(|message| {
            ProveWitnessCommitmentError::BackendUnitValue {
                unit_index,
                message,
            }
        })?;
        if value.values().len() != expected {
            return Err(ProveWitnessCommitmentError::BackendUnitValue {
                unit_index,
                message: format!(
                    "{} value count mismatch: expected {}, found {}",
                    entry.name,
                    expected,
                    value.values().len()
                ),
            });
        }
        packed_values.extend_from_slice(value.values());
    }
    Ok(packed_values)
}

fn named_stage_value_packed_field_count(value: &NamedStageValue) -> Result<usize, String> {
    let dimension = value
        .lengths
        .iter()
        .try_fold(1_usize, |dimension, length| {
            let length =
                usize::try_from(*length).map_err(|_| "proof value length overflow".to_owned())?;
            if length == 0 {
                return Err("proof value length must be nonzero".to_owned());
            }
            dimension
                .checked_mul(length)
                .ok_or_else(|| "proof value dimension overflow".to_owned())
        })?;
    if dimension == 0 {
        return Err("proof value dimension must be nonzero".to_owned());
    }
    let width = if value.stage == 1 { 1 } else { 3 };
    dimension
        .checked_mul(width)
        .ok_or_else(|| "proof value packed field count overflow".to_owned())
}

fn stage_value_packed_field_count(value: &StageValue) -> Result<usize, String> {
    let dimension = value
        .lengths
        .iter()
        .try_fold(1_usize, |dimension, length| {
            let length =
                usize::try_from(*length).map_err(|_| "unit value length overflow".to_owned())?;
            if length == 0 {
                return Err("unit value length must be nonzero".to_owned());
            }
            dimension
                .checked_mul(length)
                .ok_or_else(|| "unit value length overflow".to_owned())
        })?;
    let width = if value.stage == 1 { 1 } else { 3 };
    dimension
        .checked_mul(width)
        .ok_or_else(|| "unit value length overflow".to_owned())
}

fn run_prove_witness_commitments_with_trace_bytes_inner(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    shared_inputs: &WitnessSharedInputs,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    trace_bytes: &[u8],
    regular_hint_mode: WitnessRegularHintMode<'_>,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let unit_count = plan.run_plan.schedule.units.len();
    let unit = plan.run_plan.schedule.units.get(unit_index).ok_or(
        ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index,
            unit_count,
        },
    )?;
    let layout = derive_witness_trace_layout(unit)?;
    let output_len = trace_output_byte_len(layout.row_count(), layout.column_count())?;
    if trace_bytes.len() > output_len {
        return Err(
            WitnessTraceRunError::Call(WitnessCallError::OutputOverflow {
                produced_len: trace_bytes.len(),
                output_len,
            })
            .into(),
        );
    }
    let trace = parse_witness_trace(trace_bytes, layout.row_count(), layout.column_count())
        .map_err(WitnessTraceRunError::from)?;
    run_prove_witness_commitments_from_trace_inner(
        plan,
        unit_index,
        shared_inputs,
        auxiliary_inputs,
        WitnessTraceCommitmentInput {
            unit,
            layout,
            trace: Some(trace),
            #[cfg(feature = "cuda")]
            terminal_trace_source_prefix_rows: None,
            #[cfg(feature = "cuda")]
            stage_source_devices: None,
            #[cfg(feature = "cuda")]
            guest_pc_device_segment_material: None,
        },
        regular_hint_mode,
        ProveWitnessTraceRunObservers {
            fixed_columns_cache: None,
            #[cfg(feature = "cuda")]
            stage_commitment_reuse_cache: None,
            #[cfg(feature = "cuda")]
            leaf_workspace_cache: None,
            timing: None,
        },
    )
}

fn run_prove_witness_commitments_from_trace_inner(
    plan: &ProveExecutionPlan,
    unit_index: usize,
    shared_inputs: &WitnessSharedInputs,
    auxiliary_inputs: Arc<ProveWitnessAuxiliaryInputs>,
    input: WitnessTraceCommitmentInput<'_>,
    regular_hint_mode: WitnessRegularHintMode<'_>,
    mut observers: ProveWitnessTraceRunObservers<'_>,
) -> Result<ProveWitnessTraceCommitments, ProveWitnessCommitmentError> {
    let mut timing = observers.timing;
    let WitnessTraceCommitmentInput {
        unit,
        layout,
        trace,
        #[cfg(feature = "cuda")]
        terminal_trace_source_prefix_rows,
        #[cfg(feature = "cuda")]
        stage_source_devices,
        #[cfg(feature = "cuda")]
        guest_pc_device_segment_material,
    } = input;
    let input_byte_count = shared_inputs.input.len();
    let execution_unit =
        plan.units
            .get(unit_index)
            .ok_or(ProveWitnessCommitmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: plan.units.len(),
            })?;
    let mut local_fixed_columns = WitnessFixedColumnsCache::new();
    let fixed_columns = observers
        .fixed_columns_cache
        .as_deref_mut()
        .unwrap_or(&mut local_fixed_columns);
    let mut stage_trace_cache = WitnessStageTraceCache::default();
    #[cfg(feature = "cuda")]
    let mut stage_source_device_cache = WitnessStageSourceDeviceCache::default();
    let proof_inputs = WitnessProofInputs {
        publics: &shared_inputs.publics,
        auxiliary_inputs: auxiliary_inputs.as_ref(),
    };
    let trace_ref = trace.as_ref();
    let regular_constraint_count = execution_unit.regular_constraints.entries.len();
    #[cfg(feature = "cuda")]
    let trace_extracted = trace_ref.is_some()
        || stage_source_devices.is_some()
        || guest_pc_device_segment_material.is_some();
    #[cfg(not(feature = "cuda"))]
    let trace_extracted = trace_ref.is_some();
    #[cfg(feature = "cuda")]
    let external_source_commitment_required = guest_pc_device_segment_material.is_some();
    #[cfg(feature = "cuda")]
    {
        stage_source_device_cache.upload_from_trace_or_preloaded_if_empty(
            &layout,
            trace_ref,
            stage_source_devices,
            terminal_trace_source_prefix_rows,
        )?;
    }
    {
        let mut regular_inputs = WitnessRegularTraceInputs {
            layout: &layout,
            trace: trace_ref,
            fixed_columns: WitnessFixedColumnsSource::Cache(fixed_columns),
            stage_traces: &mut stage_trace_cache,
            #[cfg(feature = "cuda")]
            stage_source_devices: Some(&stage_source_device_cache),
        };
        record_optional_duration(
            timing
                .as_deref_mut()
                .map(|timing| &mut timing.regular_constraint_duration),
            || {
                validate_witness_regular_constraints(
                    execution_unit,
                    unit_index,
                    &mut regular_inputs,
                    proof_inputs,
                    plan.run_plan.gpu.witness_thread_pools,
                )
            },
        )?;
        record_optional_duration(
            timing
                .as_deref_mut()
                .map(|timing| &mut timing.regular_hint_duration),
            || match regular_hint_mode {
                WitnessRegularHintMode::Balanced(source_lookup_balance) => {
                    accumulate_witness_regular_hints(
                        execution_unit,
                        unit_index,
                        &mut regular_inputs,
                        proof_inputs,
                        source_lookup_balance,
                    )
                }
                WitnessRegularHintMode::AssignmentsOnly => {
                    validate_witness_regular_source_assignments(
                        execution_unit,
                        unit_index,
                        &mut regular_inputs,
                        proof_inputs,
                    )
                }
            },
        )?;
    }
    let trace_rows = layout.row_count();
    let trace_columns = layout.column_count();
    let stage_commitments = if let Some(timing) = timing.as_mut() {
        let timing = &mut **timing;
        let stage_commit_started = Instant::now();
        let (stage_commitments, stage_timing, stage_timings) = (|| {
            let mut stage_timing = WitnessStageCommitTiming::default();
            #[cfg(feature = "cuda")]
            let source_devices = stage_source_device_cache.descriptors();
            #[cfg(feature = "cuda")]
            let (stage_commitments, stage_timings) = if stage_trace_cache.is_extracted() {
                let stage_traces = record_optional_duration(
                    Some(&mut timing.stage_trace_extract_duration),
                    || {
                        stage_trace_cache.get_or_extract_optional(
                            &layout,
                            trace_ref,
                            "timed extracted CUDA stage commitment",
                        )
                    },
                )?;
                if let Some(reuse_cache) = observers.stage_commitment_reuse_cache.as_mut() {
                    commit_witness_stage_values_with_source_devices_reusing_cached_stages_and_indexed_timing(
                        stage_traces,
                        unit,
                        &source_devices,
                        reuse_cache,
                        &mut stage_timing,
                    )?
                } else {
                    commit_witness_stage_values_with_source_devices_and_indexed_timing(
                        stage_traces,
                        unit,
                        plan.run_plan.gpu.witness_thread_pools,
                        &source_devices,
                        &mut stage_timing,
                    )?
                }
            } else {
                let source_commit_result = if external_source_commitment_required {
                    commit_witness_stage_source_devices_and_indexed_timing_external_source_with_leaf_workspace_cache(
                        &source_devices,
                        unit,
                        &mut stage_timing,
                        observers.leaf_workspace_cache.as_deref_mut(),
                    )
                } else {
                    commit_witness_stage_source_devices_and_indexed_timing_with_leaf_workspace_cache(
                        &source_devices,
                        unit,
                        &mut stage_timing,
                        observers.leaf_workspace_cache.as_deref_mut(),
                    )
                };
                match source_commit_result {
                    Ok(result) => result,
                    Err(error) if source_device_retention_unavailable(&error) => {
                        let stage_traces = record_optional_duration(
                            Some(&mut timing.stage_trace_extract_duration),
                            || {
                                stage_trace_cache.get_or_extract_optional(
                                    &layout,
                                    trace_ref,
                                    "timed CUDA source-device retention fallback",
                                )
                            },
                        )?;
                        if let Some(reuse_cache) = observers.stage_commitment_reuse_cache.as_mut() {
                            commit_witness_stage_values_with_source_devices_reusing_cached_stages_and_indexed_timing(
                                stage_traces,
                                unit,
                                &source_devices,
                                reuse_cache,
                                &mut stage_timing,
                            )?
                        } else {
                            commit_witness_stage_values_with_source_devices_and_indexed_timing(
                                stage_traces,
                                unit,
                                plan.run_plan.gpu.witness_thread_pools,
                                &source_devices,
                                &mut stage_timing,
                            )?
                        }
                    }
                    Err(error) => return Err(ProveWitnessCommitmentError::from(error)),
                }
            };
            #[cfg(not(feature = "cuda"))]
            let (stage_commitments, stage_timings) = {
                let stage_traces = record_optional_duration(
                    Some(&mut timing.stage_trace_extract_duration),
                    || {
                        stage_trace_cache.get_or_extract_optional(
                            &layout,
                            trace_ref,
                            "timed CPU stage commitment",
                        )
                    },
                )?;
                commit_witness_stage_values_with_workers_and_indexed_timing(
                    stage_traces,
                    unit,
                    plan.run_plan.gpu.witness_thread_pools,
                    &mut stage_timing,
                )?
            };
            Ok::<_, ProveWitnessCommitmentError>((stage_commitments, stage_timing, stage_timings))
        })()?;
        timing.stage_commit_duration += stage_commit_started.elapsed();
        timing.stage_leaf_extend_work_duration += stage_timing.leaf_extend_duration();
        timing.stage_leaf_setup_work_duration += stage_timing.leaf_setup_duration();
        timing.stage_leaf_setup_prepare_duration += stage_timing.leaf_setup_prepare_duration();
        timing.stage_leaf_setup_output_alloc_duration +=
            stage_timing.leaf_setup_output_alloc_duration();
        timing.stage_leaf_setup_workspace_alloc_duration +=
            stage_timing.leaf_setup_workspace_alloc_duration();
        timing.stage_leaf_setup_output_alloc_byte_count +=
            stage_timing.leaf_setup_output_alloc_byte_count();
        timing.stage_leaf_setup_workspace_alloc_byte_count +=
            stage_timing.leaf_setup_workspace_alloc_byte_count();
        timing.stage_leaf_setup_output_alloc_count += stage_timing.leaf_setup_output_alloc_count();
        timing.stage_leaf_output_cache_hit_count += stage_timing.leaf_output_cache_hit_count();
        timing.stage_leaf_output_cache_miss_count += stage_timing.leaf_output_cache_miss_count();
        timing.stage_leaf_setup_workspace_alloc_count +=
            stage_timing.leaf_setup_workspace_alloc_count();
        timing.stage_leaf_upload_work_duration += stage_timing.leaf_upload_duration();
        timing.stage_leaf_kernel_work_duration += stage_timing.leaf_kernel_duration();
        timing.stage_leaf_download_work_duration += stage_timing.leaf_download_duration();
        timing.stage_leaf_validate_work_duration += stage_timing.leaf_validate_duration();
        timing.stage_leaf_hash_work_duration += stage_timing.leaf_hash_duration();
        timing.stage_leaf_hash_row_count += stage_timing.leaf_hash_row_count();
        timing.stage_leaf_hash_byte_count += stage_timing.leaf_hash_byte_count();
        timing.stage_leaf_hash_arity2_row_count += stage_timing.leaf_hash_arity2_row_count();
        timing.stage_leaf_hash_arity2_byte_count += stage_timing.leaf_hash_arity2_byte_count();
        timing.stage_leaf_hash_arity4_row_count += stage_timing.leaf_hash_arity4_row_count();
        timing.stage_leaf_hash_arity4_byte_count += stage_timing.leaf_hash_arity4_byte_count();
        timing.stage_leaf_coset_extend_call_count += stage_timing.leaf_coset_extend_call_count();
        timing.stage_leaf_coset_extend_output_byte_count +=
            stage_timing.leaf_coset_extend_output_byte_count();
        timing.stage_leaf_coset_extend_column_count +=
            stage_timing.leaf_coset_extend_column_count();
        timing.stage_leaf_coset_extend_max_column_count = timing
            .stage_leaf_coset_extend_max_column_count
            .max(stage_timing.leaf_coset_extend_max_column_count());
        timing.stage_leaf_coset_extend_ntt_launch_count +=
            stage_timing.leaf_coset_extend_ntt_launch_count();
        timing.stage_leaf_coset_extend_bit_reverse_launch_count +=
            stage_timing.leaf_coset_extend_bit_reverse_launch_count();
        timing.stage_leaf_coset_extend_ntt_stage_launch_count +=
            stage_timing.leaf_coset_extend_ntt_stage_launch_count();
        timing.stage_leaf_coset_extend_ntt_block_twiddle_launch_count +=
            stage_timing.leaf_coset_extend_ntt_block_twiddle_launch_count();
        timing.stage_leaf_coset_extend_normalize_launch_count +=
            stage_timing.leaf_coset_extend_normalize_launch_count();
        timing.stage_leaf_coset_extend_pack_launch_count +=
            stage_timing.leaf_coset_extend_pack_launch_count();
        timing.stage_leaf_coset_extend_unpack_launch_count +=
            stage_timing.leaf_coset_extend_unpack_launch_count();
        timing.stage_tree_commit_work_duration += stage_timing.tree_commit_duration();
        timing.stage_tree_commit_checkpoint_work_duration +=
            stage_timing.tree_commit_checkpoint_duration();
        timing.stage_tree_commit_root_work_duration += stage_timing.tree_commit_root_duration();
        timing.stage_tree_commit_root_count += stage_timing.tree_commit_root_count();
        timing.stage_tree_commit_root_byte_count += stage_timing.tree_commit_root_byte_count();
        timing.stage_tree_commit_retain_work_duration += stage_timing.tree_commit_retain_duration();
        for stage_timing in stage_timings {
            timing.accumulate_indexed_stage_timing(stage_timing);
        }
        stage_commitments
    } else if stage_trace_cache.is_extracted() {
        let stage_traces = stage_trace_cache.get_or_extract_optional(
            &layout,
            trace_ref,
            "extracted stage commitment",
        )?;
        #[cfg(feature = "cuda")]
        {
            let source_devices = stage_source_device_cache.descriptors();
            if let Some(reuse_cache) = observers.stage_commitment_reuse_cache.as_mut() {
                commit_witness_stage_values_with_source_devices_reusing_cached_stages_and_workers(
                    stage_traces,
                    unit,
                    &source_devices,
                    reuse_cache,
                )?
            } else {
                commit_witness_stage_values_with_source_devices_and_workers(
                    stage_traces,
                    unit,
                    plan.run_plan.gpu.witness_thread_pools,
                    &source_devices,
                )?
            }
        }
        #[cfg(not(feature = "cuda"))]
        commit_witness_stage_values_with_workers(
            stage_traces,
            unit,
            plan.run_plan.gpu.witness_thread_pools,
        )?
    } else if cfg!(feature = "cuda") {
        #[cfg(feature = "cuda")]
        {
            let source_devices = stage_source_device_cache.descriptors();
            let mut stage_timing = WitnessStageCommitTiming::default();
            let source_commit_result = if external_source_commitment_required {
                commit_witness_stage_source_devices_and_indexed_timing_external_source_with_leaf_workspace_cache(
                    &source_devices,
                    unit,
                    &mut stage_timing,
                    observers.leaf_workspace_cache.as_deref_mut(),
                )
            } else {
                commit_witness_stage_source_devices_and_indexed_timing_with_leaf_workspace_cache(
                    &source_devices,
                    unit,
                    &mut stage_timing,
                    observers.leaf_workspace_cache.as_deref_mut(),
                )
            };
            match source_commit_result {
                Ok((commitments, _)) => commitments,
                Err(error) if source_device_retention_unavailable(&error) => {
                    let stage_traces = stage_trace_cache.get_or_extract_optional(
                        &layout,
                        trace_ref,
                        "CUDA source-device retention fallback",
                    )?;
                    if let Some(reuse_cache) = observers.stage_commitment_reuse_cache.as_mut() {
                        commit_witness_stage_values_with_source_devices_reusing_cached_stages_and_workers(
                            stage_traces,
                            unit,
                            &source_devices,
                            reuse_cache,
                        )?
                    } else {
                        commit_witness_stage_values_with_source_devices_and_workers(
                            stage_traces,
                            unit,
                            plan.run_plan.gpu.witness_thread_pools,
                            &source_devices,
                        )?
                    }
                }
                Err(error) => return Err(ProveWitnessCommitmentError::from(error)),
            }
        }
        #[cfg(not(feature = "cuda"))]
        unreachable!()
    } else {
        let trace = require_host_trace(trace_ref, "CPU witness stage commitment")?;
        commit_witness_trace_stages_with_workers(
            trace,
            unit,
            plan.run_plan.gpu.witness_thread_pools,
        )?
    };
    let commitments = ProveWitnessCommitments {
        identity: ProveTraceIdentity::new(unit_index, 0),
        input_byte_count,
        trace_rows,
        trace_columns,
        stage_commitments,
    };

    #[cfg(feature = "cuda")]
    let retain_stage_sources = retain_fri_stage_source_devices();
    #[cfg(feature = "cuda")]
    let retained_stage_source_devices = if retain_stage_sources {
        stage_source_device_cache.retained_descriptors(timing.as_deref_mut())
    } else {
        Vec::new()
    };
    #[cfg(feature = "cuda")]
    let guest_pc_device_descriptor_buffer = if retain_stage_sources
        && retained_stage_source_devices.len() < stage_source_device_cache.stage_count()
    {
        stage_source_device_cache.retained_guest_pc_device_descriptor_buffer(timing)
    } else {
        None
    };
    Ok(ProveWitnessTraceCommitments {
        commitments,
        trace,
        trace_constraint_checks: ProveWitnessTraceConstraintChecks {
            regular_constraint_count,
            trace_extracted,
            regular_constraints_evaluated: true,
            witness_values_committed: true,
            constraint_checker_conformant: true,
        },
        #[cfg(feature = "cuda")]
        stage_source_devices: retained_stage_source_devices,
        #[cfg(feature = "cuda")]
        guest_pc_device_descriptor_buffer,
        #[cfg(feature = "cuda")]
        guest_pc_device_segment_material,
        publics: shared_inputs.publics.clone(),
        auxiliary_inputs,
    })
}

#[cfg(feature = "cuda")]
fn retain_fri_stage_source_devices() -> bool {
    !matches!(
        std::env::var("LZVM_CUDA_RETAIN_FRI_STAGE_SOURCES").as_deref(),
        Ok("0") | Ok("false") | Ok("no")
    )
}

#[cfg(feature = "cuda")]
fn debug_fri_stage_source_devices() -> bool {
    matches!(
        std::env::var("LZVM_CUDA_FRI_STAGE_SOURCE_DEBUG").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn record_optional_duration<T>(
    duration: Option<&mut Duration>,
    run: impl FnOnce() -> Result<T, ProveWitnessCommitmentError>,
) -> Result<T, ProveWitnessCommitmentError> {
    if let Some(duration) = duration {
        let started = Instant::now();
        let result = run()?;
        *duration += started.elapsed();
        Ok(result)
    } else {
        run()
    }
}

#[cfg(feature = "cuda")]
fn source_device_retention_unavailable(error: &WitnessTraceCommitmentError) -> bool {
    matches!(
        error,
        WitnessTraceCommitmentError::StageCommitment(
            WitnessStageCommitmentError::SourceDeviceRetentionUnavailable { .. }
        )
    )
}

pub fn run_prove_witness_commitments_for_all_units(
    plan: &ProveExecutionPlan,
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    backend: &(impl WitnessBackend + ?Sized),
) -> Result<Vec<ProveWitnessTraceCommitments>, String> {
    let mut outputs = Vec::with_capacity(plan.units.len());
    let mut source_lookup_balance = SourceLookupBalance::default();
    let shared_inputs = load_witness_shared_inputs(plan).map_err(|error| error.to_string())?;
    let auxiliary_inputs = Arc::new(auxiliary_inputs.clone());
    for unit_index in 0..plan.units.len() {
        let output = run_prove_witness_commitments_with_trace_backend_inner(
            plan,
            unit_index,
            &shared_inputs,
            Arc::clone(&auxiliary_inputs),
            backend,
            WitnessRegularHintMode::Balanced(&mut source_lookup_balance),
        )
        .map_err(|error| {
            format!("run witness commitments failed for unit {unit_index}: {error}")
        })?;
        outputs.push(output);
    }
    let global_auxiliary_inputs =
        global_auxiliary_inputs_from_outputs(auxiliary_inputs.as_ref(), &outputs)
            .map_err(|error| error.to_string())?;
    accumulate_witness_global_hints(
        plan,
        &shared_inputs.publics,
        &global_auxiliary_inputs,
        &mut source_lookup_balance,
    )
    .map_err(|error| error.to_string())?;
    source_lookup_balance
        .validate_all_units()
        .map_err(|error| error.to_string())?;
    Ok(outputs)
}

pub fn run_prove_witness_commitments_for_all_units_with_trace_bundle(
    plan: &ProveExecutionPlan,
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    bundle: &(impl TraceBundleSource + ?Sized),
) -> Result<Vec<ProveWitnessTraceCommitments>, String> {
    validate_trace_bundle_unit_set(plan.units.len(), bundle)?;
    let mut outputs = Vec::with_capacity(plan.units.len());
    let mut source_lookup_balance = SourceLookupBalance::default();
    let shared_inputs = load_witness_shared_inputs(plan).map_err(|error| error.to_string())?;
    let auxiliary_inputs = Arc::new(auxiliary_inputs.clone());
    for unit_index in 0..plan.units.len() {
        let unit_index_u32 = u32::try_from(unit_index)
            .map_err(|_| format!("trace bundle unit index is too large: {unit_index}"))?;
        let trace_bytes = bundle
            .trace_bytes_for_unit(unit_index_u32)
            .ok_or_else(|| format!("trace bundle is missing unit {unit_index}"))?;
        let output = run_prove_witness_commitments_with_trace_bytes_inner(
            plan,
            unit_index,
            &shared_inputs,
            Arc::clone(&auxiliary_inputs),
            trace_bytes,
            WitnessRegularHintMode::Balanced(&mut source_lookup_balance),
        )
        .map_err(|error| {
            format!("run witness commitments failed for unit {unit_index}: {error}")
        })?;
        outputs.push(output);
    }
    let global_auxiliary_inputs =
        global_auxiliary_inputs_from_outputs(auxiliary_inputs.as_ref(), &outputs)
            .map_err(|error| error.to_string())?;
    accumulate_witness_global_hints(
        plan,
        &shared_inputs.publics,
        &global_auxiliary_inputs,
        &mut source_lookup_balance,
    )
    .map_err(|error| error.to_string())?;
    source_lookup_balance
        .validate_all_units()
        .map_err(|error| error.to_string())?;
    Ok(outputs)
}

fn validate_trace_bundle_unit_set(
    plan_unit_count: usize,
    bundle: &(impl TraceBundleSource + ?Sized),
) -> Result<(), String> {
    for unit_index in bundle.unit_indices() {
        let unit_index_usize = usize::try_from(unit_index)
            .map_err(|_| format!("trace bundle unit index is too large: {unit_index}"))?;
        if unit_index_usize >= plan_unit_count {
            return Err(format!("trace bundle has unexpected unit {unit_index}"));
        }
    }
    Ok(())
}

fn load_witness_shared_inputs(
    plan: &ProveExecutionPlan,
) -> Result<WitnessSharedInputs, ProveWitnessCommitmentError> {
    Ok(WitnessSharedInputs {
        input: read_witness_input(&plan.run_plan.pass)?,
        publics: load_public_inputs(plan)?,
    })
}

fn validate_witness_unit_index(
    plan: &ProveExecutionPlan,
    unit_index: usize,
) -> Result<(), ProveWitnessCommitmentError> {
    let unit_count = plan.run_plan.schedule.units.len();
    if unit_index >= unit_count {
        return Err(ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index,
            unit_count,
        });
    }
    Ok(())
}

fn load_public_inputs(plan: &ProveExecutionPlan) -> Result<Vec<Felt>, ProveWitnessCommitmentError> {
    let Some(path) = &plan.inputs.public_inputs else {
        return Ok(Vec::new());
    };
    let public_values = read_public_values_file(path).map_err(|source| {
        ProveWitnessCommitmentError::PublicInputs {
            path: path.clone(),
            source,
        }
    })?;
    if public_values.setup_hash != plan.run_plan.schedule.setup_hash {
        return Err(ProveWitnessCommitmentError::PublicInputsSetupHashMismatch);
    }
    public_values_to_fields(&public_values)
}

fn public_values_to_fields(
    public_values: &PublicValues,
) -> Result<Vec<Felt>, ProveWitnessCommitmentError> {
    public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied())
        .enumerate()
        .map(|(index, value)| {
            Felt::from_canonical(value).map_err(|error| match error {
                FieldError::NonCanonical { value } => {
                    ProveWitnessCommitmentError::PublicInputNonCanonical { index, value }
                }
            })
        })
        .collect()
}

fn accumulate_witness_global_hints(
    plan: &ProveExecutionPlan,
    publics: &[Felt],
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    source_lookup_balance: &mut SourceLookupBalance,
) -> Result<(), ProveWitnessCommitmentError> {
    if plan.global_hints.hints.is_empty() {
        return Ok(());
    }
    let resolved = resolve_global_hint_program(
        &plan.global_info,
        &plan.global_hints,
        GlobalConstraintInputs {
            publics,
            proof_values: &auxiliary_inputs.proof_values,
            challenges: &auxiliary_inputs.challenges,
            group_values: &auxiliary_inputs.group_values,
        },
    )
    .map_err(|source| ProveWitnessCommitmentError::GlobalHintEval { source })?;
    source_lookup_balance.absorb(0, 0, &resolved)?;
    Ok(())
}

fn validate_witness_regular_constraints<L>(
    plan_unit: &ProveExecutionUnitArtifacts,
    unit_index: usize,
    inputs: &mut WitnessRegularTraceInputs<'_, L>,
    proof_inputs: WitnessProofInputs<'_>,
    worker_count: usize,
) -> Result<(), ProveWitnessCommitmentError>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    if plan_unit.regular_constraints.entries.is_empty() {
        return Ok(());
    }

    let material = inputs
        .fixed_columns
        .get_or_load(unit_index, plan_unit, inputs.layout)?;

    #[cfg(feature = "cuda")]
    let fixed_columns_device_buffer = material.row_major_device_buffer();

    #[cfg(feature = "cuda")]
    if let Some(stage_source_devices) = inputs.stage_source_devices {
        let mut stage_columns = Vec::with_capacity(inputs.layout.stages().len());
        let mut all_stage_devices_available = true;
        for stage in inputs.layout.stages() {
            let stage_index = u16::try_from(stage.stage_index).map_err(|_| {
                ProveWitnessCommitmentError::StageIndexTooLarge {
                    unit_index,
                    stage_index: stage.stage_index,
                }
            })?;
            let Some((row_count, column_count, row_stride, column_offset, _, values_device)) =
                stage_source_devices.get_stage(stage.stage_index)
            else {
                all_stage_devices_available = false;
                break;
            };
            let value_count = row_count.checked_mul(column_count).ok_or(
                ProveWitnessCommitmentError::RegularConstraintEval(
                    RegularConstraintEvalError::LengthOverflow,
                ),
            )?;
            stage_columns.push(RegularStageColumns {
                stage_index,
                column_count,
                values: &[],
                values_device: Some(values_device),
                values_row_stride: row_stride,
                values_column_offset: column_offset,
                value_count,
            });
        }
        if all_stage_devices_available {
            let regular_constraint_inputs = RegularConstraintInputs {
                domain_size: inputs.layout.row_count(),
                stage_count: plan_unit.stage_count,
                fixed_columns: RegularColumnMatrix {
                    column_count: plan_unit.fixed_column_count,
                    values: &material.row_major_values,
                },
                stage_columns: &stage_columns,
                custom_fixed_columns: &[],
                opening_point_offsets: &plan_unit.opening_point_offsets,
                domain_points: &[],
                zerofier_values: RegularColumnMatrix {
                    column_count: 0,
                    values: &[],
                },
                publics: proof_inputs.publics,
                unit_values: &proof_inputs.auxiliary_inputs.unit_values,
                proof_values: &proof_inputs.auxiliary_inputs.proof_values,
                group_values: &proof_inputs.auxiliary_inputs.group_values,
                challenges: &proof_inputs.auxiliary_inputs.challenges,
                evaluations: &proof_inputs.auxiliary_inputs.evaluations,
            };
            if let Some(results) = try_evaluate_regular_constraints_cuda_base(
                &plan_unit.regular_constraints,
                regular_constraint_inputs,
                fixed_columns_device_buffer,
            )
            .map_err(|error| map_regular_constraint_eval_error(unit_index, error))?
            {
                for result in results {
                    if let Some(violation) = result.invalid_rows.first() {
                        return Err(ProveWitnessCommitmentError::RegularConstraintViolation {
                            unit_index,
                            constraint_index: result.constraint_index,
                            row: violation.row,
                            value: violation.value.to_u64s(),
                        });
                    }
                }
                return Ok(());
            }
        }
    }

    let domain_points =
        build_fri_domain_points(plan_unit.setup.stark.n_bits).map_err(|source| {
            ProveWitnessCommitmentError::RegularConstraintDomainHelper { unit_index, source }
        })?;
    let zerofiers = FriPolynomialZerofierTable::build(
        plan_unit.setup.stark.n_bits,
        plan_unit.setup.stark.n_bits,
        &plan_unit.setup.boundaries,
    )
    .map_err(
        |source| ProveWitnessCommitmentError::RegularConstraintDomainHelper { unit_index, source },
    )?;

    let stage_traces = inputs.stage_traces.get_or_extract_optional(
        inputs.layout,
        inputs.trace,
        "regular constraint CPU fallback",
    )?;
    let mut stage_columns = Vec::with_capacity(stage_traces.len());
    for stage in stage_traces {
        let stage_index = u16::try_from(stage.stage_index()).map_err(|_| {
            ProveWitnessCommitmentError::StageIndexTooLarge {
                unit_index,
                stage_index: stage.stage_index(),
            }
        })?;
        stage_columns.push(RegularStageColumns {
            stage_index,
            column_count: stage.column_count(),
            values: stage.values(),
            #[cfg(feature = "cuda")]
            values_device: inputs.stage_source_devices.and_then(|sources| {
                sources
                    .get(stage.stage_index())
                    .map(|(_, _, _, values_device)| values_device)
            }),
            #[cfg(feature = "cuda")]
            values_row_stride: inputs
                .stage_source_devices
                .and_then(|sources| {
                    sources
                        .get(stage.stage_index())
                        .map(|(_, row_stride, _, _)| row_stride)
                })
                .unwrap_or(stage.column_count()),
            #[cfg(feature = "cuda")]
            values_column_offset: inputs
                .stage_source_devices
                .and_then(|sources| {
                    sources
                        .get(stage.stage_index())
                        .map(|(_, _, column_offset, _)| column_offset)
                })
                .unwrap_or(0),
            #[cfg(feature = "cuda")]
            value_count: stage.values().len(),
        });
    }
    let regular_constraint_inputs = RegularConstraintInputs {
        domain_size: inputs.layout.row_count(),
        stage_count: plan_unit.stage_count,
        fixed_columns: RegularColumnMatrix {
            column_count: plan_unit.fixed_column_count,
            values: &material.row_major_values,
        },
        stage_columns: &stage_columns,
        custom_fixed_columns: &[],
        opening_point_offsets: &plan_unit.opening_point_offsets,
        domain_points: &domain_points,
        zerofier_values: RegularColumnMatrix {
            column_count: zerofiers.column_count,
            values: &zerofiers.values,
        },
        publics: proof_inputs.publics,
        unit_values: &proof_inputs.auxiliary_inputs.unit_values,
        proof_values: &proof_inputs.auxiliary_inputs.proof_values,
        group_values: &proof_inputs.auxiliary_inputs.group_values,
        challenges: &proof_inputs.auxiliary_inputs.challenges,
        evaluations: &proof_inputs.auxiliary_inputs.evaluations,
    };
    #[cfg(feature = "cuda")]
    let results = evaluate_regular_constraints_first_violations_with_cuda_fixed_values(
        &plan_unit.regular_constraints,
        regular_constraint_inputs,
        worker_count,
        fixed_columns_device_buffer,
    )
    .map_err(|error| map_regular_constraint_eval_error(unit_index, error))?;
    #[cfg(not(feature = "cuda"))]
    let results = evaluate_regular_constraints_first_violations_with_acceleration(
        &plan_unit.regular_constraints,
        regular_constraint_inputs,
        worker_count,
    )
    .map_err(|error| map_regular_constraint_eval_error(unit_index, error))?;

    for result in results {
        if let Some(violation) = result.invalid_rows.first() {
            return Err(ProveWitnessCommitmentError::RegularConstraintViolation {
                unit_index,
                constraint_index: result.constraint_index,
                row: violation.row,
                value: violation.value.to_u64s(),
            });
        }
    }
    Ok(())
}

fn map_regular_constraint_eval_error(
    unit_index: usize,
    error: RegularConstraintEvalError,
) -> ProveWitnessCommitmentError {
    match error {
        RegularConstraintEvalError::SourceIndexOutOfRange { buffer, len: 0, .. }
            if is_regular_constraint_input_buffer(buffer) =>
        {
            ProveWitnessCommitmentError::MissingRegularConstraintInput { unit_index, buffer }
        }
        error => ProveWitnessCommitmentError::RegularConstraintEval(error),
    }
}

fn is_regular_constraint_input_buffer(buffer: &str) -> bool {
    matches!(
        buffer,
        "public" | "unit value" | "proof value" | "group value" | "challenge" | "evaluation"
    )
}

#[cfg(test)]
fn validate_witness_regular_hints(
    plan_unit: &ProveExecutionUnitArtifacts,
    unit_index: usize,
    layout: &WitnessTraceLayout,
    trace: &WitnessTraceBuffer,
    publics: &[Felt],
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
) -> Result<(), ProveWitnessCommitmentError> {
    let mut source_lookup_balance = SourceLookupBalance::default();
    let mut fixed_columns = WitnessFixedColumnsCache::new();
    let mut stage_trace_cache = WitnessStageTraceCache::default();
    let proof_inputs = WitnessProofInputs {
        publics,
        auxiliary_inputs,
    };
    let mut regular_inputs = WitnessRegularTraceInputs {
        layout,
        trace: Some(trace),
        fixed_columns: WitnessFixedColumnsSource::Cache(&mut fixed_columns),
        stage_traces: &mut stage_trace_cache,
        #[cfg(feature = "cuda")]
        stage_source_devices: None,
    };
    accumulate_witness_regular_hints(
        plan_unit,
        unit_index,
        &mut regular_inputs,
        proof_inputs,
        &mut source_lookup_balance,
    )?;
    source_lookup_balance.validate(unit_index)?;
    Ok(())
}

fn accumulate_witness_regular_hints<L>(
    plan_unit: &ProveExecutionUnitArtifacts,
    unit_index: usize,
    inputs: &mut WitnessRegularTraceInputs<'_, L>,
    proof_inputs: WitnessProofInputs<'_>,
    source_lookup_balance: &mut SourceLookupBalance,
) -> Result<(), ProveWitnessCommitmentError>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    if plan_unit.regular_hints.hints.is_empty() {
        return Ok(());
    }
    reject_unsupported_regular_hints(&plan_unit.regular_hints, unit_index)?;

    accumulate_witness_regular_hint_program(
        plan_unit,
        unit_index,
        inputs,
        WitnessRegularHintProgramInputs {
            program: &plan_unit.regular_hints,
            proof_inputs,
        },
        source_lookup_balance,
    )
}

fn validate_witness_regular_source_assignments<L>(
    plan_unit: &ProveExecutionUnitArtifacts,
    unit_index: usize,
    inputs: &mut WitnessRegularTraceInputs<'_, L>,
    proof_inputs: WitnessProofInputs<'_>,
) -> Result<(), ProveWitnessCommitmentError>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    if plan_unit.regular_hints.hints.is_empty() {
        return Ok(());
    }
    reject_unsupported_regular_hints(&plan_unit.regular_hints, unit_index)?;
    let assignment_program = HintProgram {
        hints: plan_unit
            .regular_hints
            .hints
            .iter()
            .filter(|hint| hint.name == SOURCE_ASSIGNMENT_CHECK_HINT)
            .cloned()
            .collect(),
    };
    if assignment_program.hints.is_empty() {
        return Ok(());
    }
    let mut source_lookup_balance = SourceLookupBalance::default();
    accumulate_witness_regular_hint_program(
        plan_unit,
        unit_index,
        inputs,
        WitnessRegularHintProgramInputs {
            program: &assignment_program,
            proof_inputs,
        },
        &mut source_lookup_balance,
    )?;
    source_lookup_balance.validate(unit_index)?;
    Ok(())
}

fn accumulate_witness_regular_hint_program<L>(
    plan_unit: &ProveExecutionUnitArtifacts,
    unit_index: usize,
    trace_inputs: &mut WitnessRegularTraceInputs<'_, L>,
    hint_inputs: WitnessRegularHintProgramInputs<'_>,
    source_lookup_balance: &mut SourceLookupBalance,
) -> Result<(), ProveWitnessCommitmentError>
where
    L: FnMut(usize, &ProveExecutionUnitArtifacts) -> WitnessFixedColumnsLoadResult,
{
    let program = hint_inputs.program;
    let proof_inputs = hint_inputs.proof_inputs;
    if program.hints.is_empty() {
        return Ok(());
    }

    let requirements = regular_hint_input_requirements(program);

    let fixed_material = if requirements.fixed_columns {
        Some(
            trace_inputs
                .fixed_columns
                .get_or_load(unit_index, plan_unit, trace_inputs.layout)?,
        )
    } else {
        None
    };

    let stage_traces: &[WitnessTraceStageValues] = if requirements.stage_columns {
        trace_inputs.stage_traces.get_or_extract_optional(
            trace_inputs.layout,
            trace_inputs.trace,
            "regular hint stage column input",
        )?
    } else {
        &[]
    };
    let mut stage_columns = Vec::with_capacity(stage_traces.len());
    for stage in stage_traces {
        let stage_index = u16::try_from(stage.stage_index()).map_err(|_| {
            ProveWitnessCommitmentError::StageIndexTooLarge {
                unit_index,
                stage_index: stage.stage_index(),
            }
        })?;
        stage_columns.push(RegularStageColumns {
            stage_index,
            column_count: stage.column_count(),
            values: stage.values(),
            #[cfg(feature = "cuda")]
            values_device: None,
            #[cfg(feature = "cuda")]
            values_row_stride: stage.column_count(),
            #[cfg(feature = "cuda")]
            values_column_offset: 0,
            #[cfg(feature = "cuda")]
            value_count: stage.values().len(),
        });
    }

    let fixed_columns =
        fixed_material
            .as_ref()
            .map_or_else(RegularColumnMatrix::default, |material| {
                RegularColumnMatrix {
                    column_count: plan_unit.fixed_column_count,
                    values: &material.row_major_values,
                }
            });

    for row in 0..trace_inputs.layout.row_count() {
        let resolved = resolve_regular_hint_program_for_row(
            &plan_unit.setup,
            program,
            row,
            RegularConstraintInputs {
                domain_size: trace_inputs.layout.row_count(),
                stage_count: plan_unit.stage_count,
                fixed_columns,
                stage_columns: &stage_columns,
                custom_fixed_columns: &[],
                opening_point_offsets: &plan_unit.opening_point_offsets,
                domain_points: &[],
                zerofier_values: RegularColumnMatrix::default(),
                publics: proof_inputs.publics,
                unit_values: &proof_inputs.auxiliary_inputs.unit_values,
                proof_values: &proof_inputs.auxiliary_inputs.proof_values,
                group_values: &proof_inputs.auxiliary_inputs.group_values,
                challenges: &proof_inputs.auxiliary_inputs.challenges,
                evaluations: &proof_inputs.auxiliary_inputs.evaluations,
            },
        )
        .map_err(|error| map_regular_hint_eval_error(unit_index, error))?;
        validate_source_assignment_hints(unit_index, row, &resolved)?;
        source_lookup_balance.absorb(unit_index, row, &resolved)?;
    }
    Ok(())
}

fn reject_unsupported_regular_hints(
    program: &HintProgram,
    unit_index: usize,
) -> Result<(), ProveWitnessCommitmentError> {
    if let Some(hint) = program
        .hints
        .iter()
        .find(|hint| source_unimplemented_hint_name(&hint.name))
    {
        return Err(ProveWitnessCommitmentError::UnsupportedRegularHint {
            unit_index,
            name: hint.name.clone(),
        });
    }
    Ok(())
}

fn map_regular_hint_eval_error(
    unit_index: usize,
    error: HintEvalError,
) -> ProveWitnessCommitmentError {
    match error {
        HintEvalError::SourceIndexOutOfRange { source, len: 0, .. }
            if is_regular_hint_input_source(source) =>
        {
            ProveWitnessCommitmentError::MissingRegularHintInput { unit_index, source }
        }
        source => ProveWitnessCommitmentError::RegularHintEval { unit_index, source },
    }
}

fn is_regular_hint_input_source(source: &str) -> bool {
    matches!(
        source,
        "public" | "unit value" | "proof value" | "unit group value" | "challenge" | "evaluation"
    )
}

fn load_witness_fixed_columns_material(
    unit_index: usize,
    plan_unit: &ProveExecutionUnitArtifacts,
) -> Result<crate::FixedColumnsMaterial, ProveWitnessCommitmentError> {
    load_plan_fixed_columns_material(plan_unit).map_err(|source| {
        ProveWitnessCommitmentError::FixedColumns {
            unit_index,
            path: plan_unit.fixed_columns.clone(),
            source: Box::new(source),
        }
    })
}

fn load_plan_fixed_columns_material(
    plan_unit: &ProveExecutionUnitArtifacts,
) -> Result<crate::FixedColumnsMaterial, FixedColumnsMaterialError> {
    load_execution_unit_fixed_columns_material(plan_unit)
}

fn validate_fixed_columns_shape(
    fixed_columns: &FixedColumns,
    fixed_column_count: usize,
    row_count: usize,
    unit_index: usize,
    path: &Path,
) -> Result<(), ProveWitnessCommitmentError> {
    let found_rows = usize::try_from(fixed_columns.row_count).map_err(|_| {
        ProveWitnessCommitmentError::FixedRowCountTooLarge {
            unit_index,
            path: path.to_path_buf(),
            rows: fixed_columns.row_count,
        }
    })?;
    if found_rows != row_count {
        return Err(ProveWitnessCommitmentError::FixedRowCountMismatch {
            unit_index,
            path: path.to_path_buf(),
            expected: row_count,
            found: found_rows,
        });
    }
    if fixed_columns.columns.len() != fixed_column_count {
        return Err(ProveWitnessCommitmentError::FixedColumnCountMismatch {
            unit_index,
            path: path.to_path_buf(),
            expected: fixed_column_count,
            found: fixed_columns.columns.len(),
        });
    }
    Ok(())
}

fn read_witness_input(pass: &ProvePassRequest) -> Result<Vec<u8>, ProveWitnessCommitmentError> {
    match witness_input_path(pass) {
        Some(path) => std::fs::read(path).map_err(|error| ProveWitnessCommitmentError::InputData {
            path: path.to_path_buf(),
            message: error.to_string(),
        }),
        None => Ok(Vec::new()),
    }
}

fn witness_input_path(pass: &ProvePassRequest) -> Option<&Path> {
    match pass {
        ProvePassRequest::Contributions(partition) | ProvePassRequest::Full(partition) => {
            partition.input_data.as_deref()
        }
        ProvePassRequest::Internal { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::witness_layout::{
        derive_witness_trace_layout, reset_stage_trace_count, stage_trace_count,
    };
    use crate::witness_trace::parse_witness_trace;
    use lzvm_artifacts::constraint_program::{ConstraintEntry, ConstraintProgram};
    use lzvm_artifacts::fixed::{write_raw_fixed_columns_file, FixedColumn};
    use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
    use lzvm_artifacts::guest_image::{ElfClass, ElfEndian, GuestImageInfo};
    use lzvm_artifacts::hint_program::{
        Hint, HintField, HintOperand, HintValue, SOURCE_ASSIGNMENT_CHECK_HINT,
        SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT, SOURCE_UNSUPPORTED_ASSIGNMENT_HINT,
        SOURCE_UNSUPPORTED_CALL_HINT, SOURCE_UNSUPPORTED_CONSTRAINT_HINT,
        SOURCE_UNSUPPORTED_STATEMENT_HINT,
    };
    use lzvm_artifacts::key_directory::KeyUnitKind;
    use lzvm_artifacts::setup_info::{
        CommitmentColumn, ConstantColumn, FriStep, StarkStruct, UnitSetupInfo,
    };
    use lzvm_artifacts::trace_bundle::{TraceBundle, TraceBundleUnit};
    use sha2::{Digest, Sha256};
    use std::cell::Cell;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn rejects_trace_bundles_with_unexpected_units() {
        let bundle = TraceBundle {
            units: vec![
                TraceBundleUnit {
                    unit_index: 0,
                    trace_bytes: vec![1],
                },
                TraceBundleUnit {
                    unit_index: 2,
                    trace_bytes: vec![2],
                },
            ],
        };

        let error = validate_trace_bundle_unit_set(2, &bundle)
            .expect_err("trace bundle should not carry units outside the plan");

        assert_eq!(error, "trace bundle has unexpected unit 2");
    }

    #[test]
    fn reuses_loaded_fixed_column_material_for_unit() {
        let loads = Cell::new(0);
        let plan_unit = source_lookup_plan_unit(HintProgram { hints: Vec::new() });
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let mut cache = WitnessFixedColumnsCache::with_loader(|unit_index, _| {
            assert_eq!(unit_index, 0);
            loads.set(loads.get() + 1);
            Ok(empty_fixed_columns_material(
                u64::try_from(layout.row_count()).expect("row count should fit u64"),
            ))
        });

        assert_eq!(
            cache
                .get_or_load(0, &plan_unit, &layout)
                .expect("material should load")
                .row_major_values
                .len(),
            0
        );
        assert_eq!(
            cache
                .get_or_load(0, &plan_unit, &layout)
                .expect("material should be reused")
                .row_major_values
                .len(),
            0
        );
        assert_eq!(loads.get(), 1);
    }

    fn dummy_trace_commitment_output(trace_instance_index: u32) -> ProveWitnessTraceCommitments {
        ProveWitnessTraceCommitments {
            commitments: ProveWitnessCommitments {
                identity: ProveTraceIdentity::new(0, trace_instance_index),
                input_byte_count: 0,
                trace_rows: 0,
                trace_columns: 0,
                stage_commitments: WitnessTraceCommitments::new(Vec::new()),
            },
            trace: None,
            trace_constraint_checks: ProveWitnessTraceConstraintChecks {
                regular_constraint_count: 0,
                trace_extracted: true,
                regular_constraints_evaluated: true,
                witness_values_committed: true,
                constraint_checker_conformant: true,
            },
            #[cfg(feature = "cuda")]
            stage_source_devices: Vec::new(),
            #[cfg(feature = "cuda")]
            guest_pc_device_descriptor_buffer: None,
            #[cfg(feature = "cuda")]
            guest_pc_device_segment_material: None,
            publics: Vec::new(),
            auxiliary_inputs: Arc::new(ProveWitnessAuxiliaryInputs::default()),
        }
    }

    #[test]
    fn guest_pc_segment_commit_output_collector_drains_trace_instances_in_order() {
        let mut collector = GuestPcTraceSegmentCommitOutputCollector::new();

        collector
            .collect_committed_segment(dummy_trace_commitment_output(1))
            .expect("first out-of-order output should collect");
        assert!(collector.outputs.is_empty());

        collector
            .collect_committed_segment(dummy_trace_commitment_output(0))
            .expect("missing first output should drain pending outputs");
        collector
            .collect_committed_segment(dummy_trace_commitment_output(2))
            .expect("following output should collect");

        let output = collector
            .finish()
            .expect("collector should finish without pending gaps");
        let trace_instances: Vec<_> = output
            .into_iter()
            .map(|output| output.commitments().trace_instance_index())
            .collect();
        assert_eq!(trace_instances, [0, 1, 2]);
    }

    #[test]
    fn guest_pc_segment_commit_worker_pool_finish_drains_after_error() {
        thread::scope(|scope| {
            let first = scope.spawn(
                || -> Result<GuestPcTraceSegmentCommitResult, ProveWitnessCommitmentError> {
                    Err(ProveWitnessCommitmentError::SegmentCommitOutputOrder {
                        message: "first worker failed".to_owned(),
                    })
                },
            );
            let second = scope.spawn(
                || -> Result<GuestPcTraceSegmentCommitResult, ProveWitnessCommitmentError> {
                    Err(ProveWitnessCommitmentError::SegmentCommitOutputOrder {
                        message: "second worker failed".to_owned(),
                    })
                },
            );
            let mut pool = GuestPcTraceSegmentCommitWorkerPool {
                scope,
                worker_count: 2,
                worker_state: GuestPcTraceSegmentCommitWorkerState::new(),
                pending_workers: VecDeque::from([first, second]),
            };

            let error = match pool.finish() {
                Ok(_) => panic!("pool finish should report worker errors"),
                Err(error) => error,
            };

            assert!(matches!(
                error,
                ProveWitnessCommitmentError::SegmentCommitOutputOrder { .. }
            ));
            assert!(
                pool.pending_workers.is_empty(),
                "pool finish should drain every pending worker before returning an error"
            );
        });
    }

    #[test]
    fn guest_pc_segment_commit_worker_count_uses_conservative_large_default() {
        assert_eq!(
            default_guest_pc_trace_segment_commit_worker_count_for_input(0),
            1
        );
        assert_eq!(
            default_guest_pc_trace_segment_commit_worker_count_for_input(8 * 1024 * 1024),
            1
        );
        assert_eq!(
            default_guest_pc_trace_segment_commit_worker_count_for_input(usize::MAX),
            1
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_out_of_memory_commit_error_is_retryable() {
        let oom_error = ProveWitnessCommitmentError::Commit(
            WitnessTraceCommitmentError::StageCommitment(WitnessStageCommitmentError::Leaf(
                WitnessStageLeafError::Accel(lzvm_accel::AccelError::Cuda { code: 2 }),
            )),
        );
        let other_cuda_error = ProveWitnessCommitmentError::Commit(
            WitnessTraceCommitmentError::StageCommitment(WitnessStageCommitmentError::Leaf(
                WitnessStageLeafError::Accel(lzvm_accel::AccelError::Cuda { code: 700 }),
            )),
        );
        let layout_error = ProveWitnessCommitmentError::SegmentCommitOutputOrder {
            message: "not a CUDA allocation failure".to_owned(),
        };

        assert!(prove_witness_commitment_error_is_cuda_out_of_memory(
            &oom_error
        ));
        assert!(!prove_witness_commitment_error_is_cuda_out_of_memory(
            &other_cuda_error
        ));
        assert!(!prove_witness_commitment_error_is_cuda_out_of_memory(
            &layout_error
        ));
        assert!(should_retry_guest_pc_segment_commit_with_serial_worker(
            8 * 1024 * 1024,
            Some(3),
            &oom_error
        ));
        assert!(!should_retry_guest_pc_segment_commit_with_serial_worker(
            8 * 1024 * 1024,
            Some(1),
            &oom_error
        ));
        assert!(!should_retry_guest_pc_segment_commit_with_serial_worker(
            8 * 1024 * 1024,
            Some(3),
            &other_cuda_error
        ));
    }

    #[test]
    fn guest_pc_segment_commit_worker_count_can_be_forced_serial() {
        assert_eq!(
            guest_pc_trace_segment_commit_worker_count_for_input_with_override(
                8 * 1024 * 1024,
                Some(1)
            ),
            1
        );
    }

    #[test]
    fn shares_fixed_column_material_between_regular_checks() {
        let loads = Cell::new(0);
        let plan_unit = fixed_lookup_plan_unit();
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);
        let mut cache = WitnessFixedColumnsCache::with_loader(|unit_index, _| {
            assert_eq!(unit_index, 0);
            loads.set(loads.get() + 1);
            Ok(single_fixed_columns_material(&[3, 5]))
        });
        let mut stage_trace_cache = WitnessStageTraceCache::default();
        let auxiliary_inputs = ProveWitnessAuxiliaryInputs::default();
        let proof_inputs = WitnessProofInputs {
            publics: &[],
            auxiliary_inputs: &auxiliary_inputs,
        };
        let mut regular_inputs = WitnessRegularTraceInputs {
            layout: &layout,
            trace: Some(&trace),
            fixed_columns: WitnessFixedColumnsSource::Cache(&mut cache),
            stage_traces: &mut stage_trace_cache,
            #[cfg(feature = "cuda")]
            stage_source_devices: None,
        };

        validate_witness_regular_constraints(&plan_unit, 0, &mut regular_inputs, proof_inputs, 1)
            .expect("constraint check should validate");

        let mut source_lookup_balance = SourceLookupBalance::default();
        accumulate_witness_regular_hints(
            &plan_unit,
            0,
            &mut regular_inputs,
            proof_inputs,
            &mut source_lookup_balance,
        )
        .expect("regular hints should accumulate");

        assert_eq!(loads.get(), 1);
    }

    #[test]
    fn regular_checks_accept_preloaded_fixed_column_material() {
        let plan_unit = fixed_lookup_plan_unit();
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);
        let material = single_fixed_columns_material(&[3, 5]);
        let mut stage_trace_cache = WitnessStageTraceCache::default();
        let auxiliary_inputs = ProveWitnessAuxiliaryInputs::default();
        let proof_inputs = WitnessProofInputs {
            publics: &[],
            auxiliary_inputs: &auxiliary_inputs,
        };
        let mut regular_inputs: WitnessRegularTraceInputs<'_, WitnessFixedColumnsLoader> =
            WitnessRegularTraceInputs {
                layout: &layout,
                trace: Some(&trace),
                fixed_columns: WitnessFixedColumnsSource::Material(&material),
                stage_traces: &mut stage_trace_cache,
                #[cfg(feature = "cuda")]
                stage_source_devices: None,
            };

        validate_witness_regular_constraints(&plan_unit, 0, &mut regular_inputs, proof_inputs, 1)
            .expect("constraint check should validate from preloaded fixed material");

        let mut source_lookup_balance = SourceLookupBalance::default();
        accumulate_witness_regular_hints(
            &plan_unit,
            0,
            &mut regular_inputs,
            proof_inputs,
            &mut source_lookup_balance,
        )
        .expect("regular hints should use preloaded fixed material");
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_regular_constraints_skip_domain_helper_setup_on_base_fast_path() {
        let mut plan_unit = source_lookup_plan_unit(HintProgram { hints: Vec::new() });
        plan_unit.regular_constraints = zero_constraint_program();
        plan_unit.setup.stark.n_bits = 64;
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[0, 0, 0, 0]);
        let mut stage_source_devices = WitnessStageSourceDeviceCache::default();
        stage_source_devices
            .upload_from_trace_if_empty(&layout, &trace)
            .expect("stage source should upload");
        let mut fixed_columns = WitnessFixedColumnsCache::with_loader(|unit_index, _| {
            assert_eq!(unit_index, 0);
            Ok(empty_fixed_columns_material(
                u64::try_from(layout.row_count()).expect("row count should fit u64"),
            ))
        });
        let mut stage_trace_cache = WitnessStageTraceCache::default();
        let auxiliary_inputs = ProveWitnessAuxiliaryInputs::default();
        let proof_inputs = WitnessProofInputs {
            publics: &[],
            auxiliary_inputs: &auxiliary_inputs,
        };
        let mut regular_inputs = WitnessRegularTraceInputs {
            layout: &layout,
            trace: None,
            fixed_columns: WitnessFixedColumnsSource::Cache(&mut fixed_columns),
            stage_traces: &mut stage_trace_cache,
            stage_source_devices: Some(&stage_source_devices),
        };

        validate_witness_regular_constraints(&plan_unit, 0, &mut regular_inputs, proof_inputs, 1)
            .expect("CUDA base regular checks should not require domain helpers");
    }

    #[test]
    fn materializes_each_stage_once_for_regular_checks_and_commitments() {
        let dir = temp_dir("stage-trace-materialization");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("fixture directory should be created");
        let mut plan = source_lookup_global_plan(HintProgram { hints: Vec::new() });
        let mut plan_unit = source_lookup_plan_unit(HintProgram {
            hints: vec![source_assignment_hint(
                HintOperand::Commitment {
                    id: 0,
                    row_offset_index: 0,
                },
                HintOperand::Commitment {
                    id: 0,
                    row_offset_index: 0,
                },
            )],
        });
        plan_unit.regular_constraints = zero_constraint_program();
        plan_unit.fixed_columns = dir.join("unit.const");
        write_raw_fixed_columns_file(
            &plan_unit.fixed_columns,
            &empty_fixed_columns_material(2).fixed_columns,
            &plan_unit.setup,
        )
        .expect("empty fixed columns should write");
        plan.run_plan.gpu.witness_thread_pools = 1;
        plan.units[0] = plan_unit;
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let stage_count = layout.stage_count();
        let trace = source_lookup_trace(&[7, 1, 8, 2]);
        let shared_inputs = WitnessSharedInputs {
            input: vec![7],
            publics: Vec::new(),
        };

        reset_stage_trace_count();
        let output = run_prove_witness_commitments_from_trace_inner(
            &plan,
            0,
            &shared_inputs,
            Arc::new(ProveWitnessAuxiliaryInputs::default()),
            WitnessTraceCommitmentInput {
                unit: &plan.run_plan.schedule.units[0],
                layout,
                trace: Some(trace),
                #[cfg(feature = "cuda")]
                terminal_trace_source_prefix_rows: None,
                #[cfg(feature = "cuda")]
                stage_source_devices: None,
                #[cfg(feature = "cuda")]
                guest_pc_device_segment_material: None,
            },
            WitnessRegularHintMode::AssignmentsOnly,
            ProveWitnessTraceRunObservers {
                fixed_columns_cache: None,
                #[cfg(feature = "cuda")]
                stage_commitment_reuse_cache: None,
                #[cfg(feature = "cuda")]
                leaf_workspace_cache: None,
                timing: None,
            },
        )
        .expect("trace commitments should prove");

        assert_eq!(
            output.commitments().stage_commitments().stage_count(),
            stage_count
        );
        assert_eq!(stage_trace_count(), stage_count);
        fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    }

    #[test]
    fn load_witness_fixed_columns_rejects_pcs_digest_mismatch() {
        let dir = temp_dir("witness-fixed-digest-mismatch");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("fixture directory should be created");
        let path = dir.join("unit.const");
        let fixed_columns = FixedColumns {
            group_name: "group".to_owned(),
            unit_name: "unit".to_owned(),
            row_count: 2,
            columns: vec![FixedColumn {
                name: "constant".to_owned(),
                dimensions: vec![1],
                values: vec![3, 5],
            }],
        };
        let mut plan_unit = fixed_lookup_plan_unit();
        plan_unit.fixed_columns = path.clone();
        write_raw_fixed_columns_file(&path, &fixed_columns, &plan_unit.setup)
            .expect("fixed columns should write");
        let expected_digest: [u8; 32] =
            Sha256::digest(fs::read(&path).expect("fixed file should read")).into();
        plan_unit.pcs_material_fixed_column_digest = Some(expected_digest);
        let mut mutated = fs::read(&path).expect("fixed file should read");
        mutated[0] ^= 1;
        fs::write(&path, mutated).expect("fixed file should mutate");

        let error = load_witness_fixed_columns_material(0, &plan_unit)
            .expect_err("fixed digest mismatch should reject witness material");

        let ProveWitnessCommitmentError::FixedColumns { source, .. } = error else {
            panic!("expected fixed columns error");
        };
        assert!(matches!(
            *source,
            FixedColumnsMaterialError::DigestMismatch { .. }
        ));
        fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    }

    #[test]
    fn accepts_source_lookup_regular_hints_at_unsupported_gate() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_LOOKUP_PROVES_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("lookup_proves(7, [value])".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        reject_unsupported_regular_hints(&program, 3)
            .expect("source lookup hints should reach semantic validation");
    }

    #[test]
    fn ignores_line_only_source_lookup_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_LOOKUP_PROVES_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("lookup_proves(7, [value])".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect("line-only lookup hints should be ignored by balance validation");
    }

    #[test]
    fn accepts_balanced_source_lookup_regular_hints() {
        let program = HintProgram {
            hints: vec![
                source_lookup_hint(
                    SOURCE_LOOKUP_PROVES_HINT,
                    "multiplicity",
                    HintOperand::Commitment {
                        id: 1,
                        row_offset_index: 0,
                    },
                ),
                source_lookup_hint(
                    SOURCE_LOOKUP_ASSUMES_HINT,
                    "selector",
                    HintOperand::Commitment {
                        id: 1,
                        row_offset_index: 0,
                    },
                ),
            ],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect("balanced lookup hints should validate");
    }

    #[test]
    fn accepts_balanced_source_lookup_weight_expressions() {
        let program = HintProgram {
            hints: vec![
                source_lookup_hint_with_weight_values(
                    SOURCE_LOOKUP_PROVES_HINT,
                    "multiplicity",
                    vec![
                        HintOperand::Commitment {
                            id: 1,
                            row_offset_index: 0,
                        },
                        HintOperand::Commitment {
                            id: 1,
                            row_offset_index: 0,
                        },
                        HintOperand::String("add".to_owned()),
                    ],
                ),
                source_lookup_hint_with_weight_values(
                    SOURCE_LOOKUP_ASSUMES_HINT,
                    "selector",
                    vec![
                        HintOperand::Number(2),
                        HintOperand::Commitment {
                            id: 1,
                            row_offset_index: 0,
                        },
                        HintOperand::String("mul".to_owned()),
                    ],
                ),
            ],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect("balanced lookup weight expressions should validate");
    }

    #[test]
    fn rejects_mismatched_source_assignment_regular_hints() {
        let program = HintProgram {
            hints: vec![source_assignment_hint(
                HintOperand::Commitment {
                    id: 0,
                    row_offset_index: 0,
                },
                HintOperand::Commitment {
                    id: 1,
                    row_offset_index: 0,
                },
            )],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        let error = validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect_err("mismatched assignment hint should reject");

        assert!(error.to_string().contains("source assignment"));
    }

    #[test]
    fn rejects_unbalanced_source_lookup_regular_hints() {
        let program = HintProgram {
            hints: vec![
                source_lookup_hint(
                    SOURCE_LOOKUP_PROVES_HINT,
                    "multiplicity",
                    HintOperand::Commitment {
                        id: 1,
                        row_offset_index: 0,
                    },
                ),
                source_lookup_hint(
                    SOURCE_LOOKUP_ASSUMES_HINT,
                    "selector",
                    HintOperand::Number(1),
                ),
            ],
        };
        let plan_unit = source_lookup_plan_unit(program);
        let schedule = source_lookup_schedule();
        let layout = derive_witness_trace_layout(&schedule).expect("layout should derive");
        let trace = source_lookup_trace(&[7, 1, 8, 2]);

        let error = validate_witness_regular_hints(
            &plan_unit,
            0,
            &layout,
            &trace,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
        )
        .expect_err("unbalanced lookup hints should reject");

        assert!(matches!(
            error,
            ProveWitnessCommitmentError::SourceLookup { unit_index: 0, .. }
        ));
    }

    #[test]
    fn accepts_balanced_source_lookup_global_hints() {
        let plan = source_lookup_global_plan(HintProgram {
            hints: vec![
                source_lookup_global_hint(SOURCE_LOOKUP_PROVES_HINT),
                source_lookup_global_hint(SOURCE_LOOKUP_ASSUMES_HINT),
            ],
        });
        let mut balance = SourceLookupBalance::default();

        accumulate_witness_global_hints(
            &plan,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
            &mut balance,
        )
        .expect("balanced global lookup hints should accumulate");

        balance
            .validate_all_units()
            .expect("balanced global lookup hints should validate");
    }

    #[test]
    fn rejects_unbalanced_source_lookup_global_hints() {
        let plan = source_lookup_global_plan(HintProgram {
            hints: vec![source_lookup_global_hint(SOURCE_LOOKUP_PROVES_HINT)],
        });
        let mut balance = SourceLookupBalance::default();

        accumulate_witness_global_hints(
            &plan,
            &[],
            &ProveWitnessAuxiliaryInputs::default(),
            &mut balance,
        )
        .expect("global lookup hint should accumulate");

        let error = ProveWitnessCommitmentError::from(
            balance
                .validate_all_units()
                .expect_err("unbalanced global lookup hints should reject"),
        );

        assert!(matches!(
            error,
            ProveWitnessCommitmentError::SourceLookupSet { .. }
        ));
    }

    #[test]
    fn rejects_unsupported_source_call_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_UNSUPPORTED_CALL_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("source_protocol_call()".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        let error = reject_unsupported_regular_hints(&program, 5)
            .expect_err("unsupported source call hints should be rejected before evaluation");

        assert_eq!(
            error,
            ProveWitnessCommitmentError::UnsupportedRegularHint {
                unit_index: 5,
                name: SOURCE_UNSUPPORTED_CALL_HINT.to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unsupported_source_assignment_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_UNSUPPORTED_ASSIGNMENT_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("out[0] = value + 1".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        let error = reject_unsupported_regular_hints(&program, 7)
            .expect_err("unsupported source assignment hints should reject");

        assert_eq!(
            error,
            ProveWitnessCommitmentError::UnsupportedRegularHint {
                unit_index: 7,
                name: SOURCE_UNSUPPORTED_ASSIGNMENT_HINT.to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unsupported_source_statement_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_UNSUPPORTED_STATEMENT_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("for (...) { }".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        let error = reject_unsupported_regular_hints(&program, 9)
            .expect_err("unsupported source statement hints should be rejected before evaluation");

        assert_eq!(
            error,
            ProveWitnessCommitmentError::UnsupportedRegularHint {
                unit_index: 9,
                name: SOURCE_UNSUPPORTED_STATEMENT_HINT.to_owned(),
            }
        );
    }

    #[test]
    fn rejects_unsupported_source_constraint_regular_hints() {
        let program = HintProgram {
            hints: vec![Hint {
                name: SOURCE_UNSUPPORTED_CONSTRAINT_HINT.to_owned(),
                fields: vec![HintField {
                    name: "line".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::String("value * (value - delayed) === 0".to_owned()),
                        positions: Vec::new(),
                    }],
                }],
            }],
        };

        let error = reject_unsupported_regular_hints(&program, 11)
            .expect_err("unsupported source constraint hints should be rejected before evaluation");

        assert_eq!(
            error,
            ProveWitnessCommitmentError::UnsupportedRegularHint {
                unit_index: 11,
                name: SOURCE_UNSUPPORTED_CONSTRAINT_HINT.to_owned(),
            }
        );
    }

    fn source_lookup_hint(name: &str, weight_field: &str, weight_operand: HintOperand) -> Hint {
        source_lookup_hint_with_weight_values(name, weight_field, vec![weight_operand])
    }

    fn source_lookup_hint_with_weight_values(
        name: &str,
        weight_field: &str,
        weight_operands: Vec<HintOperand>,
    ) -> Hint {
        Hint {
            name: name.to_owned(),
            fields: vec![
                HintField {
                    name: "bus_id".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Number(7),
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: "values".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Commitment {
                            id: 0,
                            row_offset_index: 0,
                        },
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: weight_field.to_owned(),
                    values: weight_operands
                        .into_iter()
                        .map(|operand| HintValue {
                            operand,
                            positions: Vec::new(),
                        })
                        .collect(),
                },
            ],
        }
    }

    fn source_assignment_hint(target: HintOperand, value: HintOperand) -> Hint {
        Hint {
            name: SOURCE_ASSIGNMENT_CHECK_HINT.to_owned(),
            fields: vec![
                HintField {
                    name: "target".to_owned(),
                    values: vec![HintValue {
                        operand: target,
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: "value".to_owned(),
                    values: vec![HintValue {
                        operand: value,
                        positions: Vec::new(),
                    }],
                },
            ],
        }
    }

    fn source_lookup_global_hint(name: &str) -> Hint {
        Hint {
            name: name.to_owned(),
            fields: vec![
                HintField {
                    name: "bus_id".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Number(7),
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: "values".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Number(11),
                        positions: Vec::new(),
                    }],
                },
            ],
        }
    }

    fn source_lookup_constant_hint() -> Hint {
        Hint {
            name: SOURCE_LOOKUP_PROVES_HINT.to_owned(),
            fields: vec![
                HintField {
                    name: "bus_id".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Number(7),
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: "values".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Constant {
                            id: 0,
                            row_offset_index: 0,
                        },
                        positions: Vec::new(),
                    }],
                },
                HintField {
                    name: "multiplicity".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Number(1),
                        positions: Vec::new(),
                    }],
                },
            ],
        }
    }

    fn source_lookup_global_plan(global_hints: HintProgram) -> ProveExecutionPlan {
        ProveExecutionPlan {
            run_plan: crate::ProveRunPlan {
                schedule: crate::ProveSchedule {
                    setup_hash: [0; 32],
                    unit_count: 1,
                    total_fixed_bytes: 0,
                    total_pcs_material_bytes: 0,
                    pcs_material_unit_count: 0,
                    total_query_count: 0,
                    max_extended_domain_bits: 0,
                    units: vec![source_lookup_schedule()],
                },
                pass: ProvePassRequest::Full(crate::ProvePartitionPlan::single()),
                options: crate::ProveRunOptions::default_for_output(PathBuf::from("out")),
                gpu: crate::GpuRunOptions::default(),
            },
            inputs: crate::ProveExecutionInputArtifacts {
                witness_library: None,
                guest_image: PathBuf::from("guest.elf"),
                public_inputs: None,
            },
            global_info: GlobalInfo {
                name: "program".to_owned(),
                air_groups: Vec::new(),
                airs: Vec::new(),
                curve: CurveKind::None,
                lattice_size: None,
                aggregation_types: Vec::new(),
                n_publics: 0,
                num_challenges: Vec::new(),
                num_proof_values: Vec::new(),
                proof_values_map: Vec::new(),
                publics_map: Vec::new(),
                transcript_arity: 4,
            },
            global_hints,
            witness_library_info: None,
            guest_image_info: GuestImageInfo {
                byte_len: 0,
                digest: [0; 32],
                elf_class: ElfClass::Elf64,
                endian: ElfEndian::Little,
                machine: 0,
                entry: 0,
                load_segments: Vec::new(),
            },
            program_image_cache: None,
            units: vec![source_lookup_plan_unit(HintProgram { hints: Vec::new() })],
        }
    }

    fn source_lookup_plan_unit(program: HintProgram) -> ProveExecutionUnitArtifacts {
        ProveExecutionUnitArtifacts {
            fixed_columns: PathBuf::from("fixed.bin"),
            pcs_material_fixed_column_digest: None,
            expression_program: lzvm_artifacts::expression_program::ExpressionProgram {
                max_tmp1: 0,
                max_tmp3: 0,
                max_args: 0,
                max_ops: 0,
                entries: Vec::new(),
                ops: Vec::new(),
                args: Vec::new(),
                numbers: Vec::new(),
            },
            fri_expression_id: None,
            regular_constraints: lzvm_artifacts::constraint_program::ConstraintProgram {
                entries: Vec::new(),
                ops: Vec::new(),
                args: Vec::new(),
                numbers: Vec::new(),
            },
            regular_hints: program,
            setup: source_lookup_setup(),
            fixed_column_count: 0,
            stage_count: 1,
            opening_point_offsets: vec![0],
            group_name: "group".to_owned(),
            unit_name: "unit".to_owned(),
        }
    }

    fn fixed_lookup_plan_unit() -> ProveExecutionUnitArtifacts {
        let mut unit = source_lookup_plan_unit(HintProgram {
            hints: vec![source_lookup_constant_hint()],
        });
        unit.fixed_column_count = 1;
        unit.regular_constraints = zero_constraint_program();
        unit.setup.n_constants = 1;
        unit.setup.constant_columns = vec![ConstantColumn {
            name: "constant".to_owned(),
            stage: 0,
            dimension: 1,
            pols_map_id: 0,
            stage_id: 0,
            lengths: Vec::new(),
        }];
        unit
    }

    fn zero_constraint_program() -> ConstraintProgram {
        ConstraintProgram {
            entries: vec![ConstraintEntry {
                stage: 1,
                destination_dimension: 1,
                destination_id: 0,
                first_row: 0,
                last_row: 2,
                temp1_count: 1,
                temp3_count: 0,
                ops_count: 0,
                ops_offset: 0,
                args_count: 0,
                args_offset: 0,
                intermediate: false,
                source_line: "0".to_owned(),
            }],
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        }
    }

    fn source_lookup_setup() -> UnitSetupInfo {
        UnitSetupInfo {
            n_stages: 1,
            n_constants: 0,
            constant_columns: Vec::new(),
            n_publics: Some(0),
            n_constraints: Some(0),
            q_degree: 3,
            opening_points: vec![0],
            section_widths: std::collections::BTreeMap::new(),
            challenge_count: 0,
            eval_count: 0,
            evaluation_map: Vec::new(),
            boundaries: Vec::new(),
            commitment_columns: vec![
                CommitmentColumn {
                    name: "value".to_owned(),
                    stage: 1,
                    dimension: 1,
                    pols_map_id: 0,
                    stage_id: 0,
                    stage_position: 0,
                    intermediate: false,
                    lengths: Vec::new(),
                },
                CommitmentColumn {
                    name: "weight".to_owned(),
                    stage: 1,
                    dimension: 1,
                    pols_map_id: 1,
                    stage_id: 1,
                    stage_position: 1,
                    intermediate: false,
                    lengths: Vec::new(),
                },
            ],
            unit_value_map: Vec::new(),
            group_value_map: Vec::new(),
            stark: StarkStruct {
                n_bits: 1,
                n_bits_ext: 2,
                n_queries: 1,
                steps: vec![FriStep { n_bits: 2 }],
                hash_commits: true,
                last_level_verification: 1,
                pow_bits: 0,
                merkle_tree_arity: 4,
                verification_hash_type: Some("GL".to_owned()),
                transcript_arity: Some(4),
                merkle_tree_custom: Some(true),
            },
        }
    }

    fn source_lookup_schedule() -> crate::ProveUnitSchedule {
        crate::ProveUnitSchedule {
            kind: KeyUnitKind::Basic,
            group_id: Some(0),
            unit_id: Some(0),
            group_name: Some("group".to_owned()),
            unit_name: Some("unit".to_owned()),
            base_domain_bits: 1,
            extended_domain_bits: 2,
            base_domain_size: 2,
            extended_domain_size: 4,
            blowup_factor: 2,
            query_count: 1,
            proof_of_work_bits: 0,
            merkle_tree_arity: 4,
            last_level_verification: 1,
            transcript_arity: Some(4),
            hash_commits: true,
            transcript_root_challenge_draws: vec![2],
            challenge_count: 0,
            evaluation_value_count: 0,
            evaluation_map: Vec::new(),
            transcript_evaluation_challenge_draws: 0,
            constant_width: 0,
            stage_commit_widths: vec![2],
            commitment_columns: source_lookup_setup().commitment_columns,
            unit_value_map: Vec::new(),
            group_value_map: Vec::new(),
            opening_points: vec![0],
            fri_layers: Vec::new(),
            final_layer_bits: 0,
            fixed_bytes: 0,
            constant_tree_root: None,
            pcs_material_bytes: None,
            pcs_material_plan_digest: None,
            pcs_material_fixed_column_digest: None,
            pcs_material_constant_tree_digest: None,
            pcs_material_constant_tree_root: None,
            pcs_material_fixed_byte_count: None,
            pcs_material_constant_tree_byte_count: None,
            pcs_material_leaf_byte_count: None,
            pcs_material_node_byte_count: None,
        }
    }

    fn source_lookup_trace(values: &[u64]) -> WitnessTraceBuffer {
        let bytes = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        parse_witness_trace(&bytes, 2, 2).expect("trace should parse")
    }

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("lzvm-witness-execution-{name}-{stamp}"))
    }

    fn empty_fixed_columns_material(row_count: u64) -> crate::FixedColumnsMaterial {
        crate::FixedColumnsMaterial {
            fixed_columns: FixedColumns {
                group_name: "group".to_owned(),
                unit_name: "unit".to_owned(),
                row_count,
                columns: Vec::new(),
            },
            row_major_values: Vec::new(),
            raw_bytes: Vec::new(),
            #[cfg(feature = "cuda")]
            device_buffer: None,
            #[cfg(feature = "cuda")]
            device_buffer_is_row_major: false,
        }
    }

    fn single_fixed_columns_material(values: &[u64]) -> crate::FixedColumnsMaterial {
        crate::FixedColumnsMaterial {
            fixed_columns: FixedColumns {
                group_name: "group".to_owned(),
                unit_name: "unit".to_owned(),
                row_count: u64::try_from(values.len()).expect("row count should fit u64"),
                columns: vec![FixedColumn {
                    name: "constant".to_owned(),
                    dimensions: Vec::new(),
                    values: values.to_vec(),
                }],
            },
            row_major_values: values
                .iter()
                .map(|value| Felt::from_canonical(*value).expect("value should be canonical"))
                .collect(),
            raw_bytes: Vec::new(),
            #[cfg(feature = "cuda")]
            device_buffer: None,
            #[cfg(feature = "cuda")]
            device_buffer_is_row_major: false,
        }
    }
}

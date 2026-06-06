use std::collections::BTreeMap;
use std::fmt;
use std::sync::mpsc;
#[cfg(feature = "cuda")]
use std::sync::Arc;
use std::thread;

use crate::guest_instruction::{
    RiscvAmoKind, RiscvAmoWidth, RiscvDmaKind, RiscvInstruction, RiscvOpImmKind, RiscvOpKind,
    RiscvPrecompileKind,
};
use crate::guest_machine::{
    advance_guest_machine_with_prepared_fcalls, decode_current_guest_instruction,
    prepare_current_guest_instruction, run_guest_machine_trace_with_fcalls,
    run_guest_machine_with_fcalls, GuestDmaProofValueFlags, GuestMachineHalt, GuestMachineMemory,
    GuestMachineReport, GuestMachineRunError, GuestMachineState, GuestMachineTraceSliceStatus,
    GuestMemoryAccess, GuestMemoryAccessKind, GuestRegisterWrite,
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
use lzvm_accel::CudaDeviceBuffer;
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
) -> Result<(), GuestPcTraceSegmentStreamError<E>> {
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
    .map(|_| ())
}

pub(crate) fn for_each_guest_pc_trace_segment_collecting_proof_values_with_context<E>(
    backend: &GuestPcTraceBackend,
    context: WitnessComputeContext<'_>,
    request: WitnessTraceRequest<'_>,
    mut emit: impl FnMut(GuestPcTraceSegmentRunOutput) -> Result<(), E>,
) -> Result<Vec<WitnessTraceProofValue>, GuestPcTraceSegmentStreamError<E>> {
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
    reports: Vec<GuestMachineReport>,
}

struct GuestPcTracePendingSegmentSlice {
    trace_instance_index: u32,
    reports: Vec<GuestMachineReport>,
    terminal_pc: u64,
    lookahead_instruction: Option<RiscvInstruction>,
    is_last_segment: bool,
}

#[cfg_attr(feature = "cuda", allow(clippy::large_enum_variant))]
enum GuestPcTraceSegmentStreamMessage {
    Segment(GuestPcTraceSegmentTrace),
    Complete(Vec<WitnessTraceProofValue>),
    Error(GuestPcTraceBackendError),
}

enum GuestPcTracePendingSegmentMessage {
    Segment(GuestPcTracePendingSegmentSlice),
    Complete(Vec<WitnessTraceProofValue>),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZiskMainDeviceTraceDescriptors {
    descriptor_words: usize,
    descriptor_rows: usize,
    row_count: usize,
    column_count: usize,
    terminal_pc: u64,
    words: Vec<u64>,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GuestPcTraceDeviceTraceStage {
    stage_index: usize,
    row_count: usize,
    column_count: usize,
    row_stride: usize,
    column_offset: usize,
}

#[cfg(feature = "cuda")]
impl ZiskMainDeviceTraceDescriptors {
    fn new(row_count: usize, column_count: usize, terminal_pc: u64) -> Self {
        let word_capacity = row_count
            .checked_mul(ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS)
            .unwrap_or(0);
        Self {
            descriptor_words: ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS,
            descriptor_rows: 0,
            row_count,
            column_count,
            terminal_pc,
            words: Vec::with_capacity(word_capacity),
        }
    }

    pub(crate) fn descriptor_rows(&self) -> usize {
        self.descriptor_rows
    }

    pub(crate) fn descriptor_word_count(&self) -> usize {
        self.descriptor_words
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
}

#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_COLUMNS: usize = 39;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS: usize = 11;
#[cfg(feature = "cuda")]
const ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS: usize = 14;
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
fn zisk_main_device_trace_descriptors(
    layout: &WitnessTraceLayout,
    columns: &ZiskMainTraceColumns<'_>,
    terminal_pc: u64,
) -> Option<ZiskMainDeviceTraceDescriptors> {
    if !guest_pc_device_trace_source_enabled()
        || !zisk_main_device_trace_layout_supported(layout, columns)
    {
        return None;
    }
    Some(ZiskMainDeviceTraceDescriptors::new(
        layout.row_count(),
        layout.column_count(),
        terminal_pc,
    ))
}

#[cfg(feature = "cuda")]
fn zisk_main_device_trace_layout_supported(
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
fn append_zisk_main_device_trace_descriptor(
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
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) fn build_guest_pc_trace_stage_source_devices_from_device_material(
    layout: &WitnessTraceLayout,
    material: &GuestPcTraceDeviceSegmentMaterial,
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

    let descriptor_buffer = Arc::new(
        CudaDeviceBuffer::from_u64_words(descriptors.words()).map_err(|error| {
            guest_pc_device_trace_source_error(format!(
                "CUDA trace descriptor upload failed: {error}"
            ))
        })?,
    );
    let mut builder = build_guest_pc_trace_stage_source_devices_from_device_descriptors(
        layout,
        material,
        descriptor_buffer.as_ref(),
    )?;
    builder.device_trace_descriptor_buffer = Some(descriptor_buffer);
    Ok(builder)
}

#[cfg(feature = "cuda")]
pub(crate) fn build_guest_pc_trace_stage_source_devices_from_device_descriptors(
    layout: &WitnessTraceLayout,
    material: &GuestPcTraceDeviceSegmentMaterial,
    device_trace_descriptor_buffer: &CudaDeviceBuffer,
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

    let trace_device = CudaDeviceBuffer::from_zisk_main_trace_descriptors_device(
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
    })?;
    let builder =
        guest_pc_device_trace_builder_from_layout_with_descriptors(layout, trace_device, None);
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
        let trace_device = CudaDeviceBuffer::from_zisk_main_trace_descriptors(
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
        })?;
        let builder = guest_pc_device_trace_builder(layout, trace, trace_device);
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
    let stages = layout
        .stages()
        .iter()
        .map(|stage| GuestPcTraceDeviceTraceStage {
            stage_index: stage.stage_index,
            row_count: layout.row_count(),
            column_count: stage.width,
            row_stride: layout.column_count(),
            column_offset: stage.start_column,
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
    std::env::var("LZVM_CUDA_GUEST_PC_DEVICE_TRACE_SOURCE")
        .map(|value| {
            !matches!(
                value.as_str(),
                "0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF"
            )
        })
        .unwrap_or(true)
}

#[cfg(feature = "cuda")]
fn guest_pc_device_trace_source_deep_validation_enabled() -> bool {
    std::env::var("LZVM_CUDA_VALIDATE_GUEST_PC_DEVICE_TRACE_SOURCE")
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
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
    handler: &mut ZiskInputFcallHandler,
    instruction_limit: u64,
    row_limit: usize,
) -> Result<GuestPcTraceSegmentSlice, GuestPcTraceBackendError> {
    let mut reports = Vec::new();
    let mut executed_instructions = 0_u64;
    let mut trace_rows = 0_usize;
    loop {
        let pc = state.pc();
        let prepared = prepare_current_guest_instruction(memory, pc)
            .map_err(GuestMachineRunError::from)
            .map_err(GuestPcTraceBackendError::GuestRun)?;
        let current = prepared.instruction();
        if current == RiscvInstruction::Ecall {
            return Ok(GuestPcTraceSegmentSlice {
                executed_instructions,
                trace_rows,
                status: GuestMachineTraceSliceStatus::Halted(GuestMachineHalt::Ecall {
                    address: pc,
                }),
                reports,
            });
        }
        if executed_instructions == instruction_limit {
            return Ok(GuestPcTraceSegmentSlice {
                executed_instructions,
                trace_rows,
                status: GuestMachineTraceSliceStatus::Paused { pc },
                reports,
            });
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
            return Ok(GuestPcTraceSegmentSlice {
                executed_instructions,
                trace_rows,
                status: GuestMachineTraceSliceStatus::Paused { pc },
                reports,
            });
        }
        let report = advance_guest_machine_with_prepared_fcalls(memory, state, handler, prepared)
            .map_err(GuestMachineRunError::from)
            .map_err(GuestPcTraceBackendError::GuestRun)?;
        let report_rows = zisk_main_report_row_count(reports.len(), &report)?;
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
        reports.push(report);
        executed_instructions += 1;
        if trace_rows == row_limit {
            let pc = state.pc();
            let current = decode_current_guest_instruction(memory, pc)
                .map_err(GuestMachineRunError::from)
                .map_err(GuestPcTraceBackendError::GuestRun)?;
            let status = if current == RiscvInstruction::Ecall {
                GuestMachineTraceSliceStatus::Halted(GuestMachineHalt::Ecall { address: pc })
            } else {
                GuestMachineTraceSliceStatus::Paused { pc }
            };
            return Ok(GuestPcTraceSegmentSlice {
                executed_instructions,
                trace_rows,
                status,
                reports,
            });
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
        let (halted, terminal_pc, lookahead_instruction) = match slice.status {
            GuestMachineTraceSliceStatus::Halted(halt) => {
                (true, guest_machine_halt_pc(&halt), None)
            }
            GuestMachineTraceSliceStatus::Paused { pc } => {
                let instruction = decode_current_guest_instruction(&memory, pc)
                    .map_err(GuestMachineRunError::from)
                    .map_err(GuestPcTraceBackendError::GuestRun)?;
                (false, pc, Some(instruction))
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
) -> Result<Vec<WitnessTraceProofValue>, GuestPcTraceSegmentStreamError<E>> {
    let (sender, receiver) = mpsc::sync_channel(guest_pc_trace_segment_queue_capacity());
    thread::scope(|scope| {
        let producer = scope.spawn(move || {
            let produced = produce_guest_pc_trace_segments(
                instruction_limit,
                context,
                input,
                expected_proof_values,
                |segment| {
                    sender
                        .send(GuestPcTraceSegmentStreamMessage::Segment(segment))
                        .map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
                            message: "guest PC trace segment consumer stopped".to_owned(),
                        })
                },
            );
            let message = match produced {
                Ok(proof_values) => GuestPcTraceSegmentStreamMessage::Complete(proof_values),
                Err(error) => GuestPcTraceSegmentStreamMessage::Error(error),
            };
            let _ = sender.send(message);
        });

        let mut emit_error = None;
        let mut stream_result: Option<
            Result<Vec<WitnessTraceProofValue>, GuestPcTraceSegmentStreamError<E>>,
        > = None;
        while let Ok(message) = receiver.recv() {
            match message {
                GuestPcTraceSegmentStreamMessage::Segment(segment) => {
                    if emit_error.is_none() {
                        if let Err(error) = emit(segment) {
                            emit_error = Some(error);
                        }
                    }
                }
                GuestPcTraceSegmentStreamMessage::Complete(proof_values) => {
                    stream_result = Some(Ok(proof_values));
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

fn produce_guest_pc_trace_segments(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    input: &[u8],
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    mut emit: impl FnMut(GuestPcTraceSegmentTrace) -> Result<(), GuestPcTraceBackendError>,
) -> Result<Vec<WitnessTraceProofValue>, GuestPcTraceBackendError> {
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
        let runner = scope.spawn(move || {
            let produced = produce_guest_pc_trace_pending_slices(
                instruction_limit,
                context,
                input,
                row_count,
                |pending| {
                    pending_sender
                        .send(GuestPcTracePendingSegmentMessage::Segment(pending))
                        .map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
                            message: "guest PC trace pending segment consumer stopped".to_owned(),
                        })
                },
            );
            let message = match produced {
                Ok(proof_values) => GuestPcTracePendingSegmentMessage::Complete(proof_values),
                Err(error) => GuestPcTracePendingSegmentMessage::Error(error),
            };
            let _ = pending_sender.send(message);
        });

        let result = lower_guest_pc_trace_pending_segments(
            layout,
            pending_receiver,
            expected_proof_values,
            &mut emit,
        );
        if let Err(payload) = runner.join() {
            std::panic::resume_unwind(payload);
        }
        let actual_proof_values = result?;
        if expected_proof_values.is_some_and(|expected| actual_proof_values.as_slice() != expected)
        {
            return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                message: "guest PC trace runtime proof values changed between passes".to_owned(),
            });
        }
        Ok(actual_proof_values)
    })
}

fn produce_guest_pc_trace_pending_slices(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    input: &[u8],
    row_count: usize,
    mut emit: impl FnMut(GuestPcTracePendingSegmentSlice) -> Result<(), GuestPcTraceBackendError>,
) -> Result<Vec<WitnessTraceProofValue>, GuestPcTraceBackendError> {
    let (mut memory, mut state, mut fcall_handler) = load_guest_pc_trace_machine(context, input)?;
    let mut executed_instructions = 0_u64;
    let mut trace_instance_count = 0_usize;
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
        let (halted, terminal_pc, lookahead_instruction) = match slice.status {
            GuestMachineTraceSliceStatus::Halted(halt) => {
                (true, guest_machine_halt_pc(&halt), None)
            }
            GuestMachineTraceSliceStatus::Paused { pc } => {
                let instruction = decode_current_guest_instruction(&memory, pc)
                    .map_err(GuestMachineRunError::from)
                    .map_err(GuestPcTraceBackendError::GuestRun)?;
                (false, pc, Some(instruction))
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
        emit(GuestPcTracePendingSegmentSlice {
            trace_instance_index,
            reports: slice.reports,
            terminal_pc,
            lookahead_instruction,
            is_last_segment,
        })?;
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

    Ok(zisk_runtime_proof_values(
        executed_instructions != 0,
        fcall_handler.input_data_was_mapped(),
        state.dma_proof_value_flags(),
    ))
}

fn lower_guest_pc_trace_pending_segments(
    layout: &WitnessTraceLayout,
    pending_receiver: mpsc::Receiver<GuestPcTracePendingSegmentMessage>,
    expected_proof_values: Option<&[WitnessTraceProofValue]>,
    emit: &mut impl FnMut(GuestPcTraceSegmentTrace) -> Result<(), GuestPcTraceBackendError>,
) -> Result<Vec<WitnessTraceProofValue>, GuestPcTraceBackendError> {
    let mut trace_state = ZiskMainTraceState::new();
    let mut previous_c = 0_u64;
    while let Ok(message) = pending_receiver.recv() {
        let pending = match message {
            GuestPcTracePendingSegmentMessage::Segment(pending) => pending,
            GuestPcTracePendingSegmentMessage::Complete(proof_values) => return Ok(proof_values),
            GuestPcTracePendingSegmentMessage::Error(error) => return Err(error),
        };
        let written = build_layout_zisk_main_trace_segment_for_segment_output(
            layout,
            &pending.reports,
            pending.terminal_pc,
            &trace_state,
            pending.lookahead_instruction,
            ZiskMainTraceSegmentInfo {
                trace_instance_index: pending.trace_instance_index,
                is_last_segment: pending.is_last_segment,
                previous_c,
            },
        )?
        .ok_or(GuestPcTraceBackendError::UnmappedTraceLayout)?;
        previous_c = written.final_state.last_c;
        trace_state = written.continuation_state;
        emit(GuestPcTraceSegmentTrace {
            trace_instance_index: pending.trace_instance_index,
            trace_source_prefix_rows: written.trace_source_prefix_rows,
            #[cfg(feature = "cuda")]
            device_segment_material: written.device_segment_material,
            trace: written.trace,
            unit_values: written.output.unit_values,
            proof_values: expected_proof_values.unwrap_or_default().to_vec(),
        })?;
    }
    Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "guest PC trace pending segment runner stopped".to_owned(),
    })
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
    internal_memory: BTreeMap<u64, u64>,
    register_mem_steps: [u64; 32],
    pending_dma: Option<ZiskMainPendingDma>,
    last_c: u64,
    next_pc: u64,
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
            internal_memory: BTreeMap::new(),
            register_mem_steps: [0; 32],
            pending_dma: None,
            last_c: 0,
            next_pc: 0,
        }
    }
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
            precompile_memory_accesses: &report.precompile_memory_accesses,
            precompile_result: report.precompile_result,
        }
    }
}

#[derive(Debug, Clone)]
struct ZiskMainLoweredReportRow<'a> {
    instruction: ZiskMainInstruction,
    effects: ZiskMainReportEffects<'a>,
    expected_next_pc: u64,
}

#[derive(Debug, Clone, Copy)]
struct ZiskMainReportWindow<'a> {
    current: &'a GuestMachineReport,
    next_instruction: Option<RiscvInstruction>,
}

#[derive(Debug, Clone, Copy)]
struct ZiskMainReportValidationContext<'a> {
    columns: Option<&'a ZiskMainTraceColumns<'a>>,
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZiskMainRegisterAccessValues {
    a_prev_mem_step: Option<u64>,
    b_prev_mem_step: Option<u64>,
    store_prev_mem_step: Option<u64>,
    store_prev_value: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZiskMainRegisterAccessUpdate {
    values: ZiskMainRegisterAccessValues,
    next_mem_steps: [u64; 32],
}

fn validate_and_apply_zisk_main_report(
    row: usize,
    report: &GuestMachineReport,
    next_instruction: Option<RiscvInstruction>,
    state: &mut ZiskMainTraceState,
    context: ZiskMainReportValidationContext<'_>,
    mut visit: impl FnMut(usize, ZiskMainReportTraceValues) -> Result<(), GuestPcTraceBackendError>,
) -> Result<usize, GuestPcTraceBackendError> {
    let consumed_pending_dma = state.pending_dma.is_some();
    let lowered = lower_stateful_zisk_main_report_rows(row, report, next_instruction, state)?;
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
        let instruction = lowered_row.instruction;
        if let Some(columns) = context.columns {
            validate_zisk_main_memory_columns(output_row, &instruction, columns)?;
        }
        let (a, a_access) = zisk_main_source_value(
            output_row,
            instruction.a,
            state,
            report,
            lowered_row.effects,
            None,
            0,
        )?;
        let (b, b_access) = zisk_main_source_value(
            output_row,
            instruction.b,
            state,
            report,
            lowered_row.effects,
            Some(a),
            instruction.ind_width,
        )?;
        validate_zisk_main_precompile_memory_accesses(output_row, report, lowered_row.effects, b)?;
        let (c, flag) =
            zisk_main_instruction_result(output_row, &instruction, a, b, lowered_row.effects)?;
        validate_zisk_main_next_pc(
            output_row,
            &instruction,
            lowered_row.expected_next_pc,
            c,
            flag,
        )?;
        let register_accesses = zisk_main_register_access_values(
            output_row,
            &instruction,
            state,
            context.row_count,
            context.segment,
        )?;
        validate_zisk_main_memory_accesses(
            output_row,
            &instruction,
            lowered_row.effects,
            a,
            c,
            a_access,
            b_access,
        )?;
        apply_zisk_main_store(
            output_row,
            &instruction,
            c,
            lowered_row.effects,
            lowered_row.expected_next_pc,
            state,
        )?;
        state.register_mem_steps = register_accesses.next_mem_steps;
        visit(
            output_row,
            ZiskMainReportTraceValues {
                instruction,
                a,
                b,
                c,
                flag,
                register_accesses: register_accesses.values,
            },
        )?;
    }
    state.pending_dma = if consumed_pending_dma {
        None
    } else {
        zisk_main_pending_dma(report)
    };
    Ok(produced_rows)
}

fn lower_stateful_zisk_main_report_rows<'a>(
    row: usize,
    report: &'a GuestMachineReport,
    next_instruction: Option<RiscvInstruction>,
    state: &ZiskMainTraceState,
) -> Result<Vec<ZiskMainLoweredReportRow<'a>>, GuestPcTraceBackendError> {
    if let Some(pending) = state.pending_dma {
        return Ok(vec![ZiskMainLoweredReportRow {
            instruction: lower_pending_dma_report(row, report, pending)?,
            effects: ZiskMainReportEffects::from_report(report),
            expected_next_pc: report.next_pc,
        }]);
    }
    if let RiscvInstruction::StoreConditional {
        width,
        rd,
        rs1,
        rs2,
        ..
    } = report.instruction
    {
        return lower_store_conditional_report_rows(row, report, width, rd, rs1, rs2);
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
        return lower_amo_report_rows(row, report, kind, width, rd, rs1, rs2);
    }
    let instruction = lower_guest_report(report)
        .map_err(|source| GuestPcTraceBackendError::ZiskMainLower { row, source })?;
    let instruction = if let RiscvInstruction::ZiskDmaPrepare { kind, .. } = report.instruction {
        lower_dma_prepare_report(row, instruction, kind, next_instruction)?
    } else {
        instruction
    };
    Ok(vec![ZiskMainLoweredReportRow {
        instruction,
        effects: ZiskMainReportEffects::from_report(report),
        expected_next_pc: report.next_pc,
    }])
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
                byte_len,
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

fn zisk_main_register_access_values(
    row: usize,
    instruction: &ZiskMainInstruction,
    state: &ZiskMainTraceState,
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
) -> Result<ZiskMainRegisterAccessUpdate, GuestPcTraceBackendError> {
    let mut next_mem_steps = state.register_mem_steps;
    let mut values = ZiskMainRegisterAccessValues {
        a_prev_mem_step: None,
        b_prev_mem_step: None,
        store_prev_mem_step: None,
        store_prev_value: None,
    };

    if let Some(index) = zisk_main_source_register_index(row, instruction.a)? {
        values.a_prev_mem_step = Some(next_mem_steps[index]);
        next_mem_steps[index] = zisk_main_row_mem_step(
            row_count,
            segment.trace_instance_index,
            row,
            ZISK_MAIN_A_MEM_STEP_OFFSET,
        )?;
    }
    if let Some(index) = zisk_main_source_register_index(row, instruction.b)? {
        values.b_prev_mem_step = Some(next_mem_steps[index]);
        next_mem_steps[index] = zisk_main_row_mem_step(
            row_count,
            segment.trace_instance_index,
            row,
            ZISK_MAIN_B_MEM_STEP_OFFSET,
        )?;
    }
    if let Some(index) = zisk_main_store_register_index(row, instruction.store)? {
        values.store_prev_mem_step = Some(next_mem_steps[index]);
        values.store_prev_value = Some(state.registers[index]);
        next_mem_steps[index] = zisk_main_row_mem_step(
            row_count,
            segment.trace_instance_index,
            row,
            ZISK_MAIN_STORE_MEM_STEP_OFFSET,
        )?;
    }

    Ok(ZiskMainRegisterAccessUpdate {
        values,
        next_mem_steps,
    })
}

fn zisk_main_source_register_index(
    row: usize,
    source: ZiskMainSource,
) -> Result<Option<usize>, GuestPcTraceBackendError> {
    match source {
        ZiskMainSource::Register(index) => zisk_main_register_index(index)
            .map(Some)
            .map_err(|()| GuestPcTraceBackendError::UnsupportedZiskMainSource { row }),
        _ => Ok(None),
    }
}

fn zisk_main_store_register_index(
    row: usize,
    store: ZiskMainStore,
) -> Result<Option<usize>, GuestPcTraceBackendError> {
    match store {
        ZiskMainStore::Register(index) => zisk_main_register_index(index)
            .map(Some)
            .map_err(|()| GuestPcTraceBackendError::UnsupportedZiskMainStore { row }),
        _ => Ok(None),
    }
}

fn zisk_main_register_index(index: u8) -> Result<usize, ()> {
    let index = usize::from(index);
    if (ZISK_MAIN_REGISTER_START..ZISK_MAIN_REGISTER_START + ZISK_MAIN_REGISTER_COUNT)
        .contains(&index)
    {
        Ok(index)
    } else {
        Err(())
    }
}

fn zisk_main_row_mem_step(
    row_count: usize,
    trace_instance_index: u32,
    row: usize,
    offset: u64,
) -> Result<u64, GuestPcTraceBackendError> {
    let row_count =
        u64::try_from(row_count).map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main row count is too large".to_owned(),
        })?;
    let row = u64::try_from(row).map_err(|_| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "Zisk Main row index is too large".to_owned(),
    })?;
    let main_step = u64::from(trace_instance_index)
        .checked_mul(row_count)
        .and_then(|base| base.checked_add(row))
        .ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: "Zisk Main step is too large".to_owned(),
        })?;
    zisk_main_mem_step(main_step, offset)
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
) -> Result<Option<GuestPcTraceDeviceSegmentBuild>, GuestPcTraceBackendError> {
    let Some(columns) = zisk_main_trace_columns(layout)? else {
        return Ok(None);
    };
    if reports.len() > layout.row_count() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len: layout_trace_byte_len(reports.len(), layout.column_count()),
            output_len: layout_trace_byte_len(layout.row_count(), layout.column_count()),
        });
    }
    let Some(mut device_trace_descriptors) =
        zisk_main_device_trace_descriptors(layout, &columns, terminal_pc)
    else {
        return Ok(None);
    };

    let mut state = initial_state.clone();
    let mut output_row = 0_usize;
    for (report_index, report) in reports.iter().enumerate() {
        let next_instruction = reports
            .get(report_index + 1)
            .map(|next| next.instruction)
            .or_else(|| {
                (report_index + 1 == reports.len())
                    .then_some(lookahead_instruction)
                    .flatten()
            });
        let written_rows = validate_and_apply_zisk_main_report(
            output_row,
            report,
            next_instruction,
            &mut state,
            ZiskMainReportValidationContext {
                columns: Some(&columns),
                row_count: layout.row_count(),
                segment,
            },
            |_, values| {
                append_zisk_main_device_trace_descriptor(&mut device_trace_descriptors, &values)
            },
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
    let unit_values = zisk_main_unit_values(
        layout.row_count(),
        output_row,
        reports,
        terminal_pc,
        &state,
        segment,
    );
    Ok(Some(GuestPcTraceDeviceSegmentBuild {
        device_segment_material: GuestPcTraceDeviceSegmentMaterial {
            trace_source_prefix_rows: output_row,
            device_trace_descriptors,
        },
        unit_values,
        final_state: state,
        continuation_state,
    }))
}

#[cfg(feature = "cuda")]
fn build_layout_zisk_main_trace_segment_from_device_material(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    initial_state: &ZiskMainTraceState,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
) -> Result<Option<ZiskMainTraceSegmentWrite>, GuestPcTraceBackendError> {
    let Some(material) = build_layout_zisk_main_trace_segment_device_material(
        layout,
        reports,
        terminal_pc,
        initial_state,
        lookahead_instruction,
        segment,
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
) -> Result<Option<ZiskMainTraceSegmentWrite>, GuestPcTraceBackendError> {
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
    )
}

#[cfg(feature = "cuda")]
fn guest_pc_trace_less_segment_output_enabled() -> bool {
    std::env::var("LZVM_CUDA_GUEST_PC_TRACELESS_SEGMENT_OUTPUT")
        .map(|value| {
            !matches!(
                value.as_str(),
                "0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF"
            )
        })
        .unwrap_or(true)
}

fn build_layout_zisk_main_trace_segment(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    initial_state: &ZiskMainTraceState,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
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
        zisk_main_device_trace_descriptors(layout, &columns, terminal_pc);
    let mut state = initial_state.clone();
    let mut output_row = 0_usize;
    for (report_index, report) in reports.iter().enumerate() {
        let next_instruction = reports
            .get(report_index + 1)
            .map(|next| next.instruction)
            .or_else(|| {
                (report_index + 1 == reports.len())
                    .then_some(lookahead_instruction)
                    .flatten()
            });
        let written_rows = write_zisk_main_report_columns(
            &mut builder,
            output_row,
            ZiskMainReportWindow {
                current: report,
                next_instruction,
            },
            &columns,
            &mut state,
            layout.row_count(),
            segment,
            #[cfg(feature = "cuda")]
            &mut device_trace_descriptors,
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
    let unit_values = zisk_main_unit_values(
        layout.row_count(),
        output_row,
        reports,
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

fn zisk_main_unit_values(
    row_count: usize,
    written_rows: usize,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    state: &ZiskMainTraceState,
    segment: ZiskMainTraceSegmentInfo,
) -> Vec<WitnessTraceUnitValue> {
    let segment_initial_pc = reports
        .first()
        .map(|report| report.address)
        .unwrap_or(terminal_pc);
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

#[cfg_attr(feature = "cuda", allow(clippy::too_many_arguments))]
fn write_zisk_main_report_columns(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    reports: ZiskMainReportWindow<'_>,
    columns: &ZiskMainTraceColumns<'_>,
    state: &mut ZiskMainTraceState,
    row_count: usize,
    segment: ZiskMainTraceSegmentInfo,
    #[cfg(feature = "cuda")] device_trace_descriptors: &mut Option<ZiskMainDeviceTraceDescriptors>,
) -> Result<usize, GuestPcTraceBackendError> {
    validate_and_apply_zisk_main_report(
        row,
        reports.current,
        reports.next_instruction,
        state,
        ZiskMainReportValidationContext {
            columns: Some(columns),
            row_count,
            segment,
        },
        |output_row, values| {
            #[cfg(feature = "cuda")]
            if let Some(descriptors) = device_trace_descriptors.as_mut() {
                append_zisk_main_device_trace_descriptor(descriptors, &values)?;
            }
            write_zisk_main_row_columns(builder, output_row, values, columns)
        },
    )
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

fn zisk_main_source_value(
    row: usize,
    source: ZiskMainSource,
    state: &ZiskMainTraceState,
    report: &GuestMachineReport,
    effects: ZiskMainReportEffects<'_>,
    base: Option<u64>,
    ind_width: u64,
) -> Result<(u64, Option<ExpectedMemoryAccess>), GuestPcTraceBackendError> {
    match source {
        ZiskMainSource::LastC => Ok((state.last_c, None)),
        ZiskMainSource::Immediate(value) => Ok((value, None)),
        ZiskMainSource::Register(index) => Ok((state.registers[usize::from(index)], None)),
        ZiskMainSource::Indirect(offset) => {
            let Some(base) = base else {
                return Err(GuestPcTraceBackendError::UnsupportedZiskMainSource { row });
            };
            let byte_len = usize::try_from(ind_width)
                .map_err(|_| GuestPcTraceBackendError::UnsupportedZiskMainSource { row })?;
            let address = base.wrapping_add_signed(offset);
            let access = matching_memory_access(
                row,
                effects,
                GuestMemoryAccessKind::Read,
                address,
                byte_len,
            )?;
            Ok((access.value, Some(access)))
        }
        ZiskMainSource::Memory(address) => {
            zisk_main_memory_source_value(row, address, state, report, effects)
        }
    }
}

fn zisk_main_memory_source_value(
    row: usize,
    address: u64,
    state: &ZiskMainTraceState,
    report: &GuestMachineReport,
    effects: ZiskMainReportEffects<'_>,
) -> Result<(u64, Option<ExpectedMemoryAccess>), GuestPcTraceBackendError> {
    if address == ZISK_INPUT_ADDRESS {
        if let RiscvInstruction::ZiskFcallResult { rd } = report.instruction {
            let value = zisk_main_fcall_result_value(row, rd, effects)?;
            return Ok((value, None));
        }
    }
    if zisk_internal_memory_address(address) && effects.memory_accesses.is_empty() {
        let Some(value) = state.internal_memory.get(&address).copied() else {
            return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                row,
                message: format!("missing internal memory value at {address}"),
            });
        };
        return Ok((value, None));
    }
    let access = matching_memory_access(row, effects, GuestMemoryAccessKind::Read, address, 8)?;
    Ok((access.value, Some(access)))
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

fn matching_memory_access(
    row: usize,
    effects: ZiskMainReportEffects<'_>,
    kind: GuestMemoryAccessKind,
    address: u64,
    byte_len: usize,
) -> Result<ExpectedMemoryAccess, GuestPcTraceBackendError> {
    let mut matching = None;
    for access in effects.memory_accesses {
        if access.kind == kind && access.address == address && access.byte_len == byte_len {
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
            byte_len,
            value: access.value,
        }),
        None => Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!("missing {kind:?} access at {address} with byte length {byte_len}"),
        }),
    }
}

fn validate_zisk_main_memory_accesses(
    row: usize,
    instruction: &ZiskMainInstruction,
    effects: ZiskMainReportEffects<'_>,
    a: u64,
    c: u64,
    a_access: Option<ExpectedMemoryAccess>,
    b_access: Option<ExpectedMemoryAccess>,
) -> Result<(), GuestPcTraceBackendError> {
    let store_value = zisk_main_store_value(instruction, c);
    let store_access = if let ZiskMainStore::Indirect(offset) = instruction.store {
        let byte_len = usize::try_from(instruction.ind_width)
            .map_err(|_| GuestPcTraceBackendError::UnsupportedZiskMainStore { row })?;
        Some(ExpectedMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: a.wrapping_add_signed(offset),
            byte_len,
            value: low_bytes_value(store_value, byte_len),
        })
    } else {
        None
    };
    let expected = [a_access, b_access, store_access];
    let expected_len = expected.iter().filter(|access| access.is_some()).count();
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
    for (found, expected) in effects
        .memory_accesses
        .iter()
        .zip(expected.iter().flatten())
    {
        if found.kind != expected.kind
            || found.address != expected.address
            || found.byte_len != expected.byte_len
            || found.value != expected.value
        {
            return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                row,
                message: format!(
                    "expected {:?} at {} byte length {} value {}, found {:?} at {} byte length {} value {}",
                    expected.kind,
                    expected.address,
                    expected.byte_len,
                    expected.value,
                    found.kind,
                    found.address,
                    found.byte_len,
                    found.value
                ),
            });
        }
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
            cursor.expect_reads(operand_address, 25)?;
            cursor.expect_writes(operand_address, 25)?;
        }
        RiscvPrecompileKind::Arith256 => {
            let params = cursor.expect_reads(operand_address, 5)?;
            cursor.expect_reads(params[0], 4)?;
            cursor.expect_reads(params[1], 4)?;
            cursor.expect_reads(params[2], 4)?;
            cursor.expect_writes(params[3], 4)?;
            cursor.expect_writes(params[4], 4)?;
        }
        RiscvPrecompileKind::Arith256Mod => {
            let params = cursor.expect_reads(operand_address, 5)?;
            cursor.expect_reads(params[0], 4)?;
            cursor.expect_reads(params[1], 4)?;
            cursor.expect_reads(params[2], 4)?;
            cursor.expect_reads(params[3], 4)?;
            cursor.expect_writes(params[4], 4)?;
        }
        RiscvPrecompileKind::Secp256k1Add => {
            let params = cursor.expect_reads(operand_address, 2)?;
            cursor.expect_reads(params[0], 8)?;
            cursor.expect_reads(params[1], 8)?;
            cursor.expect_writes(params[0], 8)?;
        }
        RiscvPrecompileKind::Secp256k1Dbl => {
            cursor.expect_reads(operand_address, 8)?;
            cursor.expect_writes(operand_address, 8)?;
        }
        RiscvPrecompileKind::Add256 => {
            let params = cursor.expect_reads(operand_address, 4)?;
            cursor.expect_reads(params[0], 4)?;
            cursor.expect_reads(params[1], 4)?;
            cursor.expect_writes(params[3], 4)?;
        }
    }
    cursor.finish()
}

struct PrecompileMemoryAccessCursor<'a> {
    row: usize,
    accesses: &'a [GuestMemoryAccess],
    offset: usize,
}

impl PrecompileMemoryAccessCursor<'_> {
    fn expect_reads(
        &mut self,
        base_address: u64,
        word_count: usize,
    ) -> Result<Vec<u64>, GuestPcTraceBackendError> {
        let mut values = Vec::with_capacity(word_count);
        for index in 0..word_count {
            values.push(
                self.expect_access(GuestMemoryAccessKind::Read, base_address + index as u64 * 8)?,
            );
        }
        Ok(values)
    }

    fn expect_writes(
        &mut self,
        base_address: u64,
        word_count: usize,
    ) -> Result<Vec<u64>, GuestPcTraceBackendError> {
        let mut values = Vec::with_capacity(word_count);
        for index in 0..word_count {
            values.push(self.expect_access(
                GuestMemoryAccessKind::Write,
                base_address + index as u64 * 8,
            )?);
        }
        Ok(values)
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
    let store_value = zisk_main_store_value(instruction, c);
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
            state.internal_memory.insert(address, store_value);
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
    write_column(builder, row, &columns.byte_len, access.byte_len as u64)
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

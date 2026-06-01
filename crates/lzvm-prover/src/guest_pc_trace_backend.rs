use std::fmt;

use crate::guest_instruction::{RiscvInstruction, RiscvPrecompileKind};
use crate::guest_machine::{
    run_guest_machine_trace_with_fcalls, GuestMachineMemory, GuestMachineReport,
    GuestMachineRunError, GuestMemoryAccess, GuestMemoryAccessKind, GuestRegisterWrite,
};
use crate::guest_memory::{load_guest_memory_image, GuestMemoryError};
use crate::witness_layout::{WitnessTraceBuildError, WitnessTraceLayout};
use crate::witness_loader::{
    WitnessBackend, WitnessCallError, WitnessComputeContext, WitnessTraceBuffers,
    WitnessTraceOutput,
};
use crate::zisk_main::{
    lower_guest_report, ZiskMainInstruction, ZiskMainLowerError, ZiskMainOp, ZiskMainSource,
    ZiskMainStore,
};
use lzvm_field::{Felt, FieldError};

use crate::zisk_fcalls::{ZiskInputFcallError, ZiskInputFcallHandler};

mod precompile_memory_trace;

const ZISK_RAM_ADDRESS: u64 = 0xa000_0000;
const ZISK_RAM_SIZE: u64 = 0x2000_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestPcTraceBackend {
    instruction_limit: u64,
}

impl GuestPcTraceBackend {
    pub fn new(instruction_limit: u64) -> Self {
        Self { instruction_limit }
    }
}

pub fn is_guest_pc_trace_layout_supported(layout: &WitnessTraceLayout) -> bool {
    layout_trace_capacity(Some(layout)).is_ok()
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
    let mut state = crate::guest_machine::GuestMachineState::new(memory.entry_address());
    let mut fcall_handler =
        ZiskInputFcallHandler::new(buffers.input()).map_err(GuestPcTraceBackendError::ZiskInput)?;
    let trace = run_guest_machine_trace_with_fcalls(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        run_instruction_limit,
    );
    let trace = match trace {
        Ok(trace) => trace,
        Err(error) => {
            if let Some(error) = layout_capacity_error(
                layout_capacity,
                run_instruction_limit,
                buffers.output().len(),
                &error,
            ) {
                return Err(error);
            }
            return Err(GuestPcTraceBackendError::GuestRun(error));
        }
    };
    if let Some(layout) = context.trace_layout {
        if let Some(produced_len) =
            write_layout_zisk_main_trace(layout, &trace.reports, buffers.output_mut())?
        {
            return Ok(WitnessTraceOutput { produced_len });
        }
        if let Some(produced_len) = precompile_memory_trace::write_layout_precompile_memory_trace(
            layout,
            &trace.reports,
            buffers.output_mut(),
        )? {
            return Ok(WitnessTraceOutput { produced_len });
        }
        if let Some(produced_len) =
            write_layout_pc_trace(layout, &trace.reports, buffers.output_mut())?
        {
            return Ok(WitnessTraceOutput { produced_len });
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
    Ok(WitnessTraceOutput { produced_len })
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
    output_len: usize,
    error: &GuestMachineRunError,
) -> Option<GuestPcTraceBackendError> {
    let capacity = capacity?;
    match error {
        GuestMachineRunError::InstructionLimitExceeded {
            instruction_limit, ..
        } if *instruction_limit == run_instruction_limit
            && run_instruction_limit == capacity.instruction_limit =>
        {
            Some(GuestPcTraceBackendError::OutputOverflow {
                produced_len: layout_trace_byte_len(
                    capacity.row_count.saturating_add(1),
                    capacity.row_width,
                ),
                output_len,
            })
        }
        _ => None,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraceColumnTarget {
    stage_index: usize,
    trace_column: usize,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PcTraceColumns {
    pc: TraceColumnTarget,
    next_pc: TraceColumnTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegisterWriteColumns {
    index: TraceColumnTarget,
    value: TraceColumnTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryAccessColumns {
    address: TraceColumnTarget,
    value: TraceColumnTarget,
    byte_len: TraceColumnTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestTraceColumns {
    pc: Option<PcTraceColumns>,
    register_write: Option<RegisterWriteColumns>,
    memory_read: Option<MemoryAccessColumns>,
    memory_write: Option<MemoryAccessColumns>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZiskMainTraceColumns {
    a: TraceColumnTarget,
    b: TraceColumnTarget,
    c: TraceColumnTarget,
    flag: TraceColumnTarget,
    pc: TraceColumnTarget,
    a_src_imm: Option<TraceColumnTarget>,
    b_src_imm: Option<TraceColumnTarget>,
    a_src_reg: Option<TraceColumnTarget>,
    b_src_reg: Option<TraceColumnTarget>,
    b_src_ind: Option<TraceColumnTarget>,
    b_offset_imm0: Option<TraceColumnTarget>,
    store_reg: Option<TraceColumnTarget>,
    store_mem: Option<TraceColumnTarget>,
    store_ind: Option<TraceColumnTarget>,
    store_offset: Option<TraceColumnTarget>,
    store_pc: Option<TraceColumnTarget>,
    set_pc: Option<TraceColumnTarget>,
    op: TraceColumnTarget,
    jmp_offset1: Option<TraceColumnTarget>,
    jmp_offset2: Option<TraceColumnTarget>,
    ind_width: Option<TraceColumnTarget>,
    m32: Option<TraceColumnTarget>,
    is_external_op: Option<TraceColumnTarget>,
    is_precompiled: Option<TraceColumnTarget>,
}

impl ZiskMainTraceColumns {
    fn has_required_memory_columns(&self) -> bool {
        self.b_src_ind.is_some()
            && self.b_offset_imm0.is_some()
            && self.ind_width.is_some()
            && self.store_ind.is_some()
            && self.store_offset.is_some()
            && self.store_mem.is_some()
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
    last_c: u64,
}

impl ZiskMainTraceState {
    fn new() -> Self {
        Self {
            registers: [0; 32],
            last_c: 0,
        }
    }
}

struct ZiskMainReportTraceValues {
    instruction: ZiskMainInstruction,
    a: u64,
    b: u64,
    c: u64,
    flag: bool,
}

fn validate_and_apply_zisk_main_report(
    row: usize,
    report: &GuestMachineReport,
    state: &mut ZiskMainTraceState,
    columns: Option<&ZiskMainTraceColumns>,
) -> Result<ZiskMainReportTraceValues, GuestPcTraceBackendError> {
    let instruction = lower_guest_report(report)
        .map_err(|source| GuestPcTraceBackendError::ZiskMainLower { row, source })?;
    if let Some(columns) = columns {
        validate_zisk_main_memory_columns(row, &instruction, columns)?;
    }
    let (a, a_access) = zisk_main_source_value(row, instruction.a, state, report, None, 0)?;
    let (b, b_access) = zisk_main_source_value(
        row,
        instruction.b,
        state,
        report,
        Some(a),
        instruction.ind_width,
    )?;
    validate_zisk_main_precompile_memory_accesses(row, report, b)?;
    let (c, flag) = zisk_main_instruction_result(row, &instruction, a, b, report)?;
    validate_zisk_main_next_pc(row, &instruction, report, c, flag)?;
    validate_zisk_main_memory_accesses(row, &instruction, report, a, c, a_access, b_access)?;
    apply_zisk_main_store(row, &instruction, c, report, state)?;
    Ok(ZiskMainReportTraceValues {
        instruction,
        a,
        b,
        c,
        flag,
    })
}

fn write_layout_zisk_main_trace(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    output: &mut [u8],
) -> Result<Option<usize>, GuestPcTraceBackendError> {
    let Some(columns) = zisk_main_trace_columns(layout)? else {
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
    let mut state = ZiskMainTraceState::new();
    for (row, report) in reports.iter().enumerate() {
        write_zisk_main_report_columns(&mut builder, row, report, &columns, &mut state)?;
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

fn write_zisk_main_report_columns(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    report: &GuestMachineReport,
    columns: &ZiskMainTraceColumns,
    state: &mut ZiskMainTraceState,
) -> Result<(), GuestPcTraceBackendError> {
    let values = validate_and_apply_zisk_main_report(row, report, state, Some(columns))?;
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
    write_optional_column(
        builder,
        row,
        &columns.b_src_imm,
        u64::from(matches!(instruction.b, ZiskMainSource::Immediate(_))),
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
        &columns.b_src_ind,
        u64::from(matches!(instruction.b, ZiskMainSource::Indirect(_))),
    )?;
    write_optional_signed_column(
        builder,
        row,
        &columns.b_offset_imm0,
        zisk_main_source_offset(row, instruction.b)?,
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
    )
}

fn validate_zisk_main_memory_columns(
    row: usize,
    instruction: &ZiskMainInstruction,
    columns: &ZiskMainTraceColumns,
) -> Result<(), GuestPcTraceBackendError> {
    let uses_memory_row = matches!(
        instruction.b,
        ZiskMainSource::Indirect(_) | ZiskMainSource::Memory(_)
    ) || matches!(
        instruction.store,
        ZiskMainStore::Indirect(_) | ZiskMainStore::Memory(_)
    );
    if uses_memory_row && !columns.has_required_memory_columns() {
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
                report,
                GuestMemoryAccessKind::Read,
                address,
                byte_len,
            )?;
            Ok((access.value, Some(access)))
        }
        ZiskMainSource::Memory(_) => {
            Err(GuestPcTraceBackendError::UnsupportedZiskMainSource { row })
        }
    }
}

fn matching_memory_access(
    row: usize,
    report: &GuestMachineReport,
    kind: GuestMemoryAccessKind,
    address: u64,
    byte_len: usize,
) -> Result<ExpectedMemoryAccess, GuestPcTraceBackendError> {
    let matching: Vec<_> = report
        .memory_accesses
        .iter()
        .filter(|access| {
            access.kind == kind && access.address == address && access.byte_len == byte_len
        })
        .collect();
    match matching.as_slice() {
        [access] => Ok(ExpectedMemoryAccess {
            kind,
            address,
            byte_len,
            value: access.value,
        }),
        [] => Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!("missing {kind:?} access at {address} with byte length {byte_len}"),
        }),
        _ => Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!("multiple {kind:?} accesses at {address} with byte length {byte_len}"),
        }),
    }
}

fn validate_zisk_main_memory_accesses(
    row: usize,
    instruction: &ZiskMainInstruction,
    report: &GuestMachineReport,
    a: u64,
    c: u64,
    a_access: Option<ExpectedMemoryAccess>,
    b_access: Option<ExpectedMemoryAccess>,
) -> Result<(), GuestPcTraceBackendError> {
    let mut expected = Vec::new();
    expected.extend(a_access);
    expected.extend(b_access);
    let store_value = zisk_main_store_value(instruction, c);
    if let ZiskMainStore::Indirect(offset) = instruction.store {
        let byte_len = usize::try_from(instruction.ind_width)
            .map_err(|_| GuestPcTraceBackendError::UnsupportedZiskMainStore { row })?;
        expected.push(ExpectedMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: a.wrapping_add_signed(offset),
            byte_len,
            value: low_bytes_value(store_value, byte_len),
        });
    }
    if report.memory_accesses.len() != expected.len() {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "expected {} memory accesses, found {}",
                expected.len(),
                report.memory_accesses.len()
            ),
        });
    }
    for (found, expected) in report.memory_accesses.iter().zip(expected.iter()) {
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
    operand_address: u64,
) -> Result<(), GuestPcTraceBackendError> {
    let RiscvInstruction::ZiskPrecompile { kind, .. } = report.instruction else {
        if report.precompile_memory_accesses.is_empty() {
            return Ok(());
        }
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "non-precompile row reported {} precompile memory accesses",
                report.precompile_memory_accesses.len()
            ),
        });
    };

    let mut cursor = PrecompileMemoryAccessCursor {
        row,
        accesses: &report.precompile_memory_accesses,
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
        ZiskMainStore::Memory(_) => Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row }),
    }
}

fn zisk_main_source_offset(
    row: usize,
    source: ZiskMainSource,
) -> Result<i64, GuestPcTraceBackendError> {
    match source {
        ZiskMainSource::LastC => Ok(0),
        ZiskMainSource::Immediate(value) => Ok(value as i64),
        ZiskMainSource::Register(index) => Ok(i64::from(index)),
        ZiskMainSource::Indirect(offset) => Ok(offset),
        ZiskMainSource::Memory(_) => {
            Err(GuestPcTraceBackendError::UnsupportedZiskMainSource { row })
        }
    }
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
        ZiskMainOp::Add256
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
    report: &GuestMachineReport,
) -> Result<(u64, bool), GuestPcTraceBackendError> {
    match instruction.op {
        ZiskMainOp::Add256 if instruction.is_precompiled => {
            zisk_main_add256_result(row, instruction, report)
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

fn zisk_main_add256_result(
    row: usize,
    instruction: &ZiskMainInstruction,
    report: &GuestMachineReport,
) -> Result<(u64, bool), GuestPcTraceBackendError> {
    let Some(result) = report.precompile_result else {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: "Add256 row missing precompile result".to_owned(),
        });
    };
    match instruction.store {
        ZiskMainStore::None => Ok((result, false)),
        ZiskMainStore::Register(index) => {
            let [write] = report.register_writes.as_slice() else {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!(
                        "Add256 row reported {} register writes",
                        report.register_writes.len()
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
    report: &GuestMachineReport,
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
    if report.next_pc != expected_next_pc {
        return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
            row,
            message: format!(
                "expected next pc {expected_next_pc}, found {}",
                report.next_pc
            ),
        });
    }
    Ok(())
}

fn apply_zisk_main_store(
    row: usize,
    instruction: &ZiskMainInstruction,
    c: u64,
    report: &GuestMachineReport,
    state: &mut ZiskMainTraceState,
) -> Result<(), GuestPcTraceBackendError> {
    let store_value = zisk_main_store_value(instruction, c);
    match instruction.store {
        ZiskMainStore::None => {
            if !report.register_writes.is_empty() {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: "store none row reported register writes".to_owned(),
                });
            }
        }
        ZiskMainStore::Register(index) => {
            let [write] = report.register_writes.as_slice() else {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: format!(
                        "store register row reported {} register writes",
                        report.register_writes.len()
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
            if !report.register_writes.is_empty() {
                return Err(GuestPcTraceBackendError::ZiskMainEffectMismatch {
                    row,
                    message: "store indirect row reported register writes".to_owned(),
                });
            }
        }
        ZiskMainStore::Memory(_) => {
            return Err(GuestPcTraceBackendError::UnsupportedZiskMainStore { row });
        }
    }
    state.last_c = c;
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
    columns: &GuestTraceColumns,
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
    columns: &RegisterWriteColumns,
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
    columns: &MemoryAccessColumns,
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
    column: &TraceColumnTarget,
    value: u64,
) -> Result<(), GuestPcTraceBackendError> {
    let value = canonical_trace_value(row, &column.name, value)?;
    builder
        .write_column_values(row, column.stage_index, &column.name, &[value])
        .map_err(GuestPcTraceBackendError::TraceBuild)
}

fn write_optional_column(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    column: &Option<TraceColumnTarget>,
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
    column: &TraceColumnTarget,
    value: u64,
) -> Result<(), GuestPcTraceBackendError> {
    let values = [
        canonical_trace_value(row, &column.name, value & 0xffff_ffff)?,
        canonical_trace_value(row, &column.name, value >> 32)?,
    ];
    builder
        .write_column_values(row, column.stage_index, &column.name, &values)
        .map_err(GuestPcTraceBackendError::TraceBuild)
}

fn write_optional_signed_column(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    column: &Option<TraceColumnTarget>,
    value: i64,
) -> Result<(), GuestPcTraceBackendError> {
    let Some(column) = column else {
        return Ok(());
    };
    let value = signed_trace_value(row, &column.name, value)?;
    builder
        .write_column_values(row, column.stage_index, &column.name, &[value])
        .map_err(GuestPcTraceBackendError::TraceBuild)
}

fn guest_trace_columns(
    layout: &WitnessTraceLayout,
) -> Result<Option<GuestTraceColumns>, GuestPcTraceBackendError> {
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

fn zisk_main_trace_columns(
    layout: &WitnessTraceLayout,
) -> Result<Option<ZiskMainTraceColumns>, GuestPcTraceBackendError> {
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
        b_src_imm: trace_column_target(layout, "b_src_imm")?,
        a_src_reg: trace_column_target(layout, "a_src_reg")?,
        b_src_reg: trace_column_target(layout, "b_src_reg")?,
        b_src_ind: trace_column_target(layout, "b_src_ind")?,
        b_offset_imm0: trace_column_target(layout, "b_offset_imm0")?,
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

fn pc_trace_columns(
    layout: &WitnessTraceLayout,
) -> Result<Option<PcTraceColumns>, GuestPcTraceBackendError> {
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
            if pc.trace_column == next_pc.trace_column {
                return Err(GuestPcTraceBackendError::InvalidPcTraceLayout {
                    message: format!(
                        "pc and next_pc columns share trace column {}",
                        pc.trace_column
                    ),
                });
            }
            Ok(Some(PcTraceColumns { pc, next_pc }))
        }
    }
}

fn register_write_columns(
    layout: &WitnessTraceLayout,
) -> Result<Option<RegisterWriteColumns>, GuestPcTraceBackendError> {
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

fn memory_access_columns(
    layout: &WitnessTraceLayout,
    address_name: &str,
    value_name: &str,
    byte_len_name: &str,
) -> Result<Option<MemoryAccessColumns>, GuestPcTraceBackendError> {
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

fn required_trace_column_target(
    layout: &WitnessTraceLayout,
    name: &str,
) -> Result<TraceColumnTarget, GuestPcTraceBackendError> {
    trace_column_target(layout, name)?.ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!("missing {name} column"),
        }
    })
}

fn required_vector_trace_column_target(
    layout: &WitnessTraceLayout,
    name: &str,
    dimension: usize,
) -> Result<TraceColumnTarget, GuestPcTraceBackendError> {
    vector_trace_column_target(layout, name, dimension)?.ok_or_else(|| {
        GuestPcTraceBackendError::InvalidPcTraceLayout {
            message: format!("missing {name} column"),
        }
    })
}

fn vector_trace_column_target(
    layout: &WitnessTraceLayout,
    name: &str,
    dimension: usize,
) -> Result<Option<TraceColumnTarget>, GuestPcTraceBackendError> {
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
        stage_index: column.stage_index(),
        trace_column: column.trace_column(),
        name: column.name().to_owned(),
    }))
}

fn trace_column_target(
    layout: &WitnessTraceLayout,
    name: &str,
) -> Result<Option<TraceColumnTarget>, GuestPcTraceBackendError> {
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
        stage_index: column.stage_index(),
        trace_column: column.trace_column(),
        name: column.name().to_owned(),
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

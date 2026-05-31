use std::fmt;

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
use lzvm_field::{Felt, FieldError};

use crate::zisk_fcalls::{ZiskInputFcallError, ZiskInputFcallHandler};

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
    InvalidPcTraceLayout {
        message: String,
    },
    ZiskMainTraceLayout,
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
            Self::InvalidPcTraceLayout { message } => {
                write!(f, "guest PC trace backend layout is invalid: {message}")
            }
            Self::ZiskMainTraceLayout => write!(
                f,
                "guest PC trace backend cannot write Zisk Main witness rows from raw guest PC reports"
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
            Self::MissingGuestImage
            | Self::MissingGuestImageInfo
            | Self::InvalidPcTraceLayout { .. }
            | Self::ZiskMainTraceLayout
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
    let row_width = match guest_trace_columns(layout)? {
        Some(_) => layout.column_count(),
        None if is_raw_pc_pair_layout(layout) => 2,
        None => return Err(GuestPcTraceBackendError::UnmappedTraceLayout),
    };
    Ok(Some(LayoutTraceCapacity {
        row_count: layout.row_count(),
        row_width,
        instruction_limit: u64::try_from(layout.row_count()).unwrap_or(u64::MAX),
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

fn guest_trace_columns(
    layout: &WitnessTraceLayout,
) -> Result<Option<GuestTraceColumns>, GuestPcTraceBackendError> {
    if is_zisk_main_trace_layout(layout) {
        return Err(GuestPcTraceBackendError::ZiskMainTraceLayout);
    }
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

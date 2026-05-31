use std::fmt;

use crate::guest_machine::{
    run_guest_machine_trace_with_fcalls, GuestMachineMemory, GuestMachineRunError,
};
use crate::guest_memory::{load_guest_memory_image, GuestMemoryError};
use crate::witness_layout::{WitnessTraceBuildError, WitnessTraceLayout};
use crate::witness_loader::{
    WitnessBackend, WitnessCallError, WitnessComputeContext, WitnessTraceBuffers,
    WitnessTraceOutput,
};
use lzvm_field::{Felt, FieldError};

use crate::zisk_fcalls::{ZiskInputFcallError, ZiskInputFcallHandler};

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
    let mut state = crate::guest_machine::GuestMachineState::new(memory.entry_address());
    let mut fcall_handler =
        ZiskInputFcallHandler::new(buffers.input()).map_err(GuestPcTraceBackendError::ZiskInput)?;
    let trace = run_guest_machine_trace_with_fcalls(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        instruction_limit,
    )
    .map_err(GuestPcTraceBackendError::GuestRun)?;
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct PcTraceColumnTarget {
    stage_index: usize,
    trace_column: usize,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PcTraceColumns {
    pc: PcTraceColumnTarget,
    next_pc: PcTraceColumnTarget,
}

fn write_layout_pc_trace(
    layout: &WitnessTraceLayout,
    reports: &[crate::guest_machine::GuestMachineReport],
    output: &mut [u8],
) -> Result<Option<usize>, GuestPcTraceBackendError> {
    let Some(columns) = pc_trace_columns(layout)? else {
        return Ok(None);
    };
    let pc_column = &columns.pc;
    let next_pc_column = &columns.next_pc;
    if reports.len() > layout.row_count() {
        let produced_len = reports
            .len()
            .checked_mul(layout.column_count())
            .and_then(|count| count.checked_mul(8))
            .unwrap_or(usize::MAX);
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len,
            output_len: output.len(),
        });
    }

    let mut builder = layout
        .trace_builder()
        .map_err(GuestPcTraceBackendError::TraceBuild)?;
    for (row, report) in reports.iter().enumerate() {
        let pc = canonical_trace_value(row, &pc_column.name, report.address)?;
        builder
            .write_column_values(row, pc_column.stage_index, &pc_column.name, &[pc])
            .map_err(GuestPcTraceBackendError::TraceBuild)?;
        let next_pc = canonical_trace_value(row, &next_pc_column.name, report.next_pc)?;
        builder
            .write_column_values(
                row,
                next_pc_column.stage_index,
                &next_pc_column.name,
                &[next_pc],
            )
            .map_err(GuestPcTraceBackendError::TraceBuild)?;
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

fn pc_trace_columns(
    layout: &WitnessTraceLayout,
) -> Result<Option<PcTraceColumns>, GuestPcTraceBackendError> {
    let pc = pc_trace_column_target(layout, "pc")?;
    let next_pc = pc_trace_column_target(layout, "next_pc")?;
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

fn pc_trace_column_target(
    layout: &WitnessTraceLayout,
    name: &str,
) -> Result<Option<PcTraceColumnTarget>, GuestPcTraceBackendError> {
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
    Ok(Some(PcTraceColumnTarget {
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

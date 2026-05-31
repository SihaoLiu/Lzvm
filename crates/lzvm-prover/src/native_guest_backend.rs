use std::fmt;

use crate::guest_machine::{run_guest_machine_trace, GuestMachineMemory, GuestMachineRunError};
use crate::guest_memory::{load_guest_memory_image, GuestMemoryError};
use crate::witness_loader::{
    WitnessBackend, WitnessCallError, WitnessComputeContext, WitnessTraceBuffers,
    WitnessTraceOutput,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGuestBackend {
    instruction_limit: u64,
}

impl NativeGuestBackend {
    pub fn new(instruction_limit: u64) -> Self {
        Self { instruction_limit }
    }
}

impl WitnessBackend for NativeGuestBackend {
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
        compute_native_guest_trace(self.instruction_limit, context, buffers)
            .map_err(|_| WitnessCallError::NativeReturn { code: -1 })
    }
}

#[derive(Debug)]
enum NativeGuestBackendError {
    MissingGuestImage,
    MissingGuestImageInfo,
    GuestImageIo(std::io::Error),
    GuestMemory(GuestMemoryError),
    GuestRun(GuestMachineRunError),
    OutputOverflow {
        produced_len: usize,
        output_len: usize,
    },
}

impl fmt::Display for NativeGuestBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingGuestImage => write!(f, "native guest backend missing guest image path"),
            Self::MissingGuestImageInfo => {
                write!(f, "native guest backend missing guest image metadata")
            }
            Self::GuestImageIo(error) => write!(f, "native guest backend guest image read failed: {error}"),
            Self::GuestMemory(error) => {
                write!(f, "native guest backend guest memory load failed: {error}")
            }
            Self::GuestRun(error) => write!(f, "native guest backend guest run failed: {error}"),
            Self::OutputOverflow {
                produced_len,
                output_len,
            } => write!(
                f,
                "native guest backend trace exceeds output buffer: produced {produced_len}, output {output_len}"
            ),
        }
    }
}

impl std::error::Error for NativeGuestBackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::GuestImageIo(error) => Some(error),
            Self::GuestMemory(error) => Some(error),
            Self::GuestRun(error) => Some(error),
            Self::MissingGuestImage | Self::MissingGuestImageInfo | Self::OutputOverflow { .. } => {
                None
            }
        }
    }
}

fn compute_native_guest_trace(
    instruction_limit: u64,
    context: WitnessComputeContext<'_>,
    buffers: &mut WitnessTraceBuffers<'_>,
) -> Result<WitnessTraceOutput, NativeGuestBackendError> {
    let guest_image = context
        .guest_image
        .ok_or(NativeGuestBackendError::MissingGuestImage)?;
    let guest_image_info = context
        .guest_image_info
        .ok_or(NativeGuestBackendError::MissingGuestImageInfo)?;
    let guest_image_bytes =
        std::fs::read(guest_image).map_err(NativeGuestBackendError::GuestImageIo)?;
    let memory_image = load_guest_memory_image(&guest_image_bytes, guest_image_info)
        .map_err(NativeGuestBackendError::GuestMemory)?;
    let mut memory = GuestMachineMemory::from_image(&memory_image);
    let mut state = crate::guest_machine::GuestMachineState::new(memory.entry_address());
    let trace = run_guest_machine_trace(&mut memory, &mut state, instruction_limit)
        .map_err(NativeGuestBackendError::GuestRun)?;
    let produced_len =
        trace
            .reports
            .len()
            .checked_mul(16)
            .ok_or(NativeGuestBackendError::OutputOverflow {
                produced_len: usize::MAX,
                output_len: buffers.output().len(),
            })?;
    if produced_len > buffers.output().len() {
        return Err(NativeGuestBackendError::OutputOverflow {
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

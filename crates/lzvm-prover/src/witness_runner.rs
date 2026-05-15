use std::fmt;

use crate::witness_loader::{LoadedWitnessLibrary, WitnessCallError, WitnessTraceBuffers};
use crate::witness_trace::{parse_witness_trace, WitnessTraceBuffer, WitnessTraceError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceRequest {
    pub input: Vec<u8>,
    pub rows: usize,
    pub columns: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessTraceRunError {
    TraceByteLengthOverflow,
    Call(WitnessCallError),
    Trace(WitnessTraceError),
}

impl fmt::Display for WitnessTraceRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TraceByteLengthOverflow => write!(f, "witness trace byte length overflow"),
            Self::Call(error) => write!(f, "witness native call failed: {error}"),
            Self::Trace(error) => write!(f, "witness trace parse failed: {error}"),
        }
    }
}

impl std::error::Error for WitnessTraceRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Call(error) => Some(error),
            Self::Trace(error) => Some(error),
            Self::TraceByteLengthOverflow => None,
        }
    }
}

impl From<WitnessCallError> for WitnessTraceRunError {
    fn from(error: WitnessCallError) -> Self {
        Self::Call(error)
    }
}

impl From<WitnessTraceError> for WitnessTraceRunError {
    fn from(error: WitnessTraceError) -> Self {
        Self::Trace(error)
    }
}

pub fn run_witness_trace(
    library: &LoadedWitnessLibrary,
    request: WitnessTraceRequest,
) -> Result<WitnessTraceBuffer, WitnessTraceRunError> {
    let output_len = trace_output_byte_len(request.rows, request.columns)?;
    let mut buffers = WitnessTraceBuffers::new(request.input, output_len)?;
    let output = library.compute(&mut buffers)?;
    Ok(parse_witness_trace(
        &buffers.output()[..output.produced_len],
        request.rows,
        request.columns,
    )?)
}

fn trace_output_byte_len(rows: usize, columns: usize) -> Result<usize, WitnessTraceRunError> {
    if rows == 0 {
        return Err(WitnessTraceError::ZeroRows.into());
    }
    if columns == 0 {
        return Err(WitnessTraceError::ZeroColumns.into());
    }
    let elements = rows
        .checked_mul(columns)
        .ok_or(WitnessTraceError::ElementCountOverflow)?;
    elements
        .checked_mul(8)
        .ok_or(WitnessTraceRunError::TraceByteLengthOverflow)
}

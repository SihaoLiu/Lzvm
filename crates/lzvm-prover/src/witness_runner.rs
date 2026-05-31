use std::borrow::Cow;
use std::fmt;

use crate::witness_loader::{
    WitnessBackend, WitnessCallError, WitnessComputeContext, WitnessTraceBuffers,
};
use crate::witness_trace::{parse_witness_trace, WitnessTraceBuffer, WitnessTraceError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceRequest<'a> {
    pub input: Cow<'a, [u8]>,
    pub rows: usize,
    pub columns: usize,
}

impl WitnessTraceRequest<'static> {
    pub fn new(input: Vec<u8>, rows: usize, columns: usize) -> Self {
        Self {
            input: Cow::Owned(input),
            rows,
            columns,
        }
    }
}

impl<'a> WitnessTraceRequest<'a> {
    pub fn borrowed(input: &'a [u8], rows: usize, columns: usize) -> Self {
        Self {
            input: Cow::Borrowed(input),
            rows,
            columns,
        }
    }
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
    backend: &(impl WitnessBackend + ?Sized),
    request: WitnessTraceRequest<'_>,
) -> Result<WitnessTraceBuffer, WitnessTraceRunError> {
    run_witness_trace_with_context(backend, WitnessComputeContext::empty(), request)
}

pub fn run_witness_trace_with_context(
    backend: &(impl WitnessBackend + ?Sized),
    context: WitnessComputeContext<'_>,
    request: WitnessTraceRequest<'_>,
) -> Result<WitnessTraceBuffer, WitnessTraceRunError> {
    let output_len = trace_output_byte_len(request.rows, request.columns)?;
    let mut buffers = WitnessTraceBuffers::new(request.input, output_len)?;
    let output = backend.compute_with_context(context, &mut buffers)?;
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

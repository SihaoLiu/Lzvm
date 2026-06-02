#[cfg(test)]
use std::cell::Cell;
use std::fmt;

use lzvm_field::{Felt, FieldError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceBuffer {
    rows: usize,
    columns: usize,
    values: Vec<Felt>,
}

impl WitnessTraceBuffer {
    pub fn from_values(
        rows: usize,
        columns: usize,
        values: Vec<Felt>,
    ) -> Result<Self, WitnessTraceError> {
        validate_trace_shape(rows, columns, values.len())?;
        Ok(Self {
            rows,
            columns,
            values,
        })
    }

    pub fn row_count(&self) -> usize {
        self.rows
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn value(&self, row: usize, column: usize) -> Option<Felt> {
        #[cfg(test)]
        TRACE_VALUE_LOOKUP_COUNT.with(|count| count.set(count.get() + 1));

        if row >= self.rows || column >= self.columns {
            return None;
        }
        Some(self.values[row * self.columns + column])
    }

    pub fn values(&self) -> &[Felt] {
        &self.values
    }
}

#[cfg(test)]
thread_local! {
    static TRACE_VALUE_LOOKUP_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_trace_value_lookup_count() {
    TRACE_VALUE_LOOKUP_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn trace_value_lookup_count() -> usize {
    TRACE_VALUE_LOOKUP_COUNT.with(Cell::get)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessTraceError {
    ZeroRows,
    ZeroColumns,
    ByteLengthNotElementAligned { byte_len: usize },
    ElementCountMismatch { expected: usize, found: usize },
    ElementCountOverflow,
    NonCanonicalElement { index: usize, value: u64 },
}

impl fmt::Display for WitnessTraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroRows => write!(f, "witness trace row count is zero"),
            Self::ZeroColumns => write!(f, "witness trace column count is zero"),
            Self::ByteLengthNotElementAligned { byte_len } => write!(
                f,
                "witness trace byte length is not element aligned: {byte_len}"
            ),
            Self::ElementCountMismatch { expected, found } => write!(
                f,
                "witness trace element count mismatch: expected {expected}, found {found}"
            ),
            Self::ElementCountOverflow => write!(f, "witness trace element count overflow"),
            Self::NonCanonicalElement { index, value } => write!(
                f,
                "witness trace element is non-canonical at index {index}: {value}"
            ),
        }
    }
}

impl std::error::Error for WitnessTraceError {}

pub fn parse_witness_trace(
    bytes: &[u8],
    rows: usize,
    columns: usize,
) -> Result<WitnessTraceBuffer, WitnessTraceError> {
    if rows == 0 {
        return Err(WitnessTraceError::ZeroRows);
    }
    if columns == 0 {
        return Err(WitnessTraceError::ZeroColumns);
    }
    if !bytes.len().is_multiple_of(8) {
        return Err(WitnessTraceError::ByteLengthNotElementAligned {
            byte_len: bytes.len(),
        });
    }

    let found = bytes.len() / 8;
    validate_trace_shape(rows, columns, found)?;

    let mut values = Vec::with_capacity(found);
    for (index, chunk) in bytes.chunks_exact(8).enumerate() {
        let raw = u64::from_le_bytes(chunk.try_into().expect("chunk length checked"));
        let value = Felt::from_canonical(raw).map_err(|error| match error {
            FieldError::NonCanonical { value } => {
                WitnessTraceError::NonCanonicalElement { index, value }
            }
        })?;
        values.push(value);
    }

    Ok(WitnessTraceBuffer::from_values(rows, columns, values).expect("shape already validated"))
}

fn validate_trace_shape(
    rows: usize,
    columns: usize,
    found: usize,
) -> Result<(), WitnessTraceError> {
    if rows == 0 {
        return Err(WitnessTraceError::ZeroRows);
    }
    if columns == 0 {
        return Err(WitnessTraceError::ZeroColumns);
    }
    let expected = rows
        .checked_mul(columns)
        .ok_or(WitnessTraceError::ElementCountOverflow)?;
    if found != expected {
        return Err(WitnessTraceError::ElementCountMismatch { expected, found });
    }
    Ok(())
}

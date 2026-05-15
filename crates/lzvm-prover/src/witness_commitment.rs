use std::fmt;

use lzvm_field::{coset_extend_evaluations, DomainError};

use crate::witness_layout::WitnessTraceStageValues;

const WORD_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessStageLeaves {
    stage_index: usize,
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    bytes: Vec<u8>,
}

impl WitnessStageLeaves {
    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn source_row_count(&self) -> usize {
        self.source_rows
    }

    pub fn extended_row_count(&self) -> usize {
        self.extended_rows
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessStageLeafError {
    Domain(DomainError),
    LengthOverflow,
}

impl fmt::Display for WitnessStageLeafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => write!(f, "witness stage leaf domain error: {error}"),
            Self::LengthOverflow => write!(f, "witness stage leaf length overflow"),
        }
    }
}

impl std::error::Error for WitnessStageLeafError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::LengthOverflow => None,
        }
    }
}

impl From<DomainError> for WitnessStageLeafError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

pub fn extend_witness_stage_leaves(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
) -> Result<WitnessStageLeaves, WitnessStageLeafError> {
    let columns = stage.column_count();
    let rows = stage.row_count();
    let mut extended_columns = Vec::with_capacity(columns);
    for column in 0..columns {
        let mut source = Vec::with_capacity(rows);
        for row in 0..rows {
            let index = row
                .checked_mul(columns)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(WitnessStageLeafError::LengthOverflow)?;
            source.push(stage.values()[index]);
        }
        extended_columns.push(coset_extend_evaluations(&source, source_bits, target_bits)?);
    }

    let extended_rows = extended_columns.first().map_or(0, Vec::len);
    let byte_count = extended_rows
        .checked_mul(columns)
        .and_then(|count| count.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    let mut bytes = Vec::with_capacity(byte_count);
    for row in 0..extended_rows {
        for column_values in &extended_columns {
            bytes.extend_from_slice(&column_values[row].to_le_bytes());
        }
    }

    Ok(WitnessStageLeaves {
        stage_index: stage.stage_index(),
        source_rows: rows,
        extended_rows,
        columns,
        bytes,
    })
}

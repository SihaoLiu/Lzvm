use std::fmt;

use crate::witness_runner::WitnessTraceRequest;
use crate::ProveUnitSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceStageLayout {
    pub stage_index: usize,
    pub start_column: usize,
    pub width: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceLayout {
    rows: usize,
    columns: usize,
    stages: Vec<WitnessTraceStageLayout>,
}

impl WitnessTraceLayout {
    pub fn row_count(&self) -> usize {
        self.rows
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn stages(&self) -> &[WitnessTraceStageLayout] {
        &self.stages
    }

    pub fn request(&self, input: Vec<u8>) -> WitnessTraceRequest {
        WitnessTraceRequest {
            input,
            rows: self.rows,
            columns: self.columns,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessTraceLayoutError {
    RowCountTooLarge { rows: u64 },
    EmptyStageSet,
    ZeroStageWidth { stage_index: usize },
    ColumnCountOverflow,
}

impl fmt::Display for WitnessTraceLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RowCountTooLarge { rows } => {
                write!(f, "witness trace row count is too large: {rows}")
            }
            Self::EmptyStageSet => write!(f, "witness trace stage set is empty"),
            Self::ZeroStageWidth { stage_index } => {
                write!(f, "witness trace stage width is zero: {stage_index}")
            }
            Self::ColumnCountOverflow => write!(f, "witness trace column count overflow"),
        }
    }
}

impl std::error::Error for WitnessTraceLayoutError {}

pub fn derive_witness_trace_layout(
    unit: &ProveUnitSchedule,
) -> Result<WitnessTraceLayout, WitnessTraceLayoutError> {
    let rows = usize::try_from(unit.base_domain_size).map_err(|_| {
        WitnessTraceLayoutError::RowCountTooLarge {
            rows: unit.base_domain_size,
        }
    })?;
    if unit.stage_commit_widths.is_empty() {
        return Err(WitnessTraceLayoutError::EmptyStageSet);
    }

    let mut columns = 0_usize;
    let mut stages = Vec::with_capacity(unit.stage_commit_widths.len());
    for (index, width) in unit.stage_commit_widths.iter().enumerate() {
        let stage_index = index + 1;
        if *width == 0 {
            return Err(WitnessTraceLayoutError::ZeroStageWidth { stage_index });
        }
        let width =
            usize::try_from(*width).map_err(|_| WitnessTraceLayoutError::ColumnCountOverflow)?;
        let start_column = columns;
        columns = columns
            .checked_add(width)
            .ok_or(WitnessTraceLayoutError::ColumnCountOverflow)?;
        stages.push(WitnessTraceStageLayout {
            stage_index,
            start_column,
            width,
        });
    }

    Ok(WitnessTraceLayout {
        rows,
        columns,
        stages,
    })
}

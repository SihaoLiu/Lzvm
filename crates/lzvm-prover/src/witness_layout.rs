use std::borrow::Cow;
use std::fmt;

use lzvm_field::Felt;

use crate::witness_runner::WitnessTraceRequest;
use crate::witness_trace::WitnessTraceBuffer;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceStageValues {
    stage_index: usize,
    rows: usize,
    columns: usize,
    values: Vec<Felt>,
}

impl WitnessTraceStageValues {
    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn row_count(&self) -> usize {
        self.rows
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn values(&self) -> &[Felt] {
        &self.values
    }
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

    pub fn request<'a>(&self, input: impl Into<Cow<'a, [u8]>>) -> WitnessTraceRequest<'a> {
        WitnessTraceRequest {
            input: input.into(),
            rows: self.rows,
            columns: self.columns,
        }
    }

    pub fn stage_trace(
        &self,
        trace: &WitnessTraceBuffer,
        stage_index: usize,
    ) -> Result<WitnessTraceStageValues, WitnessTraceLayoutError> {
        if trace.row_count() != self.rows || trace.column_count() != self.columns {
            return Err(WitnessTraceLayoutError::TraceShapeMismatch {
                expected_rows: self.rows,
                expected_columns: self.columns,
                found_rows: trace.row_count(),
                found_columns: trace.column_count(),
            });
        }
        let stage = self
            .stages
            .iter()
            .find(|stage| stage.stage_index == stage_index)
            .ok_or(WitnessTraceLayoutError::UnknownStage { stage_index })?;
        let value_count = self
            .rows
            .checked_mul(stage.width)
            .ok_or(WitnessTraceLayoutError::StageValueCountOverflow)?;
        let mut values = Vec::with_capacity(value_count);
        for row in 0..self.rows {
            for column in stage.start_column..stage.start_column + stage.width {
                values.push(trace.value(row, column).expect("trace shape checked"));
            }
        }
        Ok(WitnessTraceStageValues {
            stage_index,
            rows: self.rows,
            columns: stage.width,
            values,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessTraceLayoutError {
    RowCountTooLarge {
        rows: u64,
    },
    EmptyStageSet,
    ZeroStageWidth {
        stage_index: usize,
    },
    ColumnCountOverflow,
    UnknownStage {
        stage_index: usize,
    },
    TraceShapeMismatch {
        expected_rows: usize,
        expected_columns: usize,
        found_rows: usize,
        found_columns: usize,
    },
    StageValueCountOverflow,
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
            Self::UnknownStage { stage_index } => {
                write!(f, "witness trace stage is unknown: {stage_index}")
            }
            Self::TraceShapeMismatch {
                expected_rows,
                expected_columns,
                found_rows,
                found_columns,
            } => write!(
                f,
                "witness trace shape mismatch: expected {expected_rows}x{expected_columns}, found {found_rows}x{found_columns}"
            ),
            Self::StageValueCountOverflow => write!(f, "witness trace stage value count overflow"),
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

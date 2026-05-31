use std::borrow::Cow;
use std::fmt;

use lzvm_artifacts::setup_info::CommitmentColumn;
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
pub struct WitnessTraceColumnLayout {
    name: String,
    stage_index: usize,
    stage_position: usize,
    trace_column: usize,
    dimension: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceLayout {
    rows: usize,
    columns: usize,
    stages: Vec<WitnessTraceStageLayout>,
    commitment_columns: Vec<WitnessTraceColumnLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceStageValues {
    stage_index: usize,
    rows: usize,
    columns: usize,
    values: Vec<Felt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceBuilder<'a> {
    layout: &'a WitnessTraceLayout,
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

impl WitnessTraceColumnLayout {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn stage_position(&self) -> usize {
        self.stage_position
    }

    pub fn trace_column(&self) -> usize {
        self.trace_column
    }

    pub fn dimension(&self) -> usize {
        self.dimension
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

    pub fn columns(&self) -> &[WitnessTraceColumnLayout] {
        &self.commitment_columns
    }

    pub fn column(&self, stage_index: usize, name: &str) -> Option<&WitnessTraceColumnLayout> {
        self.commitment_columns
            .iter()
            .find(|column| column.stage_index == stage_index && column.name == name)
    }

    pub fn trace_builder(&self) -> Result<WitnessTraceBuilder<'_>, WitnessTraceBuildError> {
        let value_count = self
            .rows
            .checked_mul(self.columns)
            .ok_or(WitnessTraceBuildError::TraceValueCountOverflow)?;
        Ok(WitnessTraceBuilder {
            layout: self,
            values: vec![Felt::ZERO; value_count],
        })
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

impl WitnessTraceBuilder<'_> {
    pub fn write_column_values(
        &mut self,
        row: usize,
        stage_index: usize,
        name: &str,
        values: &[Felt],
    ) -> Result<(), WitnessTraceBuildError> {
        if row >= self.layout.rows {
            return Err(WitnessTraceBuildError::RowOutOfRange {
                row,
                rows: self.layout.rows,
            });
        }
        let column = self.layout.column(stage_index, name).ok_or_else(|| {
            WitnessTraceBuildError::UnknownColumn {
                stage_index,
                name: name.to_owned(),
            }
        })?;
        if values.len() != column.dimension {
            return Err(WitnessTraceBuildError::ColumnValueCountMismatch {
                stage_index,
                name: name.to_owned(),
                expected: column.dimension,
                found: values.len(),
            });
        }
        let row_start = row
            .checked_mul(self.layout.columns)
            .ok_or(WitnessTraceBuildError::TraceValueCountOverflow)?;
        let start = row_start
            .checked_add(column.trace_column)
            .ok_or(WitnessTraceBuildError::TraceValueCountOverflow)?;
        let end = start
            .checked_add(column.dimension)
            .ok_or(WitnessTraceBuildError::TraceValueCountOverflow)?;
        self.values[start..end].copy_from_slice(values);
        Ok(())
    }

    pub fn build(self) -> WitnessTraceBuffer {
        WitnessTraceBuffer::from_values(self.layout.rows, self.layout.columns, self.values)
            .expect("builder allocates a valid trace shape")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessTraceBuildError {
    TraceValueCountOverflow,
    RowOutOfRange {
        row: usize,
        rows: usize,
    },
    UnknownColumn {
        stage_index: usize,
        name: String,
    },
    ColumnValueCountMismatch {
        stage_index: usize,
        name: String,
        expected: usize,
        found: usize,
    },
}

impl fmt::Display for WitnessTraceBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TraceValueCountOverflow => write!(f, "witness trace builder value count overflow"),
            Self::RowOutOfRange { row, rows } => {
                write!(f, "witness trace builder row {row} is outside row count {rows}")
            }
            Self::UnknownColumn { stage_index, name } => write!(
                f,
                "witness trace builder column {name} is unknown in stage {stage_index}"
            ),
            Self::ColumnValueCountMismatch {
                stage_index,
                name,
                expected,
                found,
            } => write!(
                f,
                "witness trace builder column {name} in stage {stage_index} expected {expected} values, found {found}"
            ),
        }
    }
}

impl std::error::Error for WitnessTraceBuildError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessTraceLayoutError {
    RowCountTooLarge {
        rows: u64,
    },
    ZeroRows,
    EmptyStageSet,
    ZeroStageWidth {
        stage_index: usize,
    },
    ColumnCountOverflow,
    CommitmentColumnStageOutOfRange {
        name: String,
        stage_index: usize,
        stage_count: usize,
    },
    ZeroCommitmentColumnDimension {
        name: String,
        stage_index: usize,
    },
    CommitmentColumnPositionOutOfRange {
        name: String,
        stage_index: usize,
        stage_position: usize,
        dimension: usize,
        stage_width: usize,
    },
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
            Self::ZeroRows => write!(f, "witness trace row count is zero"),
            Self::EmptyStageSet => write!(f, "witness trace stage set is empty"),
            Self::ZeroStageWidth { stage_index } => {
                write!(f, "witness trace stage width is zero: {stage_index}")
            }
            Self::ColumnCountOverflow => write!(f, "witness trace column count overflow"),
            Self::CommitmentColumnStageOutOfRange {
                name,
                stage_index,
                stage_count,
            } => write!(
                f,
                "witness trace commitment column {name} references stage {stage_index}, but stage count is {stage_count}"
            ),
            Self::ZeroCommitmentColumnDimension { name, stage_index } => write!(
                f,
                "witness trace commitment column {name} in stage {stage_index} has zero dimension"
            ),
            Self::CommitmentColumnPositionOutOfRange {
                name,
                stage_index,
                stage_position,
                dimension,
                stage_width,
            } => write!(
                f,
                "witness trace commitment column {name} in stage {stage_index} spans position {stage_position} with dimension {dimension}, but stage width is {stage_width}"
            ),
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
    if rows == 0 {
        return Err(WitnessTraceLayoutError::ZeroRows);
    }
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

    let commitment_columns = derive_witness_trace_columns(unit, &stages)?;

    Ok(WitnessTraceLayout {
        rows,
        columns,
        stages,
        commitment_columns,
    })
}

fn derive_witness_trace_columns(
    unit: &ProveUnitSchedule,
    stages: &[WitnessTraceStageLayout],
) -> Result<Vec<WitnessTraceColumnLayout>, WitnessTraceLayoutError> {
    unit.commitment_columns
        .iter()
        .map(|column| derive_witness_trace_column(column, stages))
        .collect()
}

fn derive_witness_trace_column(
    column: &CommitmentColumn,
    stages: &[WitnessTraceStageLayout],
) -> Result<WitnessTraceColumnLayout, WitnessTraceLayoutError> {
    let stage_index =
        usize::try_from(column.stage).map_err(|_| WitnessTraceLayoutError::ColumnCountOverflow)?;
    let stage_position = usize::try_from(column.stage_position)
        .map_err(|_| WitnessTraceLayoutError::ColumnCountOverflow)?;
    let dimension = usize::try_from(column.dimension)
        .map_err(|_| WitnessTraceLayoutError::ColumnCountOverflow)?;
    let stage = stages
        .iter()
        .find(|stage| stage.stage_index == stage_index)
        .ok_or_else(
            || WitnessTraceLayoutError::CommitmentColumnStageOutOfRange {
                name: column.name.clone(),
                stage_index,
                stage_count: stages.len(),
            },
        )?;
    if dimension == 0 {
        return Err(WitnessTraceLayoutError::ZeroCommitmentColumnDimension {
            name: column.name.clone(),
            stage_index,
        });
    }
    let end_position = stage_position
        .checked_add(dimension)
        .ok_or(WitnessTraceLayoutError::ColumnCountOverflow)?;
    if end_position > stage.width {
        return Err(
            WitnessTraceLayoutError::CommitmentColumnPositionOutOfRange {
                name: column.name.clone(),
                stage_index,
                stage_position,
                dimension,
                stage_width: stage.width,
            },
        );
    }
    let trace_column = stage
        .start_column
        .checked_add(stage_position)
        .ok_or(WitnessTraceLayoutError::ColumnCountOverflow)?;

    Ok(WitnessTraceColumnLayout {
        name: column.name.clone(),
        stage_index,
        stage_position,
        trace_column,
        dimension,
    })
}

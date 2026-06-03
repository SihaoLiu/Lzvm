use std::borrow::Cow;
#[cfg(test)]
use std::cell::Cell;
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedTraceColumn<'a> {
    layout: &'a WitnessTraceLayout,
    stage_index: usize,
    trace_column: usize,
    dimension: usize,
    name: &'a str,
}

impl PartialEq for ResolvedTraceColumn<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.layout, other.layout)
            && self.stage_index == other.stage_index
            && self.trace_column == other.trace_column
            && self.dimension == other.dimension
            && self.name == other.name
    }
}

impl Eq for ResolvedTraceColumn<'_> {}

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
    #[cfg(test)]
    pub(crate) fn new_for_test(
        stage_index: usize,
        rows: usize,
        columns: usize,
        values: Vec<Felt>,
    ) -> Self {
        Self {
            stage_index,
            rows,
            columns,
            values,
        }
    }

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

impl ResolvedTraceColumn<'_> {
    pub(crate) fn name(&self) -> &str {
        self.name
    }

    pub(crate) fn trace_column(&self) -> usize {
        self.trace_column
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
        #[cfg(test)]
        COLUMN_LOOKUP_COUNT.with(|count| count.set(count.get() + 1));

        self.commitment_columns
            .iter()
            .find(|column| column.stage_index == stage_index && column.name == name)
    }

    pub(crate) fn resolved_column<'a>(
        &'a self,
        column: &'a WitnessTraceColumnLayout,
    ) -> ResolvedTraceColumn<'a> {
        assert!(
            self.commitment_columns
                .iter()
                .any(|candidate| std::ptr::eq(candidate, column)),
            "resolved witness trace column does not belong to layout"
        );
        ResolvedTraceColumn {
            layout: self,
            stage_index: column.stage_index,
            trace_column: column.trace_column,
            dimension: column.dimension,
            name: &column.name,
        }
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
        #[cfg(test)]
        STAGE_TRACE_COUNT.with(|count| count.set(count.get() + 1));
        let stage = self
            .stages
            .iter()
            .find(|stage| stage.stage_index == stage_index)
            .ok_or(WitnessTraceLayoutError::UnknownStage { stage_index })?;
        let value_count = self
            .rows
            .checked_mul(stage.width)
            .ok_or(WitnessTraceLayoutError::StageValueCountOverflow)?;
        let stage_end = stage
            .start_column
            .checked_add(stage.width)
            .ok_or(WitnessTraceLayoutError::StageValueCountOverflow)?;
        let mut values = Vec::with_capacity(value_count);
        for row_values in trace.values().chunks_exact(self.columns) {
            values.extend_from_slice(&row_values[stage.start_column..stage_end]);
        }
        Ok(WitnessTraceStageValues {
            stage_index,
            rows: self.rows,
            columns: stage.width,
            values,
        })
    }
}

#[cfg(test)]
thread_local! {
    static COLUMN_LOOKUP_COUNT: Cell<usize> = const { Cell::new(0) };
    static GENERIC_VALUE_COPY_COUNT: Cell<usize> = const { Cell::new(0) };
    static STAGE_TRACE_COUNT: Cell<usize> = const { Cell::new(0) };
    static RESOLVED_COLUMN_VALIDATION_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_column_lookup_count() {
    COLUMN_LOOKUP_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn column_lookup_count() -> usize {
    COLUMN_LOOKUP_COUNT.with(Cell::get)
}

#[cfg(test)]
pub(crate) fn reset_generic_value_copy_count() {
    GENERIC_VALUE_COPY_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn generic_value_copy_count() -> usize {
    GENERIC_VALUE_COPY_COUNT.with(Cell::get)
}

#[cfg(test)]
pub(crate) fn reset_stage_trace_count() {
    STAGE_TRACE_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn stage_trace_count() -> usize {
    STAGE_TRACE_COUNT.with(Cell::get)
}

#[cfg(test)]
pub(crate) fn reset_resolved_column_validation_count() {
    RESOLVED_COLUMN_VALIDATION_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn resolved_column_validation_count() -> usize {
    RESOLVED_COLUMN_VALIDATION_COUNT.with(Cell::get)
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
        self.write_resolved_column_values_for_valid_row(
            row,
            &self.layout.resolved_column(column),
            values,
        )
    }

    #[cfg(test)]
    pub(crate) fn write_resolved_column_values(
        &mut self,
        row: usize,
        column: &ResolvedTraceColumn<'_>,
        values: &[Felt],
    ) -> Result<(), WitnessTraceBuildError> {
        if row >= self.layout.rows {
            return Err(WitnessTraceBuildError::RowOutOfRange {
                row,
                rows: self.layout.rows,
            });
        }
        self.write_resolved_column_values_for_valid_row(row, column, values)
    }

    #[cfg(test)]
    pub(crate) fn write_resolved_scalar_value(
        &mut self,
        row: usize,
        column: &ResolvedTraceColumn<'_>,
        value: Felt,
    ) -> Result<(), WitnessTraceBuildError> {
        if row >= self.layout.rows {
            return Err(WitnessTraceBuildError::RowOutOfRange {
                row,
                rows: self.layout.rows,
            });
        }
        self.validate_resolved_column(column, 1)?;
        let (start, _) = self.resolved_column_bounds_for_valid_row(row, column)?;
        self.values[start] = value;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn write_resolved_pair_values(
        &mut self,
        row: usize,
        column: &ResolvedTraceColumn<'_>,
        values: [Felt; 2],
    ) -> Result<(), WitnessTraceBuildError> {
        if row >= self.layout.rows {
            return Err(WitnessTraceBuildError::RowOutOfRange {
                row,
                rows: self.layout.rows,
            });
        }
        self.validate_resolved_column(column, 2)?;
        let (start, _) = self.resolved_column_bounds_for_valid_row(row, column)?;
        self.values[start] = values[0];
        self.values[start + 1] = values[1];
        Ok(())
    }

    pub(crate) fn write_trusted_resolved_scalar_value(
        &mut self,
        row: usize,
        column: &ResolvedTraceColumn<'_>,
        value: Felt,
    ) -> Result<(), WitnessTraceBuildError> {
        if row >= self.layout.rows {
            return Err(WitnessTraceBuildError::RowOutOfRange {
                row,
                rows: self.layout.rows,
            });
        }
        debug_assert!(std::ptr::eq(column.layout, self.layout));
        debug_assert_eq!(column.dimension, 1);
        let (start, _) = self.resolved_column_bounds_for_valid_row(row, column)?;
        self.values[start] = value;
        Ok(())
    }

    pub(crate) fn write_trusted_resolved_pair_values(
        &mut self,
        row: usize,
        column: &ResolvedTraceColumn<'_>,
        values: [Felt; 2],
    ) -> Result<(), WitnessTraceBuildError> {
        if row >= self.layout.rows {
            return Err(WitnessTraceBuildError::RowOutOfRange {
                row,
                rows: self.layout.rows,
            });
        }
        debug_assert!(std::ptr::eq(column.layout, self.layout));
        debug_assert_eq!(column.dimension, 2);
        let (start, _) = self.resolved_column_bounds_for_valid_row(row, column)?;
        self.values[start] = values[0];
        self.values[start + 1] = values[1];
        Ok(())
    }

    fn write_resolved_column_values_for_valid_row(
        &mut self,
        row: usize,
        column: &ResolvedTraceColumn<'_>,
        values: &[Felt],
    ) -> Result<(), WitnessTraceBuildError> {
        self.validate_resolved_column(column, values.len())?;
        let (start, end) = self.resolved_column_bounds_for_valid_row(row, column)?;
        match values {
            [value] => self.values[start] = *value,
            [first, second] => {
                self.values[start] = *first;
                self.values[start + 1] = *second;
            }
            [first, second, third] => {
                self.values[start] = *first;
                self.values[start + 1] = *second;
                self.values[start + 2] = *third;
            }
            _ => {
                #[cfg(test)]
                GENERIC_VALUE_COPY_COUNT.with(|count| count.set(count.get() + 1));
                self.values[start..end].copy_from_slice(values);
            }
        }
        Ok(())
    }

    fn validate_resolved_column(
        &self,
        column: &ResolvedTraceColumn<'_>,
        found: usize,
    ) -> Result<(), WitnessTraceBuildError> {
        #[cfg(test)]
        RESOLVED_COLUMN_VALIDATION_COUNT.with(|count| count.set(count.get() + 1));
        if !std::ptr::eq(column.layout, self.layout) {
            return Err(WitnessTraceBuildError::UnknownColumn {
                stage_index: column.stage_index,
                name: column.name.to_owned(),
            });
        }
        if found != column.dimension {
            return Err(WitnessTraceBuildError::ColumnValueCountMismatch {
                stage_index: column.stage_index,
                name: column.name.to_owned(),
                expected: column.dimension,
                found,
            });
        }
        Ok(())
    }

    fn resolved_column_bounds_for_valid_row(
        &self,
        row: usize,
        column: &ResolvedTraceColumn<'_>,
    ) -> Result<(usize, usize), WitnessTraceBuildError> {
        debug_assert!(column
            .trace_column
            .checked_add(column.dimension)
            .is_some_and(|end| end <= self.layout.columns));
        let row_start = row
            .checked_mul(self.layout.columns)
            .ok_or(WitnessTraceBuildError::TraceValueCountOverflow)?;
        let start = row_start
            .checked_add(column.trace_column)
            .ok_or(WitnessTraceBuildError::TraceValueCountOverflow)?;
        let end = start
            .checked_add(column.dimension)
            .ok_or(WitnessTraceBuildError::TraceValueCountOverflow)?;
        Ok((start, end))
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
    CommitmentColumnOverlap {
        first_name: String,
        second_name: String,
        stage_index: usize,
        trace_column: usize,
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
            Self::CommitmentColumnOverlap {
                first_name,
                second_name,
                stage_index,
                trace_column,
            } => write!(
                f,
                "witness trace commitment columns {first_name} and {second_name} overlap in stage {stage_index} at trace column {trace_column}"
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
    let mut columns = Vec::with_capacity(unit.commitment_columns.len());
    for source in &unit.commitment_columns {
        let column = derive_witness_trace_column(source, stages)?;
        for existing in &columns {
            if let Some(trace_column) = overlapping_trace_column(existing, &column) {
                return Err(WitnessTraceLayoutError::CommitmentColumnOverlap {
                    first_name: existing.name.clone(),
                    second_name: column.name.clone(),
                    stage_index: column.stage_index,
                    trace_column,
                });
            }
        }
        columns.push(column);
    }
    Ok(columns)
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

fn overlapping_trace_column(
    left: &WitnessTraceColumnLayout,
    right: &WitnessTraceColumnLayout,
) -> Option<usize> {
    if left.stage_index != right.stage_index {
        return None;
    }
    let left_end = left.trace_column.checked_add(left.dimension)?;
    let right_end = right.trace_column.checked_add(right.dimension)?;
    if left.trace_column < right_end && right.trace_column < left_end {
        Some(left.trace_column.max(right.trace_column))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_trace_copies_contiguous_rows_without_value_lookup() {
        let layout = WitnessTraceLayout {
            rows: 3,
            columns: 5,
            stages: vec![
                WitnessTraceStageLayout {
                    stage_index: 1,
                    start_column: 0,
                    width: 2,
                },
                WitnessTraceStageLayout {
                    stage_index: 2,
                    start_column: 2,
                    width: 3,
                },
            ],
            commitment_columns: Vec::new(),
        };
        let values = (0_u64..15).map(Felt::from_u64).collect::<Vec<_>>();
        let trace = WitnessTraceBuffer::from_values(3, 5, values).expect("trace shape is valid");

        crate::witness_trace::reset_trace_value_lookup_count();
        let stage = layout
            .stage_trace(&trace, 2)
            .expect("stage should be present");

        let expected = [2_u64, 3, 4, 7, 8, 9, 12, 13, 14].map(Felt::from_u64);
        assert_eq!(stage.stage_index(), 2);
        assert_eq!(stage.row_count(), 3);
        assert_eq!(stage.column_count(), 3);
        assert_eq!(stage.values(), expected.as_slice());
        assert_eq!(crate::witness_trace::trace_value_lookup_count(), 0);
    }

    #[test]
    fn small_writes_store_values_without_generic_copy() {
        let layout = WitnessTraceLayout {
            rows: 2,
            columns: 8,
            stages: vec![
                WitnessTraceStageLayout {
                    stage_index: 1,
                    start_column: 0,
                    width: 3,
                },
                WitnessTraceStageLayout {
                    stage_index: 2,
                    start_column: 3,
                    width: 5,
                },
            ],
            commitment_columns: vec![
                WitnessTraceColumnLayout {
                    name: "scalar".to_owned(),
                    stage_index: 1,
                    stage_position: 1,
                    trace_column: 1,
                    dimension: 1,
                },
                WitnessTraceColumnLayout {
                    name: "wide".to_owned(),
                    stage_index: 2,
                    stage_position: 0,
                    trace_column: 3,
                    dimension: 2,
                },
                WitnessTraceColumnLayout {
                    name: "extension".to_owned(),
                    stage_index: 2,
                    stage_position: 2,
                    trace_column: 5,
                    dimension: 3,
                },
            ],
        };
        let scalar = layout.resolved_column(&layout.commitment_columns[0]);
        let wide = layout.resolved_column(&layout.commitment_columns[1]);
        let extension = layout.resolved_column(&layout.commitment_columns[2]);
        let mut builder = layout.trace_builder().expect("builder should allocate");

        reset_generic_value_copy_count();
        builder
            .write_resolved_scalar_value(0, &scalar, Felt::from_u64(7))
            .expect("scalar value should write");
        builder
            .write_resolved_pair_values(1, &wide, [Felt::from_u64(11), Felt::from_u64(13)])
            .expect("wide value should write");
        builder
            .write_resolved_column_values(
                0,
                &extension,
                &[Felt::from_u64(17), Felt::from_u64(19), Felt::from_u64(23)],
            )
            .expect("extension value should write");

        let trace = builder.build();
        assert_eq!(trace.value(0, 1), Some(Felt::from_u64(7)));
        assert_eq!(trace.value(0, 5), Some(Felt::from_u64(17)));
        assert_eq!(trace.value(0, 6), Some(Felt::from_u64(19)));
        assert_eq!(trace.value(0, 7), Some(Felt::from_u64(23)));
        assert_eq!(trace.value(1, 3), Some(Felt::from_u64(11)));
        assert_eq!(trace.value(1, 4), Some(Felt::from_u64(13)));
        assert_eq!(generic_value_copy_count(), 0);
    }
}

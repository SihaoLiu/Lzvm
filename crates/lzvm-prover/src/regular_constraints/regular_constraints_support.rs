use super::*;
use lzvm_field::FieldError;

#[cfg(test)]
use std::cell::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BufferKind {
    Fixed,
    Stage(u16),
    CustomFixed(usize),
    DomainOrZerofier,
    Tmp1,
    Tmp3,
    Public,
    Number,
    UnitValue,
    ProofValue,
    GroupValue,
    Challenge,
    Evaluation,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BufferLayout {
    stage_count: usize,
    custom_fixed_count: usize,
}

impl BufferLayout {
    pub(super) fn new(inputs: RegularConstraintInputs<'_>) -> Self {
        Self {
            stage_count: inputs.stage_count as usize,
            custom_fixed_count: inputs.custom_fixed_columns.len(),
        }
    }

    pub(super) fn resolve(&self, buffer: u16) -> Result<BufferKind, RegularConstraintEvalError> {
        #[cfg(test)]
        BUFFER_RESOLVE_COUNT.with(|count| count.set(count.get() + 1));

        let buffer = buffer as usize;
        if buffer == 0 {
            return Ok(BufferKind::Fixed);
        }
        if buffer <= self.stage_count + 1 {
            return Ok(BufferKind::Stage(buffer as u16));
        }
        if buffer == self.stage_count + 2 {
            return Ok(BufferKind::DomainOrZerofier);
        }
        if buffer == self.stage_count + 3 {
            return Err(RegularConstraintEvalError::UnsupportedSourceBuffer {
                buffer: buffer as u16,
            });
        }
        let first_custom = self.stage_count + 4;
        let custom_end = first_custom + self.custom_fixed_count;
        if (first_custom..custom_end).contains(&buffer) {
            return Ok(BufferKind::CustomFixed(buffer - first_custom));
        }

        let base = 1 + self.stage_count + 3 + self.custom_fixed_count;
        match buffer {
            value if value == base => Ok(BufferKind::Tmp1),
            value if value == base + 1 => Ok(BufferKind::Tmp3),
            value if value == base + 2 => Ok(BufferKind::Public),
            value if value == base + 3 => Ok(BufferKind::Number),
            value if value == base + 4 => Ok(BufferKind::UnitValue),
            value if value == base + 5 => Ok(BufferKind::ProofValue),
            value if value == base + 6 => Ok(BufferKind::GroupValue),
            value if value == base + 7 => Ok(BufferKind::Challenge),
            value if value == base + 8 => Ok(BufferKind::Evaluation),
            value => Err(RegularConstraintEvalError::UnsupportedSourceBuffer {
                buffer: value as u16,
            }),
        }
    }
}

#[cfg(test)]
thread_local! {
    pub(super) static BUFFER_RESOLVE_COUNT: Cell<usize> = const { Cell::new(0) };
    pub(super) static CACHED_SOURCE_COUNT: Cell<usize> = const { Cell::new(0) };
    pub(super) static BASE_ONLY_PREPARED_ROW_COUNT: Cell<usize> = const { Cell::new(0) };
    pub(super) static BASE_ONLY_TMP1_CLEAR_COUNT: Cell<usize> = const { Cell::new(0) };
    pub(super) static BASE_ONLY_TMP3_CLEAR_COUNT: Cell<usize> = const { Cell::new(0) };
    pub(super) static BASE_ONLY_KIND_DISPATCH_COUNT: Cell<usize> = const { Cell::new(0) };
    pub(super) static PREPARED_SOURCE_ROW_OFFSET_COUNT: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn find_stage_columns(
    inputs: RegularConstraintInputs<'_>,
    stage_index: u16,
) -> Result<RegularColumnMatrix<'_>, RegularConstraintEvalError> {
    if let Some(stage_slot) = usize::from(stage_index).checked_sub(1) {
        if let Some(stage) = inputs
            .stage_columns
            .get(stage_slot)
            .filter(|stage| stage.stage_index == stage_index)
        {
            if inputs.stage_columns[..stage_slot]
                .iter()
                .all(|stage| stage.stage_index != stage_index)
            {
                return Ok(RegularColumnMatrix {
                    column_count: stage.column_count,
                    values: stage.values,
                });
            }
        }
    }

    inputs
        .stage_columns
        .iter()
        .find(|stage| stage.stage_index == stage_index)
        .map(|stage| RegularColumnMatrix {
            column_count: stage.column_count,
            values: stage.values,
        })
        .ok_or(RegularConstraintEvalError::MissingStageColumns { stage_index })
}

pub(super) fn source_row_offset(
    source: SourceRef,
    inputs: RegularConstraintInputs<'_>,
) -> Result<usize, RegularConstraintEvalError> {
    let offset = inputs
        .opening_point_offsets
        .get(source.row_offset_index)
        .copied()
        .ok_or(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer: "opening point",
            offset: source.row_offset_index,
            width: 1,
            len: inputs.opening_point_offsets.len(),
        })?;
    normalize_row_offset(offset, inputs.domain_size)
}

pub(super) fn normalize_row_offset(
    offset: i64,
    domain_size: usize,
) -> Result<usize, RegularConstraintEvalError> {
    if let Ok(domain_size) = i64::try_from(domain_size) {
        return Ok(offset.rem_euclid(domain_size) as usize);
    }

    let domain_size =
        i128::try_from(domain_size).map_err(|_| RegularConstraintEvalError::LengthOverflow)?;
    Ok(i128::from(offset).rem_euclid(domain_size) as usize)
}

pub(super) fn source_row_with_offset(
    row: usize,
    row_offset: usize,
    domain_size: usize,
) -> Result<usize, RegularConstraintEvalError> {
    if row < domain_size && row_offset < domain_size {
        if row_offset == 0 {
            return Ok(row);
        }
        let wrap_at = domain_size - row_offset;
        return if row < wrap_at {
            Ok(row + row_offset)
        } else {
            Ok(row - wrap_at)
        };
    }

    let domain_size =
        i128::try_from(domain_size).map_err(|_| RegularConstraintEvalError::LengthOverflow)?;
    let shifted = u128::try_from(row).map_err(|_| RegularConstraintEvalError::LengthOverflow)?
        + u128::try_from(row_offset).map_err(|_| RegularConstraintEvalError::LengthOverflow)?;
    Ok((shifted % domain_size as u128) as usize)
}

pub(super) fn read_domain_or_zerofier(
    offset: usize,
    row: usize,
    inputs: RegularConstraintInputs<'_>,
) -> Result<Felt, RegularConstraintEvalError> {
    if offset == 0 {
        read_felt("domain point", inputs.domain_points, row)
    } else {
        read_matrix_base("zerofier", inputs.zerofier_values, offset - 1, row)
    }
}

pub(super) fn read_matrix_base(
    buffer: &'static str,
    matrix: RegularColumnMatrix<'_>,
    column: usize,
    row: usize,
) -> Result<Felt, RegularConstraintEvalError> {
    let index = matrix_index(matrix, column, 1, row)?;
    read_felt(buffer, matrix.values, index)
}

pub(super) fn read_matrix_ext(
    buffer: &'static str,
    matrix: RegularColumnMatrix<'_>,
    column: usize,
    row: usize,
) -> Result<Ext3, RegularConstraintEvalError> {
    let index = matrix_index(matrix, column, 3, row)?;
    Ok(Ext3::new(
        read_felt(buffer, matrix.values, index)?,
        read_felt(buffer, matrix.values, index + 1)?,
        read_felt(buffer, matrix.values, index + 2)?,
    ))
}

pub(super) fn matrix_index(
    matrix: RegularColumnMatrix<'_>,
    column: usize,
    width: usize,
    row: usize,
) -> Result<usize, RegularConstraintEvalError> {
    if column
        .checked_add(width)
        .is_none_or(|end| end > matrix.column_count)
    {
        return Err(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer: "column matrix",
            offset: column,
            width,
            len: matrix.column_count,
        });
    }
    row.checked_mul(matrix.column_count)
        .and_then(|base| base.checked_add(column))
        .ok_or(RegularConstraintEvalError::LengthOverflow)
}

pub(super) fn read_felt(
    buffer: &'static str,
    values: &[Felt],
    offset: usize,
) -> Result<Felt, RegularConstraintEvalError> {
    values
        .get(offset)
        .copied()
        .ok_or(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer,
            offset,
            width: 1,
            len: values.len(),
        })
}

pub(super) fn read_felt_ext(
    buffer: &'static str,
    values: &[Felt],
    offset: usize,
) -> Result<Ext3, RegularConstraintEvalError> {
    if offset.checked_add(3).is_some_and(|end| end <= values.len()) {
        Ok(Ext3::new(
            values[offset],
            values[offset + 1],
            values[offset + 2],
        ))
    } else {
        Err(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer,
            offset,
            width: 3,
            len: values.len(),
        })
    }
}

pub(super) fn read_ext_field(
    buffer: &'static str,
    values: &[Ext3],
    offset: usize,
) -> Result<Felt, RegularConstraintEvalError> {
    let value =
        values
            .get(offset / 3)
            .ok_or(RegularConstraintEvalError::SourceIndexOutOfRange {
                buffer,
                offset,
                width: 1,
                len: values.len().saturating_mul(3),
            })?;
    Ok(match offset % 3 {
        0 => value.c0,
        1 => value.c1,
        _ => value.c2,
    })
}

pub(super) fn read_ext_fields(
    buffer: &'static str,
    values: &[Ext3],
    offset: usize,
) -> Result<Ext3, RegularConstraintEvalError> {
    let len = values.len().saturating_mul(3);
    if offset.checked_add(3).is_none_or(|end| end > len) {
        return Err(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer,
            offset,
            width: 3,
            len,
        });
    }
    Ok(Ext3::new(
        read_ext_field(buffer, values, offset)?,
        read_ext_field(buffer, values, offset + 1)?,
        read_ext_field(buffer, values, offset + 2)?,
    ))
}

pub(super) fn read_number(
    program: &ConstraintProgram,
    offset: usize,
) -> Result<Felt, RegularConstraintEvalError> {
    let value =
        *program
            .numbers
            .get(offset)
            .ok_or(RegularConstraintEvalError::SourceIndexOutOfRange {
                buffer: "number",
                offset,
                width: 1,
                len: program.numbers.len(),
            })?;
    canonical_number(value)
}

pub(super) fn read_number_ext(
    program: &ConstraintProgram,
    offset: usize,
) -> Result<Ext3, RegularConstraintEvalError> {
    if offset
        .checked_add(3)
        .is_some_and(|end| end <= program.numbers.len())
    {
        Ok(Ext3::new(
            canonical_number(program.numbers[offset])?,
            canonical_number(program.numbers[offset + 1])?,
            canonical_number(program.numbers[offset + 2])?,
        ))
    } else {
        Err(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer: "number",
            offset,
            width: 3,
            len: program.numbers.len(),
        })
    }
}

pub(super) fn write_base(
    tmp1: &mut [Felt],
    offset: usize,
    value: Felt,
) -> Result<(), RegularConstraintEvalError> {
    let len = tmp1.len();
    *tmp1
        .get_mut(offset)
        .ok_or(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer: "tmp1",
            offset,
            width: 1,
            len,
        })? = value;
    Ok(())
}

pub(super) fn write_ext(
    tmp3: &mut [Felt],
    offset: usize,
    value: Ext3,
) -> Result<(), RegularConstraintEvalError> {
    if offset.checked_add(3).is_some_and(|end| end <= tmp3.len()) {
        tmp3[offset] = value.c0;
        tmp3[offset + 1] = value.c1;
        tmp3[offset + 2] = value.c2;
        Ok(())
    } else {
        Err(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer: "tmp3",
            offset,
            width: 3,
            len: tmp3.len(),
        })
    }
}

pub(super) fn read_destination(
    entry: &ConstraintEntry,
    tmp1: &[Felt],
    tmp3: &[Felt],
) -> Result<Ext3, RegularConstraintEvalError> {
    match entry.destination_dimension {
        1 => {
            let index = to_usize(entry.destination_id)?;
            Ok(scalar_ext(read_felt("tmp1", tmp1, index)?))
        }
        3 => {
            let index = to_usize(entry.destination_id)?;
            let offset = index
                .checked_mul(3)
                .ok_or(RegularConstraintEvalError::LengthOverflow)?;
            read_felt_ext("tmp3", tmp3, offset)
        }
        dimension => Err(RegularConstraintEvalError::UnsupportedDestinationDimension { dimension }),
    }
}

pub(super) fn canonical_number(value: u64) -> Result<Felt, RegularConstraintEvalError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => {
            RegularConstraintEvalError::NonCanonicalNumber { value }
        }
    })
}

pub(super) fn scalar_ext(value: Felt) -> Ext3 {
    Ext3::new(value, Felt::ZERO, Felt::ZERO)
}

pub(super) fn to_usize(value: u32) -> Result<usize, RegularConstraintEvalError> {
    usize::try_from(value).map_err(|_| RegularConstraintEvalError::LengthOverflow)
}

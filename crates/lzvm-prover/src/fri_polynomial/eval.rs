use lzvm_artifacts::expression_program::{ExpressionEntry, ExpressionProgram};
use lzvm_field::{Ext3, Felt, FieldError};

use super::{FriPolynomialColumnMatrix, FriPolynomialError, FriPolynomialInputs};

pub fn build_fri_polynomial(
    program: &ExpressionProgram,
    expression_id: u32,
    inputs: FriPolynomialInputs<'_>,
) -> Result<Vec<Ext3>, FriPolynomialError> {
    if inputs.domain_size == 0 {
        return Err(FriPolynomialError::EmptyDomain);
    }
    validate_inputs(inputs)?;
    let entry = program
        .entries
        .iter()
        .find(|entry| entry.expression_id == expression_id)
        .ok_or(FriPolynomialError::MissingExpression { expression_id })?;
    let ops = entry_ops(entry, program)?;
    let args = entry_args(entry, program)?;
    let tmp1_len = to_usize(entry.temp1_count)?;
    let tmp3_len = to_usize(entry.temp3_count)?.saturating_mul(3);
    let mut tmp1 = vec![Felt::ZERO; tmp1_len];
    let mut tmp3 = vec![Felt::ZERO; tmp3_len];
    let mut out = Vec::with_capacity(inputs.domain_size);
    for row in 0..inputs.domain_size {
        tmp1.fill(Felt::ZERO);
        tmp3.fill(Felt::ZERO);
        out.push(evaluate_row(
            row, entry, ops, args, program, inputs, &mut tmp1, &mut tmp3,
        )?);
    }
    Ok(out)
}

fn validate_inputs(inputs: FriPolynomialInputs<'_>) -> Result<(), FriPolynomialError> {
    validate_matrix("fixed column", inputs.fixed_columns, inputs.domain_size)?;
    for stage in inputs.stage_columns {
        validate_matrix(
            "stage column",
            FriPolynomialColumnMatrix {
                column_count: stage.column_count,
                values: stage.values,
            },
            inputs.domain_size,
        )?;
    }
    for matrix in inputs.custom_fixed_columns {
        validate_matrix("custom fixed column", *matrix, inputs.domain_size)?;
    }
    if !inputs.domain_points.is_empty() && inputs.domain_points.len() != inputs.domain_size {
        return Err(FriPolynomialError::MatrixLengthMismatch {
            buffer: "domain point",
            expected: inputs.domain_size,
            found: inputs.domain_points.len(),
        });
    }
    if inputs.zerofier_values.column_count != 0 || !inputs.zerofier_values.values.is_empty() {
        validate_matrix("zerofier", inputs.zerofier_values, inputs.domain_size)?;
    }
    Ok(())
}

fn validate_matrix(
    buffer: &'static str,
    matrix: FriPolynomialColumnMatrix<'_>,
    domain_size: usize,
) -> Result<(), FriPolynomialError> {
    let expected = domain_size
        .checked_mul(matrix.column_count)
        .ok_or(FriPolynomialError::LengthOverflow)?;
    if matrix.values.len() == expected {
        Ok(())
    } else {
        Err(FriPolynomialError::MatrixLengthMismatch {
            buffer,
            expected,
            found: matrix.values.len(),
        })
    }
}

fn evaluate_row(
    row: usize,
    entry: &ExpressionEntry,
    ops: &[u8],
    args: &[u16],
    program: &ExpressionProgram,
    inputs: FriPolynomialInputs<'_>,
    tmp1: &mut [Felt],
    tmp3: &mut [Felt],
) -> Result<Ext3, FriPolynomialError> {
    let layout = BufferLayout::new(inputs);
    let mut cursor = 0usize;

    for shape in ops {
        let op_args = read_operation_args(entry.expression_id, args, cursor)?;
        cursor += 8;
        match *shape {
            0 => {
                let value = apply_base_op(
                    op_args.kind,
                    read_base(op_args.src0, row, tmp1, tmp3, program, inputs, layout)?,
                    read_base(op_args.src1, row, tmp1, tmp3, program, inputs, layout)?,
                )?;
                write_base(tmp1, op_args.destination_offset, value)?;
            }
            1 => {
                let value = apply_ext_op(
                    op_args.kind,
                    read_ext(op_args.src0, row, tmp1, tmp3, program, inputs, layout)?,
                    scalar_ext(read_base(
                        op_args.src1,
                        row,
                        tmp1,
                        tmp3,
                        program,
                        inputs,
                        layout,
                    )?),
                )?;
                write_ext(tmp3, op_args.destination_offset, value)?;
            }
            2 => {
                let value = apply_ext_op(
                    op_args.kind,
                    read_ext(op_args.src0, row, tmp1, tmp3, program, inputs, layout)?,
                    read_ext(op_args.src1, row, tmp1, tmp3, program, inputs, layout)?,
                )?;
                write_ext(tmp3, op_args.destination_offset, value)?;
            }
            shape => return Err(FriPolynomialError::UnsupportedOperationShape { shape }),
        }
    }

    if cursor != args.len() {
        return Err(FriPolynomialError::ArgumentCountMismatch {
            expression_id: entry.expression_id,
            consumed: cursor,
            declared: args.len(),
        });
    }

    read_destination(entry, tmp1, tmp3)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OperationArgs {
    kind: u16,
    destination_offset: usize,
    src0: SourceRef,
    src1: SourceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceRef {
    buffer: u16,
    offset: usize,
    row_offset_index: usize,
}

fn entry_ops<'a>(
    entry: &ExpressionEntry,
    program: &'a ExpressionProgram,
) -> Result<&'a [u8], FriPolynomialError> {
    let offset = to_usize(entry.ops_offset)?;
    let count = to_usize(entry.ops_count)?;
    let end = offset
        .checked_add(count)
        .ok_or(FriPolynomialError::LengthOverflow)?;
    program
        .ops
        .get(offset..end)
        .ok_or(FriPolynomialError::OperationSpanOutOfBounds {
            expression_id: entry.expression_id,
        })
}

fn entry_args<'a>(
    entry: &ExpressionEntry,
    program: &'a ExpressionProgram,
) -> Result<&'a [u16], FriPolynomialError> {
    let offset = to_usize(entry.args_offset)?;
    let count = to_usize(entry.args_count)?;
    let end = offset
        .checked_add(count)
        .ok_or(FriPolynomialError::LengthOverflow)?;
    program
        .args
        .get(offset..end)
        .ok_or(FriPolynomialError::ArgumentSpanOutOfBounds {
            expression_id: entry.expression_id,
        })
}

fn read_operation_args(
    expression_id: u32,
    args: &[u16],
    cursor: usize,
) -> Result<OperationArgs, FriPolynomialError> {
    let fields = args
        .get(cursor..cursor + 8)
        .ok_or(FriPolynomialError::ArgumentCountMismatch {
            expression_id,
            consumed: cursor,
            declared: args.len(),
        })?;
    Ok(OperationArgs {
        kind: fields[0],
        destination_offset: fields[1] as usize,
        src0: SourceRef {
            buffer: fields[2],
            offset: fields[3] as usize,
            row_offset_index: fields[4] as usize,
        },
        src1: SourceRef {
            buffer: fields[5],
            offset: fields[6] as usize,
            row_offset_index: fields[7] as usize,
        },
    })
}

fn apply_base_op(kind: u16, left: Felt, right: Felt) -> Result<Felt, FriPolynomialError> {
    match kind {
        0 => Ok(left + right),
        1 => Ok(left - right),
        2 => Ok(left * right),
        3 => Ok(right - left),
        kind => Err(FriPolynomialError::UnsupportedOperationKind { kind }),
    }
}

fn apply_ext_op(kind: u16, left: Ext3, right: Ext3) -> Result<Ext3, FriPolynomialError> {
    match kind {
        0 => Ok(left + right),
        1 => Ok(left - right),
        2 => Ok(left * right),
        3 => Ok(right - left),
        kind => Err(FriPolynomialError::UnsupportedOperationKind { kind }),
    }
}

fn read_base(
    source: SourceRef,
    row: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
    program: &ExpressionProgram,
    inputs: FriPolynomialInputs<'_>,
    layout: BufferLayout,
) -> Result<Felt, FriPolynomialError> {
    match layout.resolve(source.buffer)? {
        BufferKind::Fixed => read_matrix_base(
            "fixed column",
            inputs.fixed_columns,
            source.offset,
            source_row(source, row, inputs)?,
        ),
        BufferKind::Stage(stage_index) => read_matrix_base(
            "stage column",
            find_stage_columns(inputs, stage_index)?,
            source.offset,
            source_row(source, row, inputs)?,
        ),
        BufferKind::CustomFixed(index) => read_matrix_base(
            "custom fixed column",
            *inputs.custom_fixed_columns.get(index).ok_or(
                FriPolynomialError::SourceIndexOutOfRange {
                    buffer: "custom fixed column",
                    offset: index,
                    width: 1,
                    len: inputs.custom_fixed_columns.len(),
                },
            )?,
            source.offset,
            source_row(source, row, inputs)?,
        ),
        BufferKind::DomainOrZerofier => read_domain_or_zerofier(source.offset, row, inputs),
        BufferKind::OpeningDenominator => Err(FriPolynomialError::UnsupportedSourceBuffer {
            buffer: source.buffer,
        }),
        BufferKind::Tmp1 => read_felt("tmp1", tmp1, source.offset),
        BufferKind::Tmp3 => read_felt("tmp3", tmp3, source.offset),
        BufferKind::Public => read_felt("public", inputs.publics, source.offset),
        BufferKind::Number => read_number(program, source.offset),
        BufferKind::UnitValue => read_felt("unit value", inputs.unit_values, source.offset),
        BufferKind::ProofValue => read_felt("proof value", inputs.proof_values, source.offset),
        BufferKind::GroupValue => read_ext_field("group value", inputs.group_values, source.offset),
        BufferKind::Challenge => read_ext_field("challenge", inputs.challenges, source.offset),
        BufferKind::Evaluation => read_ext_field("evaluation", inputs.evaluations, source.offset),
    }
}

fn read_ext(
    source: SourceRef,
    row: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
    program: &ExpressionProgram,
    inputs: FriPolynomialInputs<'_>,
    layout: BufferLayout,
) -> Result<Ext3, FriPolynomialError> {
    match layout.resolve(source.buffer)? {
        BufferKind::Fixed => read_matrix_ext(
            "fixed column",
            inputs.fixed_columns,
            source.offset,
            source_row(source, row, inputs)?,
        ),
        BufferKind::Stage(stage_index) => read_matrix_ext(
            "stage column",
            find_stage_columns(inputs, stage_index)?,
            source.offset,
            source_row(source, row, inputs)?,
        ),
        BufferKind::CustomFixed(index) => read_matrix_ext(
            "custom fixed column",
            *inputs.custom_fixed_columns.get(index).ok_or(
                FriPolynomialError::SourceIndexOutOfRange {
                    buffer: "custom fixed column",
                    offset: index,
                    width: 1,
                    len: inputs.custom_fixed_columns.len(),
                },
            )?,
            source.offset,
            source_row(source, row, inputs)?,
        ),
        BufferKind::DomainOrZerofier => Ok(scalar_ext(read_domain_or_zerofier(
            source.offset,
            row,
            inputs,
        )?)),
        BufferKind::OpeningDenominator => read_opening_denominator(source.offset, row, inputs),
        BufferKind::Tmp1 => read_felt_ext("tmp1", tmp1, source.offset),
        BufferKind::Tmp3 => read_felt_ext("tmp3", tmp3, source.offset),
        BufferKind::Public => read_felt_ext("public", inputs.publics, source.offset),
        BufferKind::Number => read_number_ext(program, source.offset),
        BufferKind::UnitValue => read_felt_ext("unit value", inputs.unit_values, source.offset),
        BufferKind::ProofValue => read_felt_ext("proof value", inputs.proof_values, source.offset),
        BufferKind::GroupValue => {
            read_ext_fields("group value", inputs.group_values, source.offset)
        }
        BufferKind::Challenge => read_ext_fields("challenge", inputs.challenges, source.offset),
        BufferKind::Evaluation => read_ext_fields("evaluation", inputs.evaluations, source.offset),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BufferKind {
    Fixed,
    Stage(u16),
    CustomFixed(usize),
    DomainOrZerofier,
    OpeningDenominator,
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
struct BufferLayout {
    stage_count: usize,
    custom_fixed_count: usize,
}

impl BufferLayout {
    fn new(inputs: FriPolynomialInputs<'_>) -> Self {
        Self {
            stage_count: inputs.stage_count as usize,
            custom_fixed_count: inputs.custom_fixed_columns.len(),
        }
    }

    fn resolve(&self, buffer: u16) -> Result<BufferKind, FriPolynomialError> {
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
            return Ok(BufferKind::OpeningDenominator);
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
            value => Err(FriPolynomialError::UnsupportedSourceBuffer {
                buffer: value as u16,
            }),
        }
    }
}

fn find_stage_columns(
    inputs: FriPolynomialInputs<'_>,
    stage_index: u16,
) -> Result<FriPolynomialColumnMatrix<'_>, FriPolynomialError> {
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
                return Ok(FriPolynomialColumnMatrix {
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
        .map(|stage| FriPolynomialColumnMatrix {
            column_count: stage.column_count,
            values: stage.values,
        })
        .ok_or(FriPolynomialError::MissingStageColumns { stage_index })
}

fn source_row(
    source: SourceRef,
    row: usize,
    inputs: FriPolynomialInputs<'_>,
) -> Result<usize, FriPolynomialError> {
    let offset = inputs
        .opening_point_offsets
        .get(source.row_offset_index)
        .ok_or(FriPolynomialError::SourceIndexOutOfRange {
            buffer: "opening point",
            offset: source.row_offset_index,
            width: 1,
            len: inputs.opening_point_offsets.len(),
        })?;
    let domain_size =
        i128::try_from(inputs.domain_size).map_err(|_| FriPolynomialError::LengthOverflow)?;
    let shifted =
        i128::try_from(row).map_err(|_| FriPolynomialError::LengthOverflow)? + i128::from(*offset);
    Ok(shifted.rem_euclid(domain_size) as usize)
}

fn read_domain_or_zerofier(
    offset: usize,
    row: usize,
    inputs: FriPolynomialInputs<'_>,
) -> Result<Felt, FriPolynomialError> {
    if offset == 0 {
        read_felt("domain point", inputs.domain_points, row)
    } else {
        read_matrix_base("zerofier", inputs.zerofier_values, offset - 1, row)
    }
}

fn read_opening_denominator(
    opening_index: usize,
    row: usize,
    inputs: FriPolynomialInputs<'_>,
) -> Result<Ext3, FriPolynomialError> {
    let x = read_felt("domain point", inputs.domain_points, row)?;
    let xi = inputs.opening_xis.get(opening_index).copied().ok_or(
        FriPolynomialError::SourceIndexOutOfRange {
            buffer: "opening xi",
            offset: opening_index,
            width: 1,
            len: inputs.opening_xis.len(),
        },
    )?;
    (scalar_ext(x) - xi)
        .inverse()
        .ok_or(FriPolynomialError::ZeroDenominator { opening_index })
}

fn read_matrix_base(
    buffer: &'static str,
    matrix: FriPolynomialColumnMatrix<'_>,
    column: usize,
    row: usize,
) -> Result<Felt, FriPolynomialError> {
    let index = matrix_index(matrix, column, 1, row)?;
    read_felt(buffer, matrix.values, index)
}

fn read_matrix_ext(
    buffer: &'static str,
    matrix: FriPolynomialColumnMatrix<'_>,
    column: usize,
    row: usize,
) -> Result<Ext3, FriPolynomialError> {
    let index = matrix_index(matrix, column, 3, row)?;
    Ok(Ext3::new(
        read_felt(buffer, matrix.values, index)?,
        read_felt(buffer, matrix.values, index + 1)?,
        read_felt(buffer, matrix.values, index + 2)?,
    ))
}

fn matrix_index(
    matrix: FriPolynomialColumnMatrix<'_>,
    column: usize,
    width: usize,
    row: usize,
) -> Result<usize, FriPolynomialError> {
    if column
        .checked_add(width)
        .is_none_or(|end| end > matrix.column_count)
    {
        return Err(FriPolynomialError::SourceIndexOutOfRange {
            buffer: "column matrix",
            offset: column,
            width,
            len: matrix.column_count,
        });
    }
    row.checked_mul(matrix.column_count)
        .and_then(|base| base.checked_add(column))
        .ok_or(FriPolynomialError::LengthOverflow)
}

fn read_felt(
    buffer: &'static str,
    values: &[Felt],
    offset: usize,
) -> Result<Felt, FriPolynomialError> {
    values
        .get(offset)
        .copied()
        .ok_or(FriPolynomialError::SourceIndexOutOfRange {
            buffer,
            offset,
            width: 1,
            len: values.len(),
        })
}

fn read_felt_ext(
    buffer: &'static str,
    values: &[Felt],
    offset: usize,
) -> Result<Ext3, FriPolynomialError> {
    if offset.checked_add(3).is_some_and(|end| end <= values.len()) {
        Ok(Ext3::new(
            values[offset],
            values[offset + 1],
            values[offset + 2],
        ))
    } else {
        Err(FriPolynomialError::SourceIndexOutOfRange {
            buffer,
            offset,
            width: 3,
            len: values.len(),
        })
    }
}

fn read_ext_field(
    buffer: &'static str,
    values: &[Ext3],
    offset: usize,
) -> Result<Felt, FriPolynomialError> {
    let value = values
        .get(offset / 3)
        .ok_or(FriPolynomialError::SourceIndexOutOfRange {
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

fn read_ext_fields(
    buffer: &'static str,
    values: &[Ext3],
    offset: usize,
) -> Result<Ext3, FriPolynomialError> {
    let len = values.len().saturating_mul(3);
    if offset.checked_add(3).is_none_or(|end| end > len) {
        return Err(FriPolynomialError::SourceIndexOutOfRange {
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

fn read_number(program: &ExpressionProgram, offset: usize) -> Result<Felt, FriPolynomialError> {
    let value = *program
        .numbers
        .get(offset)
        .ok_or(FriPolynomialError::SourceIndexOutOfRange {
            buffer: "number",
            offset,
            width: 1,
            len: program.numbers.len(),
        })?;
    canonical_number(value)
}

fn read_number_ext(program: &ExpressionProgram, offset: usize) -> Result<Ext3, FriPolynomialError> {
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
        Err(FriPolynomialError::SourceIndexOutOfRange {
            buffer: "number",
            offset,
            width: 3,
            len: program.numbers.len(),
        })
    }
}

fn write_base(tmp1: &mut [Felt], offset: usize, value: Felt) -> Result<(), FriPolynomialError> {
    let len = tmp1.len();
    *tmp1
        .get_mut(offset)
        .ok_or(FriPolynomialError::SourceIndexOutOfRange {
            buffer: "tmp1",
            offset,
            width: 1,
            len,
        })? = value;
    Ok(())
}

fn write_ext(tmp3: &mut [Felt], offset: usize, value: Ext3) -> Result<(), FriPolynomialError> {
    if offset.checked_add(3).is_some_and(|end| end <= tmp3.len()) {
        tmp3[offset] = value.c0;
        tmp3[offset + 1] = value.c1;
        tmp3[offset + 2] = value.c2;
        Ok(())
    } else {
        Err(FriPolynomialError::SourceIndexOutOfRange {
            buffer: "tmp3",
            offset,
            width: 3,
            len: tmp3.len(),
        })
    }
}

fn read_destination(
    entry: &ExpressionEntry,
    tmp1: &[Felt],
    tmp3: &[Felt],
) -> Result<Ext3, FriPolynomialError> {
    match entry.destination_dimension {
        1 => {
            let index = to_usize(entry.destination_id)?;
            Ok(scalar_ext(read_felt("tmp1", tmp1, index)?))
        }
        3 => {
            let index = to_usize(entry.destination_id)?;
            let offset = index
                .checked_mul(3)
                .ok_or(FriPolynomialError::LengthOverflow)?;
            read_felt_ext("tmp3", tmp3, offset)
        }
        dimension => Err(FriPolynomialError::UnsupportedDestinationDimension { dimension }),
    }
}

fn canonical_number(value: u64) -> Result<Felt, FriPolynomialError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => FriPolynomialError::NonCanonicalNumber { value },
    })
}

fn scalar_ext(value: Felt) -> Ext3 {
    Ext3::new(value, Felt::ZERO, Felt::ZERO)
}

fn to_usize(value: u32) -> Result<usize, FriPolynomialError> {
    usize::try_from(value).map_err(|_| FriPolynomialError::LengthOverflow)
}

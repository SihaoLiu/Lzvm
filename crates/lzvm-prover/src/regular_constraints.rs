#[cfg(test)]
use std::cell::Cell;
use std::fmt;

use lzvm_artifacts::constraint_program::{ConstraintEntry, ConstraintProgram};
use lzvm_field::{Ext3, Felt, FieldError};

#[derive(Debug, Clone, Copy, Default)]
pub struct RegularColumnMatrix<'a> {
    pub column_count: usize,
    pub values: &'a [Felt],
}

#[derive(Debug, Clone, Copy)]
pub struct RegularStageColumns<'a> {
    pub stage_index: u16,
    pub column_count: usize,
    pub values: &'a [Felt],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RegularConstraintInputs<'a> {
    pub domain_size: usize,
    pub stage_count: u16,
    pub fixed_columns: RegularColumnMatrix<'a>,
    pub stage_columns: &'a [RegularStageColumns<'a>],
    pub custom_fixed_columns: &'a [RegularColumnMatrix<'a>],
    pub opening_point_offsets: &'a [i64],
    pub domain_points: &'a [Felt],
    pub zerofier_values: RegularColumnMatrix<'a>,
    pub publics: &'a [Felt],
    pub unit_values: &'a [Felt],
    pub proof_values: &'a [Felt],
    pub group_values: &'a [Ext3],
    pub challenges: &'a [Ext3],
    pub evaluations: &'a [Ext3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegularConstraintResult {
    pub constraint_index: usize,
    pub stage: u32,
    pub intermediate: bool,
    pub invalid_rows: Vec<RegularConstraintViolation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegularConstraintViolation {
    pub row: usize,
    pub value: Ext3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegularConstraintEvalError {
    EmptyDomain,
    LengthOverflow,
    OperationSpanOutOfBounds {
        constraint_index: usize,
    },
    ArgumentSpanOutOfBounds {
        constraint_index: usize,
    },
    ArgumentCountMismatch {
        constraint_index: usize,
        consumed: usize,
        declared: usize,
    },
    UnsupportedOperationShape {
        shape: u8,
    },
    UnsupportedOperationKind {
        kind: u16,
    },
    UnsupportedDestinationDimension {
        dimension: u32,
    },
    UnsupportedSourceBuffer {
        buffer: u16,
    },
    MissingStageColumns {
        stage_index: u16,
    },
    MatrixLengthMismatch {
        buffer: &'static str,
        expected: usize,
        found: usize,
    },
    NonCanonicalNumber {
        value: u64,
    },
    SourceIndexOutOfRange {
        buffer: &'static str,
        offset: usize,
        width: usize,
        len: usize,
    },
}

impl fmt::Display for RegularConstraintEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDomain => write!(f, "regular constraint domain is empty"),
            Self::LengthOverflow => write!(f, "regular constraint length overflow"),
            Self::OperationSpanOutOfBounds { constraint_index } => write!(
                f,
                "regular constraint {constraint_index} operation span is out of bounds"
            ),
            Self::ArgumentSpanOutOfBounds { constraint_index } => write!(
                f,
                "regular constraint {constraint_index} argument span is out of bounds"
            ),
            Self::ArgumentCountMismatch {
                constraint_index,
                consumed,
                declared,
            } => write!(
                f,
                "regular constraint {constraint_index} consumed {consumed} arguments, declared {declared}"
            ),
            Self::UnsupportedOperationShape { shape } => {
                write!(f, "unsupported regular constraint operation shape: {shape}")
            }
            Self::UnsupportedOperationKind { kind } => {
                write!(f, "unsupported regular constraint operation kind: {kind}")
            }
            Self::UnsupportedDestinationDimension { dimension } => write!(
                f,
                "unsupported regular constraint destination dimension: {dimension}"
            ),
            Self::UnsupportedSourceBuffer { buffer } => {
                write!(f, "unsupported regular constraint source buffer: {buffer}")
            }
            Self::MissingStageColumns { stage_index } => {
                write!(f, "missing regular constraint stage columns: {stage_index}")
            }
            Self::MatrixLengthMismatch {
                buffer,
                expected,
                found,
            } => write!(
                f,
                "regular constraint {buffer} matrix length mismatch: expected {expected}, found {found}"
            ),
            Self::NonCanonicalNumber { value } => {
                write!(f, "non-canonical regular constraint number: {value}")
            }
            Self::SourceIndexOutOfRange {
                buffer,
                offset,
                width,
                len,
            } => write!(
                f,
                "regular constraint {buffer} offset {offset} with width {width} is outside length {len}"
            ),
        }
    }
}

impl std::error::Error for RegularConstraintEvalError {}

pub fn evaluate_regular_constraints(
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
) -> Result<Vec<RegularConstraintResult>, RegularConstraintEvalError> {
    if inputs.domain_size == 0 {
        return Err(RegularConstraintEvalError::EmptyDomain);
    }
    validate_inputs(inputs)?;
    program
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| evaluate_entry(index, entry, program, inputs))
        .collect()
}

fn validate_inputs(inputs: RegularConstraintInputs<'_>) -> Result<(), RegularConstraintEvalError> {
    validate_matrix("fixed column", inputs.fixed_columns, inputs.domain_size)?;
    for stage in inputs.stage_columns {
        validate_matrix(
            "stage column",
            RegularColumnMatrix {
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
        return Err(RegularConstraintEvalError::MatrixLengthMismatch {
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
    matrix: RegularColumnMatrix<'_>,
    domain_size: usize,
) -> Result<(), RegularConstraintEvalError> {
    let expected = domain_size
        .checked_mul(matrix.column_count)
        .ok_or(RegularConstraintEvalError::LengthOverflow)?;
    if matrix.values.len() == expected {
        Ok(())
    } else {
        Err(RegularConstraintEvalError::MatrixLengthMismatch {
            buffer,
            expected,
            found: matrix.values.len(),
        })
    }
}

fn evaluate_entry(
    constraint_index: usize,
    entry: &ConstraintEntry,
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
) -> Result<RegularConstraintResult, RegularConstraintEvalError> {
    let ops = entry_ops(constraint_index, entry, program)?;
    let args = entry_args(constraint_index, entry, program)?;
    let mut active_rows = active_rows(entry, inputs.domain_size)?;
    let Some(first_row) = active_rows.next() else {
        return Ok(RegularConstraintResult {
            constraint_index,
            stage: entry.stage,
            intermediate: entry.intermediate,
            invalid_rows: Vec::new(),
        });
    };

    let mut tmp1 = vec![Felt::ZERO; to_usize(entry.temp1_count)?];
    let mut tmp3 = vec![
        Felt::ZERO;
        to_usize(entry.temp3_count)?
            .checked_mul(3)
            .ok_or(RegularConstraintEvalError::LengthOverflow)?
    ];
    let mut context = RowEvaluationContext {
        constraint_index,
        entry,
        ops,
        args,
        operations: vec![None; ops.len()],
        sources: vec![[None, None]; ops.len()],
        program,
        inputs,
        layout: BufferLayout::new(inputs),
    };
    let mut invalid_rows = Vec::new();

    let value = evaluate_row(first_row, &mut context, &mut tmp1, &mut tmp3)?;
    if value != Ext3::ZERO {
        invalid_rows.push(RegularConstraintViolation {
            row: first_row,
            value,
        });
    }

    if active_rows.is_empty() {
        return Ok(RegularConstraintResult {
            constraint_index,
            stage: entry.stage,
            intermediate: entry.intermediate,
            invalid_rows,
        });
    }

    let prepared_operations = prepared_operations(&mut context)?;
    for row in active_rows {
        let value = evaluate_prepared_row(
            row,
            entry,
            &prepared_operations,
            program,
            inputs,
            &mut tmp1,
            &mut tmp3,
        )?;
        if value != Ext3::ZERO {
            invalid_rows.push(RegularConstraintViolation { row, value });
        }
    }

    Ok(RegularConstraintResult {
        constraint_index,
        stage: entry.stage,
        intermediate: entry.intermediate,
        invalid_rows,
    })
}

struct RowEvaluationContext<'a, 'input> {
    constraint_index: usize,
    entry: &'a ConstraintEntry,
    ops: &'a [u8],
    args: &'a [u16],
    operations: Vec<Option<DecodedOperation>>,
    sources: Vec<[Option<DecodedSource<'input>>; 2]>,
    program: &'a ConstraintProgram,
    inputs: RegularConstraintInputs<'input>,
    layout: BufferLayout,
}

fn evaluate_row(
    row: usize,
    context: &mut RowEvaluationContext<'_, '_>,
    tmp1: &mut [Felt],
    tmp3: &mut [Felt],
) -> Result<Ext3, RegularConstraintEvalError> {
    tmp1.fill(Felt::ZERO);
    tmp3.fill(Felt::ZERO);

    for operation_index in 0..context.ops.len() {
        let operation = decoded_operation(context, operation_index)?;
        match operation.shape {
            OperationShape::BaseBase => {
                let value = apply_base_op(
                    operation.kind,
                    read_base_source(context, operation_index, 0, operation.src0, row, tmp1, tmp3)?,
                    read_base_source(context, operation_index, 1, operation.src1, row, tmp1, tmp3)?,
                )?;
                write_base(tmp1, operation.destination_offset, value)?;
            }
            OperationShape::ExtBase => {
                let value = apply_ext_op(
                    operation.kind,
                    read_ext_source(context, operation_index, 0, operation.src0, row, tmp1, tmp3)?,
                    scalar_ext(read_base_source(
                        context,
                        operation_index,
                        1,
                        operation.src1,
                        row,
                        tmp1,
                        tmp3,
                    )?),
                )?;
                write_ext(tmp3, operation.destination_offset, value)?;
            }
            OperationShape::ExtExt => {
                let value = apply_ext_op(
                    operation.kind,
                    read_ext_source(context, operation_index, 0, operation.src0, row, tmp1, tmp3)?,
                    read_ext_source(context, operation_index, 1, operation.src1, row, tmp1, tmp3)?,
                )?;
                write_ext(tmp3, operation.destination_offset, value)?;
            }
        }
    }

    let consumed = context
        .ops
        .len()
        .checked_mul(8)
        .ok_or(RegularConstraintEvalError::LengthOverflow)?;
    if consumed != context.args.len() {
        return Err(RegularConstraintEvalError::ArgumentCountMismatch {
            constraint_index: context.constraint_index,
            consumed,
            declared: context.args.len(),
        });
    }

    read_destination(context.entry, tmp1, tmp3)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationShape {
    BaseBase,
    ExtBase,
    ExtExt,
}

#[derive(Debug, Clone, Copy)]
struct DecodedOperation {
    shape: OperationShape,
    kind: u16,
    destination_offset: usize,
    src0: SourceRef,
    src1: SourceRef,
}

#[derive(Debug, Clone, Copy)]
struct PreparedOperation<'input> {
    shape: OperationShape,
    kind: u16,
    destination_offset: usize,
    src0: DecodedSource<'input>,
    src1: DecodedSource<'input>,
}

#[derive(Debug, Clone, Copy)]
struct DecodedSource<'input> {
    offset: usize,
    row_offset: usize,
    kind: DecodedSourceKind<'input>,
}

#[derive(Debug, Clone, Copy)]
enum DecodedSourceKind<'input> {
    Fixed(RegularColumnMatrix<'input>),
    Stage(RegularColumnMatrix<'input>),
    CustomFixed(RegularColumnMatrix<'input>),
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
    constraint_index: usize,
    entry: &ConstraintEntry,
    program: &'a ConstraintProgram,
) -> Result<&'a [u8], RegularConstraintEvalError> {
    let offset = to_usize(entry.ops_offset)?;
    let count = to_usize(entry.ops_count)?;
    let end = offset
        .checked_add(count)
        .ok_or(RegularConstraintEvalError::LengthOverflow)?;
    program
        .ops
        .get(offset..end)
        .ok_or(RegularConstraintEvalError::OperationSpanOutOfBounds { constraint_index })
}

fn entry_args<'a>(
    constraint_index: usize,
    entry: &ConstraintEntry,
    program: &'a ConstraintProgram,
) -> Result<&'a [u16], RegularConstraintEvalError> {
    let offset = to_usize(entry.args_offset)?;
    let count = to_usize(entry.args_count)?;
    let end = offset
        .checked_add(count)
        .ok_or(RegularConstraintEvalError::LengthOverflow)?;
    program
        .args
        .get(offset..end)
        .ok_or(RegularConstraintEvalError::ArgumentSpanOutOfBounds { constraint_index })
}

fn read_operation_args(
    constraint_index: usize,
    args: &[u16],
    cursor: usize,
) -> Result<OperationArgs, RegularConstraintEvalError> {
    let fields =
        args.get(cursor..cursor + 8)
            .ok_or(RegularConstraintEvalError::ArgumentCountMismatch {
                constraint_index,
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

fn decoded_operation(
    context: &mut RowEvaluationContext<'_, '_>,
    operation_index: usize,
) -> Result<DecodedOperation, RegularConstraintEvalError> {
    if let Some(operation) = context.operations[operation_index] {
        return Ok(operation);
    }

    let cursor = operation_index
        .checked_mul(8)
        .ok_or(RegularConstraintEvalError::LengthOverflow)?;
    let op_args = read_operation_args(context.constraint_index, context.args, cursor)?;
    let shape = match context.ops[operation_index] {
        0 => OperationShape::BaseBase,
        1 => OperationShape::ExtBase,
        2 => OperationShape::ExtExt,
        shape => return Err(RegularConstraintEvalError::UnsupportedOperationShape { shape }),
    };
    let operation = DecodedOperation {
        shape,
        kind: op_args.kind,
        destination_offset: op_args.destination_offset,
        src0: op_args.src0,
        src1: op_args.src1,
    };
    context.operations[operation_index] = Some(operation);
    Ok(operation)
}

fn prepared_operations<'input>(
    context: &mut RowEvaluationContext<'_, 'input>,
) -> Result<Vec<PreparedOperation<'input>>, RegularConstraintEvalError> {
    let mut operations = Vec::with_capacity(context.ops.len());
    for operation_index in 0..context.ops.len() {
        let operation = decoded_operation(context, operation_index)?;
        operations.push(PreparedOperation {
            shape: operation.shape,
            kind: operation.kind,
            destination_offset: operation.destination_offset,
            src0: prepared_source(context, operation_index, 0, operation.src0)?,
            src1: prepared_source(context, operation_index, 1, operation.src1)?,
        });
    }
    Ok(operations)
}

fn prepared_source<'input>(
    context: &mut RowEvaluationContext<'_, 'input>,
    operation_index: usize,
    source_index: usize,
    source: SourceRef,
) -> Result<DecodedSource<'input>, RegularConstraintEvalError> {
    if let Some(source) = context.sources[operation_index][source_index] {
        return Ok(source);
    }

    let source = decode_source(source, context.inputs, context.layout)?;
    context.sources[operation_index][source_index] = Some(source);
    Ok(source)
}

fn cached_source<'input>(
    context: &mut RowEvaluationContext<'_, 'input>,
    operation_index: usize,
    source_index: usize,
    source: SourceRef,
) -> Result<DecodedSource<'input>, RegularConstraintEvalError> {
    #[cfg(test)]
    CACHED_SOURCE_COUNT.with(|count| count.set(count.get() + 1));

    if let Some(source) = context.sources[operation_index][source_index] {
        return Ok(source);
    }

    let source = decode_source(source, context.inputs, context.layout)?;
    context.sources[operation_index][source_index] = Some(source);
    Ok(source)
}

fn decode_source<'input>(
    source: SourceRef,
    inputs: RegularConstraintInputs<'input>,
    layout: BufferLayout,
) -> Result<DecodedSource<'input>, RegularConstraintEvalError> {
    let kind = match layout.resolve(source.buffer)? {
        BufferKind::Fixed => DecodedSourceKind::Fixed(inputs.fixed_columns),
        BufferKind::Stage(stage_index) => {
            DecodedSourceKind::Stage(find_stage_columns(inputs, stage_index)?)
        }
        BufferKind::CustomFixed(index) => {
            DecodedSourceKind::CustomFixed(*inputs.custom_fixed_columns.get(index).ok_or(
                RegularConstraintEvalError::SourceIndexOutOfRange {
                    buffer: "custom fixed column",
                    offset: index,
                    width: 1,
                    len: inputs.custom_fixed_columns.len(),
                },
            )?)
        }
        BufferKind::DomainOrZerofier => DecodedSourceKind::DomainOrZerofier,
        BufferKind::Tmp1 => DecodedSourceKind::Tmp1,
        BufferKind::Tmp3 => DecodedSourceKind::Tmp3,
        BufferKind::Public => DecodedSourceKind::Public,
        BufferKind::Number => DecodedSourceKind::Number,
        BufferKind::UnitValue => DecodedSourceKind::UnitValue,
        BufferKind::ProofValue => DecodedSourceKind::ProofValue,
        BufferKind::GroupValue => DecodedSourceKind::GroupValue,
        BufferKind::Challenge => DecodedSourceKind::Challenge,
        BufferKind::Evaluation => DecodedSourceKind::Evaluation,
    };
    let row_offset = match kind {
        DecodedSourceKind::Fixed(_)
        | DecodedSourceKind::Stage(_)
        | DecodedSourceKind::CustomFixed(_) => source_row_offset(source, inputs)?,
        DecodedSourceKind::DomainOrZerofier
        | DecodedSourceKind::Tmp1
        | DecodedSourceKind::Tmp3
        | DecodedSourceKind::Public
        | DecodedSourceKind::Number
        | DecodedSourceKind::UnitValue
        | DecodedSourceKind::ProofValue
        | DecodedSourceKind::GroupValue
        | DecodedSourceKind::Challenge
        | DecodedSourceKind::Evaluation => 0,
    };

    Ok(DecodedSource {
        offset: source.offset,
        row_offset,
        kind,
    })
}

fn active_rows(
    entry: &ConstraintEntry,
    domain_size: usize,
) -> Result<std::ops::Range<usize>, RegularConstraintEvalError> {
    let first = to_usize(entry.first_row)?;
    let last = to_usize(entry.last_row)?;
    Ok(first.min(domain_size)..last.min(domain_size))
}

fn apply_base_op(kind: u16, left: Felt, right: Felt) -> Result<Felt, RegularConstraintEvalError> {
    match kind {
        0 => Ok(left + right),
        1 => Ok(left - right),
        2 => Ok(left * right),
        3 => Ok(right - left),
        kind => Err(RegularConstraintEvalError::UnsupportedOperationKind { kind }),
    }
}

fn apply_ext_op(kind: u16, left: Ext3, right: Ext3) -> Result<Ext3, RegularConstraintEvalError> {
    match kind {
        0 => Ok(left + right),
        1 => Ok(left - right),
        2 => Ok(left * right),
        3 => Ok(right - left),
        kind => Err(RegularConstraintEvalError::UnsupportedOperationKind { kind }),
    }
}

fn read_base_source(
    context: &mut RowEvaluationContext<'_, '_>,
    operation_index: usize,
    source_index: usize,
    source: SourceRef,
    row: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
) -> Result<Felt, RegularConstraintEvalError> {
    let source = cached_source(context, operation_index, source_index, source)?;
    read_base(source, row, tmp1, tmp3, context.program, context.inputs)
}

fn read_ext_source(
    context: &mut RowEvaluationContext<'_, '_>,
    operation_index: usize,
    source_index: usize,
    source: SourceRef,
    row: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
) -> Result<Ext3, RegularConstraintEvalError> {
    let source = cached_source(context, operation_index, source_index, source)?;
    read_ext(source, row, tmp1, tmp3, context.program, context.inputs)
}

fn evaluate_prepared_row(
    row: usize,
    entry: &ConstraintEntry,
    operations: &[PreparedOperation<'_>],
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
    tmp1: &mut [Felt],
    tmp3: &mut [Felt],
) -> Result<Ext3, RegularConstraintEvalError> {
    tmp1.fill(Felt::ZERO);
    tmp3.fill(Felt::ZERO);

    for operation in operations {
        match operation.shape {
            OperationShape::BaseBase => {
                let value = apply_base_op(
                    operation.kind,
                    read_base(operation.src0, row, tmp1, tmp3, program, inputs)?,
                    read_base(operation.src1, row, tmp1, tmp3, program, inputs)?,
                )?;
                write_base(tmp1, operation.destination_offset, value)?;
            }
            OperationShape::ExtBase => {
                let value = apply_ext_op(
                    operation.kind,
                    read_ext(operation.src0, row, tmp1, tmp3, program, inputs)?,
                    scalar_ext(read_base(operation.src1, row, tmp1, tmp3, program, inputs)?),
                )?;
                write_ext(tmp3, operation.destination_offset, value)?;
            }
            OperationShape::ExtExt => {
                let value = apply_ext_op(
                    operation.kind,
                    read_ext(operation.src0, row, tmp1, tmp3, program, inputs)?,
                    read_ext(operation.src1, row, tmp1, tmp3, program, inputs)?,
                )?;
                write_ext(tmp3, operation.destination_offset, value)?;
            }
        }
    }

    read_destination(entry, tmp1, tmp3)
}

fn read_base(
    source: DecodedSource<'_>,
    row: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
) -> Result<Felt, RegularConstraintEvalError> {
    match source.kind {
        DecodedSourceKind::Fixed(matrix) => read_matrix_base(
            "fixed column",
            matrix,
            source.offset,
            source_row_with_offset(row, source.row_offset, inputs.domain_size)?,
        ),
        DecodedSourceKind::Stage(matrix) => read_matrix_base(
            "stage column",
            matrix,
            source.offset,
            source_row_with_offset(row, source.row_offset, inputs.domain_size)?,
        ),
        DecodedSourceKind::CustomFixed(matrix) => read_matrix_base(
            "custom fixed column",
            matrix,
            source.offset,
            source_row_with_offset(row, source.row_offset, inputs.domain_size)?,
        ),
        DecodedSourceKind::DomainOrZerofier => read_domain_or_zerofier(source.offset, row, inputs),
        DecodedSourceKind::Tmp1 => read_felt("tmp1", tmp1, source.offset),
        DecodedSourceKind::Tmp3 => read_felt("tmp3", tmp3, source.offset),
        DecodedSourceKind::Public => read_felt("public", inputs.publics, source.offset),
        DecodedSourceKind::Number => read_number(program, source.offset),
        DecodedSourceKind::UnitValue => read_felt("unit value", inputs.unit_values, source.offset),
        DecodedSourceKind::ProofValue => {
            read_felt("proof value", inputs.proof_values, source.offset)
        }
        DecodedSourceKind::GroupValue => {
            read_ext_field("group value", inputs.group_values, source.offset)
        }
        DecodedSourceKind::Challenge => {
            read_ext_field("challenge", inputs.challenges, source.offset)
        }
        DecodedSourceKind::Evaluation => {
            read_ext_field("evaluation", inputs.evaluations, source.offset)
        }
    }
}

fn read_ext(
    source: DecodedSource<'_>,
    row: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
) -> Result<Ext3, RegularConstraintEvalError> {
    match source.kind {
        DecodedSourceKind::Fixed(matrix) => read_matrix_ext(
            "fixed column",
            matrix,
            source.offset,
            source_row_with_offset(row, source.row_offset, inputs.domain_size)?,
        ),
        DecodedSourceKind::Stage(matrix) => read_matrix_ext(
            "stage column",
            matrix,
            source.offset,
            source_row_with_offset(row, source.row_offset, inputs.domain_size)?,
        ),
        DecodedSourceKind::CustomFixed(matrix) => read_matrix_ext(
            "custom fixed column",
            matrix,
            source.offset,
            source_row_with_offset(row, source.row_offset, inputs.domain_size)?,
        ),
        DecodedSourceKind::DomainOrZerofier => Ok(scalar_ext(read_domain_or_zerofier(
            source.offset,
            row,
            inputs,
        )?)),
        DecodedSourceKind::Tmp1 => read_felt_ext("tmp1", tmp1, source.offset),
        DecodedSourceKind::Tmp3 => read_felt_ext("tmp3", tmp3, source.offset),
        DecodedSourceKind::Public => read_felt_ext("public", inputs.publics, source.offset),
        DecodedSourceKind::Number => read_number_ext(program, source.offset),
        DecodedSourceKind::UnitValue => {
            read_felt_ext("unit value", inputs.unit_values, source.offset)
        }
        DecodedSourceKind::ProofValue => {
            read_felt_ext("proof value", inputs.proof_values, source.offset)
        }
        DecodedSourceKind::GroupValue => {
            read_ext_fields("group value", inputs.group_values, source.offset)
        }
        DecodedSourceKind::Challenge => {
            read_ext_fields("challenge", inputs.challenges, source.offset)
        }
        DecodedSourceKind::Evaluation => {
            read_ext_fields("evaluation", inputs.evaluations, source.offset)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BufferKind {
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
struct BufferLayout {
    stage_count: usize,
    custom_fixed_count: usize,
}

impl BufferLayout {
    fn new(inputs: RegularConstraintInputs<'_>) -> Self {
        Self {
            stage_count: inputs.stage_count as usize,
            custom_fixed_count: inputs.custom_fixed_columns.len(),
        }
    }

    fn resolve(&self, buffer: u16) -> Result<BufferKind, RegularConstraintEvalError> {
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
    static BUFFER_RESOLVE_COUNT: Cell<usize> = const { Cell::new(0) };
    static CACHED_SOURCE_COUNT: Cell<usize> = const { Cell::new(0) };
}

fn find_stage_columns(
    inputs: RegularConstraintInputs<'_>,
    stage_index: u16,
) -> Result<RegularColumnMatrix<'_>, RegularConstraintEvalError> {
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

fn source_row_offset(
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

fn normalize_row_offset(
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

fn source_row_with_offset(
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

fn read_domain_or_zerofier(
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

fn read_matrix_base(
    buffer: &'static str,
    matrix: RegularColumnMatrix<'_>,
    column: usize,
    row: usize,
) -> Result<Felt, RegularConstraintEvalError> {
    let index = matrix_index(matrix, column, 1, row)?;
    read_felt(buffer, matrix.values, index)
}

fn read_matrix_ext(
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

fn matrix_index(
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

fn read_felt(
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

fn read_felt_ext(
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

fn read_ext_field(
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

fn read_ext_fields(
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

fn read_number(
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

fn read_number_ext(
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

fn write_base(
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

fn write_ext(
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

fn read_destination(
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

fn canonical_number(value: u64) -> Result<Felt, RegularConstraintEvalError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => {
            RegularConstraintEvalError::NonCanonicalNumber { value }
        }
    })
}

fn scalar_ext(value: Felt) -> Ext3 {
    Ext3::new(value, Felt::ZERO, Felt::ZERO)
}

fn to_usize(value: u32) -> Result<usize, RegularConstraintEvalError> {
    usize::try_from(value).map_err(|_| RegularConstraintEvalError::LengthOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_constraint_buffer_resolution_is_per_source_not_per_row() {
        let program = ConstraintProgram {
            entries: vec![ConstraintEntry {
                stage: 1,
                destination_dimension: 1,
                destination_id: 0,
                first_row: 0,
                last_row: 8,
                temp1_count: 1,
                temp3_count: 0,
                ops_count: 1,
                ops_offset: 0,
                args_count: 8,
                args_offset: 0,
                intermediate: false,
                source_line: "layout resolution residual".to_owned(),
            }],
            ops: vec![0],
            args: vec![1, 0, 0, 0, 0, 8, 0, 0],
            numbers: vec![3],
        };
        let fixed = vec![Felt::from_u64(3); 8];

        BUFFER_RESOLVE_COUNT.with(|count| count.set(0));
        CACHED_SOURCE_COUNT.with(|count| count.set(0));
        let results = evaluate_regular_constraints(
            &program,
            RegularConstraintInputs {
                domain_size: 8,
                stage_count: 1,
                fixed_columns: RegularColumnMatrix {
                    column_count: 1,
                    values: &fixed,
                },
                opening_point_offsets: &[0],
                ..RegularConstraintInputs::default()
            },
        )
        .expect("regular constraint should evaluate");

        assert_eq!(results[0].invalid_rows, Vec::new());
        assert!(
            BUFFER_RESOLVE_COUNT.with(Cell::get) <= 2,
            "buffer layout should be resolved once per operation source"
        );
        assert!(
            CACHED_SOURCE_COUNT.with(Cell::get) <= 2,
            "decoded sources should be read without checking the cache on every row"
        );
    }

    #[test]
    fn source_row_offset_fallback_wraps_out_of_range_rows() {
        assert_eq!(source_row_with_offset(5, 0, 3).expect("row should wrap"), 2);
        assert_eq!(source_row_with_offset(5, 1, 3).expect("row should wrap"), 0);
    }
}

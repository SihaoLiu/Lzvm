use std::fmt;
use std::num::NonZeroUsize;
use std::thread;

use lzvm_artifacts::constraint_program::{ConstraintEntry, ConstraintProgram};
use lzvm_field::{Ext3, Felt};

mod regular_constraints_support;

use regular_constraints_support::*;

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
    WorkerPanic,
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
            Self::WorkerPanic => write!(f, "regular constraint worker panicked"),
        }
    }
}

impl std::error::Error for RegularConstraintEvalError {}

pub fn evaluate_regular_constraints(
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
) -> Result<Vec<RegularConstraintResult>, RegularConstraintEvalError> {
    evaluate_regular_constraints_with_workers(program, inputs, 1)
}

pub fn evaluate_regular_constraints_with_workers(
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
    worker_count: usize,
) -> Result<Vec<RegularConstraintResult>, RegularConstraintEvalError> {
    if inputs.domain_size == 0 {
        return Err(RegularConstraintEvalError::EmptyDomain);
    }
    validate_inputs(inputs)?;
    evaluate_constraint_entries(program, inputs, worker_count)
}

fn evaluate_constraint_entries(
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
    worker_count: usize,
) -> Result<Vec<RegularConstraintResult>, RegularConstraintEvalError> {
    let entry_count = program.entries.len();
    let worker_count = worker_count.max(1).min(entry_count);
    if worker_count <= 1 {
        return program
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| evaluate_entry(index, entry, program, inputs))
            .collect();
    }

    let chunk_size = entry_count.div_ceil(worker_count);
    let mut out = Vec::with_capacity(entry_count);
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for (chunk_index, chunk) in program.entries.chunks(chunk_size).enumerate() {
            let start_index = chunk_index
                .checked_mul(chunk_size)
                .ok_or(RegularConstraintEvalError::LengthOverflow)?;
            handles.push(scope.spawn(move || {
                chunk
                    .iter()
                    .enumerate()
                    .map(|(offset, entry)| {
                        evaluate_entry(start_index + offset, entry, program, inputs)
                    })
                    .collect::<Result<Vec<_>, _>>()
            }));
        }
        for handle in handles {
            let chunk = handle
                .join()
                .map_err(|_| RegularConstraintEvalError::WorkerPanic)??;
            out.extend(chunk);
        }
        Ok::<(), RegularConstraintEvalError>(())
    })?;
    Ok(out)
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
    if let Some(base_program) =
        prepared_base_operations(entry, &prepared_operations, program, inputs)?
    {
        for row in active_rows {
            #[cfg(test)]
            BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(count.get() + 1));

            let value = evaluate_prepared_base_row(
                row,
                entry,
                &base_program,
                inputs.domain_size,
                &mut tmp1,
                &mut tmp3,
            );
            if value != Felt::ZERO {
                invalid_rows.push(RegularConstraintViolation {
                    row,
                    value: scalar_ext(value),
                });
            }
        }
    } else {
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

#[derive(Debug, Clone)]
struct PreparedBaseProgram<'input> {
    operations: Vec<PreparedBaseOperation<'input>>,
    clear_tmp1: bool,
    clear_tmp3: bool,
}

#[derive(Debug, Clone)]
enum PreparedBaseOperation<'input> {
    Generic(PreparedGenericBaseOperation<'input>),
    MatrixTmp1Add(PreparedMatrixTmp1Operation<'input>),
    MatrixTmp1Sub(PreparedMatrixTmp1Operation<'input>),
    MatrixTmp1Mul(PreparedMatrixTmp1Operation<'input>),
    MatrixTmp1RSub(PreparedMatrixTmp1Operation<'input>),
    MatrixMatrixAdd(PreparedMatrixMatrixOperation<'input>),
    MatrixMatrixSub(PreparedMatrixMatrixOperation<'input>),
    MatrixMatrixMul(PreparedMatrixMatrixOperation<'input>),
    MatrixMatrixRSub(PreparedMatrixMatrixOperation<'input>),
    Tmp1Tmp1Add(PreparedTmp1Tmp1Operation),
    Tmp1Tmp1Sub(PreparedTmp1Tmp1Operation),
    Tmp1Tmp1Mul(PreparedTmp1Tmp1Operation),
    Tmp1Tmp1RSub(PreparedTmp1Tmp1Operation),
    MatrixConstantAdd(PreparedMatrixConstantOperation<'input>),
    MatrixConstantSub(PreparedMatrixConstantOperation<'input>),
    MatrixConstantMul(PreparedMatrixConstantOperation<'input>),
    MatrixConstantRSub(PreparedMatrixConstantOperation<'input>),
    Tmp1ConstantAdd(PreparedTmp1ConstantOperation),
    Tmp1ConstantSub(PreparedTmp1ConstantOperation),
    Tmp1ConstantMul(PreparedTmp1ConstantOperation),
    Tmp1ConstantRSub(PreparedTmp1ConstantOperation),
    ConstantAssign(PreparedConstantAssignOperation),
}

#[derive(Debug, Clone, Copy)]
struct PreparedGenericBaseOperation<'input> {
    kind: u16,
    destination_offset: usize,
    src0: PreparedBaseSource<'input>,
    src1: PreparedBaseSource<'input>,
}

#[derive(Debug, Clone, Copy)]
struct PreparedMatrixTmp1Operation<'input> {
    destination_offset: usize,
    matrix: PreparedBaseMatrix<'input>,
    tmp1_offset: usize,
}

#[derive(Debug, Clone, Copy)]
struct PreparedMatrixMatrixOperation<'input> {
    destination_offset: usize,
    left_matrix: PreparedBaseMatrix<'input>,
    right_matrix: PreparedBaseMatrix<'input>,
}

#[derive(Debug, Clone, Copy)]
struct PreparedTmp1Tmp1Operation {
    destination_offset: usize,
    left_offset: usize,
    right_offset: usize,
}

#[derive(Debug, Clone, Copy)]
struct PreparedMatrixConstantOperation<'input> {
    destination_offset: usize,
    matrix: PreparedBaseMatrix<'input>,
    constant: Felt,
}

#[derive(Debug, Clone, Copy)]
struct PreparedTmp1ConstantOperation {
    destination_offset: usize,
    tmp1_offset: usize,
    constant: Felt,
}

#[derive(Debug, Clone, Copy)]
struct PreparedConstantAssignOperation {
    destination_offset: usize,
    value: Felt,
}

#[derive(Debug, Clone, Copy)]
struct PreparedBaseMatrix<'input> {
    values: &'input [Felt],
    column_count: usize,
    column: usize,
    row_offset: Option<NonZeroUsize>,
}

#[derive(Debug, Clone, Copy)]
enum PreparedBaseSource<'input> {
    Matrix(PreparedBaseMatrix<'input>),
    DomainPoint(&'input [Felt]),
    Zerofier {
        values: &'input [Felt],
        column_count: usize,
        column: usize,
    },
    Tmp1(usize),
    Tmp3(usize),
    Constant(Felt),
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

fn prepared_base_operations<'input>(
    entry: &ConstraintEntry,
    operations: &[PreparedOperation<'input>],
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'input>,
) -> Result<Option<PreparedBaseProgram<'input>>, RegularConstraintEvalError> {
    if entry.destination_dimension != 1 {
        return Ok(None);
    }

    let tmp1_len = to_usize(entry.temp1_count)?;
    let tmp3_len = to_usize(entry.temp3_count)?
        .checked_mul(3)
        .ok_or(RegularConstraintEvalError::LengthOverflow)?;
    let destination_index = to_usize(entry.destination_id)?;
    validate_felt_index("tmp1", tmp1_len, destination_index)?;

    let mut base_operations = Vec::with_capacity(operations.len());
    let mut written_tmp1 = vec![false; tmp1_len];
    let mut clear_tmp1 = false;
    let mut clear_tmp3 = false;
    for operation in operations {
        if operation.shape != OperationShape::BaseBase {
            return Ok(None);
        }
        validate_felt_index("tmp1", tmp1_len, operation.destination_offset)?;
        let src0 = prepared_base_source(operation.src0, program, inputs, tmp1_len, tmp3_len)?;
        let src1 = prepared_base_source(operation.src1, program, inputs, tmp1_len, tmp3_len)?;
        clear_tmp1 |=
            reads_unwritten_tmp1(src0, &written_tmp1) || reads_unwritten_tmp1(src1, &written_tmp1);
        clear_tmp3 |= reads_tmp3(src0) || reads_tmp3(src1);
        written_tmp1[operation.destination_offset] = true;
        base_operations.push(prepared_base_operation(
            operation.kind,
            operation.destination_offset,
            src0,
            src1,
        ));
    }
    clear_tmp1 |= !written_tmp1[destination_index];
    Ok(Some(PreparedBaseProgram {
        operations: base_operations,
        clear_tmp1,
        clear_tmp3,
    }))
}

fn reads_unwritten_tmp1(source: PreparedBaseSource<'_>, written_tmp1: &[bool]) -> bool {
    matches!(source, PreparedBaseSource::Tmp1(offset) if !written_tmp1[offset])
}

fn reads_tmp3(source: PreparedBaseSource<'_>) -> bool {
    matches!(source, PreparedBaseSource::Tmp3(_))
}

fn prepared_base_operation<'input>(
    kind: u16,
    destination_offset: usize,
    src0: PreparedBaseSource<'input>,
    src1: PreparedBaseSource<'input>,
) -> PreparedBaseOperation<'input> {
    match (kind, src0, src1) {
        (0, PreparedBaseSource::Matrix(matrix), PreparedBaseSource::Tmp1(tmp1_offset)) => {
            PreparedBaseOperation::MatrixTmp1Add(PreparedMatrixTmp1Operation {
                destination_offset,
                matrix,
                tmp1_offset,
            })
        }
        (1, PreparedBaseSource::Matrix(matrix), PreparedBaseSource::Tmp1(tmp1_offset)) => {
            PreparedBaseOperation::MatrixTmp1Sub(PreparedMatrixTmp1Operation {
                destination_offset,
                matrix,
                tmp1_offset,
            })
        }
        (2, PreparedBaseSource::Matrix(matrix), PreparedBaseSource::Tmp1(tmp1_offset)) => {
            PreparedBaseOperation::MatrixTmp1Mul(PreparedMatrixTmp1Operation {
                destination_offset,
                matrix,
                tmp1_offset,
            })
        }
        (3, PreparedBaseSource::Matrix(matrix), PreparedBaseSource::Tmp1(tmp1_offset)) => {
            PreparedBaseOperation::MatrixTmp1RSub(PreparedMatrixTmp1Operation {
                destination_offset,
                matrix,
                tmp1_offset,
            })
        }
        (0, PreparedBaseSource::Matrix(left_matrix), PreparedBaseSource::Matrix(right_matrix)) => {
            PreparedBaseOperation::MatrixMatrixAdd(PreparedMatrixMatrixOperation {
                destination_offset,
                left_matrix,
                right_matrix,
            })
        }
        (1, PreparedBaseSource::Matrix(left_matrix), PreparedBaseSource::Matrix(right_matrix)) => {
            PreparedBaseOperation::MatrixMatrixSub(PreparedMatrixMatrixOperation {
                destination_offset,
                left_matrix,
                right_matrix,
            })
        }
        (2, PreparedBaseSource::Matrix(left_matrix), PreparedBaseSource::Matrix(right_matrix)) => {
            PreparedBaseOperation::MatrixMatrixMul(PreparedMatrixMatrixOperation {
                destination_offset,
                left_matrix,
                right_matrix,
            })
        }
        (3, PreparedBaseSource::Matrix(left_matrix), PreparedBaseSource::Matrix(right_matrix)) => {
            PreparedBaseOperation::MatrixMatrixRSub(PreparedMatrixMatrixOperation {
                destination_offset,
                left_matrix,
                right_matrix,
            })
        }
        (0, PreparedBaseSource::Tmp1(left_offset), PreparedBaseSource::Tmp1(right_offset)) => {
            PreparedBaseOperation::Tmp1Tmp1Add(PreparedTmp1Tmp1Operation {
                destination_offset,
                left_offset,
                right_offset,
            })
        }
        (1, PreparedBaseSource::Tmp1(left_offset), PreparedBaseSource::Tmp1(right_offset)) => {
            PreparedBaseOperation::Tmp1Tmp1Sub(PreparedTmp1Tmp1Operation {
                destination_offset,
                left_offset,
                right_offset,
            })
        }
        (2, PreparedBaseSource::Tmp1(left_offset), PreparedBaseSource::Tmp1(right_offset)) => {
            PreparedBaseOperation::Tmp1Tmp1Mul(PreparedTmp1Tmp1Operation {
                destination_offset,
                left_offset,
                right_offset,
            })
        }
        (3, PreparedBaseSource::Tmp1(left_offset), PreparedBaseSource::Tmp1(right_offset)) => {
            PreparedBaseOperation::Tmp1Tmp1RSub(PreparedTmp1Tmp1Operation {
                destination_offset,
                left_offset,
                right_offset,
            })
        }
        (0, PreparedBaseSource::Matrix(matrix), PreparedBaseSource::Constant(constant)) => {
            PreparedBaseOperation::MatrixConstantAdd(PreparedMatrixConstantOperation {
                destination_offset,
                matrix,
                constant,
            })
        }
        (1, PreparedBaseSource::Matrix(matrix), PreparedBaseSource::Constant(constant)) => {
            PreparedBaseOperation::MatrixConstantSub(PreparedMatrixConstantOperation {
                destination_offset,
                matrix,
                constant,
            })
        }
        (2, PreparedBaseSource::Matrix(matrix), PreparedBaseSource::Constant(constant)) => {
            PreparedBaseOperation::MatrixConstantMul(PreparedMatrixConstantOperation {
                destination_offset,
                matrix,
                constant,
            })
        }
        (3, PreparedBaseSource::Matrix(matrix), PreparedBaseSource::Constant(constant)) => {
            PreparedBaseOperation::MatrixConstantRSub(PreparedMatrixConstantOperation {
                destination_offset,
                matrix,
                constant,
            })
        }
        (0, PreparedBaseSource::Tmp1(tmp1_offset), PreparedBaseSource::Constant(constant)) => {
            PreparedBaseOperation::Tmp1ConstantAdd(PreparedTmp1ConstantOperation {
                destination_offset,
                tmp1_offset,
                constant,
            })
        }
        (1, PreparedBaseSource::Tmp1(tmp1_offset), PreparedBaseSource::Constant(constant)) => {
            PreparedBaseOperation::Tmp1ConstantSub(PreparedTmp1ConstantOperation {
                destination_offset,
                tmp1_offset,
                constant,
            })
        }
        (2, PreparedBaseSource::Tmp1(tmp1_offset), PreparedBaseSource::Constant(constant)) => {
            PreparedBaseOperation::Tmp1ConstantMul(PreparedTmp1ConstantOperation {
                destination_offset,
                tmp1_offset,
                constant,
            })
        }
        (3, PreparedBaseSource::Tmp1(tmp1_offset), PreparedBaseSource::Constant(constant)) => {
            PreparedBaseOperation::Tmp1ConstantRSub(PreparedTmp1ConstantOperation {
                destination_offset,
                tmp1_offset,
                constant,
            })
        }
        (kind, PreparedBaseSource::Constant(left), PreparedBaseSource::Constant(right)) => {
            PreparedBaseOperation::ConstantAssign(PreparedConstantAssignOperation {
                destination_offset,
                value: precompute_constant_base_op(kind, left, right),
            })
        }
        (kind, src0, src1) => PreparedBaseOperation::Generic(PreparedGenericBaseOperation {
            kind,
            destination_offset,
            src0,
            src1,
        }),
    }
}

fn prepared_base_source<'input>(
    source: DecodedSource<'input>,
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'input>,
    tmp1_len: usize,
    tmp3_len: usize,
) -> Result<PreparedBaseSource<'input>, RegularConstraintEvalError> {
    Ok(match source.kind {
        DecodedSourceKind::Fixed(matrix)
        | DecodedSourceKind::Stage(matrix)
        | DecodedSourceKind::CustomFixed(matrix) => {
            validate_matrix_source(matrix, source.offset, 1, inputs.domain_size)?;
            PreparedBaseSource::Matrix(PreparedBaseMatrix {
                values: matrix.values,
                column_count: matrix.column_count,
                column: source.offset,
                row_offset: NonZeroUsize::new(source.row_offset),
            })
        }
        DecodedSourceKind::DomainOrZerofier if source.offset == 0 => {
            if inputs.domain_points.len() < inputs.domain_size {
                return Err(RegularConstraintEvalError::SourceIndexOutOfRange {
                    buffer: "domain point",
                    offset: inputs.domain_points.len(),
                    width: inputs
                        .domain_size
                        .saturating_sub(inputs.domain_points.len()),
                    len: inputs.domain_points.len(),
                });
            }
            PreparedBaseSource::DomainPoint(inputs.domain_points)
        }
        DecodedSourceKind::DomainOrZerofier => {
            validate_matrix_source(
                inputs.zerofier_values,
                source.offset - 1,
                1,
                inputs.domain_size,
            )?;
            PreparedBaseSource::Zerofier {
                values: inputs.zerofier_values.values,
                column_count: inputs.zerofier_values.column_count,
                column: source.offset - 1,
            }
        }
        DecodedSourceKind::Tmp1 => {
            validate_felt_index("tmp1", tmp1_len, source.offset)?;
            PreparedBaseSource::Tmp1(source.offset)
        }
        DecodedSourceKind::Tmp3 => {
            validate_felt_index("tmp3", tmp3_len, source.offset)?;
            PreparedBaseSource::Tmp3(source.offset)
        }
        DecodedSourceKind::Public => {
            PreparedBaseSource::Constant(read_felt("public", inputs.publics, source.offset)?)
        }
        DecodedSourceKind::Number => {
            PreparedBaseSource::Constant(read_number(program, source.offset)?)
        }
        DecodedSourceKind::UnitValue => PreparedBaseSource::Constant(read_felt(
            "unit value",
            inputs.unit_values,
            source.offset,
        )?),
        DecodedSourceKind::ProofValue => PreparedBaseSource::Constant(read_felt(
            "proof value",
            inputs.proof_values,
            source.offset,
        )?),
        DecodedSourceKind::GroupValue => PreparedBaseSource::Constant(read_ext_field(
            "group value",
            inputs.group_values,
            source.offset,
        )?),
        DecodedSourceKind::Challenge => PreparedBaseSource::Constant(read_ext_field(
            "challenge",
            inputs.challenges,
            source.offset,
        )?),
        DecodedSourceKind::Evaluation => PreparedBaseSource::Constant(read_ext_field(
            "evaluation",
            inputs.evaluations,
            source.offset,
        )?),
    })
}

fn validate_matrix_source(
    matrix: RegularColumnMatrix<'_>,
    column: usize,
    width: usize,
    domain_size: usize,
) -> Result<(), RegularConstraintEvalError> {
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
    if domain_size != 0 {
        let last_row = domain_size - 1;
        let last_index = last_row
            .checked_mul(matrix.column_count)
            .and_then(|base| base.checked_add(column + width - 1))
            .ok_or(RegularConstraintEvalError::LengthOverflow)?;
        if last_index >= matrix.values.len() {
            return Err(RegularConstraintEvalError::SourceIndexOutOfRange {
                buffer: "column matrix",
                offset: last_index,
                width: 1,
                len: matrix.values.len(),
            });
        }
    }
    Ok(())
}

fn validate_felt_index(
    buffer: &'static str,
    len: usize,
    offset: usize,
) -> Result<(), RegularConstraintEvalError> {
    if offset < len {
        Ok(())
    } else {
        Err(RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer,
            offset,
            width: 1,
            len,
        })
    }
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

fn evaluate_prepared_base_row(
    row: usize,
    entry: &ConstraintEntry,
    program: &PreparedBaseProgram<'_>,
    domain_size: usize,
    tmp1: &mut [Felt],
    tmp3: &mut [Felt],
) -> Felt {
    if program.clear_tmp1 {
        #[cfg(test)]
        BASE_ONLY_TMP1_CLEAR_COUNT.with(|count| count.set(count.get() + 1));
        tmp1.fill(Felt::ZERO);
    }
    if program.clear_tmp3 {
        #[cfg(test)]
        BASE_ONLY_TMP3_CLEAR_COUNT.with(|count| count.set(count.get() + 1));
        tmp3.fill(Felt::ZERO);
    }

    for operation in &program.operations {
        match operation {
            PreparedBaseOperation::Generic(operation) => {
                let left = read_prepared_base(operation.src0, row, domain_size, tmp1, tmp3);
                let right = read_prepared_base(operation.src1, row, domain_size, tmp1, tmp3);
                tmp1[operation.destination_offset] =
                    apply_prepared_base_op(operation.kind, left, right);
            }
            PreparedBaseOperation::MatrixTmp1Add(operation) => {
                let (left, right) = read_matrix_tmp1_operands(operation, row, domain_size, tmp1);
                tmp1[operation.destination_offset] = left + right;
            }
            PreparedBaseOperation::MatrixTmp1Sub(operation) => {
                let (left, right) = read_matrix_tmp1_operands(operation, row, domain_size, tmp1);
                tmp1[operation.destination_offset] = left - right;
            }
            PreparedBaseOperation::MatrixTmp1Mul(operation) => {
                let (left, right) = read_matrix_tmp1_operands(operation, row, domain_size, tmp1);
                tmp1[operation.destination_offset] = left * right;
            }
            PreparedBaseOperation::MatrixTmp1RSub(operation) => {
                let (left, right) = read_matrix_tmp1_operands(operation, row, domain_size, tmp1);
                tmp1[operation.destination_offset] = right - left;
            }
            PreparedBaseOperation::MatrixMatrixAdd(operation) => {
                let (left, right) = read_matrix_matrix_operands(operation, row, domain_size);
                tmp1[operation.destination_offset] = left + right;
            }
            PreparedBaseOperation::MatrixMatrixSub(operation) => {
                let (left, right) = read_matrix_matrix_operands(operation, row, domain_size);
                tmp1[operation.destination_offset] = left - right;
            }
            PreparedBaseOperation::MatrixMatrixMul(operation) => {
                let (left, right) = read_matrix_matrix_operands(operation, row, domain_size);
                tmp1[operation.destination_offset] = left * right;
            }
            PreparedBaseOperation::MatrixMatrixRSub(operation) => {
                let (left, right) = read_matrix_matrix_operands(operation, row, domain_size);
                tmp1[operation.destination_offset] = right - left;
            }
            PreparedBaseOperation::Tmp1Tmp1Add(operation) => {
                let (left, right) = read_tmp1_tmp1_operands(operation, tmp1);
                tmp1[operation.destination_offset] = left + right;
            }
            PreparedBaseOperation::Tmp1Tmp1Sub(operation) => {
                let (left, right) = read_tmp1_tmp1_operands(operation, tmp1);
                tmp1[operation.destination_offset] = left - right;
            }
            PreparedBaseOperation::Tmp1Tmp1Mul(operation) => {
                let (left, right) = read_tmp1_tmp1_operands(operation, tmp1);
                tmp1[operation.destination_offset] = left * right;
            }
            PreparedBaseOperation::Tmp1Tmp1RSub(operation) => {
                let (left, right) = read_tmp1_tmp1_operands(operation, tmp1);
                tmp1[operation.destination_offset] = right - left;
            }
            PreparedBaseOperation::MatrixConstantAdd(operation) => {
                let left = read_prepared_matrix(operation.matrix, row, domain_size);
                tmp1[operation.destination_offset] = left + operation.constant;
            }
            PreparedBaseOperation::MatrixConstantSub(operation) => {
                let left = read_prepared_matrix(operation.matrix, row, domain_size);
                tmp1[operation.destination_offset] = left - operation.constant;
            }
            PreparedBaseOperation::MatrixConstantMul(operation) => {
                let left = read_prepared_matrix(operation.matrix, row, domain_size);
                tmp1[operation.destination_offset] = left * operation.constant;
            }
            PreparedBaseOperation::MatrixConstantRSub(operation) => {
                let left = read_prepared_matrix(operation.matrix, row, domain_size);
                tmp1[operation.destination_offset] = operation.constant - left;
            }
            PreparedBaseOperation::Tmp1ConstantAdd(operation) => {
                let left = tmp1[operation.tmp1_offset];
                tmp1[operation.destination_offset] = left + operation.constant;
            }
            PreparedBaseOperation::Tmp1ConstantSub(operation) => {
                let left = tmp1[operation.tmp1_offset];
                tmp1[operation.destination_offset] = left - operation.constant;
            }
            PreparedBaseOperation::Tmp1ConstantMul(operation) => {
                let left = tmp1[operation.tmp1_offset];
                tmp1[operation.destination_offset] = left * operation.constant;
            }
            PreparedBaseOperation::Tmp1ConstantRSub(operation) => {
                let left = tmp1[operation.tmp1_offset];
                tmp1[operation.destination_offset] = operation.constant - left;
            }
            PreparedBaseOperation::ConstantAssign(operation) => {
                tmp1[operation.destination_offset] = operation.value;
            }
        }
    }

    tmp1[to_usize(entry.destination_id).expect("destination index was prepared")]
}

#[inline(always)]
fn read_matrix_tmp1_operands(
    operation: &PreparedMatrixTmp1Operation<'_>,
    row: usize,
    domain_size: usize,
    tmp1: &[Felt],
) -> (Felt, Felt) {
    (
        read_prepared_matrix(operation.matrix, row, domain_size),
        tmp1[operation.tmp1_offset],
    )
}

#[inline(always)]
fn read_matrix_matrix_operands(
    operation: &PreparedMatrixMatrixOperation<'_>,
    row: usize,
    domain_size: usize,
) -> (Felt, Felt) {
    (
        read_prepared_matrix(operation.left_matrix, row, domain_size),
        read_prepared_matrix(operation.right_matrix, row, domain_size),
    )
}

#[inline(always)]
fn read_tmp1_tmp1_operands(operation: &PreparedTmp1Tmp1Operation, tmp1: &[Felt]) -> (Felt, Felt) {
    (tmp1[operation.left_offset], tmp1[operation.right_offset])
}

#[inline(always)]
fn apply_prepared_base_op(kind: u16, left: Felt, right: Felt) -> Felt {
    #[cfg(test)]
    BASE_ONLY_KIND_DISPATCH_COUNT.with(|count| count.set(count.get() + 1));

    precompute_constant_base_op(kind, left, right)
}

#[inline(always)]
fn precompute_constant_base_op(kind: u16, left: Felt, right: Felt) -> Felt {
    match kind {
        0 => left + right,
        1 => left - right,
        2 => left * right,
        3 => right - left,
        _ => unreachable!("operation kind was validated by the first row"),
    }
}

#[inline(always)]
fn read_prepared_base(
    source: PreparedBaseSource<'_>,
    row: usize,
    domain_size: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
) -> Felt {
    match source {
        PreparedBaseSource::Matrix(matrix) => read_prepared_matrix(matrix, row, domain_size),
        PreparedBaseSource::DomainPoint(values) => values[row],
        PreparedBaseSource::Zerofier {
            values,
            column_count,
            column,
        } => values[row * column_count + column],
        PreparedBaseSource::Tmp1(offset) => tmp1[offset],
        PreparedBaseSource::Tmp3(offset) => tmp3[offset],
        PreparedBaseSource::Constant(value) => value,
    }
}

#[inline(always)]
fn read_prepared_matrix(matrix: PreparedBaseMatrix<'_>, row: usize, domain_size: usize) -> Felt {
    let row = matrix.row_offset.map_or(row, |row_offset| {
        source_row_with_offset_prepared(row, row_offset.get(), domain_size)
    });
    matrix.values[row * matrix.column_count + matrix.column]
}

#[inline(always)]
fn source_row_with_offset_prepared(row: usize, row_offset: usize, domain_size: usize) -> usize {
    #[cfg(test)]
    PREPARED_SOURCE_ROW_OFFSET_COUNT.with(|count| count.set(count.get() + 1));

    if row < domain_size && row_offset < domain_size {
        if row_offset == 0 {
            return row;
        }
        let wrap_at = domain_size - row_offset;
        if row < wrap_at {
            return row + row_offset;
        } else {
            return row - wrap_at;
        }
    }
    ((row as u128 + row_offset as u128) % domain_size as u128) as usize
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

#[cfg(test)]
mod regular_constraints_tests;

use std::fmt;

use lzvm_artifacts::constraint_program::{GlobalConstraintEntry, GlobalConstraintProgram};
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt, FieldError};

use crate::group_values::{load_group_values_from_segments, LoadGroupValuesSegmentError};
use crate::pcs_query_plan::uses_transcript_pcs_query_plan_inputs;
use crate::pcs_transcript_segments::{
    derive_pcs_transcript_challenges_from_proof_segments, PcsTranscriptProofSegmentsError,
};
use crate::proof_values::{
    flatten_pcs_proof_values, load_pcs_proof_values_from_segments, LoadPcsProofValuesSegmentError,
    ProvePcsProofValuesSegmentError,
};
use crate::ProveSchedule;

#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalConstraintInputs<'a> {
    pub publics: &'a [Felt],
    pub proof_values: &'a [Felt],
    pub challenges: &'a [Ext3],
    pub group_values: &'a [Ext3],
}

#[derive(Debug, Clone, Copy)]
pub struct ValidateGlobalConstraintProofSegmentsRequest<'a> {
    pub program: &'a GlobalConstraintProgram,
    pub global_info: &'a GlobalInfo,
    pub schedule: &'a ProveSchedule,
    pub public_values: &'a [Felt],
    pub segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalConstraintEvalError {
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
    UnknownBuffer {
        buffer: u16,
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

impl fmt::Display for GlobalConstraintEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthOverflow => write!(f, "global constraint length overflow"),
            Self::OperationSpanOutOfBounds { constraint_index } => write!(
                f,
                "global constraint {constraint_index} operation span is out of bounds"
            ),
            Self::ArgumentSpanOutOfBounds { constraint_index } => write!(
                f,
                "global constraint {constraint_index} argument span is out of bounds"
            ),
            Self::ArgumentCountMismatch {
                constraint_index,
                consumed,
                declared,
            } => write!(
                f,
                "global constraint {constraint_index} consumed {consumed} arguments, declared {declared}"
            ),
            Self::UnsupportedOperationShape { shape } => {
                write!(f, "unsupported global constraint operation shape: {shape}")
            }
            Self::UnsupportedOperationKind { kind } => {
                write!(f, "unsupported global constraint operation kind: {kind}")
            }
            Self::UnsupportedDestinationDimension { dimension } => write!(
                f,
                "unsupported global constraint destination dimension: {dimension}"
            ),
            Self::UnknownBuffer { buffer } => {
                write!(f, "unknown global constraint source buffer: {buffer}")
            }
            Self::NonCanonicalNumber { value } => {
                write!(f, "non-canonical global constraint number: {value}")
            }
            Self::SourceIndexOutOfRange {
                buffer,
                offset,
                width,
                len,
            } => write!(
                f,
                "global constraint {buffer} offset {offset} with width {width} is outside length {len}"
            ),
        }
    }
}

impl std::error::Error for GlobalConstraintEvalError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalConstraintValidationError {
    Eval(GlobalConstraintEvalError),
    ConstraintViolation {
        constraint_index: usize,
        value: [u64; 3],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateGlobalConstraintProofSegmentsError {
    ProofValues(LoadPcsProofValuesSegmentError),
    PackedProofValues(ProvePcsProofValuesSegmentError),
    Transcript(PcsTranscriptProofSegmentsError),
    GroupValues(LoadGroupValuesSegmentError),
    Validation(GlobalConstraintValidationError),
}

impl fmt::Display for GlobalConstraintValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eval(error) => write!(f, "{error}"),
            Self::ConstraintViolation {
                constraint_index, ..
            } => {
                write!(f, "global constraint {constraint_index} is not satisfied")
            }
        }
    }
}

impl std::error::Error for GlobalConstraintValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Eval(error) => Some(error),
            Self::ConstraintViolation { .. } => None,
        }
    }
}

impl fmt::Display for ValidateGlobalConstraintProofSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProofValues(error) => write!(f, "{error}"),
            Self::PackedProofValues(error) => {
                write!(f, "global constraint proof values invalid: {error}")
            }
            Self::Transcript(error) => write!(f, "{error}"),
            Self::GroupValues(error) => write!(f, "{error}"),
            Self::Validation(GlobalConstraintValidationError::Eval(source)) => {
                write!(f, "invalid global constraint program: {source}")
            }
            Self::Validation(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ValidateGlobalConstraintProofSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ProofValues(error) => Some(error),
            Self::PackedProofValues(error) => Some(error),
            Self::Transcript(error) => Some(error),
            Self::GroupValues(error) => Some(error),
            Self::Validation(error) => Some(error),
        }
    }
}

impl From<GlobalConstraintEvalError> for GlobalConstraintValidationError {
    fn from(error: GlobalConstraintEvalError) -> Self {
        Self::Eval(error)
    }
}

pub fn evaluate_global_constraints(
    program: &GlobalConstraintProgram,
    inputs: GlobalConstraintInputs<'_>,
) -> Result<Vec<Ext3>, GlobalConstraintEvalError> {
    let mut residuals = Vec::with_capacity(program.entries.len());
    let mut tmp1 = Vec::new();
    let mut tmp3 = Vec::new();
    for (index, entry) in program.entries.iter().enumerate() {
        residuals.push(evaluate_entry(
            index, entry, program, inputs, &mut tmp1, &mut tmp3,
        )?);
    }
    Ok(residuals)
}

pub fn validate_global_constraints(
    program: &GlobalConstraintProgram,
    inputs: GlobalConstraintInputs<'_>,
) -> Result<(), GlobalConstraintValidationError> {
    let residuals = evaluate_global_constraints(program, inputs)?;
    for (constraint_index, residual) in residuals.into_iter().enumerate() {
        if residual != Ext3::ZERO {
            return Err(GlobalConstraintValidationError::ConstraintViolation {
                constraint_index,
                value: residual.to_u64s(),
            });
        }
    }
    Ok(())
}

pub fn validate_global_constraints_from_proof_segments(
    request: ValidateGlobalConstraintProofSegmentsRequest<'_>,
) -> Result<(), ValidateGlobalConstraintProofSegmentsError> {
    if request.program.entries.is_empty() {
        return Ok(());
    }

    let proof_values = load_pcs_proof_values_from_segments(request.global_info, request.segments)
        .map_err(ValidateGlobalConstraintProofSegmentsError::ProofValues)?;
    let packed_proof_values = flatten_pcs_proof_values(request.global_info, &proof_values)
        .map_err(ValidateGlobalConstraintProofSegmentsError::PackedProofValues)?;
    let challenges = if uses_transcript_pcs_query_plan_inputs(request.segments) {
        derive_pcs_transcript_challenges_from_proof_segments(
            request.schedule,
            request.public_values,
            request.segments,
        )
        .map_err(ValidateGlobalConstraintProofSegmentsError::Transcript)?
    } else {
        Vec::new()
    };
    let group_values = load_group_values_from_segments(request.global_info, request.segments)
        .map_err(ValidateGlobalConstraintProofSegmentsError::GroupValues)?;

    validate_global_constraints(
        request.program,
        GlobalConstraintInputs {
            publics: request.public_values,
            proof_values: &packed_proof_values,
            challenges: &challenges,
            group_values: &group_values,
        },
    )
    .map_err(ValidateGlobalConstraintProofSegmentsError::Validation)
}

fn evaluate_entry(
    constraint_index: usize,
    entry: &GlobalConstraintEntry,
    program: &GlobalConstraintProgram,
    inputs: GlobalConstraintInputs<'_>,
    tmp1: &mut Vec<Felt>,
    tmp3: &mut Vec<Felt>,
) -> Result<Ext3, GlobalConstraintEvalError> {
    let ops = entry_ops(constraint_index, entry, program)?;
    let args = entry_args(constraint_index, entry, program)?;
    validate_operation_arg_count(constraint_index, args, ops.len())?;
    let tmp1_len = to_usize(entry.temp1_count)?;
    let tmp3_len = to_usize(entry.temp3_count)?.saturating_mul(3);
    if tmp1.len() < tmp1_len {
        tmp1.resize(tmp1_len, Felt::ZERO);
    }
    if tmp3.len() < tmp3_len {
        tmp3.resize(tmp3_len, Felt::ZERO);
    }
    let tmp1 = &mut tmp1[..tmp1_len];
    let tmp3 = &mut tmp3[..tmp3_len];
    tmp1.fill(Felt::ZERO);
    tmp3.fill(Felt::ZERO);
    let mut cursor = 0usize;

    for shape in ops {
        let op_args = read_operation_args(constraint_index, args, cursor)?;
        cursor += 6;
        match *shape {
            0 => {
                let value = apply_base_op(
                    op_args.kind,
                    read_base(
                        op_args.src0_buffer,
                        op_args.src0_offset,
                        tmp1,
                        tmp3,
                        program,
                        inputs,
                    )?,
                    read_base(
                        op_args.src1_buffer,
                        op_args.src1_offset,
                        tmp1,
                        tmp3,
                        program,
                        inputs,
                    )?,
                )?;
                write_base(tmp1, op_args.destination_offset, value)?;
            }
            1 => {
                let value = apply_ext_op(
                    op_args.kind,
                    read_ext(
                        op_args.src0_buffer,
                        op_args.src0_offset,
                        tmp1,
                        tmp3,
                        program,
                        inputs,
                    )?,
                    scalar_ext(read_base(
                        op_args.src1_buffer,
                        op_args.src1_offset,
                        tmp1,
                        tmp3,
                        program,
                        inputs,
                    )?),
                )?;
                write_ext(tmp3, op_args.destination_offset, value)?;
            }
            2 => {
                let value = apply_ext_op(
                    op_args.kind,
                    read_ext(
                        op_args.src0_buffer,
                        op_args.src0_offset,
                        tmp1,
                        tmp3,
                        program,
                        inputs,
                    )?,
                    read_ext(
                        op_args.src1_buffer,
                        op_args.src1_offset,
                        tmp1,
                        tmp3,
                        program,
                        inputs,
                    )?,
                )?;
                write_ext(tmp3, op_args.destination_offset, value)?;
            }
            shape => return Err(GlobalConstraintEvalError::UnsupportedOperationShape { shape }),
        }
    }

    if cursor != args.len() {
        return Err(GlobalConstraintEvalError::ArgumentCountMismatch {
            constraint_index,
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
    src0_buffer: u16,
    src0_offset: usize,
    src1_buffer: u16,
    src1_offset: usize,
}

fn entry_ops<'a>(
    constraint_index: usize,
    entry: &GlobalConstraintEntry,
    program: &'a GlobalConstraintProgram,
) -> Result<&'a [u8], GlobalConstraintEvalError> {
    let offset = to_usize(entry.ops_offset)?;
    let count = to_usize(entry.ops_count)?;
    let end = offset
        .checked_add(count)
        .ok_or(GlobalConstraintEvalError::LengthOverflow)?;
    program
        .ops
        .get(offset..end)
        .ok_or(GlobalConstraintEvalError::OperationSpanOutOfBounds { constraint_index })
}

fn entry_args<'a>(
    constraint_index: usize,
    entry: &GlobalConstraintEntry,
    program: &'a GlobalConstraintProgram,
) -> Result<&'a [u16], GlobalConstraintEvalError> {
    let offset = to_usize(entry.args_offset)?;
    let count = to_usize(entry.args_count)?;
    let end = offset
        .checked_add(count)
        .ok_or(GlobalConstraintEvalError::LengthOverflow)?;
    program
        .args
        .get(offset..end)
        .ok_or(GlobalConstraintEvalError::ArgumentSpanOutOfBounds { constraint_index })
}

fn validate_operation_arg_count(
    constraint_index: usize,
    args: &[u16],
    op_count: usize,
) -> Result<(), GlobalConstraintEvalError> {
    let expected = op_count
        .checked_mul(6)
        .ok_or(GlobalConstraintEvalError::LengthOverflow)?;
    if args.len() == expected {
        return Ok(());
    }
    let consumed = if args.len() < expected {
        args.len() - (args.len() % 6)
    } else {
        expected
    };
    Err(GlobalConstraintEvalError::ArgumentCountMismatch {
        constraint_index,
        consumed,
        declared: args.len(),
    })
}

fn read_operation_args(
    constraint_index: usize,
    args: &[u16],
    cursor: usize,
) -> Result<OperationArgs, GlobalConstraintEvalError> {
    let fields =
        args.get(cursor..cursor + 6)
            .ok_or(GlobalConstraintEvalError::ArgumentCountMismatch {
                constraint_index,
                consumed: cursor,
                declared: args.len(),
            })?;
    Ok(OperationArgs {
        kind: fields[0],
        destination_offset: fields[1] as usize,
        src0_buffer: fields[2],
        src0_offset: fields[3] as usize,
        src1_buffer: fields[4],
        src1_offset: fields[5] as usize,
    })
}

fn apply_base_op(kind: u16, left: Felt, right: Felt) -> Result<Felt, GlobalConstraintEvalError> {
    match kind {
        0 => Ok(left + right),
        1 => Ok(left - right),
        2 => Ok(left * right),
        3 => Ok(right - left),
        kind => Err(GlobalConstraintEvalError::UnsupportedOperationKind { kind }),
    }
}

fn apply_ext_op(kind: u16, left: Ext3, right: Ext3) -> Result<Ext3, GlobalConstraintEvalError> {
    match kind {
        0 => Ok(left + right),
        1 => Ok(left - right),
        2 => Ok(left * right),
        3 => Ok(right - left),
        kind => Err(GlobalConstraintEvalError::UnsupportedOperationKind { kind }),
    }
}

fn read_base(
    buffer: u16,
    offset: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
    program: &GlobalConstraintProgram,
    inputs: GlobalConstraintInputs<'_>,
) -> Result<Felt, GlobalConstraintEvalError> {
    match buffer {
        0 => read_felt("tmp1", tmp1, offset),
        1 => read_felt("public", inputs.publics, offset),
        2 => read_number(program, offset),
        3 => read_felt("proof value", inputs.proof_values, offset),
        4 => read_felt("tmp3", tmp3, offset),
        5 => read_ext_field("group value", inputs.group_values, offset),
        6 => read_ext_field("challenge", inputs.challenges, offset),
        buffer => Err(GlobalConstraintEvalError::UnknownBuffer { buffer }),
    }
}

fn read_ext(
    buffer: u16,
    offset: usize,
    tmp1: &[Felt],
    tmp3: &[Felt],
    program: &GlobalConstraintProgram,
    inputs: GlobalConstraintInputs<'_>,
) -> Result<Ext3, GlobalConstraintEvalError> {
    match buffer {
        0 => read_felt_ext("tmp1", tmp1, offset),
        1 => read_felt_ext("public", inputs.publics, offset),
        2 => read_number_ext(program, offset),
        3 => read_felt_ext("proof value", inputs.proof_values, offset),
        4 => read_felt_ext("tmp3", tmp3, offset),
        5 => read_ext_fields("group value", inputs.group_values, offset),
        6 => read_ext_fields("challenge", inputs.challenges, offset),
        buffer => Err(GlobalConstraintEvalError::UnknownBuffer { buffer }),
    }
}

fn read_felt(
    buffer: &'static str,
    values: &[Felt],
    offset: usize,
) -> Result<Felt, GlobalConstraintEvalError> {
    values
        .get(offset)
        .copied()
        .ok_or(GlobalConstraintEvalError::SourceIndexOutOfRange {
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
) -> Result<Ext3, GlobalConstraintEvalError> {
    if offset.checked_add(3).is_some_and(|end| end <= values.len()) {
        Ok(Ext3::new(
            values[offset],
            values[offset + 1],
            values[offset + 2],
        ))
    } else {
        Err(GlobalConstraintEvalError::SourceIndexOutOfRange {
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
) -> Result<Felt, GlobalConstraintEvalError> {
    let value = values
        .get(offset / 3)
        .ok_or(GlobalConstraintEvalError::SourceIndexOutOfRange {
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
) -> Result<Ext3, GlobalConstraintEvalError> {
    let len = values.len().saturating_mul(3);
    if offset.checked_add(3).is_none_or(|end| end > len) {
        return Err(GlobalConstraintEvalError::SourceIndexOutOfRange {
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
    program: &GlobalConstraintProgram,
    offset: usize,
) -> Result<Felt, GlobalConstraintEvalError> {
    let value =
        *program
            .numbers
            .get(offset)
            .ok_or(GlobalConstraintEvalError::SourceIndexOutOfRange {
                buffer: "number",
                offset,
                width: 1,
                len: program.numbers.len(),
            })?;
    canonical_number(value)
}

fn read_number_ext(
    program: &GlobalConstraintProgram,
    offset: usize,
) -> Result<Ext3, GlobalConstraintEvalError> {
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
        Err(GlobalConstraintEvalError::SourceIndexOutOfRange {
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
) -> Result<(), GlobalConstraintEvalError> {
    let len = tmp1.len();
    *tmp1
        .get_mut(offset)
        .ok_or(GlobalConstraintEvalError::SourceIndexOutOfRange {
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
) -> Result<(), GlobalConstraintEvalError> {
    if offset.checked_add(3).is_some_and(|end| end <= tmp3.len()) {
        tmp3[offset] = value.c0;
        tmp3[offset + 1] = value.c1;
        tmp3[offset + 2] = value.c2;
        Ok(())
    } else {
        Err(GlobalConstraintEvalError::SourceIndexOutOfRange {
            buffer: "tmp3",
            offset,
            width: 3,
            len: tmp3.len(),
        })
    }
}

fn read_destination(
    entry: &GlobalConstraintEntry,
    tmp1: &[Felt],
    tmp3: &[Felt],
) -> Result<Ext3, GlobalConstraintEvalError> {
    match entry.destination_dimension {
        1 => {
            let index = to_usize(entry.destination_id)?;
            Ok(scalar_ext(read_felt("tmp1", tmp1, index)?))
        }
        3 => {
            let index = to_usize(entry.destination_id)?;
            let offset = index
                .checked_mul(3)
                .ok_or(GlobalConstraintEvalError::LengthOverflow)?;
            read_felt_ext("tmp3", tmp3, offset)
        }
        dimension => Err(GlobalConstraintEvalError::UnsupportedDestinationDimension { dimension }),
    }
}

fn canonical_number(value: u64) -> Result<Felt, GlobalConstraintEvalError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => {
            GlobalConstraintEvalError::NonCanonicalNumber { value }
        }
    })
}

fn scalar_ext(value: Felt) -> Ext3 {
    Ext3::new(value, Felt::ZERO, Felt::ZERO)
}

fn to_usize(value: u32) -> Result<usize, GlobalConstraintEvalError> {
    usize::try_from(value).map_err(|_| GlobalConstraintEvalError::LengthOverflow)
}

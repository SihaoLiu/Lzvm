use std::collections::BTreeMap;
use std::fmt;

use crate::constraint_program::{ConstraintEntry, ConstraintProgram};
use crate::expression_info::{
    BoundaryKind, CodeDestination, CodeOperand, CodeOperation, ConstraintCode, ExpressionCode,
    ExpressionInfo, OperationKind,
};
use crate::expression_program::{ExpressionEntry, ExpressionProgram};
use crate::hint_program::{regular_hint_program_from_expression_info, HintProgramError};
use crate::regular_program::RegularProgram;
use crate::setup_info::UnitSetupInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegularProgramLoweringError {
    EmptyCode {
        item: &'static str,
        index: usize,
    },
    UnsupportedOperation {
        operation: OperationKind,
    },
    UnsupportedSourceCount {
        operation: OperationKind,
        count: usize,
    },
    UnsupportedDestination {
        destination: CodeDestination,
    },
    UnsupportedOperand {
        operand: CodeOperand,
    },
    UnsupportedDimension {
        dimension: u32,
    },
    UnsupportedOperationShape {
        destination: u32,
        source0: u32,
        source1: u32,
    },
    MissingFrameBoundaryOffsets,
    InvalidFrameBoundary {
        offset_min: i64,
        offset_max: i64,
    },
    DomainSizeOverflow {
        n_bits: u32,
    },
    MissingTemporary {
        id: u32,
        dimension: u32,
    },
    MissingCommitmentColumn {
        id: u32,
    },
    MissingOpeningPoint {
        value: i64,
    },
    ValueOutOfRange {
        value: u32,
    },
    LengthOverflow,
    Hints(HintProgramError),
}

impl fmt::Display for RegularProgramLoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCode { item, index } => {
                write!(f, "{item} {index} has no operations to lower")
            }
            Self::UnsupportedOperation { operation } => {
                write!(f, "unsupported regular program operation: {operation:?}")
            }
            Self::UnsupportedSourceCount { operation, count } => write!(
                f,
                "unsupported source count {count} for regular program operation {operation:?}"
            ),
            Self::UnsupportedDestination { destination } => {
                write!(
                    f,
                    "unsupported regular program destination: {destination:?}"
                )
            }
            Self::UnsupportedOperand { operand } => {
                write!(f, "unsupported regular program operand: {operand:?}")
            }
            Self::UnsupportedDimension { dimension } => {
                write!(f, "unsupported regular program dimension: {dimension}")
            }
            Self::UnsupportedOperationShape {
                destination,
                source0,
                source1,
            } => write!(
                f,
                "unsupported regular program operation shape: destination {destination}, source0 {source0}, source1 {source1}"
            ),
            Self::MissingFrameBoundaryOffsets => {
                write!(f, "frame boundary is missing offset bounds")
            }
            Self::InvalidFrameBoundary {
                offset_min,
                offset_max,
            } => write!(
                f,
                "invalid frame boundary offsets: min {offset_min}, max {offset_max}"
            ),
            Self::DomainSizeOverflow { n_bits } => {
                write!(f, "regular program domain size overflows for n_bits {n_bits}")
            }
            Self::MissingTemporary { id, dimension } => {
                write!(f, "missing compact temporary id {id} with dimension {dimension}")
            }
            Self::MissingCommitmentColumn { id } => {
                write!(f, "missing commitment column {id}")
            }
            Self::MissingOpeningPoint { value } => {
                write!(f, "missing opening point {value}")
            }
            Self::ValueOutOfRange { value } => {
                write!(f, "regular program value does not fit in u16: {value}")
            }
            Self::LengthOverflow => write!(f, "regular program lowering length overflow"),
            Self::Hints(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RegularProgramLoweringError {}

impl From<HintProgramError> for RegularProgramLoweringError {
    fn from(error: HintProgramError) -> Self {
        Self::Hints(error)
    }
}

pub fn regular_program_from_expression_info(
    info: &ExpressionInfo,
    setup: &UnitSetupInfo,
) -> Result<RegularProgram, RegularProgramLoweringError> {
    let mut expression_numbers = Vec::new();
    let mut expression_program = ExpressionProgram {
        max_tmp1: 0,
        max_tmp3: 0,
        max_args: 0,
        max_ops: 0,
        entries: Vec::with_capacity(info.expressions.len()),
        ops: Vec::new(),
        args: Vec::new(),
        numbers: Vec::new(),
    };

    for (index, expression) in info.expressions.iter().enumerate() {
        let lowered = lower_expression_code(index, expression, setup, &mut expression_numbers)?;
        append_expression_entry(&mut expression_program, expression, lowered)?;
    }
    expression_program.numbers = expression_numbers;

    let mut constraint_numbers = Vec::new();
    let mut constraint_program = ConstraintProgram {
        entries: Vec::with_capacity(info.constraints.len()),
        ops: Vec::new(),
        args: Vec::new(),
        numbers: Vec::new(),
    };

    for (index, constraint) in info.constraints.iter().enumerate() {
        let lowered = lower_constraint_code(index, constraint, setup, &mut constraint_numbers)?;
        append_constraint_entry(&mut constraint_program, constraint, setup, lowered)?;
    }
    constraint_program.numbers = constraint_numbers;

    Ok(RegularProgram {
        expressions: expression_program,
        constraints: constraint_program,
        hints: regular_hint_program_from_expression_info(info)?,
    })
}

fn append_expression_entry(
    program: &mut ExpressionProgram,
    expression: &ExpressionCode,
    lowered: LoweredCode,
) -> Result<(), RegularProgramLoweringError> {
    let ops_offset = usize_to_u32(program.ops.len())?;
    let args_offset = usize_to_u32(program.args.len())?;
    let ops_count = usize_to_u32(lowered.ops.len())?;
    let args_count = usize_to_u32(lowered.args.len())?;

    program.max_tmp1 = program.max_tmp1.max(lowered.temp1_count);
    program.max_tmp3 = program.max_tmp3.max(lowered.temp3_count);
    program.max_args = program.max_args.max(args_count);
    program.max_ops = program.max_ops.max(ops_count);
    program.ops.extend(lowered.ops);
    program.args.extend(lowered.args);
    program.entries.push(ExpressionEntry {
        expression_id: expression.expression_id,
        destination_dimension: lowered.destination.dimension,
        destination_id: lowered.destination.entry_id,
        stage: expression.stage,
        temp1_count: lowered.temp1_count,
        temp3_count: lowered.temp3_count,
        ops_count,
        ops_offset,
        args_count,
        args_offset,
        source_line: expression.line.clone(),
    });
    Ok(())
}

fn append_constraint_entry(
    program: &mut ConstraintProgram,
    constraint: &ConstraintCode,
    setup: &UnitSetupInfo,
    lowered: LoweredCode,
) -> Result<(), RegularProgramLoweringError> {
    let ops_offset = usize_to_u32(program.ops.len())?;
    let args_offset = usize_to_u32(program.args.len())?;
    let ops_count = usize_to_u32(lowered.ops.len())?;
    let args_count = usize_to_u32(lowered.args.len())?;
    let (first_row, last_row) = constraint_row_bounds(constraint, setup)?;

    program.ops.extend(lowered.ops);
    program.args.extend(lowered.args);
    program.entries.push(ConstraintEntry {
        stage: constraint.stage,
        destination_dimension: lowered.destination.dimension,
        destination_id: lowered.destination.entry_id,
        first_row,
        last_row,
        temp1_count: lowered.temp1_count,
        temp3_count: lowered.temp3_count,
        ops_count,
        ops_offset,
        args_count,
        args_offset,
        intermediate: constraint.intermediate,
        source_line: constraint.line.clone(),
    });
    Ok(())
}

fn lower_expression_code(
    index: usize,
    expression: &ExpressionCode,
    setup: &UnitSetupInfo,
    numbers: &mut Vec<u64>,
) -> Result<LoweredCode, RegularProgramLoweringError> {
    lower_code("expression", index, &expression.operations, setup, numbers)
}

fn lower_constraint_code(
    index: usize,
    constraint: &ConstraintCode,
    setup: &UnitSetupInfo,
    numbers: &mut Vec<u64>,
) -> Result<LoweredCode, RegularProgramLoweringError> {
    lower_code("constraint", index, &constraint.operations, setup, numbers)
}

fn lower_code(
    item: &'static str,
    index: usize,
    operations: &[CodeOperation],
    setup: &UnitSetupInfo,
    numbers: &mut Vec<u64>,
) -> Result<LoweredCode, RegularProgramLoweringError> {
    let Some(last) = operations.last() else {
        return Err(RegularProgramLoweringError::EmptyCode { item, index });
    };

    let temporaries = TemporaryMap::build(operations)?;
    let mut ops = Vec::with_capacity(operations.len());
    let mut args = Vec::with_capacity(operations.len().saturating_mul(8));

    for operation in operations {
        lower_operation(operation, setup, &temporaries, numbers, &mut ops, &mut args)?;
    }

    Ok(LoweredCode {
        temp1_count: temporaries.count1,
        temp3_count: temporaries.count3,
        destination: lower_destination(&last.destination, &temporaries)?,
        ops,
        args,
    })
}

fn lower_operation(
    operation: &CodeOperation,
    setup: &UnitSetupInfo,
    temporaries: &TemporaryMap,
    numbers: &mut Vec<u64>,
    ops: &mut Vec<u8>,
    args: &mut Vec<u16>,
) -> Result<(), RegularProgramLoweringError> {
    if operation.op == OperationKind::Copy {
        return lower_copy_operation(operation, setup, temporaries, numbers, ops, args);
    }

    if operation.sources.len() != 2 {
        return Err(RegularProgramLoweringError::UnsupportedSourceCount {
            operation: operation.op,
            count: operation.sources.len(),
        });
    }

    let destination = lower_destination(&operation.destination, temporaries)?;
    let mut sources = operation.sources.iter().enumerate().collect::<Vec<_>>();
    sources.sort_by(|left, right| compare_sources(left.1, right.1));

    let kind = match operation.op {
        OperationKind::Add => 0,
        OperationKind::Sub if sources[0].0 == 0 => 1,
        OperationKind::Sub => 3,
        OperationKind::Mul => 2,
        OperationKind::Copy => unreachable!("copy operations are lowered before binary operations"),
    };

    let source0 = lower_source(sources[0].1, setup, temporaries, numbers)?;
    let source1 = lower_source(sources[1].1, setup, temporaries, numbers)?;
    let shape = operation_shape(destination.dimension, source0.dimension, source1.dimension)?;
    ops.push(shape);
    args.push(kind);
    args.push(u32_to_u16(destination.argument_offset)?);
    args.extend(source0.fields);
    args.extend(source1.fields);
    Ok(())
}

fn lower_copy_operation(
    operation: &CodeOperation,
    setup: &UnitSetupInfo,
    temporaries: &TemporaryMap,
    numbers: &mut Vec<u64>,
    ops: &mut Vec<u8>,
    args: &mut Vec<u16>,
) -> Result<(), RegularProgramLoweringError> {
    if operation.sources.len() != 1 {
        return Err(RegularProgramLoweringError::UnsupportedSourceCount {
            operation: operation.op,
            count: operation.sources.len(),
        });
    }

    let destination = lower_destination(&operation.destination, temporaries)?;
    let source = lower_source(&operation.sources[0], setup, temporaries, numbers)?;
    let zero = zero_source(setup, numbers)?;
    let shape = operation_shape(destination.dimension, source.dimension, zero.dimension)?;

    ops.push(shape);
    args.push(0);
    args.push(u32_to_u16(destination.argument_offset)?);
    args.extend(source.fields);
    args.extend(zero.fields);
    Ok(())
}

fn lower_destination(
    destination: &CodeDestination,
    temporaries: &TemporaryMap,
) -> Result<LoweredDestination, RegularProgramLoweringError> {
    let CodeDestination::Temporary { id, dimension } = destination else {
        return Err(RegularProgramLoweringError::UnsupportedDestination {
            destination: destination.clone(),
        });
    };

    match *dimension {
        1 => {
            let entry_id = temporaries.compact_id(*id, 1)?;
            Ok(LoweredDestination {
                dimension: 1,
                entry_id,
                argument_offset: entry_id,
            })
        }
        3 => {
            let entry_id = temporaries.compact_id(*id, 3)?;
            Ok(LoweredDestination {
                dimension: 3,
                entry_id,
                argument_offset: entry_id
                    .checked_mul(3)
                    .ok_or(RegularProgramLoweringError::LengthOverflow)?,
            })
        }
        dimension => Err(RegularProgramLoweringError::UnsupportedDimension { dimension }),
    }
}

fn lower_source(
    operand: &CodeOperand,
    setup: &UnitSetupInfo,
    temporaries: &TemporaryMap,
    numbers: &mut Vec<u64>,
) -> Result<SourceArg, RegularProgramLoweringError> {
    let base_buffer = base_buffer(setup)?;
    match operand {
        CodeOperand::Temporary { id, dimension } => match *dimension {
            1 => Ok(SourceArg {
                dimension: 1,
                fields: [
                    u32_to_u16(base_buffer)?,
                    u32_to_u16(temporaries.compact_id(*id, 1)?)?,
                    0,
                ],
            }),
            3 => Ok(SourceArg {
                dimension: 3,
                fields: [
                    u32_to_u16(base_buffer + 1)?,
                    u32_to_u16(
                        temporaries
                            .compact_id(*id, 3)?
                            .checked_mul(3)
                            .ok_or(RegularProgramLoweringError::LengthOverflow)?,
                    )?,
                    0,
                ],
            }),
            dimension => Err(RegularProgramLoweringError::UnsupportedDimension { dimension }),
        },
        CodeOperand::Number { value, dimension } => {
            if *dimension != 1 {
                return Err(RegularProgramLoweringError::UnsupportedDimension {
                    dimension: *dimension,
                });
            }
            Ok(SourceArg {
                dimension: 1,
                fields: [
                    u32_to_u16(base_buffer + 3)?,
                    u32_to_u16(intern_number(numbers, *value)?)?,
                    0,
                ],
            })
        }
        CodeOperand::Public { id, dimension } => Ok(SourceArg {
            dimension: *dimension,
            fields: [u32_to_u16(add_u32(base_buffer, 2)?)?, u32_to_u16(*id)?, 0],
        }),
        CodeOperand::Commitment {
            id,
            prime,
            dimension,
        } => {
            let column = setup
                .commitment_columns
                .get(
                    usize::try_from(*id)
                        .map_err(|_| RegularProgramLoweringError::LengthOverflow)?,
                )
                .ok_or(RegularProgramLoweringError::MissingCommitmentColumn { id: *id })?;
            Ok(SourceArg {
                dimension: *dimension,
                fields: [
                    u32_to_u16(column.stage)?,
                    u32_to_u16(column.stage_position)?,
                    u32_to_u16(opening_point_index(setup, prime.unwrap_or(0))?)?,
                ],
            })
        }
        CodeOperand::Challenge { id, dimension, .. } => Ok(SourceArg {
            dimension: *dimension,
            fields: [
                u32_to_u16(add_u32(base_buffer, 7)?)?,
                u32_to_u16(mul_u32(*id, 3)?)?,
                0,
            ],
        }),
        _ => Err(RegularProgramLoweringError::UnsupportedOperand {
            operand: operand.clone(),
        }),
    }
}

fn zero_source(
    setup: &UnitSetupInfo,
    numbers: &mut Vec<u64>,
) -> Result<SourceArg, RegularProgramLoweringError> {
    Ok(SourceArg {
        dimension: 1,
        fields: [
            u32_to_u16(add_u32(base_buffer(setup)?, 3)?)?,
            u32_to_u16(intern_number(numbers, 0)?)?,
            0,
        ],
    })
}

fn base_buffer(setup: &UnitSetupInfo) -> Result<u32, RegularProgramLoweringError> {
    1_u32
        .checked_add(setup.n_stages)
        .and_then(|value| value.checked_add(3))
        .ok_or(RegularProgramLoweringError::LengthOverflow)
}

fn operation_shape(
    destination: u32,
    source0: u32,
    source1: u32,
) -> Result<u8, RegularProgramLoweringError> {
    match (destination, source0, source1) {
        (1, 1, 1) => Ok(0),
        (3, 3, 1) => Ok(1),
        (3, 3, 3) => Ok(2),
        (destination, source0, source1) => {
            Err(RegularProgramLoweringError::UnsupportedOperationShape {
                destination,
                source0,
                source1,
            })
        }
    }
}

fn constraint_row_bounds(
    constraint: &ConstraintCode,
    setup: &UnitSetupInfo,
) -> Result<(u32, u32), RegularProgramLoweringError> {
    let domain_size = 1_u64.checked_shl(setup.stark.n_bits).ok_or(
        RegularProgramLoweringError::DomainSizeOverflow {
            n_bits: setup.stark.n_bits,
        },
    )?;
    let domain_size_u32 = u32::try_from(domain_size).map_err(|_| {
        RegularProgramLoweringError::DomainSizeOverflow {
            n_bits: setup.stark.n_bits,
        }
    })?;

    match constraint.boundary {
        BoundaryKind::EveryRow => Ok((0, domain_size_u32)),
        BoundaryKind::FirstRow | BoundaryKind::FinalProof => Ok((0, 1)),
        BoundaryKind::LastRow => Ok((domain_size_u32.saturating_sub(1), domain_size_u32)),
        BoundaryKind::EveryFrame => {
            let offset_min = constraint
                .offset_min
                .ok_or(RegularProgramLoweringError::MissingFrameBoundaryOffsets)?;
            let offset_max = constraint
                .offset_max
                .ok_or(RegularProgramLoweringError::MissingFrameBoundaryOffsets)?;
            if offset_max < 0 {
                return Err(RegularProgramLoweringError::InvalidFrameBoundary {
                    offset_min,
                    offset_max,
                });
            }
            let last = i128::from(domain_size_u32) - i128::from(offset_max);
            if last < 0 {
                return Err(RegularProgramLoweringError::InvalidFrameBoundary {
                    offset_min,
                    offset_max,
                });
            }
            let first = if offset_min < 0 {
                0
            } else {
                u32::try_from(offset_min).map_err(|_| {
                    RegularProgramLoweringError::InvalidFrameBoundary {
                        offset_min,
                        offset_max,
                    }
                })?
            };
            Ok((
                first,
                u32::try_from(last).map_err(|_| RegularProgramLoweringError::LengthOverflow)?,
            ))
        }
    }
}

fn compare_sources(left: &CodeOperand, right: &CodeOperand) -> std::cmp::Ordering {
    operand_dimension(right)
        .cmp(&operand_dimension(left))
        .then_with(|| operand_order(left).cmp(&operand_order(right)))
}

fn operand_dimension(operand: &CodeOperand) -> u32 {
    match operand {
        CodeOperand::Temporary { dimension, .. }
        | CodeOperand::Number { dimension, .. }
        | CodeOperand::Evaluation { dimension, .. }
        | CodeOperand::Challenge { dimension, .. }
        | CodeOperand::Public { dimension, .. }
        | CodeOperand::Constant { dimension, .. }
        | CodeOperand::Commitment { dimension, .. }
        | CodeOperand::BoundaryZerofier { dimension, .. }
        | CodeOperand::ProofValue { dimension, .. }
        | CodeOperand::OpeningDenominator { dimension, .. }
        | CodeOperand::CustomCommitment { dimension, .. }
        | CodeOperand::AirGroupValue { dimension, .. }
        | CodeOperand::AirValue { dimension, .. } => *dimension,
    }
}

fn operand_order(operand: &CodeOperand) -> u8 {
    match operand {
        CodeOperand::Constant { .. } | CodeOperand::BoundaryZerofier { .. } => 0,
        CodeOperand::Commitment { dimension: 1, .. } => 0,
        CodeOperand::CustomCommitment { dimension: 1, .. } => 0,
        CodeOperand::Temporary { dimension: 1, .. } => 1,
        CodeOperand::Public { .. } => 2,
        CodeOperand::Number { .. } => 3,
        CodeOperand::AirValue { dimension: 1, .. } => 4,
        CodeOperand::ProofValue { dimension: 1, .. } => 5,
        CodeOperand::Commitment { .. }
        | CodeOperand::CustomCommitment { .. }
        | CodeOperand::OpeningDenominator { .. } => 6,
        CodeOperand::Temporary { .. } => 7,
        CodeOperand::AirValue { .. } => 8,
        CodeOperand::AirGroupValue { .. } => 9,
        CodeOperand::ProofValue { .. } => 10,
        CodeOperand::Challenge { .. } => 11,
        CodeOperand::Evaluation { .. } => 12,
    }
}

fn intern_number(numbers: &mut Vec<u64>, value: u64) -> Result<u32, RegularProgramLoweringError> {
    if let Some(index) = numbers.iter().position(|existing| *existing == value) {
        return usize_to_u32(index);
    }
    let index = numbers.len();
    numbers.push(value);
    usize_to_u32(index)
}

fn opening_point_index(
    setup: &UnitSetupInfo,
    value: i64,
) -> Result<u32, RegularProgramLoweringError> {
    setup
        .opening_points
        .iter()
        .position(|candidate| *candidate == value)
        .map(usize_to_u32)
        .transpose()?
        .ok_or(RegularProgramLoweringError::MissingOpeningPoint { value })
}

fn add_u32(left: u32, right: u32) -> Result<u32, RegularProgramLoweringError> {
    left.checked_add(right)
        .ok_or(RegularProgramLoweringError::LengthOverflow)
}

fn mul_u32(left: u32, right: u32) -> Result<u32, RegularProgramLoweringError> {
    left.checked_mul(right)
        .ok_or(RegularProgramLoweringError::LengthOverflow)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoweredCode {
    temp1_count: u32,
    temp3_count: u32,
    destination: LoweredDestination,
    ops: Vec<u8>,
    args: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoweredDestination {
    dimension: u32,
    entry_id: u32,
    argument_offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceArg {
    dimension: u32,
    fields: [u16; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TemporaryMap {
    one: BTreeMap<u32, u32>,
    three: BTreeMap<u32, u32>,
    count1: u32,
    count3: u32,
}

impl TemporaryMap {
    fn build(operations: &[CodeOperation]) -> Result<Self, RegularProgramLoweringError> {
        let mut one = Vec::new();
        let mut three = Vec::new();
        for (index, operation) in operations.iter().enumerate() {
            observe_destination(&operation.destination, index, &mut one, &mut three)?;
            for operand in &operation.sources {
                observe_operand(operand, index, &mut one, &mut three)?;
            }
        }
        let one = compact_segments(one)?;
        let three = compact_segments(three)?;
        Ok(Self {
            count1: usize_to_u32(
                one.values()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
            )?,
            count3: usize_to_u32(
                three
                    .values()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
            )?,
            one,
            three,
        })
    }

    fn compact_id(&self, id: u32, dimension: u32) -> Result<u32, RegularProgramLoweringError> {
        let map = match dimension {
            1 => &self.one,
            3 => &self.three,
            dimension => {
                return Err(RegularProgramLoweringError::UnsupportedDimension { dimension });
            }
        };
        map.get(&id)
            .copied()
            .ok_or(RegularProgramLoweringError::MissingTemporary { id, dimension })
    }
}

fn observe_destination(
    destination: &CodeDestination,
    index: usize,
    one: &mut Vec<Segment>,
    three: &mut Vec<Segment>,
) -> Result<(), RegularProgramLoweringError> {
    if let CodeDestination::Temporary { id, dimension } = destination {
        observe_temporary(*id, *dimension, index, one, three)?;
    }
    Ok(())
}

fn observe_operand(
    operand: &CodeOperand,
    index: usize,
    one: &mut Vec<Segment>,
    three: &mut Vec<Segment>,
) -> Result<(), RegularProgramLoweringError> {
    if let CodeOperand::Temporary { id, dimension } = operand {
        observe_temporary(*id, *dimension, index, one, three)?;
    }
    Ok(())
}

fn observe_temporary(
    id: u32,
    dimension: u32,
    index: usize,
    one: &mut Vec<Segment>,
    three: &mut Vec<Segment>,
) -> Result<(), RegularProgramLoweringError> {
    let segments = match dimension {
        1 => one,
        3 => three,
        dimension => {
            return Err(RegularProgramLoweringError::UnsupportedDimension { dimension });
        }
    };
    if let Some(segment) = segments.iter_mut().find(|segment| segment.id == id) {
        segment.end = segment.end.max(index);
    } else {
        segments.push(Segment {
            start: index,
            end: index,
            id,
        });
    }
    Ok(())
}

fn compact_segments(
    mut segments: Vec<Segment>,
) -> Result<BTreeMap<u32, u32>, RegularProgramLoweringError> {
    segments.sort_by_key(|segment| (segment.end, segment.start, segment.id));
    let mut subsets: Vec<Vec<Segment>> = Vec::new();
    for segment in segments {
        let mut closest_subset = None;
        let mut min_distance = usize::MAX;
        for (index, subset) in subsets.iter().enumerate() {
            let last = subset
                .last()
                .ok_or(RegularProgramLoweringError::LengthOverflow)?;
            if segments_intersect(segment, *last) {
                continue;
            }
            let distance = last.end.abs_diff(segment.start);
            if distance < min_distance {
                min_distance = distance;
                closest_subset = Some(index);
            }
        }
        if let Some(index) = closest_subset {
            subsets[index].push(segment);
        } else {
            subsets.push(vec![segment]);
        }
    }

    let mut out = BTreeMap::new();
    for (index, subset) in subsets.iter().enumerate() {
        let compact_id = usize_to_u32(index)?;
        for segment in subset {
            out.insert(segment.id, compact_id);
        }
    }
    Ok(out)
}

fn segments_intersect(left: Segment, right: Segment) -> bool {
    right.start < left.end && left.start < right.end
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Segment {
    start: usize,
    end: usize,
    id: u32,
}

fn usize_to_u32(value: usize) -> Result<u32, RegularProgramLoweringError> {
    u32::try_from(value).map_err(|_| RegularProgramLoweringError::LengthOverflow)
}

fn u32_to_u16(value: u32) -> Result<u16, RegularProgramLoweringError> {
    u16::try_from(value).map_err(|_| RegularProgramLoweringError::ValueOutOfRange { value })
}

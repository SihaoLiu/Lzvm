use std::collections::BTreeMap;
use std::fmt;

use lzvm_artifacts::verifier_info::{
    VerifierCode, VerifierDestination, VerifierOperand, VerifierOperationKind,
};
use lzvm_field::{Ext3, Felt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifierOpenedStage<'a> {
    pub stage_index: u32,
    pub values: &'a [Felt],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifierCommitmentColumn {
    pub stage_index: u32,
    pub position: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VerifierEvalInputs<'a> {
    pub challenges: &'a [Ext3],
    pub evaluations: &'a [Ext3],
    pub publics: &'a [Felt],
    pub constants: &'a [Felt],
    pub commitments: &'a [Felt],
    pub opened_stages: &'a [VerifierOpenedStage<'a>],
    pub commitment_columns: &'a [VerifierCommitmentColumn],
    pub zi: &'a [Ext3],
    pub proof_values: &'a [Ext3],
    pub x_div_x_sub: &'a [Ext3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierEvalError {
    MissingResultTemporary,
    InvalidReference,
    MissingReferenceField {
        field: &'static str,
    },
    UnsupportedDestination {
        kind: String,
    },
    UnknownSourceKind {
        kind: String,
    },
    InvalidNumber {
        value: String,
    },
    UnsupportedDimension {
        dimension: usize,
    },
    MissingOpenedStage {
        stage_index: u32,
    },
    TemporaryIndexOutOfRange {
        index: usize,
        len: usize,
    },
    SourceIndexOutOfRange {
        kind: String,
        index: usize,
        len: usize,
    },
    OperationArityMismatch {
        op: VerifierOperationKind,
        expected: usize,
        found: usize,
    },
    TemporaryReadBeforeWrite {
        temporary_id: u32,
        dimension: u32,
        operation_index: usize,
    },
    TemporaryDimensionMismatch {
        temporary_id: u32,
        expected_dimension: u32,
        found_dimension: u32,
        operation_index: usize,
    },
}

impl fmt::Display for VerifierEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingResultTemporary => {
                write!(f, "verifier evaluation has no result temporary")
            }
            Self::InvalidReference => write!(f, "invalid verifier evaluation reference"),
            Self::MissingReferenceField { field } => {
                write!(f, "missing verifier evaluation reference field: {field}")
            }
            Self::UnsupportedDestination { kind } => {
                write!(f, "unsupported verifier evaluation destination: {kind}")
            }
            Self::UnknownSourceKind { kind } => {
                write!(f, "unknown verifier evaluation source kind: {kind}")
            }
            Self::InvalidNumber { value } => {
                write!(f, "invalid verifier evaluation number: {value}")
            }
            Self::UnsupportedDimension { dimension } => {
                write!(f, "unsupported verifier evaluation dimension: {dimension}")
            }
            Self::MissingOpenedStage { stage_index } => {
                write!(f, "missing verifier evaluation opened stage: {stage_index}")
            }
            Self::TemporaryIndexOutOfRange { index, len } => write!(
                f,
                "verifier evaluation temporary index {index} is outside temporary count {len}"
            ),
            Self::SourceIndexOutOfRange { kind, index, len } => write!(
                f,
                "verifier evaluation {kind} index {index} is outside input count {len}"
            ),
            Self::OperationArityMismatch {
                op,
                expected,
                found,
            } => write!(
                f,
                "verifier evaluation operation {op:?} expected {expected} sources, found {found}"
            ),
            Self::TemporaryReadBeforeWrite {
                temporary_id,
                dimension,
                operation_index,
            } => write!(
                f,
                "verifier evaluation temporary {temporary_id} with dimension {dimension} is read before write at operation {operation_index}"
            ),
            Self::TemporaryDimensionMismatch {
                temporary_id,
                expected_dimension,
                found_dimension,
                operation_index,
            } => write!(
                f,
                "verifier evaluation temporary {temporary_id} has dimension {found_dimension} at operation {operation_index}, expected {expected_dimension}"
            ),
        }
    }
}

impl std::error::Error for VerifierEvalError {}

pub fn evaluate_verifier_code(
    code: &VerifierCode,
    inputs: &VerifierEvalInputs<'_>,
) -> Result<Ext3, VerifierEvalError> {
    validate_temporary_metadata(code)?;
    let mut temporaries = vec![Ext3::ZERO; code.temporary_count as usize];
    for operation in &code.operations {
        let destination = destination_index(&operation.destination, temporaries.len())?;
        let value = match operation.op {
            VerifierOperationKind::Copy => {
                expect_arity(operation.op, operation.sources.len(), 1)?;
                resolve_source(&operation.sources[0], inputs, &temporaries)?
            }
            VerifierOperationKind::Add => {
                expect_arity(operation.op, operation.sources.len(), 2)?;
                resolve_source(&operation.sources[0], inputs, &temporaries)?
                    + resolve_source(&operation.sources[1], inputs, &temporaries)?
            }
            VerifierOperationKind::Sub => {
                expect_arity(operation.op, operation.sources.len(), 2)?;
                resolve_source(&operation.sources[0], inputs, &temporaries)?
                    - resolve_source(&operation.sources[1], inputs, &temporaries)?
            }
            VerifierOperationKind::Mul => {
                expect_arity(operation.op, operation.sources.len(), 2)?;
                resolve_source(&operation.sources[0], inputs, &temporaries)?
                    * resolve_source(&operation.sources[1], inputs, &temporaries)?
            }
        };
        temporaries[destination] = value;
    }
    temporaries
        .first()
        .copied()
        .ok_or(VerifierEvalError::MissingResultTemporary)
}

fn validate_temporary_metadata(code: &VerifierCode) -> Result<(), VerifierEvalError> {
    let mut temporary_dimensions = BTreeMap::new();
    for (operation_index, operation) in code.operations.iter().enumerate() {
        for source in &operation.sources {
            if let VerifierOperand::Temporary { id, dimension } = source {
                validate_temporary_read(&temporary_dimensions, *id, *dimension, operation_index)?;
            }
        }
        define_temporary(
            &mut temporary_dimensions,
            operation.destination.temporary_id,
            operation.destination.dimension,
            operation_index,
        )?;
    }
    Ok(())
}

fn validate_temporary_read(
    temporary_dimensions: &BTreeMap<u32, u32>,
    temporary_id: u32,
    dimension: u32,
    operation_index: usize,
) -> Result<(), VerifierEvalError> {
    match temporary_dimensions.get(&temporary_id) {
        Some(expected_dimension) if *expected_dimension == dimension => Ok(()),
        Some(expected_dimension) => Err(VerifierEvalError::TemporaryDimensionMismatch {
            temporary_id,
            expected_dimension: *expected_dimension,
            found_dimension: dimension,
            operation_index,
        }),
        None => Err(VerifierEvalError::TemporaryReadBeforeWrite {
            temporary_id,
            dimension,
            operation_index,
        }),
    }
}

fn define_temporary(
    temporary_dimensions: &mut BTreeMap<u32, u32>,
    temporary_id: u32,
    dimension: u32,
    operation_index: usize,
) -> Result<(), VerifierEvalError> {
    match temporary_dimensions.get(&temporary_id) {
        Some(expected_dimension) if *expected_dimension == dimension => Ok(()),
        Some(expected_dimension) => Err(VerifierEvalError::TemporaryDimensionMismatch {
            temporary_id,
            expected_dimension: *expected_dimension,
            found_dimension: dimension,
            operation_index,
        }),
        None => {
            temporary_dimensions.insert(temporary_id, dimension);
            Ok(())
        }
    }
}

fn expect_arity(
    op: VerifierOperationKind,
    found: usize,
    expected: usize,
) -> Result<(), VerifierEvalError> {
    if found == expected {
        Ok(())
    } else {
        Err(VerifierEvalError::OperationArityMismatch {
            op,
            expected,
            found,
        })
    }
}

fn destination_index(
    reference: &VerifierDestination,
    temporary_count: usize,
) -> Result<usize, VerifierEvalError> {
    let index =
        usize::try_from(reference.temporary_id).map_err(|_| VerifierEvalError::InvalidReference)?;
    check_index("tmp", index, temporary_count)?;
    Ok(index)
}

fn resolve_source(
    reference: &VerifierOperand,
    inputs: &VerifierEvalInputs<'_>,
    temporaries: &[Ext3],
) -> Result<Ext3, VerifierEvalError> {
    match reference {
        VerifierOperand::Temporary { id, .. } => {
            let index = to_usize(*id)?;
            read_ext3("tmp", index, temporaries)
        }
        VerifierOperand::Number { value, .. } => Ok(extension_from_scalar(Felt::from_u64(*value))),
        VerifierOperand::Evaluation { id, .. } => {
            let index = to_usize(*id)?;
            read_ext3("eval", index, inputs.evaluations)
        }
        VerifierOperand::Challenge { id, .. } => {
            let index = to_usize(*id)?;
            read_ext3("challenge", index, inputs.challenges)
        }
        VerifierOperand::Public { id, .. } => {
            let index = to_usize(*id)?;
            let value = *inputs.publics.get(index).ok_or_else(|| {
                VerifierEvalError::SourceIndexOutOfRange {
                    kind: "public".to_owned(),
                    index,
                    len: inputs.publics.len(),
                }
            })?;
            Ok(extension_from_scalar(value))
        }
        VerifierOperand::Constant { id, dimension } => read_felt_vector(
            "const",
            to_usize(*id)?,
            to_usize(*dimension)?,
            inputs.constants,
        ),
        VerifierOperand::Commitment { id, dimension } => {
            read_commitment_vector(to_usize(*id)?, to_usize(*dimension)?, inputs)
        }
        VerifierOperand::BoundaryZerofier { id, .. } => {
            let index = to_usize(*id)?;
            read_ext3("Zi", index, inputs.zi)
        }
        VerifierOperand::ProofValue { id, .. } => {
            let index = to_usize(*id)?;
            read_ext3("proofvalue", index, inputs.proof_values)
        }
        VerifierOperand::OpeningDenominator { id, .. } => {
            let index = to_usize(*id)?;
            read_ext3("xDivXSub", index, inputs.x_div_x_sub)
        }
    }
}

fn read_ext3(kind: &str, index: usize, values: &[Ext3]) -> Result<Ext3, VerifierEvalError> {
    values
        .get(index)
        .copied()
        .ok_or_else(|| VerifierEvalError::SourceIndexOutOfRange {
            kind: kind.to_owned(),
            index,
            len: values.len(),
        })
}

fn read_felt_vector(
    kind: &str,
    index: usize,
    dimension: usize,
    values: &[Felt],
) -> Result<Ext3, VerifierEvalError> {
    match dimension {
        1 => {
            let value =
                *values
                    .get(index)
                    .ok_or_else(|| VerifierEvalError::SourceIndexOutOfRange {
                        kind: kind.to_owned(),
                        index,
                        len: values.len(),
                    })?;
            Ok(extension_from_scalar(value))
        }
        3 => {
            let end = index
                .checked_add(3)
                .ok_or(VerifierEvalError::SourceIndexOutOfRange {
                    kind: kind.to_owned(),
                    index,
                    len: values.len(),
                })?;
            if end > values.len() {
                return Err(VerifierEvalError::SourceIndexOutOfRange {
                    kind: kind.to_owned(),
                    index,
                    len: values.len(),
                });
            }
            Ok(Ext3::new(
                values[index],
                values[index + 1],
                values[index + 2],
            ))
        }
        _ => Err(VerifierEvalError::UnsupportedDimension { dimension }),
    }
}

fn read_commitment_vector(
    index: usize,
    dimension: usize,
    inputs: &VerifierEvalInputs<'_>,
) -> Result<Ext3, VerifierEvalError> {
    if inputs.commitment_columns.is_empty() || inputs.opened_stages.is_empty() {
        return read_felt_vector("cm", index, dimension, inputs.commitments);
    }

    let column = inputs.commitment_columns.get(index).ok_or_else(|| {
        VerifierEvalError::SourceIndexOutOfRange {
            kind: "cm".to_owned(),
            index,
            len: inputs.commitment_columns.len(),
        }
    })?;
    let stage = opened_stage_for_column(column, inputs.opened_stages)?;

    read_felt_vector("cm", column.position, dimension, stage.values)
}

fn opened_stage_for_column<'stage, 'values>(
    column: &VerifierCommitmentColumn,
    opened_stages: &'stage [VerifierOpenedStage<'values>],
) -> Result<&'stage VerifierOpenedStage<'values>, VerifierEvalError> {
    if let Ok(stage_index) = usize::try_from(column.stage_index) {
        if let Some(stage_slot) = stage_index.checked_sub(1) {
            if let Some(stage) = opened_stages
                .get(stage_slot)
                .filter(|stage| stage.stage_index == column.stage_index)
            {
                if opened_stages[..stage_slot]
                    .iter()
                    .all(|stage| stage.stage_index != column.stage_index)
                {
                    return Ok(stage);
                }
            }
        }
    }

    opened_stages
        .iter()
        .find(|stage| stage.stage_index == column.stage_index)
        .ok_or(VerifierEvalError::MissingOpenedStage {
            stage_index: column.stage_index,
        })
}

fn check_index(kind: &str, index: usize, len: usize) -> Result<(), VerifierEvalError> {
    if index < len {
        Ok(())
    } else {
        Err(VerifierEvalError::SourceIndexOutOfRange {
            kind: kind.to_owned(),
            index,
            len,
        })
    }
}

fn to_usize(value: u32) -> Result<usize, VerifierEvalError> {
    usize::try_from(value).map_err(|_| VerifierEvalError::InvalidReference)
}

fn extension_from_scalar(value: Felt) -> Ext3 {
    Ext3::new(value, Felt::ZERO, Felt::ZERO)
}

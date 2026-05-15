use std::fmt;

use lzvm_artifacts::verifier_info::{VerifierCode, VerifierOperationKind};
use lzvm_field::{Ext3, Felt};

#[derive(Debug, Clone, Copy, Default)]
pub struct VerifierEvalInputs<'a> {
    pub challenges: &'a [Ext3],
    pub evaluations: &'a [Ext3],
    pub publics: &'a [Felt],
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
        }
    }
}

impl std::error::Error for VerifierEvalError {}

pub fn evaluate_verifier_code(
    code: &VerifierCode,
    inputs: &VerifierEvalInputs<'_>,
) -> Result<Ext3, VerifierEvalError> {
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
    reference: &serde_json::Value,
    temporary_count: usize,
) -> Result<usize, VerifierEvalError> {
    let object = reference
        .as_object()
        .ok_or(VerifierEvalError::InvalidReference)?;
    let kind = string_field(object, "type")?;
    if kind != "tmp" {
        return Err(VerifierEvalError::UnsupportedDestination {
            kind: kind.to_owned(),
        });
    }
    let index = usize_field(object, "id")?;
    check_index("tmp", index, temporary_count)?;
    Ok(index)
}

fn resolve_source(
    reference: &serde_json::Value,
    inputs: &VerifierEvalInputs<'_>,
    temporaries: &[Ext3],
) -> Result<Ext3, VerifierEvalError> {
    let object = reference
        .as_object()
        .ok_or(VerifierEvalError::InvalidReference)?;
    let kind = string_field(object, "type")?;
    match kind {
        "tmp" => {
            let index = usize_field(object, "id")?;
            read_ext3("tmp", index, temporaries)
        }
        "number" => read_number(object),
        "eval" => {
            let index = usize_field(object, "id")?;
            read_ext3("eval", index, inputs.evaluations)
        }
        "challenge" => {
            let index = usize_field(object, "id")?;
            read_ext3("challenge", index, inputs.challenges)
        }
        "public" => {
            let index = usize_field(object, "id")?;
            let value = *inputs.publics.get(index).ok_or_else(|| {
                VerifierEvalError::SourceIndexOutOfRange {
                    kind: "public".to_owned(),
                    index,
                    len: inputs.publics.len(),
                }
            })?;
            Ok(extension_from_scalar(value))
        }
        "Zi" => {
            let index = match optional_usize_field(object, "boundaryId")? {
                Some(index) => index,
                None => optional_usize_field(object, "id")?
                    .ok_or(VerifierEvalError::MissingReferenceField { field: "id" })?,
            };
            read_ext3("Zi", index, inputs.zi)
        }
        "proofvalue" | "proofValue" => {
            let index = usize_field(object, "id")?;
            read_ext3(kind, index, inputs.proof_values)
        }
        "xDivXSub" | "xdivxsub" => {
            let index = usize_field(object, "id")?;
            read_ext3(kind, index, inputs.x_div_x_sub)
        }
        _ => Err(VerifierEvalError::UnknownSourceKind {
            kind: kind.to_owned(),
        }),
    }
}

fn string_field<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a str, VerifierEvalError> {
    object
        .get(field)
        .ok_or(VerifierEvalError::MissingReferenceField { field })?
        .as_str()
        .ok_or(VerifierEvalError::InvalidReference)
}

fn usize_field(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<usize, VerifierEvalError> {
    optional_usize_field(object, field)?.ok_or(VerifierEvalError::MissingReferenceField { field })
}

fn optional_usize_field(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<usize>, VerifierEvalError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .ok_or(VerifierEvalError::InvalidReference)
        })
        .transpose()
}

fn read_number(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<Ext3, VerifierEvalError> {
    let value = object
        .get("value")
        .ok_or(VerifierEvalError::MissingReferenceField { field: "value" })?;
    let number = if let Some(text) = value.as_str() {
        text.parse::<u64>()
            .map_err(|_| VerifierEvalError::InvalidNumber {
                value: text.to_owned(),
            })?
    } else {
        value
            .as_u64()
            .ok_or_else(|| VerifierEvalError::InvalidNumber {
                value: value.to_string(),
            })?
    };
    Ok(extension_from_scalar(Felt::from_u64(number)))
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

fn extension_from_scalar(value: Felt) -> Ext3 {
    Ext3::new(value, Felt::ZERO, Felt::ZERO)
}

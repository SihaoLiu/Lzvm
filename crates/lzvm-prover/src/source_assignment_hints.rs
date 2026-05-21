use lzvm_artifacts::hint_program::SOURCE_ASSIGNMENT_CHECK_HINT;
use lzvm_field::Felt;

use crate::hint_eval::{ResolvedHint, ResolvedHintField, ResolvedHintPayload};
use crate::witness_execution::ProveWitnessCommitmentError;

pub(crate) fn validate_source_assignment_hints(
    unit_index: usize,
    row: usize,
    hints: &[ResolvedHint],
) -> Result<(), ProveWitnessCommitmentError> {
    for hint in hints {
        if hint.name != SOURCE_ASSIGNMENT_CHECK_HINT {
            continue;
        }
        let target = source_assignment_scalar_field(unit_index, row, hint, "target")?;
        let value = source_assignment_value(unit_index, row, hint)?;
        if target != value {
            return source_assignment_error(
                unit_index,
                format!(
                    "target {} does not match value {} at row {row}",
                    target.to_u64(),
                    value.to_u64()
                ),
            );
        }
    }
    Ok(())
}

fn source_assignment_value(
    unit_index: usize,
    row: usize,
    hint: &ResolvedHint,
) -> Result<Felt, ProveWitnessCommitmentError> {
    let scalar_value = source_assignment_field(hint, "value");
    let expression = source_assignment_field(hint, "expression");
    match (scalar_value, expression) {
        (Some(_), Some(_)) => source_assignment_error(
            unit_index,
            format!("assignment hint has both value and expression fields at row {row}"),
        ),
        (Some(_), None) => source_assignment_scalar_field(unit_index, row, hint, "value"),
        (None, Some(field)) => source_assignment_expression_field(unit_index, row, field),
        (None, None) => source_assignment_error(
            unit_index,
            format!("assignment hint is missing value field at row {row}"),
        ),
    }
}

fn source_assignment_expression_field(
    unit_index: usize,
    row: usize,
    field: &ResolvedHintField,
) -> Result<Felt, ProveWitnessCommitmentError> {
    let mut stack = Vec::new();
    for value in &field.values {
        match &value.payload {
            ResolvedHintPayload::Scalar(value) => stack.push(*value),
            ResolvedHintPayload::Text(op) => {
                if op == "not" {
                    let value = source_assignment_expression_pop(unit_index, row, op, &mut stack)?;
                    stack.push(Felt::from_u64(u64::from(value == Felt::ZERO)));
                    continue;
                }
                let right = source_assignment_expression_pop(unit_index, row, op, &mut stack)?;
                let left = source_assignment_expression_pop(unit_index, row, op, &mut stack)?;
                let result = match op.as_str() {
                    "add" => left + right,
                    "sub" => left - right,
                    "mul" => left * right,
                    "pow" => left.pow(right.to_u64()),
                    "div" => {
                        let inverse = right.inverse().ok_or_else(|| {
                            ProveWitnessCommitmentError::SourceAssignment {
                                unit_index,
                                message: format!("operator div has zero divisor at row {row}"),
                            }
                        })?;
                        left * inverse
                    }
                    "mod" => {
                        let divisor = right.to_u64();
                        if divisor == 0 {
                            return Err(ProveWitnessCommitmentError::SourceAssignment {
                                unit_index,
                                message: format!("operator mod has zero divisor at row {row}"),
                            });
                        }
                        Felt::from_u64(left.to_u64() % divisor)
                    }
                    "shl" => {
                        let shift = u32::try_from(right.to_u64()).map_err(|_| {
                            ProveWitnessCommitmentError::SourceAssignment {
                                unit_index,
                                message: format!("operator shl has invalid shift at row {row}"),
                            }
                        })?;
                        let shifted = left.to_u64().checked_shl(shift).ok_or_else(|| {
                            ProveWitnessCommitmentError::SourceAssignment {
                                unit_index,
                                message: format!("operator shl has invalid shift at row {row}"),
                            }
                        })?;
                        Felt::from_u64(shifted)
                    }
                    "shr" => {
                        let shift = u32::try_from(right.to_u64()).map_err(|_| {
                            ProveWitnessCommitmentError::SourceAssignment {
                                unit_index,
                                message: format!("operator shr has invalid shift at row {row}"),
                            }
                        })?;
                        let shifted = left.to_u64().checked_shr(shift).ok_or_else(|| {
                            ProveWitnessCommitmentError::SourceAssignment {
                                unit_index,
                                message: format!("operator shr has invalid shift at row {row}"),
                            }
                        })?;
                        Felt::from_u64(shifted)
                    }
                    "bitand" => Felt::from_u64(left.to_u64() & right.to_u64()),
                    "bitxor" => Felt::from_u64(left.to_u64() ^ right.to_u64()),
                    "bitor" => Felt::from_u64(left.to_u64() | right.to_u64()),
                    "and" => {
                        if left == Felt::ZERO {
                            left
                        } else {
                            right
                        }
                    }
                    "or" => {
                        if left == Felt::ZERO {
                            right
                        } else {
                            left
                        }
                    }
                    "lt" => Felt::from_u64(u64::from(left.to_u64() < right.to_u64())),
                    "le" => Felt::from_u64(u64::from(left.to_u64() <= right.to_u64())),
                    "gt" => Felt::from_u64(u64::from(left.to_u64() > right.to_u64())),
                    "ge" => Felt::from_u64(u64::from(left.to_u64() >= right.to_u64())),
                    "eq" => Felt::from_u64(u64::from(left == right)),
                    "ne" => Felt::from_u64(u64::from(left != right)),
                    _ => {
                        return source_assignment_error(
                            unit_index,
                            format!("unsupported expression operator {op} at row {row}"),
                        )
                    }
                };
                stack.push(result);
            }
            ResolvedHintPayload::Extension(_) => {
                return source_assignment_error(
                    unit_index,
                    format!("expression field contains an extension value at row {row}"),
                )
            }
        }
    }

    if stack.len() != 1 {
        return source_assignment_error(
            unit_index,
            format!(
                "expression field leaves {} values on the stack at row {row}",
                stack.len()
            ),
        );
    }
    Ok(stack[0])
}

fn source_assignment_expression_pop(
    unit_index: usize,
    row: usize,
    op: &str,
    stack: &mut Vec<Felt>,
) -> Result<Felt, ProveWitnessCommitmentError> {
    stack
        .pop()
        .ok_or_else(|| ProveWitnessCommitmentError::SourceAssignment {
            unit_index,
            message: format!("operator {op} has too few operands at row {row}"),
        })
}

fn source_assignment_scalar_field(
    unit_index: usize,
    row: usize,
    hint: &ResolvedHint,
    field_name: &str,
) -> Result<Felt, ProveWitnessCommitmentError> {
    let field = source_assignment_field(hint, field_name).ok_or_else(|| {
        source_assignment_message(unit_index, format!("missing {field_name} field"), row)
    })?;
    if field.values.len() != 1 {
        return source_assignment_error(
            unit_index,
            format!(
                "field {field_name} has {} values at row {row}",
                field.values.len()
            ),
        );
    }
    match field.values[0].payload {
        ResolvedHintPayload::Scalar(value) => Ok(value),
        _ => source_assignment_error(
            unit_index,
            format!("field {field_name} is not scalar at row {row}"),
        ),
    }
}

fn source_assignment_field<'a>(
    hint: &'a ResolvedHint,
    field_name: &str,
) -> Option<&'a ResolvedHintField> {
    hint.fields.iter().find(|field| field.name == field_name)
}

fn source_assignment_message(
    unit_index: usize,
    message: impl Into<String>,
    row: usize,
) -> ProveWitnessCommitmentError {
    ProveWitnessCommitmentError::SourceAssignment {
        unit_index,
        message: format!("{} at row {row}", message.into()),
    }
}

fn source_assignment_error<T>(
    unit_index: usize,
    message: impl Into<String>,
) -> Result<T, ProveWitnessCommitmentError> {
    Err(ProveWitnessCommitmentError::SourceAssignment {
        unit_index,
        message: message.into(),
    })
}

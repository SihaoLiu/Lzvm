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
        let value = source_assignment_scalar_field(unit_index, row, hint, "value")?;
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

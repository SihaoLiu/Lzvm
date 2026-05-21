use std::collections::BTreeMap;
use std::fmt;

use lzvm_artifacts::hint_program::{
    source_lookup_hint_name, SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT,
};
use lzvm_field::Felt;

use crate::hint_eval::{ResolvedHint, ResolvedHintField, ResolvedHintPayload};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceLookupBalance {
    entries: BTreeMap<SourceLookupKey, Felt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceLookupHintError {
    Unit { unit_index: usize, message: String },
    Set { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourceLookupKey {
    qualifiers: Vec<(String, SourceLookupPayload)>,
    values: Vec<SourceLookupPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SourceLookupPayload {
    Scalar(u64),
    Extension([u64; 3]),
    Text(String),
}

impl SourceLookupBalance {
    pub(crate) fn absorb(
        &mut self,
        unit_index: usize,
        row: usize,
        hints: &[ResolvedHint],
    ) -> Result<(), SourceLookupHintError> {
        for hint in hints {
            if !source_lookup_hint_name(&hint.name) {
                continue;
            }
            let key = source_lookup_key(unit_index, row, hint)?;
            let weight = source_lookup_weight(unit_index, row, hint)?;
            let entry = self.entries.entry(key).or_insert(Felt::ZERO);
            match hint.name.as_str() {
                SOURCE_LOOKUP_PROVES_HINT => *entry = *entry + weight,
                SOURCE_LOOKUP_ASSUMES_HINT => *entry = *entry - weight,
                _ => {
                    return source_lookup_error(
                        unit_index,
                        format!("unsupported lookup hint {} at row {row}", hint.name),
                    )
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate(self, unit_index: usize) -> Result<(), SourceLookupHintError> {
        for (key, balance) in self.entries {
            if balance != Felt::ZERO {
                return source_lookup_error(
                    unit_index,
                    format!(
                        "unbalanced lookup bus {} tuple {} has net weight {}",
                        key.bus_label(),
                        key.value_label(),
                        balance.to_u64()
                    ),
                );
            }
        }
        Ok(())
    }

    pub(crate) fn validate_all_units(self) -> Result<(), SourceLookupHintError> {
        for (key, balance) in self.entries {
            if balance != Felt::ZERO {
                return Err(SourceLookupHintError::Set {
                    message: format!(
                        "unbalanced lookup bus {} tuple {} has net weight {}",
                        key.bus_label(),
                        key.value_label(),
                        balance.to_u64()
                    ),
                });
            }
        }
        Ok(())
    }
}

impl fmt::Display for SourceLookupHintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit { message, .. } | Self::Set { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for SourceLookupHintError {}

impl SourceLookupKey {
    fn bus_label(&self) -> String {
        self.qualifiers
            .iter()
            .find(|(name, _)| name == "bus_id")
            .map(|(_, value)| value.label())
            .unwrap_or_else(|| "unknown".to_owned())
    }

    fn value_label(&self) -> String {
        self.values
            .iter()
            .map(SourceLookupPayload::label)
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl SourceLookupPayload {
    fn label(&self) -> String {
        match self {
            Self::Scalar(value) => value.to_string(),
            Self::Extension(values) => format!("{},{},{}", values[0], values[1], values[2]),
            Self::Text(value) => value.clone(),
        }
    }
}

fn source_lookup_key(
    unit_index: usize,
    row: usize,
    hint: &ResolvedHint,
) -> Result<SourceLookupKey, SourceLookupHintError> {
    let mut qualifiers = Vec::new();
    qualifiers.push((
        "bus_id".to_owned(),
        source_lookup_single_payload(unit_index, row, hint, "bus_id")?,
    ));
    for field in &hint.fields {
        if matches!(
            field.name.as_str(),
            "bus_id" | "values" | "multiplicity" | "selector" | "line"
        ) {
            continue;
        }
        qualifiers.push((
            field.name.clone(),
            source_lookup_single_payload(unit_index, row, hint, &field.name)?,
        ));
    }
    qualifiers.sort_by(|left, right| left.0.cmp(&right.0));

    let values = source_lookup_field(hint, "values")
        .ok_or_else(|| source_lookup_message(unit_index, "missing values field", row))?
        .values
        .iter()
        .map(|value| source_lookup_payload(&value.payload))
        .collect::<Vec<_>>();
    if values.is_empty() {
        return source_lookup_error(unit_index, format!("empty values field at row {row}"));
    }

    Ok(SourceLookupKey { qualifiers, values })
}

fn source_lookup_weight(
    unit_index: usize,
    row: usize,
    hint: &ResolvedHint,
) -> Result<Felt, SourceLookupHintError> {
    let multiplicity = source_lookup_field(hint, "multiplicity");
    let selector = source_lookup_field(hint, "selector");
    match (multiplicity, selector) {
        (None, None) => Ok(Felt::ONE),
        (Some(_), Some(_)) => source_lookup_error(
            unit_index,
            format!("lookup hint has both multiplicity and selector at row {row}"),
        ),
        (Some(field), None) | (None, Some(field)) => {
            source_lookup_weight_field(unit_index, row, field)
        }
    }
}

fn source_lookup_weight_field(
    unit_index: usize,
    row: usize,
    field: &ResolvedHintField,
) -> Result<Felt, SourceLookupHintError> {
    if field.values.len() == 1 {
        return match field.values[0].payload {
            ResolvedHintPayload::Scalar(value) => Ok(value),
            _ => source_lookup_error(
                unit_index,
                format!(
                    "lookup weight field {} is not scalar at row {row}",
                    field.name
                ),
            ),
        };
    }
    source_lookup_expression_field(unit_index, row, field)
}

fn source_lookup_expression_field(
    unit_index: usize,
    row: usize,
    field: &ResolvedHintField,
) -> Result<Felt, SourceLookupHintError> {
    let mut stack = Vec::new();
    for value in &field.values {
        match &value.payload {
            ResolvedHintPayload::Scalar(value) => stack.push(*value),
            ResolvedHintPayload::Text(op) => {
                let right = source_lookup_expression_pop(unit_index, row, op, &mut stack)?;
                let left = source_lookup_expression_pop(unit_index, row, op, &mut stack)?;
                let result = match op.as_str() {
                    "add" => left + right,
                    "sub" => left - right,
                    "mul" => left * right,
                    "pow" => left.pow(right.to_u64()),
                    "div" => {
                        let inverse =
                            right.inverse().ok_or_else(|| SourceLookupHintError::Unit {
                                unit_index,
                                message: format!("operator div has zero divisor at row {row}"),
                            })?;
                        left * inverse
                    }
                    "mod" => {
                        let divisor = right.to_u64();
                        if divisor == 0 {
                            return Err(SourceLookupHintError::Unit {
                                unit_index,
                                message: format!("operator mod has zero divisor at row {row}"),
                            });
                        }
                        Felt::from_u64(left.to_u64() % divisor)
                    }
                    "shl" => {
                        let shift = u32::try_from(right.to_u64()).map_err(|_| {
                            SourceLookupHintError::Unit {
                                unit_index,
                                message: format!("operator shl has invalid shift at row {row}"),
                            }
                        })?;
                        let shifted = left.to_u64().checked_shl(shift).ok_or_else(|| {
                            SourceLookupHintError::Unit {
                                unit_index,
                                message: format!("operator shl has invalid shift at row {row}"),
                            }
                        })?;
                        Felt::from_u64(shifted)
                    }
                    "shr" => {
                        let shift = u32::try_from(right.to_u64()).map_err(|_| {
                            SourceLookupHintError::Unit {
                                unit_index,
                                message: format!("operator shr has invalid shift at row {row}"),
                            }
                        })?;
                        let shifted = left.to_u64().checked_shr(shift).ok_or_else(|| {
                            SourceLookupHintError::Unit {
                                unit_index,
                                message: format!("operator shr has invalid shift at row {row}"),
                            }
                        })?;
                        Felt::from_u64(shifted)
                    }
                    "lt" => Felt::from_u64(u64::from(left.to_u64() < right.to_u64())),
                    "le" => Felt::from_u64(u64::from(left.to_u64() <= right.to_u64())),
                    "gt" => Felt::from_u64(u64::from(left.to_u64() > right.to_u64())),
                    "ge" => Felt::from_u64(u64::from(left.to_u64() >= right.to_u64())),
                    "eq" => Felt::from_u64(u64::from(left == right)),
                    "ne" => Felt::from_u64(u64::from(left != right)),
                    "bitand" => Felt::from_u64(left.to_u64() & right.to_u64()),
                    "bitxor" => Felt::from_u64(left.to_u64() ^ right.to_u64()),
                    "bitor" => Felt::from_u64(left.to_u64() | right.to_u64()),
                    _ => {
                        return source_lookup_error(
                            unit_index,
                            format!("unsupported expression operator {op} at row {row}"),
                        )
                    }
                };
                stack.push(result);
            }
            ResolvedHintPayload::Extension(_) => {
                return source_lookup_error(
                    unit_index,
                    format!(
                        "lookup weight field {} contains an extension value at row {row}",
                        field.name
                    ),
                )
            }
        }
    }

    if stack.len() != 1 {
        return source_lookup_error(
            unit_index,
            format!(
                "lookup weight field {} leaves {} values on the stack at row {row}",
                field.name,
                stack.len()
            ),
        );
    }
    Ok(stack[0])
}

fn source_lookup_expression_pop(
    unit_index: usize,
    row: usize,
    op: &str,
    stack: &mut Vec<Felt>,
) -> Result<Felt, SourceLookupHintError> {
    stack.pop().ok_or_else(|| SourceLookupHintError::Unit {
        unit_index,
        message: format!("operator {op} has too few operands at row {row}"),
    })
}

fn source_lookup_single_payload(
    unit_index: usize,
    row: usize,
    hint: &ResolvedHint,
    field_name: &str,
) -> Result<SourceLookupPayload, SourceLookupHintError> {
    let field = source_lookup_field(hint, field_name).ok_or_else(|| {
        source_lookup_message(unit_index, format!("missing {field_name} field"), row)
    })?;
    if field.values.len() != 1 {
        return source_lookup_error(
            unit_index,
            format!(
                "lookup field {field_name} has {} values at row {row}",
                field.values.len()
            ),
        );
    }
    Ok(source_lookup_payload(&field.values[0].payload))
}

fn source_lookup_payload(payload: &ResolvedHintPayload) -> SourceLookupPayload {
    match payload {
        ResolvedHintPayload::Scalar(value) => SourceLookupPayload::Scalar(value.to_u64()),
        ResolvedHintPayload::Extension(value) => SourceLookupPayload::Extension(value.to_u64s()),
        ResolvedHintPayload::Text(value) => SourceLookupPayload::Text(value.clone()),
    }
}

fn source_lookup_field<'a>(
    hint: &'a ResolvedHint,
    field_name: &str,
) -> Option<&'a ResolvedHintField> {
    hint.fields.iter().find(|field| field.name == field_name)
}

fn source_lookup_message(
    unit_index: usize,
    message: impl Into<String>,
    row: usize,
) -> SourceLookupHintError {
    SourceLookupHintError::Unit {
        unit_index,
        message: format!("{} at row {row}", message.into()),
    }
}

fn source_lookup_error<T>(
    unit_index: usize,
    message: impl Into<String>,
) -> Result<T, SourceLookupHintError> {
    Err(SourceLookupHintError::Unit {
        unit_index,
        message: message.into(),
    })
}

use std::collections::BTreeMap;
use std::fmt;

use lzvm_artifacts::hint_program::{
    source_lookup_hint_name, SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT,
};
use lzvm_field::Felt;

use crate::hint_eval::{ResolvedHint, ResolvedHintField, ResolvedHintPayload};

const PIOP_SURNAME_DYNAMIC: u64 = 2;

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
            if source_lookup_line_only_hint(hint) {
                continue;
            }
            let key = source_lookup_key(unit_index, row, hint)?;
            let weight = source_lookup_weight(unit_index, row, hint)?;
            let supported = matches!(
                hint.name.as_str(),
                SOURCE_LOOKUP_PROVES_HINT | SOURCE_LOOKUP_ASSUMES_HINT
            );
            if !supported {
                return source_lookup_error(
                    unit_index,
                    format!("unsupported lookup hint {} at row {row}", hint.name),
                );
            }
            if source_lookup_dynamic_surname(hint) {
                continue;
            }
            let entry = self.entries.entry(key).or_insert(Felt::ZERO);
            match hint.name.as_str() {
                SOURCE_LOOKUP_PROVES_HINT => *entry = *entry + weight,
                SOURCE_LOOKUP_ASSUMES_HINT => *entry = *entry - weight,
                _ => unreachable!(),
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

fn source_lookup_line_only_hint(hint: &ResolvedHint) -> bool {
    hint.fields.len() == 1 && hint.fields[0].name == "line"
}

fn source_lookup_dynamic_surname(hint: &ResolvedHint) -> bool {
    let Some(field) = source_lookup_field(hint, "surname") else {
        return false;
    };
    if field.values.len() != 1 {
        return false;
    }
    matches!(
        field.values[0].payload,
        ResolvedHintPayload::Scalar(value) if value.to_u64() == PIOP_SURNAME_DYNAMIC
    )
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
            "bus_id" | "values" | "value_lengths" | "multiplicity" | "selector" | "line"
        ) {
            continue;
        }
        qualifiers.push((
            field.name.clone(),
            source_lookup_single_payload(unit_index, row, hint, &field.name)?,
        ));
    }
    qualifiers.sort_by(|left, right| left.0.cmp(&right.0));

    let values = source_lookup_key_values(unit_index, row, hint)?;
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
                if op == "not" {
                    let value = source_lookup_expression_pop(unit_index, row, op, &mut stack)?;
                    stack.push(Felt::from_u64(u64::from(value == Felt::ZERO)));
                    continue;
                }
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

fn source_lookup_key_values(
    unit_index: usize,
    row: usize,
    hint: &ResolvedHint,
) -> Result<Vec<SourceLookupPayload>, SourceLookupHintError> {
    let field = source_lookup_field(hint, "values")
        .ok_or_else(|| source_lookup_message(unit_index, "missing values field", row))?;
    let Some(lengths) = source_lookup_value_lengths(unit_index, row, hint)? else {
        return Ok(field
            .values
            .iter()
            .map(|value| source_lookup_payload(&value.payload))
            .collect());
    };

    let mut values = Vec::with_capacity(lengths.len());
    let mut offset = 0_usize;
    for length in lengths {
        if length == 0 {
            return source_lookup_error(
                unit_index,
                format!("lookup value expression has zero length at row {row}"),
            );
        }
        let end = offset
            .checked_add(length)
            .ok_or_else(|| SourceLookupHintError::Unit {
                unit_index,
                message: format!("lookup value expression length overflow at row {row}"),
            })?;
        if end > field.values.len() {
            return source_lookup_error(
                unit_index,
                format!("lookup value expression length exceeds values at row {row}"),
            );
        }
        if length == 1 {
            values.push(source_lookup_payload(&field.values[offset].payload));
        } else {
            let expression = ResolvedHintField {
                name: "values".to_owned(),
                values: field.values[offset..end].to_vec(),
            };
            values.push(SourceLookupPayload::Scalar(
                source_lookup_expression_field(unit_index, row, &expression)?.to_u64(),
            ));
        }
        offset = end;
    }
    if offset != field.values.len() {
        return source_lookup_error(
            unit_index,
            format!("lookup value expression lengths leave unused values at row {row}"),
        );
    }
    Ok(values)
}

fn source_lookup_value_lengths(
    unit_index: usize,
    row: usize,
    hint: &ResolvedHint,
) -> Result<Option<Vec<usize>>, SourceLookupHintError> {
    let Some(field) = source_lookup_field(hint, "value_lengths") else {
        return Ok(None);
    };
    field
        .values
        .iter()
        .map(|value| match value.payload {
            ResolvedHintPayload::Scalar(value) => {
                usize::try_from(value.to_u64()).map_err(|_| SourceLookupHintError::Unit {
                    unit_index,
                    message: format!("lookup value expression length overflow at row {row}"),
                })
            }
            _ => Err(SourceLookupHintError::Unit {
                unit_index,
                message: format!("lookup value expression length is not scalar at row {row}"),
            }),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_eval::ResolvedHintValue;

    #[test]
    fn dynamic_surname_hints_do_not_require_exact_tuple_balance() {
        let mut balance = SourceLookupBalance::default();
        let hint = lookup_hint(
            SOURCE_LOOKUP_PROVES_HINT,
            &[48, 0, 0, 11, 13, 11, 13],
            "multiplicity",
            1,
            Some(2),
        );

        balance.absorb(0, 0, &[hint]).expect("hint should absorb");
        balance
            .validate_all_units()
            .expect("dynamic hints are checked by sum-bus constraints");
    }

    #[test]
    fn non_dynamic_unbalanced_hints_still_reject() {
        let mut balance = SourceLookupBalance::default();
        let hint = lookup_hint(
            SOURCE_LOOKUP_PROVES_HINT,
            &[48, 0, 0],
            "multiplicity",
            1,
            None,
        );

        balance.absorb(0, 0, &[hint]).expect("hint should absorb");
        let error = balance
            .validate_all_units()
            .expect_err("non-dynamic hints should require exact balance");

        assert!(error.to_string().contains("unbalanced lookup bus 5000"));
    }

    #[test]
    fn dynamic_surname_hints_still_require_values() {
        let mut balance = SourceLookupBalance::default();
        let hint = ResolvedHint {
            name: SOURCE_LOOKUP_PROVES_HINT.to_owned(),
            fields: vec![
                field("bus_id", &[5000]),
                field("multiplicity", &[1]),
                field("surname", &[PIOP_SURNAME_DYNAMIC]),
            ],
        };

        let error = balance
            .absorb(0, 0, &[hint])
            .expect_err("dynamic hints should still require values");

        assert!(error.to_string().contains("missing values field"));
    }

    #[test]
    fn dynamic_surname_hints_still_reject_unsupported_lookup_names() {
        let mut balance = SourceLookupBalance::default();
        let hint = lookup_hint(
            "source.lookup.unknown",
            &[48, 0, 0, 11, 13, 11, 13],
            "multiplicity",
            1,
            Some(PIOP_SURNAME_DYNAMIC),
        );

        let error = balance
            .absorb(0, 0, &[hint])
            .expect_err("unsupported dynamic hints should reject");

        assert!(error
            .to_string()
            .contains("unsupported lookup hint source.lookup.unknown"));
    }

    fn lookup_hint(
        name: &str,
        values: &[u64],
        weight_name: &str,
        weight: u64,
        surname: Option<u64>,
    ) -> ResolvedHint {
        let mut fields = vec![
            field("bus_id", &[5000]),
            ResolvedHintField {
                name: "values".to_owned(),
                values: values
                    .iter()
                    .map(|value| scalar_value(*value))
                    .collect::<Vec<_>>(),
            },
            field(weight_name, &[weight]),
        ];
        if let Some(surname) = surname {
            fields.push(field("surname", &[surname]));
        }
        ResolvedHint {
            name: name.to_owned(),
            fields,
        }
    }

    fn field(name: &str, values: &[u64]) -> ResolvedHintField {
        ResolvedHintField {
            name: name.to_owned(),
            values: values
                .iter()
                .map(|value| scalar_value(*value))
                .collect::<Vec<_>>(),
        }
    }

    fn scalar_value(value: u64) -> ResolvedHintValue {
        ResolvedHintValue {
            payload: ResolvedHintPayload::Scalar(Felt::from_u64(value)),
            positions: Vec::new(),
        }
    }
}

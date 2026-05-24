use std::collections::BTreeMap;

use lzvm_field::{Felt, MODULUS};
use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, BinaryOperator, ColumnInitializer,
    Expression, ExpressionKind, FixedFileTemplateValue, SourceProgram, SourceSpan, UnaryOperator,
};

use crate::source_fixed_columns::SourceFixedColumnsWriteError;
use crate::source_static_values::{evaluate_source_static_expression, static_value_integer};

#[derive(Debug, Clone, Default)]
pub(crate) struct SourceFixedConstantValues {
    pub(crate) scalars: BTreeMap<String, FixedFileTemplateValue>,
    pub(crate) arrays: BTreeMap<String, Vec<u64>>,
}

pub(crate) struct SourceFixedExpressionValuesRequest<'a> {
    pub(crate) program: &'a SourceProgram,
    pub(crate) source_name: &'a str,
    pub(crate) source: &'a str,
    pub(crate) column_name: &'a str,
    pub(crate) initializer: &'a ColumnInitializer,
    pub(crate) row_count: usize,
    pub(crate) constant_values: &'a SourceFixedConstantValues,
    pub(crate) column_values: &'a BTreeMap<String, Vec<u64>>,
}

pub(crate) struct SourceFixedExpressionValueAtRowRequest<'a> {
    pub(crate) program: &'a SourceProgram,
    pub(crate) source_name: &'a str,
    pub(crate) source: &'a str,
    pub(crate) column_name: &'a str,
    pub(crate) expression: &'a Expression,
    pub(crate) row_count: usize,
    pub(crate) constant_values: &'a SourceFixedConstantValues,
    pub(crate) column_values: &'a BTreeMap<String, Vec<u64>>,
}

pub(crate) fn source_fixed_column_expression_values(
    request: &SourceFixedExpressionValuesRequest<'_>,
) -> Result<Option<Vec<u64>>, SourceFixedColumnsWriteError> {
    let Some(expression) = request.initializer.expression.as_ref() else {
        return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
            source_name: request.source_name.to_owned(),
            column: request.column_name.to_owned(),
        });
    };

    let context = SourceFixedExpressionContext {
        program: request.program,
        source_name: request.source_name,
        source: request.source,
        column_name: request.column_name,
        row_count: request.row_count,
        constant_values: request.constant_values,
        column_values: request.column_values,
    };
    let mut values = Vec::with_capacity(request.row_count);
    for row in 0..request.row_count {
        let Some(value) = evaluate_source_fixed_expression_inner(&context, expression, row)? else {
            return Ok(None);
        };
        values.push(value);
    }

    Ok(Some(values))
}

pub(crate) fn source_fixed_expression_value_at_row(
    request: &SourceFixedExpressionValueAtRowRequest<'_>,
    row: usize,
) -> Result<Option<u64>, SourceFixedColumnsWriteError> {
    let context = SourceFixedExpressionContext {
        program: request.program,
        source_name: request.source_name,
        source: request.source,
        column_name: request.column_name,
        row_count: request.row_count,
        constant_values: request.constant_values,
        column_values: request.column_values,
    };
    evaluate_source_fixed_expression_inner(&context, request.expression, row)
}

struct SourceFixedExpressionContext<'a> {
    program: &'a SourceProgram,
    source_name: &'a str,
    source: &'a str,
    column_name: &'a str,
    row_count: usize,
    constant_values: &'a SourceFixedConstantValues,
    column_values: &'a BTreeMap<String, Vec<u64>>,
}

fn evaluate_source_fixed_expression_inner(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    row: usize,
) -> Result<Option<u64>, SourceFixedColumnsWriteError> {
    if let Some(value) = evaluate_source_fixed_static_value_expression(context, expression)
        .as_ref()
        .and_then(static_value_integer)
    {
        return canonical_source_fixed_expression_value(context, expression, value).map(Some);
    }

    match &expression.kind {
        ExpressionKind::Integer(value) => {
            let value = parse_expression_integer(value, context, expression)?;
            canonical_source_fixed_expression_value(context, expression, value).map(Some)
        }
        ExpressionKind::HexInteger(value) => {
            let value = parse_expression_hex_integer(value, context, expression)?;
            canonical_source_fixed_expression_value(context, expression, value).map(Some)
        }
        ExpressionKind::Name(reference) => {
            for candidate in fixed_column_reference_candidates(context.column_name, reference) {
                if let Some(values) = context.column_values.get(&candidate) {
                    return Ok(values.get(row).copied());
                }
            }
            Ok(None)
        }
        ExpressionKind::Group(inner) => evaluate_source_fixed_expression_inner(context, inner, row),
        ExpressionKind::Unary { op, expr } => {
            let Some(value) = evaluate_source_fixed_expression_inner(context, expr, row)? else {
                return Ok(None);
            };
            match op {
                UnaryOperator::Plus => Ok(Some(value)),
                UnaryOperator::Minus => Ok(Some(field_sub(0, value))),
                UnaryOperator::Not => Ok(Some(source_fixed_bool(!source_fixed_truthy(value)))),
                UnaryOperator::Increment | UnaryOperator::Decrement => {
                    Err(source_fixed_expression_unsupported(context, expression))
                }
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let Some(lhs) = evaluate_source_fixed_expression_inner(context, left, row)? else {
                return Ok(None);
            };
            if *op == BinaryOperator::LogicalAnd {
                if !source_fixed_truthy(lhs) {
                    return Ok(Some(lhs));
                }
                return evaluate_source_fixed_expression_inner(context, right, row);
            }
            if *op == BinaryOperator::LogicalOr {
                if source_fixed_truthy(lhs) {
                    return Ok(Some(lhs));
                }
                return evaluate_source_fixed_expression_inner(context, right, row);
            }
            if *op == BinaryOperator::Power {
                let exponent = source_fixed_expression_static_integer(context, right)?;
                let exponent = u64::try_from(exponent)
                    .map_err(|_| source_fixed_expression_integer_out_of_range(context, right))?;
                return Ok(Some(field_pow(lhs, exponent)));
            }
            if matches!(op, BinaryOperator::Divide | BinaryOperator::Backslash) {
                let divisor = source_fixed_expression_static_integer(context, right)?;
                return Ok(Some(field_div_by_static(
                    context, expression, lhs, divisor,
                )?));
            }
            if *op == BinaryOperator::Modulo {
                let divisor = source_fixed_expression_static_integer(context, right)?;
                return field_mod_by_static(context, expression, lhs, divisor).map(Some);
            }
            if matches!(op, BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight) {
                let shift = source_fixed_expression_static_integer(context, right)?;
                return integer_shift_by_static(context, expression, lhs, shift, *op).map(Some);
            }
            let Some(rhs) = evaluate_source_fixed_expression_inner(context, right, row)? else {
                return Ok(None);
            };
            match op {
                BinaryOperator::Add => Ok(Some(field_add(lhs, rhs))),
                BinaryOperator::Subtract => Ok(Some(field_sub(lhs, rhs))),
                BinaryOperator::Multiply => Ok(Some(field_mul(lhs, rhs))),
                BinaryOperator::BitAnd => {
                    integer_bitwise(context, expression, lhs, rhs, |a, b| a & b).map(Some)
                }
                BinaryOperator::BitXor => {
                    integer_bitwise(context, expression, lhs, rhs, |a, b| a ^ b).map(Some)
                }
                BinaryOperator::BitOr => {
                    integer_bitwise(context, expression, lhs, rhs, |a, b| a | b).map(Some)
                }
                BinaryOperator::Less => Ok(Some(integer_cmp(lhs, rhs, |a, b| a < b))),
                BinaryOperator::LessEqual => Ok(Some(integer_cmp(lhs, rhs, |a, b| a <= b))),
                BinaryOperator::Greater => Ok(Some(integer_cmp(lhs, rhs, |a, b| a > b))),
                BinaryOperator::GreaterEqual => Ok(Some(integer_cmp(lhs, rhs, |a, b| a >= b))),
                BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => {
                    Ok(Some(source_fixed_bool(lhs == rhs)))
                }
                BinaryOperator::NotEqual => Ok(Some(source_fixed_bool(lhs != rhs))),
                _ => Err(source_fixed_expression_unsupported(context, expression)),
            }
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            let Some(condition) = evaluate_source_fixed_expression_inner(context, condition, row)?
            else {
                return Ok(None);
            };
            if source_fixed_truthy(condition) {
                evaluate_source_fixed_expression_inner(context, then_expr, row)
            } else {
                evaluate_source_fixed_expression_inner(context, else_expr, row)
            }
        }
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => {
            let offset = source_fixed_expression_static_integer(context, offset)?;
            let signed_offset = if *prior {
                offset.checked_neg()
            } else {
                Some(offset)
            }
            .ok_or_else(|| source_fixed_expression_integer_out_of_range(context, expression))?;
            let row_count = i128::try_from(context.row_count)
                .map_err(|_| source_fixed_expression_integer_out_of_range(context, expression))?;
            let shifted = i128::try_from(row)
                .ok()
                .and_then(|row| row.checked_add(signed_offset))
                .ok_or_else(|| source_fixed_expression_integer_out_of_range(context, expression))?;
            let source_row = shifted.rem_euclid(row_count) as usize;
            evaluate_source_fixed_expression_inner(context, target, source_row)
        }
        ExpressionKind::Index { target, index } => {
            if let Some(value) = evaluate_source_fixed_static_value_expression(context, expression)
                .as_ref()
                .and_then(static_value_integer)
            {
                return canonical_source_fixed_expression_value(context, expression, value)
                    .map(Some);
            }
            if let Some(value) = evaluate_source_fixed_array_index(context, expression, row)? {
                return Ok(Some(value));
            }
            if source_fixed_expression_column_reference(context, target, row)?.is_some() {
                return evaluate_source_fixed_column_index(context, target, index, row);
            }
            Err(source_fixed_expression_unsupported(context, expression))
        }
        ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_)
        | ExpressionKind::Array(_)
        | ExpressionKind::Call { .. } => {
            Err(source_fixed_expression_unsupported(context, expression))
        }
    }
}

pub(crate) fn evaluate_source_fixed_template_value_expression(
    expression: &Expression,
    constant_values: &SourceFixedConstantValues,
) -> Option<FixedFileTemplateValue> {
    evaluate_source_fixed_template_value_expression_with_parts(
        expression,
        &constant_values.scalars,
        &constant_values.arrays,
    )
}

pub(crate) fn evaluate_source_fixed_template_value_expression_with_parts(
    expression: &Expression,
    scalars: &BTreeMap<String, FixedFileTemplateValue>,
    arrays: &BTreeMap<String, Vec<u64>>,
) -> Option<FixedFileTemplateValue> {
    if let Some(value) =
        evaluate_fixed_file_template_value_expression_with_values(expression, scalars)
    {
        return Some(value);
    }

    if let ExpressionKind::Group(inner) = &expression.kind {
        return evaluate_source_fixed_template_value_expression_with_parts(inner, scalars, arrays);
    }

    if let ExpressionKind::Unary { op, expr } = &expression.kind {
        let value =
            evaluate_source_fixed_template_value_expression_with_parts(expr, scalars, arrays)?;
        return match op {
            UnaryOperator::Plus => {
                source_fixed_template_integer(&value).map(FixedFileTemplateValue::Integer)
            }
            UnaryOperator::Minus => source_fixed_template_integer(&value)
                .and_then(i128::checked_neg)
                .map(FixedFileTemplateValue::Integer),
            UnaryOperator::Not => Some(FixedFileTemplateValue::Boolean(
                !source_fixed_template_truthy(&value),
            )),
            UnaryOperator::Increment | UnaryOperator::Decrement => None,
        };
    }

    if let ExpressionKind::Binary { op, left, right } = &expression.kind {
        let left =
            evaluate_source_fixed_template_value_expression_with_parts(left, scalars, arrays)?;
        if *op == BinaryOperator::LogicalAnd {
            if source_fixed_template_truthy(&left) {
                return evaluate_source_fixed_template_value_expression_with_parts(
                    right, scalars, arrays,
                );
            }
            return Some(left);
        }
        if *op == BinaryOperator::LogicalOr {
            if source_fixed_template_truthy(&left) {
                return Some(left);
            }
            return evaluate_source_fixed_template_value_expression_with_parts(
                right, scalars, arrays,
            );
        }

        let right =
            evaluate_source_fixed_template_value_expression_with_parts(right, scalars, arrays)?;
        let left_integer = source_fixed_template_integer(&left);
        let right_integer = source_fixed_template_integer(&right);
        let value = match op {
            BinaryOperator::Add => match (left_integer, right_integer) {
                (Some(left), Some(right)) => {
                    FixedFileTemplateValue::Integer(left.checked_add(right)?)
                }
                _ => FixedFileTemplateValue::String(format!(
                    "{}{}",
                    source_fixed_template_string(left),
                    source_fixed_template_string(right)
                )),
            },
            BinaryOperator::Subtract => {
                FixedFileTemplateValue::Integer(left_integer?.checked_sub(right_integer?)?)
            }
            BinaryOperator::Multiply => {
                FixedFileTemplateValue::Integer(left_integer?.checked_mul(right_integer?)?)
            }
            BinaryOperator::Divide | BinaryOperator::Backslash if right_integer? != 0 => {
                FixedFileTemplateValue::Integer(left_integer?.checked_div(right_integer?)?)
            }
            BinaryOperator::Modulo if right_integer? != 0 => {
                FixedFileTemplateValue::Integer(left_integer?.checked_rem(right_integer?)?)
            }
            BinaryOperator::Power => {
                let base = u64::try_from(left_integer?).ok()?;
                let exponent = u64::try_from(right_integer?).ok()?;
                FixedFileTemplateValue::Integer(i128::from(
                    Felt::from_u64(base).pow(exponent).to_u64(),
                ))
            }
            BinaryOperator::ShiftLeft => {
                let right = u32::try_from(right_integer?).ok()?;
                FixedFileTemplateValue::Integer(left_integer?.checked_shl(right)?)
            }
            BinaryOperator::ShiftRight => {
                let right = u32::try_from(right_integer?).ok()?;
                FixedFileTemplateValue::Integer(left_integer?.checked_shr(right)?)
            }
            BinaryOperator::BitAnd => {
                FixedFileTemplateValue::Integer(left_integer? & right_integer?)
            }
            BinaryOperator::BitXor => {
                FixedFileTemplateValue::Integer(left_integer? ^ right_integer?)
            }
            BinaryOperator::BitOr => {
                FixedFileTemplateValue::Integer(left_integer? | right_integer?)
            }
            BinaryOperator::Less => FixedFileTemplateValue::Boolean(left_integer? < right_integer?),
            BinaryOperator::LessEqual => {
                FixedFileTemplateValue::Boolean(left_integer? <= right_integer?)
            }
            BinaryOperator::Greater => {
                FixedFileTemplateValue::Boolean(left_integer? > right_integer?)
            }
            BinaryOperator::GreaterEqual => {
                FixedFileTemplateValue::Boolean(left_integer? >= right_integer?)
            }
            BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => {
                FixedFileTemplateValue::Boolean(left == right)
            }
            BinaryOperator::NotEqual => FixedFileTemplateValue::Boolean(left != right),
            _ => return None,
        };
        return Some(value);
    }

    if let ExpressionKind::Ternary {
        condition,
        then_expr,
        else_expr,
    } = &expression.kind
    {
        let condition =
            evaluate_source_fixed_template_value_expression_with_parts(condition, scalars, arrays)?;
        if source_fixed_template_truthy(&condition) {
            return evaluate_source_fixed_template_value_expression_with_parts(
                then_expr, scalars, arrays,
            );
        }
        return evaluate_source_fixed_template_value_expression_with_parts(
            else_expr, scalars, arrays,
        );
    }

    let ExpressionKind::Index { target, index } = &expression.kind else {
        return None;
    };
    let ExpressionKind::Name(array_name) = &target.kind else {
        return None;
    };
    let values = arrays.get(array_name)?;
    let value = evaluate_source_fixed_template_value_expression_with_parts(index, scalars, arrays)?;
    let index = static_value_integer(&value)?;
    let index = usize::try_from(index).ok()?;
    values
        .get(index)
        .copied()
        .map(|value| FixedFileTemplateValue::Integer(i128::from(value)))
}

fn source_fixed_template_integer(value: &FixedFileTemplateValue) -> Option<i128> {
    match value {
        FixedFileTemplateValue::Integer(value) => Some(*value),
        FixedFileTemplateValue::Boolean(value) => Some(if *value { 1 } else { 0 }),
        FixedFileTemplateValue::String(_) => None,
    }
}

fn source_fixed_template_truthy(value: &FixedFileTemplateValue) -> bool {
    match value {
        FixedFileTemplateValue::Integer(value) => *value != 0,
        FixedFileTemplateValue::Boolean(value) => *value,
        FixedFileTemplateValue::String(value) => !value.is_empty(),
    }
}

fn source_fixed_template_string(value: FixedFileTemplateValue) -> String {
    match value {
        FixedFileTemplateValue::Integer(value) => value.to_string(),
        FixedFileTemplateValue::Boolean(value) => value.to_string(),
        FixedFileTemplateValue::String(value) => value,
    }
}

fn evaluate_source_fixed_template_value_expression_with_static_index(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> Option<FixedFileTemplateValue> {
    let ExpressionKind::Index { target, index } = &expression.kind else {
        return None;
    };
    let ExpressionKind::Name(array_name) = &target.kind else {
        return None;
    };
    let values = context.constant_values.arrays.get(array_name)?;
    let value = evaluate_source_fixed_static_value_expression(context, index)?;
    let index = static_value_integer(&value)?;
    let index = usize::try_from(index).ok()?;
    values
        .get(index)
        .copied()
        .map(|value| FixedFileTemplateValue::Integer(i128::from(value)))
}

fn evaluate_source_fixed_array_index(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    row: usize,
) -> Result<Option<u64>, SourceFixedColumnsWriteError> {
    let ExpressionKind::Index { target, index } = &expression.kind else {
        return Ok(None);
    };
    let Some(array_name) = source_fixed_expression_name(target) else {
        return Ok(None);
    };
    let Some(values) = context.constant_values.arrays.get(array_name) else {
        return Ok(None);
    };
    let Some(index) = evaluate_source_fixed_expression_inner(context, index, row)? else {
        return Ok(None);
    };
    let index = usize::try_from(index)
        .map_err(|_| source_fixed_expression_integer_out_of_range(context, expression))?;
    values
        .get(index)
        .copied()
        .map(Some)
        .ok_or_else(|| source_fixed_expression_unsupported(context, expression))
}

fn evaluate_source_fixed_column_index(
    context: &SourceFixedExpressionContext<'_>,
    target: &Expression,
    index: &Expression,
    row: usize,
) -> Result<Option<u64>, SourceFixedColumnsWriteError> {
    let Some(reference) = source_fixed_expression_column_reference(context, target, row)? else {
        return Ok(None);
    };
    let Some(source_row) = evaluate_source_fixed_expression_inner(context, index, row)? else {
        return Ok(None);
    };
    let source_row = usize::try_from(source_row)
        .map_err(|_| source_fixed_expression_integer_out_of_range(context, index))?;
    if source_row >= context.row_count {
        return Err(source_fixed_expression_integer_out_of_range(context, index));
    }
    for candidate in fixed_column_reference_candidates(context.column_name, &reference) {
        if let Some(values) = context.column_values.get(&candidate) {
            return Ok(values.get(source_row).copied());
        }
    }
    Ok(None)
}

fn source_fixed_expression_column_reference(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    row: usize,
) -> Result<Option<String>, SourceFixedColumnsWriteError> {
    match &strip_source_fixed_expression_group(expression).kind {
        ExpressionKind::Name(name) => Ok(Some(name.clone())),
        ExpressionKind::Index { target, index } => {
            let Some(mut target) = source_fixed_expression_column_reference(context, target, row)?
            else {
                return Ok(None);
            };
            let Some(index) = evaluate_source_fixed_expression_inner(context, index, row)? else {
                return Ok(None);
            };
            target.push('[');
            target.push_str(&index.to_string());
            target.push(']');
            Ok(Some(target))
        }
        _ => Ok(None),
    }
}

fn source_fixed_expression_name(expression: &Expression) -> Option<&str> {
    match &strip_source_fixed_expression_group(expression).kind {
        ExpressionKind::Name(name) => Some(name),
        _ => None,
    }
}

fn strip_source_fixed_expression_group(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_source_fixed_expression_group(inner),
        _ => expression,
    }
}

fn source_fixed_expression_static_integer(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> Result<i128, SourceFixedColumnsWriteError> {
    match evaluate_source_fixed_static_value_expression(context, expression)
        .as_ref()
        .and_then(static_value_integer)
    {
        Some(value) => Ok(value),
        _ => Err(source_fixed_expression_unsupported(context, expression)),
    }
}

fn evaluate_source_fixed_static_value_expression(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> Option<FixedFileTemplateValue> {
    evaluate_source_fixed_template_value_expression(expression, context.constant_values)
        .or_else(|| {
            evaluate_source_fixed_template_value_expression_with_static_index(context, expression)
        })
        .or_else(|| {
            evaluate_source_static_expression(
                context.program,
                expression,
                &context.constant_values.scalars,
            )
        })
}

fn canonical_source_fixed_expression_value(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    value: i128,
) -> Result<u64, SourceFixedColumnsWriteError> {
    let modulus = i128::from(MODULUS);
    let canonical = value.rem_euclid(modulus);
    u64::try_from(canonical)
        .map_err(|_| source_fixed_expression_integer_out_of_range(context, expression))
}

fn parse_expression_integer(
    value: &str,
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> Result<i128, SourceFixedColumnsWriteError> {
    value
        .parse::<i128>()
        .map_err(|_| source_fixed_expression_invalid_literal(context, expression, value))
}

fn parse_expression_hex_integer(
    value: &str,
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> Result<i128, SourceFixedColumnsWriteError> {
    i128::from_str_radix(
        value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
            .unwrap_or(value),
        16,
    )
    .map_err(|_| source_fixed_expression_invalid_literal(context, expression, value))
}

fn fixed_column_reference_candidates(column_name: &str, reference: &str) -> Vec<String> {
    if reference.contains('.') {
        return vec![reference.to_owned()];
    }

    let mut candidates = Vec::new();
    if let Some((scope, _)) = column_name.rsplit_once('.') {
        candidates.push(format!("{scope}.{reference}"));
    }
    candidates.push(reference.to_owned());
    candidates
}

fn source_fixed_expression_invalid_literal(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    literal: &str,
) -> SourceFixedColumnsWriteError {
    SourceFixedColumnsWriteError::InvalidLiteral {
        source_name: context.source_name.to_owned(),
        source_span: SourceSpan {
            start: expression.start,
            end: expression.end,
        },
        literal: literal.to_owned(),
    }
}

fn source_fixed_expression_integer_out_of_range(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> SourceFixedColumnsWriteError {
    SourceFixedColumnsWriteError::IntegerOutOfRange {
        source_name: context.source_name.to_owned(),
        source_span: SourceSpan {
            start: expression.start,
            end: expression.end,
        },
        expression: source_fixed_expression_text(context.source, expression),
    }
}

fn source_fixed_expression_unsupported(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> SourceFixedColumnsWriteError {
    SourceFixedColumnsWriteError::UnsupportedExpression {
        source_name: context.source_name.to_owned(),
        source_span: SourceSpan {
            start: expression.start,
            end: expression.end,
        },
        expression: source_fixed_expression_text(context.source, expression),
    }
}

fn source_fixed_expression_text(source: &str, expression: &Expression) -> String {
    source
        .get(expression.start..expression.end)
        .unwrap_or_default()
        .to_owned()
}

fn field_add(lhs: u64, rhs: u64) -> u64 {
    let modulus = u128::from(MODULUS);
    ((u128::from(lhs) + u128::from(rhs)) % modulus) as u64
}

fn field_sub(lhs: u64, rhs: u64) -> u64 {
    let modulus = u128::from(MODULUS);
    ((u128::from(lhs) + modulus - u128::from(rhs)) % modulus) as u64
}

fn field_mul(lhs: u64, rhs: u64) -> u64 {
    let modulus = u128::from(MODULUS);
    ((u128::from(lhs) * u128::from(rhs)) % modulus) as u64
}

fn field_div_by_static(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    lhs: u64,
    divisor: i128,
) -> Result<u64, SourceFixedColumnsWriteError> {
    let divisor = canonical_source_fixed_expression_value(context, expression, divisor)?;
    let divisor = Felt::from_u64(divisor);
    let inverse = divisor
        .inverse()
        .ok_or_else(|| source_fixed_expression_unsupported(context, expression))?;
    Ok(field_mul(lhs, inverse.to_u64()))
}

fn field_mod_by_static(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    lhs: u64,
    divisor: i128,
) -> Result<u64, SourceFixedColumnsWriteError> {
    if divisor == 0 {
        return Err(source_fixed_expression_unsupported(context, expression));
    }
    let lhs = i128::from(lhs);
    let value = lhs
        .checked_rem(divisor)
        .ok_or_else(|| source_fixed_expression_integer_out_of_range(context, expression))?;
    canonical_source_fixed_expression_value(context, expression, value)
}

fn integer_shift_by_static(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    lhs: u64,
    shift: i128,
    op: BinaryOperator,
) -> Result<u64, SourceFixedColumnsWriteError> {
    let lhs = i128::from(lhs);
    let shift = u32::try_from(shift)
        .map_err(|_| source_fixed_expression_integer_out_of_range(context, expression))?;
    let value = match op {
        BinaryOperator::ShiftLeft => lhs.checked_shl(shift),
        BinaryOperator::ShiftRight => lhs.checked_shr(shift),
        _ => None,
    }
    .ok_or_else(|| source_fixed_expression_integer_out_of_range(context, expression))?;
    canonical_source_fixed_expression_value(context, expression, value)
}

fn integer_bitwise(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
    lhs: u64,
    rhs: u64,
    op: impl FnOnce(i128, i128) -> i128,
) -> Result<u64, SourceFixedColumnsWriteError> {
    let value = op(i128::from(lhs), i128::from(rhs));
    canonical_source_fixed_expression_value(context, expression, value)
}

fn integer_cmp(lhs: u64, rhs: u64, op: impl FnOnce(i128, i128) -> bool) -> u64 {
    source_fixed_bool(op(i128::from(lhs), i128::from(rhs)))
}

fn source_fixed_truthy(value: u64) -> bool {
    value != 0
}

fn source_fixed_bool(value: bool) -> u64 {
    if value {
        1
    } else {
        0
    }
}

fn field_pow(base: u64, exponent: u64) -> u64 {
    Felt::from_u64(base).pow(exponent).to_u64()
}

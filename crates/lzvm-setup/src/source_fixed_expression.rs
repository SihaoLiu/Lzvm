use std::collections::BTreeMap;

use lzvm_field::{Felt, MODULUS};
use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, BinaryOperator, ColumnInitializer,
    Expression, ExpressionKind, FixedFileTemplateValue, SourceProgram, SourceSpan, UnaryOperator,
};

use crate::source_fixed_columns::SourceFixedColumnsWriteError;
use crate::source_static_values::evaluate_source_static_expression;

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
    if let Some(FixedFileTemplateValue::Integer(value)) =
        evaluate_source_fixed_static_value_expression(context, expression)
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
                UnaryOperator::Not | UnaryOperator::Increment | UnaryOperator::Decrement => {
                    Err(source_fixed_expression_unsupported(context, expression))
                }
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let Some(lhs) = evaluate_source_fixed_expression_inner(context, left, row)? else {
                return Ok(None);
            };
            let Some(rhs) = evaluate_source_fixed_expression_inner(context, right, row)? else {
                return Ok(None);
            };
            match op {
                BinaryOperator::Add => Ok(Some(field_add(lhs, rhs))),
                BinaryOperator::Subtract => Ok(Some(field_sub(lhs, rhs))),
                BinaryOperator::Multiply => Ok(Some(field_mul(lhs, rhs))),
                _ => Err(source_fixed_expression_unsupported(context, expression)),
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
        ExpressionKind::Index { .. } => {
            if let Some(FixedFileTemplateValue::Integer(value)) =
                evaluate_source_fixed_static_value_expression(context, expression)
            {
                return canonical_source_fixed_expression_value(context, expression, value)
                    .map(Some);
            }
            Err(source_fixed_expression_unsupported(context, expression))
        }
        ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_)
        | ExpressionKind::Call { .. } => {
            Err(source_fixed_expression_unsupported(context, expression))
        }
    }
}

pub(crate) fn evaluate_source_fixed_template_value_expression(
    expression: &Expression,
    constant_values: &SourceFixedConstantValues,
) -> Option<FixedFileTemplateValue> {
    if let Some(value) = evaluate_fixed_file_template_value_expression_with_values(
        expression,
        &constant_values.scalars,
    ) {
        return Some(value);
    }

    if let ExpressionKind::Binary { op, left, right } = &expression.kind {
        let left = evaluate_source_fixed_template_value_expression(left, constant_values)?;
        let right = evaluate_source_fixed_template_value_expression(right, constant_values)?;
        let (FixedFileTemplateValue::Integer(left), FixedFileTemplateValue::Integer(right)) =
            (left, right)
        else {
            return None;
        };
        let value = match op {
            BinaryOperator::Add => left.checked_add(right)?,
            BinaryOperator::Subtract => left.checked_sub(right)?,
            BinaryOperator::Multiply => left.checked_mul(right)?,
            BinaryOperator::Divide if right != 0 => left.checked_div(right)?,
            BinaryOperator::Modulo if right != 0 => left.checked_rem(right)?,
            BinaryOperator::Power => {
                let base = u64::try_from(left).ok()?;
                let exponent = u64::try_from(right).ok()?;
                i128::from(Felt::from_u64(base).pow(exponent).to_u64())
            }
            _ => return None,
        };
        return Some(FixedFileTemplateValue::Integer(value));
    }

    let ExpressionKind::Index { target, index } = &expression.kind else {
        return None;
    };
    let ExpressionKind::Name(array_name) = &target.kind else {
        return None;
    };
    let values = constant_values.arrays.get(array_name)?;
    let FixedFileTemplateValue::Integer(index) =
        evaluate_source_fixed_template_value_expression(index, constant_values)?
    else {
        return None;
    };
    let index = usize::try_from(index).ok()?;
    values
        .get(index)
        .copied()
        .map(|value| FixedFileTemplateValue::Integer(i128::from(value)))
}

fn source_fixed_expression_static_integer(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> Result<i128, SourceFixedColumnsWriteError> {
    match evaluate_source_fixed_static_value_expression(context, expression) {
        Some(FixedFileTemplateValue::Integer(value)) => Ok(value),
        _ => Err(source_fixed_expression_unsupported(context, expression)),
    }
}

fn evaluate_source_fixed_static_value_expression(
    context: &SourceFixedExpressionContext<'_>,
    expression: &Expression,
) -> Option<FixedFileTemplateValue> {
    evaluate_source_fixed_template_value_expression(expression, context.constant_values).or_else(
        || {
            evaluate_source_static_expression(
                context.program,
                expression,
                &context.constant_values.scalars,
            )
        },
    )
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

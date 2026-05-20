use std::collections::BTreeSet;

use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, Expression, ExpressionKind,
    FixedFileTemplateValue, FunctionStatementKind, SourceProgram, UnaryOperator,
};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::source_static_if_statement_is_false,
};

pub(crate) fn source_opening_points(
    program: &SourceProgram,
    constant_values: &std::collections::BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
) -> Result<Vec<i64>, SourceKeyDirectoryMetadataError> {
    let mut points = vec![0_i64];
    for module in &program.modules {
        for template in &module.air_templates {
            if !active_templates.contains(&template.name) {
                continue;
            }
            for statement in &template.statements {
                if statement.kind != FunctionStatementKind::Expression
                    || source_static_if_statement_is_false(
                        program,
                        module,
                        statement,
                        constant_values,
                    )
                {
                    continue;
                }
                if let Some(expression) = statement.value_expression.as_ref() {
                    collect_source_opening_points(expression, constant_values, &mut points)?;
                }
            }
        }
    }
    Ok(points)
}

fn collect_source_opening_points(
    expression: &Expression,
    constant_values: &std::collections::BTreeMap<String, FixedFileTemplateValue>,
    points: &mut Vec<i64>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    match &expression.kind {
        ExpressionKind::Group(inner) => {
            collect_source_opening_points(inner, constant_values, points)
        }
        ExpressionKind::Unary { expr, .. } => {
            collect_source_opening_points(expr, constant_values, points)
        }
        ExpressionKind::Binary { left, right, .. } => {
            collect_source_opening_points(left, constant_values, points)?;
            collect_source_opening_points(right, constant_values, points)
        }
        ExpressionKind::Call { callee, args } => {
            collect_source_opening_points(callee, constant_values, points)?;
            for arg in args {
                collect_source_opening_points(&arg.value, constant_values, points)?;
            }
            Ok(())
        }
        ExpressionKind::Index { target, index } => {
            collect_source_opening_points(target, constant_values, points)?;
            collect_source_opening_points(index, constant_values, points)
        }
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => {
            collect_source_opening_points(target, constant_values, points)?;
            collect_source_opening_points(offset, constant_values, points)?;
            if *prior {
                return Ok(());
            }
            let offset = eval_i128_expression_with_values(offset, constant_values)?;
            if offset < 0 {
                return Ok(());
            }
            let offset = i64::try_from(offset)
                .map_err(|_| unsupported_source_message("source row offset overflow"))?;
            if !points.contains(&offset) {
                points.push(offset);
            }
            Ok(())
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::Name(_)
        | ExpressionKind::PositionalParam(_) => Ok(()),
    }
}

fn eval_i128_expression_with_values(
    expression: &Expression,
    values: &std::collections::BTreeMap<String, FixedFileTemplateValue>,
) -> Result<i128, SourceKeyDirectoryMetadataError> {
    if let Some(FixedFileTemplateValue::Integer(value)) =
        evaluate_fixed_file_template_value_expression_with_values(expression, values)
    {
        return Ok(value);
    }
    eval_i128_expression(expression)
}

fn eval_i128_expression(expression: &Expression) -> Result<i128, SourceKeyDirectoryMetadataError> {
    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128_literal(value)
        }
        ExpressionKind::Group(value) => eval_i128_expression(value),
        ExpressionKind::Unary { op, expr } => {
            let value = eval_i128_expression(expr)?;
            match op {
                UnaryOperator::Plus => Ok(value),
                UnaryOperator::Minus => value
                    .checked_neg()
                    .ok_or_else(|| unsupported_source_message("source expression overflow")),
                _ => unsupported("unsupported source unary expression"),
            }
        }
        _ => unsupported("source row offset must be a static integer"),
    }
}

fn parse_i128_literal(value: &str) -> Result<i128, SourceKeyDirectoryMetadataError> {
    let value = value.trim().replace('_', "");
    if let Some(hex) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        let parsed = i128::from_str_radix(hex, 16)
            .map_err(|_| unsupported_source_message("invalid source integer literal"))?;
        parsed
            .checked_neg()
            .ok_or_else(|| unsupported_source_message("source integer literal overflow"))
    } else if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        i128::from_str_radix(hex, 16)
            .map_err(|_| unsupported_source_message("invalid source integer literal"))
    } else {
        value
            .parse::<i128>()
            .map_err(|_| unsupported_source_message("invalid source integer literal"))
    }
}

fn unsupported<T>(message: impl Into<String>) -> Result<T, SourceKeyDirectoryMetadataError> {
    Err(unsupported_source_message(message))
}

fn unsupported_source_message(message: impl Into<String>) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}

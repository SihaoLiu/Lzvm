use std::collections::BTreeMap;

use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, FunctionStatement,
    SourceProgram, SourceProgramModule,
};

use crate::{
    source_expression_info::source_call_expression,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_statement_hints::source_statement_line,
    source_static_values::{
        evaluate_source_static_expression, source_static_array_length, static_value_integer,
    },
};

pub(crate) fn source_static_assertion(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some((name, arguments)) = source_call_expression(statement.value_expression.as_ref())
    else {
        return Ok(false);
    };
    if name == "assert" && (1..=2).contains(&arguments.len()) && arguments[0].name.is_none() {
        return match source_static_condition(program, &arguments[0].value, values) {
            Some(true) => Ok(true),
            Some(false) => Err(SourceKeyDirectoryMetadataError::StaticAssertionFailed {
                line: source_statement_line(module, statement),
            }),
            None => Ok(false),
        };
    }
    if name != "assert_eq" || arguments.len() != 2 || arguments.iter().any(|arg| arg.name.is_some())
    {
        return Ok(false);
    }
    let left = evaluate_source_static_expression(program, &arguments[0].value, values);
    let right = evaluate_source_static_expression(program, &arguments[1].value, values);
    match (left, right) {
        (Some(left), Some(right)) if left == right => Ok(true),
        (Some(_), Some(_)) => Err(SourceKeyDirectoryMetadataError::StaticAssertionFailed {
            line: source_statement_line(module, statement),
        }),
        _ => Ok(false),
    }
}

fn source_static_condition(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<bool> {
    if let Some(value) = evaluate_source_static_expression(program, expression, values) {
        return Some(source_static_truthy_value(&value));
    }
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_group_expression(expression).kind
    else {
        return None;
    };
    let left = source_static_integer_expression(program, left, values)?;
    let right = source_static_integer_expression(program, right, values)?;
    match op {
        BinaryOperator::Less => Some(left < right),
        BinaryOperator::LessEqual => Some(left <= right),
        BinaryOperator::Greater => Some(left > right),
        BinaryOperator::GreaterEqual => Some(left >= right),
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => Some(left == right),
        BinaryOperator::NotEqual => Some(left != right),
        _ => None,
    }
}

fn source_static_integer_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<i128> {
    let expression = strip_source_group_expression(expression);
    if let ExpressionKind::Call { callee, args } = &expression.kind {
        if args.len() == 1 && args[0].name.is_none() {
            if let ExpressionKind::Name(callee) = &strip_source_group_expression(callee).kind {
                if callee == "length" {
                    let name =
                        source_static_indexed_array_target_name(program, &args[0].value, values)?;
                    return source_static_array_length(values, &name);
                }
            }
        }
    }
    let value = evaluate_source_static_expression(program, expression, values)?;
    static_value_integer(&value)
}

fn strip_source_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_source_group_expression(inner),
        _ => expression,
    }
}

fn source_static_indexed_array_target_name(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<String> {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(name.clone()),
        ExpressionKind::Index { target, index } => {
            let name = source_static_indexed_array_target_name(program, target, values)?;
            let index = evaluate_source_static_expression(program, index, values)?;
            let index = usize::try_from(static_value_integer(&index)?).ok()?;
            Some(format!("{name}[{index}]"))
        }
        _ => None,
    }
}

fn source_static_truthy_value(value: &FixedFileTemplateValue) -> bool {
    match value {
        FixedFileTemplateValue::Integer(value) => *value != 0,
        FixedFileTemplateValue::Boolean(value) => *value,
        FixedFileTemplateValue::String(value) => !value.is_empty(),
    }
}

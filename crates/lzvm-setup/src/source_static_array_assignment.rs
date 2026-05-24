use std::collections::BTreeMap;

use lzvm_pil::{
    parse_expression_tokens, BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue,
    SourceProgram, SourceProgramModule, Token, UnaryOperator,
};

use crate::source_static_values::{
    evaluate_source_static_expression, source_static_array_element,
    source_static_array_element_key, source_static_array_length, static_value_integer,
};

pub(crate) fn execute_source_static_array_assignment_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    let (expression, consumed) =
        parse_expression_tokens(tokens, index, end, &module.source).ok()?;
    if consumed != end {
        return None;
    }
    execute_source_static_array_assignment_expression(program, &expression, values)
}

fn execute_source_static_array_assignment_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    match &expression.kind {
        ExpressionKind::Binary { op, left, right } => {
            execute_source_static_array_binary_assignment(program, values, *op, left, right)
        }
        ExpressionKind::Unary { op, expr } => {
            execute_source_static_array_unary_update(program, values, *op, expr)
        }
        _ => None,
    }
}

fn execute_source_static_array_binary_assignment(
    program: &SourceProgram,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    op: BinaryOperator,
    left: &Expression,
    right: &Expression,
) -> Option<()> {
    if !matches!(
        op,
        BinaryOperator::Assign
            | BinaryOperator::PlusAssign
            | BinaryOperator::MinusAssign
            | BinaryOperator::StarAssign
    ) {
        return None;
    }
    let ExpressionKind::Index { target, index } = &left.kind else {
        return None;
    };
    let name = expression_name(target)?;
    let length = usize::try_from(source_static_array_length(values, name)?).ok()?;
    let index = evaluate_source_static_expression(program, index, values)?;
    let index = usize::try_from(static_value_integer(&index)?).ok()?;
    if index >= length {
        return None;
    }

    let right = evaluate_source_static_expression(program, right, values)?;
    let value = match op {
        BinaryOperator::Assign => right,
        BinaryOperator::PlusAssign => {
            source_static_array_integer_update(values, name, index, &right, i128::checked_add)?
        }
        BinaryOperator::MinusAssign => {
            source_static_array_integer_update(values, name, index, &right, i128::checked_sub)?
        }
        BinaryOperator::StarAssign => {
            source_static_array_integer_update(values, name, index, &right, i128::checked_mul)?
        }
        _ => return None,
    };
    values.insert(source_static_array_element_key(name, index), value);
    Some(())
}

fn execute_source_static_array_unary_update(
    program: &SourceProgram,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    op: UnaryOperator,
    expression: &Expression,
) -> Option<()> {
    let delta = match op {
        UnaryOperator::Increment => 1,
        UnaryOperator::Decrement => -1,
        _ => return None,
    };
    let ExpressionKind::Index { target, index } = &expression.kind else {
        return None;
    };
    let name = expression_name(target)?;
    let length = usize::try_from(source_static_array_length(values, name)?).ok()?;
    let index = evaluate_source_static_expression(program, index, values)?;
    let index = usize::try_from(static_value_integer(&index)?).ok()?;
    if index >= length {
        return None;
    }

    let current = source_static_array_element(values, name, index)?;
    let value = static_value_integer(&current)?
        .checked_add(delta)
        .map(FixedFileTemplateValue::Integer)?;
    values.insert(source_static_array_element_key(name, index), value);
    Some(())
}

fn source_static_array_integer_update(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
    index: usize,
    right: &FixedFileTemplateValue,
    op: impl FnOnce(i128, i128) -> Option<i128>,
) -> Option<FixedFileTemplateValue> {
    let current = source_static_array_element(values, name, index)?;
    op(
        static_value_integer(&current)?,
        static_value_integer(right)?,
    )
    .map(FixedFileTemplateValue::Integer)
}

fn expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => expression_name(inner),
        _ => None,
    }
}

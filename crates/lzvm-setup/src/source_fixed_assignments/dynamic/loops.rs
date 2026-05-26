use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use lzvm_pil::{
    parse_expression_tokens, BinaryOperator, Expression, ExpressionKind, FunctionStatement,
    FunctionStatementDeclaration, FunctionStatementKind, SourceSpan, Token, TokenKind,
    UnaryOperator,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_fixed_columns::SourceFixedColumnsWriteError,
};

use super::{SourceFixedDynamicOperation, SOURCE_FIXED_DYNAMIC_LOCAL_FOR_LIMIT};
use crate::source_fixed_assignments::{
    evaluate_source_fixed_assignment_value_expression, source_fixed_assignment_integer,
    source_fixed_physical_assignment_column_name, strip_source_fixed_group_expression,
    SourceFixedAssignmentValues, SourceFixedTemplateAssignmentContext,
};

pub(super) struct SourceFixedDynamicForLoop {
    pub(super) body_statements: Arc<[FunctionStatement]>,
    pub(super) variable_name: String,
    pub(super) start: usize,
    pub(super) count: usize,
}

pub(super) struct SourceFixedDynamicLocalForLoop {
    pub(super) body_statements: Arc<[FunctionStatement]>,
    pub(super) variable_name: String,
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn source_fixed_dynamic_local_for_loop(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicLocalForLoop>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::For {
        return Ok(None);
    }
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some(variable) = source_fixed_dynamic_for_variable(statement) else {
        return Ok(None);
    };
    let Some(initializer) = variable.initializer_expression.as_ref() else {
        return Ok(None);
    };
    let Some(start) = source_fixed_dynamic_usize_expression(initializer, assignment_values) else {
        return Ok(None);
    };
    let Some(header) = statement.header else {
        return Ok(None);
    };
    let Some(condition) = source_fixed_dynamic_for_header(
        context,
        header,
        body_cache,
        &variable.name,
        assignment_values,
    )?
    else {
        return Ok(None);
    };
    let Some(end) =
        source_fixed_dynamic_for_condition_end(&condition, &variable.name, assignment_values)
    else {
        return Ok(None);
    };
    let Some(count) = end.checked_sub(start) else {
        return Ok(None);
    };
    if count > SOURCE_FIXED_DYNAMIC_LOCAL_FOR_LIMIT {
        return Ok(None);
    }
    let body_statements = body_cache
        .body_statements(context.tokens, body, &context.module.source)
        .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
            source_name: context.module.source_name.clone(),
            source_span: body,
            source,
        })?;
    Ok(Some(SourceFixedDynamicLocalForLoop {
        body_statements,
        variable_name: variable.name.clone(),
        start,
        end,
    }))
}

pub(super) fn source_fixed_dynamic_for_loop(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicForLoop>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::For {
        return Ok(None);
    }
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some(variable) = source_fixed_dynamic_for_variable(statement) else {
        return Ok(None);
    };
    let Some(initializer) = variable.initializer_expression.as_ref() else {
        return Ok(None);
    };
    let Some(start) = source_fixed_dynamic_usize_expression(initializer, assignment_values) else {
        return Ok(None);
    };
    let Some(header) = statement.header else {
        return Ok(None);
    };
    let Some(condition) = source_fixed_dynamic_for_header(
        context,
        header,
        body_cache,
        &variable.name,
        assignment_values,
    )?
    else {
        return Ok(None);
    };
    let Some(end) =
        source_fixed_dynamic_for_condition_end(&condition, &variable.name, assignment_values)
    else {
        return Ok(None);
    };
    if end < start {
        return Ok(None);
    }
    let count = end - start;
    if start != 0 || count != context.row_count {
        return Ok(None);
    }
    let body_statements = body_cache
        .body_statements(context.tokens, body, &context.module.source)
        .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
            source_name: context.module.source_name.clone(),
            source_span: body,
            source,
        })?;
    Ok(Some(SourceFixedDynamicForLoop {
        body_statements,
        variable_name: variable.name.clone(),
        start,
        count,
    }))
}

fn source_fixed_dynamic_for_variable(
    statement: &FunctionStatement,
) -> Option<&lzvm_pil::VariableDeclaration> {
    let Some(FunctionStatementDeclaration::Variable(declaration)) =
        statement.header_declaration.as_ref()
    else {
        return None;
    };
    if declaration.type_name != "int" || !declaration.array_dims.is_empty() {
        return None;
    }
    Some(declaration)
}

fn source_fixed_dynamic_for_header(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    header: SourceSpan,
    body_cache: &mut SourceControlBodyCache,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Result<Option<Expression>, SourceFixedColumnsWriteError> {
    let Some((open, close)) = source_fixed_header_token_bounds(context.tokens, header, body_cache)
    else {
        return Ok(None);
    };
    let semicolons = source_fixed_for_header_semicolons(context.tokens, open + 1, close);
    let [first_semicolon, second_semicolon] = semicolons.as_slice() else {
        return Ok(None);
    };
    let (condition, consumed) = parse_expression_tokens(
        context.tokens,
        first_semicolon + 1,
        *second_semicolon,
        &context.module.source,
    )
    .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
        source_name: context.module.source_name.clone(),
        source_span: header,
        source,
    })?;
    if consumed != *second_semicolon {
        return Ok(None);
    }
    if !source_fixed_dynamic_for_update_is_unit_increment(
        context,
        second_semicolon + 1,
        close,
        variable_name,
        assignment_values,
    )? {
        return Ok(None);
    }
    Ok(Some(condition))
}

fn source_fixed_header_token_bounds(
    tokens: &[Token],
    header: SourceSpan,
    body_cache: &mut SourceControlBodyCache,
) -> Option<(usize, usize)> {
    let (open, close) = body_cache.span_token_bounds(tokens, header)?;
    let close = close.checked_sub(1)?;
    (open < close).then_some((open, close))
}

fn source_fixed_for_header_semicolons(tokens: &[Token], start: usize, end: usize) -> Vec<usize> {
    let mut semicolons = Vec::new();
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                depth = depth.saturating_add(1);
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Semicolon if depth == 0 => semicolons.push(index),
            _ => {}
        }
    }
    semicolons
}

fn source_fixed_dynamic_for_update_is_unit_increment(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    start: usize,
    end: usize,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    if source_fixed_dynamic_for_postfix_delta(context.tokens, start, end, variable_name) == Some(1)
    {
        return Ok(true);
    }
    let (expression, consumed) =
        parse_expression_tokens(context.tokens, start, end, &context.module.source).map_err(
            |source| SourceFixedColumnsWriteError::ExpressionParse {
                source_name: context.module.source_name.clone(),
                source_span: SourceSpan {
                    start: context.tokens.get(start).map_or(0, |token| token.start),
                    end: context
                        .tokens
                        .get(end.saturating_sub(1))
                        .map_or(0, |token| token.end),
                },
                source,
            },
        )?;
    if consumed != end {
        return Ok(false);
    }
    Ok(
        source_fixed_dynamic_for_update_delta(&expression, variable_name, assignment_values)
            == Some(1),
    )
}

fn source_fixed_dynamic_for_postfix_delta(
    tokens: &[Token],
    start: usize,
    end: usize,
    variable_name: &str,
) -> Option<i128> {
    let first = tokens.get(start)?;
    let second = tokens.get(start + 1)?;
    if start + 2 != end || first.lexeme != variable_name {
        return None;
    }
    match second.kind {
        TokenKind::Increment => Some(1),
        TokenKind::Decrement => Some(-1),
        _ => None,
    }
}

fn source_fixed_dynamic_for_update_delta(
    expression: &Expression,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<i128> {
    match &strip_source_fixed_group_expression(expression).kind {
        ExpressionKind::Unary { op, expr } => {
            if !source_fixed_expression_is_loop_variable(expr, variable_name) {
                return None;
            }
            match op {
                UnaryOperator::Increment => Some(1),
                UnaryOperator::Decrement => Some(-1),
                _ => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            if !source_fixed_expression_is_loop_variable(left, variable_name) {
                return None;
            }
            match op {
                BinaryOperator::Assign => {
                    source_fixed_assignment_update_delta(right, variable_name, assignment_values)
                }
                BinaryOperator::PlusAssign => {
                    source_fixed_dynamic_integer_expression(right, assignment_values)
                }
                BinaryOperator::MinusAssign => {
                    source_fixed_dynamic_integer_expression(right, assignment_values)?.checked_neg()
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn source_fixed_assignment_update_delta(
    expression: &Expression,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<i128> {
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return None;
    };
    if !source_fixed_expression_is_loop_variable(left, variable_name) {
        return None;
    }
    let delta = source_fixed_dynamic_integer_expression(right, assignment_values)?;
    match op {
        BinaryOperator::Add => Some(delta),
        BinaryOperator::Subtract => delta.checked_neg(),
        _ => None,
    }
}

fn source_fixed_dynamic_for_condition_end(
    expression: &Expression,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<usize> {
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return None;
    };
    if !source_fixed_expression_is_loop_variable(left, variable_name) {
        return None;
    }
    let right = source_fixed_dynamic_usize_expression(right, assignment_values)?;
    match op {
        BinaryOperator::Less => Some(right),
        BinaryOperator::LessEqual => right.checked_add(1),
        _ => None,
    }
}

pub(super) fn source_fixed_dynamic_assignment_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    loop_variable: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Result<Option<(String, Expression)>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(None);
    }
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(None);
    };
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return Ok(None);
    };
    if *op != BinaryOperator::Assign {
        return Ok(None);
    }
    let Some(target_column) = source_fixed_dynamic_assignment_target(
        left,
        loop_variable,
        context.expected_columns,
        context.logical_dimensions,
        assignment_values,
    ) else {
        return Ok(None);
    };
    Ok(Some((target_column, (**right).clone())))
}

fn source_fixed_dynamic_assignment_target(
    expression: &Expression,
    loop_variable: &str,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<String> {
    let ExpressionKind::Index { target, index } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return None;
    };
    if !source_fixed_expression_is_loop_variable(index, loop_variable) {
        return None;
    }
    source_fixed_physical_assignment_column_name(
        target,
        expected_columns,
        logical_dimensions,
        values,
    )
}

fn source_fixed_expression_is_loop_variable(expression: &Expression, variable_name: &str) -> bool {
    match &strip_source_fixed_group_expression(expression).kind {
        ExpressionKind::Name(name) => name == variable_name,
        _ => false,
    }
}

fn source_fixed_dynamic_usize_expression(
    expression: &Expression,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<usize> {
    let value = source_fixed_dynamic_integer_expression(expression, assignment_values)?;
    usize::try_from(value).ok()
}

fn source_fixed_dynamic_integer_expression(
    expression: &Expression,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<i128> {
    let value = evaluate_source_fixed_assignment_value_expression(expression, assignment_values)?;
    source_fixed_assignment_integer(&value)
}

pub(super) fn source_fixed_dynamic_range_end(
    operation: &SourceFixedDynamicOperation,
    row_count: usize,
) -> Result<usize, SourceFixedColumnsWriteError> {
    let end = operation
        .start
        .checked_add(operation.count)
        .ok_or_else(|| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: operation.source_name.clone(),
            source_span: operation.source_span,
            expression: operation.count.to_string(),
        })?;
    if end > row_count {
        return Err(SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: operation.source_name.clone(),
            source_span: operation.source_span,
            expression: end.to_string(),
        });
    }
    Ok(end)
}

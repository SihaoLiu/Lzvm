use std::collections::BTreeMap;

use lzvm_pil::{
    parse_expression_tokens, BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue,
    SourceProgram, SourceProgramModule, Token, TokenKind, UnaryOperator,
};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{
        evaluate_source_static_expression, static_value_integer, static_value_truthy,
    },
};

use super::{
    lower_top_level_global_constraint, unsupported_source_message, SourceGlobalAliasScope,
    SourceGlobalConstraintBuilder, SourceGlobalSlots,
};

const STATIC_TOP_LEVEL_FOR_LOOP_LIMIT: usize = 10_000;

pub(super) fn lower_top_level_static_for_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    slots: &SourceGlobalSlots<'_>,
    alias_scope: &SourceGlobalAliasScope<'_>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<Option<usize>, SourceKeyDirectoryMetadataError> {
    let loop_info =
        parse_top_level_for_loop(program, module, tokens, index, &alias_scope.static_values)?;
    let Some(loop_info) = loop_info else {
        return Ok(None);
    };
    let mut values = alias_scope.static_values.clone();
    values.insert(
        loop_info.variable_name.clone(),
        FixedFileTemplateValue::Integer(loop_info.initial_value),
    );
    let checkpoint = constraints.checkpoint();

    for _ in 0..STATIC_TOP_LEVEL_FOR_LOOP_LIMIT {
        let Some(condition_value) =
            evaluate_source_static_expression(program, &loop_info.condition, &values)
        else {
            constraints.rollback(checkpoint);
            return Ok(None);
        };
        if !static_value_truthy(&condition_value) {
            return Ok(Some(loop_info.next_index));
        }
        let iteration_alias_scope = SourceGlobalAliasScope {
            program: alias_scope.program,
            expressions: alias_scope.expressions.clone(),
            expression_arrays: alias_scope.expression_arrays.clone(),
            static_values: values.clone(),
        };
        if !lower_top_level_for_body(
            module,
            tokens,
            loop_info.body_start,
            loop_info.body_end,
            slots,
            &iteration_alias_scope,
            constraints,
        )? {
            constraints.rollback(checkpoint);
            return Ok(None);
        }
        apply_top_level_for_update(
            program,
            &loop_info.update,
            &loop_info.variable_name,
            &mut values,
        )?;
    }
    constraints.rollback(checkpoint);
    Ok(None)
}

struct TopLevelForLoop {
    variable_name: String,
    initial_value: i128,
    condition: Expression,
    update: TopLevelForUpdate,
    body_start: usize,
    body_end: usize,
    next_index: usize,
}

enum TopLevelForUpdate {
    Expression(Expression),
    Postfix { name: String, delta: i128 },
}

fn parse_top_level_for_loop(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    static_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Option<TopLevelForLoop>, SourceKeyDirectoryMetadataError> {
    if tokens.get(index).map(|token| token.kind) != Some(TokenKind::For) {
        return Err(unsupported_source_message("top-level for loop expected"));
    }
    let open_index = index
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("top-level for loop header overflow"))?;
    if tokens.get(open_index).map(|token| token.kind) != Some(TokenKind::LParen) {
        return Err(unsupported_source_message(
            "top-level for loop header must be parenthesized",
        ));
    }
    let close_index = matching_delimiter(tokens, open_index, TokenKind::RParen)?;
    let [initializer_range, condition_range, update_range] =
        split_for_header_ranges(tokens, open_index + 1, close_index)?;
    let body_open = close_index
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("top-level for loop body overflow"))?;
    if tokens.get(body_open).map(|token| token.kind) != Some(TokenKind::LBrace) {
        return Err(unsupported_source_message(
            "top-level for loop body must be braced",
        ));
    }
    let body_close = matching_delimiter(tokens, body_open, TokenKind::RBrace)?;
    let Some((variable_name, initial_value)) =
        parse_for_initializer(program, module, tokens, initializer_range, static_values)?
    else {
        return Ok(None);
    };
    let condition = parse_expression_range(module, tokens, condition_range)?;
    let Some(update) = parse_for_update(module, tokens, update_range)? else {
        return Ok(None);
    };
    Ok(Some(TopLevelForLoop {
        variable_name,
        initial_value,
        condition,
        update,
        body_start: body_open + 1,
        body_end: body_close,
        next_index: body_close + 1,
    }))
}

fn split_for_header_ranges(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Result<[(usize, usize); 3], SourceKeyDirectoryMetadataError> {
    let mut ranges = Vec::new();
    let mut range_start = start;
    let mut stack = Vec::<TokenKind>::new();
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            ranges.push((range_start, index));
            range_start = index + 1;
            continue;
        }
        update_delimiter_stack(token.kind, &mut stack)?;
    }
    ranges.push((range_start, end));
    if !stack.is_empty() {
        return Err(unsupported_source_message(
            "top-level for loop header delimiters are not balanced",
        ));
    }
    <[(usize, usize); 3]>::try_from(ranges).map_err(|_| {
        unsupported_source_message(
            "top-level for loop header must have initializer, condition, and update",
        )
    })
}

fn parse_for_initializer(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    range: (usize, usize),
    static_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Option<(String, i128)>, SourceKeyDirectoryMetadataError> {
    let mut cursor = range.0;
    if tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Const) {
        cursor += 1;
    }
    if tokens.get(cursor).map(|token| token.kind) != Some(TokenKind::Int) {
        return Ok(None);
    }
    let name_index = cursor + 1;
    let assign_index = cursor + 2;
    let Some(name) = tokens.get(name_index) else {
        return Ok(None);
    };
    if name.kind != TokenKind::Identifier {
        return Ok(None);
    }
    if tokens.get(assign_index).map(|token| token.kind) != Some(TokenKind::Assign) {
        return Ok(None);
    }
    let expression = parse_expression_range(module, tokens, (assign_index + 1, range.1))?;
    let Some(value) = evaluate_source_static_expression(program, &expression, static_values) else {
        return Ok(None);
    };
    let Some(value) = static_value_integer(&value) else {
        return Ok(None);
    };
    Ok(Some((name.lexeme.clone(), value)))
}

fn parse_for_update(
    module: &SourceProgramModule,
    tokens: &[Token],
    range: (usize, usize),
) -> Result<Option<TopLevelForUpdate>, SourceKeyDirectoryMetadataError> {
    if range.0 + 2 == range.1 {
        let name = &tokens[range.0];
        let update = &tokens[range.0 + 1];
        if name.kind == TokenKind::Identifier {
            let delta = match update.kind {
                TokenKind::Increment => Some(1),
                TokenKind::Decrement => Some(-1),
                _ => None,
            };
            if let Some(delta) = delta {
                return Ok(Some(TopLevelForUpdate::Postfix {
                    name: name.lexeme.clone(),
                    delta,
                }));
            }
        }
    }
    parse_expression_range(module, tokens, range)
        .map(TopLevelForUpdate::Expression)
        .map(Some)
}

fn lower_top_level_for_body(
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    slots: &SourceGlobalSlots<'_>,
    alias_scope: &SourceGlobalAliasScope<'_>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let mut index = start;
    while index < end {
        let token = &tokens[index];
        if matches!(token.kind, TokenKind::Semicolon) {
            index += 1;
            continue;
        }
        if token.kind == TokenKind::For {
            return Ok(false);
        }
        let next_index = skip_statement_until(tokens, index, end)?;
        let expression_end = next_index.checked_sub(1).ok_or_else(|| {
            unsupported_source_message("top-level for loop body has no expression")
        })?;
        let expression = parse_expression_range(module, tokens, (index, expression_end))?;
        match lower_top_level_global_constraint(
            &expression,
            &module.source.contents[expression.start..expression.end],
            slots,
            alias_scope,
            constraints,
        ) {
            Ok(()) => {}
            Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
        index = next_index;
    }
    Ok(true)
}

fn apply_top_level_for_update(
    program: &SourceProgram,
    update: &TopLevelForUpdate,
    variable_name: &str,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    match update {
        TopLevelForUpdate::Postfix { name, delta } => {
            if name != variable_name {
                return Err(unsupported_source_message(
                    "top-level for loop update must target the loop variable",
                ));
            }
            apply_loop_delta(variable_name, *delta, values)
        }
        TopLevelForUpdate::Expression(expression) => {
            apply_top_level_for_expression_update(program, expression, variable_name, values)
        }
    }
}

fn apply_top_level_for_expression_update(
    program: &SourceProgram,
    expression: &Expression,
    variable_name: &str,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            if expression_name(expr) != Some(variable_name) {
                return Err(unsupported_source_message(
                    "top-level for loop update must target the loop variable",
                ));
            }
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => {
                    return Err(unsupported_source_message(
                        "top-level for loop update must be static",
                    ))
                }
            };
            apply_loop_delta(variable_name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            if expression_name(left) != Some(variable_name) {
                return Err(unsupported_source_message(
                    "top-level for loop update must target the loop variable",
                ));
            }
            let Some(right) = evaluate_source_static_expression(program, right, values) else {
                return Err(unsupported_source_message(
                    "top-level for loop update must be static",
                ));
            };
            let Some(right) = static_value_integer(&right) else {
                return Err(unsupported_source_message(
                    "top-level for loop update must be an integer",
                ));
            };
            match op {
                BinaryOperator::Assign => {
                    values.insert(
                        variable_name.to_owned(),
                        FixedFileTemplateValue::Integer(right),
                    );
                    Ok(())
                }
                BinaryOperator::PlusAssign => apply_loop_delta(variable_name, right, values),
                BinaryOperator::MinusAssign => apply_loop_delta(variable_name, -right, values),
                _ => Err(unsupported_source_message(
                    "top-level for loop update must be static",
                )),
            }
        }
        _ => Err(unsupported_source_message(
            "top-level for loop update must be static",
        )),
    }
}

fn apply_loop_delta(
    variable_name: &str,
    delta: i128,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let Some(current) = values.get(variable_name).and_then(static_value_integer) else {
        return Err(unsupported_source_message(
            "top-level for loop variable must be an integer",
        ));
    };
    let Some(value) = current.checked_add(delta) else {
        return Err(unsupported_source_message(
            "top-level for loop update overflow",
        ));
    };
    values.insert(
        variable_name.to_owned(),
        FixedFileTemplateValue::Integer(value),
    );
    Ok(())
}

fn expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => expression_name(inner),
        _ => None,
    }
}

fn parse_expression_range(
    module: &SourceProgramModule,
    tokens: &[Token],
    range: (usize, usize),
) -> Result<Expression, SourceKeyDirectoryMetadataError> {
    let (expression, consumed) = parse_expression_tokens(tokens, range.0, range.1, &module.source)?;
    if consumed != range.1 {
        return Err(unsupported_source_message(
            "top-level for loop expression has unsupported trailing tokens",
        ));
    }
    Ok(expression)
}

fn skip_statement_until(
    tokens: &[Token],
    index: usize,
    end: usize,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let mut stack = Vec::<TokenKind>::new();
    for (cursor, token) in tokens.iter().enumerate().take(end).skip(index) {
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Ok(cursor + 1);
        }
        update_delimiter_stack(token.kind, &mut stack)?;
    }
    Err(unsupported_source_message(
        "top-level for loop body statement has no terminator",
    ))
}

fn matching_delimiter(
    tokens: &[Token],
    index: usize,
    close_kind: TokenKind,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let open_kind = tokens
        .get(index)
        .map(|token| token.kind)
        .ok_or_else(|| unsupported_source_message("top-level for loop delimiter missing"))?;
    let mut depth = 0_usize;
    for (cursor, token) in tokens.iter().enumerate().skip(index) {
        if token.kind == open_kind {
            depth = depth
                .checked_add(1)
                .ok_or_else(|| unsupported_source_message("top-level for loop nesting overflow"))?;
        } else if token.kind == close_kind {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| unsupported_source_message("top-level for loop body underflow"))?;
            if depth == 0 {
                return Ok(cursor);
            }
        }
    }
    Err(unsupported_source_message(
        "top-level for loop delimiter is not closed",
    ))
}

fn update_delimiter_stack(
    kind: TokenKind,
    stack: &mut Vec<TokenKind>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    match kind {
        TokenKind::LParen => stack.push(TokenKind::RParen),
        TokenKind::LBracket => stack.push(TokenKind::RBracket),
        TokenKind::LBrace => stack.push(TokenKind::RBrace),
        TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
            let Some(expected) = stack.pop() else {
                return Err(unsupported_source_message(
                    "top-level for loop has an unmatched closing delimiter",
                ));
            };
            if kind != expected {
                return Err(unsupported_source_message(
                    "top-level for loop delimiters are not balanced",
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

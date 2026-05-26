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
    lower_top_level_global_constraints_range, unsupported_source_message, SourceGlobalAliasScope,
    SourceGlobalConstraintBuilder, SourceTopLevelGlobalConstraintContext,
};

pub(super) const STATIC_TOP_LEVEL_FOR_LOOP_LIMIT: usize = 10_000;

pub(super) struct TopLevelForOutcome {
    pub(super) next_index: usize,
    pub(super) final_variable_value: Option<(String, FixedFileTemplateValue)>,
}

pub(super) fn lower_top_level_static_for_statement(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    index: usize,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<Option<TopLevelForOutcome>, SourceKeyDirectoryMetadataError> {
    let loop_info = parse_top_level_for_loop(
        context.program,
        context.module,
        context.tokens,
        index,
        &context.alias_scope.static_values,
    )?;
    let Some(loop_info) = loop_info else {
        return Ok(None);
    };
    let mut values = context.alias_scope.static_values.clone();
    loop_info.apply_initial_value(&mut values);
    let checkpoint = constraints.checkpoint();

    for _ in 0..STATIC_TOP_LEVEL_FOR_LOOP_LIMIT {
        let Some(condition_truthy) = loop_info.condition_truthy(context.program, &values) else {
            constraints.rollback(checkpoint);
            return Ok(None);
        };
        if !condition_truthy {
            return Ok(Some(TopLevelForOutcome {
                next_index: loop_info.next_index,
                final_variable_value: loop_info.final_variable_value(&values),
            }));
        }
        let iteration_alias_scope = SourceGlobalAliasScope {
            program: context.alias_scope.program,
            expressions: context.alias_scope.expressions.clone(),
            expression_arrays: context.alias_scope.expression_arrays.clone(),
            static_values: values.clone(),
        };
        if !lower_top_level_for_body(
            context,
            loop_info.body_start,
            loop_info.body_end,
            &iteration_alias_scope,
            constraints,
        )? {
            constraints.rollback(checkpoint);
            return Ok(None);
        }
        loop_info.apply_update(context.program, &mut values)?;
    }
    constraints.rollback(checkpoint);
    Ok(None)
}

pub(super) struct TopLevelForLoop {
    variable_name: String,
    initial_value: i128,
    updates_existing_variable: bool,
    condition: Expression,
    update: TopLevelForUpdate,
    body_start: usize,
    body_end: usize,
    next_index: usize,
}

impl TopLevelForLoop {
    pub(super) fn apply_initial_value(
        &self,
        values: &mut BTreeMap<String, FixedFileTemplateValue>,
    ) {
        values.insert(
            self.variable_name.clone(),
            FixedFileTemplateValue::Integer(self.initial_value),
        );
    }

    pub(super) fn condition_truthy(
        &self,
        program: &SourceProgram,
        values: &BTreeMap<String, FixedFileTemplateValue>,
    ) -> Option<bool> {
        evaluate_source_static_expression(program, &self.condition, values)
            .map(|value| static_value_truthy(&value))
    }

    pub(super) fn apply_update(
        &self,
        program: &SourceProgram,
        values: &mut BTreeMap<String, FixedFileTemplateValue>,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        apply_top_level_for_update(program, &self.update, &self.variable_name, values)
    }

    pub(super) fn final_variable_value(
        &self,
        values: &BTreeMap<String, FixedFileTemplateValue>,
    ) -> Option<(String, FixedFileTemplateValue)> {
        self.updates_existing_variable.then(|| {
            (
                self.variable_name.clone(),
                values
                    .get(&self.variable_name)
                    .cloned()
                    .unwrap_or(FixedFileTemplateValue::Integer(self.initial_value)),
            )
        })
    }

    pub(super) fn body_start(&self) -> usize {
        self.body_start
    }

    pub(super) fn body_end(&self) -> usize {
        self.body_end
    }

    pub(super) fn next_index(&self) -> usize {
        self.next_index
    }
}

enum TopLevelForUpdate {
    Expression(Expression),
    Postfix { name: String, delta: i128 },
}

pub(super) fn parse_top_level_for_loop(
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
    let Some(initializer) =
        parse_for_initializer(program, module, tokens, initializer_range, static_values)?
    else {
        return Ok(None);
    };
    let condition = parse_expression_range(module, tokens, condition_range)?;
    let Some(update) = parse_for_update(module, tokens, update_range)? else {
        return Ok(None);
    };
    Ok(Some(TopLevelForLoop {
        variable_name: initializer.variable_name,
        initial_value: initializer.initial_value,
        updates_existing_variable: initializer.updates_existing_variable,
        condition,
        update,
        body_start: body_open + 1,
        body_end: body_close,
        next_index: body_close + 1,
    }))
}

struct TopLevelForInitializer {
    variable_name: String,
    initial_value: i128,
    updates_existing_variable: bool,
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
) -> Result<Option<TopLevelForInitializer>, SourceKeyDirectoryMetadataError> {
    let mut cursor = range.0;
    if tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Const) {
        cursor += 1;
    }
    if tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Int) {
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
        let Some(value) = evaluate_source_static_expression(program, &expression, static_values)
        else {
            return Ok(None);
        };
        let Some(value) = static_value_integer(&value) else {
            return Ok(None);
        };
        return Ok(Some(TopLevelForInitializer {
            variable_name: name.lexeme.clone(),
            initial_value: value,
            updates_existing_variable: false,
        }));
    }

    let expression = parse_expression_range(module, tokens, range)?;
    let ExpressionKind::Binary {
        op: BinaryOperator::Assign,
        left,
        right,
    } = &expression.kind
    else {
        return Ok(None);
    };
    let Some(variable_name) = expression_name(left) else {
        return Ok(None);
    };
    if !static_values.contains_key(variable_name) {
        return Ok(None);
    }
    let Some(value) = evaluate_source_static_expression(program, right, static_values) else {
        return Ok(None);
    };
    let Some(value) = static_value_integer(&value) else {
        return Ok(None);
    };
    Ok(Some(TopLevelForInitializer {
        variable_name: variable_name.to_owned(),
        initial_value: value,
        updates_existing_variable: true,
    }))
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
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    start: usize,
    end: usize,
    alias_scope: &SourceGlobalAliasScope<'_>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let iteration_context = SourceTopLevelGlobalConstraintContext {
        program: context.program,
        module: context.module,
        tokens: context.tokens,
        slots: context.slots,
        alias_scope,
    };
    match lower_top_level_global_constraints_range(&iteration_context, start, end, constraints) {
        Ok(()) => Ok(true),
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => Ok(false),
        Err(error) => Err(error),
    }
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

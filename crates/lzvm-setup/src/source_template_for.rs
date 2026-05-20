use std::collections::BTreeMap;

use lzvm_pil::{
    parse_expression_tokens, BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue,
    FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind, SourceProgram,
    SourceProgramModule, SourceSpan, Token, TokenKind, UnaryOperator,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{evaluate_source_static_expression, static_value_truthy},
};

const STATIC_FOR_LOOP_LIMIT: usize = 10_000;

pub(crate) struct SourceStaticForLoop {
    pub(crate) body_statements: Vec<FunctionStatement>,
    pub(crate) iteration_values: Vec<FixedFileTemplateValue>,
    pub(crate) variable_name: String,
}

pub(crate) fn source_static_for_loop_with_tokens(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceStaticForLoop>, SourceKeyDirectoryMetadataError> {
    if statement.kind != FunctionStatementKind::For {
        return Ok(None);
    }
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some(variable) = source_for_loop_variable(statement) else {
        return Ok(None);
    };
    let Some(initializer) = variable.initializer_expression.as_ref() else {
        return Ok(None);
    };
    let Some(initial_value) = source_static_integer(program, initializer, base_values) else {
        return Ok(None);
    };

    let Some(header) = statement.header else {
        return Ok(None);
    };
    let Some((condition, update)) = source_for_loop_header(tokens, header, module)? else {
        return Ok(None);
    };
    let body_statements = body_cache.body_statements(tokens, body, &module.source)?;

    let mut values = base_values.clone();
    values.insert(
        variable.name.clone(),
        FixedFileTemplateValue::Integer(initial_value),
    );
    let mut iteration_values = Vec::new();
    for _ in 0..STATIC_FOR_LOOP_LIMIT {
        let Some(condition_value) = evaluate_source_static_expression(program, &condition, &values)
        else {
            return Ok(None);
        };
        if !static_value_truthy(&condition_value) {
            return Ok(Some(SourceStaticForLoop {
                body_statements,
                iteration_values,
                variable_name: variable.name.clone(),
            }));
        }
        let Some(iteration_value) = values.get(&variable.name).cloned() else {
            return Ok(None);
        };
        iteration_values.push(iteration_value);
        if !apply_source_for_loop_update(program, &update, &variable.name, &mut values) {
            return Ok(None);
        }
    }
    Ok(None)
}

fn source_for_loop_variable(
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

fn source_for_loop_header(
    tokens: &[Token],
    header: SourceSpan,
    module: &SourceProgramModule,
) -> Result<Option<(Expression, SourceForLoopUpdate)>, SourceKeyDirectoryMetadataError> {
    let Some((open, close)) = source_header_token_bounds(tokens, header) else {
        return Ok(None);
    };
    let semicolons = source_for_header_semicolons(tokens, open + 1, close);
    let [first_semicolon, second_semicolon] = semicolons.as_slice() else {
        return Ok(None);
    };
    let (condition, consumed) = parse_expression_tokens(
        tokens,
        first_semicolon + 1,
        *second_semicolon,
        &module.source,
    )?;
    if consumed != *second_semicolon {
        return Ok(None);
    }
    let Some(update) = source_for_loop_update(tokens, second_semicolon + 1, close, module)? else {
        return Ok(None);
    };
    Ok(Some((condition, update)))
}

fn source_header_token_bounds(tokens: &[Token], header: SourceSpan) -> Option<(usize, usize)> {
    let open = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LParen && token.start == header.start)?;
    let close = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::RParen && token.end == header.end)?;
    (open < close).then_some((open, close))
}

fn source_for_header_semicolons(tokens: &[Token], start: usize, end: usize) -> Vec<usize> {
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

struct SourceForLoopUpdate {
    expression: Option<Expression>,
    postfix: Option<SourceForLoopPostfixUpdate>,
}

struct SourceForLoopPostfixUpdate {
    name: String,
    delta: i128,
}

fn source_for_loop_update(
    tokens: &[Token],
    start: usize,
    end: usize,
    module: &SourceProgramModule,
) -> Result<Option<SourceForLoopUpdate>, SourceKeyDirectoryMetadataError> {
    if let Some(postfix) = source_for_loop_postfix_update(tokens, start, end) {
        return Ok(Some(SourceForLoopUpdate {
            expression: None,
            postfix: Some(postfix),
        }));
    }
    let (expression, consumed) = parse_expression_tokens(tokens, start, end, &module.source)?;
    if consumed != end {
        return Ok(None);
    }
    Ok(Some(SourceForLoopUpdate {
        expression: Some(expression),
        postfix: None,
    }))
}

fn source_for_loop_postfix_update(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Option<SourceForLoopPostfixUpdate> {
    let first = tokens.get(start)?;
    let second = tokens.get(start + 1)?;
    if start + 2 != end {
        return None;
    }
    match (first.kind, second.kind) {
        (TokenKind::Identifier, TokenKind::Increment) => Some(SourceForLoopPostfixUpdate {
            name: first.lexeme.clone(),
            delta: 1,
        }),
        (TokenKind::Identifier, TokenKind::Decrement) => Some(SourceForLoopPostfixUpdate {
            name: first.lexeme.clone(),
            delta: -1,
        }),
        _ => None,
    }
}

fn apply_source_for_loop_update(
    program: &SourceProgram,
    update: &SourceForLoopUpdate,
    variable_name: &str,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    if let Some(postfix) = &update.postfix {
        return apply_source_for_loop_delta(variable_name, &postfix.name, postfix.delta, values);
    }
    let Some(expression) = update.expression.as_ref() else {
        return false;
    };
    match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            let Some(name) = source_expression_name(expr) else {
                return false;
            };
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => return false,
            };
            apply_source_for_loop_delta(variable_name, name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let Some(name) = source_expression_name(left) else {
                return false;
            };
            if name != variable_name {
                return false;
            }
            let Some(right) = source_static_integer(program, right, values) else {
                return false;
            };
            let Some(current) = source_static_integer_value(values.get(variable_name)) else {
                return false;
            };
            let next = match op {
                BinaryOperator::Assign => Some(right),
                BinaryOperator::PlusAssign => current.checked_add(right),
                BinaryOperator::MinusAssign => current.checked_sub(right),
                BinaryOperator::StarAssign => current.checked_mul(right),
                _ => None,
            };
            let Some(next) = next else {
                return false;
            };
            values.insert(
                variable_name.to_owned(),
                FixedFileTemplateValue::Integer(next),
            );
            true
        }
        _ => false,
    }
}

fn apply_source_for_loop_delta(
    variable_name: &str,
    update_name: &str,
    delta: i128,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    if update_name != variable_name {
        return false;
    }
    let Some(current) = source_static_integer_value(values.get(variable_name)) else {
        return false;
    };
    let Some(next) = current.checked_add(delta) else {
        return false;
    };
    values.insert(
        variable_name.to_owned(),
        FixedFileTemplateValue::Integer(next),
    );
    true
}

fn source_static_integer(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<i128> {
    source_static_integer_value(
        evaluate_source_static_expression(program, expression, values).as_ref(),
    )
}

fn source_static_integer_value(value: Option<&FixedFileTemplateValue>) -> Option<i128> {
    match value {
        Some(FixedFileTemplateValue::Integer(value)) => Some(*value),
        _ => None,
    }
}

fn source_expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => source_expression_name(inner),
        _ => None,
    }
}

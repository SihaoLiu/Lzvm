use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use lzvm_pil::{
    parse_expression_tokens, BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue,
    FunctionStatement, FunctionStatementKind, SourceProgram, SourceProgramModule, SourceSpan,
    Token, TokenKind, UnaryOperator,
};

use crate::{
    source_constraint_lowering::SourceExpressionAliases,
    source_control_body_cache::SourceControlBodyCache,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{
        evaluate_source_static_expression_with_lookup, static_value_integer, static_value_truthy,
        SourceStaticValueLookup,
    },
};

pub(crate) fn source_static_if_body_statements_with_tokens(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Arc<[FunctionStatement]>>, SourceKeyDirectoryMetadataError> {
    source_static_if_body_statements_with_lookup(
        program, module, tokens, statement, values, body_cache,
    )
}

pub(crate) fn source_static_if_body_statements_with_aliases(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Arc<[FunctionStatement]>>, SourceKeyDirectoryMetadataError> {
    source_static_if_body_statements_with_lookup_and_aliases(
        program,
        module,
        tokens,
        statement,
        values,
        Some(expression_aliases),
        body_cache,
    )
}

pub(crate) fn source_static_if_body_statements_with_lookup(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    values: &(impl SourceStaticValueLookup + ?Sized),
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Arc<[FunctionStatement]>>, SourceKeyDirectoryMetadataError> {
    source_static_if_body_statements_with_lookup_and_aliases(
        program, module, tokens, statement, values, None, body_cache,
    )
}

fn source_static_if_body_statements_with_lookup_and_aliases(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: Option<&SourceExpressionAliases>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Arc<[FunctionStatement]>>, SourceKeyDirectoryMetadataError> {
    if statement.kind != FunctionStatementKind::If {
        return Ok(None);
    }
    let Some((start, end)) = body_cache.span_token_bounds(
        tokens,
        SourceSpan {
            start: statement.start,
            end: statement.end,
        },
    ) else {
        return Ok(None);
    };
    let Some(selection) = source_static_if_body_span(
        program,
        module,
        tokens,
        start,
        end,
        values,
        expression_aliases,
    )?
    else {
        return Ok(None);
    };
    let Some(body) = selection else {
        return Ok(Some(Arc::from([])));
    };
    Ok(Some(body_cache.body_statements(
        tokens,
        body,
        &module.source,
    )?))
}

pub(crate) fn source_static_if_body_span_with_tokens(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    values: &(impl SourceStaticValueLookup + ?Sized),
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Option<SourceSpan>>, SourceKeyDirectoryMetadataError> {
    if statement.kind != FunctionStatementKind::If {
        return Ok(None);
    }
    let Some((start, end)) = body_cache.span_token_bounds(
        tokens,
        SourceSpan {
            start: statement.start,
            end: statement.end,
        },
    ) else {
        return Ok(None);
    };
    source_static_if_body_span(program, module, tokens, start, end, values, None)
}

fn source_static_if_body_span(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: Option<&SourceExpressionAliases>,
) -> Result<Option<Option<SourceSpan>>, SourceKeyDirectoryMetadataError> {
    if !matches!(
        tokens.get(index).map(|token| token.kind),
        Some(TokenKind::If | TokenKind::ElseIf)
    ) {
        return Ok(None);
    }
    let Some(open) = next_token_kind(tokens, index + 1, end, TokenKind::LParen) else {
        return Ok(None);
    };
    let Some(close) = matching_closing_token(tokens, open, end) else {
        return Ok(None);
    };
    let Some(condition) = source_static_token_value_with_tokens(
        program,
        module,
        tokens,
        open + 1,
        close,
        values,
        expression_aliases,
    ) else {
        return Ok(None);
    };
    let Some(body) = control_body_span(tokens, close + 1, end) else {
        return Ok(None);
    };
    if static_value_truthy(&condition) {
        if body.braced {
            return Ok(Some(Some(body.span)));
        }
        return Ok(None);
    }
    source_static_else_body_span(
        program,
        module,
        tokens,
        body.after,
        end,
        values,
        expression_aliases,
    )
}

fn source_static_else_body_span(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: Option<&SourceExpressionAliases>,
) -> Result<Option<Option<SourceSpan>>, SourceKeyDirectoryMetadataError> {
    match tokens.get(index).map(|token| token.kind) {
        Some(TokenKind::ElseIf) => source_static_if_body_span(
            program,
            module,
            tokens,
            index,
            end,
            values,
            expression_aliases,
        ),
        Some(TokenKind::Else)
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::If) =>
        {
            source_static_if_body_span(
                program,
                module,
                tokens,
                index + 1,
                end,
                values,
                expression_aliases,
            )
        }
        Some(TokenKind::Else) => {
            let Some(body) = control_body_span(tokens, index + 1, end) else {
                return Ok(None);
            };
            if body.braced {
                Ok(Some(Some(body.span)))
            } else {
                Ok(None)
            }
        }
        _ => Ok(Some(None)),
    }
}

fn strip_static_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_static_group_expression(inner),
        _ => expression,
    }
}

fn source_static_token_value_with_tokens(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: Option<&SourceExpressionAliases>,
) -> Option<FixedFileTemplateValue> {
    let (expression, consumed) =
        parse_expression_tokens(tokens, start, end, &module.source).ok()?;
    if consumed != end {
        return None;
    }
    if let Some(expression_aliases) = expression_aliases {
        if let Some(value) =
            source_static_value_with_degree(program, &expression, values, expression_aliases)
        {
            return Some(value);
        }
    }
    evaluate_source_static_expression_with_lookup(program, &expression, values)
}

fn source_static_value_with_degree(
    program: &SourceProgram,
    expression: &Expression,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: &SourceExpressionAliases,
) -> Option<FixedFileTemplateValue> {
    source_static_i128_with_degree(program, expression, values, expression_aliases)
        .map(FixedFileTemplateValue::Integer)
}

fn source_static_i128_with_degree(
    program: &SourceProgram,
    expression: &Expression,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: &SourceExpressionAliases,
) -> Option<i128> {
    if let Some(value) = evaluate_source_static_expression_with_lookup(program, expression, values)
    {
        return static_value_integer(&value);
    }
    match &expression.kind {
        ExpressionKind::Group(inner) => {
            source_static_i128_with_degree(program, inner, values, expression_aliases)
        }
        ExpressionKind::Unary { op, expr } => {
            let value = source_static_i128_with_degree(program, expr, values, expression_aliases)?;
            match op {
                UnaryOperator::Plus => Some(value),
                UnaryOperator::Minus => value.checked_neg(),
                UnaryOperator::Not => Some(i128::from(value == 0)),
                _ => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            source_static_binary_with_degree(program, *op, left, right, values, expression_aliases)
        }
        ExpressionKind::Call { callee, args } => {
            let ExpressionKind::Name(name) = &strip_static_group_expression(callee).kind else {
                return None;
            };
            if name != "degree" || args.len() != 1 || args[0].name.is_some() {
                return None;
            }
            let mut resolving_aliases = BTreeSet::new();
            source_expression_degree(
                program,
                &args[0].value,
                values,
                expression_aliases,
                &mut resolving_aliases,
            )
        }
        _ => None,
    }
}

fn source_static_binary_with_degree(
    program: &SourceProgram,
    op: BinaryOperator,
    left: &Expression,
    right: &Expression,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: &SourceExpressionAliases,
) -> Option<i128> {
    if op == BinaryOperator::LogicalAnd {
        let left = source_static_i128_with_degree(program, left, values, expression_aliases)?;
        if left == 0 {
            return Some(0);
        }
        return Some(i128::from(
            source_static_i128_with_degree(program, right, values, expression_aliases)? != 0,
        ));
    }
    if op == BinaryOperator::LogicalOr {
        let left = source_static_i128_with_degree(program, left, values, expression_aliases)?;
        if left != 0 {
            return Some(1);
        }
        return Some(i128::from(
            source_static_i128_with_degree(program, right, values, expression_aliases)? != 0,
        ));
    }

    let left = source_static_i128_with_degree(program, left, values, expression_aliases)?;
    let right = source_static_i128_with_degree(program, right, values, expression_aliases)?;
    match op {
        BinaryOperator::Add => left.checked_add(right),
        BinaryOperator::Subtract => left.checked_sub(right),
        BinaryOperator::Multiply => left.checked_mul(right),
        BinaryOperator::Divide | BinaryOperator::Backslash if right != 0 => Some(left / right),
        BinaryOperator::Modulo if right != 0 => Some(left % right),
        BinaryOperator::Power => u32::try_from(right)
            .ok()
            .and_then(|exponent| left.checked_pow(exponent)),
        BinaryOperator::ShiftLeft => u32::try_from(right)
            .ok()
            .and_then(|amount| left.checked_shl(amount)),
        BinaryOperator::ShiftRight => u32::try_from(right)
            .ok()
            .and_then(|amount| left.checked_shr(amount)),
        BinaryOperator::Less => Some(i128::from(left < right)),
        BinaryOperator::LessEqual => Some(i128::from(left <= right)),
        BinaryOperator::Greater => Some(i128::from(left > right)),
        BinaryOperator::GreaterEqual => Some(i128::from(left >= right)),
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => Some(i128::from(left == right)),
        BinaryOperator::NotEqual => Some(i128::from(left != right)),
        BinaryOperator::BitAnd => Some(left & right),
        BinaryOperator::BitXor => Some(left ^ right),
        BinaryOperator::BitOr => Some(left | right),
        _ => None,
    }
}

fn source_expression_degree(
    program: &SourceProgram,
    expression: &Expression,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: &SourceExpressionAliases,
    resolving_aliases: &mut BTreeSet<String>,
) -> Option<i128> {
    if evaluate_source_static_expression_with_lookup(program, expression, values).is_some() {
        return Some(0);
    }
    match &strip_static_group_expression(expression).kind {
        ExpressionKind::Name(name) => {
            if let Some(alias) = expression_aliases.get(name.as_str()) {
                if !resolving_aliases.insert(name.clone()) {
                    return None;
                }
                let degree = source_expression_degree(
                    program,
                    alias,
                    values,
                    expression_aliases,
                    resolving_aliases,
                );
                resolving_aliases.remove(name);
                return degree;
            }
            Some(1)
        }
        ExpressionKind::Index { .. } => Some(1),
        ExpressionKind::RowOffset { target, .. } => source_expression_degree(
            program,
            target.as_ref(),
            values,
            expression_aliases,
            resolving_aliases,
        ),
        ExpressionKind::Unary { expr, .. } => source_expression_degree(
            program,
            expr.as_ref(),
            values,
            expression_aliases,
            resolving_aliases,
        ),
        ExpressionKind::Binary { op, left, right } => source_binary_expression_degree(
            program,
            *op,
            left.as_ref(),
            right.as_ref(),
            values,
            expression_aliases,
            resolving_aliases,
        ),
        _ => None,
    }
}

fn source_binary_expression_degree(
    program: &SourceProgram,
    op: BinaryOperator,
    left: &Expression,
    right: &Expression,
    values: &(impl SourceStaticValueLookup + ?Sized),
    expression_aliases: &SourceExpressionAliases,
    resolving_aliases: &mut BTreeSet<String>,
) -> Option<i128> {
    match op {
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::EqualEqual
        | BinaryOperator::TripleEqual
        | BinaryOperator::NotEqual
        | BinaryOperator::BitAnd
        | BinaryOperator::BitXor
        | BinaryOperator::BitOr
        | BinaryOperator::LogicalAnd
        | BinaryOperator::LogicalOr => {
            let left = source_expression_degree(
                program,
                left,
                values,
                expression_aliases,
                resolving_aliases,
            )?;
            let right = source_expression_degree(
                program,
                right,
                values,
                expression_aliases,
                resolving_aliases,
            )?;
            Some(left.max(right))
        }
        BinaryOperator::Multiply => {
            let left = source_expression_degree(
                program,
                left,
                values,
                expression_aliases,
                resolving_aliases,
            )?;
            let right = source_expression_degree(
                program,
                right,
                values,
                expression_aliases,
                resolving_aliases,
            )?;
            left.checked_add(right)
        }
        BinaryOperator::Divide | BinaryOperator::Backslash | BinaryOperator::Modulo => {
            let right_degree = source_expression_degree(
                program,
                right,
                values,
                expression_aliases,
                resolving_aliases,
            )?;
            if right_degree != 0 {
                return None;
            }
            source_expression_degree(program, left, values, expression_aliases, resolving_aliases)
        }
        BinaryOperator::Power => {
            let base = source_expression_degree(
                program,
                left,
                values,
                expression_aliases,
                resolving_aliases,
            )?;
            let exponent =
                source_static_i128_with_degree(program, right, values, expression_aliases)?;
            if exponent < 0 {
                return None;
            }
            base.checked_mul(exponent)
        }
        BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight => {
            let right_degree = source_expression_degree(
                program,
                right,
                values,
                expression_aliases,
                resolving_aliases,
            )?;
            if right_degree != 0 {
                return None;
            }
            source_expression_degree(program, left, values, expression_aliases, resolving_aliases)
        }
        _ => None,
    }
}

struct SourceIfBody {
    span: SourceSpan,
    braced: bool,
    after: usize,
}

fn control_body_span(tokens: &[Token], index: usize, end: usize) -> Option<SourceIfBody> {
    match tokens.get(index)?.kind {
        TokenKind::LBrace => {
            let close = matching_closing_token(tokens, index, end)?;
            Some(SourceIfBody {
                span: SourceSpan {
                    start: tokens[index].start,
                    end: tokens[close].end,
                },
                braced: true,
                after: close + 1,
            })
        }
        _ => {
            let semicolon = next_semicolon_limited(tokens, index, end)?;
            Some(SourceIfBody {
                span: SourceSpan {
                    start: tokens[index].start,
                    end: tokens[semicolon].end,
                },
                braced: false,
                after: semicolon + 1,
            })
        }
    }
}

fn next_token_kind(tokens: &[Token], start: usize, end: usize, kind: TokenKind) -> Option<usize> {
    tokens
        .iter()
        .enumerate()
        .take(end)
        .skip(start)
        .find_map(|(index, token)| (token.kind == kind).then_some(index))
}

fn matching_closing_token(tokens: &[Token], open: usize, end: usize) -> Option<usize> {
    let (open_kind, close_kind) = match tokens.get(open)?.kind {
        TokenKind::LParen => (TokenKind::LParen, TokenKind::RParen),
        TokenKind::LBracket => (TokenKind::LBracket, TokenKind::RBracket),
        TokenKind::LBrace => (TokenKind::LBrace, TokenKind::RBrace),
        _ => return None,
    };
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().take(end).skip(open) {
        if token.kind == open_kind {
            depth = depth.checked_add(1)?;
        } else if token.kind == close_kind {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn next_semicolon_limited(tokens: &[Token], start: usize, end: usize) -> Option<usize> {
    let mut expected = Vec::<TokenKind>::new();
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        match token.kind {
            TokenKind::LParen => expected.push(TokenKind::RParen),
            TokenKind::LBracket => expected.push(TokenKind::RBracket),
            TokenKind::LBrace => expected.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if expected.pop()? != token.kind {
                    return None;
                }
            }
            TokenKind::Semicolon if expected.is_empty() => return Some(index),
            _ => {}
        }
    }
    None
}

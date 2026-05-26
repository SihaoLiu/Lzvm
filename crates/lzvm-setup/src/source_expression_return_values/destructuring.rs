use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    parse_expression_tokens, Expression, FixedFileTemplateValue, FunctionStatement,
    FunctionStatementKind, SourceSpan, Token, TokenKind,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_aliases::insert_source_expression_array_alias_binding,
    source_expression_info::{source_expression_array_alias, SourceExpressionAliasScope},
    source_statement_hints::SourceExpressionArrayAlias,
    source_template_context::SourceTemplateLoweringContext,
};

use super::{insert_source_expr_array_alias_length, source_resolved_expression_array_element};

pub(crate) fn collect_source_expr_destructuring_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
) -> bool {
    let Some(expressions) = source_expr_destructuring_expressions(context, statement, body_cache)
    else {
        return false;
    };
    for (name, expression) in expressions {
        let mut inserted_array_alias = false;
        if let Some(alias) = source_expr_destructuring_array_alias(
            context,
            &expression,
            values,
            body_cache,
            call_stack,
            alias_scope,
        ) {
            let expression_arrays = alias_scope.expression_arrays_mut();
            insert_source_expression_array_alias_binding(expression_arrays, &name, alias.clone());
            let _ = insert_source_expr_array_alias_length(values, &name, &alias, expression_arrays);
            inserted_array_alias = true;
        }
        if !inserted_array_alias {
            alias_scope.expressions_mut().insert(name, expression);
        }
    }
    true
}

fn source_expr_destructuring_array_alias(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &SourceExpressionAliasScope,
) -> Option<SourceExpressionArrayAlias> {
    if let Some(alias) = source_expression_array_alias(expression) {
        return Some(alias);
    }
    let mut resolving_array_aliases = BTreeSet::new();
    let (_, _, element) = source_resolved_expression_array_element(
        expression,
        context,
        values,
        alias_scope,
        body_cache,
        call_stack,
        true,
        true,
        &mut resolving_array_aliases,
    )?;
    source_expression_array_alias(&element)
}

fn source_expr_destructuring_expressions(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    body_cache: &mut SourceControlBodyCache,
) -> Option<Vec<(String, Expression)>> {
    if statement.kind != FunctionStatementKind::Declaration || statement.declaration.is_some() {
        return None;
    }
    let (start_index, end_index) = body_cache.span_token_bounds(
        context.tokens,
        SourceSpan {
            start: statement.start,
            end: statement.end,
        },
    )?;
    if !context
        .tokens
        .get(start_index)
        .is_some_and(|token| token.kind == TokenKind::Expr)
    {
        return None;
    }
    let names_open = start_index + 1;
    if !context
        .tokens
        .get(names_open)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        return None;
    }
    let names_close = source_destructuring_delimited_end(context.tokens, names_open)?;
    let names = source_destructuring_name_list(context.tokens, names_open + 1, names_close)?;
    let assign_index = names_close + 1;
    if !context
        .tokens
        .get(assign_index)
        .is_some_and(|token| token.kind == TokenKind::Assign)
    {
        return None;
    }
    let values_open = assign_index + 1;
    if !context
        .tokens
        .get(values_open)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        return None;
    }
    let values_close = source_destructuring_delimited_end(context.tokens, values_open)?;
    let semicolon_index = end_index.checked_sub(1)?;
    if values_close + 1 != semicolon_index
        || !context
            .tokens
            .get(semicolon_index)
            .is_some_and(|token| token.kind == TokenKind::Semicolon)
    {
        return None;
    }
    let expressions = source_destructuring_expression_list(context, values_open + 1, values_close)?;
    (names.len() == expressions.len()).then(|| names.into_iter().zip(expressions).collect())
}

fn source_destructuring_delimited_end(tokens: &[Token], open_index: usize) -> Option<usize> {
    let open = tokens.get(open_index)?;
    let close_kind = match open.kind {
        TokenKind::LParen => TokenKind::RParen,
        TokenKind::LBracket => TokenKind::RBracket,
        TokenKind::LBrace => TokenKind::RBrace,
        _ => return None,
    };
    let mut depth = 1_usize;
    let mut cursor = open_index + 1;
    while let Some(token) = tokens.get(cursor) {
        if token.kind == open.kind {
            depth = depth.checked_add(1)?;
        } else if token.kind == close_kind {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(cursor);
            }
        }
        cursor += 1;
    }
    None
}

fn source_destructuring_name_list(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
) -> Option<Vec<String>> {
    let mut names = Vec::new();
    let mut expect_name = true;
    let mut cursor = start_index;
    while cursor < end_index {
        let token = tokens.get(cursor)?;
        if expect_name {
            if token.kind != TokenKind::Identifier {
                return None;
            }
            names.push(token.lexeme.clone());
            expect_name = false;
        } else {
            if token.kind != TokenKind::Comma {
                return None;
            }
            expect_name = true;
        }
        cursor += 1;
    }
    (!names.is_empty() && !expect_name).then_some(names)
}

fn source_destructuring_expression_list(
    context: &SourceTemplateLoweringContext<'_>,
    start_index: usize,
    end_index: usize,
) -> Option<Vec<Expression>> {
    if start_index >= end_index {
        return Some(Vec::new());
    }

    let mut expressions = Vec::new();
    let mut segment_start = start_index;
    let mut stack = Vec::<TokenKind>::new();
    let mut cursor = start_index;

    while cursor < end_index {
        let token = context.tokens.get(cursor)?;
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if stack.pop()? != token.kind {
                    return None;
                }
            }
            TokenKind::Comma if stack.is_empty() => {
                if segment_start >= cursor {
                    return None;
                }
                expressions.push(source_destructuring_expression(
                    context,
                    segment_start,
                    cursor,
                )?);
                segment_start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }

    if segment_start >= end_index {
        return None;
    }
    expressions.push(source_destructuring_expression(
        context,
        segment_start,
        end_index,
    )?);
    Some(expressions)
}

fn source_destructuring_expression(
    context: &SourceTemplateLoweringContext<'_>,
    start_index: usize,
    end_index: usize,
) -> Option<Expression> {
    let (expression, next_index) = parse_expression_tokens(
        context.tokens,
        start_index,
        end_index,
        &context.module.source,
    )
    .ok()?;
    (next_index == end_index).then_some(expression)
}

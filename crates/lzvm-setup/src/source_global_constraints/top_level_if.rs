use lzvm_pil::{
    parse_expression_tokens, Expression, FixedFileTemplateValue, SourceProgram,
    SourceProgramModule, Token, TokenKind,
};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{evaluate_source_static_expression, static_value_truthy},
};

use super::{
    lower_top_level_global_constraints_range, unsupported_source_message,
    SourceGlobalConstraintBuilder, SourceTopLevelGlobalConstraintContext,
};

pub(super) fn lower_top_level_static_if_statement(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    index: usize,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let selection = select_top_level_static_if_body(
        context.program,
        context.module,
        context.tokens,
        index,
        &context.alias_scope.static_values,
    )?;
    let Some((body_start, body_end)) = selection.body else {
        return Ok(selection.next_index);
    };

    let checkpoint = constraints.checkpoint();
    match lower_top_level_global_constraints_range(context, body_start, body_end, constraints) {
        Ok(()) => Ok(selection.next_index),
        Err(error) => {
            constraints.rollback(checkpoint);
            Err(error)
        }
    }
}

struct TopLevelIfSelection {
    body: Option<(usize, usize)>,
    next_index: usize,
}

fn select_top_level_static_if_body(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    static_values: &std::collections::BTreeMap<String, FixedFileTemplateValue>,
) -> Result<TopLevelIfSelection, SourceKeyDirectoryMetadataError> {
    let mut cursor = index;
    let mut selected_body = None;
    loop {
        if !matches!(
            tokens.get(cursor).map(|token| token.kind),
            Some(TokenKind::If | TokenKind::ElseIf)
        ) {
            return Err(unsupported_source_message(
                "top-level if statement expected",
            ));
        }

        let (condition, body_start, body_end) = parse_top_level_if_branch(module, tokens, cursor)?;
        if selected_body.is_none() {
            let Some(condition_value) =
                evaluate_source_static_expression(program, &condition, static_values)
            else {
                return Err(unsupported_source_message(
                    "top-level if condition must be static",
                ));
            };
            if static_value_truthy(&condition_value) {
                selected_body = Some((body_start, body_end));
            }
        }

        cursor = body_end
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("top-level if body overflow"))?;
        match tokens.get(cursor).map(|token| token.kind) {
            Some(TokenKind::ElseIf) => {}
            Some(TokenKind::Else)
                if tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::If) =>
            {
                cursor += 1;
            }
            Some(TokenKind::Else) => {
                let body_open = cursor
                    .checked_add(1)
                    .ok_or_else(|| unsupported_source_message("top-level else body overflow"))?;
                if tokens.get(body_open).map(|token| token.kind) != Some(TokenKind::LBrace) {
                    return Err(unsupported_source_message(
                        "top-level else body must be braced",
                    ));
                }
                let body_close = matching_delimiter(tokens, body_open, TokenKind::RBrace)?;
                if selected_body.is_none() {
                    selected_body = Some((body_open + 1, body_close));
                }
                cursor = body_close
                    .checked_add(1)
                    .ok_or_else(|| unsupported_source_message("top-level else body overflow"))?;
                break;
            }
            _ => break,
        }
    }

    Ok(TopLevelIfSelection {
        body: selected_body,
        next_index: cursor,
    })
}

fn parse_top_level_if_branch(
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
) -> Result<(Expression, usize, usize), SourceKeyDirectoryMetadataError> {
    let open_index = index
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("top-level if condition overflow"))?;
    if tokens.get(open_index).map(|token| token.kind) != Some(TokenKind::LParen) {
        return Err(unsupported_source_message(
            "top-level if condition must be parenthesized",
        ));
    }
    let close_index = matching_delimiter(tokens, open_index, TokenKind::RParen)?;
    let condition = parse_expression_range(module, tokens, (open_index + 1, close_index))?;
    let body_open = close_index
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("top-level if body overflow"))?;
    if tokens.get(body_open).map(|token| token.kind) != Some(TokenKind::LBrace) {
        return Err(unsupported_source_message(
            "top-level if body must be braced",
        ));
    }
    let body_close = matching_delimiter(tokens, body_open, TokenKind::RBrace)?;
    Ok((condition, body_open + 1, body_close))
}

fn parse_expression_range(
    module: &SourceProgramModule,
    tokens: &[Token],
    range: (usize, usize),
) -> Result<Expression, SourceKeyDirectoryMetadataError> {
    let (expression, consumed) = parse_expression_tokens(tokens, range.0, range.1, &module.source)?;
    if consumed != range.1 {
        return Err(unsupported_source_message(
            "top-level if condition has unsupported trailing tokens",
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
        .ok_or_else(|| unsupported_source_message("top-level if delimiter missing"))?;
    let mut depth = 0_usize;
    for (cursor, token) in tokens.iter().enumerate().skip(index) {
        if token.kind == open_kind {
            depth = depth
                .checked_add(1)
                .ok_or_else(|| unsupported_source_message("top-level if nesting overflow"))?;
        } else if token.kind == close_kind {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| unsupported_source_message("top-level if body underflow"))?;
            if depth == 0 {
                return Ok(cursor);
            }
        }
    }
    Err(unsupported_source_message(
        "top-level if delimiter is not closed",
    ))
}

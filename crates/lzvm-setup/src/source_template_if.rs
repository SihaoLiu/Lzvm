use std::collections::BTreeMap;

use lzvm_pil::{
    lex_source, parse_expression, parse_function_body_statements, FixedFileTemplateValue,
    FunctionStatement, FunctionStatementKind, SourceProgram, SourceProgramModule, SourceSpan,
    Token, TokenKind,
};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{evaluate_source_static_expression, static_value_truthy},
};

pub(crate) fn source_static_if_body_statements(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Option<Vec<FunctionStatement>>, SourceKeyDirectoryMetadataError> {
    if statement.kind != FunctionStatementKind::If {
        return Ok(None);
    }
    let tokens = lex_source(&module.source.contents).map_err(|source| {
        SourceKeyDirectoryMetadataError::Lex {
            source_name: module.source_name.clone(),
            source,
        }
    })?;
    let Some(start) = tokens
        .iter()
        .position(|token| token.start == statement.start)
    else {
        return Ok(None);
    };
    let Some(end) = tokens
        .iter()
        .position(|token| token.end == statement.end)
        .map(|index| index + 1)
    else {
        return Ok(None);
    };
    let Some(selection) = source_static_if_body_span(program, module, &tokens, start, end, values)?
    else {
        return Ok(None);
    };
    let Some(body) = selection else {
        return Ok(Some(Vec::new()));
    };
    Ok(Some(parse_function_body_statements(
        &tokens,
        body,
        &module.source,
    )?))
}

fn source_static_if_body_span(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
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
    let Some(condition) = source_static_token_value(program, module, open + 1, close, values)
    else {
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
    source_static_else_body_span(program, module, tokens, body.after, end, values)
}

fn source_static_else_body_span(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Option<Option<SourceSpan>>, SourceKeyDirectoryMetadataError> {
    match tokens.get(index).map(|token| token.kind) {
        Some(TokenKind::ElseIf) => {
            source_static_if_body_span(program, module, tokens, index, end, values)
        }
        Some(TokenKind::Else)
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::If) =>
        {
            source_static_if_body_span(program, module, tokens, index + 1, end, values)
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

fn source_static_token_value(
    program: &SourceProgram,
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<FixedFileTemplateValue> {
    let (expression, consumed) = parse_expression(&module.source, start, end).ok()?;
    if consumed != end {
        return None;
    }
    evaluate_source_static_expression(program, &expression, values)
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

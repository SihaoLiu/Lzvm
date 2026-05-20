use super::declarations::{
    expected_close_error, missing_start, parse_delimited_span,
    parse_function_statement_declaration_at, parse_name_reference, parse_required_braced_span,
};
use super::expressions::{parse_expression_span_best_effort, parse_expression_tokens};
use super::types::{
    Expression, FunctionDeclaration, FunctionParameter, FunctionStatement,
    FunctionStatementDeclaration, FunctionStatementKind, FunctionVisibility, ParseError,
    SourceSpan,
};
use crate::{lex_source, SourceFile, Token, TokenKind};

pub fn parse_function_declarations(
    source: &SourceFile,
) -> Result<Vec<FunctionDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if let Some(parsed) = parse_function_at(&tokens, index, source)? {
            index = parsed.next_index;
            declarations.push(parsed.declaration);
        } else {
            index += 1;
        }
    }

    Ok(declarations)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFunction {
    declaration: FunctionDeclaration,
    next_index: usize,
}

fn parse_function_at(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<Option<ParsedFunction>, ParseError> {
    let Some(token) = tokens.get(index) else {
        return Ok(None);
    };
    let (visibility, function_index, start) = match token.kind {
        TokenKind::Function => (None, index, token.start),
        TokenKind::Public
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::Function) =>
        {
            (Some(FunctionVisibility::Public), index + 1, token.start)
        }
        TokenKind::Private
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::Function) =>
        {
            (Some(FunctionVisibility::Private), index + 1, token.start)
        }
        _ => return Ok(None),
    };

    let (name, after_name) = parse_name_reference(tokens, function_index + 1, source)?;
    let (params, after_params) = parse_delimited_span(tokens, after_name, source)?;
    let parameters = parse_function_parameters(tokens, after_name + 1, after_params - 1, source)?;
    let (return_type, body_index) = parse_function_return_type(tokens, after_params, source)?;
    let (body, next_index) = parse_required_braced_span(tokens, body_index, source)?;
    let statements = parse_function_body_statements(tokens, body, source)?;

    Ok(Some(ParsedFunction {
        declaration: FunctionDeclaration {
            name,
            visibility,
            params,
            parameters,
            return_type,
            body,
            statements,
            source_name: source.source_name.clone(),
            start,
            end: body.end,
        },
        next_index,
    }))
}

fn parse_function_return_type(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(Option<SourceSpan>, usize), ParseError> {
    let Some(token) = tokens.get(index) else {
        return Err(ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        });
    };
    if token.kind == TokenKind::LBrace {
        return Ok((None, index));
    }
    if token.kind != TokenKind::Colon {
        return Err(ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start: token.start,
        });
    }

    let start_token = tokens
        .get(index + 1)
        .ok_or_else(|| ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index + 1),
        })?;
    if start_token.kind == TokenKind::LBrace {
        return Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: start_token.start,
        });
    }

    let mut stack: Vec<TokenKind> = Vec::new();
    let mut cursor = index + 1;
    while let Some(token) = tokens.get(cursor) {
        if stack.is_empty() && token.kind == TokenKind::LBrace {
            let return_end = tokens
                .get(cursor.saturating_sub(1))
                .map_or(start_token.end, |token| token.end);
            return Ok((
                Some(SourceSpan {
                    start: start_token.start,
                    end: return_end,
                }),
                cursor,
            ));
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return Err(expected_close_error(token.kind, source, token.start));
                };
                if token.kind != expected {
                    return Err(expected_close_error(expected, source, token.start));
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    Err(ParseError::ExpectedCloseBrace {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, cursor),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFunctionStatement {
    statement: FunctionStatement,
    next_index: usize,
}

pub(crate) fn parse_function_body_statements(
    tokens: &[Token],
    body: SourceSpan,
    source: &SourceFile,
) -> Result<Vec<FunctionStatement>, ParseError> {
    let open_index = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LBrace && token.start == body.start)
        .ok_or_else(|| ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start: body.start,
        })?;
    let close_index = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::RBrace && token.end == body.end)
        .ok_or_else(|| ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start: body.start,
        })?;

    let mut statements = Vec::new();
    let mut cursor = open_index + 1;
    while cursor < close_index {
        if tokens[cursor].kind == TokenKind::Pragma {
            cursor += 1;
            continue;
        }
        let parsed = parse_function_statement(tokens, cursor, close_index, source)?;
        cursor = parsed.next_index;
        statements.push(parsed.statement);
    }
    Ok(statements)
}

fn parse_function_statement(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<ParsedFunctionStatement, ParseError> {
    let token = tokens
        .get(index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        })?;

    let (kind, span, next_index) = match token.kind {
        TokenKind::LBrace => {
            let (span, next_index) = parse_delimited_span(tokens, index, source)?;
            (FunctionStatementKind::Block, span, next_index)
        }
        TokenKind::If => {
            let (span, next_index) = parse_if_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::If, span, next_index)
        }
        TokenKind::ElseIf => {
            let (span, next_index) =
                parse_control_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::ElseIf, span, next_index)
        }
        TokenKind::Else => {
            let (span, next_index) = parse_else_tail_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Else, span, next_index)
        }
        TokenKind::For => {
            let (span, next_index) =
                parse_control_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::For, span, next_index)
        }
        TokenKind::While => {
            let (span, next_index) =
                parse_control_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::While, span, next_index)
        }
        TokenKind::Do => {
            let (span, next_index) =
                parse_control_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Do, span, next_index)
        }
        TokenKind::Switch => {
            let (span, next_index) =
                parse_control_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Switch, span, next_index)
        }
        TokenKind::Return => {
            let (span, next_index) =
                parse_semicolon_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Return, span, next_index)
        }
        TokenKind::Break => {
            let (span, next_index) =
                parse_semicolon_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Break, span, next_index)
        }
        TokenKind::Continue => {
            let (span, next_index) =
                parse_semicolon_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Continue, span, next_index)
        }
        TokenKind::Function | TokenKind::Public | TokenKind::Private
            if function_statement_function_start(tokens, index) =>
        {
            let (span, next_index) =
                parse_nested_function_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Declaration, span, next_index)
        }
        TokenKind::AtIdentifier => {
            let (span, next_index) =
                parse_at_identifier_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Expression, span, next_index)
        }
        TokenKind::Container => {
            let (span, next_index) =
                parse_container_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Declaration, span, next_index)
        }
        kind if function_statement_declaration_start(kind) => {
            let (span, next_index) =
                parse_semicolon_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Declaration, span, next_index)
        }
        _ => {
            let (span, next_index) =
                parse_semicolon_statement_span(tokens, index, limit_index, source)?;
            (FunctionStatementKind::Expression, span, next_index)
        }
    };
    let header = function_statement_header_span(tokens, index, next_index, kind, source)?;
    let header_expression = function_statement_expression(tokens, header.as_ref(), kind, source)?;
    let body = function_statement_body_span(tokens, index, next_index, kind, span, source)?;
    let value = function_statement_value_span(tokens, index, next_index, kind);
    let value_expression = function_statement_expression(tokens, value.as_ref(), kind, source)?;
    let declaration = function_statement_declaration(tokens, index, next_index, kind, source);
    let header_declaration =
        function_statement_header_declaration(tokens, header.as_ref(), kind, source);

    Ok(ParsedFunctionStatement {
        statement: FunctionStatement {
            kind,
            declaration,
            header_declaration,
            header,
            body,
            value,
            header_expression,
            value_expression,
            source_name: source.source_name.clone(),
            start: span.start,
            end: span.end,
        },
        next_index,
    })
}

fn parse_if_statement_span(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let (mut span, mut cursor) = parse_control_statement_span(tokens, index, limit_index, source)?;
    while cursor < limit_index {
        let Some(token) = tokens.get(cursor) else {
            break;
        };
        match token.kind {
            TokenKind::ElseIf => {
                let (tail, next_index) =
                    parse_control_statement_span(tokens, cursor, limit_index, source)?;
                span.end = tail.end;
                cursor = next_index;
            }
            TokenKind::Else => {
                let (tail, next_index) = parse_else_tail_span(tokens, cursor, limit_index, source)?;
                span.end = tail.end;
                cursor = next_index;
            }
            _ => break,
        }
    }
    Ok((span, cursor))
}

fn parse_else_tail_span(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let else_token = tokens
        .get(index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        })?;
    let next_index = index + 1;
    let next = tokens
        .get(next_index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, next_index),
        })?;

    if next.kind == TokenKind::If {
        let (if_span, cursor) = parse_if_statement_span(tokens, next_index, limit_index, source)?;
        return Ok((
            SourceSpan {
                start: else_token.start,
                end: if_span.end,
            },
            cursor,
        ));
    }
    if next.kind == TokenKind::LBrace {
        let (body, cursor) = parse_delimited_span(tokens, next_index, source)?;
        return Ok((
            SourceSpan {
                start: else_token.start,
                end: body.end,
            },
            cursor,
        ));
    }

    let (statement, cursor) =
        parse_semicolon_statement_span(tokens, next_index, limit_index, source)?;
    Ok((
        SourceSpan {
            start: else_token.start,
            end: statement.end,
        },
        cursor,
    ))
}

fn parse_control_statement_span(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let start = tokens
        .get(index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        })?
        .start;
    let mut stack: Vec<TokenKind> = Vec::new();
    let mut cursor = index;

    while cursor < limit_index {
        let token = &tokens[cursor];
        if stack.is_empty() && token.kind == TokenKind::LBrace {
            let (body, next_index) = parse_delimited_span(tokens, cursor, source)?;
            return Ok((
                SourceSpan {
                    start,
                    end: body.end,
                },
                next_index,
            ));
        }
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Ok((
                SourceSpan {
                    start,
                    end: token.end,
                },
                cursor + 1,
            ));
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return Err(expected_close_error(token.kind, source, token.start));
                };
                if token.kind != expected {
                    return Err(expected_close_error(expected, source, token.start));
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    Err(ParseError::ExpectedTerminator {
        source_name: source.source_name.clone(),
        start: tokens
            .get(limit_index)
            .map_or_else(|| missing_start(tokens, limit_index), |token| token.start),
    })
}

fn parse_semicolon_statement_span(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let start = tokens
        .get(index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        })?
        .start;
    let mut stack: Vec<TokenKind> = Vec::new();
    let mut cursor = index;

    while cursor < limit_index {
        let token = &tokens[cursor];
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Ok((
                SourceSpan {
                    start,
                    end: token.end,
                },
                cursor + 1,
            ));
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return Err(expected_close_error(token.kind, source, token.start));
                };
                if token.kind != expected {
                    return Err(expected_close_error(expected, source, token.start));
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    Err(ParseError::ExpectedTerminator {
        source_name: source.source_name.clone(),
        start: tokens
            .get(limit_index)
            .map_or_else(|| missing_start(tokens, limit_index), |token| token.start),
    })
}

fn parse_container_statement_span(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let start = tokens
        .get(index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        })?
        .start;
    let cursor = parse_container_statement_header(tokens, index, source)?;
    let terminator = tokens
        .get(cursor)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, cursor),
        })?;

    match terminator.kind {
        TokenKind::Semicolon => Ok((
            SourceSpan {
                start,
                end: terminator.end,
            },
            cursor + 1,
        )),
        TokenKind::LBrace => {
            let (body, next_index) = parse_delimited_span(tokens, cursor, source)?;
            if next_index > limit_index {
                return Err(ParseError::ExpectedCloseBrace {
                    source_name: source.source_name.clone(),
                    start: body.start,
                });
            }
            Ok((
                SourceSpan {
                    start,
                    end: body.end,
                },
                next_index,
            ))
        }
        _ => Err(ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: terminator.start,
        }),
    }
}

fn parse_container_statement_header(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<usize, ParseError> {
    let (_, mut cursor) = parse_name_reference(tokens, index + 1, source)?;
    if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Alias)
    {
        let alias = tokens
            .get(cursor + 1)
            .ok_or_else(|| ParseError::ExpectedAlias {
                source_name: source.source_name.clone(),
                start: missing_start(tokens, cursor + 1),
            })?;
        if !matches!(
            alias.kind,
            TokenKind::Identifier | TokenKind::StringLiteral | TokenKind::TemplateLiteral
        ) {
            return Err(ParseError::ExpectedAlias {
                source_name: source.source_name.clone(),
                start: alias.start,
            });
        }
        cursor += 2;
    }

    Ok(cursor)
}

fn parse_at_identifier_statement_span(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let start = tokens
        .get(index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        })?
        .start;
    if !tokens
        .get(index + 1)
        .is_some_and(|token| token.kind == TokenKind::LBrace)
    {
        return parse_semicolon_statement_span(tokens, index, limit_index, source);
    }

    let (payload, mut next_index) = parse_delimited_span(tokens, index + 1, source)?;
    if next_index > limit_index {
        return Err(ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start: payload.start,
        });
    }

    let mut end = payload.end;
    if next_index < limit_index
        && tokens
            .get(next_index)
            .is_some_and(|token| token.kind == TokenKind::Semicolon)
    {
        end = tokens[next_index].end;
        next_index += 1;
    }

    Ok((SourceSpan { start, end }, next_index))
}

fn parse_nested_function_statement_span(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let parsed =
        parse_function_at(tokens, index, source)?.ok_or_else(|| ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        })?;
    if parsed.next_index > limit_index {
        return Err(ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start: parsed.declaration.body.start,
        });
    }

    Ok((
        SourceSpan {
            start: parsed.declaration.start,
            end: parsed.declaration.end,
        },
        parsed.next_index,
    ))
}

fn function_statement_header_span(
    tokens: &[Token],
    index: usize,
    next_index: usize,
    kind: FunctionStatementKind,
    source: &SourceFile,
) -> Result<Option<SourceSpan>, ParseError> {
    if !function_statement_has_header(kind) {
        return Ok(None);
    }

    let mut cursor = index + 1;
    while cursor < next_index {
        let Some(token) = tokens.get(cursor) else {
            break;
        };
        match token.kind {
            TokenKind::LParen => {
                let (span, _) = parse_delimited_span(tokens, cursor, source)?;
                return Ok(Some(span));
            }
            TokenKind::LBrace | TokenKind::Semicolon => break,
            _ => cursor += 1,
        }
    }
    Ok(None)
}

fn function_statement_body_span(
    tokens: &[Token],
    index: usize,
    next_index: usize,
    kind: FunctionStatementKind,
    span: SourceSpan,
    source: &SourceFile,
) -> Result<Option<SourceSpan>, ParseError> {
    if kind == FunctionStatementKind::Block {
        return Ok(Some(span));
    }
    if kind == FunctionStatementKind::Declaration
        && function_statement_function_start(tokens, index)
    {
        return function_statement_nested_function_body_span(tokens, index, next_index, source);
    }
    if kind == FunctionStatementKind::Declaration
        && tokens
            .get(index)
            .is_some_and(|token| token.kind == TokenKind::Container)
    {
        return function_statement_container_body_span(tokens, index, next_index, source);
    }
    if !function_statement_has_body(kind) {
        return Ok(None);
    }

    let mut cursor = index + 1;
    while cursor < next_index {
        let Some(token) = tokens.get(cursor) else {
            break;
        };
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket => {
                let (_, next) = parse_delimited_span(tokens, cursor, source)?;
                cursor = next;
            }
            TokenKind::LBrace => {
                let (body, _) = parse_delimited_span(tokens, cursor, source)?;
                return Ok(Some(body));
            }
            _ => cursor += 1,
        }
    }
    Ok(None)
}

fn function_statement_nested_function_body_span(
    tokens: &[Token],
    index: usize,
    next_index: usize,
    source: &SourceFile,
) -> Result<Option<SourceSpan>, ParseError> {
    let Some(parsed) = parse_function_at(tokens, index, source)? else {
        return Ok(None);
    };
    if parsed.next_index == next_index {
        Ok(Some(parsed.declaration.body))
    } else {
        Ok(None)
    }
}

fn function_statement_container_body_span(
    tokens: &[Token],
    index: usize,
    next_index: usize,
    source: &SourceFile,
) -> Result<Option<SourceSpan>, ParseError> {
    let cursor = parse_container_statement_header(tokens, index, source)?;
    if cursor >= next_index {
        return Ok(None);
    }
    if !tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::LBrace)
    {
        return Ok(None);
    }

    let (body, body_next_index) = parse_delimited_span(tokens, cursor, source)?;
    if body_next_index == next_index {
        Ok(Some(body))
    } else {
        Ok(None)
    }
}

fn function_statement_value_span(
    tokens: &[Token],
    index: usize,
    next_index: usize,
    kind: FunctionStatementKind,
) -> Option<SourceSpan> {
    if !function_statement_has_value(kind) {
        return None;
    }
    let semicolon_index = next_index.checked_sub(1)?;
    if tokens.get(semicolon_index)?.kind != TokenKind::Semicolon {
        return None;
    }
    let value_start_index = if kind == FunctionStatementKind::Return {
        index + 1
    } else {
        index
    };
    if value_start_index >= semicolon_index {
        return None;
    }
    Some(SourceSpan {
        start: tokens.get(value_start_index)?.start,
        end: tokens.get(semicolon_index.checked_sub(1)?)?.end,
    })
}

fn function_statement_expression(
    tokens: &[Token],
    span: Option<&SourceSpan>,
    kind: FunctionStatementKind,
    source: &SourceFile,
) -> Result<Option<Expression>, ParseError> {
    let Some(span) = span else {
        return Ok(None);
    };
    if !function_statement_expression_kind(kind) {
        return Ok(None);
    }

    let Some((start_index, end_index)) = token_span_bounds(tokens, span) else {
        return Ok(None);
    };
    if !function_statement_expression_supported(tokens, start_index, end_index) {
        return Ok(None);
    }

    match parse_expression_tokens(tokens, start_index, end_index, source) {
        Ok((expression, next_index)) if next_index == end_index => Ok(Some(expression)),
        Ok(_) | Err(_) => Ok(None),
    }
}

fn function_statement_expression_kind(kind: FunctionStatementKind) -> bool {
    matches!(
        kind,
        FunctionStatementKind::If
            | FunctionStatementKind::ElseIf
            | FunctionStatementKind::While
            | FunctionStatementKind::Switch
            | FunctionStatementKind::Return
            | FunctionStatementKind::Expression
    )
}

fn token_span_bounds(tokens: &[Token], span: &SourceSpan) -> Option<(usize, usize)> {
    let start_index = tokens.iter().position(|token| token.start == span.start)?;
    let end_index = tokens
        .iter()
        .position(|token| token.end == span.end)?
        .checked_add(1)?;
    Some((start_index, end_index))
}

fn function_statement_expression_supported(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
) -> bool {
    let Some(start_token) = tokens.get(start_index) else {
        return false;
    };
    if matches!(
        start_token.kind,
        TokenKind::LBracket
            | TokenKind::Int
            | TokenKind::Fe
            | TokenKind::Expr
            | TokenKind::String
            | TokenKind::Col
            | TokenKind::Virtual
    ) {
        return false;
    }

    !tokens[start_index..end_index].iter().any(|token| {
        matches!(
            token.kind,
            TokenKind::Question | TokenKind::In | TokenKind::Is
        )
    })
}

fn function_statement_has_header(kind: FunctionStatementKind) -> bool {
    matches!(
        kind,
        FunctionStatementKind::If
            | FunctionStatementKind::ElseIf
            | FunctionStatementKind::For
            | FunctionStatementKind::While
            | FunctionStatementKind::Switch
    )
}

fn function_statement_has_body(kind: FunctionStatementKind) -> bool {
    matches!(
        kind,
        FunctionStatementKind::If
            | FunctionStatementKind::ElseIf
            | FunctionStatementKind::Else
            | FunctionStatementKind::For
            | FunctionStatementKind::While
            | FunctionStatementKind::Do
            | FunctionStatementKind::Switch
    )
}

fn function_statement_has_value(kind: FunctionStatementKind) -> bool {
    matches!(
        kind,
        FunctionStatementKind::Return
            | FunctionStatementKind::Declaration
            | FunctionStatementKind::Expression
    )
}

fn function_statement_declaration_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Const
            | TokenKind::Constant
            | TokenKind::Int
            | TokenKind::Fe
            | TokenKind::Expr
            | TokenKind::String
            | TokenKind::Col
    )
}

fn function_statement_function_start(tokens: &[Token], index: usize) -> bool {
    match tokens.get(index).map(|token| token.kind) {
        Some(TokenKind::Function) => true,
        Some(TokenKind::Public | TokenKind::Private) => tokens
            .get(index + 1)
            .is_some_and(|token| token.kind == TokenKind::Function),
        _ => false,
    }
}

fn function_statement_declaration(
    tokens: &[Token],
    index: usize,
    next_index: usize,
    kind: FunctionStatementKind,
    source: &SourceFile,
) -> Option<FunctionStatementDeclaration> {
    if kind != FunctionStatementKind::Declaration {
        return None;
    }

    let parsed = parse_function_statement_declaration_at(tokens, index, source)
        .ok()
        .flatten()?;
    if parsed.next_index != next_index {
        return None;
    }

    Some(parsed.declaration)
}

fn function_statement_header_declaration(
    tokens: &[Token],
    header: Option<&SourceSpan>,
    kind: FunctionStatementKind,
    source: &SourceFile,
) -> Option<FunctionStatementDeclaration> {
    if kind != FunctionStatementKind::For {
        return None;
    }

    let header = header?;
    let (start_index, end_index) = token_span_bounds(tokens, header)?;
    let init_index = start_index.checked_add(1)?;
    if init_index >= end_index {
        return None;
    }
    let semicolon_index = first_for_header_semicolon(tokens, init_index, end_index)?;
    let parsed = parse_function_statement_declaration_at(tokens, init_index, source)
        .ok()
        .flatten()?;
    if parsed.next_index != semicolon_index + 1 {
        return None;
    }

    Some(parsed.declaration)
}

fn first_for_header_semicolon(
    tokens: &[Token],
    mut cursor: usize,
    limit_index: usize,
) -> Option<usize> {
    let mut stack: Vec<TokenKind> = Vec::new();

    while cursor < limit_index {
        let token = tokens.get(cursor)?;
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Some(cursor);
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let expected = stack.pop()?;
                if expected != token.kind {
                    return None;
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFunctionParameter {
    parameter: FunctionParameter,
    next_index: usize,
}

pub(crate) fn parse_function_parameters(
    tokens: &[Token],
    mut cursor: usize,
    close_index: usize,
    source: &SourceFile,
) -> Result<Vec<FunctionParameter>, ParseError> {
    if cursor == close_index {
        return Ok(Vec::new());
    }

    let mut parameters = Vec::new();
    loop {
        let parsed = parse_function_parameter(tokens, cursor, source)?;
        cursor = parsed.next_index;
        parameters.push(parsed.parameter);

        if cursor == close_index {
            return Ok(parameters);
        }

        let comma = tokens
            .get(cursor)
            .ok_or_else(|| ParseError::ExpectedCloseParen {
                source_name: source.source_name.clone(),
                start: missing_start(tokens, cursor),
            })?;
        if comma.kind != TokenKind::Comma {
            return Err(ParseError::ExpectedCloseParen {
                source_name: source.source_name.clone(),
                start: comma.start,
            });
        }
        cursor += 1;
        if cursor == close_index {
            return Err(ParseError::ExpectedName {
                source_name: source.source_name.clone(),
                start: missing_start(tokens, cursor),
            });
        }
    }
}

fn parse_function_parameter(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<ParsedFunctionParameter, ParseError> {
    let Some(token) = tokens.get(index) else {
        return Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        });
    };

    let mut cursor = index;
    let mut is_const = false;
    if token.kind == TokenKind::Const {
        is_const = true;
        cursor += 1;
    }

    let type_token = tokens.get(cursor).ok_or_else(|| ParseError::ExpectedName {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, cursor),
    })?;
    if !function_parameter_type_start(type_token.kind) {
        return Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: type_token.start,
        });
    }
    let type_name = type_token.lexeme.clone();
    cursor += 1;

    let mut by_reference = false;
    if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Amp)
    {
        by_reference = true;
        cursor += 1;
    }

    let name_token = tokens.get(cursor).ok_or_else(|| ParseError::ExpectedName {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, cursor),
    })?;
    if name_token.kind != TokenKind::Identifier {
        return Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: name_token.start,
        });
    }
    let name = name_token.lexeme.clone();
    cursor += 1;

    let mut array_dims = Vec::new();
    while tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        let (span, next_index) = parse_delimited_span(tokens, cursor, source)?;
        array_dims.push(span);
        cursor = next_index;
    }

    let mut default_expression = None;
    let default_value = if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Assign)
    {
        let (span, next_index) =
            parse_function_parameter_default_value(tokens, cursor + 1, source)?;
        default_expression = parse_expression_span_best_effort(tokens, span, source);
        cursor = next_index;
        Some(span)
    } else {
        None
    };

    let end = tokens
        .get(cursor.saturating_sub(1))
        .map_or(name_token.end, |token| token.end);

    Ok(ParsedFunctionParameter {
        parameter: FunctionParameter {
            is_const,
            by_reference,
            type_name,
            name,
            array_dims,
            default_value,
            default_expression,
            source_name: source.source_name.clone(),
            start: token.start,
            end,
        },
        next_index: cursor,
    })
}

fn parse_function_parameter_default_value(
    tokens: &[Token],
    start_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let start_token = tokens
        .get(start_index)
        .ok_or_else(|| ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, start_index),
        })?;
    if start_token.kind == TokenKind::Comma || start_token.kind == TokenKind::RParen {
        return Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: start_token.start,
        });
    }

    let mut stack: Vec<TokenKind> = Vec::new();
    let mut cursor = start_index;
    while let Some(token) = tokens.get(cursor) {
        if stack.is_empty() && (token.kind == TokenKind::Comma || token.kind == TokenKind::RParen) {
            return Ok((
                SourceSpan {
                    start: start_token.start,
                    end: tokens
                        .get(cursor.saturating_sub(1))
                        .map_or(start_token.end, |token| token.end),
                },
                cursor,
            ));
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return Err(expected_close_error(token.kind, source, token.start));
                };
                if token.kind != expected {
                    return Err(expected_close_error(expected, source, token.start));
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    Err(ParseError::ExpectedCloseParen {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, cursor),
    })
}

fn function_parameter_type_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Int
            | TokenKind::Fe
            | TokenKind::Expr
            | TokenKind::String
            | TokenKind::Identifier
            | TokenKind::TemplateLiteral
    )
}

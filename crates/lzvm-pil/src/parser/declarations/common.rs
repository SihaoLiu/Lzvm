use super::*;

pub(crate) fn parse_alias_identifier(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(String, usize), ParseError> {
    let Some(alias_token) = tokens.get(index) else {
        return Err(ParseError::ExpectedAlias {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        });
    };
    if !matches!(
        alias_token.kind,
        TokenKind::Identifier | TokenKind::StringLiteral | TokenKind::TemplateLiteral
    ) {
        return Err(ParseError::ExpectedAlias {
            source_name: source.source_name.clone(),
            start: alias_token.start,
        });
    }
    Ok((alias_token.lexeme.clone(), index + 1))
}

pub(crate) fn parse_name_reference(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(String, usize), ParseError> {
    let token = tokens.get(index).ok_or_else(|| ParseError::ExpectedName {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, index),
    })?;

    match token.kind {
        TokenKind::Air | TokenKind::AirGroup | TokenKind::Proof => {
            let dot_index = index + 1;
            let dot = tokens
                .get(dot_index)
                .ok_or_else(|| ParseError::ExpectedName {
                    source_name: source.source_name.clone(),
                    start: missing_start(tokens, dot_index),
                })?;
            if dot.kind != TokenKind::Dot {
                return Err(ParseError::ExpectedName {
                    source_name: source.source_name.clone(),
                    start: dot.start,
                });
            }
            let (tail, next) = parse_name_tail(tokens, dot_index + 1, source)?;
            Ok((format!("{}.{}", token.lexeme, tail), next))
        }
        TokenKind::Identifier => {
            let mut name = token.lexeme.clone();
            let mut next = index + 1;
            if tokens
                .get(next)
                .is_some_and(|token| token.kind == TokenKind::Dot)
            {
                let (tail, cursor) = parse_name_tail(tokens, next + 1, source)?;
                name.push('.');
                name.push_str(&tail);
                next = cursor;
            }
            Ok((name, next))
        }
        _ => Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: token.start,
        }),
    }
}

fn parse_name_tail(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(String, usize), ParseError> {
    let mut cursor = index;
    let mut segments = Vec::new();

    loop {
        let token = tokens.get(cursor).ok_or_else(|| ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, cursor),
        })?;
        match token.kind {
            TokenKind::Identifier | TokenKind::TemplateLiteral => {
                segments.push(token.lexeme.clone());
            }
            _ => {
                return Err(ParseError::ExpectedName {
                    source_name: source.source_name.clone(),
                    start: token.start,
                });
            }
        }
        cursor += 1;
        if tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::Dot)
        {
            cursor += 1;
            continue;
        }
        break;
    }

    Ok((segments.join("."), cursor))
}

pub(crate) fn parse_delimited_span(
    tokens: &[Token],
    open_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let open = tokens
        .get(open_index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, open_index),
        })?;
    let (expected_close, close_error_start) = match open.kind {
        TokenKind::LParen => (TokenKind::RParen, open.start),
        TokenKind::LBracket => (TokenKind::RBracket, open.start),
        TokenKind::LBrace => (TokenKind::RBrace, open.start),
        _ => {
            return Err(ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: open.start,
            });
        }
    };

    let mut stack = vec![expected_close];
    for (index, token) in tokens.iter().enumerate().skip(open_index + 1) {
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
                if stack.is_empty() {
                    return Ok((
                        SourceSpan {
                            start: open.start,
                            end: token.end,
                        },
                        index + 1,
                    ));
                }
            }
            _ => {}
        }
    }

    Err(expected_close_error(
        expected_close,
        source,
        close_error_start,
    ))
}

pub(crate) fn parse_expression_span_until_terminator(
    tokens: &[Token],
    start_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let start = tokens
        .get(start_index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, start_index),
        })?
        .start;
    let mut stack: Vec<TokenKind> = Vec::new();
    let mut cursor = start_index;

    while let Some(token) = tokens.get(cursor) {
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Ok((
                SourceSpan {
                    start,
                    end: token.start,
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

    Err(ParseError::ExpectedTerminator {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, cursor),
    })
}

pub(crate) fn expected_close_error(
    kind: TokenKind,
    source: &SourceFile,
    start: usize,
) -> ParseError {
    match kind {
        TokenKind::RParen => ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start,
        },
        TokenKind::RBracket => ParseError::ExpectedCloseBracket {
            source_name: source.source_name.clone(),
            start,
        },
        TokenKind::RBrace => ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start,
        },
        _ => ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start,
        },
    }
}

pub(crate) fn missing_start(tokens: &[Token], index: usize) -> usize {
    tokens.get(index).map_or_else(
        || tokens.last().map_or(0, |token| token.end),
        |token| token.start,
    )
}

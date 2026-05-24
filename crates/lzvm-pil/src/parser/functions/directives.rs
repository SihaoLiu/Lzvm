use super::super::declarations::{expected_close_error, include_header, missing_start};
use super::super::types::{ParseError, SourceSpan};
use crate::{SourceFile, Token, TokenKind};

pub(super) fn source_directive_statement_start(tokens: &[Token], index: usize) -> Option<usize> {
    match tokens.get(index)?.kind {
        TokenKind::Include | TokenKind::Require | TokenKind::Use => Some(index),
        TokenKind::Public | TokenKind::Private
            if tokens.get(index + 1).is_some_and(|token| {
                matches!(
                    token.kind,
                    TokenKind::Include | TokenKind::Require | TokenKind::Use
                )
            }) =>
        {
            Some(index + 1)
        }
        _ => None,
    }
}

pub(super) fn parse_source_directive_statement_span(
    tokens: &[Token],
    index: usize,
    limit_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    if let Some(header) = include_header(tokens, index) {
        return parse_source_include_statement_span(tokens, header.directive_index, index, source);
    }
    parse_source_line_statement_span(tokens, index, limit_index, source)
}

fn parse_source_include_statement_span(
    tokens: &[Token],
    directive_index: usize,
    start_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let path_index = directive_index + 1;
    let path_token = tokens
        .get(path_index)
        .ok_or_else(|| ParseError::ExpectedPath {
            source_name: source.source_name.clone(),
            start: tokens[directive_index].end,
        })?;
    if !matches!(
        path_token.kind,
        TokenKind::StringLiteral | TokenKind::TemplateLiteral
    ) {
        return Err(ParseError::ExpectedPath {
            source_name: source.source_name.clone(),
            start: path_token.start,
        });
    }

    let terminator_index = path_index + 1;
    match tokens.get(terminator_index) {
        Some(terminator) if terminator.kind == TokenKind::Semicolon => Ok((
            SourceSpan {
                start: tokens[start_index].start,
                end: terminator.end,
            },
            terminator_index + 1,
        )),
        Some(terminator)
            if has_line_break_between(&source.contents, path_token.end, terminator.start) =>
        {
            Ok((
                SourceSpan {
                    start: tokens[start_index].start,
                    end: path_token.end,
                },
                terminator_index,
            ))
        }
        Some(terminator) => Err(ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: terminator.start,
        }),
        None => Ok((
            SourceSpan {
                start: tokens[start_index].start,
                end: path_token.end,
            },
            tokens.len(),
        )),
    }
}

fn parse_source_line_statement_span(
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
    let mut last_end = tokens[index].end;

    while cursor < limit_index {
        let token = &tokens[cursor];
        if stack.is_empty() {
            if token.kind == TokenKind::Semicolon {
                return Ok((
                    SourceSpan {
                        start,
                        end: token.end,
                    },
                    cursor + 1,
                ));
            }
            if cursor > index && has_line_break_between(&source.contents, last_end, token.start) {
                return Ok((
                    SourceSpan {
                        start,
                        end: last_end,
                    },
                    cursor,
                ));
            }
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
        last_end = token.end;
        cursor += 1;
    }

    Ok((
        SourceSpan {
            start,
            end: last_end,
        },
        limit_index,
    ))
}

fn has_line_break_between(source: &str, start: usize, end: usize) -> bool {
    source[start..end]
        .bytes()
        .any(|byte| matches!(byte, b'\n' | b'\r'))
}

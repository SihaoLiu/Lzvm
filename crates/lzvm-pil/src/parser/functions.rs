use super::declarations::{
    expected_close_error, missing_start, parse_delimited_span, parse_name_reference,
    parse_required_braced_span,
};
use super::types::{
    FunctionDeclaration, FunctionParameter, FunctionVisibility, ParseError, SourceSpan,
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

    Ok(Some(ParsedFunction {
        declaration: FunctionDeclaration {
            name,
            visibility,
            params,
            parameters,
            return_type,
            body,
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
struct ParsedFunctionParameter {
    parameter: FunctionParameter,
    next_index: usize,
}

fn parse_function_parameters(
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

    let default_value = if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Assign)
    {
        let (span, next_index) =
            parse_function_parameter_default_value(tokens, cursor + 1, source)?;
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

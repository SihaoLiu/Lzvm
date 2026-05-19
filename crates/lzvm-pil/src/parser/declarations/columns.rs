use super::super::expressions::{
    parse_expression_list_span_best_effort, parse_expression_range_best_effort,
    parse_expression_span_best_effort,
};
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedColumnDeclaration {
    pub(super) declaration: ColumnDeclaration,
    pub(super) next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ColumnHeader {
    kind: ColumnKind,
    commit: Option<String>,
    start: usize,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedNameReference {
    name: String,
    template: bool,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedColumnFeature {
    feature: ColumnFeature,
    next_index: usize,
}

pub(super) fn parse_column_declaration_at(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<Option<ParsedColumnDeclaration>, ParseError> {
    if tokens
        .get(index)
        .is_none_or(|token| token.kind != TokenKind::Col)
    {
        return Ok(None);
    }

    let Some(header) = parse_column_header(tokens, index) else {
        return Ok(None);
    };

    let mut cursor = header.next_index;
    let (features, next_cursor) = parse_column_features(tokens, cursor, source)?;
    cursor = next_cursor;
    let (items, initializer, next_index, end) =
        parse_column_body(tokens, cursor, header.kind, source)?;

    Ok(Some(ParsedColumnDeclaration {
        declaration: ColumnDeclaration {
            kind: header.kind,
            commit: header.commit,
            features,
            items,
            initializer,
            source_name: source.source_name.clone(),
            start: header.start,
            end,
        },
        next_index,
    }))
}

fn parse_column_header(tokens: &[Token], index: usize) -> Option<ColumnHeader> {
    let next = tokens.get(index + 1)?;
    let (kind, commit, next_index) = match next.kind {
        TokenKind::Witness => (ColumnKind::Witness, None, index + 2),
        TokenKind::Fixed => (ColumnKind::Fixed, None, index + 2),
        TokenKind::Identifier => (ColumnKind::Custom, Some(next.lexeme.clone()), index + 2),
        _ => return None,
    };
    Some(ColumnHeader {
        kind,
        commit,
        start: tokens[index].start,
        next_index,
    })
}

fn parse_column_features(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(Vec<ColumnFeature>, usize), ParseError> {
    let mut features = Vec::new();
    let mut cursor = index;

    while let Some(feature) = parse_column_feature(tokens, cursor, source)? {
        cursor = feature.next_index;
        features.push(feature.feature);
    }

    Ok((features, cursor))
}

fn parse_column_feature(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<Option<ParsedColumnFeature>, ParseError> {
    let Some(token) = tokens.get(index) else {
        return Ok(None);
    };
    let name = match token.kind {
        TokenKind::Identifier | TokenKind::Stage | TokenKind::Virtual => token.lexeme.clone(),
        _ => return Ok(None),
    };
    let Some(next) = tokens.get(index + 1) else {
        return Ok(None);
    };
    if next.kind != TokenKind::LParen {
        return Ok(None);
    }
    let (args, next_index) = parse_delimited_span(tokens, index + 1, source)?;
    let args_expressions = parse_expression_list_span_best_effort(tokens, args, source);
    Ok(Some(ParsedColumnFeature {
        feature: ColumnFeature {
            name,
            args,
            args_expressions,
        },
        next_index,
    }))
}

fn parse_column_body(
    tokens: &[Token],
    index: usize,
    kind: ColumnKind,
    source: &SourceFile,
) -> Result<(Vec<ColumnItem>, Option<ColumnInitializer>, usize, usize), ParseError> {
    match kind {
        ColumnKind::Witness | ColumnKind::Custom => {
            let (items, next_index, end) = parse_column_item_list(tokens, index, source)?;
            Ok((items, None, next_index, end))
        }
        ColumnKind::Fixed => {
            let (first_item, mut cursor) = parse_column_item(tokens, index, source)?;
            if tokens
                .get(cursor)
                .is_some_and(|token| token.kind == TokenKind::Assign)
            {
                let (initializer, next_index, end) =
                    parse_column_initializer(tokens, cursor + 1, source)?;
                Ok((vec![first_item], Some(initializer), next_index, end))
            } else {
                let mut items = vec![first_item];
                while tokens
                    .get(cursor)
                    .is_some_and(|token| token.kind == TokenKind::Comma)
                {
                    let (item, next) = parse_column_item(tokens, cursor + 1, source)?;
                    items.push(item);
                    cursor = next;
                }
                let terminator =
                    tokens
                        .get(cursor)
                        .ok_or_else(|| ParseError::ExpectedTerminator {
                            source_name: source.source_name.clone(),
                            start: missing_start(tokens, cursor),
                        })?;
                if terminator.kind != TokenKind::Semicolon {
                    return Err(ParseError::ExpectedTerminator {
                        source_name: source.source_name.clone(),
                        start: terminator.start,
                    });
                }
                Ok((items, None, cursor + 1, terminator.end))
            }
        }
    }
}

pub(super) fn parse_column_item_list(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(Vec<ColumnItem>, usize, usize), ParseError> {
    let (first_item, mut cursor) = parse_column_item(tokens, index, source)?;
    let mut items = vec![first_item];
    while tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Comma)
    {
        let (item, next) = parse_column_item(tokens, cursor + 1, source)?;
        items.push(item);
        cursor = next;
    }

    let terminator = tokens
        .get(cursor)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, cursor),
        })?;
    if terminator.kind != TokenKind::Semicolon {
        return Err(ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: terminator.start,
        });
    }

    Ok((items, cursor + 1, terminator.end))
}

pub(super) fn parse_column_item(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(ColumnItem, usize), ParseError> {
    let parsed = parse_column_name_reference(tokens, index, source)?;
    let mut array_dims = Vec::new();
    let mut array_dim_expressions = Vec::new();
    let mut cursor = parsed.next_index;

    while tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        let (span, next) = parse_delimited_span(tokens, cursor, source)?;
        let expression = if cursor + 1 < next.saturating_sub(1) {
            parse_expression_range_best_effort(tokens, cursor + 1, next - 1, source)
        } else {
            None
        };
        array_dims.push(span);
        array_dim_expressions.push(expression);
        cursor = next;
    }

    Ok((
        ColumnItem {
            name: parsed.name,
            template: parsed.template,
            array_dims,
            array_dim_expressions,
        },
        cursor,
    ))
}

fn parse_column_initializer(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(ColumnInitializer, usize, usize), ParseError> {
    let Some(token) = tokens.get(index) else {
        return Err(ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, index),
        });
    };
    if token.kind == TokenKind::LBracket {
        let (mut span, mut next_index) = parse_delimited_span(tokens, index, source)?;
        if let Some(token) = tokens.get(next_index) {
            if token.kind == TokenKind::Ellipsis {
                span.end = token.end;
                next_index += 1;
            }
        }
        let terminator = tokens
            .get(next_index)
            .ok_or_else(|| ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: missing_start(tokens, next_index),
            })?;
        if terminator.kind != TokenKind::Semicolon {
            return Err(ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: terminator.start,
            });
        }
        return Ok((
            ColumnInitializer {
                kind: ColumnInitializerKind::Sequence,
                span,
                expression: None,
            },
            next_index + 1,
            terminator.end,
        ));
    }

    let (span, next_index) = parse_expression_span_until_terminator(tokens, index, source)?;
    let expression = parse_expression_span_best_effort(tokens, span, source);
    let terminator = tokens
        .get(next_index)
        .ok_or_else(|| ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, next_index),
        })?;
    if terminator.kind != TokenKind::Semicolon {
        return Err(ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: terminator.start,
        });
    }

    Ok((
        ColumnInitializer {
            kind: ColumnInitializerKind::Expression,
            span,
            expression,
        },
        next_index + 1,
        terminator.end,
    ))
}

fn parse_column_name_reference(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<ParsedNameReference, ParseError> {
    let token = tokens.get(index).ok_or_else(|| ParseError::ExpectedName {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, index),
    })?;

    match token.kind {
        TokenKind::TemplateLiteral => Ok(ParsedNameReference {
            name: token.lexeme.clone(),
            template: true,
            next_index: index + 1,
        }),
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
            let (tail, template, next) = parse_name_tail_detail(tokens, dot_index + 1, source)?;
            Ok(ParsedNameReference {
                name: format!("{}.{}", token.lexeme, tail),
                template,
                next_index: next,
            })
        }
        TokenKind::Identifier => {
            let mut name = token.lexeme.clone();
            let mut next = index + 1;
            let mut template = false;
            if tokens
                .get(next)
                .is_some_and(|token| token.kind == TokenKind::Dot)
            {
                let (tail, tail_template, cursor) =
                    parse_name_tail_detail(tokens, next + 1, source)?;
                name.push('.');
                name.push_str(&tail);
                next = cursor;
                template = tail_template;
            }
            Ok(ParsedNameReference {
                name,
                template,
                next_index: next,
            })
        }
        _ => Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: token.start,
        }),
    }
}

fn parse_name_tail_detail(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(String, bool, usize), ParseError> {
    let mut cursor = index;
    let mut segments = Vec::new();
    let mut template = false;

    loop {
        let token = tokens.get(cursor).ok_or_else(|| ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, cursor),
        })?;
        match token.kind {
            TokenKind::Identifier => {
                segments.push(token.lexeme.clone());
            }
            TokenKind::TemplateLiteral => {
                segments.push(token.lexeme.clone());
                template = true;
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

    Ok((segments.join("."), template, cursor))
}

use super::expressions::{
    parse_expression_list_range_best_effort, parse_expression_range_best_effort,
    parse_expression_span_best_effort,
};
mod columns;
mod common;
mod named;

use columns::{parse_column_declaration_at, parse_column_item, parse_column_item_list};
pub(crate) use common::{
    expected_close_error, missing_start, parse_alias_identifier, parse_delimited_span,
    parse_expression_span_until_terminator, parse_name_reference,
};
pub(crate) use named::{include_header, parse_named_statement, parse_required_braced_span};
pub use named::{
    parse_air_group_declarations, parse_air_instance_declarations, parse_air_template_declarations,
    parse_container_declarations,
};

use super::*;

pub fn parse_column_declarations(
    source: &SourceFile,
) -> Result<Vec<ColumnDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let Some(parsed) = parse_column_declaration_at(&tokens, index, source)? else {
            index += 1;
            continue;
        };

        index = parsed.next_index;
        declarations.push(parsed.declaration);
    }

    Ok(declarations)
}

pub fn parse_constant_declarations(
    source: &SourceFile,
) -> Result<Vec<ConstantDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;
    let mut stack = Vec::new();

    while index < tokens.len() {
        if matches!(tokens[index].kind, TokenKind::Const | TokenKind::Constant)
            && declaration_statement_context(&stack)
        {
            let parsed = parse_constant_declaration_at(&tokens, index, source)?;
            index = parsed.next_index;
            declarations.push(parsed.declaration);
            continue;
        }

        update_declaration_scan_stack(&mut stack, tokens[index].kind);
        index += 1;
    }

    Ok(declarations)
}

pub fn parse_variable_declarations(
    source: &SourceFile,
) -> Result<Vec<VariableDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;
    let mut stack = Vec::new();

    while index < tokens.len() {
        if variable_statement_context(&tokens, index, &stack) {
            let parsed = parse_variable_declaration_at(&tokens, index, source)?;
            index = parsed.next_index;
            declarations.push(parsed.declaration);
            continue;
        }

        update_declaration_scan_stack(&mut stack, tokens[index].kind);
        index += 1;
    }

    Ok(declarations)
}

pub fn parse_value_declarations(source: &SourceFile) -> Result<Vec<ValueDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let Some(header) = parse_value_header(&tokens, index) else {
            index += 1;
            continue;
        };
        let (stage, item_start) = parse_optional_stage_definition(
            &tokens,
            header.next_index,
            header.default_stage,
            source,
        )?;
        let (items, next_index, end) = parse_column_item_list(&tokens, item_start, source)?;

        declarations.push(ValueDeclaration {
            kind: header.kind,
            stage,
            items,
            source_name: source.source_name.clone(),
            start: header.start,
            end,
        });
        index = next_index;
    }

    Ok(declarations)
}

pub fn parse_air_group_value_declarations(
    source: &SourceFile,
) -> Result<Vec<AirGroupValueDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::AirGroupValue
            || !tokens
                .get(index + 1)
                .is_some_and(|token| group_value_property_start(token.kind))
        {
            index += 1;
            continue;
        }

        let properties = parse_group_value_properties(&tokens, index + 1, source)?;
        let (items, next_index, end) =
            parse_column_item_list(&tokens, properties.next_index, source)?;

        declarations.push(AirGroupValueDeclaration {
            stage: properties.stage,
            default_value: properties.default_value,
            default_expression: properties.default_expression,
            aggregate_type: properties.aggregate_type,
            items,
            source_name: source.source_name.clone(),
            start: tokens[index].start,
            end,
        });
        index = next_index;
    }

    Ok(declarations)
}

pub fn parse_commit_declarations(
    source: &SourceFile,
) -> Result<Vec<CommitDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Commit
            || !tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::Stage)
        {
            index += 1;
            continue;
        }

        let (stage, after_stage) = parse_optional_stage_definition(&tokens, index + 1, 0, source)?;
        let (publics, after_publics) = parse_commit_public_reference(&tokens, after_stage, source)?;
        let (name, next_index) = parse_alias_identifier(&tokens, after_publics, source)?;
        let terminator = tokens
            .get(next_index)
            .ok_or_else(|| ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: missing_start(&tokens, next_index),
            })?;
        if terminator.kind != TokenKind::Semicolon {
            return Err(ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: terminator.start,
            });
        }

        declarations.push(CommitDeclaration {
            stage,
            publics,
            name,
            source_name: source.source_name.clone(),
            start: tokens[index].start,
            end: terminator.end,
        });
        index = next_index + 1;
    }

    Ok(declarations)
}

pub fn parse_public_declarations(
    source: &SourceFile,
) -> Result<Vec<PublicDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Public
            || !tokens
                .get(index + 1)
                .is_some_and(|token| public_declaration_start(token.kind))
        {
            index += 1;
            continue;
        }

        let (first_item, cursor) = parse_column_item(&tokens, index + 1, source)?;
        let (items, initializer, initializer_expression, next_index, end) = if tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::Assign)
        {
            let (span, next_index) =
                parse_expression_span_until_terminator(&tokens, cursor + 1, source)?;
            let expression = parse_expression_span_best_effort(&tokens, span, source);
            let terminator =
                tokens
                    .get(next_index)
                    .ok_or_else(|| ParseError::ExpectedTerminator {
                        source_name: source.source_name.clone(),
                        start: missing_start(&tokens, next_index),
                    })?;
            if terminator.kind != TokenKind::Semicolon {
                return Err(ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: terminator.start,
                });
            }
            (
                vec![first_item],
                Some(span),
                expression,
                next_index + 1,
                terminator.end,
            )
        } else {
            let mut items = vec![first_item];
            let mut cursor = cursor;
            while tokens
                .get(cursor)
                .is_some_and(|token| token.kind == TokenKind::Comma)
            {
                let (item, next) = parse_column_item(&tokens, cursor + 1, source)?;
                items.push(item);
                cursor = next;
            }
            let terminator = tokens
                .get(cursor)
                .ok_or_else(|| ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: missing_start(&tokens, cursor),
                })?;
            if terminator.kind != TokenKind::Semicolon {
                return Err(ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: terminator.start,
                });
            }
            (items, None, None, cursor + 1, terminator.end)
        };

        declarations.push(PublicDeclaration {
            items,
            initializer,
            initializer_expression,
            source_name: source.source_name.clone(),
            start: tokens[index].start,
            end,
        });
        index = next_index;
    }

    Ok(declarations)
}

pub fn parse_public_table_declarations(
    source: &SourceFile,
) -> Result<Vec<PublicTableDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::PublicTable
            || !tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::Aggregate)
        {
            index += 1;
            continue;
        }

        let aggregate_open = index + 2;
        let (_aggregate_span, after_aggregate) =
            parse_delimited_span(&tokens, aggregate_open, source)?;

        let aggregate_type_index = index + 3;
        let Some(aggregate_type_token) = tokens.get(aggregate_type_index) else {
            return Err(ParseError::ExpectedName {
                source_name: source.source_name.clone(),
                start: missing_start(&tokens, aggregate_type_index),
            });
        };
        if aggregate_type_token.kind != TokenKind::Identifier {
            return Err(ParseError::ExpectedName {
                source_name: source.source_name.clone(),
                start: aggregate_type_token.start,
            });
        }

        let aggregate_comma =
            tokens
                .get(index + 4)
                .ok_or_else(|| ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: missing_start(&tokens, index + 4),
                })?;
        if aggregate_comma.kind != TokenKind::Comma {
            return Err(ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: aggregate_comma.start,
            });
        }

        let aggregate_function_index = index + 5;
        let Some(aggregate_function_token) = tokens.get(aggregate_function_index) else {
            return Err(ParseError::ExpectedName {
                source_name: source.source_name.clone(),
                start: missing_start(&tokens, aggregate_function_index),
            });
        };
        if aggregate_function_token.kind != TokenKind::Identifier {
            return Err(ParseError::ExpectedName {
                source_name: source.source_name.clone(),
                start: aggregate_function_token.start,
            });
        }

        let mut cursor = index + 6;
        let (args, args_expressions) = if tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::Comma)
        {
            let args_start_index = cursor + 1;
            let close_index =
                after_aggregate
                    .checked_sub(1)
                    .ok_or_else(|| ParseError::ExpectedCloseParen {
                        source_name: source.source_name.clone(),
                        start: aggregate_function_token.end,
                    })?;
            let Some(args_start) = tokens.get(args_start_index) else {
                return Err(ParseError::ExpectedName {
                    source_name: source.source_name.clone(),
                    start: missing_start(&tokens, args_start_index),
                });
            };
            if args_start.kind == TokenKind::RParen {
                return Err(ParseError::ExpectedName {
                    source_name: source.source_name.clone(),
                    start: args_start.start,
                });
            }
            cursor = after_aggregate;
            (
                Some(SourceSpan {
                    start: args_start.start,
                    end: tokens[close_index].start,
                }),
                parse_expression_list_range_best_effort(
                    &tokens,
                    args_start_index,
                    close_index,
                    source,
                ),
            )
        } else {
            if tokens
                .get(cursor)
                .is_none_or(|token| token.kind != TokenKind::RParen)
            {
                return Err(ParseError::ExpectedCloseParen {
                    source_name: source.source_name.clone(),
                    start: tokens
                        .get(cursor)
                        .map_or_else(|| missing_start(&tokens, cursor), |token| token.start),
                });
            }
            cursor = after_aggregate;
            (None, None)
        };

        let (name, after_name) = parse_alias_identifier(&tokens, cursor, source)?;
        let (cols, after_cols) = parse_delimited_span(&tokens, after_name, source)?;
        let (rows, after_rows) = parse_delimited_span(&tokens, after_cols, source)?;
        let cols_expression =
            parse_expression_range_best_effort(&tokens, after_name + 1, after_cols - 1, source);
        let rows_expression =
            parse_expression_range_best_effort(&tokens, after_cols + 1, after_rows - 1, source);

        declarations.push(PublicTableDeclaration {
            aggregate_type: aggregate_type_token.lexeme.clone(),
            aggregate_function: aggregate_function_token.lexeme.clone(),
            name,
            args,
            args_expressions,
            cols,
            cols_expression,
            rows,
            rows_expression,
            source_name: source.source_name.clone(),
            start: tokens[index].start,
            end: rows.end,
        });
        index = after_rows;
    }

    Ok(declarations)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GroupValueProperties {
    stage: u32,
    default_value: Option<SourceSpan>,
    default_expression: Option<Expression>,
    aggregate_type: Option<String>,
    next_index: usize,
}

fn parse_group_value_properties(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<GroupValueProperties, ParseError> {
    let mut stage = DEFAULT_AIR_GROUP_VALUE_STAGE;
    let mut stage_seen = false;
    let mut default_value = None;
    let mut default_expression = None;
    let mut aggregate_type = None;
    let mut cursor = index;

    while let Some(token) = tokens.get(cursor) {
        match token.kind {
            TokenKind::Stage => {
                if stage_seen {
                    return Err(ParseError::DuplicateProperty {
                        source_name: source.source_name.clone(),
                        start: token.start,
                        name: "stage",
                    });
                }
                let (parsed_stage, next) =
                    parse_optional_stage_definition(tokens, cursor, stage, source)?;
                stage = parsed_stage;
                stage_seen = true;
                cursor = next;
            }
            TokenKind::Default => {
                if default_value.is_some() {
                    return Err(ParseError::DuplicateProperty {
                        source_name: source.source_name.clone(),
                        start: token.start,
                        name: "default",
                    });
                }
                let open_index = cursor + 1;
                let (span, next) = parse_delimited_span(tokens, open_index, source)?;
                let close_index = next.saturating_sub(1);
                default_expression =
                    parse_expression_range_best_effort(tokens, open_index + 1, close_index, source);
                default_value = Some(span);
                cursor = next;
            }
            TokenKind::Aggregate => {
                if aggregate_type.is_some() {
                    return Err(ParseError::DuplicateProperty {
                        source_name: source.source_name.clone(),
                        start: token.start,
                        name: "aggregate",
                    });
                }
                let (parsed_type, next) = parse_aggregate_type_definition(tokens, cursor, source)?;
                aggregate_type = Some(parsed_type);
                cursor = next;
            }
            _ => break,
        }
    }

    Ok(GroupValueProperties {
        stage,
        default_value,
        default_expression,
        aggregate_type,
        next_index: cursor,
    })
}

fn parse_aggregate_type_definition(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(String, usize), ParseError> {
    let open_index = index + 1;
    let Some(open) = tokens.get(open_index) else {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, open_index),
        });
    };
    if open.kind != TokenKind::LParen {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: open.start,
        });
    }

    let name_index = index + 2;
    let Some(name) = tokens.get(name_index) else {
        return Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, name_index),
        });
    };
    if name.kind != TokenKind::Identifier {
        return Err(ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: name.start,
        });
    }

    let close_index = index + 3;
    let Some(close) = tokens.get(close_index) else {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, close_index),
        });
    };
    if close.kind != TokenKind::RParen {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: close.start,
        });
    }

    Ok((name.lexeme.clone(), close_index + 1))
}

fn parse_commit_public_reference(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(Vec<String>, usize), ParseError> {
    let Some(token) = tokens.get(index) else {
        return Ok((Vec::new(), index));
    };
    if token.kind != TokenKind::Public {
        return Ok((Vec::new(), index));
    }

    let open_index = index + 1;
    let Some(open) = tokens.get(open_index) else {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, open_index),
        });
    };
    if open.kind != TokenKind::LParen {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: open.start,
        });
    }

    let mut cursor = open_index + 1;
    let (first_name, next_index) = parse_commit_public_name(tokens, cursor, source)?;
    let mut names = vec![first_name];
    cursor = next_index;

    while tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Comma)
    {
        let (name, next) = parse_commit_public_name(tokens, cursor + 1, source)?;
        names.push(name);
        cursor = next;
    }

    let close = tokens
        .get(cursor)
        .ok_or_else(|| ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, cursor),
        })?;
    if close.kind != TokenKind::RParen {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: close.start,
        });
    }

    Ok((names, cursor + 1))
}

fn parse_commit_public_name(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(String, usize), ParseError> {
    parse_name_reference(tokens, index, source)
}

fn public_declaration_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::TemplateLiteral
            | TokenKind::Air
            | TokenKind::AirGroup
            | TokenKind::Proof
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValueHeader {
    kind: ValueDeclarationKind,
    default_stage: u32,
    start: usize,
    next_index: usize,
}

fn parse_value_header(tokens: &[Token], index: usize) -> Option<ValueHeader> {
    let (kind, default_stage) = match tokens.get(index)?.kind {
        TokenKind::Challenge => (ValueDeclarationKind::Challenge, DEFAULT_CHALLENGE_STAGE),
        TokenKind::ProofValue => (ValueDeclarationKind::ProofValue, DEFAULT_VALUE_STAGE),
        TokenKind::AirValue => (ValueDeclarationKind::AirValue, DEFAULT_VALUE_STAGE),
        _ => return None,
    };
    if !tokens
        .get(index + 1)
        .is_some_and(|token| value_tail_start(token.kind))
    {
        return None;
    }
    Some(ValueHeader {
        kind,
        default_stage,
        start: tokens[index].start,
        next_index: index + 1,
    })
}

fn value_tail_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Stage
            | TokenKind::Identifier
            | TokenKind::TemplateLiteral
            | TokenKind::Air
            | TokenKind::AirGroup
            | TokenKind::Proof
    )
}

fn group_value_property_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Stage | TokenKind::Default | TokenKind::Aggregate
    )
}

fn parse_optional_stage_definition(
    tokens: &[Token],
    index: usize,
    default_stage: u32,
    source: &SourceFile,
) -> Result<(u32, usize), ParseError> {
    if !tokens
        .get(index)
        .is_some_and(|token| token.kind == TokenKind::Stage)
    {
        return Ok((default_stage, index));
    }

    let open_index = index + 1;
    let Some(open) = tokens.get(open_index) else {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, open_index),
        });
    };
    if open.kind != TokenKind::LParen {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: open.start,
        });
    }

    let number_index = index + 2;
    let Some(number) = tokens.get(number_index) else {
        return Err(ParseError::ExpectedNumber {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, number_index),
        });
    };
    let stage = parse_u32_literal(number, source)?;

    let close_index = index + 3;
    let Some(close) = tokens.get(close_index) else {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, close_index),
        });
    };
    if close.kind != TokenKind::RParen {
        return Err(ParseError::ExpectedCloseParen {
            source_name: source.source_name.clone(),
            start: close.start,
        });
    }

    Ok((stage, close_index + 1))
}

fn parse_u32_literal(token: &Token, source: &SourceFile) -> Result<u32, ParseError> {
    let parsed = match token.kind {
        TokenKind::Integer => token.lexeme.parse::<u32>(),
        TokenKind::HexInteger => u32::from_str_radix(
            token
                .lexeme
                .strip_prefix("0x")
                .or_else(|| token.lexeme.strip_prefix("0X"))
                .unwrap_or(token.lexeme.as_str()),
            16,
        ),
        _ => {
            return Err(ParseError::ExpectedNumber {
                source_name: source.source_name.clone(),
                start: token.start,
            });
        }
    };
    parsed.map_err(|_| ParseError::ExpectedNumber {
        source_name: source.source_name.clone(),
        start: token.start,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedConstantDeclaration {
    declaration: ConstantDeclaration,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedArrayDimensions {
    dims: Vec<SourceSpan>,
    dim_expressions: Vec<Option<Expression>>,
    next_index: usize,
}

fn parse_constant_declaration_at(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<ParsedConstantDeclaration, ParseError> {
    let token = tokens.get(index).ok_or_else(|| ParseError::ExpectedName {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, index),
    })?;
    let kind = match token.kind {
        TokenKind::Constant => ConstantDeclarationKind::Constant,
        TokenKind::Const => ConstantDeclarationKind::Const,
        _ => {
            return Err(ParseError::ExpectedName {
                source_name: source.source_name.clone(),
                start: token.start,
            });
        }
    };

    let mut cursor = index + 1;
    let type_name = if kind == ConstantDeclarationKind::Const {
        let type_token = tokens.get(cursor).ok_or_else(|| ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, cursor),
        })?;
        let type_name = constant_type_name(type_token).ok_or_else(|| ParseError::ExpectedName {
            source_name: source.source_name.clone(),
            start: type_token.start,
        })?;
        cursor += 1;
        Some(type_name)
    } else {
        None
    };

    let (name, after_name) = parse_name_reference(tokens, cursor, source)?;
    cursor = after_name;
    let array_parse = parse_array_dimensions(tokens, cursor, source)?;
    cursor = array_parse.next_index;

    let (initializer, initializer_expression, next_index, end) = if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Assign)
    {
        let (span, terminator_index) =
            parse_expression_span_until_terminator(tokens, cursor + 1, source)?;
        let expression = parse_expression_span_best_effort(tokens, span, source);
        let terminator =
            tokens
                .get(terminator_index)
                .ok_or_else(|| ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: missing_start(tokens, terminator_index),
                })?;
        if terminator.kind != TokenKind::Semicolon {
            return Err(ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: terminator.start,
            });
        }
        (Some(span), expression, terminator_index + 1, terminator.end)
    } else {
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
        (None, None, cursor + 1, terminator.end)
    };

    Ok(ParsedConstantDeclaration {
        declaration: ConstantDeclaration {
            kind,
            type_name,
            name,
            array_dims: array_parse.dims,
            array_dim_expressions: array_parse.dim_expressions,
            initializer,
            initializer_expression,
            source_name: source.source_name.clone(),
            start: token.start,
            end,
        },
        next_index,
    })
}

fn constant_type_name(token: &Token) -> Option<String> {
    match token.kind {
        TokenKind::Expr | TokenKind::Fe | TokenKind::Int | TokenKind::String => {
            Some(token.lexeme.clone())
        }
        _ => None,
    }
}

fn parse_array_dimensions(
    tokens: &[Token],
    mut cursor: usize,
    source: &SourceFile,
) -> Result<ParsedArrayDimensions, ParseError> {
    let mut array_dims = Vec::new();
    let mut array_dim_expressions = Vec::new();

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

    Ok(ParsedArrayDimensions {
        dims: array_dims,
        dim_expressions: array_dim_expressions,
        next_index: cursor,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVariableDeclaration {
    declaration: VariableDeclaration,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedFunctionStatementDeclaration {
    pub(crate) declaration: FunctionStatementDeclaration,
    pub(crate) next_index: usize,
}

fn parse_variable_declaration_at(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<ParsedVariableDeclaration, ParseError> {
    let type_token = tokens.get(index).ok_or_else(|| ParseError::ExpectedName {
        source_name: source.source_name.clone(),
        start: missing_start(tokens, index),
    })?;
    let type_name = variable_type_name(type_token).ok_or_else(|| ParseError::ExpectedName {
        source_name: source.source_name.clone(),
        start: type_token.start,
    })?;

    let mut cursor = index + 1;
    let (name, after_name) = parse_name_reference(tokens, cursor, source)?;
    cursor = after_name;
    let array_parse = parse_array_dimensions(tokens, cursor, source)?;
    cursor = array_parse.next_index;

    let (initializer, initializer_expression, next_index, end) = if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Assign)
    {
        let (span, terminator_index) =
            parse_expression_span_until_terminator(tokens, cursor + 1, source)?;
        let expression = parse_expression_span_best_effort(tokens, span, source);
        let terminator =
            tokens
                .get(terminator_index)
                .ok_or_else(|| ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: missing_start(tokens, terminator_index),
                })?;
        if terminator.kind != TokenKind::Semicolon {
            return Err(ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: terminator.start,
            });
        }
        (Some(span), expression, terminator_index + 1, terminator.end)
    } else {
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
        (None, None, cursor + 1, terminator.end)
    };

    Ok(ParsedVariableDeclaration {
        declaration: VariableDeclaration {
            type_name,
            name,
            array_dims: array_parse.dims,
            array_dim_expressions: array_parse.dim_expressions,
            initializer,
            initializer_expression,
            source_name: source.source_name.clone(),
            start: type_token.start,
            end,
        },
        next_index,
    })
}

pub(crate) fn parse_function_statement_declaration_at(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<Option<ParsedFunctionStatementDeclaration>, ParseError> {
    let Some(token) = tokens.get(index) else {
        return Ok(None);
    };

    match token.kind {
        TokenKind::Const | TokenKind::Constant => {
            let parsed = parse_constant_declaration_at(tokens, index, source)?;
            Ok(Some(ParsedFunctionStatementDeclaration {
                declaration: FunctionStatementDeclaration::Constant(parsed.declaration),
                next_index: parsed.next_index,
            }))
        }
        TokenKind::Int | TokenKind::Fe | TokenKind::Expr | TokenKind::String => {
            let parsed = parse_variable_declaration_at(tokens, index, source)?;
            Ok(Some(ParsedFunctionStatementDeclaration {
                declaration: FunctionStatementDeclaration::Variable(parsed.declaration),
                next_index: parsed.next_index,
            }))
        }
        TokenKind::Col => {
            let Some(parsed) = parse_column_declaration_at(tokens, index, source)? else {
                return Ok(None);
            };
            Ok(Some(ParsedFunctionStatementDeclaration {
                declaration: FunctionStatementDeclaration::Column(parsed.declaration),
                next_index: parsed.next_index,
            }))
        }
        _ => Ok(None),
    }
}

fn variable_type_name(token: &Token) -> Option<String> {
    match token.kind {
        TokenKind::Expr | TokenKind::Fe | TokenKind::Int | TokenKind::String => {
            Some(token.lexeme.clone())
        }
        _ => None,
    }
}

fn declaration_statement_context(stack: &[TokenKind]) -> bool {
    stack
        .iter()
        .all(|kind| !matches!(kind, TokenKind::RParen | TokenKind::RBracket))
}

fn variable_statement_context(tokens: &[Token], index: usize, stack: &[TokenKind]) -> bool {
    if !declaration_statement_context(stack) {
        return false;
    }
    if tokens
        .get(index)
        .is_none_or(|token| variable_type_name(token).is_none())
    {
        return false;
    }
    if !tokens
        .get(index + 1)
        .is_some_and(|token| token.kind == TokenKind::Identifier)
    {
        return false;
    }

    index == 0
        || tokens.get(index - 1).is_some_and(|token| {
            matches!(
                token.kind,
                TokenKind::Semicolon | TokenKind::LBrace | TokenKind::RBrace
            )
        })
}

fn update_declaration_scan_stack(stack: &mut Vec<TokenKind>, kind: TokenKind) {
    match kind {
        TokenKind::LParen => stack.push(TokenKind::RParen),
        TokenKind::LBracket => stack.push(TokenKind::RBracket),
        TokenKind::LBrace => stack.push(TokenKind::RBrace),
        TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
            if stack.last().is_some_and(|expected| *expected == kind) {
                stack.pop();
            }
        }
        _ => {}
    }
}

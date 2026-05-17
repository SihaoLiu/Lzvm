use super::super::expressions::parse_call_arguments_span_best_effort;
use super::super::functions::{parse_function_body_statements, parse_function_parameters};
use super::*;

pub fn parse_container_declarations(
    source: &SourceFile,
) -> Result<Vec<ContainerDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Container {
            index += 1;
            continue;
        }

        let header = parse_named_header(&tokens, index, source)?;
        let next_token =
            tokens
                .get(header.next_index)
                .ok_or_else(|| ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: missing_start(&tokens, header.next_index),
                })?;
        let (body, end, next_index) = match next_token.kind {
            TokenKind::Semicolon => (None, next_token.end, header.next_index + 1),
            TokenKind::LBrace => {
                let (span, next_index) = parse_braced_span(&tokens, header.next_index, source)?;
                (Some(span), span.end, next_index)
            }
            _ => {
                return Err(ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: next_token.start,
                });
            }
        };

        declarations.push(ContainerDeclaration {
            name: header.name,
            alias: header.alias,
            body,
            source_name: source.source_name.clone(),
            start: header.start,
            end,
        });
        index = next_index;
    }

    Ok(declarations)
}

pub fn parse_air_template_declarations(
    source: &SourceFile,
) -> Result<Vec<AirTemplateDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::AirTemplate {
            index += 1;
            continue;
        }

        let start = tokens[index].start;
        let (name, after_name) = parse_name_reference(&tokens, index + 1, source)?;
        let (params, parameters, after_params) = if tokens
            .get(after_name)
            .is_some_and(|token| token.kind == TokenKind::LParen)
        {
            let (params, after_params) = parse_delimited_span(&tokens, after_name, source)?;
            let parameters =
                parse_function_parameters(&tokens, after_name + 1, after_params - 1, source)?;
            (Some(params), parameters, after_params)
        } else {
            (None, Vec::new(), after_name)
        };
        if !tokens
            .get(after_params)
            .is_some_and(|token| token.kind == TokenKind::LBrace)
        {
            index += 1;
            continue;
        }
        let (body, next_index) = parse_required_braced_span(&tokens, after_params, source)?;
        let statements = parse_function_body_statements(&tokens, body, source)?;

        declarations.push(AirTemplateDeclaration {
            name,
            params,
            parameters,
            body,
            statements,
            source_name: source.source_name.clone(),
            start,
            end: body.end,
        });
        index = next_index;
    }

    Ok(declarations)
}

pub fn parse_air_group_declarations(
    source: &SourceFile,
) -> Result<Vec<AirGroupDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::AirGroup {
            index += 1;
            continue;
        }
        if tokens
            .get(index + 1)
            .is_some_and(|token| token.kind == TokenKind::Dot)
        {
            index += 1;
            continue;
        }

        let start = tokens[index].start;
        let (name, after_name) = parse_name_reference(&tokens, index + 1, source)?;
        if !tokens
            .get(after_name)
            .is_some_and(|token| token.kind == TokenKind::LBrace)
        {
            index += 1;
            continue;
        }
        let (body, next_index) = parse_required_braced_span(&tokens, after_name, source)?;
        let statements = parse_function_body_statements(&tokens, body, source)?;

        declarations.push(AirGroupDeclaration {
            name,
            body,
            statements,
            source_name: source.source_name.clone(),
            start,
            end: body.end,
        });
        index = next_index;
    }

    Ok(declarations)
}

pub fn parse_air_instance_declarations(
    source: &SourceFile,
) -> Result<Vec<AirInstanceDeclaration>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let local_templates = parse_air_template_declarations(source)?
        .into_iter()
        .map(|declaration| declaration.name)
        .collect::<std::collections::BTreeSet<_>>();
    let mut declarations = Vec::new();

    for group in parse_air_group_declarations(source)? {
        let open_index = tokens
            .iter()
            .position(|token| token.kind == TokenKind::LBrace && token.start == group.body.start)
            .ok_or_else(|| ParseError::ExpectedCloseBrace {
                source_name: source.source_name.clone(),
                start: group.body.start,
            })?;
        let close_index = tokens
            .iter()
            .rposition(|token| token.kind == TokenKind::RBrace && token.end == group.body.end)
            .ok_or_else(|| ParseError::ExpectedCloseBrace {
                source_name: source.source_name.clone(),
                start: group.body.start,
            })?;

        let mut cursor = open_index + 1;
        while cursor < close_index {
            if tokens[cursor].kind == TokenKind::LBrace {
                let (_, next_index) = parse_braced_span(&tokens, cursor, source)?;
                cursor = next_index;
                continue;
            }
            if let Some(parsed) =
                parse_air_instance_at(&tokens, cursor, source, &group, &local_templates)?
            {
                cursor = parsed.next_index;
                declarations.push(parsed.declaration);
            } else {
                cursor += 1;
            }
        }
    }

    Ok(declarations)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedAirInstance {
    declaration: AirInstanceDeclaration,
    next_index: usize,
}

fn parse_air_instance_at(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
    group: &AirGroupDeclaration,
    local_templates: &std::collections::BTreeSet<String>,
) -> Result<Option<ParsedAirInstance>, ParseError> {
    let Some(token) = tokens.get(index) else {
        return Ok(None);
    };
    let (virtual_instance, name_index, start) = if token.kind == TokenKind::Virtual {
        (true, index + 1, token.start)
    } else if air_instance_name_start(token.kind) {
        (false, index, token.start)
    } else {
        return Ok(None);
    };

    let Some(name_token) = tokens.get(name_index) else {
        return Ok(None);
    };
    if !air_instance_name_start(name_token.kind) {
        return Ok(None);
    }
    let (template, after_name) = parse_name_reference(tokens, name_index, source)?;
    if !tokens
        .get(after_name)
        .is_some_and(|token| token.kind == TokenKind::LParen)
    {
        return Ok(None);
    }
    let (args, mut cursor) = parse_delimited_span(tokens, after_name, source)?;
    let args_expressions = parse_call_arguments_span_best_effort(tokens, args, source);
    let alias = if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Alias)
    {
        let (alias, next) = parse_alias_identifier(tokens, cursor + 1, source)?;
        cursor = next;
        Some(alias)
    } else {
        None
    };

    if !air_instance_candidate(&template, virtual_instance, alias.as_ref(), local_templates) {
        return Ok(None);
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

    Ok(Some(ParsedAirInstance {
        declaration: AirInstanceDeclaration {
            air_group: group.name.clone(),
            template,
            alias,
            virtual_instance,
            args,
            args_expressions,
            source_name: source.source_name.clone(),
            start,
            end: terminator.end,
        },
        next_index: cursor + 1,
    }))
}

fn air_instance_name_start(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Identifier | TokenKind::TemplateLiteral)
}

fn air_instance_candidate(
    template: &str,
    virtual_instance: bool,
    alias: Option<&String>,
    local_templates: &std::collections::BTreeSet<String>,
) -> bool {
    virtual_instance
        || alias.is_some()
        || local_templates.contains(template)
        || template
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IncludeHeader {
    pub(crate) start_index: usize,
    pub(crate) directive_index: usize,
    pub(crate) kind: IncludeKind,
    pub(crate) visibility: IncludeVisibility,
}

pub(crate) fn include_header(tokens: &[Token], index: usize) -> Option<IncludeHeader> {
    match tokens[index].kind {
        TokenKind::Private => directive_after_visibility(tokens, index, IncludeVisibility::Private),
        TokenKind::Public => directive_after_visibility(tokens, index, IncludeVisibility::Public),
        TokenKind::Include => Some(IncludeHeader {
            start_index: index,
            directive_index: index,
            kind: IncludeKind::Include,
            visibility: IncludeVisibility::Public,
        }),
        TokenKind::Require => Some(IncludeHeader {
            start_index: index,
            directive_index: index,
            kind: IncludeKind::Require,
            visibility: IncludeVisibility::Public,
        }),
        _ => None,
    }
}

fn directive_after_visibility(
    tokens: &[Token],
    index: usize,
    visibility: IncludeVisibility,
) -> Option<IncludeHeader> {
    let directive_index = index + 1;
    let kind = match tokens.get(directive_index)?.kind {
        TokenKind::Include => IncludeKind::Include,
        TokenKind::Require => IncludeKind::Require,
        _ => return None,
    };
    Some(IncludeHeader {
        start_index: index,
        directive_index,
        kind,
        visibility,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedStatement {
    pub(crate) name: String,
    pub(crate) alias: Option<String>,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NamedHeader {
    name: String,
    alias: Option<String>,
    start: usize,
    next_index: usize,
}

pub(crate) fn parse_named_statement(
    tokens: &[Token],
    keyword_index: usize,
    source: &SourceFile,
) -> Result<NamedStatement, ParseError> {
    let header = parse_named_header(tokens, keyword_index, source)?;

    let terminator =
        tokens
            .get(header.next_index)
            .ok_or_else(|| ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: missing_start(tokens, header.next_index),
            })?;
    if terminator.kind != TokenKind::Semicolon {
        return Err(ParseError::ExpectedTerminator {
            source_name: source.source_name.clone(),
            start: terminator.start,
        });
    }

    Ok(NamedStatement {
        name: header.name,
        alias: header.alias,
        start: header.start,
        end: terminator.end,
        next_index: header.next_index + 1,
    })
}

fn parse_named_header(
    tokens: &[Token],
    keyword_index: usize,
    source: &SourceFile,
) -> Result<NamedHeader, ParseError> {
    let start = tokens[keyword_index].start;
    let (name, mut cursor) = parse_name_reference(tokens, keyword_index + 1, source)?;
    let mut alias = None;
    if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Alias)
    {
        let (alias_value, next) = parse_alias_identifier(tokens, cursor + 1, source)?;
        alias = Some(alias_value);
        cursor = next;
    }

    Ok(NamedHeader {
        name,
        alias,
        start,
        next_index: cursor,
    })
}

fn parse_braced_span(
    tokens: &[Token],
    open_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    parse_delimited_span(tokens, open_index, source)
}

pub(crate) fn parse_required_braced_span(
    tokens: &[Token],
    open_index: usize,
    source: &SourceFile,
) -> Result<(SourceSpan, usize), ParseError> {
    let Some(open) = tokens.get(open_index) else {
        return Err(ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start: missing_start(tokens, open_index),
        });
    };
    if open.kind != TokenKind::LBrace {
        return Err(ParseError::ExpectedCloseBrace {
            source_name: source.source_name.clone(),
            start: open.start,
        });
    }
    parse_braced_span(tokens, open_index, source)
}

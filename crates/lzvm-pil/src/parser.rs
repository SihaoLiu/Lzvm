use crate::{lex_source, LexError, SourceFile, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeKind {
    Include,
    Require,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeVisibility {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDirective {
    pub kind: IncludeKind,
    pub visibility: IncludeVisibility,
    pub file: String,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseDirective {
    pub name: String,
    pub alias: Option<String>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerDeclaration {
    pub name: String,
    pub alias: Option<String>,
    pub body: Option<SourceSpan>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    Witness,
    Fixed,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnFeature {
    pub name: String,
    pub args: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnItem {
    pub name: String,
    pub template: bool,
    pub array_dims: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnInitializerKind {
    Expression,
    Sequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnInitializer {
    pub kind: ColumnInitializerKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDeclaration {
    pub kind: ColumnKind,
    pub commit: Option<String>,
    pub features: Vec<ColumnFeature>,
    pub items: Vec<ColumnItem>,
    pub initializer: Option<ColumnInitializer>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Lex {
        source_name: String,
        error: LexError,
    },
    ExpectedPath {
        source_name: String,
        start: usize,
    },
    TemplatePath {
        source_name: String,
        start: usize,
        end: usize,
    },
    ExpectedTerminator {
        source_name: String,
        start: usize,
    },
    ExpectedName {
        source_name: String,
        start: usize,
    },
    ExpectedAlias {
        source_name: String,
        start: usize,
    },
    ExpectedCloseBrace {
        source_name: String,
        start: usize,
    },
    ExpectedCloseParen {
        source_name: String,
        start: usize,
    },
    ExpectedCloseBracket {
        source_name: String,
        start: usize,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex { source_name, error } => write!(f, "{source_name}: {error}"),
            Self::ExpectedPath { source_name, start } => {
                write!(f, "{source_name}: expected include path at {start}")
            }
            Self::TemplatePath {
                source_name, start, ..
            } => write!(f, "{source_name}: template include path at {start}"),
            Self::ExpectedTerminator { source_name, start } => {
                write!(f, "{source_name}: expected statement terminator at {start}")
            }
            Self::ExpectedName { source_name, start } => {
                write!(f, "{source_name}: expected name at {start}")
            }
            Self::ExpectedAlias { source_name, start } => {
                write!(f, "{source_name}: expected alias at {start}")
            }
            Self::ExpectedCloseBrace { source_name, start } => {
                write!(f, "{source_name}: expected closing brace at {start}")
            }
            Self::ExpectedCloseParen { source_name, start } => {
                write!(f, "{source_name}: expected closing parenthesis at {start}")
            }
            Self::ExpectedCloseBracket { source_name, start } => {
                write!(f, "{source_name}: expected closing bracket at {start}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_include_directives(source: &SourceFile) -> Result<Vec<IncludeDirective>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut directives = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let Some(header) = include_header(&tokens, index) else {
            index += 1;
            continue;
        };
        let path_index = header.directive_index + 1;
        let Some(path_token) = tokens.get(path_index) else {
            return Err(ParseError::ExpectedPath {
                source_name: source.source_name.clone(),
                start: tokens[header.directive_index].end,
            });
        };
        let file = match path_token.kind {
            TokenKind::StringLiteral => path_token.lexeme.clone(),
            TokenKind::TemplateLiteral => {
                return Err(ParseError::TemplatePath {
                    source_name: source.source_name.clone(),
                    start: path_token.start,
                    end: path_token.end,
                });
            }
            _ => {
                return Err(ParseError::ExpectedPath {
                    source_name: source.source_name.clone(),
                    start: path_token.start,
                });
            }
        };

        let terminator_index = path_index + 1;
        let Some(terminator) = tokens.get(terminator_index) else {
            return Err(ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: path_token.end,
            });
        };
        if terminator.kind != TokenKind::Semicolon {
            return Err(ParseError::ExpectedTerminator {
                source_name: source.source_name.clone(),
                start: terminator.start,
            });
        }

        directives.push(IncludeDirective {
            kind: header.kind,
            visibility: header.visibility,
            file,
            source_name: source.source_name.clone(),
            start: tokens[header.start_index].start,
            end: terminator.end,
        });
        index = terminator_index + 1;
    }

    Ok(directives)
}

pub fn parse_use_directives(source: &SourceFile) -> Result<Vec<UseDirective>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut directives = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Use {
            index += 1;
            continue;
        }

        let statement = parse_named_statement(&tokens, index, source)?;
        directives.push(UseDirective {
            name: statement.name,
            alias: statement.alias,
            source_name: source.source_name.clone(),
            start: statement.start,
            end: statement.end,
        });
        index = statement.next_index;
    }

    Ok(directives)
}

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
        if tokens[index].kind != TokenKind::Col {
            index += 1;
            continue;
        }

        let Some(header) = parse_column_header(&tokens, index) else {
            index += 1;
            continue;
        };

        let mut cursor = header.next_index;
        let (features, next_cursor) = parse_column_features(&tokens, cursor, source)?;
        cursor = next_cursor;
        let (items, initializer, next_index, end) =
            parse_column_body(&tokens, cursor, header.kind, source)?;

        declarations.push(ColumnDeclaration {
            kind: header.kind,
            commit: header.commit,
            features,
            items,
            initializer,
            source_name: source.source_name.clone(),
            start: header.start,
            end,
        });
        index = next_index;
    }

    Ok(declarations)
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
    Ok(Some(ParsedColumnFeature {
        feature: ColumnFeature { name, args },
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

fn parse_column_item_list(
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

fn parse_column_item(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<(ColumnItem, usize), ParseError> {
    let parsed = parse_column_name_reference(tokens, index, source)?;
    let mut array_dims = Vec::new();
    let mut cursor = parsed.next_index;

    while tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        let (span, next) = parse_delimited_span(tokens, cursor, source)?;
        array_dims.push(span);
        cursor = next;
    }

    Ok((
        ColumnItem {
            name: parsed.name,
            template: parsed.template,
            array_dims,
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
        let (span, next_index) = parse_delimited_span(tokens, index, source)?;
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
            },
            next_index + 1,
            terminator.end,
        ));
    }

    let (span, next_index) = parse_expression_span_until_terminator(tokens, index, source)?;
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

#[derive(Debug, Clone, Copy)]
struct IncludeHeader {
    start_index: usize,
    directive_index: usize,
    kind: IncludeKind,
    visibility: IncludeVisibility,
}

fn include_header(tokens: &[Token], index: usize) -> Option<IncludeHeader> {
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
struct NamedStatement {
    name: String,
    alias: Option<String>,
    start: usize,
    end: usize,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NamedHeader {
    name: String,
    alias: Option<String>,
    start: usize,
    next_index: usize,
}

fn parse_named_statement(
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

fn parse_alias_identifier(
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
    if alias_token.kind != TokenKind::Identifier {
        return Err(ParseError::ExpectedAlias {
            source_name: source.source_name.clone(),
            start: alias_token.start,
        });
    }
    Ok((alias_token.lexeme.clone(), index + 1))
}

fn parse_name_reference(
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

fn parse_delimited_span(
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

fn parse_expression_span_until_terminator(
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

fn expected_close_error(kind: TokenKind, source: &SourceFile, start: usize) -> ParseError {
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

fn missing_start(tokens: &[Token], index: usize) -> usize {
    tokens.get(index).map_or_else(
        || tokens.last().map_or(0, |token| token.end),
        |token| token.start,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        parse_column_declarations, parse_container_declarations, parse_include_directives,
        parse_use_directives, ColumnInitializerKind, ColumnKind, IncludeKind, IncludeVisibility,
        ParseError,
    };
    use crate::SourceFile;
    use std::path::PathBuf;

    fn source(contents: &str) -> SourceFile {
        SourceFile {
            contents: contents.to_owned(),
            file_dir: PathBuf::from("/case"),
            full_path: PathBuf::from("/case/main.pil"),
            source_name: "main.pil".to_owned(),
        }
    }

    #[test]
    fn parses_static_include_directives() {
        let source =
            source("include \"a.pil\";\nprivate require \"b.pil\";\npublic include \"c.pil\";");

        let directives = parse_include_directives(&source).expect("directives should parse");

        assert_eq!(directives.len(), 3);
        assert_eq!(directives[0].kind, IncludeKind::Include);
        assert_eq!(directives[0].visibility, IncludeVisibility::Public);
        assert_eq!(directives[0].file, "a.pil");
        assert_eq!(directives[1].kind, IncludeKind::Require);
        assert_eq!(directives[1].visibility, IncludeVisibility::Private);
        assert_eq!(directives[1].file, "b.pil");
        assert_eq!(directives[2].kind, IncludeKind::Include);
        assert_eq!(directives[2].visibility, IncludeVisibility::Public);
        assert_eq!(directives[2].file, "c.pil");
    }

    #[test]
    fn ignores_visibility_modifiers_that_do_not_start_include_directives() {
        let source = source("public function f() { return; }\nprivate int x = 1;");

        let directives = parse_include_directives(&source).expect("source should parse");

        assert!(directives.is_empty());
    }

    #[test]
    fn rejects_template_include_paths() {
        let source = source("include `dynamic/${name}.pil`;");

        let error = parse_include_directives(&source).expect_err("template path should fail");

        assert!(matches!(
            error,
            ParseError::TemplatePath { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn rejects_include_without_path_literal() {
        let source = source("include ;");

        let error = parse_include_directives(&source).expect_err("path should be required");

        assert!(matches!(
            error,
            ParseError::ExpectedPath { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn rejects_missing_statement_terminator() {
        let source = source("include \"a.pil\" const N = 1;");

        let error = parse_include_directives(&source).expect_err("semicolon should be required");

        assert!(matches!(
            error,
            ParseError::ExpectedTerminator { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn parses_use_directives_with_names_and_aliases() {
        let source =
            source("use air.main;\nuse proof.root.branch alias local_root;\nuse pkg.item;");

        let directives = parse_use_directives(&source).expect("use directives should parse");

        assert_eq!(directives.len(), 3);
        assert_eq!(directives[0].name, "air.main");
        assert_eq!(directives[0].alias, None);
        assert_eq!(directives[1].name, "proof.root.branch");
        assert_eq!(directives[1].alias.as_deref(), Some("local_root"));
        assert_eq!(directives[2].name, "pkg.item");
    }

    #[test]
    fn rejects_use_without_name_reference() {
        let source = source("use ;");

        let error = parse_use_directives(&source).expect_err("name should be required");

        assert!(matches!(
            error,
            ParseError::ExpectedName { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn rejects_use_alias_without_identifier() {
        let source = source("use pkg.item alias ;");

        let error = parse_use_directives(&source).expect_err("alias identifier should be required");

        assert!(matches!(
            error,
            ParseError::ExpectedAlias { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn rejects_use_without_statement_terminator() {
        let source = source("use pkg.item include \"x.pil\";");

        let error = parse_use_directives(&source).expect_err("semicolon should be required");

        assert!(matches!(
            error,
            ParseError::ExpectedTerminator { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn parses_container_declarations_with_names_and_aliases() {
        let source = source(
            "container air.main;\ncontainer proof.root.branch alias local_root;\ncontainer pkg.item;",
        );

        let declarations =
            parse_container_declarations(&source).expect("container declarations should parse");

        assert_eq!(declarations.len(), 3);
        assert_eq!(declarations[0].name, "air.main");
        assert_eq!(declarations[0].alias, None);
        assert_eq!(declarations[0].body, None);
        assert_eq!(declarations[1].name, "proof.root.branch");
        assert_eq!(declarations[1].alias.as_deref(), Some("local_root"));
        assert_eq!(declarations[1].body, None);
        assert_eq!(declarations[2].name, "pkg.item");
        assert_eq!(declarations[2].body, None);
    }

    #[test]
    fn parses_closed_container_body_span() {
        let source = source("container air.main { col witness x; }");

        let declarations =
            parse_container_declarations(&source).expect("container body should parse");

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "air.main");
        let body = declarations[0].body.expect("body span should be recorded");
        assert_eq!(&source.contents[body.start..body.end], "{ col witness x; }");
        assert_eq!(declarations[0].end, body.end);
    }

    #[test]
    fn parses_closed_container_alias_body_span() {
        let source = source("container proof.root alias local_root { }");

        let declarations =
            parse_container_declarations(&source).expect("container body should parse");

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "proof.root");
        assert_eq!(declarations[0].alias.as_deref(), Some("local_root"));
        let body = declarations[0].body.expect("body span should be recorded");
        assert_eq!(&source.contents[body.start..body.end], "{ }");
    }

    #[test]
    fn keeps_nested_blocks_inside_closed_container_body_span() {
        let source = source("container pkg.item { function run() { return; } }");

        let declarations =
            parse_container_declarations(&source).expect("container body should parse");

        assert_eq!(declarations.len(), 1);
        let body = declarations[0].body.expect("body span should be recorded");
        assert_eq!(
            &source.contents[body.start..body.end],
            "{ function run() { return; } }"
        );
        assert_eq!(declarations[0].end, source.contents.len());
    }

    #[test]
    fn rejects_container_without_name_reference() {
        let source = source("container ;");

        let error = parse_container_declarations(&source).expect_err("name should be required");

        assert!(matches!(
            error,
            ParseError::ExpectedName { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn rejects_container_alias_without_identifier() {
        let source = source("container pkg.item alias ;");

        let error =
            parse_container_declarations(&source).expect_err("alias identifier should be required");

        assert!(matches!(
            error,
            ParseError::ExpectedAlias { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn rejects_unclosed_container_body() {
        let source = source("container pkg.item { col witness x;");

        let error = parse_container_declarations(&source).expect_err("body should close");

        assert!(matches!(
            error,
            ParseError::ExpectedCloseBrace { source_name, .. } if source_name == "main.pil"
        ));
    }

    #[test]
    fn parses_witness_column_declarations_with_array_items() {
        let source = source("col witness air.main[2], local[1][];");

        let declarations = parse_column_declarations(&source).expect("columns should parse");

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].kind, ColumnKind::Witness);
        assert_eq!(declarations[0].commit, None);
        assert!(declarations[0].features.is_empty());
        assert_eq!(declarations[0].items.len(), 2);
        assert_eq!(declarations[0].items[0].name, "air.main");
        assert!(!declarations[0].items[0].template);
        assert_eq!(
            &source.contents[declarations[0].items[0].array_dims[0].start
                ..declarations[0].items[0].array_dims[0].end],
            "[2]"
        );
        assert_eq!(declarations[0].items[1].name, "local");
        assert_eq!(declarations[0].items[1].array_dims.len(), 2);
        assert_eq!(
            &source.contents[declarations[0].items[1].array_dims[0].start
                ..declarations[0].items[1].array_dims[0].end],
            "[1]"
        );
        assert_eq!(
            &source.contents[declarations[0].items[1].array_dims[1].start
                ..declarations[0].items[1].array_dims[1].end],
            "[]"
        );
    }

    #[test]
    fn parses_custom_column_declarations_with_feature_spans() {
        let source = source("col local_commit stage(1 + (2)) virtual(foo(bar)) air.main, local;");

        let declarations = parse_column_declarations(&source).expect("columns should parse");

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].kind, ColumnKind::Custom);
        assert_eq!(declarations[0].commit.as_deref(), Some("local_commit"));
        assert_eq!(declarations[0].features.len(), 2);
        assert_eq!(declarations[0].features[0].name, "stage");
        assert_eq!(
            &source.contents
                [declarations[0].features[0].args.start..declarations[0].features[0].args.end],
            "(1 + (2))"
        );
        assert_eq!(declarations[0].features[1].name, "virtual");
        assert_eq!(
            &source.contents
                [declarations[0].features[1].args.start..declarations[0].features[1].args.end],
            "(foo(bar))"
        );
        assert_eq!(declarations[0].items.len(), 2);
        assert_eq!(declarations[0].items[0].name, "air.main");
        assert_eq!(declarations[0].items[1].name, "local");
    }

    #[test]
    fn parses_fixed_column_initializer_spans() {
        let source = source("col fixed stage(3) x = foo(bar[1] + baz);");

        let declarations = parse_column_declarations(&source).expect("columns should parse");

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].kind, ColumnKind::Fixed);
        assert_eq!(declarations[0].features.len(), 1);
        assert_eq!(declarations[0].features[0].name, "stage");
        assert_eq!(declarations[0].items.len(), 1);
        assert_eq!(declarations[0].items[0].name, "x");
        let initializer = declarations[0]
            .initializer
            .expect("initializer should be recorded");
        assert_eq!(initializer.kind, ColumnInitializerKind::Expression);
        assert_eq!(
            &source.contents[initializer.span.start..initializer.span.end],
            "foo(bar[1] + baz)"
        );
    }

    #[test]
    fn parses_sequence_initializer_spans() {
        let source = source("col fixed x = [foo(bar), baz[1]];");

        let declarations = parse_column_declarations(&source).expect("columns should parse");

        assert_eq!(declarations.len(), 1);
        let initializer = declarations[0]
            .initializer
            .expect("initializer should be recorded");
        assert_eq!(initializer.kind, ColumnInitializerKind::Sequence);
        assert_eq!(
            &source.contents[initializer.span.start..initializer.span.end],
            "[foo(bar), baz[1]]"
        );
    }

    #[test]
    fn skips_col_cast_expressions() {
        let source = source("value = col(x);");

        let declarations = parse_column_declarations(&source).expect("source should parse");

        assert!(declarations.is_empty());
    }
}

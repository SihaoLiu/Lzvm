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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueDeclarationKind {
    Challenge,
    ProofValue,
    AirValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueDeclaration {
    pub kind: ValueDeclarationKind,
    pub stage: u32,
    pub items: Vec<ColumnItem>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirGroupValueDeclaration {
    pub stage: u32,
    pub default_value: Option<SourceSpan>,
    pub aggregate_type: Option<String>,
    pub items: Vec<ColumnItem>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDeclaration {
    pub stage: u32,
    pub publics: Vec<String>,
    pub name: String,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

const DEFAULT_CHALLENGE_STAGE: u32 = 2;
const DEFAULT_VALUE_STAGE: u32 = 1;
const DEFAULT_AIR_GROUP_VALUE_STAGE: u32 = 2;

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
    ExpectedNumber {
        source_name: String,
        start: usize,
    },
    DuplicateProperty {
        source_name: String,
        start: usize,
        name: &'static str,
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
            Self::ExpectedNumber { source_name, start } => {
                write!(f, "{source_name}: expected number at {start}")
            }
            Self::DuplicateProperty {
                source_name,
                start,
                name,
            } => write!(f, "{source_name}: duplicate {name} property at {start}"),
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct GroupValueProperties {
    stage: u32,
    default_value: Option<SourceSpan>,
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
                let (span, next) = parse_delimited_span(tokens, cursor + 1, source)?;
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
mod tests;

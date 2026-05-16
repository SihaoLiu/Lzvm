use crate::{lex_source, LexError, SourceFile, Token, TokenKind};

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

        let statement = parse_named_statement(&tokens, index, source)?;
        declarations.push(ContainerDeclaration {
            name: statement.name,
            alias: statement.alias,
            source_name: source.source_name.clone(),
            start: statement.start,
            end: statement.end,
        });
        index = statement.next_index;
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

fn parse_named_statement(
    tokens: &[Token],
    keyword_index: usize,
    source: &SourceFile,
) -> Result<NamedStatement, ParseError> {
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

    Ok(NamedStatement {
        name,
        alias,
        start,
        end: terminator.end,
        next_index: cursor + 1,
    })
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

fn missing_start(tokens: &[Token], index: usize) -> usize {
    tokens.get(index).map_or_else(
        || tokens.last().map_or(0, |token| token.end),
        |token| token.start,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        parse_container_declarations, parse_include_directives, parse_use_directives, IncludeKind,
        IncludeVisibility, ParseError,
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
        assert_eq!(declarations[1].name, "proof.root.branch");
        assert_eq!(declarations[1].alias.as_deref(), Some("local_root"));
        assert_eq!(declarations[2].name, "pkg.item");
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
    fn rejects_container_body_without_declare_block_parser() {
        let source = source("container pkg.item { col witness x; }");

        let error =
            parse_container_declarations(&source).expect_err("body should require body parser");

        assert!(matches!(
            error,
            ParseError::ExpectedTerminator { source_name, .. } if source_name == "main.pil"
        ));
    }
}

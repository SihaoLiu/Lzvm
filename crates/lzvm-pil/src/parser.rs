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

#[cfg(test)]
mod tests {
    use super::{parse_include_directives, IncludeKind, IncludeVisibility, ParseError};
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
}

mod declarations;
mod functions;
mod types;

pub use declarations::*;
pub use functions::*;
pub use types::*;

use crate::{lex_source, SourceFile, Token, TokenKind};
use declarations::{include_header, parse_named_statement};

const DEFAULT_CHALLENGE_STAGE: u32 = 2;
const DEFAULT_VALUE_STAGE: u32 = 1;
const DEFAULT_AIR_GROUP_VALUE_STAGE: u32 = 2;

pub fn parse_pragma_directives(source: &SourceFile) -> Result<Vec<PragmaDirective>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut directives = Vec::new();
    for token in &tokens {
        if token.kind == TokenKind::Pragma {
            directives.push(PragmaDirective {
                value: token.lexeme.clone(),
                source_name: source.source_name.clone(),
                start: token.start,
                end: token.end,
            });
        }
    }
    Ok(directives)
}

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

#[cfg(test)]
mod tests;

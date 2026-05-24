use lzvm_pil::{Token, TokenKind};

use super::{unsupported, unsupported_source_message};
use crate::source_key_directory::SourceKeyDirectoryMetadataError;

pub(super) fn source_directive_statement_start(tokens: &[Token], index: usize) -> Option<usize> {
    match tokens.get(index)?.kind {
        TokenKind::Include | TokenKind::Require | TokenKind::Use => Some(index),
        TokenKind::Public | TokenKind::Private
            if tokens.get(index + 1).is_some_and(|next| {
                matches!(
                    next.kind,
                    TokenKind::Include | TokenKind::Require | TokenKind::Use
                )
            }) =>
        {
            Some(index + 1)
        }
        _ => None,
    }
}

pub(super) fn skip_source_directive_statement(
    tokens: &[Token],
    index: usize,
    limit: usize,
    source: &str,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let mut stack = Vec::<TokenKind>::new();
    let mut cursor = index;
    let mut last_end = tokens
        .get(index)
        .ok_or_else(|| unsupported_source_message("source directive has no terminator"))?
        .end;
    while cursor < limit {
        let Some(token) = tokens.get(cursor) else {
            break;
        };
        if stack.is_empty() {
            if token.kind == TokenKind::Semicolon {
                return Ok(cursor + 1);
            }
            if cursor > index && source_has_line_break(source, last_end, token.start) {
                return Ok(cursor);
            }
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return unsupported("source directive has an unmatched closing delimiter");
                };
                if token.kind != expected {
                    return unsupported("source directive delimiters are not balanced");
                }
            }
            _ => {}
        }
        last_end = token.end;
        cursor += 1;
    }
    Ok(limit)
}

fn source_has_line_break(source: &str, start: usize, end: usize) -> bool {
    source[start..end]
        .bytes()
        .any(|byte| matches!(byte, b'\n' | b'\r'))
}

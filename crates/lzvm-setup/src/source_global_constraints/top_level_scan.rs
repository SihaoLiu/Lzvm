use lzvm_pil::{Token, TokenKind};

use crate::source_key_directory::SourceKeyDirectoryMetadataError;

use super::{unsupported, unsupported_source_message};

pub(super) fn top_level_declaration_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::AirGroup
            | TokenKind::AirGroupValue
            | TokenKind::AirTemplate
            | TokenKind::AirValue
            | TokenKind::Challenge
            | TokenKind::Col
            | TokenKind::Commit
            | TokenKind::Const
            | TokenKind::Constant
            | TokenKind::Container
            | TokenKind::Declare
            | TokenKind::Expr
            | TokenKind::Fe
            | TokenKind::For
            | TokenKind::Function
            | TokenKind::Include
            | TokenKind::Int
            | TokenKind::Package
            | TokenKind::ProofValue
            | TokenKind::Public
            | TokenKind::PublicTable
            | TokenKind::Require
            | TokenKind::String
            | TokenKind::Switch
            | TokenKind::Use
    )
}

pub(super) fn skip_top_level_item(
    tokens: &[Token],
    index: usize,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let mut stack = Vec::<TokenKind>::new();
    let mut cursor = index;
    while let Some(token) = tokens.get(cursor) {
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Ok(cursor + 1);
        }
        if stack.is_empty() && token.kind == TokenKind::LBrace {
            return skip_balanced_delimiter(tokens, cursor, TokenKind::RBrace);
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return unsupported("source declaration has an unmatched closing delimiter");
                };
                if token.kind != expected {
                    return unsupported("source declaration delimiters are not balanced");
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    unsupported("source declaration has no terminator")
}

pub(super) fn skip_balanced_delimiter(
    tokens: &[Token],
    index: usize,
    close_kind: TokenKind,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let open_kind = tokens
        .get(index)
        .map(|token| token.kind)
        .ok_or_else(|| unsupported_source_message("source declaration has no body"))?;
    let mut depth = 0_usize;
    let mut cursor = index;
    while let Some(token) = tokens.get(cursor) {
        if token.kind == open_kind {
            depth = depth
                .checked_add(1)
                .ok_or_else(|| unsupported_source_message("source declaration nesting overflow"))?;
        } else if token.kind == close_kind {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| unsupported_source_message("source declaration body underflow"))?;
            if depth == 0 {
                return Ok(cursor + 1);
            }
        }
        cursor += 1;
    }
    unsupported("source declaration body is not closed")
}

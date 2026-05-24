use lzvm_pil::{
    parse_expression_tokens, Expression, ExpressionKind, FunctionStatement, SourceFile,
    SourceProgramModule, Token, TokenKind,
};

use crate::source_key_directory::SourceKeyDirectoryMetadataError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceFinalScope {
    Air,
    AirGroup,
    Proof,
}

pub(crate) struct SourceFinalCall {
    pub(crate) scope: SourceFinalScope,
    pub(crate) expression: Expression,
    pub(crate) next_index: usize,
}

pub(crate) fn source_final_call_at(
    tokens: &[Token],
    index: usize,
    source: &SourceFile,
) -> Result<Option<SourceFinalCall>, SourceKeyDirectoryMetadataError> {
    if !matches!(
        (tokens.get(index), tokens.get(index + 1)),
        (Some(on), Some(final_token))
            if on.kind == TokenKind::On && final_token.kind == TokenKind::Final
    ) {
        return Ok(None);
    }

    let mut cursor = index + 2;
    if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::LParen)
    {
        let Some(next) = skip_balanced_delimiter(tokens, cursor, TokenKind::RParen) else {
            return Ok(None);
        };
        cursor = next;
    }
    let Some(scope) = source_final_scope(tokens.get(cursor).map(|token| token.kind)) else {
        return Ok(None);
    };

    let call_start = cursor + 1;
    let Some(semicolon_index) = skip_final_call_statement(tokens, call_start) else {
        return Ok(None);
    };
    let (expression, next_index) =
        parse_expression_tokens(tokens, call_start, semicolon_index, source)?;
    if next_index != semicolon_index || !matches!(expression.kind, ExpressionKind::Call { .. }) {
        return Ok(None);
    }
    Ok(Some(SourceFinalCall {
        scope,
        expression,
        next_index: semicolon_index + 1,
    }))
}

pub(crate) fn source_final_statement_call(
    tokens: &[Token],
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<SourceFinalCall>, SourceKeyDirectoryMetadataError> {
    let Some(index) = tokens
        .iter()
        .position(|token| token.start == statement.start)
    else {
        return Ok(None);
    };
    let Some(call) = source_final_call_at(tokens, index, &module.source)? else {
        return Ok(None);
    };
    if tokens
        .get(call.next_index.saturating_sub(1))
        .is_none_or(|token| token.end != statement.end)
    {
        return Ok(None);
    }
    Ok(Some(call))
}

fn source_final_scope(kind: Option<TokenKind>) -> Option<SourceFinalScope> {
    match kind? {
        TokenKind::Air => Some(SourceFinalScope::Air),
        TokenKind::AirGroup => Some(SourceFinalScope::AirGroup),
        TokenKind::Proof => Some(SourceFinalScope::Proof),
        _ => None,
    }
}

fn skip_final_call_statement(tokens: &[Token], index: usize) -> Option<usize> {
    let mut expected = Vec::<TokenKind>::new();
    let mut cursor = index;
    while let Some(token) = tokens.get(cursor) {
        if expected.is_empty() && token.kind == TokenKind::Semicolon {
            return Some(cursor);
        }
        match token.kind {
            TokenKind::LParen => expected.push(TokenKind::RParen),
            TokenKind::LBracket => expected.push(TokenKind::RBracket),
            TokenKind::LBrace => expected.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if expected.pop()? != token.kind {
                    return None;
                }
            }
            TokenKind::EndOfInput => return None,
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn skip_balanced_delimiter(tokens: &[Token], open_index: usize, close: TokenKind) -> Option<usize> {
    let mut expected = vec![close];
    let mut cursor = open_index + 1;
    while let Some(token) = tokens.get(cursor) {
        match token.kind {
            TokenKind::LParen => expected.push(TokenKind::RParen),
            TokenKind::LBracket => expected.push(TokenKind::RBracket),
            TokenKind::LBrace => expected.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if expected.pop()? != token.kind {
                    return None;
                }
                if expected.is_empty() {
                    return Some(cursor + 1);
                }
            }
            TokenKind::EndOfInput => return None,
            _ => {}
        }
        cursor += 1;
    }
    None
}

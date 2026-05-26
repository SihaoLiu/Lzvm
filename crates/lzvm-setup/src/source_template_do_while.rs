use std::collections::BTreeMap;
use std::sync::Arc;

use lzvm_pil::{
    parse_expression_tokens, Expression, FixedFileTemplateValue, FunctionStatement,
    FunctionStatementKind, SourceProgram, SourceProgramModule, Token, TokenKind,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_tokens::matching_closing_token,
};

pub(crate) struct SourceStaticDoWhileLoop {
    pub(crate) body_statements: Arc<[FunctionStatement]>,
    pub(crate) condition: Expression,
}

pub(crate) fn source_static_do_while_loop_with_tokens(
    _program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    _base_values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceStaticDoWhileLoop>, SourceKeyDirectoryMetadataError> {
    if statement.kind != FunctionStatementKind::Do {
        return Ok(None);
    }
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some((_, close_after)) = body_cache.span_token_bounds(tokens, body) else {
        return Ok(None);
    };
    if tokens.get(close_after).map(|token| token.kind) != Some(TokenKind::While) {
        return Ok(None);
    }
    let open = close_after + 1;
    if tokens.get(open).map(|token| token.kind) != Some(TokenKind::LParen) {
        return Ok(None);
    }
    let Some(close) = matching_closing_token(tokens, open, tokens.len()) else {
        return Ok(None);
    };
    let semicolon = close + 1;
    if tokens.get(semicolon).map(|token| token.kind) != Some(TokenKind::Semicolon) {
        return Ok(None);
    }
    let (condition, consumed) = parse_expression_tokens(tokens, open + 1, close, &module.source)?;
    if consumed != close {
        return Ok(None);
    }
    let body_statements = body_cache.body_statements(tokens, body, &module.source)?;
    Ok(Some(SourceStaticDoWhileLoop {
        body_statements,
        condition,
    }))
}

use std::collections::BTreeMap;
use std::sync::Arc;

use lzvm_pil::{
    Expression, FixedFileTemplateValue, FunctionStatement, FunctionStatementKind, SourceProgram,
    SourceProgramModule, Token,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_key_directory::SourceKeyDirectoryMetadataError,
};

pub(crate) const STATIC_WHILE_LOOP_LIMIT: usize = 10_000;

pub(crate) struct SourceStaticWhileLoop {
    pub(crate) body_statements: Arc<[FunctionStatement]>,
    pub(crate) condition: Expression,
}

pub(crate) fn source_static_while_loop_with_tokens(
    _program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    _base_values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceStaticWhileLoop>, SourceKeyDirectoryMetadataError> {
    if statement.kind != FunctionStatementKind::While {
        return Ok(None);
    }
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some(condition) = statement.header_expression.clone() else {
        return Ok(None);
    };
    let body_statements = body_cache.body_statements(tokens, body, &module.source)?;
    Ok(Some(SourceStaticWhileLoop {
        body_statements,
        condition,
    }))
}

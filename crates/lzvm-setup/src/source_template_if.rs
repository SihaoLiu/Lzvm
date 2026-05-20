use std::collections::BTreeMap;

use lzvm_pil::{
    lex_source, parse_function_body_statements, FixedFileTemplateValue, FunctionStatement,
    FunctionStatementKind, SourceProgram, SourceProgramModule,
};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{evaluate_source_static_expression, static_value_truthy},
};

pub(crate) fn source_static_if_body_statements(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Option<Vec<FunctionStatement>>, SourceKeyDirectoryMetadataError> {
    if statement.kind != FunctionStatementKind::If {
        return Ok(None);
    }
    let Some(condition) = statement.header_expression.as_ref() else {
        return Ok(None);
    };
    let Some(condition_value) = evaluate_source_static_expression(program, condition, values)
    else {
        return Ok(None);
    };
    if !static_value_truthy(&condition_value) {
        return Ok(Some(Vec::new()));
    }
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let tokens = lex_source(&module.source.contents).map_err(|source| {
        SourceKeyDirectoryMetadataError::Lex {
            source_name: module.source_name.clone(),
            source,
        }
    })?;
    Ok(Some(parse_function_body_statements(
        &tokens,
        body,
        &module.source,
    )?))
}

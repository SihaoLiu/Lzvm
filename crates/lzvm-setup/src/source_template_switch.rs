use std::collections::BTreeMap;
use std::sync::Arc;

use lzvm_pil::{
    parse_expression_tokens, parse_function_statement_tokens, FixedFileTemplateValue,
    FunctionStatement, FunctionStatementKind, SourceProgram, SourceProgramModule, Token, TokenKind,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_tokens::{
        static_switch_case_value_ranges, static_switch_label_colon, update_static_delimiter_stack,
    },
    source_static_values::evaluate_source_static_expression,
};

pub(crate) fn source_static_switch_body_statements(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Arc<[FunctionStatement]>>, SourceKeyDirectoryMetadataError> {
    if statement.kind != FunctionStatementKind::Switch {
        return Ok(None);
    }
    let Some(condition_expression) = statement.header_expression.as_ref() else {
        return Ok(None);
    };
    let Some(condition) = evaluate_source_static_expression(program, condition_expression, values)
    else {
        return Ok(None);
    };
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some((open_index, close_after)) = body_cache.span_token_bounds(tokens, body) else {
        return Ok(None);
    };
    let Some(body_end) = close_after.checked_sub(1) else {
        return Ok(None);
    };
    let Some(branch) = source_static_switch_branch(
        program,
        module,
        tokens,
        open_index + 1,
        body_end,
        values,
        &condition,
    ) else {
        return Ok(None);
    };
    let Some(branch) = branch else {
        return Ok(Some(Arc::from([])));
    };
    let statements =
        parse_function_statement_tokens(tokens, branch.start, branch.end, &module.source)?;
    Ok(Some(statements.into()))
}

struct SourceSwitchBranch {
    start: usize,
    end: usize,
}

fn source_static_switch_branch(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    condition: &FixedFileTemplateValue,
) -> Option<Option<SourceSwitchBranch>> {
    let mut default_branch = None;
    let mut cursor = start;
    while cursor < end {
        while tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::Semicolon)
        {
            cursor += 1;
        }
        if cursor >= end {
            break;
        }
        let token = tokens.get(cursor)?;
        let colon = match token.kind {
            TokenKind::Case | TokenKind::Default => {
                static_switch_label_colon(tokens, cursor + 1, end)?
            }
            _ => return None,
        };
        let branch_start = colon + 1;
        let branch_end = next_static_switch_label(tokens, branch_start, end).unwrap_or(end);
        if token.kind == TokenKind::Default {
            if default_branch.is_none() {
                default_branch = Some(SourceSwitchBranch {
                    start: branch_start,
                    end: branch_end,
                });
            }
        } else if source_static_switch_case_matches(
            program,
            module,
            tokens,
            cursor + 1,
            colon,
            values,
            condition,
        )? {
            return Some(Some(SourceSwitchBranch {
                start: branch_start,
                end: branch_end,
            }));
        }
        cursor = branch_end;
    }
    Some(default_branch)
}

fn source_static_switch_case_matches(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    condition: &FixedFileTemplateValue,
) -> Option<bool> {
    for (start, end) in static_switch_case_value_ranges(tokens, start, end)? {
        let (expression, consumed) =
            parse_expression_tokens(tokens, start, end, &module.source).ok()?;
        if consumed != end {
            return None;
        }
        if evaluate_source_static_expression(program, &expression, values)? == *condition {
            return Some(true);
        }
    }
    Some(false)
}

fn next_static_switch_label(tokens: &[Token], start: usize, end: usize) -> Option<usize> {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        if stack.is_empty() && matches!(token.kind, TokenKind::Case | TokenKind::Default) {
            return Some(index);
        }
        update_static_delimiter_stack(token.kind, &mut stack)?;
    }
    None
}

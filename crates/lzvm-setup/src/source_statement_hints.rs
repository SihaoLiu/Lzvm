use lzvm_artifacts::expression_info::{HintFieldInfo, HintInfo, HintPayload, HintValueInfo};
use lzvm_artifacts::hint_program::{
    SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT, SOURCE_UNSUPPORTED_ASSIGNMENT_HINT,
    SOURCE_UNSUPPORTED_CALL_HINT, SOURCE_UNSUPPORTED_CONSTRAINT_HINT,
    SOURCE_UNSUPPORTED_STATEMENT_HINT,
};
use lzvm_pil::{lex_source, FunctionStatement, LexError, SourceProgramModule, TokenKind};

pub(crate) fn source_statement_first_token_kind(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<TokenKind>, LexError> {
    let text = &module.source.contents[statement.start..statement.end];
    let tokens = lex_source(text)?;
    Ok(tokens.first().map(|token| token.kind))
}

pub(crate) fn source_statement_contains_assignment_operator(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<bool, LexError> {
    let text = &module.source.contents[statement.start..statement.end];
    let tokens = lex_source(text)?;
    Ok(tokens.iter().any(|token| {
        matches!(
            token.kind,
            TokenKind::Assign
                | TokenKind::ConstrainedAssign
                | TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::Increment
                | TokenKind::Decrement
        )
    }))
}

pub(crate) fn lower_source_lookup_statement(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<HintInfo>, LexError> {
    let Some(name) = source_lookup_hint_name(module, statement)? else {
        return Ok(None);
    };
    let line = module.source.contents[statement.start..statement.end]
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_owned();
    Ok(Some(HintInfo {
        name: name.to_owned(),
        fields: vec![HintFieldInfo {
            name: "line".to_owned(),
            values: vec![HintValueInfo {
                positions: Vec::new(),
                payload: HintPayload::string(line),
            }],
        }],
    }))
}

pub(crate) fn lower_unsupported_source_call_statement(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<HintInfo>, LexError> {
    let Some(name) = source_call_name(module, statement)? else {
        return Ok(None);
    };
    Ok(Some(HintInfo {
        name: SOURCE_UNSUPPORTED_CALL_HINT.to_owned(),
        fields: vec![
            HintFieldInfo {
                name: "name".to_owned(),
                values: vec![HintValueInfo {
                    positions: Vec::new(),
                    payload: HintPayload::string(name),
                }],
            },
            HintFieldInfo {
                name: "line".to_owned(),
                values: vec![HintValueInfo {
                    positions: Vec::new(),
                    payload: HintPayload::string(source_statement_line(module, statement)),
                }],
            },
        ],
    }))
}

pub(crate) fn lower_unsupported_source_assignment_statement(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> HintInfo {
    HintInfo {
        name: SOURCE_UNSUPPORTED_ASSIGNMENT_HINT.to_owned(),
        fields: vec![HintFieldInfo {
            name: "line".to_owned(),
            values: vec![HintValueInfo {
                positions: Vec::new(),
                payload: HintPayload::string(source_statement_line(module, statement)),
            }],
        }],
    }
}

pub(crate) fn lower_unsupported_source_template_statement(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> HintInfo {
    HintInfo {
        name: SOURCE_UNSUPPORTED_STATEMENT_HINT.to_owned(),
        fields: vec![HintFieldInfo {
            name: "line".to_owned(),
            values: vec![HintValueInfo {
                positions: Vec::new(),
                payload: HintPayload::string(source_statement_line(module, statement)),
            }],
        }],
    }
}

pub(crate) fn lower_unsupported_source_constraint_statement(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> HintInfo {
    HintInfo {
        name: SOURCE_UNSUPPORTED_CONSTRAINT_HINT.to_owned(),
        fields: vec![HintFieldInfo {
            name: "line".to_owned(),
            values: vec![HintValueInfo {
                positions: Vec::new(),
                payload: HintPayload::string(source_statement_line(module, statement)),
            }],
        }],
    }
}

fn source_lookup_hint_name(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<&'static str>, LexError> {
    let Some(name) = source_call_name(module, statement)? else {
        return Ok(None);
    };
    match name.as_str() {
        "lookup_proves" => Ok(Some(SOURCE_LOOKUP_PROVES_HINT)),
        "lookup_assumes" => Ok(Some(SOURCE_LOOKUP_ASSUMES_HINT)),
        _ => Ok(None),
    }
}

fn source_call_name(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<String>, LexError> {
    let text = &module.source.contents[statement.start..statement.end];
    let tokens = lex_source(text)?;
    let Some(name) = tokens.first() else {
        return Ok(None);
    };
    let Some(open) = tokens.get(1) else {
        return Ok(None);
    };
    if name.kind != TokenKind::Identifier || open.kind != TokenKind::LParen {
        return Ok(None);
    }
    Ok(Some(name.lexeme.clone()))
}

pub(crate) fn source_statement_line(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> String {
    module.source.contents[statement.start..statement.end]
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_owned()
}

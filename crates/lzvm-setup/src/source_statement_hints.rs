use std::collections::BTreeMap;
use std::path::PathBuf;

use lzvm_artifacts::expression_info::{HintFieldInfo, HintInfo, HintPayload, HintValueInfo};
use lzvm_artifacts::hint_program::{
    SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT, SOURCE_UNSUPPORTED_ASSIGNMENT_HINT,
    SOURCE_UNSUPPORTED_CALL_HINT, SOURCE_UNSUPPORTED_CONSTRAINT_HINT,
    SOURCE_UNSUPPORTED_STATEMENT_HINT,
};
use lzvm_pil::{
    lex_source, parse_expression_tokens, FixedFileTemplateValue, FunctionStatement, LexError,
    SourceFile, SourceProgram, SourceProgramModule, Token, TokenKind,
};

use crate::source_static_values::evaluate_source_static_expression;

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
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Option<HintInfo>, LexError> {
    let Some(name) = source_lookup_hint_name(module, statement)? else {
        return Ok(None);
    };
    let line = source_statement_line(module, statement);
    if let Some(hint) = lower_structured_source_lookup_hint(program, module, values, name, &line)? {
        return Ok(Some(hint));
    }
    Ok(Some(source_lookup_line_hint(name, line)))
}

fn lower_structured_source_lookup_hint(
    program: &SourceProgram,
    module: &SourceProgramModule,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
    line: &str,
) -> Result<Option<HintInfo>, LexError> {
    let tokens = lex_source(line)?;
    let Some(open_index) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
    else {
        return Ok(None);
    };
    let Some(close_index) = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::RParen)
    else {
        return Ok(None);
    };
    let Some(first) = tokens.first() else {
        return Ok(None);
    };
    if first.kind != TokenKind::Identifier || first.lexeme.as_str() != source_lookup_call_name(name)
    {
        return Ok(None);
    }

    let Some(arguments) = top_level_argument_ranges(&tokens, open_index, close_index) else {
        return Ok(None);
    };
    if arguments.len() < 2 {
        return Ok(None);
    }

    let first_argument = split_named_argument(&tokens, arguments[0]);
    if first_argument.name.is_some() {
        return Ok(None);
    }
    let Some(bus_id) = parse_unsigned_argument(
        program,
        module,
        line,
        &tokens,
        first_argument.value_range,
        values,
    ) else {
        return Ok(None);
    };

    let second_argument = split_named_argument(&tokens, arguments[1]);
    if second_argument.name.is_some() {
        return Ok(None);
    }
    let Some(values) = source_lookup_values(line, &tokens, second_argument.value_range) else {
        return Ok(None);
    };

    let mut fields = vec![
        hint_number_field("bus_id", bus_id),
        hint_string_values_field("values", values),
    ];
    for range in arguments.into_iter().skip(2) {
        let argument = split_named_argument(&tokens, range);
        let Some(argument_name) = argument.name else {
            return Ok(None);
        };
        let field_name = match argument_name.as_str() {
            "mul" => "multiplicity",
            "sel" => "selector",
            _ => return Ok(None),
        };
        let Some(value) = token_range_text(line, &tokens, argument.value_range) else {
            return Ok(None);
        };
        fields.push(hint_string_field(field_name, value));
    }

    Ok(Some(HintInfo {
        name: name.to_owned(),
        fields,
    }))
}

fn source_lookup_line_hint(name: &str, line: String) -> HintInfo {
    HintInfo {
        name: name.to_owned(),
        fields: vec![HintFieldInfo {
            name: "line".to_owned(),
            values: vec![HintValueInfo {
                positions: Vec::new(),
                payload: HintPayload::string(line),
            }],
        }],
    }
}

fn source_lookup_call_name(name: &str) -> &'static str {
    match name {
        SOURCE_LOOKUP_PROVES_HINT => "lookup_proves",
        SOURCE_LOOKUP_ASSUMES_HINT => "lookup_assumes",
        _ => "",
    }
}

fn top_level_argument_ranges(
    tokens: &[Token],
    open_index: usize,
    close_index: usize,
) -> Option<Vec<(usize, usize)>> {
    if open_index >= close_index {
        return None;
    }
    let mut ranges = Vec::new();
    let mut start = open_index + 1;
    let mut depth = 0_i32;
    for (index, token) in tokens
        .iter()
        .enumerate()
        .take(close_index)
        .skip(open_index + 1)
    {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket => depth += 1,
            TokenKind::RParen | TokenKind::RBracket => depth -= 1,
            TokenKind::Comma if depth == 0 => {
                if start == index {
                    return None;
                }
                ranges.push((start, index));
                start = index + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return None;
        }
    }
    if depth != 0 {
        return None;
    }
    if start < close_index {
        ranges.push((start, close_index));
    }
    Some(ranges)
}

struct SourceLookupArgument {
    name: Option<String>,
    value_range: (usize, usize),
}

fn split_named_argument(tokens: &[Token], range: (usize, usize)) -> SourceLookupArgument {
    if range.0 + 2 <= range.1
        && tokens[range.0].kind == TokenKind::Identifier
        && tokens[range.0 + 1].kind == TokenKind::Colon
    {
        return SourceLookupArgument {
            name: Some(tokens[range.0].lexeme.clone()),
            value_range: (range.0 + 2, range.1),
        };
    }
    SourceLookupArgument {
        name: None,
        value_range: range,
    }
}

fn parse_unsigned_argument(
    program: &SourceProgram,
    module: &SourceProgramModule,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<u64> {
    if range.0 + 1 != range.1 {
        return evaluate_unsigned_argument(program, module, line, tokens, range, values);
    }
    let token = &tokens[range.0];
    let literal = match token.kind {
        TokenKind::Integer => token.lexeme.replace('_', "").parse::<u64>().ok(),
        TokenKind::HexInteger => u64::from_str_radix(
            token
                .lexeme
                .trim_start_matches("0x")
                .trim_start_matches("0X")
                .replace('_', "")
                .as_str(),
            16,
        )
        .ok(),
        _ => None,
    };
    literal.or_else(|| evaluate_unsigned_argument(program, module, line, tokens, range, values))
}

fn evaluate_unsigned_argument(
    program: &SourceProgram,
    module: &SourceProgramModule,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<u64> {
    let source = SourceFile {
        contents: line.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::new(),
        source_name: module.source_name.clone(),
    };
    let (expression, consumed) = parse_expression_tokens(tokens, range.0, range.1, &source).ok()?;
    if consumed != range.1 {
        return None;
    }
    match evaluate_source_static_expression(program, &expression, values)? {
        FixedFileTemplateValue::Integer(value) => u64::try_from(value).ok(),
        FixedFileTemplateValue::Boolean(value) => Some(u64::from(value)),
        FixedFileTemplateValue::String(_) => None,
    }
}

fn source_lookup_values(
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
) -> Option<Vec<String>> {
    if range.0 >= range.1 {
        return None;
    }
    if tokens[range.0].kind == TokenKind::LBracket
        && range.0 + 1 < range.1
        && tokens[range.1 - 1].kind == TokenKind::RBracket
    {
        let ranges = top_level_argument_ranges(tokens, range.0, range.1 - 1)?;
        return ranges
            .into_iter()
            .map(|value_range| token_range_text(line, tokens, value_range))
            .collect();
    }
    Some(vec![token_range_text(line, tokens, range)?])
}

fn token_range_text(line: &str, tokens: &[Token], range: (usize, usize)) -> Option<String> {
    if range.0 >= range.1 || range.1 > tokens.len() {
        return None;
    }
    let start = tokens[range.0].start;
    let end = tokens[range.1 - 1].end;
    Some(line.get(start..end)?.trim().to_owned())
}

fn hint_number_field(name: &str, value: u64) -> HintFieldInfo {
    HintFieldInfo {
        name: name.to_owned(),
        values: vec![HintValueInfo {
            positions: Vec::new(),
            payload: HintPayload::number(value),
        }],
    }
}

fn hint_string_field(name: &str, value: String) -> HintFieldInfo {
    hint_string_values_field(name, vec![value])
}

fn hint_string_values_field(name: &str, values: Vec<String>) -> HintFieldInfo {
    HintFieldInfo {
        name: name.to_owned(),
        values: values
            .into_iter()
            .map(|value| HintValueInfo {
                positions: Vec::new(),
                payload: HintPayload::string(value),
            })
            .collect(),
    }
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

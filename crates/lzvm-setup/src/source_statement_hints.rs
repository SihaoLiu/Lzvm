use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use lzvm_artifacts::expression_info::{
    CodeOperand, HintFieldInfo, HintInfo, HintPayload, HintValueInfo,
};
use lzvm_artifacts::hint_program::{
    SOURCE_ASSIGNMENT_CHECK_HINT, SOURCE_LOOKUP_ASSUMES_HINT, SOURCE_LOOKUP_PROVES_HINT,
    SOURCE_UNSUPPORTED_ASSIGNMENT_HINT, SOURCE_UNSUPPORTED_CALL_HINT,
    SOURCE_UNSUPPORTED_CONSTRAINT_HINT, SOURCE_UNSUPPORTED_STATEMENT_HINT,
};
use lzvm_field::MODULUS;
use lzvm_pil::{
    lex_source, parse_expression_tokens, BinaryOperator, Expression, ExpressionKind,
    FixedFileTemplateValue, FunctionStatement, LexError, SourceFile, SourceProgram,
    SourceProgramModule, Token, TokenKind, UnaryOperator,
};

use crate::{
    source_constraint_lowering::SourceExpressionAliases,
    source_expression_strings::source_expression_string_call_value,
    source_scalar_slots::SourceScalarSlots,
    source_static_values::{
        evaluate_source_static_expression, source_static_array_element, source_static_array_values,
        static_value_integer,
    },
};

#[derive(Clone)]
pub(crate) enum SourceExpressionArrayAlias {
    Name(String),
    Values(Vec<Expression>),
}

pub(crate) type SourceExpressionArrayAliases = BTreeMap<String, SourceExpressionArrayAlias>;

pub(crate) fn source_statement_first_token_kind(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<TokenKind>, LexError> {
    let text = &module.source.contents[statement.start..statement.end];
    let tokens = lex_source(text)?;
    Ok(tokens.first().map(|token| token.kind))
}

pub(crate) fn source_statement_is_source_directive(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<bool, LexError> {
    let text = &module.source.contents[statement.start..statement.end];
    let tokens = lex_source(text)?;
    let Some(first) = tokens.first() else {
        return Ok(false);
    };
    if source_directive_token(first.kind) {
        return Ok(true);
    }
    if matches!(first.kind, TokenKind::Public | TokenKind::Private) {
        return Ok(tokens
            .get(1)
            .is_some_and(|token| source_directive_token(token.kind)));
    }
    Ok(false)
}

fn source_directive_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Include | TokenKind::Require | TokenKind::Use
    )
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
    inputs: &SourceLookupInputs<'_>,
    statement: &FunctionStatement,
) -> Result<Option<HintInfo>, LexError> {
    let Some(name) = source_lookup_hint_name(inputs.module, statement)? else {
        return Ok(None);
    };
    let line = source_statement_line(inputs.module, statement);
    if let Some(hint) = lower_structured_source_lookup_hint(inputs, name, &line)? {
        return Ok(Some(hint));
    }
    Ok(Some(source_lookup_line_hint(name, line)))
}

pub(crate) fn lower_source_assignment_statement(
    inputs: &SourceLookupInputs<'_>,
    statement: &FunctionStatement,
) -> Result<Option<HintInfo>, LexError> {
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(None);
    };
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return Ok(None);
    };
    if *op != BinaryOperator::Assign {
        return Ok(None);
    }

    let line = source_statement_line(inputs.module, statement);
    let context = SourceLookupLowering {
        program: inputs.program,
        module: inputs.module,
        line: &line,
        tokens: &[],
        values: inputs.values,
        expression_aliases: inputs.expression_aliases,
        expression_array_aliases: inputs.expression_array_aliases,
        scalar_slots: inputs.scalar_slots,
        opening_points: inputs.opening_points,
    };
    let target = source_lookup_scalar_operand(&context, left, 0)
        .and_then(|operand| source_assignment_target_payload(operand, inputs.opening_points));
    let value = source_assignment_expression_values(&context, right);
    let (Some(target), Some(value)) = (target, value) else {
        return Ok(None);
    };
    let value_field_name = if value.len() == 1 {
        "value"
    } else {
        "expression"
    };

    Ok(Some(HintInfo {
        name: SOURCE_ASSIGNMENT_CHECK_HINT.to_owned(),
        fields: vec![
            HintFieldInfo {
                name: "target".to_owned(),
                values: vec![HintValueInfo {
                    positions: Vec::new(),
                    payload: target,
                }],
            },
            HintFieldInfo {
                name: value_field_name.to_owned(),
                values: value,
            },
        ],
    }))
}

pub(crate) fn lower_source_annotation_statement(
    inputs: &SourceLookupInputs<'_>,
    statement: &FunctionStatement,
) -> Result<Option<HintInfo>, LexError> {
    let line = source_statement_line(inputs.module, statement);
    let tokens = lex_source(&line)?;
    let Some(name) = source_annotation_name(&tokens) else {
        return Ok(None);
    };
    let context = SourceLookupLowering {
        program: inputs.program,
        module: inputs.module,
        line: &line,
        tokens: &tokens,
        values: inputs.values,
        expression_aliases: inputs.expression_aliases,
        expression_array_aliases: inputs.expression_array_aliases,
        scalar_slots: inputs.scalar_slots,
        opening_points: inputs.opening_points,
    };
    let Some(fields) = source_annotation_fields(&context) else {
        return Ok(None);
    };
    Ok(Some(HintInfo {
        name: name.to_owned(),
        fields,
    }))
}

pub(crate) fn source_lookup_statement_expressions(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<Vec<Expression>>, LexError> {
    let Some(name) = source_lookup_hint_name(module, statement)? else {
        return Ok(None);
    };
    let line = source_statement_line(module, statement);
    let tokens = lex_source(&line)?;
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
    let call_name = first.lexeme.as_str();
    if first.kind != TokenKind::Identifier || !source_lookup_call_matches(name, call_name) {
        return Ok(None);
    }

    let Some(arguments) = top_level_argument_ranges(&tokens, open_index, close_index) else {
        return Ok(None);
    };
    if arguments.len() < 2 {
        return Ok(None);
    }

    let second_argument = split_named_argument(&tokens, arguments[1]);
    match second_argument.name.as_deref() {
        None | Some("expressions") => {}
        _ => return Ok(None),
    }
    let mut expressions = Vec::new();
    if !source_lookup_value_expressions(
        module,
        &line,
        &tokens,
        second_argument.value_range,
        &mut expressions,
    ) {
        return Ok(None);
    }

    for (positional_index, range) in arguments.into_iter().skip(2).enumerate() {
        let argument = split_named_argument(&tokens, range);
        let Some(_) = source_lookup_extra_field(name, call_name, &argument, positional_index)
        else {
            return Ok(None);
        };
        let Some(value_range) = source_lookup_argument_value_range(&argument) else {
            return Ok(None);
        };
        if !source_lookup_value_expressions(module, &line, &tokens, value_range, &mut expressions) {
            return Ok(None);
        }
    }

    Ok(Some(expressions))
}

fn source_lookup_value_expressions(
    module: &SourceProgramModule,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
    expressions: &mut Vec<Expression>,
) -> bool {
    if range.0 >= range.1 {
        return false;
    }
    if tokens[range.0].kind == TokenKind::LBracket
        && range.0 + 1 < range.1
        && tokens[range.1 - 1].kind == TokenKind::RBracket
    {
        let Some(ranges) = top_level_argument_ranges(tokens, range.0, range.1 - 1) else {
            return false;
        };
        for value_range in ranges {
            if let Some(expression) =
                source_lookup_spread_expression(module, line, tokens, value_range)
            {
                expressions.push(expression);
                continue;
            }
            let Some(expression) =
                parse_source_lookup_expression(module, line, tokens, value_range)
            else {
                return false;
            };
            expressions.push(expression);
        }
        return true;
    }

    let Some(expression) = parse_source_lookup_expression(module, line, tokens, range) else {
        return false;
    };
    expressions.push(expression);
    true
}

fn source_lookup_spread_expression(
    module: &SourceProgramModule,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
) -> Option<Expression> {
    let name = source_lookup_spread_name(module, line, tokens, range)?;
    Some(Expression {
        kind: ExpressionKind::Name(name),
        source_name: module.source_name.clone(),
        start: tokens[range.0].start,
        end: tokens[range.1 - 1].end,
    })
}

pub(crate) struct SourceLookupInputs<'a> {
    pub(crate) program: &'a SourceProgram,
    pub(crate) module: &'a SourceProgramModule,
    pub(crate) values: &'a BTreeMap<String, FixedFileTemplateValue>,
    pub(crate) expression_aliases: &'a SourceExpressionAliases,
    pub(crate) expression_array_aliases: &'a SourceExpressionArrayAliases,
    pub(crate) scalar_slots: &'a SourceScalarSlots,
    pub(crate) opening_points: &'a [i64],
}

fn lower_structured_source_lookup_hint(
    inputs: &SourceLookupInputs<'_>,
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
    let call_name = first.lexeme.as_str();
    if first.kind != TokenKind::Identifier || !source_lookup_call_matches(name, call_name) {
        return Ok(None);
    }

    let Some(arguments) = top_level_argument_ranges(&tokens, open_index, close_index) else {
        return Ok(None);
    };
    if arguments.len() < 2 {
        return Ok(None);
    }
    let context = SourceLookupLowering {
        program: inputs.program,
        module: inputs.module,
        line,
        tokens: &tokens,
        values: inputs.values,
        expression_aliases: inputs.expression_aliases,
        expression_array_aliases: inputs.expression_array_aliases,
        scalar_slots: inputs.scalar_slots,
        opening_points: inputs.opening_points,
    };

    let first_argument = split_named_argument(&tokens, arguments[0]);
    let Some(bus_id_range) = source_lookup_bus_id_argument_value_range(&first_argument) else {
        return Ok(None);
    };
    let Some(bus_id) = parse_unsigned_argument(
        inputs.program,
        inputs.module,
        line,
        &tokens,
        bus_id_range,
        inputs.values,
    ) else {
        return Ok(None);
    };

    let second_argument = split_named_argument(&tokens, arguments[1]);
    match second_argument.name.as_deref() {
        None | Some("expressions") => {}
        _ => return Ok(None),
    }
    let Some(lookup_values) = source_lookup_values(&context, second_argument.value_range) else {
        return Ok(None);
    };

    let mut fields = vec![hint_number_field("bus_id", bus_id)];
    let needs_value_lengths = lookup_values.needs_lengths();
    if needs_value_lengths {
        let Some(length_values) =
            source_lookup_value_length_values(&lookup_values.component_lengths)
        else {
            return Ok(None);
        };
        fields.push(HintFieldInfo {
            name: "values".to_owned(),
            values: lookup_values.values,
        });
        fields.push(HintFieldInfo {
            name: "value_lengths".to_owned(),
            values: length_values,
        });
    } else {
        fields.push(HintFieldInfo {
            name: "values".to_owned(),
            values: lookup_values.values,
        });
    }
    for (positional_index, range) in arguments.into_iter().skip(2).enumerate() {
        let argument = split_named_argument(&tokens, range);
        let Some(field) = source_lookup_extra_field(name, call_name, &argument, positional_index)
        else {
            return Ok(None);
        };
        let Some(value_range) = source_lookup_argument_value_range(&argument) else {
            return Ok(None);
        };
        let values = match field.value_kind {
            SourceLookupFieldValueKind::Dynamic => {
                source_lookup_dynamic_field_values(&context, value_range)
            }
            SourceLookupFieldValueKind::Static => {
                source_lookup_static_value(&context, value_range).map(|value| vec![value])
            }
        };
        let Some(values) = values else {
            return Ok(None);
        };
        fields.push(HintFieldInfo {
            name: field.name.to_owned(),
            values,
        });
    }

    Ok(Some(HintInfo {
        name: name.to_owned(),
        fields,
    }))
}

pub(crate) fn lower_structured_source_lookup_line(
    inputs: &SourceLookupInputs<'_>,
    name: &str,
    line: &str,
) -> Result<Option<HintInfo>, LexError> {
    lower_structured_source_lookup_hint(inputs, name, line)
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

fn source_lookup_call_matches(hint_name: &str, call_name: &str) -> bool {
    match hint_name {
        SOURCE_LOOKUP_PROVES_HINT => matches!(
            call_name,
            "lookup_proves"
                | "permutation_proves"
                | "direct_update_proves"
                | "direct_global_update_proves"
        ),
        SOURCE_LOOKUP_ASSUMES_HINT => matches!(
            call_name,
            "lookup_assumes"
                | "permutation_assumes"
                | "direct_update_assumes"
                | "direct_global_update_assumes"
        ),
        _ => false,
    }
}

fn source_annotation_name(tokens: &[Token]) -> Option<&str> {
    let name = tokens.first()?;
    (name.kind == TokenKind::AtIdentifier).then_some(name.lexeme.as_str())
}

fn source_annotation_fields(context: &SourceLookupLowering<'_>) -> Option<Vec<HintFieldInfo>> {
    let second = context.tokens.get(1)?;
    match second.kind {
        TokenKind::LBrace => source_annotation_object_fields(context),
        TokenKind::LBracket => source_annotation_array_field(context),
        _ => source_annotation_value_field(context),
    }
}

fn source_annotation_object_fields(
    context: &SourceLookupLowering<'_>,
) -> Option<Vec<HintFieldInfo>> {
    let close_index = context
        .tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::RBrace)?;
    if close_index + 1 != context.tokens.len() {
        return None;
    }
    let ranges = top_level_argument_ranges(context.tokens, 1, close_index)?;
    let mut fields = Vec::new();
    for range in ranges {
        let argument = split_named_argument(context.tokens, range);
        let field_name = source_annotation_field_name(context, &argument)?;
        let value_range = source_lookup_argument_value_range(&argument)?;
        let values = source_lookup_values(context, value_range)?.values;
        fields.push(HintFieldInfo {
            name: field_name,
            values,
        });
    }
    Some(fields)
}

fn source_annotation_array_field(context: &SourceLookupLowering<'_>) -> Option<Vec<HintFieldInfo>> {
    if !context
        .tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::RBracket)
    {
        return None;
    }
    Some(vec![HintFieldInfo {
        name: "values".to_owned(),
        values: source_lookup_values(context, (1, context.tokens.len()))?.values,
    }])
}

fn source_annotation_value_field(context: &SourceLookupLowering<'_>) -> Option<Vec<HintFieldInfo>> {
    Some(vec![HintFieldInfo {
        name: "value".to_owned(),
        values: source_lookup_values(context, (1, context.tokens.len()))?.values,
    }])
}

fn source_annotation_field_name(
    context: &SourceLookupLowering<'_>,
    argument: &SourceLookupArgument,
) -> Option<String> {
    if let Some(name) = argument.name.as_ref() {
        return Some(name.clone());
    }
    source_lookup_bare_name(
        context.module,
        context.line,
        context.tokens,
        argument.value_range,
    )
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
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => depth -= 1,
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
    name_range: Option<(usize, usize)>,
    value_range: (usize, usize),
}

#[derive(Clone, Copy)]
enum SourceLookupFieldValueKind {
    Dynamic,
    Static,
}

#[derive(Clone, Copy)]
struct SourceLookupExtraField {
    name: &'static str,
    value_kind: SourceLookupFieldValueKind,
}

struct SourceLookupLowering<'a> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    line: &'a str,
    tokens: &'a [Token],
    values: &'a BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &'a SourceExpressionAliases,
    expression_array_aliases: &'a SourceExpressionArrayAliases,
    scalar_slots: &'a SourceScalarSlots,
    opening_points: &'a [i64],
}

#[derive(Default)]
struct SourceLookupValues {
    values: Vec<HintValueInfo>,
    component_lengths: Vec<usize>,
}

impl SourceLookupValues {
    fn push_component(&mut self, values: Vec<HintValueInfo>) -> Option<()> {
        if values.is_empty() {
            return None;
        }
        self.component_lengths.push(values.len());
        self.values.extend(values);
        Some(())
    }

    fn extend(&mut self, values: SourceLookupValues) {
        self.component_lengths.extend(values.component_lengths);
        self.values.extend(values.values);
    }

    fn needs_lengths(&self) -> bool {
        self.component_lengths.iter().any(|length| *length != 1)
    }
}

fn split_named_argument(tokens: &[Token], range: (usize, usize)) -> SourceLookupArgument {
    if range.0 + 2 <= range.1
        && tokens[range.0].kind == TokenKind::Identifier
        && tokens[range.0 + 1].kind == TokenKind::Colon
    {
        return SourceLookupArgument {
            name: Some(tokens[range.0].lexeme.clone()),
            name_range: Some((range.0, range.0 + 1)),
            value_range: (range.0 + 2, range.1),
        };
    }
    SourceLookupArgument {
        name: None,
        name_range: None,
        value_range: range,
    }
}

fn source_lookup_argument_value_range(argument: &SourceLookupArgument) -> Option<(usize, usize)> {
    if argument.value_range.0 < argument.value_range.1 {
        return Some(argument.value_range);
    }
    argument.name_range
}

fn source_lookup_bus_id_argument_value_range(
    argument: &SourceLookupArgument,
) -> Option<(usize, usize)> {
    match argument.name.as_deref() {
        None | Some("opid") | Some("bus_id") => source_lookup_argument_value_range(argument),
        _ => None,
    }
}

fn source_lookup_extra_field(
    hint_name: &str,
    call_name: &str,
    argument: &SourceLookupArgument,
    positional_index: usize,
) -> Option<SourceLookupExtraField> {
    if let Some(name) = argument.name.as_deref() {
        return source_lookup_named_extra_field(name);
    }
    source_lookup_positional_extra_field(hint_name, call_name, positional_index)
}

fn source_lookup_named_extra_field(name: &str) -> Option<SourceLookupExtraField> {
    match name {
        "mul" => Some(SourceLookupExtraField {
            name: "multiplicity",
            value_kind: SourceLookupFieldValueKind::Dynamic,
        }),
        "sel" => Some(SourceLookupExtraField {
            name: "selector",
            value_kind: SourceLookupFieldValueKind::Dynamic,
        }),
        "table_id" => Some(source_lookup_static_extra_field("table_id")),
        "bus_type" => Some(source_lookup_static_extra_field("bus_type")),
        "name" => Some(source_lookup_static_extra_field("name")),
        "surname" => Some(source_lookup_static_extra_field("surname")),
        _ => None,
    }
}

fn source_lookup_positional_extra_field(
    hint_name: &str,
    call_name: &str,
    positional_index: usize,
) -> Option<SourceLookupExtraField> {
    match call_name {
        "lookup_proves" => match positional_index {
            0 => Some(SourceLookupExtraField {
                name: "multiplicity",
                value_kind: SourceLookupFieldValueKind::Dynamic,
            }),
            1 => Some(source_lookup_static_extra_field("name")),
            2 => Some(source_lookup_static_extra_field("surname")),
            3 => Some(source_lookup_static_extra_field("table_id")),
            _ => None,
        },
        "lookup_assumes" => source_lookup_positional_selector_field(positional_index, false),
        "permutation_proves"
        | "permutation_assumes"
        | "direct_update_proves"
        | "direct_update_assumes"
        | "direct_global_update_proves"
        | "direct_global_update_assumes" => {
            source_lookup_positional_selector_field(positional_index, true)
        }
        _ if hint_name == SOURCE_LOOKUP_PROVES_HINT => match positional_index {
            0 => Some(SourceLookupExtraField {
                name: "multiplicity",
                value_kind: SourceLookupFieldValueKind::Dynamic,
            }),
            _ => None,
        },
        _ if hint_name == SOURCE_LOOKUP_ASSUMES_HINT => {
            source_lookup_positional_selector_field(positional_index, false)
        }
        _ => None,
    }
}

fn source_lookup_positional_selector_field(
    positional_index: usize,
    has_bus_type: bool,
) -> Option<SourceLookupExtraField> {
    match (positional_index, has_bus_type) {
        (0, _) => Some(SourceLookupExtraField {
            name: "selector",
            value_kind: SourceLookupFieldValueKind::Dynamic,
        }),
        (1, true) => Some(source_lookup_static_extra_field("bus_type")),
        (1, false) | (2, true) => Some(source_lookup_static_extra_field("name")),
        (2, false) | (3, true) => Some(source_lookup_static_extra_field("surname")),
        _ => None,
    }
}

fn source_lookup_static_extra_field(name: &'static str) -> SourceLookupExtraField {
    SourceLookupExtraField {
        name,
        value_kind: SourceLookupFieldValueKind::Static,
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
    context: &SourceLookupLowering<'_>,
    range: (usize, usize),
) -> Option<SourceLookupValues> {
    if range.0 >= range.1 {
        return None;
    }
    if context.tokens[range.0].kind == TokenKind::LBracket
        && range.0 + 1 < range.1
        && context.tokens[range.1 - 1].kind == TokenKind::RBracket
    {
        let ranges = top_level_argument_ranges(context.tokens, range.0, range.1 - 1)?;
        let mut values = SourceLookupValues::default();
        for value_range in ranges {
            if let Some(name) =
                source_lookup_spread_name(context.module, context.line, context.tokens, value_range)
            {
                values.extend(source_lookup_spread_values(context, &name)?);
            } else {
                values
                    .push_component(source_lookup_value_expression_values(context, value_range)?)?;
            }
        }
        return Some(values);
    }
    if let Some(name) = source_lookup_bare_name(context.module, context.line, context.tokens, range)
    {
        if let Some(values) = source_lookup_spread_values(context, &name) {
            return Some(values);
        }
    }
    let mut values = SourceLookupValues::default();
    values.push_component(source_lookup_value_expression_values(context, range)?)?;
    Some(values)
}

fn source_lookup_bare_name(
    module: &SourceProgramModule,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
) -> Option<String> {
    let expression = parse_source_lookup_expression(module, line, tokens, range)?;
    match &strip_group_expression(&expression).kind {
        ExpressionKind::Name(name) => Some(name.clone()),
        _ => None,
    }
}

fn source_lookup_spread_name(
    module: &SourceProgramModule,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
) -> Option<String> {
    if range.0 + 1 >= range.1 || tokens[range.0].kind != TokenKind::Ellipsis {
        return None;
    }
    let expression = parse_source_lookup_expression(module, line, tokens, (range.0 + 1, range.1))?;
    match &strip_group_expression(&expression).kind {
        ExpressionKind::Name(name) => Some(name.clone()),
        _ => None,
    }
}

fn source_lookup_spread_values(
    context: &SourceLookupLowering<'_>,
    name: &str,
) -> Option<SourceLookupValues> {
    if let Some(alias) = context.expression_array_aliases.get(name) {
        let mut resolving_aliases = BTreeSet::new();
        return source_lookup_spread_alias_values(context, alias, &mut resolving_aliases);
    }
    source_lookup_named_spread_values(context, name)
}

fn source_lookup_spread_alias_values(
    context: &SourceLookupLowering<'_>,
    alias: &SourceExpressionArrayAlias,
    resolving_aliases: &mut BTreeSet<String>,
) -> Option<SourceLookupValues> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if let Some(next_alias) = context.expression_array_aliases.get(name) {
                if !resolving_aliases.insert(name.clone()) {
                    return None;
                }
                let values =
                    source_lookup_spread_alias_values(context, next_alias, resolving_aliases);
                resolving_aliases.remove(name);
                return values;
            }
            source_lookup_named_spread_values(context, name)
        }
        SourceExpressionArrayAlias::Values(expressions) => {
            let mut values = SourceLookupValues::default();
            for expression in expressions {
                values.push_component(source_assignment_expression_values(context, expression)?)?;
            }
            Some(values)
        }
    }
}

fn source_lookup_named_spread_values(
    context: &SourceLookupLowering<'_>,
    name: &str,
) -> Option<SourceLookupValues> {
    if let Some(values) = source_static_array_values(context.values, name) {
        let mut out = SourceLookupValues::default();
        for value in values {
            out.push_component(vec![HintValueInfo {
                positions: Vec::new(),
                payload: hint_payload_from_static_value(value)?,
            }])?;
        }
        return Some(out);
    }
    let mut out = SourceLookupValues::default();
    for operand in context.scalar_slots.operand_elements_at(name, 0).ok()? {
        out.push_component(vec![HintValueInfo {
            positions: Vec::new(),
            payload: hint_payload_from_code_operand(operand, context.opening_points)?,
        }])?;
    }
    Some(out)
}

fn source_lookup_value_length_values(lengths: &[usize]) -> Option<Vec<HintValueInfo>> {
    lengths
        .iter()
        .map(|length| {
            Some(HintValueInfo {
                positions: Vec::new(),
                payload: HintPayload::number(u64::try_from(*length).ok()?),
            })
        })
        .collect()
}

fn source_lookup_value_expression_values(
    context: &SourceLookupLowering<'_>,
    range: (usize, usize),
) -> Option<Vec<HintValueInfo>> {
    let expression =
        parse_source_lookup_expression(context.module, context.line, context.tokens, range)?;
    source_assignment_expression_values(context, &expression)
}

fn source_lookup_value_from_expression(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
) -> Option<HintValueInfo> {
    Some(HintValueInfo {
        positions: Vec::new(),
        payload: source_lookup_value_payload_from_expression(context, expression)?,
    })
}

fn source_lookup_dynamic_field_values(
    context: &SourceLookupLowering<'_>,
    range: (usize, usize),
) -> Option<Vec<HintValueInfo>> {
    let expression =
        parse_source_lookup_expression(context.module, context.line, context.tokens, range)?;
    source_assignment_expression_values(context, &expression)
}

fn source_assignment_expression_values(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
) -> Option<Vec<HintValueInfo>> {
    let mut resolving_aliases = BTreeSet::new();
    source_assignment_expression_values_inner(context, expression, &mut resolving_aliases)
}

fn source_assignment_expression_values_inner(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
    resolving_aliases: &mut BTreeSet<String>,
) -> Option<Vec<HintValueInfo>> {
    let expression = strip_group_expression(expression);
    if let ExpressionKind::Binary { op, left, right } = &expression.kind {
        let op = source_assignment_binary_operator(*op)?;
        let mut values =
            source_assignment_expression_values_inner(context, left, resolving_aliases)?;
        values.extend(source_assignment_expression_values_inner(
            context,
            right,
            resolving_aliases,
        )?);
        values.push(HintValueInfo {
            positions: Vec::new(),
            payload: HintPayload::string(op),
        });
        return Some(values);
    }
    if let ExpressionKind::Unary { op, expr } = &expression.kind {
        match op {
            UnaryOperator::Plus => {
                return source_assignment_expression_values_inner(context, expr, resolving_aliases);
            }
            UnaryOperator::Minus => {
                let mut values = vec![HintValueInfo {
                    positions: Vec::new(),
                    payload: HintPayload::number(0),
                }];
                values.extend(source_assignment_expression_values_inner(
                    context,
                    expr,
                    resolving_aliases,
                )?);
                values.push(HintValueInfo {
                    positions: Vec::new(),
                    payload: HintPayload::string("sub"),
                });
                return Some(values);
            }
            UnaryOperator::Not => {
                let mut values =
                    source_assignment_expression_values_inner(context, expr, resolving_aliases)?;
                values.push(HintValueInfo {
                    positions: Vec::new(),
                    payload: HintPayload::string("not"),
                });
                return Some(values);
            }
            _ => return None,
        }
    }
    if let ExpressionKind::Name(name) = &expression.kind {
        if let Some(alias) = context.expression_aliases.get(name) {
            if !resolving_aliases.insert(name.clone()) {
                return None;
            }
            let values =
                source_assignment_expression_values_inner(context, alias, resolving_aliases);
            resolving_aliases.remove(name);
            return values;
        }
    }

    Some(vec![source_lookup_value_from_expression(
        context, expression,
    )?])
}

fn source_assignment_binary_operator(op: BinaryOperator) -> Option<&'static str> {
    match op {
        BinaryOperator::Power => Some("pow"),
        BinaryOperator::Add => Some("add"),
        BinaryOperator::Subtract => Some("sub"),
        BinaryOperator::Multiply => Some("mul"),
        BinaryOperator::Divide | BinaryOperator::Backslash => Some("div"),
        BinaryOperator::Modulo => Some("mod"),
        BinaryOperator::ShiftLeft => Some("shl"),
        BinaryOperator::ShiftRight => Some("shr"),
        BinaryOperator::BitAnd => Some("bitand"),
        BinaryOperator::BitXor => Some("bitxor"),
        BinaryOperator::BitOr => Some("bitor"),
        BinaryOperator::LogicalAnd => Some("and"),
        BinaryOperator::LogicalOr => Some("or"),
        BinaryOperator::Less => Some("lt"),
        BinaryOperator::LessEqual => Some("le"),
        BinaryOperator::Greater => Some("gt"),
        BinaryOperator::GreaterEqual => Some("ge"),
        BinaryOperator::EqualEqual => Some("eq"),
        BinaryOperator::NotEqual => Some("ne"),
        _ => None,
    }
}

fn source_lookup_static_value(
    context: &SourceLookupLowering<'_>,
    range: (usize, usize),
) -> Option<HintValueInfo> {
    Some(HintValueInfo {
        positions: Vec::new(),
        payload: source_lookup_static_value_payload(context, range)?,
    })
}

fn source_lookup_static_value_payload(
    context: &SourceLookupLowering<'_>,
    range: (usize, usize),
) -> Option<HintPayload> {
    let expression =
        parse_source_lookup_expression(context.module, context.line, context.tokens, range)?;
    hint_payload_from_static_value(evaluate_source_static_expression(
        context.program,
        &expression,
        context.values,
    )?)
}

fn source_lookup_value_payload_from_expression(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
) -> Option<HintPayload> {
    if let Some(value) = source_lookup_static_array_element(context, expression) {
        return hint_payload_from_static_value(value);
    }
    if let Some(value) = source_expression_string_call_value(
        context.program,
        expression,
        context.values,
        context.expression_aliases,
        context.expression_array_aliases,
    ) {
        return Some(HintPayload::string(value));
    }
    if let Some(value) =
        evaluate_source_static_expression(context.program, expression, context.values)
    {
        return hint_payload_from_static_value(value);
    }
    let operand = source_lookup_scalar_operand(context, expression, 0)?;
    hint_payload_from_code_operand(operand, context.opening_points)
}

fn source_lookup_static_array_element(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
) -> Option<FixedFileTemplateValue> {
    let ExpressionKind::Index { target, index } = &strip_group_expression(expression).kind else {
        return None;
    };
    let ExpressionKind::Name(name) = &strip_group_expression(target).kind else {
        return None;
    };
    let index =
        usize::try_from(source_lookup_index(context.program, index, context.values)?).ok()?;
    source_static_array_element(context.values, name, index)
}

fn hint_payload_from_static_value(value: FixedFileTemplateValue) -> Option<HintPayload> {
    match value {
        FixedFileTemplateValue::Integer(value) => {
            Some(HintPayload::number(canonical_hint_number(value)?))
        }
        FixedFileTemplateValue::Boolean(value) => Some(HintPayload::number(u64::from(value))),
        FixedFileTemplateValue::String(value) => Some(HintPayload::string(value)),
    }
}

fn canonical_hint_number_from_value(value: FixedFileTemplateValue) -> Option<u64> {
    match value {
        FixedFileTemplateValue::Integer(value) => canonical_hint_number(value),
        FixedFileTemplateValue::Boolean(value) => Some(u64::from(value)),
        FixedFileTemplateValue::String(_) => None,
    }
}

fn parse_source_lookup_expression(
    module: &SourceProgramModule,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
) -> Option<Expression> {
    let source = SourceFile {
        contents: line.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::new(),
        source_name: module.source_name.clone(),
    };
    let (expression, consumed) = parse_expression_tokens(tokens, range.0, range.1, &source).ok()?;
    (consumed == range.1).then_some(expression)
}

fn source_lookup_scalar_operand(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
    row_offset: i64,
) -> Option<CodeOperand> {
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    source_lookup_scalar_operand_inner(
        context,
        expression,
        row_offset,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
    )
}

fn source_lookup_scalar_operand_inner(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
    row_offset: i64,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<CodeOperand> {
    if let Some(value) =
        evaluate_source_static_expression(context.program, expression, context.values)
    {
        return Some(CodeOperand::number(
            canonical_hint_number_from_value(value)?,
            1,
        ));
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => {
            if let Some(alias) = context.expression_aliases.get(name) {
                if !resolving_aliases.insert(name.clone()) {
                    return None;
                }
                let operand = source_lookup_scalar_operand_inner(
                    context,
                    alias,
                    row_offset,
                    resolving_aliases,
                    resolving_array_aliases,
                );
                resolving_aliases.remove(name);
                return operand;
            }
            if row_offset == 0 {
                context.scalar_slots.operand(name).ok()
            } else {
                context.scalar_slots.operand_at(name, row_offset).ok()
            }
        }
        ExpressionKind::Index { .. } => {
            let (name, index_expressions) =
                source_lookup_index_chain(strip_group_expression(expression))?;
            let indices = index_expressions
                .iter()
                .map(|index| source_lookup_index(context.program, index, context.values))
                .collect::<Option<Vec<_>>>()?;
            if let Some(alias) = context.expression_array_aliases.get(name) {
                let element = source_lookup_array_alias_path_element(
                    alias,
                    &indices,
                    context.expression_array_aliases,
                    resolving_array_aliases,
                )?;
                return match element {
                    SourceLookupArrayAliasElement::Expression(expression) => {
                        source_lookup_scalar_operand_inner(
                            context,
                            expression,
                            row_offset,
                            resolving_aliases,
                            resolving_array_aliases,
                        )
                    }
                    SourceLookupArrayAliasElement::NamedArray(name) => context
                        .scalar_slots
                        .operand_indices_at(name, &indices, row_offset)
                        .ok(),
                };
            }
            context
                .scalar_slots
                .operand_indices_at(name, &indices, row_offset)
                .ok()
        }
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => {
            let signed_offset =
                source_lookup_row_offset_value(context.program, offset, *prior, context.values)?;
            let combined_offset = row_offset.checked_add(signed_offset)?;
            source_lookup_scalar_operand_inner(
                context,
                target,
                combined_offset,
                resolving_aliases,
                resolving_array_aliases,
            )
        }
        _ => None,
    }
}

enum SourceLookupArrayAliasElement<'a> {
    Expression(&'a Expression),
    NamedArray(&'a str),
}

fn source_lookup_array_alias_path_element<'a>(
    alias: &'a SourceExpressionArrayAlias,
    indices: &[u32],
    expression_array_aliases: &'a SourceExpressionArrayAliases,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceLookupArrayAliasElement<'a>> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if let Some(next_alias) = expression_array_aliases.get(name) {
                if !resolving_array_aliases.insert(name.clone()) {
                    return None;
                }
                let element = source_lookup_array_alias_path_element(
                    next_alias,
                    indices,
                    expression_array_aliases,
                    resolving_array_aliases,
                );
                resolving_array_aliases.remove(name);
                return element;
            }
            Some(SourceLookupArrayAliasElement::NamedArray(name))
        }
        SourceExpressionArrayAlias::Values(expressions) => source_lookup_expression_array_element(
            expressions,
            indices,
            expression_array_aliases,
            resolving_array_aliases,
        ),
    }
}

fn source_lookup_expression_array_element<'a>(
    expressions: &'a [Expression],
    indices: &[u32],
    expression_array_aliases: &'a SourceExpressionArrayAliases,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceLookupArrayAliasElement<'a>> {
    let (index, rest) = indices.split_first()?;
    let expression = expressions.get(usize::try_from(*index).ok()?)?;
    if rest.is_empty() {
        return Some(SourceLookupArrayAliasElement::Expression(expression));
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Array(expressions) => source_lookup_expression_array_element(
            expressions,
            rest,
            expression_array_aliases,
            resolving_array_aliases,
        ),
        ExpressionKind::Name(name) => {
            let alias = expression_array_aliases.get(name)?;
            source_lookup_array_alias_path_element(
                alias,
                rest,
                expression_array_aliases,
                resolving_array_aliases,
            )
        }
        _ => None,
    }
}

fn strip_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_group_expression(inner),
        _ => expression,
    }
}

fn source_lookup_index_chain(expression: &Expression) -> Option<(&str, Vec<&Expression>)> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some((name, Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (name, mut indices) = source_lookup_index_chain(target)?;
            indices.push(index);
            Some((name, indices))
        }
        _ => None,
    }
}

fn source_lookup_index(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<u32> {
    let value = evaluate_source_static_expression(program, expression, values)?;
    u32::try_from(static_value_integer(&value)?).ok()
}

fn source_lookup_row_offset_value(
    program: &SourceProgram,
    expression: &Expression,
    prior: bool,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<i64> {
    let value = evaluate_source_static_expression(program, expression, values)?;
    let offset = static_value_integer(&value)?;
    let signed = if prior { offset.checked_neg()? } else { offset };
    i64::try_from(signed).ok()
}

fn canonical_hint_number(value: i128) -> Option<u64> {
    let modulus = i128::from(MODULUS);
    u64::try_from(value.rem_euclid(modulus)).ok()
}

fn hint_payload_from_code_operand(
    operand: CodeOperand,
    opening_points: &[i64],
) -> Option<HintPayload> {
    match operand {
        CodeOperand::Number { value, .. } => Some(HintPayload::number(value)),
        CodeOperand::Commitment {
            id,
            prime,
            dimension,
        } => {
            let row_offset = prime.unwrap_or(0);
            Some(HintPayload::Commitment {
                id,
                row_offset_index: Some(opening_point_index(opening_points, row_offset)?),
                row_offset: Some(row_offset),
                stage: None,
                stage_id: None,
                dimension: Some(dimension),
                air_group_id: None,
                air_id: None,
            })
        }
        CodeOperand::CommitmentElement {
            id,
            element,
            prime,
            dimension,
        } => {
            let row_offset = prime.unwrap_or(0);
            Some(HintPayload::commitment_element(
                id,
                element,
                Some(opening_point_index(opening_points, row_offset)?),
                Some(row_offset),
                Some(dimension),
            ))
        }
        CodeOperand::Constant { id, dimension } => Some(HintPayload::constant(
            id,
            Some(opening_point_index(opening_points, 0)?),
            Some(0),
            Some(dimension),
            None,
            None,
        )),
        CodeOperand::ConstantAt {
            id,
            prime,
            dimension,
        } => {
            let row_offset = prime.unwrap_or(0);
            Some(HintPayload::constant(
                id,
                Some(opening_point_index(opening_points, row_offset)?),
                Some(row_offset),
                Some(dimension),
                None,
                None,
            ))
        }
        CodeOperand::AirValue {
            id,
            stage,
            dimension,
            ..
        } => Some(HintPayload::air_value(id, stage, Some(dimension))),
        CodeOperand::AirGroupValue {
            id,
            stage,
            air_group_id,
            dimension,
        } => Some(HintPayload::air_group_value(
            id,
            air_group_id,
            stage,
            Some(dimension),
        )),
        CodeOperand::Public { id, .. } => Some(HintPayload::public(id, None)),
        CodeOperand::Challenge {
            id,
            stage,
            stage_id,
            ..
        } => Some(HintPayload::challenge(id, stage, stage_id)),
        CodeOperand::ProofValue {
            id,
            stage,
            dimension,
        } => Some(HintPayload::proof_value(id, stage, Some(dimension))),
        _ => None,
    }
}

fn source_assignment_target_payload(
    operand: CodeOperand,
    opening_points: &[i64],
) -> Option<HintPayload> {
    match operand {
        CodeOperand::Commitment { .. } | CodeOperand::CommitmentElement { .. } => {
            hint_payload_from_code_operand(operand, opening_points)
        }
        _ => None,
    }
}

fn opening_point_index(opening_points: &[i64], row_offset: i64) -> Option<u32> {
    opening_points
        .iter()
        .position(|point| *point == row_offset)
        .and_then(|index| u32::try_from(index).ok())
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
        "lookup_proves"
        | "permutation_proves"
        | "direct_update_proves"
        | "direct_global_update_proves" => Ok(Some(SOURCE_LOOKUP_PROVES_HINT)),
        "lookup_assumes"
        | "permutation_assumes"
        | "direct_update_assumes"
        | "direct_global_update_assumes" => Ok(Some(SOURCE_LOOKUP_ASSUMES_HINT)),
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

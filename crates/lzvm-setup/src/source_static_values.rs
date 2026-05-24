use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    lex_source, parse_expression_tokens, BinaryOperator, Expression, ExpressionKind,
    FixedFileTemplateValue, FunctionStatement, FunctionStatementKind, SourceFile, SourceProgram,
    SourceProgramModule, SourceSpan, Token, TokenKind, UnaryOperator,
};

use crate::{
    source_scope::{declaration_in_function_body, declaration_in_inactive_template},
    source_static_functions::evaluate_static_i128,
};

const STATIC_TEMPLATE_LOOP_LIMIT: usize = 10_000;

pub(crate) type SourceTemplateConstantValueCache =
    BTreeMap<(String, usize, usize), BTreeMap<String, FixedFileTemplateValue>>;

pub(crate) fn source_scalar_constant_values(
    program: &SourceProgram,
    row_count: u64,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = BTreeMap::from([
        (
            "BITS".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count.trailing_zeros())),
        ),
        (
            "N".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count)),
        ),
    ]);
    let declarations = program
        .modules
        .iter()
        .flat_map(|module| {
            module.constants.iter().filter(move |declaration| {
                !declaration_in_function_body(module, declaration.start, declaration.end)
                    && !declaration_in_template(module, declaration.start, declaration.end)
            })
        })
        .collect::<Vec<_>>();
    let mut resolved = vec![false; declarations.len()];

    loop {
        let mut progressed = false;
        let integer_env = integer_values(&values);
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved[index] {
                continue;
            }
            if !declaration.array_dims.is_empty() {
                continue;
            }
            if values.contains_key(&declaration.name) {
                resolved[index] = true;
                progressed = true;
                continue;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                continue;
            };
            let Some(value) = evaluate_source_static_expression_with_integer_env(
                program,
                expression,
                &values,
                &integer_env,
            ) else {
                continue;
            };
            values.insert(declaration.name.clone(), value);
            resolved[index] = true;
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    let mut resolved_arrays = vec![false; declarations.len()];
    loop {
        let mut progressed = false;
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved_arrays[index] || declaration.array_dims.is_empty() {
                continue;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                continue;
            };
            let Some(elements) = source_static_array_expression(program, expression, &values)
            else {
                continue;
            };
            if insert_source_static_array(&mut values, &declaration.name, elements).is_none() {
                continue;
            }
            resolved_arrays[index] = true;
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    values
}

pub(crate) trait SourceStaticValueLookup {
    fn source_static_value(&self, name: &str) -> Option<&FixedFileTemplateValue>;
    fn source_static_array_element(
        &self,
        name: &str,
        index: usize,
    ) -> Option<FixedFileTemplateValue>;
    fn source_static_integer_values(&self) -> BTreeMap<String, i128>;
}

impl SourceStaticValueLookup for BTreeMap<String, FixedFileTemplateValue> {
    fn source_static_value(&self, name: &str) -> Option<&FixedFileTemplateValue> {
        self.get(name)
    }

    fn source_static_array_element(
        &self,
        name: &str,
        index: usize,
    ) -> Option<FixedFileTemplateValue> {
        self.get(&source_static_array_element_key(name, index))
            .cloned()
    }

    fn source_static_integer_values(&self) -> BTreeMap<String, i128> {
        integer_values(self)
    }
}

pub(crate) fn insert_source_static_array(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
    elements: Vec<FixedFileTemplateValue>,
) -> Option<()> {
    let length = i128::try_from(elements.len()).ok()?;
    values.insert(
        source_static_array_length_key(name),
        FixedFileTemplateValue::Integer(length),
    );
    for (index, value) in elements.into_iter().enumerate() {
        values.insert(source_static_array_element_key(name, index), value);
    }
    Some(())
}

pub(crate) fn source_static_array_values(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
) -> Option<Vec<FixedFileTemplateValue>> {
    let length = usize::try_from(source_static_array_length(values, name)?).ok()?;
    (0..length)
        .map(|index| source_static_array_element(values, name, index))
        .collect()
}

pub(crate) fn source_static_array_length(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
) -> Option<i128> {
    let FixedFileTemplateValue::Integer(length) =
        values.get(&source_static_array_length_key(name))?
    else {
        return None;
    };
    Some(*length)
}

pub(crate) fn source_static_array_element(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
    index: usize,
) -> Option<FixedFileTemplateValue> {
    values
        .get(&source_static_array_element_key(name, index))
        .cloned()
}

pub(crate) fn source_static_array_length_key(name: &str) -> String {
    format!("__lzvm_array_len::{name}")
}

pub(crate) fn source_static_array_element_key(name: &str, index: usize) -> String {
    format!("__lzvm_array_value::{name}::{index}")
}

pub(crate) fn evaluate_source_static_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<FixedFileTemplateValue> {
    evaluate_source_static_expression_with_lookup(program, expression, values)
}

pub(crate) fn evaluate_source_static_expression_with_lookup(
    program: &SourceProgram,
    expression: &Expression,
    values: &(impl SourceStaticValueLookup + ?Sized),
) -> Option<FixedFileTemplateValue> {
    if let Some(value) =
        evaluate_source_template_value_expression_with_lookup(program, expression, values)
    {
        return Some(value);
    }
    if !source_expression_needs_integer_env(expression) {
        return None;
    }
    let env = values.source_static_integer_values();
    evaluate_static_i128(program, expression, &env).map(FixedFileTemplateValue::Integer)
}

fn evaluate_source_static_expression_with_integer_env(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    integer_env: &BTreeMap<String, i128>,
) -> Option<FixedFileTemplateValue> {
    evaluate_source_template_value_expression(program, expression, values).or_else(|| {
        if !source_expression_needs_integer_env(expression) {
            return None;
        }
        evaluate_static_i128(program, expression, integer_env).map(FixedFileTemplateValue::Integer)
    })
}

fn source_expression_needs_integer_env(expression: &Expression) -> bool {
    match &expression.kind {
        ExpressionKind::Call { .. } => true,
        ExpressionKind::Group(inner) => source_expression_needs_integer_env(inner),
        ExpressionKind::Unary { expr, .. } => source_expression_needs_integer_env(expr),
        ExpressionKind::Binary { left, right, .. } => {
            source_expression_needs_integer_env(left) || source_expression_needs_integer_env(right)
        }
        ExpressionKind::Array(values) => values.iter().any(source_expression_needs_integer_env),
        _ => false,
    }
}

fn evaluate_source_template_value_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<FixedFileTemplateValue> {
    evaluate_source_template_value_expression_with_lookup(program, expression, values)
}

fn evaluate_source_template_value_expression_with_lookup(
    program: &SourceProgram,
    expression: &Expression,
    values: &(impl SourceStaticValueLookup + ?Sized),
) -> Option<FixedFileTemplateValue> {
    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128(value).map(FixedFileTemplateValue::Integer)
        }
        ExpressionKind::StringLiteral(value) | ExpressionKind::TemplateLiteral(value) => {
            Some(FixedFileTemplateValue::String(value.clone()))
        }
        ExpressionKind::Name(name) => values.source_static_value(name).cloned(),
        ExpressionKind::Group(inner) => {
            evaluate_source_template_value_expression_with_lookup(program, inner, values)
        }
        ExpressionKind::Index { target, index } => {
            let name = expression_name(target)?;
            let index = evaluate_source_static_expression_with_lookup(program, index, values)?;
            let index = usize::try_from(static_value_integer(&index)?).ok()?;
            values.source_static_array_element(name, index)
        }
        ExpressionKind::Unary { op, expr } => {
            let value =
                evaluate_source_template_value_expression_with_lookup(program, expr, values)?;
            match op {
                UnaryOperator::Plus => {
                    static_value_integer(&value).map(FixedFileTemplateValue::Integer)
                }
                UnaryOperator::Minus => static_value_integer(&value)
                    .and_then(i128::checked_neg)
                    .map(FixedFileTemplateValue::Integer),
                UnaryOperator::Not => Some(FixedFileTemplateValue::Boolean(!static_value_truthy(
                    &value,
                ))),
                UnaryOperator::Increment | UnaryOperator::Decrement => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let left =
                evaluate_source_template_value_expression_with_lookup(program, left, values)?;
            match op {
                BinaryOperator::LogicalAnd => {
                    if static_value_truthy(&left) {
                        evaluate_source_template_value_expression_with_lookup(
                            program, right, values,
                        )
                    } else {
                        Some(left)
                    }
                }
                BinaryOperator::LogicalOr => {
                    if static_value_truthy(&left) {
                        Some(left)
                    } else {
                        evaluate_source_template_value_expression_with_lookup(
                            program, right, values,
                        )
                    }
                }
                _ => {
                    let right = evaluate_source_template_value_expression_with_lookup(
                        program, right, values,
                    )?;
                    evaluate_source_template_binary(*op, left, right)
                }
            }
        }
        _ => None,
    }
}

fn evaluate_source_template_binary(
    op: BinaryOperator,
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
) -> Option<FixedFileTemplateValue> {
    match op {
        BinaryOperator::Add => match (&left, &right) {
            (FixedFileTemplateValue::Integer(left), FixedFileTemplateValue::Integer(right)) => left
                .checked_add(*right)
                .map(FixedFileTemplateValue::Integer),
            _ => Some(FixedFileTemplateValue::String(format!(
                "{}{}",
                source_template_value_string(left),
                source_template_value_string(right)
            ))),
        },
        BinaryOperator::Subtract => source_template_integer_op(left, right, i128::checked_sub),
        BinaryOperator::Multiply => source_template_integer_op(left, right, i128::checked_mul),
        BinaryOperator::Divide | BinaryOperator::Backslash => {
            let left = static_value_integer(&left)?;
            let right = static_value_integer(&right)?;
            (right != 0).then(|| FixedFileTemplateValue::Integer(left / right))
        }
        BinaryOperator::Modulo => {
            let left = static_value_integer(&left)?;
            let right = static_value_integer(&right)?;
            (right != 0).then(|| FixedFileTemplateValue::Integer(left % right))
        }
        BinaryOperator::Power => {
            let exponent = u32::try_from(static_value_integer(&right)?).ok()?;
            static_value_integer(&left)?
                .checked_pow(exponent)
                .map(FixedFileTemplateValue::Integer)
        }
        BinaryOperator::ShiftLeft => source_template_integer_shift(left, right, true),
        BinaryOperator::ShiftRight => source_template_integer_shift(left, right, false),
        BinaryOperator::BitAnd => source_template_integer_bitwise(left, right, |a, b| a & b),
        BinaryOperator::BitXor => source_template_integer_bitwise(left, right, |a, b| a ^ b),
        BinaryOperator::BitOr => source_template_integer_bitwise(left, right, |a, b| a | b),
        BinaryOperator::Less => source_template_integer_cmp(left, right, |a, b| a < b),
        BinaryOperator::LessEqual => source_template_integer_cmp(left, right, |a, b| a <= b),
        BinaryOperator::Greater => source_template_integer_cmp(left, right, |a, b| a > b),
        BinaryOperator::GreaterEqual => source_template_integer_cmp(left, right, |a, b| a >= b),
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => {
            Some(FixedFileTemplateValue::Boolean(left == right))
        }
        BinaryOperator::NotEqual => Some(FixedFileTemplateValue::Boolean(left != right)),
        _ => None,
    }
}

fn source_template_integer_op(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl FnOnce(i128, i128) -> Option<i128>,
) -> Option<FixedFileTemplateValue> {
    op(static_value_integer(&left)?, static_value_integer(&right)?)
        .map(FixedFileTemplateValue::Integer)
}

fn source_template_integer_shift(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    shift_left: bool,
) -> Option<FixedFileTemplateValue> {
    let left = static_value_integer(&left)?;
    let right = u32::try_from(static_value_integer(&right)?).ok()?;
    if shift_left {
        left.checked_shl(right).map(FixedFileTemplateValue::Integer)
    } else {
        left.checked_shr(right).map(FixedFileTemplateValue::Integer)
    }
}

fn source_template_integer_bitwise(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl FnOnce(i128, i128) -> i128,
) -> Option<FixedFileTemplateValue> {
    Some(FixedFileTemplateValue::Integer(op(
        static_value_integer(&left)?,
        static_value_integer(&right)?,
    )))
}

fn source_template_integer_cmp(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl FnOnce(i128, i128) -> bool,
) -> Option<FixedFileTemplateValue> {
    Some(FixedFileTemplateValue::Boolean(op(
        static_value_integer(&left)?,
        static_value_integer(&right)?,
    )))
}

fn source_template_value_string(value: FixedFileTemplateValue) -> String {
    match value {
        FixedFileTemplateValue::Integer(value) => value.to_string(),
        FixedFileTemplateValue::Boolean(value) => value.to_string(),
        FixedFileTemplateValue::String(value) => value,
    }
}

fn integer_values(values: &BTreeMap<String, FixedFileTemplateValue>) -> BTreeMap<String, i128> {
    values
        .iter()
        .filter_map(|(name, value)| match value {
            FixedFileTemplateValue::Integer(value) => Some((name.clone(), *value)),
            _ => None,
        })
        .collect()
}

fn expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => expression_name(inner),
        _ => None,
    }
}

fn parse_i128(value: &str) -> Option<i128> {
    let value = value.trim().replace('_', "");
    if let Some(hex) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        return i128::from_str_radix(hex, 16)
            .ok()
            .and_then(i128::checked_neg);
    }
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return i128::from_str_radix(hex, 16).ok();
    }
    value.parse::<i128>().ok()
}

fn evaluate_source_static_expression_or_token_span(
    program: &SourceProgram,
    source: &SourceFile,
    tokens: &[Token],
    expression: Option<&Expression>,
    span: Option<SourceSpan>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<FixedFileTemplateValue> {
    if let Some(expression) = expression {
        if let Some(value) = evaluate_source_static_expression(program, expression, values) {
            return Some(value);
        }
    }
    let span = span?;
    let start = source_token_index_at_start(tokens, span.start)?;
    let end = source_token_index_after_end(tokens, span.end)?;
    evaluate_source_static_token_range(program, source, tokens, start, end, values)
}

fn evaluate_source_static_token_range(
    program: &SourceProgram,
    source: &SourceFile,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<FixedFileTemplateValue> {
    if let Some((question, colon)) = top_level_ternary(tokens, start, end) {
        let condition =
            evaluate_source_static_token_range(program, source, tokens, start, question, values)?;
        let branch = if static_value_truthy(&condition) {
            question + 1..colon
        } else {
            colon + 1..end
        };
        return evaluate_source_static_token_range(
            program,
            source,
            tokens,
            branch.start,
            branch.end,
            values,
        );
    }

    let (expression, consumed) = parse_expression_tokens(tokens, start, end, source).ok()?;
    if consumed != end {
        return None;
    }
    evaluate_source_static_expression(program, &expression, values)
}

fn top_level_ternary(tokens: &[Token], start: usize, end: usize) -> Option<(usize, usize)> {
    let mut expected = Vec::<TokenKind>::new();
    let mut question = None;
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        match token.kind {
            TokenKind::LParen => expected.push(TokenKind::RParen),
            TokenKind::LBracket => expected.push(TokenKind::RBracket),
            TokenKind::LBrace => expected.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if expected.pop()? != token.kind {
                    return None;
                }
            }
            TokenKind::Question if expected.is_empty() => {
                if question.is_some() {
                    return None;
                }
                question = Some(index);
            }
            TokenKind::Colon if expected.is_empty() => {
                return question.map(|question| (question, index));
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn static_value_truthy(value: &FixedFileTemplateValue) -> bool {
    match value {
        FixedFileTemplateValue::Integer(value) => *value != 0,
        FixedFileTemplateValue::Boolean(value) => *value,
        FixedFileTemplateValue::String(value) => !value.is_empty(),
    }
}

pub(crate) fn static_value_integer(value: &FixedFileTemplateValue) -> Option<i128> {
    match value {
        FixedFileTemplateValue::Integer(value) => Some(*value),
        FixedFileTemplateValue::Boolean(value) => Some(if *value { 1 } else { 0 }),
        FixedFileTemplateValue::String(_) => None,
    }
}

fn execute_static_template_range(
    program: &SourceProgram,
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    let tokens = lex_source(&module.source.contents).ok()?;
    let start = tokens
        .iter()
        .position(|token| token.start >= start)
        .unwrap_or(tokens.len());
    let end = tokens
        .iter()
        .position(|token| token.start >= end)
        .unwrap_or(tokens.len());
    execute_static_template_tokens(program, module, &tokens, start, end, values)
}

fn execute_static_template_tokens(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    mut index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    while index < end {
        match tokens.get(index).map(|token| token.kind) {
            Some(TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon) => {
                index += 1;
            }
            Some(TokenKind::EndOfInput) | None => break,
            _ => {
                let next =
                    execute_static_template_statement(program, module, tokens, index, end, values)
                        .or_else(|| skip_static_template_statement(tokens, index, end))
                        .filter(|next| *next > index)
                        .unwrap_or(index + 1);
                index = next;
            }
        }
    }
    Some(())
}

fn execute_static_template_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    match tokens.get(index)?.kind {
        TokenKind::If => execute_static_template_if(program, module, tokens, index, end, values),
        TokenKind::For => execute_static_template_for(program, module, tokens, index, end, values),
        TokenKind::While => {
            execute_static_template_while(program, module, tokens, index, end, values)
        }
        TokenKind::Switch => {
            execute_static_template_switch(program, module, tokens, index, end, values)
        }
        kind if static_declaration_start(kind) => {
            execute_static_template_declaration(program, module, tokens, index, values);
            skip_static_template_statement(tokens, index, end)
        }
        _ => {
            let semicolon = next_static_semicolon_limited(tokens, index, end)?;
            if !static_statement_contains_assignment_operator(tokens, index, semicolon) {
                return Some(semicolon + 1);
            }
            if crate::source_static_array_assignment::execute_source_static_array_assignment_statement(
                program, module, tokens, index, semicolon, values,
            )
            .is_some()
            {
                return Some(semicolon + 1);
            }
            if unsupported_static_assignment_statement(tokens, index, semicolon) {
                return Some(semicolon + 1);
            }
            if execute_source_static_postfix_update(
                program, module, tokens, index, semicolon, values,
            )
            .is_some()
            {
                return Some(semicolon + 1);
            }
            let (expression, consumed) =
                parse_expression_tokens(tokens, index, semicolon, &module.source).ok()?;
            if consumed == semicolon {
                execute_source_static_expression_statement(
                    program,
                    module,
                    tokens.get(index)?.start,
                    &expression,
                    values,
                );
            }
            Some(semicolon + 1)
        }
    }
}

fn execute_static_template_if(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let open = next_token_kind(tokens, index + 1, end, TokenKind::LParen)?;
    let close = matching_closing_token(tokens, open, end)?;
    let condition = evaluate_source_static_token_range(
        program,
        &module.source,
        tokens,
        open + 1,
        close,
        values,
    )?;
    let (body_start, body_end, after_body) = control_body_range(tokens, close + 1, end)?;
    if static_value_truthy(&condition) {
        execute_static_template_tokens(program, module, tokens, body_start, body_end, values);
        return skip_static_else_tail(tokens, after_body, end);
    }
    execute_static_else_tail(program, module, tokens, after_body, end, values)
}

fn execute_static_template_for(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let checkpoint = values.clone();
    let next = execute_static_template_for_inner(program, module, tokens, index, end, values);
    if next.is_none() {
        *values = checkpoint;
    }
    next
}

fn execute_static_template_for_inner(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let open = next_token_kind(tokens, index + 1, end, TokenKind::LParen)?;
    let close = matching_closing_token(tokens, open, end)?;
    let [initializer, condition, update] = static_for_header_ranges(tokens, open + 1, close)?;
    let (body_start, body_end, after_body) = control_body_range(tokens, close + 1, end)?;
    let loop_variable =
        execute_static_for_initializer(program, module, tokens, initializer, values)?;

    for _ in 0..STATIC_TEMPLATE_LOOP_LIMIT {
        let condition = evaluate_source_static_token_range(
            program,
            &module.source,
            tokens,
            condition.0,
            condition.1,
            values,
        )?;
        if !static_value_truthy(&condition) {
            values.remove(&loop_variable);
            return Some(after_body);
        }
        execute_static_template_tokens(program, module, tokens, body_start, body_end, values)?;
        execute_static_for_update(program, module, tokens, update, values)?;
    }
    None
}

fn static_for_header_ranges(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Option<[(usize, usize); 3]> {
    let mut ranges = Vec::new();
    let mut range_start = start;
    let mut stack = Vec::<TokenKind>::new();
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            ranges.push((range_start, index));
            range_start = index + 1;
            continue;
        }
        update_static_delimiter_stack(token.kind, &mut stack)?;
    }
    ranges.push((range_start, end));
    if !stack.is_empty() {
        return None;
    }
    <[(usize, usize); 3]>::try_from(ranges).ok()
}

fn execute_static_for_initializer(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    range: (usize, usize),
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<String> {
    let mut cursor = range.0;
    if tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Const) {
        cursor += 1;
    }
    if tokens.get(cursor).map(|token| token.kind) != Some(TokenKind::Int) {
        return None;
    }
    let name = tokens.get(cursor + 1)?;
    if name.kind != TokenKind::Identifier {
        return None;
    }
    if tokens.get(cursor + 2).map(|token| token.kind) != Some(TokenKind::Assign) {
        return None;
    }
    let value = evaluate_source_static_token_range(
        program,
        &module.source,
        tokens,
        cursor + 3,
        range.1,
        values,
    )?;
    let value = static_value_integer(&value)?;
    values.insert(name.lexeme.clone(), FixedFileTemplateValue::Integer(value));
    Some(name.lexeme.clone())
}

fn execute_static_for_update(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    range: (usize, usize),
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    if execute_source_static_postfix_update(program, module, tokens, range.0, range.1, values)
        .is_some()
    {
        return Some(());
    }
    let (expression, consumed) =
        parse_expression_tokens(tokens, range.0, range.1, &module.source).ok()?;
    if consumed != range.1 {
        return None;
    }
    execute_source_static_expression_statement(
        program,
        module,
        tokens.get(range.0)?.start,
        &expression,
        values,
    )
}

fn execute_static_template_while(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let checkpoint = values.clone();
    let next = execute_static_template_while_inner(program, module, tokens, index, end, values);
    if next.is_none() {
        *values = checkpoint;
    }
    next
}

fn execute_static_template_while_inner(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let open = next_token_kind(tokens, index + 1, end, TokenKind::LParen)?;
    let close = matching_closing_token(tokens, open, end)?;
    let (body_start, body_end, after_body) = control_body_range(tokens, close + 1, end)?;
    for _ in 0..STATIC_TEMPLATE_LOOP_LIMIT {
        let condition = evaluate_source_static_token_range(
            program,
            &module.source,
            tokens,
            open + 1,
            close,
            values,
        )?;
        if !static_value_truthy(&condition) {
            return Some(after_body);
        }
        execute_static_template_tokens(program, module, tokens, body_start, body_end, values)?;
    }
    None
}

fn execute_static_template_switch(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let checkpoint = values.clone();
    let next = execute_static_template_switch_inner(program, module, tokens, index, end, values);
    if next.is_none() {
        *values = checkpoint;
    }
    next
}

fn execute_static_template_switch_inner(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let open = next_token_kind(tokens, index + 1, end, TokenKind::LParen)?;
    let close = matching_closing_token(tokens, open, end)?;
    let condition = evaluate_source_static_token_range(
        program,
        &module.source,
        tokens,
        open + 1,
        close,
        values,
    )?;
    let (body_start, body_end, after_body) = control_body_range(tokens, close + 1, end)?;
    let branch_start = static_switch_branch_start(
        program, module, tokens, body_start, body_end, values, &condition,
    )?;
    if let Some(branch_start) = branch_start {
        execute_static_switch_branch(program, module, tokens, branch_start, body_end, values)?;
    }
    Some(after_body)
}

fn static_switch_branch_start(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    condition: &FixedFileTemplateValue,
) -> Option<Option<usize>> {
    let mut default_start = None;
    let mut stack = Vec::<TokenKind>::new();
    let mut index = start;
    while index < end {
        let token = tokens.get(index)?;
        if stack.is_empty() {
            match token.kind {
                TokenKind::Case => {
                    let colon = static_switch_label_colon(tokens, index + 1, end)?;
                    let value = evaluate_source_static_token_range(
                        program,
                        &module.source,
                        tokens,
                        index + 1,
                        colon,
                        values,
                    )?;
                    if value == *condition {
                        return Some(Some(colon + 1));
                    }
                    index = colon + 1;
                    continue;
                }
                TokenKind::Default => {
                    let colon = static_switch_label_colon(tokens, index + 1, end)?;
                    if default_start.is_none() {
                        default_start = Some(colon + 1);
                    }
                    index = colon + 1;
                    continue;
                }
                _ => {}
            }
        }
        update_static_delimiter_stack(token.kind, &mut stack)?;
        index += 1;
    }
    if stack.is_empty() {
        Some(default_start)
    } else {
        None
    }
}

fn execute_static_switch_branch(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    mut index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    while index < end {
        match tokens.get(index).map(|token| token.kind) {
            Some(TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon) => {
                index += 1;
            }
            Some(TokenKind::Case | TokenKind::Default) => {
                index = static_switch_label_colon(tokens, index + 1, end)? + 1;
            }
            Some(TokenKind::Break) => {
                let semicolon = next_static_semicolon_limited(tokens, index, end)?;
                return (semicolon < end).then_some(());
            }
            Some(TokenKind::EndOfInput) | None => break,
            _ => {
                let next =
                    execute_static_template_statement(program, module, tokens, index, end, values)
                        .or_else(|| skip_static_template_statement(tokens, index, end))
                        .filter(|next| *next > index)
                        .unwrap_or(index + 1);
                index = next;
            }
        }
    }
    Some(())
}

fn execute_static_else_tail(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    match tokens.get(index).map(|token| token.kind) {
        Some(TokenKind::ElseIf) => {
            execute_static_template_if(program, module, tokens, index, end, values)
        }
        Some(TokenKind::Else)
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::If) =>
        {
            execute_static_template_if(program, module, tokens, index + 1, end, values)
        }
        Some(TokenKind::Else) => {
            let (body_start, body_end, after_body) = control_body_range(tokens, index + 1, end)?;
            execute_static_template_tokens(program, module, tokens, body_start, body_end, values);
            Some(after_body)
        }
        _ => Some(index),
    }
}

fn skip_static_else_tail(tokens: &[Token], index: usize, end: usize) -> Option<usize> {
    match tokens.get(index).map(|token| token.kind) {
        Some(TokenKind::ElseIf) => skip_static_if_statement(tokens, index, end),
        Some(TokenKind::Else)
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::If) =>
        {
            skip_static_if_statement(tokens, index + 1, end)
        }
        Some(TokenKind::Else) => {
            let (_, _, after_body) = control_body_range(tokens, index + 1, end)?;
            Some(after_body)
        }
        _ => Some(index),
    }
}

fn skip_static_template_statement(tokens: &[Token], index: usize, end: usize) -> Option<usize> {
    match tokens.get(index)?.kind {
        TokenKind::If | TokenKind::ElseIf => skip_static_if_statement(tokens, index, end),
        TokenKind::Else
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::If) =>
        {
            skip_static_if_statement(tokens, index + 1, end)
        }
        TokenKind::Else => {
            let (_, _, after_body) = control_body_range(tokens, index + 1, end)?;
            Some(after_body)
        }
        TokenKind::For | TokenKind::While | TokenKind::Switch => {
            let open = next_token_kind(tokens, index + 1, end, TokenKind::LParen)?;
            let close = matching_closing_token(tokens, open, end)?;
            let (_, _, after_body) = control_body_range(tokens, close + 1, end)?;
            Some(after_body)
        }
        _ => next_static_semicolon_limited(tokens, index, end).map(|semicolon| semicolon + 1),
    }
}

fn skip_static_if_statement(tokens: &[Token], index: usize, end: usize) -> Option<usize> {
    let open = next_token_kind(tokens, index + 1, end, TokenKind::LParen)?;
    let close = matching_closing_token(tokens, open, end)?;
    let (_, _, after_body) = control_body_range(tokens, close + 1, end)?;
    skip_static_else_tail(tokens, after_body, end)
}

fn execute_static_template_declaration(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    let start = tokens.get(index)?.start;
    if let Some(declaration) = module
        .constants
        .iter()
        .find(|declaration| declaration.start == start)
    {
        if values.contains_key(&declaration.name) {
            return Some(());
        }
        if !declaration.array_dims.is_empty() {
            let elements = source_static_array_expression(
                program,
                declaration.initializer_expression.as_ref()?,
                values,
            )?;
            insert_source_static_array(values, &declaration.name, elements)?;
            return Some(());
        }
        let value = evaluate_source_static_expression_or_token_span(
            program,
            &module.source,
            tokens,
            declaration.initializer_expression.as_ref(),
            declaration.initializer,
            values,
        )?;
        values.insert(declaration.name.clone(), value);
        return Some(());
    }

    let declaration = module
        .variables
        .iter()
        .find(|declaration| declaration.start == start)?;
    if !declaration.array_dims.is_empty() {
        let elements = source_static_array_expression(
            program,
            declaration.initializer_expression.as_ref()?,
            values,
        )?;
        insert_source_static_array(values, &declaration.name, elements)?;
        return Some(());
    }
    let value = evaluate_source_static_expression_or_token_span(
        program,
        &module.source,
        tokens,
        declaration.initializer_expression.as_ref(),
        declaration.initializer,
        values,
    )?;
    values.insert(declaration.name.clone(), value);
    Some(())
}

fn static_statement_contains_assignment_operator(
    tokens: &[Token],
    index: usize,
    end: usize,
) -> bool {
    tokens.iter().take(end).skip(index).any(|token| {
        matches!(
            token.kind,
            TokenKind::Assign
                | TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::Increment
                | TokenKind::Decrement
        )
    })
}

fn unsupported_static_assignment_statement(tokens: &[Token], index: usize, end: usize) -> bool {
    let mut has_bracket = false;
    for token in tokens.iter().take(end).skip(index) {
        match token.kind {
            TokenKind::LBracket | TokenKind::RBracket => has_bracket = true,
            TokenKind::Assign
            | TokenKind::ConstrainedAssign
            | TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual
            | TokenKind::Increment
            | TokenKind::Decrement => return has_bracket,
            _ => {}
        }
    }
    false
}

fn execute_source_static_postfix_update(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    let name = tokens.get(index)?;
    let update = tokens.get(index + 1)?;
    if index + 2 != end || name.kind != TokenKind::Identifier {
        return None;
    }
    let delta = match update.kind {
        TokenKind::Increment => 1,
        TokenKind::Decrement => -1,
        _ => return None,
    };
    if !source_static_assignment_target_visible(program, module, &name.lexeme, name.start, values) {
        return None;
    }
    execute_source_static_delta(&name.lexeme, delta, values)
}

fn execute_source_static_delta(
    name: &str,
    delta: i128,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    let current = static_value_integer(
        values
            .get(name)
            .unwrap_or(&FixedFileTemplateValue::Integer(0)),
    )?;
    values.insert(
        name.to_owned(),
        FixedFileTemplateValue::Integer(current.checked_add(delta)?),
    );
    Some(())
}

fn execute_source_static_expression_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement_start: usize,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            let name = expression_name(expr)?;
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => return None,
            };
            if !source_static_assignment_target_visible(
                program,
                module,
                name,
                statement_start,
                values,
            ) {
                return None;
            }
            execute_source_static_delta(name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let name = expression_name(left)?.to_owned();
            if !source_static_assignment_target_visible(
                program,
                module,
                &name,
                statement_start,
                values,
            ) {
                return None;
            }
            let right = evaluate_source_static_expression(program, right, values)?;
            let value = match op {
                BinaryOperator::Assign => right,
                BinaryOperator::PlusAssign => {
                    let current = static_value_integer(
                        values
                            .get(&name)
                            .unwrap_or(&FixedFileTemplateValue::Integer(0)),
                    )?;
                    FixedFileTemplateValue::Integer(
                        current.checked_add(static_value_integer(&right)?)?,
                    )
                }
                BinaryOperator::MinusAssign => {
                    let current = static_value_integer(
                        values
                            .get(&name)
                            .unwrap_or(&FixedFileTemplateValue::Integer(0)),
                    )?;
                    FixedFileTemplateValue::Integer(
                        current.checked_sub(static_value_integer(&right)?)?,
                    )
                }
                BinaryOperator::StarAssign => {
                    let current = static_value_integer(
                        values
                            .get(&name)
                            .unwrap_or(&FixedFileTemplateValue::Integer(0)),
                    )?;
                    FixedFileTemplateValue::Integer(
                        current.checked_mul(static_value_integer(&right)?)?,
                    )
                }
                _ => return None,
            };
            values.insert(name, value);
            Some(())
        }
        _ => None,
    }
}

fn source_static_assignment_target_visible(
    program: &SourceProgram,
    module: &SourceProgramModule,
    name: &str,
    statement_start: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    if values.contains_key(name) {
        return true;
    }
    let lookup = SourceStaticAssignmentTargetLookup {
        program,
        module,
        name,
        statement_start,
        values,
    };
    module.constants.iter().any(|declaration| {
        source_static_declaration_visible(
            &lookup,
            &declaration.name,
            declaration.start,
            declaration.end,
        )
    }) || module.variables.iter().any(|declaration| {
        source_static_declaration_visible(
            &lookup,
            &declaration.name,
            declaration.start,
            declaration.end,
        )
    })
}

struct SourceStaticAssignmentTargetLookup<'a> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    name: &'a str,
    statement_start: usize,
    values: &'a BTreeMap<String, FixedFileTemplateValue>,
}

fn source_static_declaration_visible(
    lookup: &SourceStaticAssignmentTargetLookup<'_>,
    declaration_name: &str,
    start: usize,
    end: usize,
) -> bool {
    declaration_name == lookup.name
        && start < lookup.statement_start
        && source_declaration_visible_from_statement(
            lookup.module,
            start,
            end,
            lookup.statement_start,
        )
        && !declaration_in_function_body(lookup.module, start, end)
        && !source_declaration_in_static_false_branch(
            lookup.program,
            lookup.module,
            start,
            end,
            lookup.values,
        )
}

fn source_declaration_visible_from_statement(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    statement_start: usize,
) -> bool {
    let Some(statement_template) = module.air_templates.iter().find(|template| {
        template.body.start <= statement_start && statement_start <= template.body.end
    }) else {
        return true;
    };
    if statement_template.body.start <= start && end <= statement_template.body.end {
        return true;
    }
    !module
        .air_templates
        .iter()
        .any(|template| template.body.start <= start && end <= template.body.end)
}

fn declaration_in_template(module: &SourceProgramModule, start: usize, end: usize) -> bool {
    module
        .air_templates
        .iter()
        .any(|template| template.body.start <= start && end <= template.body.end)
}

pub(crate) fn source_static_assignment_expression(
    program: &SourceProgram,
    module: &SourceProgramModule,
    active_templates: &BTreeSet<String>,
    expression: Option<&Expression>,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
    template_values: &SourceTemplateConstantValueCache,
) -> bool {
    let Some(expression) = expression else {
        return false;
    };
    let name = match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            if !matches!(op, UnaryOperator::Increment | UnaryOperator::Decrement) {
                return false;
            }
            expression_name(expr)
        }
        ExpressionKind::Binary { op, left, .. } => {
            if !matches!(
                op,
                BinaryOperator::Assign
                    | BinaryOperator::PlusAssign
                    | BinaryOperator::MinusAssign
                    | BinaryOperator::StarAssign
            ) {
                return false;
            }
            expression_name(left)
        }
        _ => None,
    };
    name.is_some_and(|name| {
        source_active_static_name(
            program,
            module,
            active_templates,
            name,
            base_values,
            template_values,
        )
    })
}

pub(crate) fn source_active_static_name(
    program: &SourceProgram,
    module: &SourceProgramModule,
    active_templates: &BTreeSet<String>,
    name: &str,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
    template_values: &SourceTemplateConstantValueCache,
) -> bool {
    let lookup = SourceActiveStaticNameLookup {
        program,
        module,
        active_templates,
        name,
        base_values,
        template_values,
    };
    module.constants.iter().any(|declaration| {
        source_active_static_declaration_name(
            &lookup,
            &declaration.name,
            declaration.start,
            declaration.end,
        )
    }) || module.variables.iter().any(|declaration| {
        source_active_static_declaration_name(
            &lookup,
            &declaration.name,
            declaration.start,
            declaration.end,
        )
    })
}

struct SourceActiveStaticNameLookup<'a> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    active_templates: &'a BTreeSet<String>,
    name: &'a str,
    base_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    template_values: &'a SourceTemplateConstantValueCache,
}

fn source_active_static_declaration_name(
    lookup: &SourceActiveStaticNameLookup<'_>,
    declaration_name: &str,
    start: usize,
    end: usize,
) -> bool {
    if declaration_name != lookup.name
        || declaration_in_function_body(lookup.module, start, end)
        || declaration_in_inactive_template(lookup.module, start, end, lookup.active_templates)
    {
        return false;
    }
    let declaration_values = source_declaration_constant_values_from_cache(
        lookup.module,
        start,
        end,
        lookup.base_values,
        lookup.template_values,
    );
    !source_declaration_in_static_false_branch(
        lookup.program,
        lookup.module,
        start,
        end,
        declaration_values,
    )
}

pub(crate) fn source_static_if_statement_is_false(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    if statement.kind != FunctionStatementKind::If {
        return false;
    }
    let mut values = base_values.clone();
    if let Some(template) = module.air_templates.iter().find(|template| {
        template.body.start <= statement.start && statement.end <= template.body.end
    }) {
        for parameter in &template.parameters {
            if values.contains_key(&parameter.name) {
                continue;
            }
            let Some(expression) = parameter.default_expression.as_ref() else {
                continue;
            };
            let Some(value) = evaluate_source_static_expression(program, expression, &values)
            else {
                continue;
            };
            values.insert(parameter.name.clone(), value);
        }
    }
    statement
        .header_expression
        .as_ref()
        .and_then(|expression| evaluate_source_static_expression(program, expression, &values))
        .is_some_and(|value| !static_value_truthy(&value))
}

pub(crate) fn source_declaration_in_static_false_branch(
    program: &SourceProgram,
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(template) = module
        .air_templates
        .iter()
        .find(|template| template.body.start <= start && end <= template.body.end)
    else {
        return false;
    };
    template.statements.iter().any(|statement| {
        statement
            .body
            .is_some_and(|body| body.start <= start && end <= body.end)
            && source_static_if_statement_is_false(program, module, statement, base_values)
    })
}

fn static_declaration_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Const
            | TokenKind::Constant
            | TokenKind::Int
            | TokenKind::Fe
            | TokenKind::Expr
            | TokenKind::String
    )
}

fn control_body_range(tokens: &[Token], index: usize, end: usize) -> Option<(usize, usize, usize)> {
    match tokens.get(index)?.kind {
        TokenKind::LBrace => {
            let close = matching_closing_token(tokens, index, end)?;
            Some((index + 1, close, close + 1))
        }
        _ => {
            let semicolon = next_static_semicolon_limited(tokens, index, end)?;
            Some((index, semicolon + 1, semicolon + 1))
        }
    }
}

fn next_token_kind(tokens: &[Token], start: usize, end: usize, kind: TokenKind) -> Option<usize> {
    tokens
        .iter()
        .enumerate()
        .take(end)
        .skip(start)
        .find_map(|(index, token)| (token.kind == kind).then_some(index))
}

fn matching_closing_token(tokens: &[Token], open: usize, end: usize) -> Option<usize> {
    let close_kind = match tokens.get(open)?.kind {
        TokenKind::LParen => TokenKind::RParen,
        TokenKind::LBracket => TokenKind::RBracket,
        TokenKind::LBrace => TokenKind::RBrace,
        _ => return None,
    };
    let mut expected = vec![close_kind];
    for (index, token) in tokens.iter().enumerate().take(end).skip(open + 1) {
        match token.kind {
            TokenKind::LParen => expected.push(TokenKind::RParen),
            TokenKind::LBracket => expected.push(TokenKind::RBracket),
            TokenKind::LBrace => expected.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if expected.pop()? != token.kind {
                    return None;
                }
                if expected.is_empty() {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn update_static_delimiter_stack(kind: TokenKind, stack: &mut Vec<TokenKind>) -> Option<()> {
    match kind {
        TokenKind::LParen => stack.push(TokenKind::RParen),
        TokenKind::LBracket => stack.push(TokenKind::RBracket),
        TokenKind::LBrace => stack.push(TokenKind::RBrace),
        TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
            if stack.pop()? != kind {
                return None;
            }
        }
        _ => {}
    }
    Some(())
}

fn static_switch_label_colon(tokens: &[Token], start: usize, end: usize) -> Option<usize> {
    let mut expected = Vec::<TokenKind>::new();
    let mut ternary_depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        match token.kind {
            TokenKind::LParen => expected.push(TokenKind::RParen),
            TokenKind::LBracket => expected.push(TokenKind::RBracket),
            TokenKind::LBrace => expected.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if expected.pop()? != token.kind {
                    return None;
                }
            }
            TokenKind::Question if expected.is_empty() => {
                ternary_depth = ternary_depth.checked_add(1)?;
            }
            TokenKind::Colon if expected.is_empty() && ternary_depth > 0 => {
                ternary_depth -= 1;
            }
            TokenKind::Colon if expected.is_empty() => return Some(index),
            TokenKind::EndOfInput => return None,
            _ => {}
        }
    }
    None
}

fn next_static_semicolon_limited(tokens: &[Token], index: usize, end: usize) -> Option<usize> {
    let mut expected = Vec::<TokenKind>::new();
    for (cursor, token) in tokens.iter().enumerate().take(end).skip(index) {
        match token.kind {
            TokenKind::LParen => expected.push(TokenKind::RParen),
            TokenKind::LBracket => expected.push(TokenKind::RBracket),
            TokenKind::LBrace => expected.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if expected.pop()? != token.kind {
                    return None;
                }
            }
            TokenKind::Semicolon if expected.is_empty() => return Some(cursor),
            TokenKind::EndOfInput => return None,
            _ => {}
        }
    }
    None
}

fn source_token_index_at_start(tokens: &[Token], start: usize) -> Option<usize> {
    tokens
        .iter()
        .position(|token| token.start == start && token.kind != TokenKind::EndOfInput)
}

fn source_token_index_after_end(tokens: &[Token], end: usize) -> Option<usize> {
    tokens
        .iter()
        .position(|token| token.end == end)
        .and_then(|index| index.checked_add(1))
}

pub(crate) fn source_static_array_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Vec<FixedFileTemplateValue>> {
    match &expression.kind {
        ExpressionKind::Array(elements) => elements
            .iter()
            .map(|element| evaluate_source_static_expression(program, element, values))
            .collect(),
        ExpressionKind::Group(inner) => source_static_array_expression(program, inner, values),
        ExpressionKind::Name(name) => source_static_array_values(values, name),
        _ => None,
    }
}

pub(crate) fn source_declaration_constant_values(
    program: &SourceProgram,
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = base_values.clone();
    let Some(template) = module
        .air_templates
        .iter()
        .find(|template| template.body.start <= start && end <= template.body.end)
    else {
        return values;
    };

    for parameter in &template.parameters {
        if values.contains_key(&parameter.name) {
            continue;
        }
        let Some(expression) = parameter.default_expression.as_ref() else {
            continue;
        };
        let Some(value) = evaluate_source_static_expression(program, expression, &values) else {
            continue;
        };
        values.insert(parameter.name.clone(), value);
    }

    execute_static_template_range(program, module, template.body.start, start, &mut values);

    values
}

pub(crate) fn source_template_constant_value_cache(
    program: &SourceProgram,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> SourceTemplateConstantValueCache {
    let mut cache = BTreeMap::new();
    for module in &program.modules {
        for template in &module.air_templates {
            let values = source_declaration_constant_values(
                program,
                module,
                template.body.end,
                template.body.end,
                base_values,
            );
            cache.insert(
                (
                    module.source_name.clone(),
                    template.body.start,
                    template.body.end,
                ),
                values,
            );
        }
    }
    cache
}

pub(crate) fn source_declaration_constant_values_from_cache<'a>(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    base_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    cache: &'a SourceTemplateConstantValueCache,
) -> &'a BTreeMap<String, FixedFileTemplateValue> {
    let Some(template) = module
        .air_templates
        .iter()
        .find(|template| template.body.start <= start && end <= template.body.end)
    else {
        return base_values;
    };
    cache
        .get(&(
            module.source_name.clone(),
            template.body.start,
            template.body.end,
        ))
        .unwrap_or(base_values)
}

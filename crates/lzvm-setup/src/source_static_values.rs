use std::collections::BTreeMap;
use std::path::PathBuf;

use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, lex_source, parse_expression,
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, FunctionDeclaration,
    FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind, SourceFile,
    SourceProgram, SourceProgramModule, SourceSpan, Token, TokenKind, UnaryOperator,
};

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
        .flat_map(|module| module.constants.iter())
        .collect::<Vec<_>>();
    let mut resolved = vec![false; declarations.len()];

    loop {
        let mut progressed = false;
        let integer_env = integer_values(&values);
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved[index] {
                continue;
            }
            if !declaration.array_dims.is_empty() || values.contains_key(&declaration.name) {
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

    values
}

const STATIC_LOOP_LIMIT: usize = 10_000;

pub(crate) fn evaluate_source_static_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<FixedFileTemplateValue> {
    let env = integer_values(values);
    evaluate_source_static_expression_with_integer_env(program, expression, values, &env)
}

fn evaluate_source_static_expression_with_integer_env(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    integer_env: &BTreeMap<String, i128>,
) -> Option<FixedFileTemplateValue> {
    evaluate_fixed_file_template_value_expression_with_values(expression, values).or_else(|| {
        evaluate_static_i128(program, expression, integer_env).map(FixedFileTemplateValue::Integer)
    })
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

fn evaluate_static_i128(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => parse_i128(value),
        ExpressionKind::Name(name) => values.get(name).copied(),
        ExpressionKind::Group(inner) => evaluate_static_i128(program, inner, values),
        ExpressionKind::Unary { op, expr } => {
            let value = evaluate_static_i128(program, expr, values)?;
            match op {
                UnaryOperator::Plus => Some(value),
                UnaryOperator::Minus => value.checked_neg(),
                UnaryOperator::Not => Some(static_bool(value == 0)),
                _ => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            evaluate_static_binary(program, *op, left, right, values)
        }
        ExpressionKind::Call { callee, args } if args.is_empty() => {
            let name = expression_name(callee)?;
            evaluate_static_zero_arg_function(program, name, values)
        }
        _ => None,
    }
}

fn evaluate_static_binary(
    program: &SourceProgram,
    op: BinaryOperator,
    left: &Expression,
    right: &Expression,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    if op == BinaryOperator::LogicalAnd {
        let left = evaluate_static_i128(program, left, values)?;
        if left == 0 {
            return Some(0);
        }
        return Some(static_bool(
            evaluate_static_i128(program, right, values)? != 0,
        ));
    }
    if op == BinaryOperator::LogicalOr {
        let left = evaluate_static_i128(program, left, values)?;
        if left != 0 {
            return Some(1);
        }
        return Some(static_bool(
            evaluate_static_i128(program, right, values)? != 0,
        ));
    }

    let left = evaluate_static_i128(program, left, values)?;
    let right = evaluate_static_i128(program, right, values)?;
    match op {
        BinaryOperator::Add => left.checked_add(right),
        BinaryOperator::Subtract => left.checked_sub(right),
        BinaryOperator::Multiply => left.checked_mul(right),
        BinaryOperator::Divide | BinaryOperator::Backslash if right != 0 => Some(left / right),
        BinaryOperator::Modulo if right != 0 => Some(left % right),
        BinaryOperator::Power => u32::try_from(right)
            .ok()
            .and_then(|exponent| left.checked_pow(exponent)),
        BinaryOperator::ShiftLeft => u32::try_from(right)
            .ok()
            .and_then(|amount| left.checked_shl(amount)),
        BinaryOperator::ShiftRight => u32::try_from(right)
            .ok()
            .and_then(|amount| left.checked_shr(amount)),
        BinaryOperator::Less => Some(static_bool(left < right)),
        BinaryOperator::LessEqual => Some(static_bool(left <= right)),
        BinaryOperator::Greater => Some(static_bool(left > right)),
        BinaryOperator::GreaterEqual => Some(static_bool(left >= right)),
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => {
            Some(static_bool(left == right))
        }
        BinaryOperator::NotEqual => Some(static_bool(left != right)),
        BinaryOperator::BitAnd => Some(left & right),
        BinaryOperator::BitXor => Some(left ^ right),
        BinaryOperator::BitOr => Some(left | right),
        _ => None,
    }
}

fn static_bool(value: bool) -> i128 {
    if value {
        1
    } else {
        0
    }
}

fn evaluate_static_zero_arg_function(
    program: &SourceProgram,
    name: &str,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    for module in &program.modules {
        let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == name && function.parameters.is_empty())
        else {
            continue;
        };
        return evaluate_static_function(program, module, function, values);
    }
    None
}

fn evaluate_static_function(
    program: &SourceProgram,
    module: &SourceProgramModule,
    function: &FunctionDeclaration,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    let mut values = values.clone();
    for statement in &function.statements {
        if let StaticFlow::Return(value) =
            execute_static_statement(program, module, statement, &mut values)?
        {
            return Some(value);
        }
    }
    None
}

enum StaticFlow {
    Continue,
    Return(i128),
}

fn execute_static_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    match statement.kind {
        FunctionStatementKind::Declaration => {
            execute_static_declaration(program, statement, values)?;
            Some(StaticFlow::Continue)
        }
        FunctionStatementKind::Expression => {
            execute_static_expression_statement(
                program,
                statement.value_expression.as_ref()?,
                values,
            )?;
            Some(StaticFlow::Continue)
        }
        FunctionStatementKind::Return => {
            let value =
                evaluate_static_i128(program, statement.value_expression.as_ref()?, values)?;
            Some(StaticFlow::Return(value))
        }
        FunctionStatementKind::While => {
            let condition = statement.header_expression.as_ref()?;
            let body = statement.body.as_ref()?;
            for _ in 0..STATIC_LOOP_LIMIT {
                if evaluate_static_i128(program, condition, values)? == 0 {
                    return Some(StaticFlow::Continue);
                }
                if let StaticFlow::Return(value) =
                    execute_static_body(program, module, body.start, body.end, values)?
                {
                    return Some(StaticFlow::Return(value));
                }
            }
            None
        }
        _ => None,
    }
}

fn execute_static_declaration(
    program: &SourceProgram,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    match statement.declaration.as_ref()? {
        FunctionStatementDeclaration::Constant(declaration) => {
            let expression = declaration.initializer_expression.as_ref()?;
            let value = evaluate_static_i128(program, expression, values)?;
            values.insert(declaration.name.clone(), value);
        }
        FunctionStatementDeclaration::Variable(declaration) => {
            let expression = declaration.initializer_expression.as_ref()?;
            let value = evaluate_static_i128(program, expression, values)?;
            values.insert(declaration.name.clone(), value);
        }
        FunctionStatementDeclaration::Column(_) => return None,
    }
    Some(())
}

fn execute_static_body(
    program: &SourceProgram,
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    let body = module.source.contents.get(start..end)?;
    let body = body
        .trim()
        .strip_prefix('{')
        .and_then(|body| body.strip_suffix('}'))
        .unwrap_or(body);
    let source = SourceFile {
        contents: body.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::new(),
        source_name: module.source_name.clone(),
    };
    execute_static_expression_statements(program, &source, values)?;
    Some(StaticFlow::Continue)
}

fn execute_static_expression_statements(
    program: &SourceProgram,
    source: &SourceFile,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    let tokens = lex_source(&source.contents).ok()?;
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon => {
                index += 1;
            }
            TokenKind::EndOfInput => break,
            _ => {
                let end = next_static_semicolon(&tokens, index)?;
                let (expression, consumed) = parse_expression(source, index, end).ok()?;
                if consumed != end {
                    return None;
                }
                execute_static_expression_statement(program, &expression, values)?;
                index = end + 1;
            }
        }
    }
    Some(())
}

fn next_static_semicolon(tokens: &[lzvm_pil::Token], index: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for (cursor, token) in tokens.iter().enumerate().skip(index) {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                depth = depth.checked_add(1)?;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.checked_sub(1)?;
            }
            TokenKind::Semicolon if depth == 0 => return Some(cursor),
            TokenKind::EndOfInput => return None,
            _ => {}
        }
    }
    None
}

fn execute_static_expression_statement(
    program: &SourceProgram,
    expression: &Expression,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            let name = expression_name(expr)?;
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => return None,
            };
            execute_static_delta(name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let name = expression_name(left)?.to_owned();
            let right = evaluate_static_i128(program, right, values)?;
            let current = values.get(&name).copied().unwrap_or_default();
            let value = match op {
                BinaryOperator::Assign => right,
                BinaryOperator::PlusAssign => current.checked_add(right)?,
                BinaryOperator::MinusAssign => current.checked_sub(right)?,
                BinaryOperator::StarAssign => current.checked_mul(right)?,
                _ => return None,
            };
            values.insert(name, value);
            Some(())
        }
        _ => None,
    }
}

fn execute_static_delta(
    name: &str,
    delta: i128,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    let current = values.get(name).copied().unwrap_or_default();
    values.insert(name.to_owned(), current.checked_add(delta)?);
    Some(())
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

    let (expression, consumed) = parse_expression(source, start, end).ok()?;
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

fn static_value_integer(value: &FixedFileTemplateValue) -> Option<i128> {
    match value {
        FixedFileTemplateValue::Integer(value) => Some(*value),
        FixedFileTemplateValue::Boolean(value) => Some(static_bool(*value)),
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
        kind if static_declaration_start(kind) => {
            execute_static_template_declaration(program, module, tokens, index, values);
            skip_static_template_statement(tokens, index, end)
        }
        _ => {
            let semicolon = next_static_semicolon_limited(tokens, index, end)?;
            if !static_statement_contains_assignment_operator(tokens, index, semicolon) {
                return Some(semicolon + 1);
            }
            if unsupported_static_assignment_statement(tokens, index, semicolon) {
                return Some(semicolon + 1);
            }
            if execute_source_static_postfix_update(tokens, index, semicolon, values).is_some() {
                return Some(semicolon + 1);
            }
            let (expression, consumed) = parse_expression(&module.source, index, semicolon).ok()?;
            if consumed == semicolon {
                execute_source_static_expression_statement(program, &expression, values);
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
        if !declaration.array_dims.is_empty() || values.contains_key(&declaration.name) {
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
            execute_source_static_delta(name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let name = expression_name(left)?.to_owned();
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

pub(crate) fn source_static_assignment_expression(
    module: &SourceProgramModule,
    expression: Option<&Expression>,
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
    name.is_some_and(|name| source_static_name(module, name))
}

fn source_static_name(module: &SourceProgramModule, name: &str) -> bool {
    module
        .constants
        .iter()
        .any(|declaration| declaration.name == name)
        || module
            .variables
            .iter()
            .any(|declaration| declaration.name == name)
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

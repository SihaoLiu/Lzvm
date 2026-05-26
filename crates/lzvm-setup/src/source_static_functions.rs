#![allow(clippy::items_after_test_module)]

use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    lex_source, parse_expression_tokens, parse_function_body_statements,
    parse_function_statement_tokens, BinaryOperator, CallArgument, Expression, ExpressionKind,
    FunctionDeclaration, FunctionParameter, FunctionStatement, FunctionStatementDeclaration,
    FunctionStatementKind, SourceProgram, SourceProgramModule, SourceSpan, Token, TokenKind,
    UnaryOperator,
};

use crate::{
    source_static_tokens::{
        static_switch_case_value_ranges, static_switch_label_colon, update_static_delimiter_stack,
    },
    source_static_values::{source_static_array_element_key, source_static_array_length_key},
};

const STATIC_LOOP_LIMIT: usize = 10_000;

pub(crate) fn evaluate_static_i128(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => parse_i128(value),
        ExpressionKind::Name(name) => values.get(name).copied(),
        ExpressionKind::Group(inner) => evaluate_static_i128(program, inner, values),
        ExpressionKind::Index { target, index } => {
            let name = static_expression_name(target)?;
            let index = evaluate_static_i128(program, index, values)?;
            let index = usize::try_from(index).ok()?;
            values
                .get(&source_static_array_element_key(name, index))
                .copied()
        }
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
        ExpressionKind::Call { callee, args } => {
            let name = static_expression_name(callee)?;
            evaluate_static_function_call(program, name, args, values)
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

fn evaluate_static_function_call(
    program: &SourceProgram,
    name: &str,
    arguments: &[CallArgument],
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    for module in &program.modules {
        let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == name)
        else {
            continue;
        };
        if let Some(value) =
            evaluate_simple_static_return_function_call(program, function, arguments, values)
        {
            return Some(value);
        }
        let bindings = static_function_call_bindings(program, function, arguments, values)?;
        return evaluate_static_function(program, module, function, &bindings);
    }
    None
}

fn evaluate_simple_static_return_function_call(
    program: &SourceProgram,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    let bindings = simple_static_function_call_bindings(program, function, arguments, values)?;
    for statement in &function.statements {
        match statement.kind {
            FunctionStatementKind::Expression => {
                evaluate_static_assert_statement(
                    program,
                    statement.value_expression.as_ref()?,
                    &bindings,
                )?;
            }
            FunctionStatementKind::Return => {
                return evaluate_static_i128(
                    program,
                    statement.value_expression.as_ref()?,
                    &bindings,
                );
            }
            _ => return None,
        }
    }
    None
}

fn simple_static_function_call_bindings(
    program: &SourceProgram,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, i128>,
) -> Option<BTreeMap<String, i128>> {
    if function
        .parameters
        .iter()
        .any(|parameter| !static_integer_parameter(parameter))
    {
        return None;
    }

    let mut bindings = BTreeMap::new();
    let mut provided = BTreeSet::new();
    let mut positional_index = 0_usize;
    for argument in arguments {
        let parameter = if let Some(name) = argument.name.as_ref() {
            function
                .parameters
                .iter()
                .find(|parameter| parameter.name == *name)?
        } else {
            while function
                .parameters
                .get(positional_index)
                .is_some_and(|parameter| provided.contains(&parameter.name))
            {
                positional_index = positional_index.checked_add(1)?;
            }
            function.parameters.get(positional_index)?
        };
        if provided.contains(&parameter.name) {
            return None;
        }
        let value = evaluate_static_i128(program, &argument.value, values)?;
        bindings.insert(parameter.name.clone(), value);
        provided.insert(parameter.name.clone());
        if argument.name.is_none() {
            positional_index = positional_index.checked_add(1)?;
        }
    }

    (provided.len() == function.parameters.len()).then_some(bindings)
}

fn evaluate_static_assert_statement(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, i128>,
) -> Option<()> {
    let ExpressionKind::Call { callee, args } = &expression.kind else {
        return None;
    };
    if static_expression_name(callee)? != "assert" {
        return None;
    }
    for argument in args {
        if argument.name.is_some() {
            return None;
        }
        if evaluate_static_i128(program, &argument.value, values)? == 0 {
            return None;
        }
    }
    Some(())
}

fn static_function_call_bindings(
    program: &SourceProgram,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, i128>,
) -> Option<BTreeMap<String, i128>> {
    let mut bindings = values.clone();
    let mut provided = BTreeSet::new();

    let mut positional_index = 0_usize;
    for argument in arguments {
        let parameter = if let Some(name) = argument.name.as_ref() {
            function
                .parameters
                .iter()
                .find(|parameter| parameter.name == *name)?
        } else {
            while function
                .parameters
                .get(positional_index)
                .is_some_and(|parameter| provided.contains(&parameter.name))
            {
                positional_index = positional_index.checked_add(1)?;
            }
            let parameter = function.parameters.get(positional_index)?;
            parameter
        };
        bind_static_function_argument(
            program,
            parameter,
            &argument.value,
            &mut bindings,
            &mut provided,
        )?;
        if argument.name.is_none() {
            positional_index = positional_index.checked_add(1)?;
        }
    }

    for parameter in &function.parameters {
        if provided.contains(&parameter.name) {
            continue;
        }
        bind_static_function_default(program, parameter, &mut bindings)?;
    }
    Some(bindings)
}

fn bind_static_function_default(
    program: &SourceProgram,
    parameter: &FunctionParameter,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    if !static_integer_parameter(parameter) {
        return None;
    }
    let expression = parameter.default_expression.as_ref()?;
    let value = evaluate_static_i128(program, expression, values)?;
    values.insert(parameter.name.clone(), value);
    Some(())
}

fn bind_static_function_argument(
    program: &SourceProgram,
    parameter: &FunctionParameter,
    expression: &Expression,
    values: &mut BTreeMap<String, i128>,
    provided: &mut BTreeSet<String>,
) -> Option<()> {
    if !static_integer_parameter(parameter) {
        return None;
    }
    if provided.contains(&parameter.name) {
        return None;
    }
    let value = evaluate_static_i128(program, expression, values)?;
    values.insert(parameter.name.clone(), value);
    provided.insert(parameter.name.clone());
    Some(())
}

fn static_integer_parameter(parameter: &FunctionParameter) -> bool {
    !parameter.by_reference && parameter.type_name == "int" && parameter.array_dims.is_empty()
}

fn evaluate_static_function(
    program: &SourceProgram,
    module: &SourceProgramModule,
    function: &FunctionDeclaration,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    let mut values = values.clone();
    match execute_static_statements(program, module, &function.statements, &mut values)? {
        StaticFlow::Continue => None,
        StaticFlow::Return(value) => Some(value),
        StaticFlow::Break | StaticFlow::LoopContinue => None,
    }
}

enum StaticFlow {
    Continue,
    Break,
    LoopContinue,
    Return(i128),
}

fn execute_static_statements(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statements: &[FunctionStatement],
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    for statement in statements {
        match execute_static_statement(program, module, statement, values)? {
            StaticFlow::Continue => {}
            flow => return Some(flow),
        }
    }
    Some(StaticFlow::Continue)
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
        FunctionStatementKind::Break => Some(StaticFlow::Break),
        FunctionStatementKind::Continue => Some(StaticFlow::LoopContinue),
        FunctionStatementKind::While => {
            let Some(body) = statement.body.as_ref() else {
                return Some(StaticFlow::Continue);
            };
            let condition = statement.header_expression.as_ref()?;
            for _ in 0..STATIC_LOOP_LIMIT {
                if evaluate_static_i128(program, condition, values)? == 0 {
                    return Some(StaticFlow::Continue);
                }
                match execute_static_body(program, module, body.start, body.end, values)? {
                    StaticFlow::Continue => {}
                    StaticFlow::LoopContinue => continue,
                    StaticFlow::Break => return Some(StaticFlow::Continue),
                    StaticFlow::Return(value) => return Some(StaticFlow::Return(value)),
                }
            }
            None
        }
        FunctionStatementKind::Do => {
            execute_static_do_statement(program, module, statement, values)
        }
        FunctionStatementKind::Switch => {
            execute_static_switch_statement(program, module, statement, values)
        }
        FunctionStatementKind::If => {
            execute_static_if_statement(program, module, statement, values)
        }
        FunctionStatementKind::For => {
            execute_static_for_statement(program, module, statement, values)
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
            if !declaration.array_dims.is_empty() {
                let elements = source_static_integer_array_expression(program, expression, values)?;
                insert_static_integer_array(values, &declaration.name, elements)?;
                return Some(());
            }
            let value = evaluate_static_i128(program, expression, values)?;
            values.insert(declaration.name.clone(), value);
        }
        FunctionStatementDeclaration::Variable(declaration) => {
            let expression = declaration.initializer_expression.as_ref()?;
            if !declaration.array_dims.is_empty() {
                let elements = source_static_integer_array_expression(program, expression, values)?;
                insert_static_integer_array(values, &declaration.name, elements)?;
                return Some(());
            }
            let value = evaluate_static_i128(program, expression, values)?;
            values.insert(declaration.name.clone(), value);
        }
        FunctionStatementDeclaration::Column(_) => return None,
    }
    Some(())
}

fn insert_static_integer_array(
    values: &mut BTreeMap<String, i128>,
    name: &str,
    elements: Vec<i128>,
) -> Option<()> {
    let length = i128::try_from(elements.len()).ok()?;
    values.insert(source_static_array_length_key(name), length);
    for (index, value) in elements.into_iter().enumerate() {
        values.insert(source_static_array_element_key(name, index), value);
    }
    Some(())
}

fn source_static_integer_array_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, i128>,
) -> Option<Vec<i128>> {
    match &expression.kind {
        ExpressionKind::Array(elements) => elements
            .iter()
            .map(|element| evaluate_static_i128(program, element, values))
            .collect(),
        ExpressionKind::Group(inner) => {
            source_static_integer_array_expression(program, inner, values)
        }
        ExpressionKind::Name(name) => {
            let length =
                usize::try_from(*values.get(&source_static_array_length_key(name))?).ok()?;
            (0..length)
                .map(|index| {
                    values
                        .get(&source_static_array_element_key(name, index))
                        .copied()
                })
                .collect()
        }
        _ => None,
    }
}

fn execute_static_body(
    program: &SourceProgram,
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    let tokens = lex_source(&module.source.contents).ok()?;
    let start_index = tokens.iter().position(|token| token.start == start)?;
    let end_index = tokens
        .iter()
        .position(|token| token.end == end)?
        .checked_add(1)?;
    let statements = if tokens
        .get(start_index)
        .is_some_and(|token| token.kind == TokenKind::LBrace)
    {
        parse_function_body_statements(&tokens, SourceSpan { start, end }, &module.source).ok()?
    } else {
        parse_function_statement_tokens(&tokens, start_index, end_index, &module.source).ok()?
    };
    execute_static_statements(program, module, &statements, values)
}

fn execute_static_statement_tokens(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    let statements = parse_function_statement_tokens(tokens, start, end, &module.source).ok()?;
    execute_static_statements(program, module, &statements, values)
}

fn execute_static_do_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    let body = statement.body?;
    let tokens = lex_source(&module.source.contents).ok()?;
    let close_after = tokens
        .iter()
        .position(|token| token.end == body.end)?
        .checked_add(1)?;
    if tokens.get(close_after).map(|token| token.kind) != Some(TokenKind::While) {
        return None;
    }
    let open = close_after.checked_add(1)?;
    if tokens.get(open).map(|token| token.kind) != Some(TokenKind::LParen) {
        return None;
    }
    let close = matching_closing_token(&tokens, open, tokens.len())?;
    if tokens.get(close.checked_add(1)?).map(|token| token.kind) != Some(TokenKind::Semicolon) {
        return None;
    }
    let (condition, consumed) =
        parse_expression_tokens(&tokens, open + 1, close, &module.source).ok()?;
    if consumed != close {
        return None;
    }
    for _ in 0..STATIC_LOOP_LIMIT {
        match execute_static_body(program, module, body.start, body.end, values)? {
            StaticFlow::Continue | StaticFlow::LoopContinue => {}
            StaticFlow::Break => return Some(StaticFlow::Continue),
            StaticFlow::Return(value) => return Some(StaticFlow::Return(value)),
        }
        if evaluate_static_i128(program, &condition, values)? == 0 {
            return Some(StaticFlow::Continue);
        }
    }
    None
}

fn execute_static_switch_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    let body = statement.body?;
    let condition = evaluate_static_i128(program, statement.header_expression.as_ref()?, values)?;
    let tokens = lex_source(&module.source.contents).ok()?;
    let open = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LBrace && token.start == body.start)?;
    let close = tokens
        .iter()
        .position(|token| token.kind == TokenKind::RBrace && token.end == body.end)?;
    let Some((start, end)) =
        static_switch_branch_range(program, module, &tokens, open + 1, close, values, condition)?
    else {
        return Some(StaticFlow::Continue);
    };
    match execute_static_statement_tokens(program, module, &tokens, start, end, values)? {
        StaticFlow::Break | StaticFlow::Continue => Some(StaticFlow::Continue),
        StaticFlow::LoopContinue => Some(StaticFlow::LoopContinue),
        StaticFlow::Return(value) => Some(StaticFlow::Return(value)),
    }
}

fn static_switch_branch_range(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &BTreeMap<String, i128>,
    condition: i128,
) -> Option<Option<(usize, usize)>> {
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
            default_branch.get_or_insert((branch_start, branch_end));
        } else if static_switch_case_matches(
            program,
            module,
            tokens,
            cursor + 1,
            colon,
            values,
            condition,
        )? {
            return Some(Some((branch_start, branch_end)));
        }
        cursor = branch_end;
    }
    Some(default_branch)
}

fn static_switch_case_matches(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &BTreeMap<String, i128>,
    condition: i128,
) -> Option<bool> {
    for (start, end) in static_switch_case_value_ranges(tokens, start, end)? {
        let (expression, consumed) =
            parse_expression_tokens(tokens, start, end, &module.source).ok()?;
        if consumed != end {
            return None;
        }
        if evaluate_static_i128(program, &expression, values)? == condition {
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

fn execute_static_if_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    let tokens = lex_source(&module.source.contents).ok()?;
    let start = tokens
        .iter()
        .position(|token| token.start == statement.start)?;
    let end = tokens
        .iter()
        .position(|token| token.end == statement.end)?
        .checked_add(1)?;
    match static_if_body_span(program, module, &tokens, start, end, values)? {
        Some(body) => execute_static_body_span(program, module, &tokens, &body, values),
        None => Some(StaticFlow::Continue),
    }
}

fn static_if_body_span(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &BTreeMap<String, i128>,
) -> Option<Option<StaticBodySpan>> {
    if !matches!(
        tokens.get(index).map(|token| token.kind),
        Some(TokenKind::If | TokenKind::ElseIf)
    ) {
        return None;
    }
    let open = next_token_kind(tokens, index + 1, end, TokenKind::LParen)?;
    let close = matching_closing_token(tokens, open, end)?;
    let (condition, consumed) =
        parse_expression_tokens(tokens, open + 1, close, &module.source).ok()?;
    if consumed != close {
        return None;
    }
    let body = static_control_body_span(tokens, close + 1, end)?;
    if evaluate_static_i128(program, &condition, values)? != 0 {
        return Some(Some(body));
    }
    static_else_body_span(program, module, tokens, body.after, end, values)
}

fn static_else_body_span(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &BTreeMap<String, i128>,
) -> Option<Option<StaticBodySpan>> {
    match tokens.get(index).map(|token| token.kind) {
        Some(TokenKind::ElseIf) => static_if_body_span(program, module, tokens, index, end, values),
        Some(TokenKind::Else)
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::If) =>
        {
            static_if_body_span(program, module, tokens, index + 1, end, values)
        }
        Some(TokenKind::Else) => {
            let body = static_control_body_span(tokens, index + 1, end)?;
            Some(Some(body))
        }
        _ => Some(None),
    }
}

fn execute_static_body_span(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    body: &StaticBodySpan,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    if body.braced {
        return execute_static_body(program, module, body.span.start, body.span.end, values);
    }
    let start = tokens
        .iter()
        .position(|token| token.start == body.span.start)?;
    let end = tokens
        .iter()
        .position(|token| token.end == body.span.end)?
        .checked_add(1)?;
    execute_static_statement_tokens(program, module, tokens, start, end, values)
}

struct StaticBodySpan {
    span: SourceSpan,
    braced: bool,
    after: usize,
}

fn static_control_body_span(tokens: &[Token], index: usize, end: usize) -> Option<StaticBodySpan> {
    match tokens.get(index)?.kind {
        TokenKind::LBrace => {
            let close = matching_closing_token(tokens, index, end)?;
            Some(StaticBodySpan {
                span: SourceSpan {
                    start: tokens[index].start,
                    end: tokens[close].end,
                },
                braced: true,
                after: close + 1,
            })
        }
        _ => {
            let semicolon = next_static_semicolon_limited(tokens, index, end)?;
            Some(StaticBodySpan {
                span: SourceSpan {
                    start: tokens[index].start,
                    end: tokens[semicolon].end,
                },
                braced: false,
                after: semicolon + 1,
            })
        }
    }
}

fn execute_static_for_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    let variable = static_for_loop_variable(statement)?;
    let initializer = variable.initializer_expression.as_ref()?;
    let initial_value = evaluate_static_i128(program, initializer, values)?;
    let tokens = lex_source(&module.source.contents).ok()?;
    let header = statement.header?;
    let (condition, update) = static_for_loop_header(&tokens, header, module)?;
    let body = static_control_body_after_header(&tokens, header, statement.end)?;
    let previous = values.insert(variable.name.clone(), initial_value);
    let loop_body = StaticForLoop {
        body,
        condition: &condition,
        update: &update,
        variable_name: &variable.name,
    };
    let flow = execute_static_for_loop(program, module, &tokens, loop_body, values);
    match previous {
        Some(value) => {
            values.insert(variable.name.clone(), value);
        }
        None => {
            values.remove(&variable.name);
        }
    }
    flow
}

struct StaticForLoop<'a> {
    body: StaticBodySpan,
    condition: &'a Expression,
    update: &'a StaticForLoopUpdate,
    variable_name: &'a str,
}

fn execute_static_for_loop(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    loop_body: StaticForLoop<'_>,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    for _ in 0..STATIC_LOOP_LIMIT {
        if evaluate_static_i128(program, loop_body.condition, values)? == 0 {
            return Some(StaticFlow::Continue);
        }
        match execute_static_body_span(program, module, tokens, &loop_body.body, values)? {
            StaticFlow::Continue | StaticFlow::LoopContinue => {}
            StaticFlow::Break => return Some(StaticFlow::Continue),
            StaticFlow::Return(value) => return Some(StaticFlow::Return(value)),
        }
        apply_static_for_loop_update(program, loop_body.update, loop_body.variable_name, values)?;
    }
    None
}

fn static_control_body_after_header(
    tokens: &[Token],
    header: SourceSpan,
    statement_end: usize,
) -> Option<StaticBodySpan> {
    let after_header = tokens
        .iter()
        .position(|token| token.end == header.end)?
        .checked_add(1)?;
    let end = tokens
        .iter()
        .position(|token| token.end == statement_end)?
        .checked_add(1)?;
    static_control_body_span(tokens, after_header, end)
}

fn static_for_loop_variable(
    statement: &FunctionStatement,
) -> Option<&lzvm_pil::VariableDeclaration> {
    let Some(FunctionStatementDeclaration::Variable(declaration)) =
        statement.header_declaration.as_ref()
    else {
        return None;
    };
    if declaration.type_name != "int" || !declaration.array_dims.is_empty() {
        return None;
    }
    Some(declaration)
}

fn static_for_loop_header(
    tokens: &[Token],
    header: SourceSpan,
    module: &SourceProgramModule,
) -> Option<(Expression, StaticForLoopUpdate)> {
    let open = tokens
        .iter()
        .position(|token| token.start == header.start)?;
    let close = tokens.iter().position(|token| token.end == header.end)?;
    let semicolons = static_for_header_semicolons(tokens, open + 1, close);
    let [first_semicolon, second_semicolon] = semicolons.as_slice() else {
        return None;
    };
    let (condition, consumed) = parse_expression_tokens(
        tokens,
        first_semicolon + 1,
        *second_semicolon,
        &module.source,
    )
    .ok()?;
    if consumed != *second_semicolon {
        return None;
    }
    let update = static_for_loop_update(tokens, second_semicolon + 1, close, module)?;
    Some((condition, update))
}

fn static_for_header_semicolons(tokens: &[Token], start: usize, end: usize) -> Vec<usize> {
    let mut semicolons = Vec::new();
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                depth = depth.saturating_add(1);
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Semicolon if depth == 0 => semicolons.push(index),
            _ => {}
        }
    }
    semicolons
}

struct StaticForLoopUpdate {
    expression: Option<Expression>,
    postfix: Option<StaticForLoopPostfixUpdate>,
}

struct StaticForLoopPostfixUpdate {
    name: String,
    delta: i128,
}

fn static_for_loop_update(
    tokens: &[Token],
    start: usize,
    end: usize,
    module: &SourceProgramModule,
) -> Option<StaticForLoopUpdate> {
    if let Some(postfix) = static_for_loop_postfix_update(tokens, start, end) {
        return Some(StaticForLoopUpdate {
            expression: None,
            postfix: Some(postfix),
        });
    }
    let (expression, consumed) =
        parse_expression_tokens(tokens, start, end, &module.source).ok()?;
    if consumed != end {
        return None;
    }
    Some(StaticForLoopUpdate {
        expression: Some(expression),
        postfix: None,
    })
}

fn static_for_loop_postfix_update(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Option<StaticForLoopPostfixUpdate> {
    let first = tokens.get(start)?;
    let second = tokens.get(start + 1)?;
    if start + 2 != end {
        return None;
    }
    match (first.kind, second.kind) {
        (TokenKind::Identifier, TokenKind::Increment) => Some(StaticForLoopPostfixUpdate {
            name: first.lexeme.clone(),
            delta: 1,
        }),
        (TokenKind::Identifier, TokenKind::Decrement) => Some(StaticForLoopPostfixUpdate {
            name: first.lexeme.clone(),
            delta: -1,
        }),
        _ => None,
    }
}

fn apply_static_for_loop_update(
    program: &SourceProgram,
    update: &StaticForLoopUpdate,
    variable_name: &str,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    if let Some(postfix) = &update.postfix {
        return apply_static_for_loop_delta(variable_name, &postfix.name, postfix.delta, values);
    }
    let expression = update.expression.as_ref()?;
    match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            let name = static_expression_name(expr)?;
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => return None,
            };
            apply_static_for_loop_delta(variable_name, name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let name = static_expression_name(left)?;
            if name != variable_name {
                return None;
            }
            let right = evaluate_static_i128(program, right, values)?;
            let current = values.get(variable_name).copied()?;
            let next = match op {
                BinaryOperator::Assign => Some(right),
                BinaryOperator::PlusAssign => current.checked_add(right),
                BinaryOperator::MinusAssign => current.checked_sub(right),
                BinaryOperator::StarAssign => current.checked_mul(right),
                _ => None,
            }?;
            values.insert(variable_name.to_owned(), next);
            Some(())
        }
        _ => None,
    }
}

fn apply_static_for_loop_delta(
    variable_name: &str,
    update_name: &str,
    delta: i128,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    if update_name != variable_name {
        return None;
    }
    let current = values.get(variable_name).copied()?;
    values.insert(variable_name.to_owned(), current.checked_add(delta)?);
    Some(())
}

fn execute_static_expression_statement(
    program: &SourceProgram,
    expression: &Expression,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            let name = static_expression_name(expr)?;
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => return None,
            };
            execute_static_delta(name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let name = static_expression_name(left)?.to_owned();
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

fn static_expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => static_expression_name(inner),
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

#[cfg(test)]
mod tests {
    use std::fs;

    use lzvm_pil::{lex_source, parse_expression_tokens, SourceLoaderConfig, SourceProgramLoader};

    use super::*;

    #[test]
    fn evaluates_static_return_functions_after_assert_calls() {
        let dir = std::env::temp_dir().join(format!(
            "lzvm-setup-static-functions-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("fixture directory should be created");
        fs::write(
            dir.join("main.pil"),
            "function slot(const int x, const int y, const int z): int {\n\
                 assert(x >= 0);\n\
                 assert(y >= 0);\n\
                 assert(z >= 0);\n\
                 return 64 * x + 320 * y + z;\n\
             }",
        )
        .expect("fixture should be written");

        let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
            working_dir: dir.clone(),
            include_paths: Vec::new(),
            include_path_first: false,
        });
        let program = loader
            .load_main("main.pil")
            .expect("source program should parse");
        let source = &program.modules[0].source;
        let tokens = lex_source("slot(2, 3, 4)").expect("expression should lex");
        let (expression, consumed) = parse_expression_tokens(&tokens, 0, tokens.len(), source)
            .expect("expression should parse");
        assert_eq!(consumed, tokens.len());

        let value = evaluate_static_i128(&program, &expression, &BTreeMap::new());

        fs::remove_dir_all(&dir).expect("fixture directory should be removed");
        assert_eq!(value, Some(1092));
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

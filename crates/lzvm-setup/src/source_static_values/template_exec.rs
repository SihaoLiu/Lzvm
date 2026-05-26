use std::collections::BTreeMap;

use lzvm_pil::{
    lex_source, parse_expression_tokens, BinaryOperator, Expression, ExpressionKind,
    FixedFileTemplateValue, SourceProgram, SourceProgramModule, Token, TokenKind, UnaryOperator,
};

use crate::source_static_tokens::{
    control_body_range, matching_closing_token, next_static_semicolon_limited, next_token_kind,
    skip_static_do_while_statement, static_switch_case_value_ranges, static_switch_label_colon,
    update_static_delimiter_stack, StaticTemplateFlow, StaticTemplateStatementResult,
};

use super::{
    evaluate_source_static_expression, evaluate_source_static_token_range, expression_name,
    source_static_assignment_target_visible, static_declaration_start, static_value_integer,
    static_value_truthy, STATIC_TEMPLATE_LOOP_LIMIT,
};

pub(crate) fn execute_static_template_range(
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
    execute_static_template_tokens(program, module, &tokens, start, end, values).map(|_| ())
}

fn execute_static_template_tokens(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    mut index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<StaticTemplateFlow> {
    while index < end {
        match tokens.get(index).map(|token| token.kind) {
            Some(TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon) => {
                index += 1;
            }
            Some(TokenKind::Break) => {
                next_static_semicolon_limited(tokens, index, end)?;
                return Some(StaticTemplateFlow::Break);
            }
            Some(TokenKind::Continue) => {
                next_static_semicolon_limited(tokens, index, end)?;
                return Some(StaticTemplateFlow::Continue);
            }
            Some(TokenKind::EndOfInput) | None => break,
            _ => {
                let result =
                    execute_static_template_statement(program, module, tokens, index, end, values)
                        .or_else(|| {
                            skip_static_template_statement(tokens, index, end)
                                .map(StaticTemplateStatementResult::fallthrough)
                        })
                        .filter(|result| result.next > index)
                        .unwrap_or_else(|| StaticTemplateStatementResult::fallthrough(index + 1));
                if result.flow != StaticTemplateFlow::Fallthrough {
                    return Some(result.flow);
                }
                index = result.next;
            }
        }
    }
    Some(StaticTemplateFlow::Fallthrough)
}

fn execute_static_template_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<StaticTemplateStatementResult> {
    match tokens.get(index)?.kind {
        TokenKind::If => execute_static_template_if(program, module, tokens, index, end, values),
        TokenKind::For => execute_static_template_for(program, module, tokens, index, end, values),
        TokenKind::While => {
            execute_static_template_while(program, module, tokens, index, end, values)
        }
        TokenKind::Do => {
            execute_static_template_do_while(program, module, tokens, index, end, values)
        }
        TokenKind::Switch => {
            execute_static_template_switch(program, module, tokens, index, end, values)
        }
        kind if static_declaration_start(kind) => {
            crate::source_static_declarations::execute_static_template_declaration(
                program, module, tokens, index, values,
            );
            skip_static_template_statement(tokens, index, end)
                .map(StaticTemplateStatementResult::fallthrough)
        }
        _ => {
            let semicolon = next_static_semicolon_limited(tokens, index, end)?;
            if !static_statement_contains_assignment_operator(tokens, index, semicolon) {
                return Some(StaticTemplateStatementResult::fallthrough(semicolon + 1));
            }
            if crate::source_static_array_assignment::execute_source_static_array_assignment_statement(
                program, module, tokens, index, semicolon, values,
            )
            .is_some()
            {
                return Some(StaticTemplateStatementResult::fallthrough(semicolon + 1));
            }
            if unsupported_static_assignment_statement(tokens, index, semicolon) {
                return Some(StaticTemplateStatementResult::fallthrough(semicolon + 1));
            }
            if execute_source_static_postfix_update(
                program, module, tokens, index, semicolon, values,
            )
            .is_some()
            {
                return Some(StaticTemplateStatementResult::fallthrough(semicolon + 1));
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
            Some(StaticTemplateStatementResult::fallthrough(semicolon + 1))
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
) -> Option<StaticTemplateStatementResult> {
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
        let flow =
            execute_static_template_tokens(program, module, tokens, body_start, body_end, values)?;
        if flow != StaticTemplateFlow::Fallthrough {
            return Some(StaticTemplateStatementResult::control(after_body, flow));
        }
        return skip_static_else_tail(tokens, after_body, end)
            .map(StaticTemplateStatementResult::fallthrough);
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
) -> Option<StaticTemplateStatementResult> {
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
) -> Option<StaticTemplateStatementResult> {
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
            return Some(StaticTemplateStatementResult::fallthrough(after_body));
        }
        let flow =
            execute_static_template_tokens(program, module, tokens, body_start, body_end, values)?;
        if flow == StaticTemplateFlow::Break {
            values.remove(&loop_variable);
            return Some(StaticTemplateStatementResult::fallthrough(after_body));
        }
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
) -> Option<StaticTemplateStatementResult> {
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
) -> Option<StaticTemplateStatementResult> {
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
            return Some(StaticTemplateStatementResult::fallthrough(after_body));
        }
        let flow =
            execute_static_template_tokens(program, module, tokens, body_start, body_end, values)?;
        if flow == StaticTemplateFlow::Break {
            return Some(StaticTemplateStatementResult::fallthrough(after_body));
        }
    }
    None
}

fn execute_static_template_do_while(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<StaticTemplateStatementResult> {
    let checkpoint = values.clone();
    let next = execute_static_template_do_while_inner(program, module, tokens, index, end, values);
    if next.is_none() {
        *values = checkpoint;
    }
    next
}

fn execute_static_template_do_while_inner(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<StaticTemplateStatementResult> {
    let (body_start, body_end, after_body) = control_body_range(tokens, index + 1, end)?;
    if tokens.get(after_body).map(|token| token.kind) != Some(TokenKind::While) {
        return None;
    }
    let open = after_body + 1;
    if tokens.get(open).map(|token| token.kind) != Some(TokenKind::LParen) {
        return None;
    }
    let close = matching_closing_token(tokens, open, end)?;
    let semicolon = close + 1;
    if tokens.get(semicolon).map(|token| token.kind) != Some(TokenKind::Semicolon) {
        return None;
    }

    for _ in 0..STATIC_TEMPLATE_LOOP_LIMIT {
        let flow =
            execute_static_template_tokens(program, module, tokens, body_start, body_end, values)?;
        if flow == StaticTemplateFlow::Break {
            return Some(StaticTemplateStatementResult::fallthrough(semicolon + 1));
        }
        let condition = evaluate_source_static_token_range(
            program,
            &module.source,
            tokens,
            open + 1,
            close,
            values,
        )?;
        if !static_value_truthy(&condition) {
            return Some(StaticTemplateStatementResult::fallthrough(semicolon + 1));
        }
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
) -> Option<StaticTemplateStatementResult> {
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
) -> Option<StaticTemplateStatementResult> {
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
        let flow =
            execute_static_switch_branch(program, module, tokens, branch_start, body_end, values)?;
        if flow == StaticTemplateFlow::Continue {
            return Some(StaticTemplateStatementResult::control(after_body, flow));
        }
    }
    Some(StaticTemplateStatementResult::fallthrough(after_body))
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
                    if static_switch_case_matches(
                        program,
                        module,
                        tokens,
                        index + 1,
                        colon,
                        values,
                        condition,
                    )? {
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

fn static_switch_case_matches(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    start: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    condition: &FixedFileTemplateValue,
) -> Option<bool> {
    for (start, end) in static_switch_case_value_ranges(tokens, start, end)? {
        let value = evaluate_source_static_token_range(
            program,
            &module.source,
            tokens,
            start,
            end,
            values,
        )?;
        if value == *condition {
            return Some(true);
        }
    }
    Some(false)
}

fn execute_static_switch_branch(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    mut index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<StaticTemplateFlow> {
    while index < end {
        match tokens.get(index).map(|token| token.kind) {
            Some(TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon) => {
                index += 1;
            }
            Some(TokenKind::Case | TokenKind::Default) => {
                return Some(StaticTemplateFlow::Fallthrough);
            }
            Some(TokenKind::Break) => {
                next_static_semicolon_limited(tokens, index, end)?;
                return Some(StaticTemplateFlow::Break);
            }
            Some(TokenKind::Continue) => {
                next_static_semicolon_limited(tokens, index, end)?;
                return Some(StaticTemplateFlow::Continue);
            }
            Some(TokenKind::EndOfInput) | None => break,
            _ => {
                let result =
                    execute_static_template_statement(program, module, tokens, index, end, values)
                        .or_else(|| {
                            skip_static_template_statement(tokens, index, end)
                                .map(StaticTemplateStatementResult::fallthrough)
                        })
                        .filter(|result| result.next > index)
                        .unwrap_or_else(|| StaticTemplateStatementResult::fallthrough(index + 1));
                if result.flow != StaticTemplateFlow::Fallthrough {
                    return Some(result.flow);
                }
                index = result.next;
            }
        }
    }
    Some(StaticTemplateFlow::Fallthrough)
}

fn execute_static_else_tail(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    end: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<StaticTemplateStatementResult> {
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
            let flow = execute_static_template_tokens(
                program, module, tokens, body_start, body_end, values,
            )?;
            Some(StaticTemplateStatementResult::control(after_body, flow))
        }
        _ => Some(StaticTemplateStatementResult::fallthrough(index)),
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
        TokenKind::Do => skip_static_do_while_statement(tokens, index, end),
        _ => next_static_semicolon_limited(tokens, index, end).map(|semicolon| semicolon + 1),
    }
}

fn skip_static_if_statement(tokens: &[Token], index: usize, end: usize) -> Option<usize> {
    let open = next_token_kind(tokens, index + 1, end, TokenKind::LParen)?;
    let close = matching_closing_token(tokens, open, end)?;
    let (_, _, after_body) = control_body_range(tokens, close + 1, end)?;
    skip_static_else_tail(tokens, after_body, end)
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

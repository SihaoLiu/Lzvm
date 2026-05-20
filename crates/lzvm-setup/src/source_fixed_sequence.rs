use std::path::PathBuf;

use lzvm_field::{Felt, MODULUS};
use lzvm_pil::{
    lex_source, parse_expression, FixedFileTemplateValue, SourceFile, SourceProgram, SourceSpan,
    Token, TokenKind,
};

use crate::source_fixed_columns::SourceFixedColumnsWriteError;
use crate::source_fixed_expression::{
    evaluate_source_fixed_template_value_expression, SourceFixedConstantValues,
};
use crate::source_static_values::evaluate_source_static_expression;

pub(crate) fn parse_literal_sequence(
    program: &SourceProgram,
    source_name: &str,
    source_span: SourceSpan,
    source: &str,
    row_count: usize,
    constant_values: &SourceFixedConstantValues,
) -> Result<Vec<u64>, SourceFixedColumnsWriteError> {
    parse_literal_sequence_values(
        program,
        source_name,
        source_span,
        source,
        row_count,
        constant_values,
    )?
    .into_iter()
    .map(|value| canonical_fixed_value(value, source_name, source_span))
    .collect()
}

pub(crate) fn parse_literal_sequence_values(
    program: &SourceProgram,
    source_name: &str,
    source_span: SourceSpan,
    source: &str,
    row_count: usize,
    constant_values: &SourceFixedConstantValues,
) -> Result<Vec<i128>, SourceFixedColumnsWriteError> {
    let tokens = lex_source(source).map_err(|source| SourceFixedColumnsWriteError::Lex {
        source_name: source_name.to_owned(),
        source_span,
        source,
    })?;
    let mut cursor = 0_usize;
    expect_token(
        &tokens,
        &mut cursor,
        TokenKind::LBracket,
        source_name,
        source_span,
    )?;
    let context = SequenceParseContext {
        program,
        source_name,
        source_span,
        source,
        tokens: &tokens,
        constant_values,
    };
    let mut values = Vec::<i128>::new();

    let mut segment_start = cursor;
    let mut pending_progression = None::<(TokenKind, usize)>;
    let mut stack = Vec::new();
    while let Some(token) = tokens.get(cursor) {
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                        source_name: source_name.to_owned(),
                        source_span,
                        token: token.lexeme.clone(),
                    });
                };
                if expected != token.kind {
                    return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                        source_name: source_name.to_owned(),
                        source_span,
                        token: token.lexeme.clone(),
                    });
                }
            }
            TokenKind::Comma if stack.is_empty() => {
                if let Some((kind, marker_start)) = pending_progression.take() {
                    append_sequence_comma_progression(
                        &mut values,
                        &context,
                        kind,
                        marker_start,
                        segment_start,
                        cursor,
                        row_count,
                    )?;
                } else if let Some(kind) =
                    comma_delimited_progression_marker(&context, segment_start, cursor)?
                {
                    pending_progression = Some((kind, segment_start));
                } else {
                    append_sequence_segment(
                        &mut values,
                        &context,
                        segment_start,
                        cursor,
                        row_count,
                        false,
                    )?;
                }
                segment_start = cursor + 1;
            }
            TokenKind::RBracket if stack.is_empty() => {
                if cursor > segment_start {
                    if let Some((kind, marker_start)) = pending_progression.take() {
                        append_sequence_comma_progression(
                            &mut values,
                            &context,
                            kind,
                            marker_start,
                            segment_start,
                            cursor,
                            row_count,
                        )?;
                    } else if let Some(kind) =
                        comma_delimited_progression_marker(&context, segment_start, cursor)?
                    {
                        append_sequence_comma_progression_fill(
                            &mut values,
                            &context,
                            kind,
                            segment_start,
                            row_count,
                        )?;
                    } else {
                        append_sequence_segment(
                            &mut values,
                            &context,
                            segment_start,
                            cursor,
                            row_count,
                            true,
                        )?;
                    }
                } else if !values.is_empty() {
                    return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                        source_name: source_name.to_owned(),
                        source_span,
                        token: token.lexeme.clone(),
                    });
                }
                cursor += 1;
                break;
            }
            TokenKind::RBracket => {
                let Some(expected) = stack.pop() else {
                    return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                        source_name: source_name.to_owned(),
                        source_span,
                        token: token.lexeme.clone(),
                    });
                };
                if expected != TokenKind::RBracket {
                    return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                        source_name: source_name.to_owned(),
                        source_span,
                        token: token.lexeme.clone(),
                    });
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    if cursor == tokens.len()
        && !matches!(
            tokens.last().map(|token| token.kind),
            Some(TokenKind::RBracket)
        )
    {
        return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: source_name.to_owned(),
            source_span,
            token: "<end>".to_owned(),
        });
    }
    if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Ellipsis)
    {
        cursor += 1;
        fill_sequence_pattern(&mut values, row_count, source_name, source_span)?;
    }
    expect_end(&tokens, cursor, source_name, source_span)?;
    Ok(values)
}

fn fill_sequence_pattern(
    values: &mut Vec<i128>,
    row_count: usize,
    source_name: &str,
    source_span: SourceSpan,
) -> Result<(), SourceFixedColumnsWriteError> {
    if values.is_empty() {
        return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: source_name.to_owned(),
            source_span,
            token: "...".to_owned(),
        });
    }
    let pattern = values.clone();
    let mut index = 0_usize;
    while values.len() < row_count {
        values.push(pattern[index]);
        index = (index + 1) % pattern.len();
    }
    Ok(())
}

struct SequenceParseContext<'a> {
    program: &'a SourceProgram,
    source_name: &'a str,
    source_span: SourceSpan,
    source: &'a str,
    tokens: &'a [Token],
    constant_values: &'a SourceFixedConstantValues,
}

struct ProgressionSegment<'a, 'b> {
    context: &'a SequenceParseContext<'b>,
    start: usize,
    end: usize,
    row_count: usize,
}

fn append_sequence_segment(
    values: &mut Vec<i128>,
    context: &SequenceParseContext<'_>,
    start: usize,
    end: usize,
    row_count: usize,
    allow_fill: bool,
) -> Result<(), SourceFixedColumnsWriteError> {
    if context
        .tokens
        .get(end.saturating_sub(1))
        .is_some_and(|token| token.kind == TokenKind::Ellipsis)
    {
        if !allow_fill || start + 1 >= end {
            let token = context
                .tokens
                .get(end.saturating_sub(1))
                .map(|token| token.lexeme.clone())
                .unwrap_or_else(|| "...".to_owned());
            return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                source_name: context.source_name.to_owned(),
                source_span: context.source_span,
                token,
            });
        }
        let value = parse_sequence_expression(context, start, end - 1)?;
        while values.len() < row_count {
            values.push(value);
        }
        return Ok(());
    }

    if let Some(range_index) = top_level_range_index(context, start, end)? {
        append_sequence_range(values, context, start, range_index, end, row_count)?;
        return Ok(());
    }

    if let Some(progression_index) =
        top_level_token_index(context, start, end, TokenKind::RangeFill)?
    {
        append_sequence_add_progression(values, context, start, progression_index, end, row_count)?;
        return Ok(());
    }

    if let Some(progression_index) =
        top_level_token_index(context, start, end, TokenKind::RangeMulFill)?
    {
        append_sequence_mul_progression(values, context, start, progression_index, end, row_count)?;
        return Ok(());
    }

    if let Some(repeat_index) = top_level_token_index(context, start, end, TokenKind::Colon)? {
        append_sequence_repeat(values, context, start, repeat_index, end, row_count)?;
        return Ok(());
    }

    values.extend(parse_sequence_values(context, start, end, row_count)?);
    Ok(())
}

fn comma_delimited_progression_marker(
    context: &SequenceParseContext<'_>,
    start: usize,
    end: usize,
) -> Result<Option<TokenKind>, SourceFixedColumnsWriteError> {
    if end != start + 1 {
        return Ok(None);
    }
    let Some(token) = context.tokens.get(start) else {
        return Ok(None);
    };
    match token.kind {
        TokenKind::RangeFill | TokenKind::RangeMulFill => Ok(Some(token.kind)),
        TokenKind::Range | TokenKind::Ellipsis => {
            Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                source_name: context.source_name.to_owned(),
                source_span: context.source_span,
                token: token.lexeme.clone(),
            })
        }
        _ => Ok(None),
    }
}

fn append_sequence_comma_progression(
    values: &mut Vec<i128>,
    context: &SequenceParseContext<'_>,
    kind: TokenKind,
    marker_start: usize,
    value_start: usize,
    value_end: usize,
    row_count: usize,
) -> Result<(), SourceFixedColumnsWriteError> {
    if value_start >= value_end {
        return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            token: context.tokens[marker_start].lexeme.clone(),
        });
    }
    let (previous, current) = comma_progression_seed(values, context, marker_start)?;
    let last = parse_sequence_expression(context, value_start, value_end)?;
    let segment = ProgressionSegment {
        context,
        start: marker_start,
        end: value_end,
        row_count,
    };
    match kind {
        TokenKind::RangeFill => {
            let step = current.checked_sub(previous).ok_or_else(|| {
                SourceFixedColumnsWriteError::IntegerOutOfRange {
                    source_name: context.source_name.to_owned(),
                    source_span: context.source_span,
                    expression: progression_expression(&segment),
                }
            })?;
            append_comma_add_progression_values(values, &segment, current, last, step)
        }
        TokenKind::RangeMulFill => {
            append_comma_mul_progression_values(values, &segment, previous, current, last)
        }
        _ => Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            token: context.tokens[marker_start].lexeme.clone(),
        }),
    }
}

fn append_sequence_comma_progression_fill(
    values: &mut Vec<i128>,
    context: &SequenceParseContext<'_>,
    kind: TokenKind,
    marker_start: usize,
    row_count: usize,
) -> Result<(), SourceFixedColumnsWriteError> {
    let (previous, current) = comma_progression_seed(values, context, marker_start)?;
    let segment = ProgressionSegment {
        context,
        start: marker_start,
        end: marker_start + 1,
        row_count,
    };
    match kind {
        TokenKind::RangeFill => {
            let step = current.checked_sub(previous).ok_or_else(|| {
                SourceFixedColumnsWriteError::IntegerOutOfRange {
                    source_name: context.source_name.to_owned(),
                    source_span: context.source_span,
                    expression: progression_expression(&segment),
                }
            })?;
            append_comma_add_progression_fill(values, &segment, current, step)
        }
        TokenKind::RangeMulFill => {
            append_comma_mul_progression_fill(values, &segment, previous, current)
        }
        _ => Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            token: context.tokens[marker_start].lexeme.clone(),
        }),
    }
}

fn comma_progression_seed(
    values: &[i128],
    context: &SequenceParseContext<'_>,
    marker_start: usize,
) -> Result<(i128, i128), SourceFixedColumnsWriteError> {
    if values.len() < 2 {
        return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            token: context.tokens[marker_start].lexeme.clone(),
        });
    }
    Ok((values[values.len() - 2], values[values.len() - 1]))
}

fn append_comma_add_progression_fill(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    step: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    let mut value = current;
    while values.len() < segment.row_count {
        value = value.checked_add(step).ok_or_else(|| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: segment.context.source_name.to_owned(),
                source_span: segment.context.source_span,
                expression: progression_expression(segment),
            }
        })?;
        push_progression_value(values, segment, value)?;
    }
    Ok(())
}

fn append_comma_add_progression_values(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    last: i128,
    step: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if (step > 0 && current > last)
        || (step < 0 && current < last)
        || (step == 0 && current != last)
    {
        return Err(progression_unsupported(segment));
    }

    let mut value = current;
    while value != last {
        value = value.checked_add(step).ok_or_else(|| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: segment.context.source_name.to_owned(),
                source_span: segment.context.source_span,
                expression: progression_expression(segment),
            }
        })?;
        if (step > 0 && value > last) || (step < 0 && value < last) {
            return Err(progression_unsupported(segment));
        }
        push_progression_value(values, segment, value)?;
    }
    Ok(())
}

fn append_comma_mul_progression_values(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    previous: i128,
    current: i128,
    last: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if progression_uses_omega(segment) {
        return append_comma_mul_progression_values_field(values, segment, previous, current, last);
    }
    if let Some(factor) = comma_mul_factor(previous, current, segment)? {
        return append_comma_mul_factor_values(values, segment, current, last, factor);
    }
    let Some(divisor) = comma_mul_divisor(previous, current, segment)? else {
        return Err(progression_unsupported(segment));
    };
    append_comma_div_progression_values(values, segment, current, last, divisor)
}

fn append_comma_mul_progression_fill(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    previous: i128,
    current: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if progression_uses_omega(segment) {
        return append_comma_mul_progression_fill_field(values, segment, previous, current);
    }
    if let Some(factor) = comma_mul_factor(previous, current, segment)? {
        return append_comma_mul_factor_fill(values, segment, current, factor);
    }
    let Some(divisor) = comma_mul_divisor(previous, current, segment)? else {
        return Err(progression_unsupported(segment));
    };
    append_comma_div_progression_fill(values, segment, current, divisor)
}

fn append_comma_mul_progression_fill_field(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    previous: i128,
    current: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    let factor = field_progression_factor(previous, current, segment)?;
    let mut value = field_progression_value(current, segment)?;
    while values.len() < segment.row_count {
        value = value * factor;
        push_progression_value(values, segment, i128::from(value.to_u64()))?;
    }
    Ok(())
}

fn append_comma_mul_progression_values_field(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    previous: i128,
    current: i128,
    last: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    let factor = field_progression_factor(previous, current, segment)?;
    let mut value = field_progression_value(current, segment)?;
    let last = field_progression_value(last, segment)?;
    while value != last {
        value = value * factor;
        push_progression_value(values, segment, i128::from(value.to_u64()))?;
    }
    Ok(())
}

fn comma_mul_factor(
    previous: i128,
    current: i128,
    segment: &ProgressionSegment<'_, '_>,
) -> Result<Option<u128>, SourceFixedColumnsWriteError> {
    if previous > 0 && current >= previous && current % previous == 0 {
        return u128::try_from(current / previous).map(Some).map_err(|_| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: segment.context.source_name.to_owned(),
                source_span: segment.context.source_span,
                expression: progression_expression(segment),
            }
        });
    }
    Ok(None)
}

fn comma_mul_divisor(
    previous: i128,
    current: i128,
    segment: &ProgressionSegment<'_, '_>,
) -> Result<Option<u128>, SourceFixedColumnsWriteError> {
    if current > 0 && previous > current && previous % current == 0 {
        return u128::try_from(previous / current).map(Some).map_err(|_| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: segment.context.source_name.to_owned(),
                source_span: segment.context.source_span,
                expression: progression_expression(segment),
            }
        });
    }
    Ok(None)
}

fn append_comma_mul_factor_fill(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    factor: u128,
) -> Result<(), SourceFixedColumnsWriteError> {
    let mut value = current;
    let factor =
        i128::try_from(factor).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: segment.context.source_name.to_owned(),
            source_span: segment.context.source_span,
            expression: progression_expression(segment),
        })?;
    while values.len() < segment.row_count {
        value = value.checked_mul(factor).ok_or_else(|| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: segment.context.source_name.to_owned(),
                source_span: segment.context.source_span,
                expression: progression_expression(segment),
            }
        })?;
        push_progression_value(values, segment, value)?;
    }
    Ok(())
}

fn append_comma_mul_factor_values(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    last: i128,
    factor: u128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if factor <= 1 && current != last {
        return Err(progression_unsupported(segment));
    }
    let mut value = current;
    let factor =
        i128::try_from(factor).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: segment.context.source_name.to_owned(),
            source_span: segment.context.source_span,
            expression: progression_expression(segment),
        })?;
    while value != last {
        value = value.checked_mul(factor).ok_or_else(|| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: segment.context.source_name.to_owned(),
                source_span: segment.context.source_span,
                expression: progression_expression(segment),
            }
        })?;
        if value > last {
            return Err(progression_unsupported(segment));
        }
        push_progression_value(values, segment, value)?;
    }
    Ok(())
}

fn append_comma_div_progression_fill(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    divisor: u128,
) -> Result<(), SourceFixedColumnsWriteError> {
    let mut value = current;
    let divisor =
        i128::try_from(divisor).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: segment.context.source_name.to_owned(),
            source_span: segment.context.source_span,
            expression: progression_expression(segment),
        })?;
    while values.len() < segment.row_count {
        if value % divisor != 0 {
            return Err(progression_unsupported(segment));
        }
        value /= divisor;
        push_progression_value(values, segment, value)?;
    }
    Ok(())
}

fn append_comma_div_progression_values(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    last: i128,
    divisor: u128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if divisor <= 1 && current != last {
        return Err(progression_unsupported(segment));
    }
    let mut value = current;
    let divisor =
        i128::try_from(divisor).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: segment.context.source_name.to_owned(),
            source_span: segment.context.source_span,
            expression: progression_expression(segment),
        })?;
    while value != last {
        if value % divisor != 0 {
            return Err(progression_unsupported(segment));
        }
        value /= divisor;
        if value < last {
            return Err(progression_unsupported(segment));
        }
        push_progression_value(values, segment, value)?;
    }
    Ok(())
}

fn append_sequence_repeat(
    values: &mut Vec<i128>,
    context: &SequenceParseContext<'_>,
    start: usize,
    repeat_index: usize,
    end: usize,
    row_count: usize,
) -> Result<(), SourceFixedColumnsWriteError> {
    let repeated_values = parse_sequence_values(context, start, repeat_index, row_count)?;
    let count = parse_sequence_count(context, repeat_index + 1, end)?;
    for _ in 0..count {
        for value in &repeated_values {
            if values.len() >= row_count {
                return Err(SourceFixedColumnsWriteError::UnsupportedExpression {
                    source_name: context.source_name.to_owned(),
                    source_span: context.source_span,
                    expression: segment_text(context, start, end),
                });
            }
            values.push(*value);
        }
    }
    Ok(())
}

fn append_sequence_add_progression(
    values: &mut Vec<i128>,
    context: &SequenceParseContext<'_>,
    start: usize,
    progression_index: usize,
    end: usize,
    row_count: usize,
) -> Result<(), SourceFixedColumnsWriteError> {
    let previous =
        *values
            .last()
            .ok_or_else(|| SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                source_name: context.source_name.to_owned(),
                source_span: context.source_span,
                token: context.tokens[progression_index].lexeme.clone(),
            })?;
    let current = parse_sequence_expression(context, start, progression_index)?;
    let step = current.checked_sub(previous).ok_or_else(|| {
        SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            expression: segment_text(context, start, end),
        }
    })?;
    let segment = ProgressionSegment {
        context,
        start,
        end,
        row_count,
    };
    if progression_index + 1 == end {
        return append_add_progression_fill(values, &segment, current, step);
    }
    let last = parse_sequence_expression(context, progression_index + 1, end)?;
    append_add_progression_values(values, &segment, current, last, step)
}

fn append_add_progression_fill(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    step: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    let mut value = current;
    while values.len() < segment.row_count {
        push_progression_value(values, segment, value)?;
        if values.len() < segment.row_count {
            value = value.checked_add(step).ok_or_else(|| {
                SourceFixedColumnsWriteError::IntegerOutOfRange {
                    source_name: segment.context.source_name.to_owned(),
                    source_span: segment.context.source_span,
                    expression: progression_expression(segment),
                }
            })?;
        }
    }
    Ok(())
}

fn append_add_progression_values(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    last: i128,
    step: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if (step > 0 && current > last)
        || (step < 0 && current < last)
        || (step == 0 && current != last)
    {
        return Err(progression_unsupported(segment));
    }

    let mut value = current;
    loop {
        push_progression_value(values, segment, value)?;
        if value == last {
            break;
        }
        value = value.checked_add(step).ok_or_else(|| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: segment.context.source_name.to_owned(),
                source_span: segment.context.source_span,
                expression: progression_expression(segment),
            }
        })?;
        if (step > 0 && value > last) || (step < 0 && value < last) {
            return Err(progression_unsupported(segment));
        }
    }
    Ok(())
}

fn append_sequence_mul_progression(
    values: &mut Vec<i128>,
    context: &SequenceParseContext<'_>,
    start: usize,
    progression_index: usize,
    end: usize,
    row_count: usize,
) -> Result<(), SourceFixedColumnsWriteError> {
    let previous =
        *values
            .last()
            .ok_or_else(|| SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                source_name: context.source_name.to_owned(),
                source_span: context.source_span,
                token: context.tokens[progression_index].lexeme.clone(),
            })?;
    let current = parse_sequence_expression(context, start, progression_index)?;

    let segment = ProgressionSegment {
        context,
        start,
        end,
        row_count,
    };
    if progression_uses_omega(&segment) {
        let factor = field_progression_factor(previous, current, &segment)?;
        if progression_index + 1 == end {
            return append_mul_progression_fill_field(values, &segment, current, factor);
        }
        let last = parse_sequence_expression(context, progression_index + 1, end)?;
        return append_mul_progression_values_field(values, &segment, current, last, factor);
    }

    if previous > 0 && current >= previous && current % previous == 0 {
        let factor = u128::try_from(current / previous).map_err(|_| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: context.source_name.to_owned(),
                source_span: context.source_span,
                expression: segment_text(context, start, end),
            }
        })?;
        if progression_index + 1 == end {
            return append_mul_progression_fill(values, &segment, current, factor);
        }
        let last = parse_sequence_expression(context, progression_index + 1, end)?;
        return append_mul_progression_values(values, &segment, current, last, factor);
    }
    if current > 0 && previous > current && previous % current == 0 {
        let divisor = u128::try_from(previous / current).map_err(|_| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: context.source_name.to_owned(),
                source_span: context.source_span,
                expression: segment_text(context, start, end),
            }
        })?;
        if progression_index + 1 == end {
            return append_div_progression_fill(values, &segment, current, divisor);
        }
        let last = parse_sequence_expression(context, progression_index + 1, end)?;
        return append_div_progression_values(values, &segment, current, last, divisor);
    }
    Err(SourceFixedColumnsWriteError::UnsupportedExpression {
        source_name: context.source_name.to_owned(),
        source_span: context.source_span,
        expression: segment_text(context, start, end),
    })
}

fn append_mul_progression_fill(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    factor: u128,
) -> Result<(), SourceFixedColumnsWriteError> {
    let mut value = current;
    while values.len() < segment.row_count {
        push_progression_value(values, segment, value)?;
        if values.len() < segment.row_count {
            let factor = i128::try_from(factor).map_err(|_| {
                SourceFixedColumnsWriteError::IntegerOutOfRange {
                    source_name: segment.context.source_name.to_owned(),
                    source_span: segment.context.source_span,
                    expression: progression_expression(segment),
                }
            })?;
            value = value.checked_mul(factor).ok_or_else(|| {
                SourceFixedColumnsWriteError::IntegerOutOfRange {
                    source_name: segment.context.source_name.to_owned(),
                    source_span: segment.context.source_span,
                    expression: progression_expression(segment),
                }
            })?;
        }
    }
    Ok(())
}

fn append_mul_progression_fill_field(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    factor: Felt,
) -> Result<(), SourceFixedColumnsWriteError> {
    let mut value = field_progression_value(current, segment)?;
    while values.len() < segment.row_count {
        push_progression_value(values, segment, i128::from(value.to_u64()))?;
        if values.len() < segment.row_count {
            value = value * factor;
        }
    }
    Ok(())
}

fn append_mul_progression_values(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    last: i128,
    factor: u128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if factor <= 1 && current != last {
        return Err(progression_unsupported(segment));
    }
    let mut value = current;
    let factor =
        i128::try_from(factor).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: segment.context.source_name.to_owned(),
            source_span: segment.context.source_span,
            expression: progression_expression(segment),
        })?;
    loop {
        push_progression_value(values, segment, value)?;
        if value == last {
            break;
        }
        value = value.checked_mul(factor).ok_or_else(|| {
            SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: segment.context.source_name.to_owned(),
                source_span: segment.context.source_span,
                expression: progression_expression(segment),
            }
        })?;
        if value > last {
            return Err(progression_unsupported(segment));
        }
    }
    Ok(())
}

fn append_mul_progression_values_field(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    last: i128,
    factor: Felt,
) -> Result<(), SourceFixedColumnsWriteError> {
    let mut value = field_progression_value(current, segment)?;
    let last = field_progression_value(last, segment)?;
    loop {
        push_progression_value(values, segment, i128::from(value.to_u64()))?;
        if value == last {
            break;
        }
        value = value * factor;
    }
    Ok(())
}

fn append_div_progression_fill(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    divisor: u128,
) -> Result<(), SourceFixedColumnsWriteError> {
    let mut value = current;
    let divisor =
        i128::try_from(divisor).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: segment.context.source_name.to_owned(),
            source_span: segment.context.source_span,
            expression: progression_expression(segment),
        })?;
    while values.len() < segment.row_count {
        push_progression_value(values, segment, value)?;
        if values.len() < segment.row_count {
            if value % divisor != 0 {
                return Err(progression_unsupported(segment));
            }
            value /= divisor;
        }
    }
    Ok(())
}

fn append_div_progression_values(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    current: i128,
    last: i128,
    divisor: u128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if divisor <= 1 && current != last {
        return Err(progression_unsupported(segment));
    }
    let mut value = current;
    let divisor =
        i128::try_from(divisor).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: segment.context.source_name.to_owned(),
            source_span: segment.context.source_span,
            expression: progression_expression(segment),
        })?;
    loop {
        push_progression_value(values, segment, value)?;
        if value == last {
            break;
        }
        if value % divisor != 0 {
            return Err(progression_unsupported(segment));
        }
        value /= divisor;
        if value < last {
            return Err(progression_unsupported(segment));
        }
    }
    Ok(())
}

fn push_progression_value(
    values: &mut Vec<i128>,
    segment: &ProgressionSegment<'_, '_>,
    value: i128,
) -> Result<(), SourceFixedColumnsWriteError> {
    if values.len() >= segment.row_count {
        return Err(progression_unsupported(segment));
    }
    values.push(value);
    Ok(())
}

fn field_progression_factor(
    previous: i128,
    current: i128,
    segment: &ProgressionSegment<'_, '_>,
) -> Result<Felt, SourceFixedColumnsWriteError> {
    let previous = field_progression_value(previous, segment)?;
    let current = field_progression_value(current, segment)?;
    Ok(current
        * previous
            .inverse()
            .ok_or_else(|| progression_unsupported(segment))?)
}

fn field_progression_value(
    value: i128,
    segment: &ProgressionSegment<'_, '_>,
) -> Result<Felt, SourceFixedColumnsWriteError> {
    let modulus = i128::from(MODULUS);
    let canonical = value.rem_euclid(modulus);
    let canonical = u64::try_from(canonical).map_err(|_| progression_unsupported(segment))?;
    Ok(Felt::from_u64(canonical))
}

fn progression_unsupported(segment: &ProgressionSegment<'_, '_>) -> SourceFixedColumnsWriteError {
    SourceFixedColumnsWriteError::UnsupportedExpression {
        source_name: segment.context.source_name.to_owned(),
        source_span: segment.context.source_span,
        expression: progression_expression(segment),
    }
}

fn progression_uses_omega(segment: &ProgressionSegment<'_, '_>) -> bool {
    segment
        .context
        .tokens
        .iter()
        .any(|token| token.lexeme == "omega")
}

fn progression_expression(segment: &ProgressionSegment<'_, '_>) -> String {
    segment_text(segment.context, segment.start, segment.end)
}

fn append_sequence_range(
    values: &mut Vec<i128>,
    context: &SequenceParseContext<'_>,
    start: usize,
    range_index: usize,
    end: usize,
    row_count: usize,
) -> Result<(), SourceFixedColumnsWriteError> {
    let (first, first_count) = parse_range_endpoint(context, start, range_index)?;
    let (last, last_count) = parse_range_endpoint(context, range_index + 1, end)?;
    if first_count != last_count {
        return Err(SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            expression: segment_text(context, start, end),
        });
    }
    let ascending = first <= last;
    let mut value = first;
    loop {
        for _ in 0..first_count {
            if values.len() >= row_count {
                return Err(SourceFixedColumnsWriteError::UnsupportedExpression {
                    source_name: context.source_name.to_owned(),
                    source_span: context.source_span,
                    expression: segment_text(context, start, end),
                });
            }
            values.push(value);
        }
        if value == last {
            break;
        }
        value = if ascending {
            value.checked_add(1)
        } else {
            value.checked_sub(1)
        }
        .ok_or_else(|| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            expression: segment_text(context, start, end),
        })?;
    }
    Ok(())
}

fn parse_range_endpoint(
    context: &SequenceParseContext<'_>,
    start: usize,
    end: usize,
) -> Result<(i128, usize), SourceFixedColumnsWriteError> {
    let Some(repeat_index) = top_level_token_index(context, start, end, TokenKind::Colon)? else {
        return Ok((parse_sequence_expression(context, start, end)?, 1));
    };
    let value = parse_sequence_expression(context, start, repeat_index)?;
    let count = parse_sequence_count(context, repeat_index + 1, end)?;
    Ok((value, count))
}

fn parse_sequence_values(
    context: &SequenceParseContext<'_>,
    start: usize,
    end: usize,
    row_count: usize,
) -> Result<Vec<i128>, SourceFixedColumnsWriteError> {
    if start < end
        && context
            .tokens
            .get(start)
            .is_some_and(|token| token.kind == TokenKind::LBracket)
        && context
            .tokens
            .get(end - 1)
            .is_some_and(|token| token.kind == TokenKind::RBracket)
    {
        let source = segment_text(context, start, end);
        return parse_literal_sequence_values(
            context.program,
            context.source_name,
            context.source_span,
            &source,
            row_count,
            context.constant_values,
        );
    }

    Ok(vec![parse_sequence_expression(context, start, end)?])
}

fn top_level_range_index(
    context: &SequenceParseContext<'_>,
    start: usize,
    end: usize,
) -> Result<Option<usize>, SourceFixedColumnsWriteError> {
    top_level_token_index(context, start, end, TokenKind::Range)
}

fn top_level_token_index(
    context: &SequenceParseContext<'_>,
    start: usize,
    end: usize,
    kind: TokenKind,
) -> Result<Option<usize>, SourceFixedColumnsWriteError> {
    let mut found_index = None;
    let mut stack = Vec::new();
    for index in start..end {
        let token = &context.tokens[index];
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                        source_name: context.source_name.to_owned(),
                        source_span: context.source_span,
                        token: token.lexeme.clone(),
                    });
                };
                if expected != token.kind {
                    return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                        source_name: context.source_name.to_owned(),
                        source_span: context.source_span,
                        token: token.lexeme.clone(),
                    });
                }
            }
            token_kind if token_kind == kind && stack.is_empty() => {
                if found_index.replace(index).is_some() {
                    return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                        source_name: context.source_name.to_owned(),
                        source_span: context.source_span,
                        token: token.lexeme.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(found_index)
}

fn segment_text(context: &SequenceParseContext<'_>, start: usize, end: usize) -> String {
    if start >= end || end > context.tokens.len() {
        return String::new();
    }
    let start_byte = context.tokens[start].start;
    let end_byte = context.tokens[end - 1].end;
    context.source[start_byte..end_byte].to_owned()
}

fn parse_sequence_expression(
    context: &SequenceParseContext<'_>,
    start: usize,
    end: usize,
) -> Result<i128, SourceFixedColumnsWriteError> {
    if start >= end {
        return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            token: ",".to_owned(),
        });
    }
    if end == start + 1
        && matches!(
            context.tokens[start].kind,
            TokenKind::Integer | TokenKind::HexInteger
        )
    {
        return parse_literal_token(
            &context.tokens[start],
            context.source_name,
            context.source_span,
        );
    }

    let start_byte = context.tokens[start].start;
    let end_byte = context.tokens[end - 1].end;
    let expression_text = &context.source[start_byte..end_byte];
    let expression_source = SourceFile {
        contents: expression_text.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::from(context.source_name),
        source_name: context.source_name.to_owned(),
    };
    let token_count = lex_source(expression_text)
        .map_err(|source| SourceFixedColumnsWriteError::Lex {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            source,
        })?
        .len();
    let (expression, next_index) =
        parse_expression(&expression_source, 0, token_count).map_err(|source| {
            SourceFixedColumnsWriteError::ExpressionParse {
                source_name: context.source_name.to_owned(),
                source_span: context.source_span,
                source,
            }
        })?;
    if next_index != token_count {
        return Err(SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            expression: expression_text.to_owned(),
        });
    }

    let value =
        evaluate_source_fixed_template_value_expression(&expression, context.constant_values)
            .or_else(|| {
                evaluate_source_static_expression(
                    context.program,
                    &expression,
                    &context.constant_values.scalars,
                )
            });
    match value {
        Some(FixedFileTemplateValue::Integer(value)) => Ok(value),
        _ => Err(SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: context.source_name.to_owned(),
            source_span: context.source_span,
            expression: expression_text.to_owned(),
        }),
    }
}

fn parse_sequence_count(
    context: &SequenceParseContext<'_>,
    start: usize,
    end: usize,
) -> Result<usize, SourceFixedColumnsWriteError> {
    let value = parse_sequence_expression(context, start, end)?;
    usize::try_from(value).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
        source_name: context.source_name.to_owned(),
        source_span: context.source_span,
        expression: segment_text(context, start, end),
    })
}

pub(crate) fn canonical_fixed_value(
    value: i128,
    source_name: &str,
    source_span: SourceSpan,
) -> Result<u64, SourceFixedColumnsWriteError> {
    let modulus = i128::from(MODULUS);
    let canonical = value.rem_euclid(modulus);
    u64::try_from(canonical).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
        source_name: source_name.to_owned(),
        source_span,
        expression: value.to_string(),
    })
}

fn expect_token(
    tokens: &[Token],
    cursor: &mut usize,
    kind: TokenKind,
    source_name: &str,
    source_span: SourceSpan,
) -> Result<(), SourceFixedColumnsWriteError> {
    match tokens.get(*cursor) {
        Some(token) if token.kind == kind => {
            *cursor += 1;
            Ok(())
        }
        Some(token) => Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: source_name.to_owned(),
            source_span,
            token: token.lexeme.clone(),
        }),
        None => Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: source_name.to_owned(),
            source_span,
            token: "<end>".to_owned(),
        }),
    }
}

fn expect_end(
    tokens: &[Token],
    cursor: usize,
    source_name: &str,
    source_span: SourceSpan,
) -> Result<(), SourceFixedColumnsWriteError> {
    if let Some(token) = tokens.get(cursor) {
        return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: source_name.to_owned(),
            source_span,
            token: token.lexeme.clone(),
        });
    }
    Ok(())
}

fn parse_literal_token(
    token: &Token,
    source_name: &str,
    source_span: SourceSpan,
) -> Result<i128, SourceFixedColumnsWriteError> {
    match token.kind {
        TokenKind::Integer => token.lexeme.parse::<i128>(),
        TokenKind::HexInteger => i128::from_str_radix(
            token
                .lexeme
                .strip_prefix("0x")
                .or_else(|| token.lexeme.strip_prefix("0X"))
                .unwrap_or(&token.lexeme),
            16,
        ),
        _ => {
            return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                source_name: source_name.to_owned(),
                source_span,
                token: token.lexeme.clone(),
            });
        }
    }
    .map_err(|_| SourceFixedColumnsWriteError::InvalidLiteral {
        source_name: source_name.to_owned(),
        source_span,
        literal: token.lexeme.clone(),
    })
}

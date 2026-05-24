use super::declarations::{expected_close_error, missing_start, parse_name_reference};
use super::types::{
    BinaryOperator, CallArgument, Expression, ExpressionKind, ParseError, SourceSpan, UnaryOperator,
};
use crate::{lex_source, SourceFile, Token, TokenKind};

pub fn parse_expression(
    source: &SourceFile,
    start_index: usize,
    end_index: usize,
) -> Result<(Expression, usize), ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    parse_expression_tokens(&tokens, start_index, end_index, source)
}

pub fn parse_expression_tokens(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
    source: &SourceFile,
) -> Result<(Expression, usize), ParseError> {
    let mut parser = ExpressionParser {
        tokens,
        cursor: start_index,
        limit: end_index.min(tokens.len()),
        source,
    };
    let expression = parser.parse_expression(0)?;
    Ok((expression, parser.cursor))
}

pub(crate) fn parse_expression_span_best_effort(
    tokens: &[Token],
    span: SourceSpan,
    source: &SourceFile,
) -> Option<Expression> {
    let (start_index, end_index) = token_span_bounds(tokens, &span)?;
    parse_expression_range_best_effort(tokens, start_index, end_index, source)
}

pub(crate) fn parse_expression_list_span_best_effort(
    tokens: &[Token],
    span: SourceSpan,
    source: &SourceFile,
) -> Option<Vec<Expression>> {
    let (start_index, end_index) = token_span_bounds(tokens, &span)?;
    if end_index <= start_index + 1 {
        return Some(Vec::new());
    }
    parse_expression_list_range_best_effort(tokens, start_index + 1, end_index - 1, source)
}

pub(crate) fn parse_call_arguments_span_best_effort(
    tokens: &[Token],
    span: SourceSpan,
    source: &SourceFile,
) -> Option<Vec<CallArgument>> {
    let (start_index, end_index) = token_span_bounds(tokens, &span)?;
    if end_index <= start_index + 2 {
        return Some(Vec::new());
    }

    let mut args = Vec::new();
    let mut segment_start = start_index + 1;
    let mut stack: Vec<TokenKind> = Vec::new();
    let mut cursor = start_index + 1;

    while cursor < end_index - 1 {
        let token = tokens.get(cursor)?;
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let expected = stack.pop()?;
                if token.kind != expected {
                    return None;
                }
            }
            TokenKind::Comma if stack.is_empty() => {
                if segment_start >= cursor {
                    return None;
                }
                args.push(parse_call_argument_segment(
                    tokens,
                    segment_start,
                    cursor,
                    source,
                )?);
                segment_start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }

    if segment_start >= end_index - 1 {
        return None;
    }
    args.push(parse_call_argument_segment(
        tokens,
        segment_start,
        end_index - 1,
        source,
    )?);

    Some(args)
}

pub(crate) fn parse_expression_range_best_effort(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
    source: &SourceFile,
) -> Option<Expression> {
    if start_index >= end_index {
        return None;
    }
    let (expression, next_index) =
        parse_expression_tokens(tokens, start_index, end_index, source).ok()?;
    (next_index == end_index).then_some(expression)
}

pub(crate) fn parse_expression_list_range_best_effort(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
    source: &SourceFile,
) -> Option<Vec<Expression>> {
    if start_index >= end_index {
        return Some(Vec::new());
    }

    let mut values = Vec::new();
    let mut segment_start = start_index;
    let mut stack: Vec<TokenKind> = Vec::new();
    let mut cursor = start_index;

    while cursor < end_index {
        let token = tokens.get(cursor)?;
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let expected = stack.pop()?;
                if token.kind != expected {
                    return None;
                }
            }
            TokenKind::Comma if stack.is_empty() => {
                if segment_start >= cursor {
                    return None;
                }
                values.push(parse_expression_range_best_effort(
                    tokens,
                    segment_start,
                    cursor,
                    source,
                )?);
                segment_start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }

    if segment_start >= end_index {
        return None;
    }
    values.push(parse_expression_range_best_effort(
        tokens,
        segment_start,
        end_index,
        source,
    )?);

    Some(values)
}

fn parse_call_argument_segment(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
    source: &SourceFile,
) -> Option<CallArgument> {
    if start_index >= end_index {
        return None;
    }

    let name = if tokens
        .get(start_index)
        .is_some_and(|token| token.kind == TokenKind::Identifier)
        && tokens
            .get(start_index + 1)
            .is_some_and(|token| token.kind == TokenKind::Colon)
    {
        Some(tokens[start_index].clone())
    } else {
        None
    };

    let value = if let Some(name_token) = &name {
        if start_index + 2 >= end_index
            || tokens
                .get(start_index + 2)
                .is_some_and(|token| token.kind == TokenKind::Comma)
        {
            Expression {
                kind: ExpressionKind::Name(name_token.lexeme.clone()),
                source_name: source.source_name.clone(),
                start: name_token.start,
                end: name_token.end,
            }
        } else {
            parse_expression_range_best_effort(tokens, start_index + 2, end_index, source)?
        }
    } else {
        parse_expression_range_best_effort(tokens, start_index, end_index, source)?
    };

    Some(CallArgument {
        name: name.map(|token| token.lexeme),
        value,
    })
}

fn token_span_bounds(tokens: &[Token], span: &SourceSpan) -> Option<(usize, usize)> {
    let start_index = tokens.iter().position(|token| token.start == span.start)?;
    let end_index = tokens
        .iter()
        .position(|token| token.end == span.end)?
        .checked_add(1)?;
    Some((start_index, end_index))
}

struct ExpressionParser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    limit: usize,
    source: &'a SourceFile,
}

impl ExpressionParser<'_> {
    fn parse_expression(&mut self, min_bp: u8) -> Result<Expression, ParseError> {
        let primary = self.parse_prefix()?;
        let mut lhs = self.parse_postfix_chain(primary)?;

        loop {
            let Some(token) = self.peek() else {
                break;
            };
            let Some((op, left_bp, right_bp)) = binary_operator(token.kind) else {
                break;
            };
            if left_bp < min_bp {
                break;
            }

            self.cursor += 1;
            let rhs = self.parse_expression(right_bp)?;
            let start = lhs.start;
            let end = rhs.end;
            lhs = Expression {
                kind: ExpressionKind::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                source_name: self.source.source_name.clone(),
                start,
                end,
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expression, ParseError> {
        let token = self.peek().ok_or_else(|| ParseError::ExpectedName {
            source_name: self.source.source_name.clone(),
            start: missing_start(self.tokens, self.cursor),
        })?;

        match token.kind {
            TokenKind::Plus => self.parse_unary(UnaryOperator::Plus),
            TokenKind::Minus => self.parse_unary(UnaryOperator::Minus),
            TokenKind::Bang => self.parse_unary(UnaryOperator::Not),
            TokenKind::Increment => self.parse_unary(UnaryOperator::Increment),
            TokenKind::Decrement => self.parse_unary(UnaryOperator::Decrement),
            TokenKind::Apostrophe => self.parse_prior_row_offset(),
            TokenKind::Integer => self.parse_atom(ExpressionKind::Integer(token.lexeme.clone())),
            TokenKind::HexInteger => {
                self.parse_atom(ExpressionKind::HexInteger(token.lexeme.clone()))
            }
            TokenKind::StringLiteral => {
                self.parse_atom(ExpressionKind::StringLiteral(token.lexeme.clone()))
            }
            TokenKind::TemplateLiteral => {
                self.parse_atom(ExpressionKind::TemplateLiteral(token.lexeme.clone()))
            }
            TokenKind::PositionalParam => {
                self.parse_atom(ExpressionKind::PositionalParam(token.lexeme.clone()))
            }
            TokenKind::AtIdentifier => self.parse_atom(ExpressionKind::Name(token.lexeme.clone())),
            TokenKind::Identifier | TokenKind::Air | TokenKind::AirGroup | TokenKind::Proof => {
                self.parse_name()
            }
            TokenKind::String => self.parse_atom(ExpressionKind::Name(token.lexeme.clone())),
            TokenKind::LParen => self.parse_group(),
            TokenKind::LBracket => self.parse_array(),
            _ => Err(ParseError::ExpectedName {
                source_name: self.source.source_name.clone(),
                start: token.start,
            }),
        }
    }

    fn parse_unary(&mut self, op: UnaryOperator) -> Result<Expression, ParseError> {
        const PREFIX_BP: u8 = 13;

        let token = self.current_token()?;
        self.cursor += 1;
        let expr = self.parse_expression(PREFIX_BP)?;
        let end = expr.end;
        Ok(Expression {
            kind: ExpressionKind::Unary {
                op,
                expr: Box::new(expr),
            },
            source_name: self.source.source_name.clone(),
            start: token.start,
            end,
        })
    }

    fn parse_atom(&mut self, kind: ExpressionKind) -> Result<Expression, ParseError> {
        let token = self.current_token()?;
        self.cursor += 1;
        Ok(Expression {
            kind,
            source_name: self.source.source_name.clone(),
            start: token.start,
            end: token.end,
        })
    }

    fn parse_name(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_token()?.start;
        let (name, next_index) = parse_name_reference(self.tokens, self.cursor, self.source)?;
        if next_index > self.limit {
            return Err(ParseError::ExpectedName {
                source_name: self.source.source_name.clone(),
                start: missing_start(self.tokens, self.limit),
            });
        }
        self.cursor = next_index;
        let end = self
            .tokens
            .get(next_index.saturating_sub(1))
            .map_or(start, |token| token.end);

        Ok(Expression {
            kind: ExpressionKind::Name(name),
            source_name: self.source.source_name.clone(),
            start,
            end,
        })
    }

    fn parse_group(&mut self) -> Result<Expression, ParseError> {
        let open_index = self.cursor;
        let open = self.current_token()?;
        let close_index = self.find_delimited_close(open_index, TokenKind::RParen)?;

        self.cursor = open_index + 1;
        if self.cursor == close_index {
            return Err(ParseError::ExpectedName {
                source_name: self.source.source_name.clone(),
                start: self.tokens[close_index].start,
            });
        }
        let inner = self.parse_expression(0)?;
        if self.cursor != close_index {
            return Err(ParseError::ExpectedCloseParen {
                source_name: self.source.source_name.clone(),
                start: missing_start(self.tokens, self.cursor),
            });
        }
        self.cursor = close_index + 1;

        Ok(Expression {
            kind: ExpressionKind::Group(Box::new(inner)),
            source_name: self.source.source_name.clone(),
            start: open.start,
            end: self.tokens[close_index].end,
        })
    }

    fn parse_array(&mut self) -> Result<Expression, ParseError> {
        let open_index = self.cursor;
        let open = self.current_token()?;
        let close_index = self.find_delimited_close(open_index, TokenKind::RBracket)?;

        self.cursor = open_index + 1;
        let mut values = Vec::new();
        if self.cursor != close_index {
            loop {
                values.push(self.parse_expression(0)?);
                if self.cursor == close_index {
                    break;
                }
                let token = self
                    .peek()
                    .ok_or_else(|| ParseError::ExpectedCloseBracket {
                        source_name: self.source.source_name.clone(),
                        start: missing_start(self.tokens, self.cursor),
                    })?;
                if token.kind != TokenKind::Comma {
                    return Err(ParseError::ExpectedCloseBracket {
                        source_name: self.source.source_name.clone(),
                        start: token.start,
                    });
                }
                self.cursor += 1;
                if self.cursor == close_index {
                    return Err(ParseError::ExpectedName {
                        source_name: self.source.source_name.clone(),
                        start: self.tokens[close_index].start,
                    });
                }
            }
        }
        self.cursor = close_index + 1;

        Ok(Expression {
            kind: ExpressionKind::Array(values),
            source_name: self.source.source_name.clone(),
            start: open.start,
            end: self.tokens[close_index].end,
        })
    }

    fn parse_postfix_chain(&mut self, mut expr: Expression) -> Result<Expression, ParseError> {
        loop {
            let Some(token) = self.peek() else {
                break;
            };
            match token.kind {
                TokenKind::LParen => {
                    expr = self.parse_call(expr)?;
                }
                TokenKind::LBracket => {
                    expr = self.parse_index(expr)?;
                }
                TokenKind::Apostrophe
                    if row_offset_offset_expression(&expr)
                        && self.next_token_starts_prior_target() =>
                {
                    expr = self.parse_explicit_prior_row_offset(expr)?;
                }
                TokenKind::Apostrophe => {
                    expr = self.parse_postfix_row_offset(expr)?;
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_prior_row_offset(&mut self) -> Result<Expression, ParseError> {
        let marker = self.current_token()?;
        self.cursor += 1;
        let primary = self.parse_prefix()?;
        let target = self.parse_postfix_chain(primary)?;
        let offset = self.default_row_offset(&marker);
        let end = target.end;

        Ok(self.row_offset_expression(target, offset, true, marker.start, end))
    }

    fn parse_explicit_prior_row_offset(
        &mut self,
        offset: Expression,
    ) -> Result<Expression, ParseError> {
        self.cursor += 1;
        let primary = self.parse_prefix()?;
        let target = self.parse_postfix_chain(primary)?;
        let start = offset.start;
        let end = target.end;

        Ok(self.row_offset_expression(target, offset, true, start, end))
    }

    fn parse_postfix_row_offset(&mut self, target: Expression) -> Result<Expression, ParseError> {
        let marker = self.current_token()?;
        self.cursor += 1;
        let offset = self.parse_row_offset_suffix(&marker)?;
        let start = target.start;
        let end = offset.end.max(marker.end);

        Ok(self.row_offset_expression(target, offset, false, start, end))
    }

    fn parse_row_offset_suffix(&mut self, marker: &Token) -> Result<Expression, ParseError> {
        let Some(token) = self.peek() else {
            return Ok(self.default_row_offset(marker));
        };
        match token.kind {
            TokenKind::Integer
            | TokenKind::HexInteger
            | TokenKind::PositionalParam
            | TokenKind::LParen => self.parse_prefix(),
            _ => Ok(self.default_row_offset(marker)),
        }
    }

    fn default_row_offset(&self, marker: &Token) -> Expression {
        Expression {
            kind: ExpressionKind::Integer("1".to_owned()),
            source_name: self.source.source_name.clone(),
            start: marker.start,
            end: marker.end,
        }
    }

    fn row_offset_expression(
        &self,
        target: Expression,
        offset: Expression,
        prior: bool,
        start: usize,
        end: usize,
    ) -> Expression {
        Expression {
            kind: ExpressionKind::RowOffset {
                target: Box::new(target),
                offset: Box::new(offset),
                prior,
            },
            source_name: self.source.source_name.clone(),
            start,
            end,
        }
    }

    fn next_token_starts_prior_target(&self) -> bool {
        self.tokens
            .get(self.cursor + 1)
            .is_some_and(|token| row_offset_target_start(token.kind))
    }

    fn parse_call(&mut self, callee: Expression) -> Result<Expression, ParseError> {
        let open_index = self.cursor;
        let close_index = self.find_delimited_close(open_index, TokenKind::RParen)?;
        self.cursor = open_index + 1;
        let args = self.parse_call_arguments(close_index)?;
        self.cursor = close_index + 1;
        let start = callee.start;

        Ok(Expression {
            kind: ExpressionKind::Call {
                callee: Box::new(callee),
                args,
            },
            source_name: self.source.source_name.clone(),
            start,
            end: self.tokens[close_index].end,
        })
    }

    fn parse_call_arguments(
        &mut self,
        close_index: usize,
    ) -> Result<Vec<CallArgument>, ParseError> {
        let mut args = Vec::new();
        if self.cursor == close_index {
            return Ok(args);
        }

        loop {
            let name = if self
                .tokens
                .get(self.cursor)
                .is_some_and(|token| token.kind == TokenKind::Identifier)
                && self
                    .tokens
                    .get(self.cursor + 1)
                    .is_some_and(|token| token.kind == TokenKind::Colon)
            {
                let name_token = self.tokens[self.cursor].clone();
                self.cursor += 2;
                Some(name_token)
            } else {
                None
            };

            let value = if let Some(name_token) = &name {
                if self.cursor == close_index
                    || self
                        .tokens
                        .get(self.cursor)
                        .is_some_and(|token| token.kind == TokenKind::Comma)
                {
                    Expression {
                        kind: ExpressionKind::Name(name_token.lexeme.clone()),
                        source_name: self.source.source_name.clone(),
                        start: name_token.start,
                        end: name_token.end,
                    }
                } else {
                    self.parse_expression(0)?
                }
            } else {
                self.parse_expression(0)?
            };

            args.push(CallArgument {
                name: name.map(|token| token.lexeme),
                value,
            });

            if self.cursor == close_index {
                break;
            }

            let token = self.peek().ok_or_else(|| ParseError::ExpectedCloseParen {
                source_name: self.source.source_name.clone(),
                start: missing_start(self.tokens, self.cursor),
            })?;
            if token.kind != TokenKind::Comma {
                return Err(ParseError::ExpectedCloseParen {
                    source_name: self.source.source_name.clone(),
                    start: token.start,
                });
            }
            self.cursor += 1;
            if self.cursor == close_index {
                return Err(ParseError::ExpectedName {
                    source_name: self.source.source_name.clone(),
                    start: self.tokens[close_index].start,
                });
            }
        }

        Ok(args)
    }

    fn parse_index(&mut self, target: Expression) -> Result<Expression, ParseError> {
        let open_index = self.cursor;
        let close_index = self.find_delimited_close(open_index, TokenKind::RBracket)?;
        self.cursor = open_index + 1;
        if self.cursor == close_index {
            return Err(ParseError::ExpectedName {
                source_name: self.source.source_name.clone(),
                start: self.tokens[close_index].start,
            });
        }
        let index = self.parse_expression(0)?;
        if self.cursor != close_index {
            return Err(ParseError::ExpectedCloseBracket {
                source_name: self.source.source_name.clone(),
                start: missing_start(self.tokens, self.cursor),
            });
        }
        self.cursor = close_index + 1;
        let start = target.start;

        Ok(Expression {
            kind: ExpressionKind::Index {
                target: Box::new(target),
                index: Box::new(index),
            },
            source_name: self.source.source_name.clone(),
            start,
            end: self.tokens[close_index].end,
        })
    }

    fn find_delimited_close(
        &self,
        open_index: usize,
        expected_close: TokenKind,
    ) -> Result<usize, ParseError> {
        let open = self.tokens.get(open_index).ok_or_else(|| {
            expected_close_error(
                expected_close,
                self.source,
                missing_start(self.tokens, open_index),
            )
        })?;
        let mut stack = vec![expected_close];

        for (index, token) in self
            .tokens
            .iter()
            .enumerate()
            .take(self.limit)
            .skip(open_index + 1)
        {
            match token.kind {
                TokenKind::LParen => stack.push(TokenKind::RParen),
                TokenKind::LBracket => stack.push(TokenKind::RBracket),
                TokenKind::LBrace => stack.push(TokenKind::RBrace),
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    let Some(expected) = stack.pop() else {
                        return Err(expected_close_error(token.kind, self.source, token.start));
                    };
                    if token.kind != expected {
                        return Err(expected_close_error(expected, self.source, token.start));
                    }
                    if stack.is_empty() {
                        return Ok(index);
                    }
                }
                _ => {}
            }
        }

        Err(expected_close_error(
            expected_close,
            self.source,
            open.start,
        ))
    }

    fn peek(&self) -> Option<&Token> {
        if self.cursor < self.limit {
            self.tokens.get(self.cursor)
        } else {
            None
        }
    }

    fn current_token(&self) -> Result<Token, ParseError> {
        self.peek()
            .cloned()
            .ok_or_else(|| ParseError::ExpectedName {
                source_name: self.source.source_name.clone(),
                start: missing_start(self.tokens, self.cursor),
            })
    }
}

fn binary_operator(kind: TokenKind) -> Option<(BinaryOperator, u8, u8)> {
    let (op, precedence, right_assoc) = match kind {
        TokenKind::Range => (BinaryOperator::Range, 1, false),
        TokenKind::RangeFill => (BinaryOperator::RangeFill, 1, false),
        TokenKind::RangeMulFill => (BinaryOperator::RangeMulFill, 1, false),
        TokenKind::Assign => (BinaryOperator::Assign, 2, true),
        TokenKind::PlusEqual => (BinaryOperator::PlusAssign, 2, true),
        TokenKind::MinusEqual => (BinaryOperator::MinusAssign, 2, true),
        TokenKind::StarEqual => (BinaryOperator::StarAssign, 2, true),
        TokenKind::TripleEqual => (BinaryOperator::TripleEqual, 2, true),
        TokenKind::ConstrainedAssign => (BinaryOperator::ConstrainedAssign, 2, true),
        TokenKind::PipePipe => (BinaryOperator::LogicalOr, 3, false),
        TokenKind::AmpAmp => (BinaryOperator::LogicalAnd, 4, false),
        TokenKind::Pipe => (BinaryOperator::BitOr, 5, false),
        TokenKind::Caret => (BinaryOperator::BitXor, 6, false),
        TokenKind::Amp => (BinaryOperator::BitAnd, 7, false),
        TokenKind::EqualEqual => (BinaryOperator::EqualEqual, 8, false),
        TokenKind::NotEqual => (BinaryOperator::NotEqual, 8, false),
        TokenKind::Less => (BinaryOperator::Less, 9, false),
        TokenKind::LessEqual => (BinaryOperator::LessEqual, 9, false),
        TokenKind::Greater => (BinaryOperator::Greater, 9, false),
        TokenKind::GreaterEqual => (BinaryOperator::GreaterEqual, 9, false),
        TokenKind::ShiftLeft => (BinaryOperator::ShiftLeft, 10, false),
        TokenKind::ShiftRight => (BinaryOperator::ShiftRight, 10, false),
        TokenKind::Plus => (BinaryOperator::Add, 11, false),
        TokenKind::Minus => (BinaryOperator::Subtract, 11, false),
        TokenKind::Star => (BinaryOperator::Multiply, 12, false),
        TokenKind::Slash => (BinaryOperator::Divide, 12, false),
        TokenKind::Percent => (BinaryOperator::Modulo, 12, false),
        TokenKind::Backslash => (BinaryOperator::Backslash, 12, false),
        TokenKind::Pow => (BinaryOperator::Power, 14, true),
        _ => return None,
    };

    if right_assoc {
        Some((op, precedence, precedence))
    } else {
        Some((op, precedence, precedence + 1))
    }
}

fn row_offset_offset_expression(expression: &Expression) -> bool {
    matches!(
        &expression.kind,
        ExpressionKind::Integer(_)
            | ExpressionKind::HexInteger(_)
            | ExpressionKind::PositionalParam(_)
            | ExpressionKind::Group(_)
    )
}

fn row_offset_target_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier | TokenKind::Air | TokenKind::AirGroup | TokenKind::Proof
    )
}

#[cfg(test)]
mod tests {
    use super::super::types::{BinaryOperator, ExpressionKind, UnaryOperator};
    use super::*;
    use crate::lex_source;
    use std::path::PathBuf;

    fn source(contents: &str) -> SourceFile {
        SourceFile {
            contents: contents.to_owned(),
            file_dir: PathBuf::from("/case"),
            full_path: PathBuf::from("/case/main.pil"),
            source_name: "main.pil".to_owned(),
        }
    }

    fn parse(contents: &str) -> Expression {
        let source = source(contents);
        let tokens = lex_source(&source.contents).expect("lexing should work");
        let (expression, next_index) =
            parse_expression(&source, 0, tokens.len()).expect("expression should parse");
        assert_eq!(next_index, tokens.len());
        expression
    }

    #[test]
    fn parses_multiplication_before_addition() {
        let expression = parse("a + 2 * b");

        let ExpressionKind::Binary { op, left, right } = expression.kind else {
            panic!("root should be binary");
        };
        assert_eq!(op, BinaryOperator::Add);
        assert!(matches!(left.kind, ExpressionKind::Name(ref name) if name == "a"));
        assert!(matches!(
            right.kind,
            ExpressionKind::Binary {
                op: BinaryOperator::Multiply,
                ..
            }
        ));
    }

    #[test]
    fn parses_calls_and_indexes() {
        let expression = parse("vec[sum(2, 3)]");

        let ExpressionKind::Index { target, index } = expression.kind else {
            panic!("root should be index");
        };
        assert!(matches!(target.kind, ExpressionKind::Name(ref name) if name == "vec"));
        let ExpressionKind::Call { callee, args } = index.kind else {
            panic!("index should be call");
        };
        assert!(matches!(callee.kind, ExpressionKind::Name(ref name) if name == "sum"));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn parses_named_call_arguments() {
        let expression = parse("sum(a: a, b: 2 + 3)");

        let ExpressionKind::Call { args, .. } = expression.kind else {
            panic!("root should be call");
        };
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name.as_deref(), Some("a"));
        assert_eq!(args[1].name.as_deref(), Some("b"));
        assert!(matches!(
            args[1].value.kind,
            ExpressionKind::Binary {
                op: BinaryOperator::Add,
                ..
            }
        ));
    }

    #[test]
    fn parses_array_literal_call_arguments() {
        let expression = parse("sum(values: [a, b + 1], empty: [])");

        let ExpressionKind::Call { args, .. } = expression.kind else {
            panic!("root should be call");
        };
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name.as_deref(), Some("values"));
        let ExpressionKind::Array(values) = &args[0].value.kind else {
            panic!("values argument should be an array");
        };
        assert_eq!(values.len(), 2);
        assert!(matches!(&values[0].kind, ExpressionKind::Name(name) if name == "a"));
        assert!(matches!(
            &values[1].kind,
            ExpressionKind::Binary {
                op: BinaryOperator::Add,
                ..
            }
        ));
        assert_eq!(args[1].name.as_deref(), Some("empty"));
        assert!(matches!(&args[1].value.kind, ExpressionKind::Array(values) if values.is_empty()));
    }

    #[test]
    fn parses_prefix_and_grouped_binary_expression() {
        let expression = parse("!(value == -1)");

        let ExpressionKind::Unary {
            op: UnaryOperator::Not,
            expr,
        } = expression.kind
        else {
            panic!("root should be unary");
        };
        let ExpressionKind::Group(inner) = expr.kind else {
            panic!("unary input should be grouped");
        };
        assert!(matches!(
            inner.kind,
            ExpressionKind::Binary {
                op: BinaryOperator::EqualEqual,
                ..
            }
        ));
    }

    #[test]
    fn parses_postfix_row_offsets() {
        let expression = parse("vec[sum(2, 3)]'(row + 1)");

        let ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } = expression.kind
        else {
            panic!("root should be row offset");
        };
        assert!(!prior);
        assert!(matches!(target.kind, ExpressionKind::Index { .. }));
        assert!(matches!(
            offset.kind,
            ExpressionKind::Group(inner) if matches!(
                inner.kind,
                ExpressionKind::Binary {
                    op: BinaryOperator::Add,
                    ..
                }
            )
        ));
    }

    #[test]
    fn parses_postfix_row_offset_literals() {
        let expression = parse("lane'512 + flag'");

        let ExpressionKind::Binary {
            op: BinaryOperator::Add,
            left,
            right,
        } = expression.kind
        else {
            panic!("root should be add");
        };

        let ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } = left.kind
        else {
            panic!("left should be row offset");
        };
        assert!(!prior);
        assert!(matches!(target.kind, ExpressionKind::Name(ref name) if name == "lane"));
        assert!(matches!(offset.kind, ExpressionKind::Integer(ref value) if value == "512"));

        let ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } = right.kind
        else {
            panic!("right should be row offset");
        };
        assert!(!prior);
        assert!(matches!(target.kind, ExpressionKind::Name(ref name) if name == "flag"));
        assert!(matches!(offset.kind, ExpressionKind::Integer(ref value) if value == "1"));
    }

    #[test]
    fn parses_prior_row_offsets() {
        let expression = parse("2'byte_in + 'byte_out");

        let ExpressionKind::Binary {
            op: BinaryOperator::Add,
            left,
            right,
        } = expression.kind
        else {
            panic!("root should be add");
        };

        let ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } = left.kind
        else {
            panic!("left should be row offset");
        };
        assert!(prior);
        assert!(matches!(target.kind, ExpressionKind::Name(ref name) if name == "byte_in"));
        assert!(matches!(offset.kind, ExpressionKind::Integer(ref value) if value == "2"));

        let ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } = right.kind
        else {
            panic!("right should be row offset");
        };
        assert!(prior);
        assert!(matches!(target.kind, ExpressionKind::Name(ref name) if name == "byte_out"));
        assert!(matches!(offset.kind, ExpressionKind::Integer(ref value) if value == "1"));
    }
}

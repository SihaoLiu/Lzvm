use super::declarations::{expected_close_error, missing_start, parse_name_reference};
use super::types::{
    BinaryOperator, CallArgument, Expression, ExpressionKind, ParseError, UnaryOperator,
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

pub(crate) fn parse_expression_tokens(
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
            TokenKind::LParen => self.parse_group(),
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
                _ => break,
            }
        }
        Ok(expr)
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
}

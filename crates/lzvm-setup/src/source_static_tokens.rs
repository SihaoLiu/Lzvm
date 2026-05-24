use lzvm_pil::{Token, TokenKind};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticTemplateFlow {
    Fallthrough,
    Break,
    Continue,
}

pub(crate) struct StaticTemplateStatementResult {
    pub(crate) next: usize,
    pub(crate) flow: StaticTemplateFlow,
}

impl StaticTemplateStatementResult {
    pub(crate) fn fallthrough(next: usize) -> Self {
        Self {
            next,
            flow: StaticTemplateFlow::Fallthrough,
        }
    }

    pub(crate) fn control(next: usize, flow: StaticTemplateFlow) -> Self {
        Self { next, flow }
    }
}

pub(crate) fn control_body_range(
    tokens: &[Token],
    index: usize,
    end: usize,
) -> Option<(usize, usize, usize)> {
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

pub(crate) fn skip_static_do_while_statement(
    tokens: &[Token],
    index: usize,
    end: usize,
) -> Option<usize> {
    let (_, _, after_body) = control_body_range(tokens, index + 1, end)?;
    if tokens.get(after_body).map(|token| token.kind) != Some(TokenKind::While) {
        return None;
    }
    let open = after_body + 1;
    if tokens.get(open).map(|token| token.kind) != Some(TokenKind::LParen) {
        return None;
    }
    let close = matching_closing_token(tokens, open, end)?;
    let semicolon = close + 1;
    (tokens.get(semicolon).map(|token| token.kind) == Some(TokenKind::Semicolon))
        .then_some(semicolon + 1)
}

pub(crate) fn next_token_kind(
    tokens: &[Token],
    start: usize,
    end: usize,
    kind: TokenKind,
) -> Option<usize> {
    tokens
        .iter()
        .enumerate()
        .take(end)
        .skip(start)
        .find_map(|(index, token)| (token.kind == kind).then_some(index))
}

pub(crate) fn matching_closing_token(tokens: &[Token], open: usize, end: usize) -> Option<usize> {
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

pub(crate) fn update_static_delimiter_stack(
    kind: TokenKind,
    stack: &mut Vec<TokenKind>,
) -> Option<()> {
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

pub(crate) fn static_switch_label_colon(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Option<usize> {
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

pub(crate) fn next_static_semicolon_limited(
    tokens: &[Token],
    index: usize,
    end: usize,
) -> Option<usize> {
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

pub(crate) fn source_token_index_at_start(tokens: &[Token], start: usize) -> Option<usize> {
    tokens
        .iter()
        .position(|token| token.start == start && token.kind != TokenKind::EndOfInput)
}

pub(crate) fn source_token_index_after_end(tokens: &[Token], end: usize) -> Option<usize> {
    tokens
        .iter()
        .position(|token| token.end == end)
        .and_then(|index| index.checked_add(1))
}

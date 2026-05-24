use lzvm_pil::{Token, TokenKind};

pub(super) fn skip_known_top_level_metadata_directive(
    tokens: &[Token],
    index: usize,
) -> Option<usize> {
    let name = tokens.get(index)?.lexeme.as_str();
    let open = tokens.get(index + 1)?;
    if open.kind != TokenKind::LParen {
        return None;
    }

    let close_index = skip_parenthesized_arguments(tokens, index + 1)?;
    let semicolon = tokens.get(close_index + 1)?;
    if semicolon.kind != TokenKind::Semicolon {
        return None;
    }

    if name == "println" || (name == "enable_range_stats" && close_index == index + 2) {
        Some(close_index + 2)
    } else {
        None
    }
}

fn skip_parenthesized_arguments(tokens: &[Token], open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, token) in tokens.get(open_index..)?.iter().enumerate() {
        match token.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open_index + offset);
                }
            }
            _ => {}
        }
    }
    None
}

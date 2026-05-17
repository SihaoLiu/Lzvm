use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Pragma,
    Col,
    Witness,
    Fixed,
    Container,
    Declare,
    Use,
    Alias,
    Include,
    Require,
    In,
    Is,
    PublicTable,
    Public,
    Constant,
    Const,
    ProofValue,
    AirGroupValue,
    AirValue,
    AirGroup,
    AirTemplate,
    Air,
    Proof,
    Commit,
    Package,
    Virtual,
    Int,
    Fe,
    Expr,
    String,
    Challenge,
    For,
    While,
    Do,
    Break,
    Continue,
    If,
    ElseIf,
    Else,
    Switch,
    Case,
    Default,
    When,
    Aggregate,
    Stage,
    On,
    Private,
    Final,
    Function,
    Return,
    RangeFill,
    RangeMulFill,
    Ellipsis,
    Range,
    Integer,
    HexInteger,
    StringLiteral,
    TemplateLiteral,
    Identifier,
    AtIdentifier,
    PositionalParam,
    Pow,
    Increment,
    Decrement,
    Apostrophe,
    PlusEqual,
    MinusEqual,
    StarEqual,
    Plus,
    Minus,
    Star,
    Percent,
    Slash,
    Backslash,
    AmpAmp,
    PipePipe,
    Amp,
    Pipe,
    Caret,
    TripleEqual,
    ConstrainedAssign,
    ShiftLeft,
    ShiftRight,
    LessEqual,
    GreaterEqual,
    Less,
    Greater,
    NotEqual,
    EqualEqual,
    Assign,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    ColonColon,
    Colon,
    Bang,
    Question,
    Semicolon,
    Comma,
    Dot,
    EndOfInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub index: usize,
    pub message: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lex error at {}: {}", self.index, self.message)
    }
}

impl std::error::Error for LexError {}

pub fn lex_source(input: &str) -> Result<Vec<Token>, LexError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0_usize;

    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if starts_with(input, index, "/*") {
            index = skip_block_comment(input, index)?;
            continue;
        }
        if starts_with(input, index, "//") {
            index = skip_line_comment(input, index);
            continue;
        }
        if starts_with_pragma(input, index) {
            let end = line_end(input, index);
            let value_start = index + "#pragma".len();
            let lexeme = input[value_start..end].trim_start().to_owned();
            tokens.push(Token {
                kind: TokenKind::Pragma,
                lexeme,
                start: index,
                end,
            });
            index = end;
            continue;
        }

        if bytes[index] == b'"' {
            let (token, next) = scan_quoted(input, index, b'"', TokenKind::StringLiteral)?;
            tokens.push(token);
            index = next;
            continue;
        }
        if bytes[index] == b'`' {
            let (token, next) = scan_quoted(input, index, b'`', TokenKind::TemplateLiteral)?;
            tokens.push(token);
            index = next;
            continue;
        }
        if bytes[index] == b'@' {
            let (token, next) =
                scan_prefixed_identifier(input, index, b'@', TokenKind::AtIdentifier)?;
            tokens.push(token);
            index = next;
            continue;
        }
        if bytes[index] == b'$' {
            let (token, next) = scan_positional_param(input, index)?;
            tokens.push(token);
            index = next;
            continue;
        }
        if bytes[index].is_ascii_digit() {
            let (token, next) = scan_number(input, index)?;
            tokens.push(token);
            index = next;
            continue;
        }
        if is_identifier_start(bytes[index]) {
            let (token, next) = scan_identifier(input, index);
            tokens.push(token);
            index = next;
            continue;
        }
        if let Some((kind, width)) = operator_at(input, index) {
            tokens.push(Token {
                kind,
                lexeme: input[index..index + width].to_owned(),
                start: index,
                end: index + width,
            });
            index += width;
            continue;
        }

        return Err(LexError {
            index,
            message: format!("unexpected character {:?}", input[index..].chars().next()),
        });
    }

    Ok(tokens)
}

fn starts_with(input: &str, index: usize, pattern: &str) -> bool {
    input.as_bytes()[index..].starts_with(pattern.as_bytes())
}

fn starts_with_pragma(input: &str, index: usize) -> bool {
    let pattern = "#pragma";
    if !starts_with(input, index, pattern) {
        return false;
    }
    let after = index + pattern.len();
    input
        .as_bytes()
        .get(after)
        .is_some_and(u8::is_ascii_whitespace)
}

fn skip_line_comment(input: &str, index: usize) -> usize {
    line_end(input, index)
}

fn skip_block_comment(input: &str, index: usize) -> Result<usize, LexError> {
    let mut cursor = index + 2;
    while cursor + 1 < input.len() {
        if starts_with(input, cursor, "*/") {
            return Ok(cursor + 2);
        }
        cursor += 1;
    }
    Err(LexError {
        index,
        message: "unterminated block comment".to_owned(),
    })
}

fn line_end(input: &str, index: usize) -> usize {
    input.as_bytes()[index..]
        .iter()
        .position(|byte| matches!(byte, b'\n' | b'\r'))
        .map_or(input.len(), |offset| index + offset)
}

fn scan_quoted(
    input: &str,
    index: usize,
    quote: u8,
    kind: TokenKind,
) -> Result<(Token, usize), LexError> {
    let mut cursor = index + 1;
    let mut escaped = false;
    while cursor < input.len() {
        let byte = input.as_bytes()[cursor];
        if escaped {
            escaped = false;
            cursor += 1;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            cursor += 1;
            continue;
        }
        if byte == quote {
            let end = cursor + 1;
            return Ok((
                Token {
                    kind,
                    lexeme: input[index + 1..cursor].to_owned(),
                    start: index,
                    end,
                },
                end,
            ));
        }
        cursor += 1;
    }
    Err(LexError {
        index,
        message: "unterminated quoted literal".to_owned(),
    })
}

fn scan_prefixed_identifier(
    input: &str,
    index: usize,
    prefix: u8,
    kind: TokenKind,
) -> Result<(Token, usize), LexError> {
    debug_assert_eq!(input.as_bytes()[index], prefix);
    let start = index + 1;
    let Some(first) = input.as_bytes().get(start).copied() else {
        return Err(LexError {
            index,
            message: "missing prefixed identifier".to_owned(),
        });
    };
    if !is_identifier_start(first) {
        return Err(LexError {
            index,
            message: "missing prefixed identifier".to_owned(),
        });
    }
    let end = scan_identifier_end(input, start);
    Ok((
        Token {
            kind,
            lexeme: input[start..end].to_owned(),
            start: index,
            end,
        },
        end,
    ))
}

fn scan_positional_param(input: &str, index: usize) -> Result<(Token, usize), LexError> {
    let start = index + 1;
    let Some(first) = input.as_bytes().get(start).copied() else {
        return Err(LexError {
            index,
            message: "missing positional parameter index".to_owned(),
        });
    };
    if !first.is_ascii_digit() {
        return Err(LexError {
            index,
            message: "missing positional parameter index".to_owned(),
        });
    }
    let mut end = start + 1;
    while input.as_bytes().get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    Ok((
        Token {
            kind: TokenKind::PositionalParam,
            lexeme: input[start..end].to_owned(),
            start: index,
            end,
        },
        end,
    ))
}

fn scan_number(input: &str, index: usize) -> Result<(Token, usize), LexError> {
    let bytes = input.as_bytes();
    let mut end = index;
    let mut kind = TokenKind::Integer;
    if (starts_with(input, index, "0x") || starts_with(input, index, "0X"))
        && bytes
            .get(index + 2)
            .is_some_and(|byte| byte.is_ascii_hexdigit())
    {
        let first_digit = index + 2;
        kind = TokenKind::HexInteger;
        end = first_digit + 1;
        while bytes
            .get(end)
            .is_some_and(|byte| byte.is_ascii_hexdigit() || *byte == b'_')
        {
            end += 1;
        }
    } else {
        while bytes
            .get(end)
            .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'_')
        {
            end += 1;
        }
    }
    Ok((
        Token {
            kind,
            lexeme: input[index..end].replace('_', ""),
            start: index,
            end,
        },
        end,
    ))
}

fn scan_identifier(input: &str, index: usize) -> (Token, usize) {
    let end = scan_identifier_end(input, index);
    let lexeme = &input[index..end];
    let kind = keyword_kind(lexeme).unwrap_or(TokenKind::Identifier);
    (
        Token {
            kind,
            lexeme: lexeme.to_owned(),
            start: index,
            end,
        },
        end,
    )
}

fn scan_identifier_end(input: &str, index: usize) -> usize {
    let mut end = index + 1;
    while input
        .as_bytes()
        .get(end)
        .is_some_and(|byte| is_identifier_continue(*byte))
    {
        end += 1;
    }
    end
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn keyword_kind(value: &str) -> Option<TokenKind> {
    Some(match value {
        "col" => TokenKind::Col,
        "witness" => TokenKind::Witness,
        "fixed" => TokenKind::Fixed,
        "container" => TokenKind::Container,
        "declare" => TokenKind::Declare,
        "use" => TokenKind::Use,
        "alias" => TokenKind::Alias,
        "include" => TokenKind::Include,
        "require" => TokenKind::Require,
        "in" => TokenKind::In,
        "is" => TokenKind::Is,
        "publictable" => TokenKind::PublicTable,
        "public" => TokenKind::Public,
        "constant" => TokenKind::Constant,
        "const" => TokenKind::Const,
        "proofval" => TokenKind::ProofValue,
        "airgroupval" => TokenKind::AirGroupValue,
        "airval" => TokenKind::AirValue,
        "airgroup" => TokenKind::AirGroup,
        "airtemplate" => TokenKind::AirTemplate,
        "air" => TokenKind::Air,
        "proof" => TokenKind::Proof,
        "commit" => TokenKind::Commit,
        "package" => TokenKind::Package,
        "virtual" => TokenKind::Virtual,
        "int" => TokenKind::Int,
        "fe" => TokenKind::Fe,
        "expr" => TokenKind::Expr,
        "string" => TokenKind::String,
        "challenge" => TokenKind::Challenge,
        "for" => TokenKind::For,
        "while" => TokenKind::While,
        "do" => TokenKind::Do,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "if" => TokenKind::If,
        "elseif" => TokenKind::ElseIf,
        "else" => TokenKind::Else,
        "switch" => TokenKind::Switch,
        "case" => TokenKind::Case,
        "default" => TokenKind::Default,
        "when" => TokenKind::When,
        "aggregate" => TokenKind::Aggregate,
        "stage" => TokenKind::Stage,
        "on" => TokenKind::On,
        "private" => TokenKind::Private,
        "final" => TokenKind::Final,
        "function" => TokenKind::Function,
        "return" => TokenKind::Return,
        _ => return None,
    })
}

fn operator_at(input: &str, index: usize) -> Option<(TokenKind, usize)> {
    const OPERATORS: &[(&str, TokenKind)] = &[
        ("..+..", TokenKind::RangeFill),
        ("..*..", TokenKind::RangeMulFill),
        ("...", TokenKind::Ellipsis),
        ("===", TokenKind::TripleEqual),
        ("<==", TokenKind::ConstrainedAssign),
        ("**", TokenKind::Pow),
        ("++", TokenKind::Increment),
        ("--", TokenKind::Decrement),
        ("+=", TokenKind::PlusEqual),
        ("-=", TokenKind::MinusEqual),
        ("*=", TokenKind::StarEqual),
        ("..", TokenKind::Range),
        ("&&", TokenKind::AmpAmp),
        ("||", TokenKind::PipePipe),
        ("<<", TokenKind::ShiftLeft),
        (">>", TokenKind::ShiftRight),
        ("<=", TokenKind::LessEqual),
        (">=", TokenKind::GreaterEqual),
        ("!=", TokenKind::NotEqual),
        ("==", TokenKind::EqualEqual),
        ("::", TokenKind::ColonColon),
        ("+", TokenKind::Plus),
        ("-", TokenKind::Minus),
        ("*", TokenKind::Star),
        ("'", TokenKind::Apostrophe),
        ("%", TokenKind::Percent),
        ("\\", TokenKind::Backslash),
        ("/", TokenKind::Slash),
        (";", TokenKind::Semicolon),
        (",", TokenKind::Comma),
        (".", TokenKind::Dot),
        ("&", TokenKind::Amp),
        ("|", TokenKind::Pipe),
        ("^", TokenKind::Caret),
        ("<", TokenKind::Less),
        (">", TokenKind::Greater),
        ("=", TokenKind::Assign),
        ("(", TokenKind::LParen),
        (")", TokenKind::RParen),
        ("[", TokenKind::LBracket),
        ("]", TokenKind::RBracket),
        ("{", TokenKind::LBrace),
        ("}", TokenKind::RBrace),
        (":", TokenKind::Colon),
        ("!", TokenKind::Bang),
        ("?", TokenKind::Question),
    ];
    OPERATORS
        .iter()
        .find(|(pattern, _)| starts_with(input, index, pattern))
        .map(|(pattern, kind)| (*kind, pattern.len()))
}

#[cfg(test)]
mod tests {
    use super::{lex_source, TokenKind};

    #[test]
    fn tokenizes_directives_keywords_and_operators() {
        let tokens = lex_source(
            "#pragma keep\ncontainer demo { declare col x; witness y <= 1; if x === y { return x + 1; } }",
        )
        .expect("lexing should work");

        let kinds = tokens
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Pragma,
                TokenKind::Container,
                TokenKind::Identifier,
                TokenKind::LBrace,
                TokenKind::Declare,
                TokenKind::Col,
                TokenKind::Identifier,
                TokenKind::Semicolon,
                TokenKind::Witness,
                TokenKind::Identifier,
                TokenKind::LessEqual,
                TokenKind::Integer,
                TokenKind::Semicolon,
                TokenKind::If,
                TokenKind::Identifier,
                TokenKind::TripleEqual,
                TokenKind::Identifier,
                TokenKind::LBrace,
                TokenKind::Return,
                TokenKind::Identifier,
                TokenKind::Plus,
                TokenKind::Integer,
                TokenKind::Semicolon,
                TokenKind::RBrace,
                TokenKind::RBrace,
            ]
        );
    }

    #[test]
    fn tokenizes_literals_comments_and_special_identifiers() {
        let tokens = lex_source(
            "/* skip */ const a = 0x1f_FF; // line\nproofval p = 123_456; @hint $12 \"abc\" `x + y` ..+.. ..*.. ... .. **",
        )
        .expect("lexing should work");

        let kinds = tokens.iter().map(|token| token.kind).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Const,
                TokenKind::Identifier,
                TokenKind::Assign,
                TokenKind::HexInteger,
                TokenKind::Semicolon,
                TokenKind::ProofValue,
                TokenKind::Identifier,
                TokenKind::Assign,
                TokenKind::Integer,
                TokenKind::Semicolon,
                TokenKind::AtIdentifier,
                TokenKind::PositionalParam,
                TokenKind::StringLiteral,
                TokenKind::TemplateLiteral,
                TokenKind::RangeFill,
                TokenKind::RangeMulFill,
                TokenKind::Ellipsis,
                TokenKind::Range,
                TokenKind::Pow,
            ]
        );
        assert_eq!(tokens[3].lexeme, "0x1fFF");
        assert_eq!(tokens[8].lexeme, "123456");
        assert_eq!(tokens[12].lexeme, "abc");
        assert_eq!(tokens[13].lexeme, "x + y");
    }

    #[test]
    fn tokenizes_uppercase_hex_integer_prefixes() {
        let tokens = lex_source("const N = 0X1f_FF;").expect("lexing should work");

        assert_eq!(tokens[3].kind, TokenKind::HexInteger);
        assert_eq!(tokens[3].lexeme, "0X1fFF");
    }

    #[test]
    fn keeps_escaped_delimiters_inside_quoted_literals() {
        let tokens = lex_source(r#""a\"b" `x\`y`"#).expect("lexing should work");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, r#"a\"b"#);
        assert_eq!(tokens[1].kind, TokenKind::TemplateLiteral);
        assert_eq!(tokens[1].lexeme, r#"x\`y"#);
    }

    #[test]
    fn terminates_quoted_literals_after_escaped_backslashes() {
        let tokens = lex_source(r#""a\\" b"#).expect("lexing should work");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, r#"a\\"#);
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].lexeme, "b");
    }

    #[test]
    fn tokenizes_row_offset_markers() {
        let tokens = lex_source("a' b'2 'c 2'd").expect("lexing should work");

        let kinds = tokens.iter().map(|token| token.kind).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier,
                TokenKind::Apostrophe,
                TokenKind::Identifier,
                TokenKind::Apostrophe,
                TokenKind::Integer,
                TokenKind::Apostrophe,
                TokenKind::Identifier,
                TokenKind::Integer,
                TokenKind::Apostrophe,
                TokenKind::Identifier,
            ]
        );
    }

    #[test]
    fn rejects_unknown_characters() {
        let error = lex_source("col @").expect_err("bare at sign should fail");
        assert_eq!(error.index, 4);
    }
}

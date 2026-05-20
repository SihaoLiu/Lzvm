mod declarations;
mod expressions;
mod functions;
mod types;

pub use declarations::*;
pub use expressions::*;
pub use functions::*;
pub use types::*;

use std::collections::BTreeMap;

use crate::{lex_source, SourceFile, Token, TokenKind};
use declarations::{include_header, parse_named_statement};
use expressions::parse_expression_tokens;

const DEFAULT_CHALLENGE_STAGE: u32 = 2;
const DEFAULT_VALUE_STAGE: u32 = 1;
const DEFAULT_AIR_GROUP_VALUE_STAGE: u32 = 2;

pub fn parse_pragma_directives(source: &SourceFile) -> Result<Vec<PragmaDirective>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut directives = Vec::new();
    for token in &tokens {
        if token.kind == TokenKind::Pragma {
            directives.push(PragmaDirective {
                value: token.lexeme.clone(),
                source_name: source.source_name.clone(),
                start: token.start,
                end: token.end,
            });
        }
    }
    Ok(directives)
}

pub fn parse_fixed_file_pragmas(source: &SourceFile) -> Result<Vec<FixedFilePragma>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut directives = Vec::new();

    for token in &tokens {
        if token.kind != TokenKind::Pragma {
            continue;
        }

        let words = tokenize_pragma_words(
            &token.lexeme,
            token.end.saturating_sub(token.lexeme.len()),
            &source.source_name,
        )?;
        let Some(kind) = words
            .first()
            .and_then(|word| parse_fixed_file_pragma_kind(&word.value))
        else {
            continue;
        };

        validate_fixed_file_pragma_word_count(kind, &words, token.end, &source.source_name)?;
        let path = words.get(1).map(pragma_text_value);
        let column =
            match kind {
                FixedFilePragmaKind::FixedLoad => {
                    let word = &words[2];
                    Some(word.value.parse::<u32>().map_err(|_| {
                        ParseError::InvalidPragmaArgument {
                            source_name: source.source_name.clone(),
                            start: word.start,
                        }
                    })?)
                }
                _ => None,
            };

        directives.push(FixedFilePragma {
            kind,
            path,
            column,
            source_name: source.source_name.clone(),
            start: token.start,
            end: token.end,
        });
    }

    Ok(directives)
}

fn validate_fixed_file_pragma_word_count(
    kind: FixedFilePragmaKind,
    words: &[PragmaWord],
    pragma_end: usize,
    source_name: &str,
) -> Result<(), ParseError> {
    let expected = match kind {
        FixedFilePragmaKind::FixedExternal => 1,
        FixedFilePragmaKind::ExternFixedFile | FixedFilePragmaKind::OutputFixedFile => 2,
        FixedFilePragmaKind::FixedLoad => 3,
    };
    if words.len() == expected {
        return Ok(());
    }
    let start = words
        .get(expected)
        .map(|word| word.start)
        .unwrap_or(pragma_end);
    Err(ParseError::InvalidPragmaArgument {
        source_name: source_name.to_owned(),
        start,
    })
}

pub fn resolve_fixed_file_pragma_path(
    source: &SourceFile,
    pragma: &FixedFilePragma,
    context: &FixedFileTemplateContext,
) -> Result<Option<String>, ParseError> {
    resolve_fixed_file_pragma_path_with_values(source, pragma, context, &BTreeMap::new())
}

pub fn resolve_fixed_file_pragma_path_with_values(
    source: &SourceFile,
    pragma: &FixedFilePragma,
    context: &FixedFileTemplateContext,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Option<String>, ParseError> {
    let Some(path) = pragma.path.as_ref() else {
        return Ok(None);
    };
    if !path.template {
        return Ok(Some(path.value.clone()));
    }

    evaluate_template_text(
        source,
        &path.value,
        &TemplateBindings::fixed_file(context, values),
    )
    .map(Some)
    .map_err(|_| ParseError::InvalidPragmaArgument {
        source_name: pragma.source_name.clone(),
        start: pragma.start,
    })
}

pub fn parse_include_directives(source: &SourceFile) -> Result<Vec<IncludeDirective>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut directives = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let Some(header) = include_header(&tokens, index) else {
            index += 1;
            continue;
        };
        let path_index = header.directive_index + 1;
        let Some(path_token) = tokens.get(path_index) else {
            return Err(ParseError::ExpectedPath {
                source_name: source.source_name.clone(),
                start: tokens[header.directive_index].end,
            });
        };
        let file = resolve_include_path(source, path_token)?;

        let terminator_index = path_index + 1;
        let (end, next_index) = match tokens.get(terminator_index) {
            Some(terminator) if terminator.kind == TokenKind::Semicolon => {
                (terminator.end, terminator_index + 1)
            }
            Some(terminator)
                if has_line_break_between(&source.contents, path_token.end, terminator.start) =>
            {
                (path_token.end, terminator_index)
            }
            Some(terminator) => {
                return Err(ParseError::ExpectedTerminator {
                    source_name: source.source_name.clone(),
                    start: terminator.start,
                });
            }
            None => (source.contents.len(), tokens.len()),
        };

        directives.push(IncludeDirective {
            kind: header.kind,
            visibility: header.visibility,
            file,
            source_name: source.source_name.clone(),
            start: tokens[header.start_index].start,
            end,
        });
        index = next_index;
    }

    Ok(directives)
}

fn has_line_break_between(source: &str, start: usize, end: usize) -> bool {
    source
        .as_bytes()
        .get(start..end)
        .is_some_and(|bytes| bytes.iter().any(|byte| matches!(byte, b'\n' | b'\r')))
}

fn resolve_include_path(source: &SourceFile, token: &Token) -> Result<String, ParseError> {
    match token.kind {
        TokenKind::StringLiteral => Ok(token.lexeme.clone()),
        TokenKind::TemplateLiteral => {
            evaluate_template_include_path(source, token.start, token.end, &token.lexeme)
        }
        _ => Err(ParseError::ExpectedPath {
            source_name: source.source_name.clone(),
            start: token.start,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PragmaWord {
    value: String,
    template: bool,
    start: usize,
}

fn tokenize_pragma_words(
    value: &str,
    base: usize,
    source_name: &str,
) -> Result<Vec<PragmaWord>, ParseError> {
    let mut words = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        if bytes[index..].starts_with(b"//") {
            break;
        }
        if bytes[index..].starts_with(b"/*") {
            let Some(end) = value[index + 2..].find("*/") else {
                return Err(ParseError::InvalidPragmaArgument {
                    source_name: source_name.to_owned(),
                    start: base + index,
                });
            };
            index += end + 4;
            continue;
        }

        let start = index;
        let quote = bytes[index];
        if matches!(quote, b'"' | b'\'' | b'`') {
            index += 1;
            let content_start = index;
            let mut escaped = false;
            while index < bytes.len() {
                if escaped {
                    escaped = false;
                    index += 1;
                    continue;
                }
                if bytes[index] == b'\\' {
                    escaped = true;
                    index += 1;
                    continue;
                }
                if bytes[index] == quote {
                    break;
                }
                index += 1;
            }
            if index >= bytes.len() {
                return Err(ParseError::InvalidPragmaArgument {
                    source_name: source_name.to_owned(),
                    start: base + start,
                });
            }
            words.push(PragmaWord {
                value: value[content_start..index].to_owned(),
                template: quote == b'`',
                start: base + start,
            });
            index += 1;
            continue;
        }

        while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        words.push(PragmaWord {
            value: value[start..index].to_owned(),
            template: false,
            start: base + start,
        });
    }

    Ok(words)
}

fn parse_fixed_file_pragma_kind(value: &str) -> Option<FixedFilePragmaKind> {
    match value {
        "fixed_external" => Some(FixedFilePragmaKind::FixedExternal),
        "extern_fixed_file" => Some(FixedFilePragmaKind::ExternFixedFile),
        "fixed_load" => Some(FixedFilePragmaKind::FixedLoad),
        "output_fixed_file" => Some(FixedFilePragmaKind::OutputFixedFile),
        _ => None,
    }
}

fn pragma_text_value(word: &PragmaWord) -> PragmaTextValue {
    PragmaTextValue {
        value: word.value.clone(),
        template: word.template,
    }
}

fn evaluate_template_include_path(
    source: &SourceFile,
    start: usize,
    end: usize,
    template: &str,
) -> Result<String, ParseError> {
    evaluate_template_text(source, template, &TemplateBindings::default()).map_err(|_| {
        ParseError::TemplatePath {
            source_name: source.source_name.clone(),
            start,
            end,
        }
    })
}

fn evaluate_template_text(
    source: &SourceFile,
    template: &str,
    bindings: &TemplateBindings,
) -> Result<String, ()> {
    let mut resolved = String::new();
    let mut cursor = 0;

    while let Some(relative_start) = template[cursor..].find("${") {
        let segment_start = cursor + relative_start;
        resolved.push_str(&template[cursor..segment_start]);

        let expression_start = segment_start + 2;
        let Some(expression_end) = find_template_expression_end(template, expression_start) else {
            return Err(());
        };
        let expression = &template[expression_start..expression_end];
        let evaluated = evaluate_template_expression(source, expression, bindings)?;
        resolved.push_str(&evaluated);
        cursor = expression_end + 1;
    }

    resolved.push_str(&template[cursor..]);
    Ok(resolved)
}

fn find_template_expression_end(template: &str, start: usize) -> Option<usize> {
    let bytes = template.as_bytes();
    let mut index = start;
    let mut quote = None;
    let mut escaped = false;
    let mut brace_depth = 0_usize;

    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }

        match byte {
            b'"' | b'\'' | b'`' => quote = Some(byte),
            b'{' => brace_depth += 1,
            b'}' => {
                if brace_depth == 0 {
                    return Some(index);
                }
                brace_depth -= 1;
            }
            _ => {}
        }
        index += 1;
    }

    None
}

#[derive(Debug, Clone, Default)]
struct TemplateBindings {
    values: BTreeMap<String, TemplateValue>,
}

impl TemplateBindings {
    fn fixed_file(
        context: &FixedFileTemplateContext,
        bindings: &BTreeMap<String, FixedFileTemplateValue>,
    ) -> Self {
        let mut values = BTreeMap::new();
        for (name, value) in bindings {
            values.insert(name.clone(), TemplateValue::from(value));
        }
        values.insert(
            "AIRGROUP".to_owned(),
            TemplateValue::String(context.group_name.clone()),
        );
        values.insert(
            "AIRGROUP_ID".to_owned(),
            TemplateValue::Integer(context.group_id),
        );
        values.insert("AIR_ID".to_owned(), TemplateValue::Integer(context.unit_id));
        values.insert(
            "AIR_NAME".to_owned(),
            TemplateValue::String(context.unit_name.clone()),
        );
        values.insert(
            "AIRTEMPLATE".to_owned(),
            TemplateValue::String(context.template_name.clone()),
        );
        Self { values }
    }

    fn get(&self, name: &str) -> Option<TemplateValue> {
        self.values.get(name).cloned()
    }
}

impl From<&FixedFileTemplateValue> for TemplateValue {
    fn from(value: &FixedFileTemplateValue) -> Self {
        match value {
            FixedFileTemplateValue::Integer(value) => Self::Integer(*value),
            FixedFileTemplateValue::Boolean(value) => Self::Boolean(*value),
            FixedFileTemplateValue::String(value) => Self::String(value.clone()),
        }
    }
}

fn evaluate_template_expression(
    source: &SourceFile,
    expression: &str,
    bindings: &TemplateBindings,
) -> Result<String, ()> {
    let expression_source = SourceFile {
        contents: expression.to_owned(),
        file_dir: source.file_dir.clone(),
        full_path: source.full_path.clone(),
        source_name: source.source_name.clone(),
    };
    let tokens = lex_source(&expression_source.contents).map_err(|_| ())?;
    let (parsed, next_index) =
        parse_expression_tokens(&tokens, 0, tokens.len(), &expression_source).map_err(|_| ())?;
    if next_index != tokens.len() {
        return Err(());
    }
    evaluate_template_expression_value(&parsed, bindings)
        .map(TemplateValue::into_string)
        .ok_or(())
}

pub fn evaluate_fixed_file_template_value_expression(
    expression: &Expression,
) -> Option<FixedFileTemplateValue> {
    evaluate_fixed_file_template_value_expression_with_values(expression, &BTreeMap::new())
}

pub fn evaluate_fixed_file_template_value_expression_with_values(
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<FixedFileTemplateValue> {
    let values = values
        .iter()
        .map(|(name, value)| (name.clone(), TemplateValue::from(value)))
        .collect();
    evaluate_template_expression_value(expression, &TemplateBindings { values })
        .map(FixedFileTemplateValue::from)
}

#[derive(Debug, Clone)]
enum TemplateValue {
    Integer(i128),
    Boolean(bool),
    String(String),
}

impl From<TemplateValue> for FixedFileTemplateValue {
    fn from(value: TemplateValue) -> Self {
        match value {
            TemplateValue::Integer(value) => Self::Integer(value),
            TemplateValue::Boolean(value) => Self::Boolean(value),
            TemplateValue::String(value) => Self::String(value),
        }
    }
}

impl TemplateValue {
    fn into_string(self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::String(value) => value,
        }
    }

    fn as_integer(&self) -> Option<i128> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    fn truthy(&self) -> bool {
        match self {
            Self::Integer(value) => *value != 0,
            Self::Boolean(value) => *value,
            Self::String(value) => !value.is_empty(),
        }
    }
}

fn evaluate_template_expression_value(
    expression: &Expression,
    bindings: &TemplateBindings,
) -> Option<TemplateValue> {
    match &expression.kind {
        ExpressionKind::Integer(value) => parse_decimal_integer(value)
            .ok()
            .map(TemplateValue::Integer),
        ExpressionKind::HexInteger(value) => {
            parse_hex_integer(value).ok().map(TemplateValue::Integer)
        }
        ExpressionKind::StringLiteral(value) | ExpressionKind::TemplateLiteral(value) => {
            Some(TemplateValue::String(value.clone()))
        }
        ExpressionKind::Name(value) => bindings.get(value),
        ExpressionKind::Group(inner) => evaluate_template_expression_value(inner, bindings),
        ExpressionKind::Unary { op, expr } => {
            let value = evaluate_template_expression_value(expr, bindings)?;
            match op {
                UnaryOperator::Plus => value.as_integer().map(TemplateValue::Integer),
                UnaryOperator::Minus => value
                    .as_integer()
                    .and_then(|value| value.checked_neg())
                    .map(TemplateValue::Integer),
                UnaryOperator::Not => Some(TemplateValue::Boolean(!value.truthy())),
                UnaryOperator::Increment | UnaryOperator::Decrement => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let lhs = evaluate_template_expression_value(left, bindings)?;
            match op {
                BinaryOperator::LogicalAnd => {
                    if lhs.truthy() {
                        let rhs = evaluate_template_expression_value(right, bindings)?;
                        Some(rhs)
                    } else {
                        Some(lhs)
                    }
                }
                BinaryOperator::LogicalOr => {
                    if lhs.truthy() {
                        Some(lhs)
                    } else {
                        let rhs = evaluate_template_expression_value(right, bindings)?;
                        Some(rhs)
                    }
                }
                _ => {
                    let rhs = evaluate_template_expression_value(right, bindings)?;
                    evaluate_template_binary(*op, lhs, rhs)
                }
            }
        }
        _ => None,
    }
}

fn evaluate_template_binary(
    op: BinaryOperator,
    lhs: TemplateValue,
    rhs: TemplateValue,
) -> Option<TemplateValue> {
    match op {
        BinaryOperator::Add => match (lhs, rhs) {
            (TemplateValue::Integer(lhs), TemplateValue::Integer(rhs)) => {
                lhs.checked_add(rhs).map(TemplateValue::Integer)
            }
            (lhs, rhs) => Some(TemplateValue::String(format!(
                "{}{}",
                lhs.into_string(),
                rhs.into_string()
            ))),
        },
        BinaryOperator::Subtract => binary_integer_op(lhs, rhs, i128::checked_sub),
        BinaryOperator::Multiply => binary_integer_op(lhs, rhs, i128::checked_mul),
        BinaryOperator::Divide | BinaryOperator::Backslash => binary_integer_div(lhs, rhs),
        BinaryOperator::Modulo => binary_integer_mod(lhs, rhs),
        BinaryOperator::Power => binary_integer_pow(lhs, rhs),
        BinaryOperator::ShiftLeft => binary_integer_shift(lhs, rhs, true),
        BinaryOperator::ShiftRight => binary_integer_shift(lhs, rhs, false),
        BinaryOperator::BitAnd => binary_integer_bitwise(lhs, rhs, |a, b| a & b),
        BinaryOperator::BitXor => binary_integer_bitwise(lhs, rhs, |a, b| a ^ b),
        BinaryOperator::BitOr => binary_integer_bitwise(lhs, rhs, |a, b| a | b),
        BinaryOperator::Less => binary_integer_cmp(lhs, rhs, |a, b| a < b),
        BinaryOperator::LessEqual => binary_integer_cmp(lhs, rhs, |a, b| a <= b),
        BinaryOperator::Greater => binary_integer_cmp(lhs, rhs, |a, b| a > b),
        BinaryOperator::GreaterEqual => binary_integer_cmp(lhs, rhs, |a, b| a >= b),
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => binary_value_eq(lhs, rhs),
        BinaryOperator::NotEqual => binary_value_ne(lhs, rhs),
        _ => None,
    }
}

fn binary_integer_op(
    lhs: TemplateValue,
    rhs: TemplateValue,
    op: impl Fn(i128, i128) -> Option<i128>,
) -> Option<TemplateValue> {
    Some(TemplateValue::Integer(op(
        lhs.as_integer()?,
        rhs.as_integer()?,
    )?))
}

fn binary_integer_div(lhs: TemplateValue, rhs: TemplateValue) -> Option<TemplateValue> {
    let lhs = lhs.as_integer()?;
    let rhs = rhs.as_integer()?;
    (rhs != 0).then(|| TemplateValue::Integer(lhs / rhs))
}

fn binary_integer_mod(lhs: TemplateValue, rhs: TemplateValue) -> Option<TemplateValue> {
    let lhs = lhs.as_integer()?;
    let rhs = rhs.as_integer()?;
    (rhs != 0).then(|| TemplateValue::Integer(lhs % rhs))
}

fn binary_integer_pow(lhs: TemplateValue, rhs: TemplateValue) -> Option<TemplateValue> {
    let lhs = lhs.as_integer()?;
    let rhs = rhs.as_integer()?;
    let exponent = u32::try_from(rhs).ok()?;
    lhs.checked_pow(exponent).map(TemplateValue::Integer)
}

fn binary_integer_shift(
    lhs: TemplateValue,
    rhs: TemplateValue,
    left: bool,
) -> Option<TemplateValue> {
    let lhs = lhs.as_integer()?;
    let rhs = u32::try_from(rhs.as_integer()?).ok()?;
    if left {
        lhs.checked_shl(rhs).map(TemplateValue::Integer)
    } else {
        lhs.checked_shr(rhs).map(TemplateValue::Integer)
    }
}

fn binary_integer_bitwise(
    lhs: TemplateValue,
    rhs: TemplateValue,
    op: impl Fn(i128, i128) -> i128,
) -> Option<TemplateValue> {
    Some(TemplateValue::Integer(op(
        lhs.as_integer()?,
        rhs.as_integer()?,
    )))
}

fn binary_integer_cmp(
    lhs: TemplateValue,
    rhs: TemplateValue,
    op: impl Fn(i128, i128) -> bool,
) -> Option<TemplateValue> {
    Some(TemplateValue::Boolean(op(
        lhs.as_integer()?,
        rhs.as_integer()?,
    )))
}

fn binary_value_eq(lhs: TemplateValue, rhs: TemplateValue) -> Option<TemplateValue> {
    Some(TemplateValue::Boolean(match (lhs, rhs) {
        (TemplateValue::Integer(lhs), TemplateValue::Integer(rhs)) => lhs == rhs,
        (TemplateValue::Boolean(lhs), TemplateValue::Boolean(rhs)) => lhs == rhs,
        (TemplateValue::String(lhs), TemplateValue::String(rhs)) => lhs == rhs,
        _ => false,
    }))
}

fn binary_value_ne(lhs: TemplateValue, rhs: TemplateValue) -> Option<TemplateValue> {
    binary_value_eq(lhs, rhs).map(|value| match value {
        TemplateValue::Boolean(value) => TemplateValue::Boolean(!value),
        other => other,
    })
}

fn parse_decimal_integer(value: &str) -> Result<i128, ()> {
    value.parse::<i128>().map_err(|_| ())
}

fn parse_hex_integer(value: &str) -> Result<i128, ()> {
    let digits = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .ok_or(())?;
    i128::from_str_radix(digits, 16).map_err(|_| ())
}

pub fn parse_use_directives(source: &SourceFile) -> Result<Vec<UseDirective>, ParseError> {
    let tokens = lex_source(&source.contents).map_err(|error| ParseError::Lex {
        source_name: source.source_name.clone(),
        error,
    })?;
    let mut directives = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Use {
            index += 1;
            continue;
        }

        let statement = parse_named_statement(&tokens, index, source)?;
        directives.push(UseDirective {
            name: statement.name,
            alias: statement.alias,
            source_name: source.source_name.clone(),
            start: statement.start,
            end: statement.end,
        });
        index = statement.next_index;
    }

    Ok(directives)
}

#[cfg(test)]
mod tests;

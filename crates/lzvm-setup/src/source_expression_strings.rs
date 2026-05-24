use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, SourceProgram,
    UnaryOperator,
};

use crate::{
    source_constraint_lowering::SourceExpressionAliases,
    source_statement_hints::{SourceExpressionArrayAlias, SourceExpressionArrayAliases},
    source_static_values::{evaluate_source_static_expression, static_value_integer},
};

pub(crate) fn source_expression_string_call_value(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<String> {
    let ExpressionKind::Call { callee, args } = &strip_group_expression(expression).kind else {
        return None;
    };
    if args.len() != 1 || args[0].name.is_some() {
        return None;
    }
    let ExpressionKind::Name(name) = &strip_group_expression(callee).kind else {
        return None;
    };
    if name != "string" {
        return None;
    }
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    source_expression_string_value(
        program,
        &args[0].value,
        values,
        expression_aliases,
        expression_array_aliases,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
        0,
    )
}

fn source_expression_string_value(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
    parent_precedence: u8,
) -> Option<String> {
    let expression = strip_group_expression(expression);
    if let ExpressionKind::Name(name) = &expression.kind {
        if let Some(alias) = expression_aliases.get(name) {
            if !resolving_aliases.insert(name.clone()) {
                return None;
            }
            let value = source_expression_string_value(
                program,
                alias,
                values,
                expression_aliases,
                expression_array_aliases,
                resolving_aliases,
                resolving_array_aliases,
                parent_precedence,
            );
            resolving_aliases.remove(name);
            return value;
        }
    }
    if matches!(&expression.kind, ExpressionKind::Index { .. })
        && source_expression_index_chain(expression)
            .is_some_and(|(name, _)| expression_array_aliases.contains_key(name))
    {
        return source_index_expression_string(
            program,
            expression,
            values,
            expression_aliases,
            expression_array_aliases,
            resolving_aliases,
            resolving_array_aliases,
            parent_precedence,
        );
    }
    if let Some(value) = evaluate_source_static_expression(program, expression, values) {
        return Some(source_static_value_string(value));
    }

    let precedence = source_expression_string_precedence(expression);
    let value = match &expression.kind {
        ExpressionKind::Integer(value)
        | ExpressionKind::HexInteger(value)
        | ExpressionKind::StringLiteral(value)
        | ExpressionKind::TemplateLiteral(value)
        | ExpressionKind::PositionalParam(value) => value.clone(),
        ExpressionKind::Name(name) => name.clone(),
        ExpressionKind::Array(items) => items
            .iter()
            .map(|item| {
                source_expression_string_value(
                    program,
                    item,
                    values,
                    expression_aliases,
                    expression_array_aliases,
                    resolving_aliases,
                    resolving_array_aliases,
                    0,
                )
            })
            .collect::<Option<Vec<_>>>()?
            .join(","),
        ExpressionKind::Unary { op, expr } => {
            let value = source_expression_string_value(
                program,
                expr,
                values,
                expression_aliases,
                expression_array_aliases,
                resolving_aliases,
                resolving_array_aliases,
                precedence,
            )?;
            match op {
                UnaryOperator::Plus => value,
                UnaryOperator::Minus => format!("-{value}"),
                UnaryOperator::Not => format!("!{value}"),
                UnaryOperator::Increment | UnaryOperator::Decrement => return None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let op = source_expression_string_binary_operator(*op)?;
            let left = source_expression_string_value(
                program,
                left,
                values,
                expression_aliases,
                expression_array_aliases,
                resolving_aliases,
                resolving_array_aliases,
                precedence,
            )?;
            let right = source_expression_string_value(
                program,
                right,
                values,
                expression_aliases,
                expression_array_aliases,
                resolving_aliases,
                resolving_array_aliases,
                precedence,
            )?;
            format!("{left} {op} {right}")
        }
        ExpressionKind::Call { callee, args } => {
            let callee = source_expression_string_value(
                program,
                callee,
                values,
                expression_aliases,
                expression_array_aliases,
                resolving_aliases,
                resolving_array_aliases,
                precedence,
            )?;
            let args = args
                .iter()
                .map(|arg| {
                    let value = source_expression_string_value(
                        program,
                        &arg.value,
                        values,
                        expression_aliases,
                        expression_array_aliases,
                        resolving_aliases,
                        resolving_array_aliases,
                        0,
                    )?;
                    Some(match &arg.name {
                        Some(name) => format!("{name}: {value}"),
                        None => value,
                    })
                })
                .collect::<Option<Vec<_>>>()?
                .join(", ");
            format!("{callee}({args})")
        }
        ExpressionKind::Index { .. } => source_index_expression_string(
            program,
            expression,
            values,
            expression_aliases,
            expression_array_aliases,
            resolving_aliases,
            resolving_array_aliases,
            precedence,
        )?,
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => {
            let target = source_expression_string_value(
                program,
                target,
                values,
                expression_aliases,
                expression_array_aliases,
                resolving_aliases,
                resolving_array_aliases,
                precedence,
            )?;
            let offset = source_expression_row_offset_value(program, offset, *prior, values)?;
            source_row_offset_string(&target, offset)
        }
        ExpressionKind::Group(_) => return None,
    };

    if precedence < parent_precedence {
        Some(format!("({value})"))
    } else {
        Some(value)
    }
}

fn source_index_expression_string(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
    precedence: u8,
) -> Option<String> {
    let (name, index_expressions) = source_expression_index_chain(expression)?;
    let indices = index_expressions
        .iter()
        .map(|index| source_expression_index(program, index, values))
        .collect::<Option<Vec<_>>>()?;
    if let Some(alias) = expression_array_aliases.get(name) {
        let element = source_expression_array_alias_path_element(
            alias,
            &indices,
            expression_array_aliases,
            resolving_array_aliases,
        )?;
        return match element {
            SourceExpressionArrayAliasElement::Expression(expression) => {
                source_expression_string_value(
                    program,
                    expression,
                    values,
                    expression_aliases,
                    expression_array_aliases,
                    resolving_aliases,
                    resolving_array_aliases,
                    precedence,
                )
            }
            SourceExpressionArrayAliasElement::NamedArray(name) => {
                Some(source_indexed_name_string(name, &indices))
            }
        };
    }
    Some(source_indexed_name_string(name, &indices))
}

fn source_expression_array_alias_path_element<'a>(
    alias: &'a SourceExpressionArrayAlias,
    indices: &[u32],
    expression_array_aliases: &'a SourceExpressionArrayAliases,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceExpressionArrayAliasElement<'a>> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if let Some(next_alias) = expression_array_aliases.get(name) {
                if !resolving_array_aliases.insert(name.clone()) {
                    return None;
                }
                let element = source_expression_array_alias_path_element(
                    next_alias,
                    indices,
                    expression_array_aliases,
                    resolving_array_aliases,
                );
                resolving_array_aliases.remove(name);
                return element;
            }
            Some(SourceExpressionArrayAliasElement::NamedArray(name))
        }
        SourceExpressionArrayAlias::Values(expressions) => source_expression_array_element(
            expressions,
            indices,
            expression_array_aliases,
            resolving_array_aliases,
        ),
    }
}

fn source_expression_array_element<'a>(
    expressions: &'a [Expression],
    indices: &[u32],
    expression_array_aliases: &'a SourceExpressionArrayAliases,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceExpressionArrayAliasElement<'a>> {
    let (index, rest) = indices.split_first()?;
    let expression = expressions.get(usize::try_from(*index).ok()?)?;
    if rest.is_empty() {
        return Some(SourceExpressionArrayAliasElement::Expression(expression));
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Array(expressions) => source_expression_array_element(
            expressions,
            rest,
            expression_array_aliases,
            resolving_array_aliases,
        ),
        ExpressionKind::Name(name) => {
            let alias = expression_array_aliases.get(name)?;
            source_expression_array_alias_path_element(
                alias,
                rest,
                expression_array_aliases,
                resolving_array_aliases,
            )
        }
        _ => None,
    }
}

enum SourceExpressionArrayAliasElement<'a> {
    Expression(&'a Expression),
    NamedArray(&'a str),
}

fn source_expression_index_chain(expression: &Expression) -> Option<(&str, Vec<&Expression>)> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some((name, Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (name, mut indices) = source_expression_index_chain(target)?;
            indices.push(index);
            Some((name, indices))
        }
        _ => None,
    }
}

fn source_expression_index(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<u32> {
    let value = evaluate_source_static_expression(program, expression, values)?;
    u32::try_from(static_value_integer(&value)?).ok()
}

fn source_expression_row_offset_value(
    program: &SourceProgram,
    expression: &Expression,
    prior: bool,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<i64> {
    let value = evaluate_source_static_expression(program, expression, values)?;
    let offset = static_value_integer(&value)?;
    let signed = if prior { offset.checked_neg()? } else { offset };
    i64::try_from(signed).ok()
}

fn source_indexed_name_string(name: &str, indices: &[u32]) -> String {
    let mut value = name.to_owned();
    for index in indices {
        value.push('[');
        value.push_str(&index.to_string());
        value.push(']');
    }
    value
}

fn source_row_offset_string(target: &str, offset: i64) -> String {
    match offset {
        0 => target.to_owned(),
        -1 => format!("'{target}"),
        1 => format!("{target}'"),
        offset if offset < 0 => format!("{}'{target}", offset.abs()),
        offset => format!("{target}'{offset}"),
    }
}

fn source_static_value_string(value: FixedFileTemplateValue) -> String {
    match value {
        FixedFileTemplateValue::Integer(value) => value.to_string(),
        FixedFileTemplateValue::Boolean(value) => value.to_string(),
        FixedFileTemplateValue::String(value) => value,
    }
}

fn source_expression_string_precedence(expression: &Expression) -> u8 {
    match &expression.kind {
        ExpressionKind::Binary { op, .. } => match op {
            BinaryOperator::LogicalOr => 1,
            BinaryOperator::LogicalAnd => 2,
            BinaryOperator::BitOr => 3,
            BinaryOperator::BitXor => 4,
            BinaryOperator::BitAnd => 5,
            BinaryOperator::EqualEqual | BinaryOperator::NotEqual | BinaryOperator::TripleEqual => {
                6
            }
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => 7,
            BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight => 8,
            BinaryOperator::Add | BinaryOperator::Subtract => 9,
            BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulo
            | BinaryOperator::Backslash => 10,
            BinaryOperator::Power => 11,
            BinaryOperator::Assign
            | BinaryOperator::PlusAssign
            | BinaryOperator::MinusAssign
            | BinaryOperator::StarAssign
            | BinaryOperator::ConstrainedAssign
            | BinaryOperator::Range
            | BinaryOperator::RangeFill
            | BinaryOperator::RangeMulFill => 0,
        },
        ExpressionKind::Unary { .. } => 12,
        ExpressionKind::Call { .. }
        | ExpressionKind::Index { .. }
        | ExpressionKind::RowOffset { .. } => 13,
        ExpressionKind::Group(inner) => source_expression_string_precedence(inner),
        _ => 14,
    }
}

fn source_expression_string_binary_operator(op: BinaryOperator) -> Option<&'static str> {
    match op {
        BinaryOperator::Power => Some("**"),
        BinaryOperator::Multiply => Some("*"),
        BinaryOperator::Divide => Some("/"),
        BinaryOperator::Modulo => Some("%"),
        BinaryOperator::Backslash => Some("\\"),
        BinaryOperator::Add => Some("+"),
        BinaryOperator::Subtract => Some("-"),
        BinaryOperator::ShiftLeft => Some("<<"),
        BinaryOperator::ShiftRight => Some(">>"),
        BinaryOperator::Less => Some("<"),
        BinaryOperator::LessEqual => Some("<="),
        BinaryOperator::Greater => Some(">"),
        BinaryOperator::GreaterEqual => Some(">="),
        BinaryOperator::EqualEqual => Some("=="),
        BinaryOperator::NotEqual => Some("!="),
        BinaryOperator::TripleEqual => Some("==="),
        BinaryOperator::BitAnd => Some("&"),
        BinaryOperator::BitXor => Some("^"),
        BinaryOperator::BitOr => Some("|"),
        BinaryOperator::LogicalAnd => Some("&&"),
        BinaryOperator::LogicalOr => Some("||"),
        _ => None,
    }
}

fn strip_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_group_expression(inner),
        _ => expression,
    }
}

use std::collections::BTreeMap;

use lzvm_pil::{Expression, ExpressionKind, FixedFileTemplateValue};

use crate::{
    source_statement_hints::{SourceExpressionArrayAlias, SourceExpressionArrayAliases},
    source_static_values::{source_static_array_length, source_static_array_length_key},
};

pub(crate) fn insert_source_expr_array_alias_length(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    target_name: &str,
    alias: &SourceExpressionArrayAlias,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<()> {
    let length = source_expression_array_alias_length(values, alias, expression_array_aliases)?;
    insert_source_expr_array_length_value(values, target_name, length)?;
    insert_source_expr_array_nested_lengths(values, target_name, alias, expression_array_aliases);
    Some(())
}

fn insert_source_expr_array_length_value(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    target_name: &str,
    length: usize,
) -> Option<()> {
    values.insert(
        source_static_array_length_key(target_name),
        FixedFileTemplateValue::Integer(i128::try_from(length).ok()?),
    );
    Some(())
}

fn insert_source_expr_array_nested_lengths(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    target_name: &str,
    alias: &SourceExpressionArrayAlias,
    expression_array_aliases: &SourceExpressionArrayAliases,
) {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if let Some(alias) = expression_array_aliases.get(name) {
                insert_source_expr_array_nested_lengths(
                    values,
                    target_name,
                    alias,
                    expression_array_aliases,
                );
            } else {
                insert_source_expr_array_nested_static_lengths(values, target_name, name);
            }
        }
        SourceExpressionArrayAlias::Values(expressions)
        | SourceExpressionArrayAlias::ScopedValues { expressions, .. } => {
            insert_source_expr_array_nested_expression_lengths(
                values,
                target_name,
                expressions,
                expression_array_aliases,
            );
        }
        SourceExpressionArrayAlias::Call { lengths, .. } => {
            insert_source_expr_array_nested_uniform_lengths(values, target_name, lengths);
        }
    }
}

fn insert_source_expr_array_nested_static_lengths(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    target_name: &str,
    source_name: &str,
) {
    let Some(length) = source_static_array_length(values, source_name)
        .and_then(|length| usize::try_from(length).ok())
    else {
        return;
    };
    for index in 0..length {
        let source_child = format!("{source_name}[{index}]");
        let Some(child_length) = source_static_array_length(values, &source_child)
            .and_then(|length| usize::try_from(length).ok())
        else {
            continue;
        };
        let target_child = format!("{target_name}[{index}]");
        let _ = insert_source_expr_array_length_value(values, &target_child, child_length);
        insert_source_expr_array_nested_static_lengths(values, &target_child, &source_child);
    }
}

fn insert_source_expr_array_nested_expression_lengths(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    target_name: &str,
    expressions: &[Expression],
    expression_array_aliases: &SourceExpressionArrayAliases,
) {
    for (index, expression) in expressions.iter().enumerate() {
        let target_child = format!("{target_name}[{index}]");
        match &expression.kind {
            ExpressionKind::Array(elements) => {
                let _ =
                    insert_source_expr_array_length_value(values, &target_child, elements.len());
                insert_source_expr_array_nested_expression_lengths(
                    values,
                    &target_child,
                    elements,
                    expression_array_aliases,
                );
            }
            ExpressionKind::Name(name) => {
                if let Some(alias) = expression_array_aliases.get(name) {
                    if let Some(length) = source_expression_array_alias_length(
                        values,
                        alias,
                        expression_array_aliases,
                    ) {
                        let _ =
                            insert_source_expr_array_length_value(values, &target_child, length);
                    }
                    insert_source_expr_array_nested_lengths(
                        values,
                        &target_child,
                        alias,
                        expression_array_aliases,
                    );
                } else if let Some(length) = source_static_array_length(values, name)
                    .and_then(|length| usize::try_from(length).ok())
                {
                    let _ = insert_source_expr_array_length_value(values, &target_child, length);
                    insert_source_expr_array_nested_static_lengths(values, &target_child, name);
                }
            }
            _ => {}
        }
    }
}

fn insert_source_expr_array_nested_uniform_lengths(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    target_name: &str,
    lengths: &[usize],
) {
    let Some((&length, rest)) = lengths.split_first() else {
        return;
    };
    if rest.is_empty() {
        return;
    }
    for index in 0..length {
        let target_child = format!("{target_name}[{index}]");
        let _ = insert_source_expr_array_length_value(values, &target_child, rest[0]);
        insert_source_expr_array_nested_uniform_lengths(values, &target_child, rest);
    }
}

fn source_expression_array_alias_length(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias: &SourceExpressionArrayAlias,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<usize> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => expression_array_aliases
            .get(name)
            .and_then(|alias| {
                source_expression_array_alias_length(values, alias, expression_array_aliases)
            })
            .or_else(|| {
                source_static_array_length(values, name)
                    .and_then(|length| usize::try_from(length).ok())
            }),
        SourceExpressionArrayAlias::Values(expressions) => Some(expressions.len()),
        SourceExpressionArrayAlias::ScopedValues { expressions, .. } => Some(expressions.len()),
        SourceExpressionArrayAlias::Call { lengths, .. } => lengths.first().copied(),
    }
}

use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{CallArgument, Expression, ExpressionKind};

use crate::{
    source_expression_info::SourceExpressionAliasScope,
    source_statement_hints::{SourceExpressionArrayAlias, SourceExpressionArrayAliases},
};

use super::source_strip_group_expression;

pub(super) fn source_import_expression_scope(
    source_expression: &Expression,
    expression: Expression,
    base_scope: &SourceExpressionAliasScope,
    imported_scope: &SourceExpressionAliasScope,
    alias_scope: &mut SourceExpressionAliasScope,
) -> Expression {
    let expression_renames = source_expression_scope_renames(
        source_expression,
        base_scope.expressions.as_ref(),
        imported_scope.expressions.as_ref(),
        alias_scope,
    );
    let array_renames = source_expression_scope_array_renames(
        source_expression,
        base_scope.expression_arrays.as_ref(),
        imported_scope.expression_arrays.as_ref(),
        alias_scope,
    );
    let mut renames = expression_renames.clone();
    renames.extend(array_renames.clone());

    for (name, expression) in imported_scope.expressions.iter() {
        let Some(renamed) = expression_renames.get(name) else {
            continue;
        };
        alias_scope.expressions_mut().insert(
            renamed.clone(),
            source_expression_with_name_renames(expression, &renames),
        );
    }
    for (name, alias) in imported_scope.expression_arrays.iter() {
        let Some(renamed) = array_renames.get(name) else {
            continue;
        };
        let alias_renames = source_array_alias_binding_renames(name, alias, &renames);
        alias_scope.expression_arrays_mut().insert(
            renamed.clone(),
            source_array_alias_with_name_renames(alias, &alias_renames),
        );
    }

    source_expression_with_name_renames(&expression, &renames)
}

fn source_expression_scope_renames(
    source_expression: &Expression,
    base_aliases: &BTreeMap<String, Expression>,
    imported_aliases: &BTreeMap<String, Expression>,
    alias_scope: &SourceExpressionAliasScope,
) -> BTreeMap<String, String> {
    let mut reserved_names = source_alias_scope_reserved_names(alias_scope);
    let mut renames = BTreeMap::new();
    for (name, _) in imported_aliases
        .iter()
        .filter(|(name, expression)| base_aliases.get(*name) != Some(*expression))
    {
        let renamed = source_imported_alias_name(source_expression, &reserved_names, renames.len());
        reserved_names.insert(renamed.clone());
        renames.insert(name.clone(), renamed);
    }
    renames
}

fn source_expression_scope_array_renames(
    source_expression: &Expression,
    base_aliases: &SourceExpressionArrayAliases,
    imported_aliases: &SourceExpressionArrayAliases,
    alias_scope: &SourceExpressionAliasScope,
) -> BTreeMap<String, String> {
    let mut reserved_names = source_alias_scope_reserved_names(alias_scope);
    let mut renames = BTreeMap::new();
    for (name, _) in imported_aliases.iter().filter(|(name, alias)| {
        base_aliases
            .get(*name)
            .is_none_or(|base_alias| !source_expression_array_alias_same(base_alias, alias))
    }) {
        let renamed =
            source_imported_array_alias_name(source_expression, &reserved_names, renames.len());
        reserved_names.insert(renamed.clone());
        renames.insert(name.clone(), renamed);
    }
    renames
}

fn source_expression_array_alias_same(
    left: &SourceExpressionArrayAlias,
    right: &SourceExpressionArrayAlias,
) -> bool {
    match (left, right) {
        (SourceExpressionArrayAlias::Name(left), SourceExpressionArrayAlias::Name(right)) => {
            left == right
        }
        (SourceExpressionArrayAlias::Values(left), SourceExpressionArrayAlias::Values(right)) => {
            left == right
        }
        (
            SourceExpressionArrayAlias::ScopedValues {
                expressions: left,
                scope: left_scope,
            },
            SourceExpressionArrayAlias::ScopedValues {
                expressions: right,
                scope: right_scope,
            },
        ) => left == right && std::sync::Arc::ptr_eq(left_scope, right_scope),
        (
            SourceExpressionArrayAlias::Call {
                expression: left,
                lengths: left_lengths,
            },
            SourceExpressionArrayAlias::Call {
                expression: right,
                lengths: right_lengths,
            },
        ) => left == right && left_lengths == right_lengths,
        _ => false,
    }
}

fn source_imported_alias_name(
    source_expression: &Expression,
    reserved_names: &BTreeSet<String>,
    index: usize,
) -> String {
    source_unique_imported_name(
        "__lzvm_expr_call",
        source_expression.start,
        index,
        reserved_names,
    )
}

fn source_imported_array_alias_name(
    source_expression: &Expression,
    reserved_names: &BTreeSet<String>,
    index: usize,
) -> String {
    source_unique_imported_name(
        "__lzvm_expr_array_call",
        source_expression.start,
        index,
        reserved_names,
    )
}

fn source_unique_imported_name(
    prefix: &str,
    start: usize,
    index: usize,
    reserved_names: &BTreeSet<String>,
) -> String {
    let mut suffix = index;
    loop {
        let name = format!("{prefix}_{start}_{suffix}");
        if !reserved_names.contains(&name) {
            return name;
        }
        suffix += 1;
    }
}

fn source_alias_scope_reserved_names(alias_scope: &SourceExpressionAliasScope) -> BTreeSet<String> {
    alias_scope
        .expressions
        .keys()
        .chain(alias_scope.expression_arrays.keys())
        .cloned()
        .collect()
}

fn source_expression_with_name_renames(
    expression: &Expression,
    renames: &BTreeMap<String, String>,
) -> Expression {
    let kind = match &expression.kind {
        ExpressionKind::Name(name) => {
            if let Some(renamed) = renames.get(name) {
                return Expression {
                    kind: ExpressionKind::Name(renamed.clone()),
                    source_name: expression.source_name.clone(),
                    start: expression.start,
                    end: expression.end,
                };
            }
            ExpressionKind::Name(name.clone())
        }
        ExpressionKind::Group(inner) => ExpressionKind::Group(Box::new(
            source_expression_with_name_renames(inner, renames),
        )),
        ExpressionKind::Array(expressions) => ExpressionKind::Array(
            expressions
                .iter()
                .map(|expression| source_expression_with_name_renames(expression, renames))
                .collect(),
        ),
        ExpressionKind::Unary { op, expr } => ExpressionKind::Unary {
            op: *op,
            expr: Box::new(source_expression_with_name_renames(expr, renames)),
        },
        ExpressionKind::Binary { op, left, right } => ExpressionKind::Binary {
            op: *op,
            left: Box::new(source_expression_with_name_renames(left, renames)),
            right: Box::new(source_expression_with_name_renames(right, renames)),
        },
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => ExpressionKind::Ternary {
            condition: Box::new(source_expression_with_name_renames(condition, renames)),
            then_expr: Box::new(source_expression_with_name_renames(then_expr, renames)),
            else_expr: Box::new(source_expression_with_name_renames(else_expr, renames)),
        },
        ExpressionKind::Call { callee, args } => ExpressionKind::Call {
            callee: Box::new(source_expression_with_name_renames(callee, renames)),
            args: args
                .iter()
                .map(|arg| CallArgument {
                    name: arg.name.clone(),
                    value: source_expression_with_name_renames(&arg.value, renames),
                })
                .collect(),
        },
        ExpressionKind::Index { target, index } => ExpressionKind::Index {
            target: Box::new(source_expression_with_name_renames(target, renames)),
            index: Box::new(source_expression_with_name_renames(index, renames)),
        },
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => ExpressionKind::RowOffset {
            target: Box::new(source_expression_with_name_renames(target, renames)),
            offset: Box::new(source_expression_with_name_renames(offset, renames)),
            prior: *prior,
        },
        _ => expression.kind.clone(),
    };
    Expression {
        kind,
        source_name: expression.source_name.clone(),
        start: expression.start,
        end: expression.end,
    }
}

fn source_array_alias_with_name_renames(
    alias: &SourceExpressionArrayAlias,
    renames: &BTreeMap<String, String>,
) -> SourceExpressionArrayAlias {
    match alias {
        SourceExpressionArrayAlias::Name(name) => SourceExpressionArrayAlias::Name(
            renames.get(name).cloned().unwrap_or_else(|| name.clone()),
        ),
        SourceExpressionArrayAlias::Values(expressions) => SourceExpressionArrayAlias::Values(
            expressions
                .iter()
                .map(|expression| source_expression_with_name_renames(expression, renames))
                .collect(),
        ),
        SourceExpressionArrayAlias::ScopedValues { expressions, scope } => {
            SourceExpressionArrayAlias::ScopedValues {
                expressions: expressions.clone(),
                scope: scope.clone(),
            }
        }
        SourceExpressionArrayAlias::Call {
            expression,
            lengths,
        } => SourceExpressionArrayAlias::Call {
            expression: Box::new(source_expression_with_name_renames(expression, renames)),
            lengths: lengths.clone(),
        },
    }
}

fn source_array_alias_binding_renames(
    name: &str,
    alias: &SourceExpressionArrayAlias,
    renames: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    if !source_array_alias_self_reference_is_row_offset_projection(alias, name) {
        return renames.clone();
    }
    let mut alias_renames = renames.clone();
    alias_renames.remove(name);
    alias_renames
}

fn source_array_alias_self_reference_is_row_offset_projection(
    alias: &SourceExpressionArrayAlias,
    name: &str,
) -> bool {
    match alias {
        SourceExpressionArrayAlias::Values(expressions) => expressions.iter().all(|expression| {
            !source_expression_references_name_outside_row_offset_target(expression, name, false)
        }),
        SourceExpressionArrayAlias::ScopedValues { expressions, .. } => {
            expressions.iter().all(|expression| {
                !source_expression_references_name_outside_row_offset_target(
                    expression, name, false,
                )
            })
        }
        SourceExpressionArrayAlias::Name(_) | SourceExpressionArrayAlias::Call { .. } => false,
    }
}

fn source_expression_references_name_outside_row_offset_target(
    expression: &Expression,
    name: &str,
    in_row_offset_target: bool,
) -> bool {
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Name(candidate) => candidate == name && !in_row_offset_target,
        ExpressionKind::Group(inner) => {
            source_expression_references_name_outside_row_offset_target(
                inner,
                name,
                in_row_offset_target,
            )
        }
        ExpressionKind::Array(expressions) => expressions.iter().any(|expression| {
            source_expression_references_name_outside_row_offset_target(
                expression,
                name,
                in_row_offset_target,
            )
        }),
        ExpressionKind::Unary { expr, .. } => {
            source_expression_references_name_outside_row_offset_target(
                expr,
                name,
                in_row_offset_target,
            )
        }
        ExpressionKind::Binary { left, right, .. } => {
            source_expression_references_name_outside_row_offset_target(
                left,
                name,
                in_row_offset_target,
            ) || source_expression_references_name_outside_row_offset_target(
                right,
                name,
                in_row_offset_target,
            )
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            source_expression_references_name_outside_row_offset_target(
                condition,
                name,
                in_row_offset_target,
            ) || source_expression_references_name_outside_row_offset_target(
                then_expr,
                name,
                in_row_offset_target,
            ) || source_expression_references_name_outside_row_offset_target(
                else_expr,
                name,
                in_row_offset_target,
            )
        }
        ExpressionKind::Call { callee, args } => {
            source_expression_references_name_outside_row_offset_target(
                callee,
                name,
                in_row_offset_target,
            ) || args.iter().any(|arg| {
                source_expression_references_name_outside_row_offset_target(
                    &arg.value,
                    name,
                    in_row_offset_target,
                )
            })
        }
        ExpressionKind::Index { target, index } => {
            source_expression_references_name_outside_row_offset_target(
                target,
                name,
                in_row_offset_target,
            ) || source_expression_references_name_outside_row_offset_target(index, name, false)
        }
        ExpressionKind::RowOffset { target, offset, .. } => {
            source_expression_references_name_outside_row_offset_target(target, name, true)
                || source_expression_references_name_outside_row_offset_target(offset, name, false)
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => false,
    }
}

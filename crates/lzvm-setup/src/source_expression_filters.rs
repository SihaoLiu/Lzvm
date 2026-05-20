use std::collections::BTreeSet;

use lzvm_pil::{BinaryOperator, Expression, ExpressionKind};

pub(crate) fn source_expression_assigns_fixed_index(
    expression: Option<&Expression>,
    fixed_columns: &BTreeSet<String>,
) -> bool {
    let Some(expression) = expression else {
        return false;
    };
    let ExpressionKind::Binary { op, left, .. } = &strip_group_expression(expression).kind else {
        return false;
    };
    if *op != BinaryOperator::Assign {
        return false;
    }
    source_fixed_index_assignment_name(left).is_some_and(|name| fixed_columns.contains(name))
}

fn source_fixed_index_assignment_name(expression: &Expression) -> Option<&str> {
    let ExpressionKind::Index { target, .. } = &strip_group_expression(expression).kind else {
        return None;
    };
    let ExpressionKind::Name(name) = &strip_group_expression(target).kind else {
        return None;
    };
    Some(name)
}

fn strip_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_group_expression(inner),
        _ => expression,
    }
}

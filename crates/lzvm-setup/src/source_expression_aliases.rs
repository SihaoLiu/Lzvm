use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FunctionStatement, FunctionStatementDeclaration,
};

use crate::source_constraint_lowering::SourceExpressionAliases;

pub(crate) fn collect_source_template_expression_alias(
    statement: &FunctionStatement,
    expression_aliases: &mut SourceExpressionAliases,
) -> bool {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if declaration.type_name.as_deref() != Some("expr")
                || !declaration.array_dims.is_empty()
            {
                return false;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return false;
            };
            expression_aliases.insert(declaration.name.clone(), expression.clone());
            true
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            if declaration.type_name != "expr" || !declaration.array_dims.is_empty() {
                return false;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return false;
            };
            expression_aliases.insert(declaration.name.clone(), expression.clone());
            true
        }
        _ => source_expression_alias_assignment(
            statement.value_expression.as_ref(),
            expression_aliases,
        ),
    }
}

pub(crate) fn source_expression_alias_assignment(
    expression: Option<&Expression>,
    expression_aliases: &mut SourceExpressionAliases,
) -> bool {
    let Some(Expression {
        kind: ExpressionKind::Binary { op, left, right },
        ..
    }) = expression.map(strip_group_expression)
    else {
        return false;
    };
    let Some(name) = source_expression_name(left) else {
        return false;
    };
    let Some(current) = expression_aliases.get(name).cloned() else {
        return false;
    };
    let kind = match op {
        BinaryOperator::Assign => right.kind.clone(),
        BinaryOperator::PlusAssign => {
            source_binary_expression_kind(BinaryOperator::Add, current, (**right).clone())
        }
        BinaryOperator::MinusAssign => {
            source_binary_expression_kind(BinaryOperator::Subtract, current, (**right).clone())
        }
        BinaryOperator::StarAssign => {
            source_binary_expression_kind(BinaryOperator::Multiply, current, (**right).clone())
        }
        _ => return false,
    };
    expression_aliases.insert(
        name.to_owned(),
        Expression {
            kind,
            source_name: right.source_name.clone(),
            start: right.start,
            end: right.end,
        },
    );
    true
}

pub(crate) fn source_expression_alias_assignment_target(
    expression: Option<&Expression>,
) -> Option<&str> {
    let ExpressionKind::Binary { left, .. } = &expression.map(strip_group_expression)?.kind else {
        return None;
    };
    source_expression_name(left)
}

fn source_binary_expression_kind(
    op: BinaryOperator,
    left: Expression,
    right: Expression,
) -> ExpressionKind {
    ExpressionKind::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn source_expression_name(expression: &Expression) -> Option<&str> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(name),
        _ => None,
    }
}

fn strip_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_group_expression(inner),
        _ => expression,
    }
}

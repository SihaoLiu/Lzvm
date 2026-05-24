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
    let right = source_expression_with_alias_substitution(right, name, &current);
    let source_name = right.source_name.clone();
    let start = right.start;
    let end = right.end;
    let kind = match op {
        BinaryOperator::Assign => right.kind.clone(),
        BinaryOperator::PlusAssign => {
            source_binary_expression_kind(BinaryOperator::Add, current, right)
        }
        BinaryOperator::MinusAssign => {
            source_binary_expression_kind(BinaryOperator::Subtract, current, right)
        }
        BinaryOperator::StarAssign => {
            source_binary_expression_kind(BinaryOperator::Multiply, current, right)
        }
        _ => return false,
    };
    expression_aliases.insert(
        name.to_owned(),
        Expression {
            kind,
            source_name,
            start,
            end,
        },
    );
    true
}

fn source_expression_with_alias_substitution(
    expression: &Expression,
    name: &str,
    replacement: &Expression,
) -> Expression {
    let kind = match &expression.kind {
        ExpressionKind::Name(candidate) if candidate == name => {
            return replacement.clone();
        }
        ExpressionKind::Group(inner) => ExpressionKind::Group(Box::new(
            source_expression_with_alias_substitution(inner, name, replacement),
        )),
        ExpressionKind::Array(items) => ExpressionKind::Array(
            items
                .iter()
                .map(|item| source_expression_with_alias_substitution(item, name, replacement))
                .collect(),
        ),
        ExpressionKind::Unary { op, expr } => ExpressionKind::Unary {
            op: *op,
            expr: Box::new(source_expression_with_alias_substitution(
                expr,
                name,
                replacement,
            )),
        },
        ExpressionKind::Binary { op, left, right } => ExpressionKind::Binary {
            op: *op,
            left: Box::new(source_expression_with_alias_substitution(
                left,
                name,
                replacement,
            )),
            right: Box::new(source_expression_with_alias_substitution(
                right,
                name,
                replacement,
            )),
        },
        ExpressionKind::Call { callee, args } => ExpressionKind::Call {
            callee: Box::new(source_expression_with_alias_substitution(
                callee,
                name,
                replacement,
            )),
            args: args
                .iter()
                .map(|arg| lzvm_pil::CallArgument {
                    name: arg.name.clone(),
                    value: source_expression_with_alias_substitution(&arg.value, name, replacement),
                })
                .collect(),
        },
        ExpressionKind::Index { target, index } => ExpressionKind::Index {
            target: Box::new(source_expression_with_alias_substitution(
                target,
                name,
                replacement,
            )),
            index: Box::new(source_expression_with_alias_substitution(
                index,
                name,
                replacement,
            )),
        },
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => ExpressionKind::RowOffset {
            target: Box::new(source_expression_with_alias_substitution(
                target,
                name,
                replacement,
            )),
            offset: Box::new(source_expression_with_alias_substitution(
                offset,
                name,
                replacement,
            )),
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

use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FunctionStatement, FunctionStatementDeclaration,
};

use crate::{
    source_constraint_lowering::SourceExpressionAliases,
    source_statement_hints::{SourceExpressionArrayAlias, SourceExpressionArrayAliases},
};

pub(crate) fn collect_source_template_expression_alias(
    statement: &FunctionStatement,
    expression_aliases: &mut SourceExpressionAliases,
) -> bool {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if declaration.type_name.as_deref() != Some("expr") {
                if declaration.type_name.as_deref() == Some("int")
                    && declaration.array_dims.is_empty()
                {
                    return insert_source_scoped_name_alias_binding(
                        expression_aliases,
                        &declaration.name,
                        &declaration.source_name,
                        declaration.start,
                        declaration.end,
                    );
                }
                return false;
            }
            if !declaration.array_dims.is_empty() {
                return false;
            }
            let binding_name = source_alias_binding_name(&declaration.name);
            let expression = declaration
                .initializer_expression
                .clone()
                .unwrap_or_else(|| {
                    source_expression_self_reference(
                        binding_name.to_owned(),
                        declaration.source_name.clone(),
                        declaration.start,
                        declaration.end,
                    )
                });
            insert_source_expression_alias_binding(
                expression_aliases,
                &declaration.name,
                expression,
                &declaration.source_name,
                declaration.start,
                declaration.end,
            );
            true
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            if declaration.type_name != "expr" || !declaration.array_dims.is_empty() {
                return false;
            }
            let binding_name = source_alias_binding_name(&declaration.name);
            let expression = declaration
                .initializer_expression
                .clone()
                .unwrap_or_else(|| {
                    source_expression_self_reference(
                        binding_name.to_owned(),
                        declaration.source_name.clone(),
                        declaration.start,
                        declaration.end,
                    )
                });
            insert_source_expression_alias_binding(
                expression_aliases,
                &declaration.name,
                expression,
                &declaration.source_name,
                declaration.start,
                declaration.end,
            );
            true
        }
        Some(FunctionStatementDeclaration::Column(declaration)) => {
            for item in &declaration.items {
                if !item.array_dims.is_empty() {
                    continue;
                }
                insert_source_scoped_name_alias_binding(
                    expression_aliases,
                    &item.name,
                    &declaration.source_name,
                    declaration.start,
                    declaration.end,
                );
            }
            true
        }
        _ => source_expression_alias_assignment(
            statement.value_expression.as_ref(),
            expression_aliases,
        ),
    }
}

pub(crate) fn source_template_expression_alias_can_apply(
    statement: &FunctionStatement,
    expression_aliases: &SourceExpressionAliases,
) -> bool {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if declaration.type_name.as_deref() == Some("int") && declaration.array_dims.is_empty()
            {
                return true;
            }
            declaration.type_name.as_deref() == Some("expr") && declaration.array_dims.is_empty()
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            declaration.type_name == "expr" && declaration.array_dims.is_empty()
        }
        Some(FunctionStatementDeclaration::Column(declaration)) => declaration
            .items
            .iter()
            .any(|item| item.array_dims.is_empty()),
        _ => source_expression_alias_assignment_can_apply(
            statement.value_expression.as_ref(),
            expression_aliases,
        ),
    }
}

fn source_expression_self_reference(
    name: String,
    source_name: String,
    start: usize,
    end: usize,
) -> Expression {
    Expression {
        kind: ExpressionKind::Name(name),
        source_name,
        start,
        end,
    }
}

pub(crate) fn source_expression_is_self_reference(name: &str, expression: &Expression) -> bool {
    matches!(
        &strip_group_expression(expression).kind,
        ExpressionKind::Name(candidate) if candidate == name
    )
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
    let Some(name) = source_expression_assignment_alias_name(name, expression_aliases) else {
        return false;
    };
    if !matches!(
        op,
        BinaryOperator::Assign
            | BinaryOperator::PlusAssign
            | BinaryOperator::MinusAssign
            | BinaryOperator::StarAssign
    ) {
        return false;
    }
    let current = expression_aliases
        .remove(&name)
        .expect("resolved alias assignment target should exist");
    let right = if source_expression_references_name(right, &name) {
        source_expression_with_alias_substitution(right, &name, &current)
    } else {
        right.as_ref().clone()
    };
    let source_name = right.source_name.clone();
    let start = right.start;
    let end = right.end;
    let kind = match op {
        BinaryOperator::Assign => right.kind,
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
        name,
        Expression {
            kind,
            source_name,
            start,
            end,
        },
    );
    true
}

fn source_expression_references_name(expression: &Expression, name: &str) -> bool {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(candidate) => candidate == name,
        ExpressionKind::Group(inner) => source_expression_references_name(inner, name),
        ExpressionKind::Array(items) => items
            .iter()
            .any(|item| source_expression_references_name(item, name)),
        ExpressionKind::Unary { expr, .. } => source_expression_references_name(expr, name),
        ExpressionKind::Binary { left, right, .. } => {
            source_expression_references_name(left, name)
                || source_expression_references_name(right, name)
        }
        ExpressionKind::Call { callee, args } => {
            source_expression_references_name(callee, name)
                || args
                    .iter()
                    .any(|arg| source_expression_references_name(&arg.value, name))
        }
        ExpressionKind::Index { target, index } => {
            source_expression_references_name(target, name)
                || source_expression_references_name(index, name)
        }
        ExpressionKind::RowOffset { target, offset, .. } => {
            source_expression_references_name(target, name)
                || source_expression_references_name(offset, name)
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            source_expression_references_name(condition, name)
                || source_expression_references_name(then_expr, name)
                || source_expression_references_name(else_expr, name)
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => false,
    }
}

fn source_expression_alias_assignment_can_apply(
    expression: Option<&Expression>,
    expression_aliases: &SourceExpressionAliases,
) -> bool {
    let Some(Expression {
        kind: ExpressionKind::Binary { op, left, .. },
        ..
    }) = expression.map(strip_group_expression)
    else {
        return false;
    };
    if !matches!(
        op,
        BinaryOperator::Assign
            | BinaryOperator::PlusAssign
            | BinaryOperator::MinusAssign
            | BinaryOperator::StarAssign
    ) {
        return false;
    }
    let Some(name) = source_expression_name(left) else {
        return false;
    };
    source_expression_assignment_alias_name(name, expression_aliases).is_some()
}

pub(crate) fn insert_source_expression_alias_binding(
    expression_aliases: &mut SourceExpressionAliases,
    declaration_name: &str,
    expression: Expression,
    source_name: &str,
    start: usize,
    end: usize,
) {
    let binding_name = source_alias_binding_name(declaration_name);
    expression_aliases.insert(binding_name.to_owned(), expression);
    if binding_name != declaration_name {
        expression_aliases.insert(
            declaration_name.to_owned(),
            source_expression_self_reference(
                binding_name.to_owned(),
                source_name.to_owned(),
                start,
                end,
            ),
        );
    }
}

fn insert_source_scoped_name_alias_binding(
    expression_aliases: &mut SourceExpressionAliases,
    declaration_name: &str,
    source_name: &str,
    start: usize,
    end: usize,
) -> bool {
    let binding_name = source_alias_binding_name(declaration_name);
    if binding_name == declaration_name {
        return false;
    }
    expression_aliases.insert(
        binding_name.to_owned(),
        source_expression_self_reference(
            declaration_name.to_owned(),
            source_name.to_owned(),
            start,
            end,
        ),
    );
    true
}

pub(crate) fn insert_source_expression_array_alias_binding(
    expression_array_aliases: &mut SourceExpressionArrayAliases,
    declaration_name: &str,
    alias: SourceExpressionArrayAlias,
) {
    let binding_name = source_alias_binding_name(declaration_name);
    expression_array_aliases.insert(binding_name.to_owned(), alias);
    if binding_name != declaration_name {
        expression_array_aliases.insert(
            declaration_name.to_owned(),
            SourceExpressionArrayAlias::Name(binding_name.to_owned()),
        );
    }
}

pub(crate) fn source_alias_binding_name(name: &str) -> &str {
    name.rsplit_once('.')
        .map(|(_, binding_name)| binding_name)
        .filter(|binding_name| !binding_name.is_empty())
        .unwrap_or(name)
}

fn source_expression_assignment_alias_name(
    name: &str,
    expression_aliases: &SourceExpressionAliases,
) -> Option<String> {
    let current = expression_aliases.get(name)?;
    if let Some(target) = source_expression_name(current) {
        if target != name && expression_aliases.contains_key(target) {
            return Some(target.to_owned());
        }
    }
    Some(name.to_owned())
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

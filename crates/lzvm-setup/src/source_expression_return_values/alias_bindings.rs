use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, FunctionStatement,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_aliases::{
        collect_source_template_expression_alias, source_alias_binding_name,
        source_expression_alias_assignment_target, source_template_expression_alias_can_apply,
    },
    source_expression_info::{
        collect_expression_dependencies_into_scope, SourceExpressionAliasScope,
    },
    source_template_context::SourceTemplateLoweringContext,
};

use super::{
    import_scope::source_import_expression_scope,
    returned_calls::{
        source_expression_contains_returned_expr_call, source_import_returned_expression_calls,
    },
    source_expression_may_resolve, source_indexed_expression_scoped_array_scope,
    source_resolve_expression, source_statement_expression_alias_name,
    source_static_value_expression, source_strip_group_expression,
};

pub(crate) fn collect_source_returned_expression_alias_with_stack(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
) -> bool {
    let alias_name = source_statement_expression_alias_name(statement);
    let assignment_target =
        source_expression_alias_assignment_target(statement.value_expression.as_ref())
            .map(str::to_owned);
    let compound_operator =
        source_statement_compound_assignment_operator(statement.value_expression.as_ref());
    if !source_template_expression_alias_can_apply(statement, &alias_scope.expressions) {
        return false;
    }
    if !collect_source_template_expression_alias(statement, alias_scope.expressions_mut()) {
        return false;
    }
    if let Some(target) = assignment_target.as_ref() {
        values.remove(target);
    }
    let Some(name) = alias_name.or(assignment_target) else {
        return true;
    };
    let name = source_expression_alias_scope_key(alias_scope, &name)
        .unwrap_or(&name)
        .to_owned();
    if let Some(operator) = compound_operator {
        source_resolve_compound_assignment_alias_binding(
            context,
            &name,
            values,
            alias_scope,
            body_cache,
            call_stack,
            operator,
        );
        source_import_returned_expression_calls_for_compound_assignment_alias_binding(
            context,
            &name,
            values,
            alias_scope,
            body_cache,
            call_stack,
            operator,
        );
    } else {
        source_resolve_expression_alias_binding(
            context,
            &name,
            values,
            alias_scope,
            body_cache,
            call_stack,
            true,
            false,
        );
        source_import_returned_expression_calls_for_alias_binding(
            context,
            &name,
            values,
            alias_scope,
            body_cache,
            call_stack,
        );
    }
    true
}

fn source_statement_compound_assignment_operator(
    expression: Option<&Expression>,
) -> Option<BinaryOperator> {
    let ExpressionKind::Binary { op, .. } = &expression.map(source_strip_group_expression)?.kind
    else {
        return None;
    };
    match op {
        BinaryOperator::PlusAssign => Some(BinaryOperator::Add),
        BinaryOperator::MinusAssign => Some(BinaryOperator::Subtract),
        BinaryOperator::StarAssign => Some(BinaryOperator::Multiply),
        _ => None,
    }
}

fn source_expression_alias_scope_key<'a>(
    alias_scope: &'a SourceExpressionAliasScope,
    name: &'a str,
) -> Option<&'a str> {
    if alias_scope.expressions.contains_key(name) {
        return Some(name);
    }
    let binding_name = source_alias_binding_name(name);
    (binding_name != name && alias_scope.expressions.contains_key(binding_name))
        .then_some(binding_name)
}

#[allow(clippy::too_many_arguments)]
fn source_resolve_expression_alias_binding(
    context: &SourceTemplateLoweringContext<'_>,
    name: &str,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolve_calls: bool,
) -> bool {
    let Some(expression) = alias_scope.expressions.get(name).cloned() else {
        return false;
    };
    let indexed_scope =
        source_indexed_expression_scoped_array_scope(&expression, alias_scope).cloned();
    if !source_expression_may_resolve(
        context.program,
        context.module,
        &expression,
        values,
        alias_scope,
        resolve_aliases,
        resolve_calls,
    ) {
        return false;
    };
    let mut changed = false;
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    let Some(resolved) = source_resolve_expression(
        context,
        &expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        resolve_aliases,
        resolve_calls,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
        &mut changed,
    ) else {
        return false;
    };
    if changed {
        let resolved = if let Some(indexed_scope) = indexed_scope.as_ref() {
            let base_scope = alias_scope.clone();
            source_import_expression_scope(
                &expression,
                resolved,
                &base_scope,
                indexed_scope,
                alias_scope,
            )
        } else {
            let source_alias_scope = alias_scope.clone();
            collect_expression_dependencies_into_scope(
                &expression,
                &source_alias_scope,
                alias_scope,
            );
            resolved
        };
        alias_scope
            .expressions_mut()
            .insert(name.to_owned(), resolved);
    }
    changed
}

#[allow(clippy::too_many_arguments)]
pub(super) fn source_resolve_compound_assignment_alias_binding(
    context: &SourceTemplateLoweringContext<'_>,
    name: &str,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    operator: BinaryOperator,
) -> bool {
    let Some(expression) = alias_scope.expressions_mut().remove(name) else {
        return false;
    };
    let mut changed = false;
    let Some(expression) = source_resolve_compound_assignment_expression(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        operator,
        &mut changed,
    ) else {
        return false;
    };
    alias_scope
        .expressions_mut()
        .insert(name.to_owned(), expression);
    changed
}

#[allow(clippy::too_many_arguments)]
fn source_resolve_compound_assignment_expression(
    context: &SourceTemplateLoweringContext<'_>,
    expression: Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    operator: BinaryOperator,
    changed: &mut bool,
) -> Option<Expression> {
    let Expression {
        kind,
        source_name,
        start,
        end,
    } = expression;
    let ExpressionKind::Binary { op, left, right } = kind else {
        return Some(Expression {
            kind,
            source_name,
            start,
            end,
        });
    };
    if op != operator {
        return Some(Expression {
            kind: ExpressionKind::Binary { op, left, right },
            source_name,
            start,
            end,
        });
    }
    let left = *left;
    let right = source_resolve_compound_assignment_operand(
        context,
        *right,
        values,
        alias_scope,
        body_cache,
        call_stack,
        changed,
    );
    let expression = Expression {
        kind: ExpressionKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
        source_name,
        start,
        end,
    };
    if let Some(expression) = source_static_value_expression(context, &expression, values) {
        *changed = true;
        return Some(expression);
    }
    Some(expression)
}

fn source_resolve_compound_assignment_operand(
    context: &SourceTemplateLoweringContext<'_>,
    expression: Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    changed: &mut bool,
) -> Expression {
    if source_expression_may_resolve(
        context.program,
        context.module,
        &expression,
        values,
        alias_scope,
        true,
        false,
    ) {
        let mut resolving_aliases = BTreeSet::new();
        let mut resolving_array_aliases = BTreeSet::new();
        let mut operand_changed = false;
        if let Some(resolved) = source_resolve_expression(
            context,
            &expression,
            values,
            alias_scope,
            body_cache,
            call_stack,
            true,
            false,
            &mut resolving_aliases,
            &mut resolving_array_aliases,
            &mut operand_changed,
        ) {
            *changed |= operand_changed;
            return resolved;
        }
    }
    expression
}

fn source_import_returned_expression_calls_for_alias_binding(
    context: &SourceTemplateLoweringContext<'_>,
    name: &str,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> bool {
    let Some(expression) = alias_scope.expressions.get(name) else {
        return false;
    };
    if !source_expression_contains_returned_expr_call(context.module, expression) {
        return false;
    }
    let Some(expression) = alias_scope.expressions_mut().remove(name) else {
        return false;
    };
    let Some(resolved) = source_import_returned_expression_calls(
        context,
        &expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
    ) else {
        alias_scope
            .expressions_mut()
            .insert(name.to_owned(), expression);
        return false;
    };
    let changed = resolved != expression;
    alias_scope
        .expressions_mut()
        .insert(name.to_owned(), resolved);
    changed
}

#[allow(clippy::too_many_arguments)]
fn source_import_returned_expression_calls_for_compound_assignment_alias_binding(
    context: &SourceTemplateLoweringContext<'_>,
    name: &str,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    operator: BinaryOperator,
) -> bool {
    let Some(expression) = alias_scope.expressions_mut().remove(name) else {
        return false;
    };
    let Some((resolved, changed)) = source_import_returned_expression_calls_for_compound_assignment(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        operator,
    ) else {
        return false;
    };
    alias_scope
        .expressions_mut()
        .insert(name.to_owned(), resolved);
    changed
}

#[allow(clippy::too_many_arguments)]
fn source_import_returned_expression_calls_for_compound_assignment(
    context: &SourceTemplateLoweringContext<'_>,
    expression: Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    operator: BinaryOperator,
) -> Option<(Expression, bool)> {
    let Expression {
        kind,
        source_name,
        start,
        end,
    } = expression;
    let ExpressionKind::Binary { op, left, right } = kind else {
        return Some((
            Expression {
                kind,
                source_name,
                start,
                end,
            },
            false,
        ));
    };
    if op != operator {
        return Some((
            Expression {
                kind: ExpressionKind::Binary { op, left, right },
                source_name,
                start,
                end,
            },
            false,
        ));
    }
    let right = *right;
    if !source_expression_contains_returned_expr_call(context.module, &right) {
        return Some((
            Expression {
                kind: ExpressionKind::Binary {
                    op,
                    left,
                    right: Box::new(right),
                },
                source_name,
                start,
                end,
            },
            false,
        ));
    }
    let Some(resolved_right) = source_import_returned_expression_calls(
        context,
        &right,
        values,
        alias_scope,
        body_cache,
        call_stack,
    ) else {
        return Some((
            Expression {
                kind: ExpressionKind::Binary {
                    op,
                    left,
                    right: Box::new(right),
                },
                source_name,
                start,
                end,
            },
            false,
        ));
    };
    let changed = resolved_right != right;
    Some((
        Expression {
            kind: ExpressionKind::Binary {
                op,
                left,
                right: Box::new(resolved_right),
            },
            source_name,
            start,
            end,
        },
        changed,
    ))
}

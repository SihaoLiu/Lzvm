use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    CallArgument, Expression, ExpressionKind, FixedFileTemplateValue, SourceProgramModule,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_info::{
        source_call_expression, source_function_call_bindings, SourceExpressionAliasScope,
    },
    source_template_context::SourceTemplateLoweringContext,
};

use super::{
    import_scope::source_import_expression_scope,
    returned_body::{source_import_returned_expression_body, source_returned_expression_body},
    source_function_returns_expr, source_strip_group_expression,
};

pub(super) fn source_expression_contains_returned_expr_call(
    module: &SourceProgramModule,
    expression: &Expression,
) -> bool {
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Call { callee, args } => {
            source_call_expression(Some(expression)).is_some_and(|(name, _)| {
                module
                    .functions
                    .iter()
                    .find(|function| function.name == name)
                    .is_some_and(|function| source_function_returns_expr(module, function))
            }) || source_expression_contains_returned_expr_call(module, callee)
                || args
                    .iter()
                    .any(|arg| source_expression_contains_returned_expr_call(module, &arg.value))
        }
        ExpressionKind::Group(inner) => {
            source_expression_contains_returned_expr_call(module, inner)
        }
        ExpressionKind::Array(expressions) => expressions
            .iter()
            .any(|expression| source_expression_contains_returned_expr_call(module, expression)),
        ExpressionKind::Unary { expr, .. } => {
            source_expression_contains_returned_expr_call(module, expr)
        }
        ExpressionKind::Binary { left, right, .. } => {
            source_expression_contains_returned_expr_call(module, left)
                || source_expression_contains_returned_expr_call(module, right)
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            source_expression_contains_returned_expr_call(module, condition)
                || source_expression_contains_returned_expr_call(module, then_expr)
                || source_expression_contains_returned_expr_call(module, else_expr)
        }
        ExpressionKind::Index { target, index } => {
            source_expression_contains_returned_expr_call(module, target)
                || source_expression_contains_returned_expr_call(module, index)
        }
        ExpressionKind::RowOffset { target, offset, .. } => {
            source_expression_contains_returned_expr_call(module, target)
                || source_expression_contains_returned_expr_call(module, offset)
        }
        ExpressionKind::Name(_)
        | ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => false,
    }
}

pub(crate) fn source_import_returned_expression_calls(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<Expression> {
    match &expression.kind {
        ExpressionKind::Call { callee, args } => {
            if let Some(imported) = source_import_returned_expression_call(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
            ) {
                return Some(imported);
            }
            let callee = source_import_returned_expression_calls(
                context,
                callee,
                values,
                alias_scope,
                body_cache,
                call_stack,
            )?;
            let args = args
                .iter()
                .map(|arg| {
                    Some(CallArgument {
                        name: arg.name.clone(),
                        value: source_import_returned_expression_calls(
                            context,
                            &arg.value,
                            values,
                            alias_scope,
                            body_cache,
                            call_stack,
                        )?,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Expression {
                kind: ExpressionKind::Call {
                    callee: Box::new(callee),
                    args,
                },
                source_name: expression.source_name.clone(),
                start: expression.start,
                end: expression.end,
            })
        }
        ExpressionKind::Group(inner) => Some(Expression {
            kind: ExpressionKind::Group(Box::new(source_import_returned_expression_calls(
                context,
                inner,
                values,
                alias_scope,
                body_cache,
                call_stack,
            )?)),
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        }),
        ExpressionKind::Array(expressions) => Some(Expression {
            kind: ExpressionKind::Array(
                expressions
                    .iter()
                    .map(|expression| {
                        source_import_returned_expression_calls(
                            context,
                            expression,
                            values,
                            alias_scope,
                            body_cache,
                            call_stack,
                        )
                    })
                    .collect::<Option<Vec<_>>>()?,
            ),
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        }),
        ExpressionKind::Unary { op, expr } => Some(Expression {
            kind: ExpressionKind::Unary {
                op: *op,
                expr: Box::new(source_import_returned_expression_calls(
                    context,
                    expr,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
            },
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        }),
        ExpressionKind::Binary { op, left, right } => Some(Expression {
            kind: ExpressionKind::Binary {
                op: *op,
                left: Box::new(source_import_returned_expression_calls(
                    context,
                    left,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
                right: Box::new(source_import_returned_expression_calls(
                    context,
                    right,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
            },
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        }),
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => Some(Expression {
            kind: ExpressionKind::Ternary {
                condition: Box::new(source_import_returned_expression_calls(
                    context,
                    condition,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
                then_expr: Box::new(source_import_returned_expression_calls(
                    context,
                    then_expr,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
                else_expr: Box::new(source_import_returned_expression_calls(
                    context,
                    else_expr,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
            },
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        }),
        ExpressionKind::Index { target, index } => Some(Expression {
            kind: ExpressionKind::Index {
                target: Box::new(source_import_returned_expression_calls(
                    context,
                    target,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
                index: Box::new(source_import_returned_expression_calls(
                    context,
                    index,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
            },
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        }),
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => Some(Expression {
            kind: ExpressionKind::RowOffset {
                target: Box::new(source_import_returned_expression_calls(
                    context,
                    target,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
                offset: Box::new(source_import_returned_expression_calls(
                    context,
                    offset,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                )?),
                prior: *prior,
            },
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        }),
        _ => Some(expression.clone()),
    }
}

pub(super) fn source_returned_expression_alias(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<Expression> {
    let (name, arguments) = source_call_expression(Some(expression))?;
    let function = context
        .module
        .functions
        .iter()
        .find(|function| function.name == name)?;
    if !source_function_returns_expr(context.module, function) {
        return None;
    }
    let mut bindings = source_function_call_bindings(
        context,
        function,
        arguments,
        values,
        alias_scope,
        body_cache,
        call_stack,
    )?;
    if !call_stack.insert(function.name.clone()) {
        return None;
    }
    let mut body_alias_scope = bindings.alias_scope;
    let expression = source_returned_expression_body(
        context,
        &function.statements,
        &mut bindings.values,
        &mut body_alias_scope,
        body_cache,
        call_stack,
    );
    call_stack.remove(&function.name);
    expression
}

fn source_import_returned_expression_call(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<Expression> {
    let (name, arguments) = source_call_expression(Some(expression))?;
    let function = context
        .module
        .functions
        .iter()
        .find(|function| function.name == name)?;
    if !source_function_returns_expr(context.module, function) {
        return None;
    }
    let base_scope = alias_scope.clone();
    let mut bindings = source_function_call_bindings(
        context,
        function,
        arguments,
        values,
        &base_scope,
        body_cache,
        call_stack,
    )?;
    if !call_stack.insert(function.name.clone()) {
        return None;
    }
    let returned = source_import_returned_expression_body(
        context,
        &function.statements,
        &mut bindings.values,
        &mut bindings.alias_scope,
        body_cache,
        call_stack,
    );
    call_stack.remove(&function.name);
    let returned = returned?;
    Some(source_import_expression_scope(
        expression,
        returned,
        &base_scope,
        &bindings.alias_scope,
        alias_scope,
    ))
}

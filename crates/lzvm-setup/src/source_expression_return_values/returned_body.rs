use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{Expression, FixedFileTemplateValue, FunctionStatement, FunctionStatementKind};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_info::SourceExpressionAliasScope,
    source_expression_statements::{
        apply_source_static_declaration, apply_source_static_expression_statement,
    },
    source_static_values::{evaluate_source_static_expression, static_value_truthy},
    source_template_context::SourceTemplateLoweringContext,
    source_template_do_while::source_static_do_while_loop_with_tokens,
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_aliases,
    source_template_while::{source_static_while_loop_with_tokens, STATIC_WHILE_LOOP_LIMIT},
};

use super::{
    collect_source_expr_destructuring_aliases,
    collect_source_template_expression_aliases_with_stack, source_expression_may_resolve,
    source_import_returned_expression_calls, source_resolve_expression,
};

pub(super) fn source_import_returned_expression_body(
    context: &SourceTemplateLoweringContext<'_>,
    statements: &[FunctionStatement],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<Expression> {
    for statement in statements {
        if statement.kind == FunctionStatementKind::Return {
            let expression = statement.value_expression.as_ref()?;
            return source_import_returned_expression_calls(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
            );
        }
        if statement.kind == FunctionStatementKind::If {
            if let Ok(Some(body)) = source_static_if_body_statements_with_aliases(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                &alias_scope.expressions,
                body_cache,
            ) {
                if let Some(expression) = source_import_returned_expression_body(
                    context,
                    &body,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                ) {
                    return Some(expression);
                }
            }
            continue;
        }
        if statement.kind == FunctionStatementKind::For {
            if let Ok(Some(loop_info)) = source_static_for_loop_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                body_cache,
            ) {
                for iteration_value in &loop_info.iteration_values {
                    values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                    if let Some(expression) = source_import_returned_expression_body(
                        context,
                        &loop_info.body_statements,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                    ) {
                        return Some(expression);
                    }
                }
                loop_info.apply_final_variable_value(values);
            }
            continue;
        }
        if statement.kind == FunctionStatementKind::While {
            if let Ok(Some(loop_info)) = source_static_while_loop_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                body_cache,
            ) {
                for _ in 0..STATIC_WHILE_LOOP_LIMIT {
                    let condition_value = evaluate_source_static_expression(
                        context.program,
                        &loop_info.condition,
                        values,
                    )?;
                    if !static_value_truthy(&condition_value) {
                        break;
                    }
                    if let Some(expression) = source_import_returned_expression_body(
                        context,
                        &loop_info.body_statements,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                    ) {
                        return Some(expression);
                    }
                }
            }
            continue;
        }
        if statement.kind == FunctionStatementKind::Do {
            if let Ok(Some(loop_info)) = source_static_do_while_loop_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                body_cache,
            ) {
                for _ in 0..STATIC_WHILE_LOOP_LIMIT {
                    if let Some(expression) = source_import_returned_expression_body(
                        context,
                        &loop_info.body_statements,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                    ) {
                        return Some(expression);
                    }
                    let condition_value = evaluate_source_static_expression(
                        context.program,
                        &loop_info.condition,
                        values,
                    )?;
                    if !static_value_truthy(&condition_value) {
                        break;
                    }
                }
            }
            continue;
        }
        apply_source_static_declaration(context.program, statement, values);
        apply_source_static_expression_statement(
            context.program,
            statement.value_expression.as_ref(),
            values,
        );
        collect_source_template_expression_aliases_with_stack(
            context,
            statement,
            values,
            body_cache,
            call_stack,
            alias_scope,
        );
    }
    None
}

pub(super) fn source_returned_expression_body(
    context: &SourceTemplateLoweringContext<'_>,
    statements: &[FunctionStatement],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<Expression> {
    for statement in statements {
        if statement.kind == FunctionStatementKind::Return {
            let expression = statement.value_expression.as_ref()?;
            if !source_expression_may_resolve(
                context.program,
                context.module,
                expression,
                values,
                alias_scope,
                true,
                true,
            ) {
                return Some(expression.clone());
            }
            let mut changed = false;
            let mut resolving_aliases = BTreeSet::new();
            let mut resolving_array_aliases = BTreeSet::new();
            return source_resolve_expression(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
                true,
                true,
                &mut resolving_aliases,
                &mut resolving_array_aliases,
                &mut changed,
            );
        }
        if statement.kind == FunctionStatementKind::If {
            if let Ok(Some(body)) = source_static_if_body_statements_with_aliases(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                &alias_scope.expressions,
                body_cache,
            ) {
                if let Some(expression) = source_returned_expression_body(
                    context,
                    &body,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                ) {
                    return Some(expression);
                }
            }
            continue;
        }
        if statement.kind == FunctionStatementKind::For {
            if let Ok(Some(loop_info)) = source_static_for_loop_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                body_cache,
            ) {
                for iteration_value in &loop_info.iteration_values {
                    values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                    if let Some(expression) = source_returned_expression_body(
                        context,
                        &loop_info.body_statements,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                    ) {
                        return Some(expression);
                    }
                }
                loop_info.apply_final_variable_value(values);
            }
            continue;
        }
        if statement.kind == FunctionStatementKind::While {
            if let Ok(Some(loop_info)) = source_static_while_loop_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                body_cache,
            ) {
                for _ in 0..STATIC_WHILE_LOOP_LIMIT {
                    let condition_value = evaluate_source_static_expression(
                        context.program,
                        &loop_info.condition,
                        values,
                    )?;
                    if !static_value_truthy(&condition_value) {
                        break;
                    }
                    if let Some(expression) = source_returned_expression_body(
                        context,
                        &loop_info.body_statements,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                    ) {
                        return Some(expression);
                    }
                }
            }
            continue;
        }
        if statement.kind == FunctionStatementKind::Do {
            if let Ok(Some(loop_info)) = source_static_do_while_loop_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                body_cache,
            ) {
                for _ in 0..STATIC_WHILE_LOOP_LIMIT {
                    if let Some(expression) = source_returned_expression_body(
                        context,
                        &loop_info.body_statements,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                    ) {
                        return Some(expression);
                    }
                    let condition_value = evaluate_source_static_expression(
                        context.program,
                        &loop_info.condition,
                        values,
                    )?;
                    if !static_value_truthy(&condition_value) {
                        break;
                    }
                }
            }
            continue;
        }
        apply_source_static_declaration(context.program, statement, values);
        apply_source_static_expression_statement(
            context.program,
            statement.value_expression.as_ref(),
            values,
        );
        if collect_source_expr_destructuring_aliases(
            context,
            statement,
            values,
            body_cache,
            call_stack,
            alias_scope,
        ) {
            continue;
        }
        collect_source_template_expression_aliases_with_stack(
            context,
            statement,
            values,
            body_cache,
            call_stack,
            alias_scope,
        );
    }
    None
}

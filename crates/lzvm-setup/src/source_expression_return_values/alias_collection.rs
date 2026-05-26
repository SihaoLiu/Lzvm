use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    FixedFileTemplateValue, FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind,
    SourceProgram,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_info::{
        collect_source_template_expression_array_alias, SourceExpressionAliasScope,
    },
    source_expression_statements::{
        apply_source_static_declaration, apply_source_static_expression_statement,
    },
    source_static_values::{evaluate_source_static_expression, static_value_truthy},
    source_template_context::SourceTemplateLoweringContext,
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_aliases,
    source_template_switch::source_static_switch_body_statements,
    source_template_while::{source_static_while_loop_with_tokens, STATIC_WHILE_LOOP_LIMIT},
};

use super::{
    collect_source_expr_destructuring_aliases, collect_source_returned_expression_alias_with_stack,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceExpressionAliasFlow {
    Fallthrough,
    Continue,
    Break,
}

pub(crate) fn collect_source_template_expression_aliases_with_stack(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
) {
    let _ = collect_source_template_expression_aliases_with_stack_flow(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
        false,
    );
}

pub(crate) fn collect_source_template_expression_aliases_with_static_state(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    alias_scope: &mut SourceExpressionAliasScope,
) {
    let mut call_stack = BTreeSet::new();
    let _ = collect_source_template_expression_aliases_with_stack_flow(
        context,
        statement,
        values,
        body_cache,
        &mut call_stack,
        alias_scope,
        true,
    );
}

fn collect_source_template_expression_aliases_with_stack_flow(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
    sync_static_state: bool,
) -> SourceExpressionAliasFlow {
    if sync_static_state && statement.kind == FunctionStatementKind::Continue {
        return SourceExpressionAliasFlow::Continue;
    }
    if sync_static_state && statement.kind == FunctionStatementKind::Break {
        return SourceExpressionAliasFlow::Break;
    }
    if sync_static_state {
        apply_source_alias_collection_static_declaration(context.program, statement, values);
    }
    if let Some(flow) = collect_source_static_if_expression_aliases(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
        sync_static_state,
    ) {
        return flow;
    }
    if let Some(flow) = collect_source_static_for_expression_aliases(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
        sync_static_state,
    ) {
        return flow;
    }
    if let Some(flow) = collect_source_static_while_expression_aliases(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
        sync_static_state,
    ) {
        return flow;
    }
    if let Some(flow) = collect_source_static_switch_expression_aliases(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
        sync_static_state,
    ) {
        return flow;
    }
    if sync_static_state
        && apply_source_static_expression_statement(
            context.program,
            statement.value_expression.as_ref(),
            values,
        )
    {
        return SourceExpressionAliasFlow::Fallthrough;
    }
    if collect_source_expr_destructuring_aliases(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
    ) {
        return SourceExpressionAliasFlow::Fallthrough;
    }
    collect_source_returned_expression_alias_with_stack(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
    );
    let expression_aliases = alias_scope.expressions.clone();
    collect_source_template_expression_array_alias(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        &expression_aliases,
        alias_scope,
    );
    SourceExpressionAliasFlow::Fallthrough
}

fn apply_source_alias_collection_static_declaration(
    program: &SourceProgram,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration))
            if declaration.type_name.as_deref() == Some("expr") =>
        {
            false
        }
        Some(FunctionStatementDeclaration::Variable(declaration))
            if declaration.type_name == "expr" =>
        {
            false
        }
        _ => apply_source_static_declaration(program, statement, values),
    }
}

fn collect_source_static_if_expression_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
    sync_static_state: bool,
) -> Option<SourceExpressionAliasFlow> {
    if statement.kind != FunctionStatementKind::If {
        return None;
    }
    let Ok(Some(body_statements)) = source_static_if_body_statements_with_aliases(
        context.program,
        context.module,
        context.tokens,
        statement,
        values,
        &alias_scope.expressions,
        body_cache,
    ) else {
        return None;
    };
    for body_statement in body_statements.iter() {
        let flow = collect_source_template_expression_aliases_with_stack_flow(
            context,
            body_statement,
            values,
            body_cache,
            call_stack,
            alias_scope,
            sync_static_state,
        );
        if flow != SourceExpressionAliasFlow::Fallthrough {
            return Some(flow);
        }
    }
    Some(SourceExpressionAliasFlow::Fallthrough)
}

fn collect_source_static_for_expression_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
    sync_static_state: bool,
) -> Option<SourceExpressionAliasFlow> {
    if statement.kind != FunctionStatementKind::For {
        return None;
    }
    let Ok(Some(loop_info)) = source_static_for_loop_with_tokens(
        context.program,
        context.module,
        context.tokens,
        statement,
        values,
        body_cache,
    ) else {
        return None;
    };
    for iteration_value in &loop_info.iteration_values {
        values.insert(loop_info.variable_name.clone(), iteration_value.clone());
        for body_statement in loop_info.body_statements.iter() {
            let flow = collect_source_template_expression_aliases_with_stack_flow(
                context,
                body_statement,
                values,
                body_cache,
                call_stack,
                alias_scope,
                sync_static_state,
            );
            match flow {
                SourceExpressionAliasFlow::Fallthrough => {}
                SourceExpressionAliasFlow::Continue => break,
                SourceExpressionAliasFlow::Break => {
                    return Some(SourceExpressionAliasFlow::Fallthrough);
                }
            }
        }
    }
    loop_info.apply_final_variable_value(values);
    Some(SourceExpressionAliasFlow::Fallthrough)
}

fn collect_source_static_while_expression_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
    sync_static_state: bool,
) -> Option<SourceExpressionAliasFlow> {
    if statement.kind != FunctionStatementKind::While || !sync_static_state {
        return None;
    }
    let Ok(Some(loop_info)) = source_static_while_loop_with_tokens(
        context.program,
        context.module,
        context.tokens,
        statement,
        values,
        body_cache,
    ) else {
        return None;
    };
    for _ in 0..STATIC_WHILE_LOOP_LIMIT {
        let condition_value =
            evaluate_source_static_expression(context.program, &loop_info.condition, values)?;
        if !static_value_truthy(&condition_value) {
            return Some(SourceExpressionAliasFlow::Fallthrough);
        }
        for body_statement in loop_info.body_statements.iter() {
            let flow = collect_source_template_expression_aliases_with_stack_flow(
                context,
                body_statement,
                values,
                body_cache,
                call_stack,
                alias_scope,
                sync_static_state,
            );
            match flow {
                SourceExpressionAliasFlow::Fallthrough => {}
                SourceExpressionAliasFlow::Continue => break,
                SourceExpressionAliasFlow::Break => {
                    return Some(SourceExpressionAliasFlow::Fallthrough);
                }
            }
        }
    }
    None
}

fn collect_source_static_switch_expression_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
    sync_static_state: bool,
) -> Option<SourceExpressionAliasFlow> {
    if statement.kind != FunctionStatementKind::Switch {
        return None;
    }
    let Ok(Some(body_statements)) = source_static_switch_body_statements(
        context.program,
        context.module,
        context.tokens,
        statement,
        values,
        body_cache,
    ) else {
        return None;
    };
    for body_statement in body_statements.iter() {
        let flow = collect_source_template_expression_aliases_with_stack_flow(
            context,
            body_statement,
            values,
            body_cache,
            call_stack,
            alias_scope,
            sync_static_state,
        );
        match flow {
            SourceExpressionAliasFlow::Fallthrough => {}
            SourceExpressionAliasFlow::Break => {
                return Some(SourceExpressionAliasFlow::Fallthrough)
            }
            SourceExpressionAliasFlow::Continue => {
                return Some(SourceExpressionAliasFlow::Continue)
            }
        }
    }
    Some(SourceExpressionAliasFlow::Fallthrough)
}

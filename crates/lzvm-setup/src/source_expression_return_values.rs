use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    lex_source, CallArgument, Expression, ExpressionKind, FixedFileTemplateValue,
    FunctionDeclaration, FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind,
    SourceProgramModule, TokenKind,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_aliases::{
        collect_source_template_expression_alias, source_expression_alias_assignment_target,
    },
    source_expression_info::{
        collect_source_template_expression_array_alias, source_call_expression,
        source_expression_array_alias_assignment, source_function_call_bindings,
        SourceExpressionAliasScope,
    },
    source_expression_statements::{
        apply_source_static_declaration, apply_source_static_expression_statement,
    },
    source_statement_hints::{SourceExpressionArrayAlias, SourceExpressionArrayAliases},
    source_static_values::{
        evaluate_source_static_expression, source_static_array_length,
        source_static_array_length_key,
    },
    source_template_context::SourceTemplateLoweringContext,
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

pub(crate) fn insert_source_expr_array_alias_length(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    target_name: &str,
    alias: &SourceExpressionArrayAlias,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<()> {
    let length = source_expression_array_alias_length(values, alias, expression_array_aliases)?;
    let length = i128::try_from(length).ok()?;
    values.insert(
        source_static_array_length_key(target_name),
        FixedFileTemplateValue::Integer(length),
    );
    Some(())
}

fn source_expression_array_alias_length(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias: &SourceExpressionArrayAlias,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<usize> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => source_static_array_length(values, name)
            .and_then(|length| usize::try_from(length).ok())
            .or_else(|| {
                expression_array_aliases.get(name).and_then(|alias| {
                    source_expression_array_alias_length(values, alias, expression_array_aliases)
                })
            }),
        SourceExpressionArrayAlias::Values(expressions) => Some(expressions.len()),
    }
}

pub(crate) fn collect_source_template_expression_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    alias_scope: &mut SourceExpressionAliasScope,
) {
    let mut call_stack = BTreeSet::new();
    collect_source_template_expression_aliases_with_stack(
        context,
        statement,
        values,
        body_cache,
        &mut call_stack,
        alias_scope,
    );
}

pub(crate) fn collect_source_template_expression_aliases_with_stack(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
) {
    if collect_source_static_if_expression_aliases(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
    ) {
        return;
    }
    collect_source_returned_expression_alias_with_stack(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        alias_scope,
    );
    collect_source_template_expression_array_alias(
        context,
        statement,
        values,
        body_cache,
        &alias_scope.expressions,
        &mut alias_scope.expression_arrays,
    );
}

fn collect_source_static_if_expression_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
) -> bool {
    if statement.kind != FunctionStatementKind::If {
        return false;
    }
    let Ok(Some(body_statements)) = source_static_if_body_statements_with_tokens(
        context.program,
        context.module,
        context.tokens,
        statement,
        values,
        body_cache,
    ) else {
        return false;
    };
    for body_statement in body_statements.iter() {
        collect_source_template_expression_aliases_with_stack(
            context,
            body_statement,
            values,
            body_cache,
            call_stack,
            alias_scope,
        );
    }
    true
}

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
    if !collect_source_template_expression_alias(statement, &mut alias_scope.expressions) {
        return false;
    }
    if let Some(target) = assignment_target.as_ref() {
        values.remove(target);
    }
    let Some(name) = alias_name.or(assignment_target) else {
        return true;
    };
    let Some(expression) = alias_scope.expressions.get(&name).cloned() else {
        return true;
    };
    let mut changed = false;
    let mut resolving_aliases = BTreeSet::new();
    let Some(resolved) = source_resolve_expression(
        context,
        &expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        false,
        &mut resolving_aliases,
        &mut changed,
    ) else {
        return true;
    };
    if changed {
        alias_scope.expressions.insert(name, resolved);
    }
    true
}

fn source_returned_expression_alias(
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
        context.program,
        context.module,
        function,
        arguments,
        values,
        alias_scope,
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

fn source_returned_expression_body(
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
            let mut changed = false;
            let mut resolving_aliases = BTreeSet::new();
            return source_resolve_expression(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
                true,
                &mut resolving_aliases,
                &mut changed,
            );
        }
        if statement.kind == FunctionStatementKind::If {
            if let Ok(Some(body)) = source_static_if_body_statements_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
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
            }
            continue;
        }
        apply_source_static_declaration(context.program, statement, values);
        apply_source_static_expression_statement(
            context.program,
            statement.value_expression.as_ref(),
            values,
        );
        source_expression_array_alias_assignment(
            context.program,
            statement.value_expression.as_ref(),
            values,
            &mut alias_scope.expression_arrays,
        );
        collect_source_returned_expression_alias_with_stack(
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

pub(crate) fn source_resolved_expression_value(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
) -> Option<Expression> {
    let mut changed = false;
    let mut resolving_aliases = BTreeSet::new();
    source_resolve_expression(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        resolve_aliases,
        &mut resolving_aliases,
        &mut changed,
    )
}

#[allow(clippy::too_many_arguments)]
fn source_resolve_expression(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolving_aliases: &mut BTreeSet<String>,
    changed: &mut bool,
) -> Option<Expression> {
    let kind = match &expression.kind {
        ExpressionKind::Name(name) => {
            if resolve_aliases {
                if let Some(alias) = alias_scope.expressions.get(name) {
                    if !resolving_aliases.insert(name.clone()) {
                        return None;
                    }
                    let resolved = source_resolve_expression(
                        context,
                        alias,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                        resolve_aliases,
                        resolving_aliases,
                        changed,
                    )?;
                    resolving_aliases.remove(name);
                    *changed = true;
                    return Some(resolved);
                }
            }
            if let Some(expression) = source_static_value_expression(context, expression, values) {
                *changed = true;
                return Some(expression);
            }
            ExpressionKind::Name(name.clone())
        }
        ExpressionKind::Call { callee, args } => {
            if let Some(expression) = source_returned_expression_alias(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
            ) {
                *changed = true;
                return Some(expression);
            }
            ExpressionKind::Call {
                callee: Box::new(source_resolve_expression(
                    context,
                    callee,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    resolve_aliases,
                    resolving_aliases,
                    changed,
                )?),
                args: args
                    .iter()
                    .map(|arg| {
                        Some(CallArgument {
                            name: arg.name.clone(),
                            value: source_resolve_expression(
                                context,
                                &arg.value,
                                values,
                                alias_scope,
                                body_cache,
                                call_stack,
                                resolve_aliases,
                                resolving_aliases,
                                changed,
                            )?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?,
            }
        }
        ExpressionKind::Group(inner) => ExpressionKind::Group(Box::new(source_resolve_expression(
            context,
            inner,
            values,
            alias_scope,
            body_cache,
            call_stack,
            resolve_aliases,
            resolving_aliases,
            changed,
        )?)),
        ExpressionKind::Array(expressions) => ExpressionKind::Array(
            expressions
                .iter()
                .map(|expression| {
                    source_resolve_expression(
                        context,
                        expression,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                        resolve_aliases,
                        resolving_aliases,
                        changed,
                    )
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        ExpressionKind::Unary { op, expr } => ExpressionKind::Unary {
            op: *op,
            expr: Box::new(source_resolve_expression(
                context,
                expr,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolving_aliases,
                changed,
            )?),
        },
        ExpressionKind::Binary { op, left, right } => ExpressionKind::Binary {
            op: *op,
            left: Box::new(source_resolve_expression(
                context,
                left,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolving_aliases,
                changed,
            )?),
            right: Box::new(source_resolve_expression(
                context,
                right,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolving_aliases,
                changed,
            )?),
        },
        ExpressionKind::Index { target, index } => {
            let target = source_resolve_expression(
                context,
                target,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolving_aliases,
                changed,
            )?;
            let index = source_resolve_expression(
                context,
                index,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolving_aliases,
                changed,
            )?;
            let indexed = Expression {
                kind: ExpressionKind::Index {
                    target: Box::new(target),
                    index: Box::new(index),
                },
                source_name: expression.source_name.clone(),
                start: expression.start,
                end: expression.end,
            };
            if let Some(expression) =
                source_resolved_expression_array_element(&indexed, context, values, alias_scope)
            {
                *changed = true;
                return source_resolve_expression(
                    context,
                    &expression,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    resolve_aliases,
                    resolving_aliases,
                    changed,
                );
            }
            indexed.kind
        }
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => ExpressionKind::RowOffset {
            target: Box::new(source_resolve_expression(
                context,
                target,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolving_aliases,
                changed,
            )?),
            offset: Box::new(source_resolve_expression(
                context,
                offset,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolving_aliases,
                changed,
            )?),
            prior: *prior,
        },
        _ => expression.kind.clone(),
    };
    let resolved = Expression {
        kind,
        source_name: expression.source_name.clone(),
        start: expression.start,
        end: expression.end,
    };
    if let Some(expression) = source_static_value_expression(context, &resolved, values) {
        *changed = true;
        return Some(expression);
    }
    Some(resolved)
}

fn source_static_value_expression(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Expression> {
    let value = evaluate_source_static_expression(context.program, expression, values)?;
    let kind = match value {
        FixedFileTemplateValue::Integer(value) => ExpressionKind::Integer(value.to_string()),
        FixedFileTemplateValue::Boolean(value) => {
            ExpressionKind::Integer(if value { "1" } else { "0" }.to_owned())
        }
        FixedFileTemplateValue::String(value) => ExpressionKind::StringLiteral(value),
    };
    Some(Expression {
        kind,
        source_name: expression.source_name.clone(),
        start: expression.start,
        end: expression.end,
    })
}

fn source_resolved_expression_array_element(
    expression: &Expression,
    context: &SourceTemplateLoweringContext<'_>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
) -> Option<Expression> {
    let (name, index_expressions) = source_expression_index_chain(expression)?;
    let indices = index_expressions
        .iter()
        .map(|index| source_expression_index(context, index, values))
        .collect::<Option<Vec<_>>>()?;
    let alias = alias_scope.expression_arrays.get(name)?;
    source_expression_array_alias_element(alias, &indices, alias_scope)
}

fn source_expression_array_alias_element(
    alias: &SourceExpressionArrayAlias,
    indices: &[usize],
    alias_scope: &SourceExpressionAliasScope,
) -> Option<Expression> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => alias_scope
            .expression_arrays
            .get(name)
            .and_then(|alias| source_expression_array_alias_element(alias, indices, alias_scope)),
        SourceExpressionArrayAlias::Values(expressions) => {
            source_expression_array_element(expressions, indices, alias_scope)
        }
    }
}

fn source_expression_array_element(
    expressions: &[Expression],
    indices: &[usize],
    alias_scope: &SourceExpressionAliasScope,
) -> Option<Expression> {
    let (index, rest) = indices.split_first()?;
    let expression = expressions.get(*index)?;
    if rest.is_empty() {
        return Some(expression.clone());
    }
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Array(expressions) => {
            source_expression_array_element(expressions, rest, alias_scope)
        }
        ExpressionKind::Name(name) => {
            let alias = alias_scope.expression_arrays.get(name)?;
            source_expression_array_alias_element(alias, rest, alias_scope)
        }
        _ => None,
    }
}

fn source_expression_index(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let value = evaluate_source_static_expression(context.program, expression, values)?;
    usize::try_from(source_static_integer_value(&value)?).ok()
}

fn source_expression_index_chain(expression: &Expression) -> Option<(&str, Vec<&Expression>)> {
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some((name, Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (name, mut indices) = source_expression_index_chain(target)?;
            indices.push(index);
            Some((name, indices))
        }
        _ => None,
    }
}

fn source_strip_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => source_strip_group_expression(inner),
        _ => expression,
    }
}

fn source_static_integer_value(value: &FixedFileTemplateValue) -> Option<i128> {
    match value {
        FixedFileTemplateValue::Integer(value) => Some(*value),
        FixedFileTemplateValue::Boolean(value) => Some(if *value { 1 } else { 0 }),
        FixedFileTemplateValue::String(_) => None,
    }
}

fn source_statement_expression_alias_name(statement: &FunctionStatement) -> Option<String> {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration))
            if declaration.type_name.as_deref() == Some("expr")
                && declaration.array_dims.is_empty()
                && declaration.initializer_expression.is_some() =>
        {
            Some(declaration.name.clone())
        }
        Some(FunctionStatementDeclaration::Variable(declaration))
            if declaration.type_name == "expr"
                && declaration.array_dims.is_empty()
                && declaration.initializer_expression.is_some() =>
        {
            Some(declaration.name.clone())
        }
        _ => None,
    }
}

fn source_function_returns_expr(
    module: &SourceProgramModule,
    function: &FunctionDeclaration,
) -> bool {
    let Some(span) = function.return_type else {
        return false;
    };
    let Some(text) = module.source.contents.get(span.start..span.end) else {
        return false;
    };
    let Ok(tokens) = lex_source(text) else {
        return false;
    };
    let tokens = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::EndOfInput)
        .collect::<Vec<_>>();
    let type_index = usize::from(
        tokens
            .first()
            .is_some_and(|token| token.kind == TokenKind::Const),
    );
    tokens.len() == type_index + 1
        && tokens[type_index].kind == TokenKind::Expr
        && tokens[type_index].lexeme == "expr"
}

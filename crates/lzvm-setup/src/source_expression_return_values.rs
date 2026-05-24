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
        apply_source_static_declaration, apply_source_static_expression_statement,
        collect_source_template_expression_array_alias, source_call_expression,
        source_expression_array_alias_assignment, source_function_call_bindings,
        SourceExpressionAliasScope,
    },
    source_template_context::SourceTemplateLoweringContext,
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

pub(crate) fn collect_source_template_expression_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    alias_scope: &mut SourceExpressionAliasScope,
) {
    collect_source_returned_expression_alias(context, statement, values, body_cache, alias_scope);
    collect_source_template_expression_array_alias(
        context,
        statement,
        values,
        body_cache,
        &alias_scope.expressions,
        &mut alias_scope.expression_arrays,
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

pub(crate) fn collect_source_returned_expression_alias(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    alias_scope: &mut SourceExpressionAliasScope,
) -> bool {
    let mut call_stack = BTreeSet::new();
    collect_source_returned_expression_alias_with_stack(
        context,
        statement,
        values,
        body_cache,
        &mut call_stack,
        alias_scope,
    )
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
        ExpressionKind::Name(name) if resolve_aliases => {
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
        ExpressionKind::Index { target, index } => ExpressionKind::Index {
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
            index: Box::new(source_resolve_expression(
                context,
                index,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolving_aliases,
                changed,
            )?),
        },
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
    Some(Expression {
        kind,
        source_name: expression.source_name.clone(),
        start: expression.start,
        end: expression.end,
    })
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

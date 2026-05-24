use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    lex_source, CallArgument, Expression, ExpressionKind, FixedFileTemplateValue,
    FunctionDeclaration, FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind,
    SourceProgramModule, TokenKind,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_aliases::collect_source_template_expression_alias,
    source_expression_info::{
        apply_source_static_declaration, apply_source_static_expression_statement,
        source_call_expression, source_expression_array_alias,
        source_expression_array_alias_assignment, source_function_call_bindings,
        SourceExpressionAliasScope,
    },
    source_statement_hints::SourceExpressionArrayAlias,
    source_static_values::evaluate_source_static_expression,
    source_template_context::SourceTemplateLoweringContext,
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

pub(crate) fn source_returned_expression_array_alias(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<SourceExpressionArrayAlias> {
    let (name, arguments) = source_call_expression(Some(expression))?;
    let function = context
        .module
        .functions
        .iter()
        .find(|function| function.name == name)?;
    if !source_function_returns_expr_array(context.module, function) {
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
    let alias = source_returned_expression_array_body(
        context,
        &function.statements,
        &mut bindings.values,
        &mut body_alias_scope,
        body_cache,
        call_stack,
    );
    call_stack.remove(&function.name);
    alias
}

fn source_returned_expression_array_body(
    context: &SourceTemplateLoweringContext<'_>,
    statements: &[FunctionStatement],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<SourceExpressionArrayAlias> {
    for statement in statements {
        if statement.kind == FunctionStatementKind::Return {
            let expression = statement.value_expression.as_ref()?;
            return source_resolved_expression_array_alias(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
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
                if let Some(alias) = source_returned_expression_array_body(
                    context,
                    &body,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                ) {
                    return Some(alias);
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
                    if let Some(alias) = source_returned_expression_array_body(
                        context,
                        &loop_info.body_statements,
                        values,
                        alias_scope,
                        body_cache,
                        call_stack,
                    ) {
                        return Some(alias);
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
        collect_source_template_expression_alias(statement, &mut alias_scope.expressions);
        collect_returned_expression_array_alias(
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

fn collect_returned_expression_array_alias(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
) {
    let alias = match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration))
            if declaration.type_name.as_deref() == Some("expr")
                && !declaration.array_dims.is_empty() =>
        {
            source_returned_declaration_expression_array_alias(
                context,
                ReturnedExpressionArrayDeclaration {
                    name: &declaration.name,
                    dim_expressions: &declaration.array_dim_expressions,
                    initializer: declaration.initializer_expression.as_ref(),
                    source_name: &declaration.source_name,
                    start: declaration.start,
                },
                values,
                body_cache,
                call_stack,
                alias_scope,
            )
        }
        Some(FunctionStatementDeclaration::Variable(declaration))
            if declaration.type_name == "expr" && !declaration.array_dims.is_empty() =>
        {
            source_returned_declaration_expression_array_alias(
                context,
                ReturnedExpressionArrayDeclaration {
                    name: &declaration.name,
                    dim_expressions: &declaration.array_dim_expressions,
                    initializer: declaration.initializer_expression.as_ref(),
                    source_name: &declaration.source_name,
                    start: declaration.start,
                },
                values,
                body_cache,
                call_stack,
                alias_scope,
            )
        }
        _ => {
            source_expression_array_alias_assignment(
                context.program,
                statement.value_expression.as_ref(),
                values,
                &mut alias_scope.expression_arrays,
            );
            None
        }
    };
    if let Some((name, alias)) = alias {
        alias_scope.expression_arrays.insert(name, alias);
    }
}

struct ReturnedExpressionArrayDeclaration<'a> {
    name: &'a str,
    dim_expressions: &'a [Option<Expression>],
    initializer: Option<&'a Expression>,
    source_name: &'a str,
    start: usize,
}

fn source_returned_declaration_expression_array_alias(
    context: &SourceTemplateLoweringContext<'_>,
    declaration: ReturnedExpressionArrayDeclaration<'_>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &SourceExpressionAliasScope,
) -> Option<(String, SourceExpressionArrayAlias)> {
    let alias = if let Some(expression) = declaration.initializer {
        source_expression_array_alias(expression).or_else(|| {
            source_returned_expression_array_alias(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
            )
        })?
    } else {
        let lengths = declaration
            .dim_expressions
            .iter()
            .map(|expression| {
                let value = evaluate_source_static_expression(
                    context.program,
                    expression.as_ref()?,
                    values,
                )?;
                usize::try_from(source_static_integer_value(Some(&value))?).ok()
            })
            .collect::<Option<Vec<_>>>()?;
        SourceExpressionArrayAlias::Values(source_zero_expression_array(
            declaration.name,
            declaration.source_name,
            declaration.start,
            &lengths,
        )?)
    };
    Some((declaration.name.to_owned(), alias))
}

fn source_resolved_expression_array_alias(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<SourceExpressionArrayAlias> {
    if let Some(alias) = source_expression_array_alias(expression) {
        return source_resolve_expression_array_alias(&alias, alias_scope);
    }
    source_returned_expression_array_alias(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
    )
}

fn source_resolve_expression_array_alias(
    alias: &SourceExpressionArrayAlias,
    alias_scope: &SourceExpressionAliasScope,
) -> Option<SourceExpressionArrayAlias> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => alias_scope
            .expression_arrays
            .get(name)
            .map(|alias| source_resolve_expression_array_alias(alias, alias_scope))
            .unwrap_or_else(|| Some(SourceExpressionArrayAlias::Name(name.clone()))),
        SourceExpressionArrayAlias::Values(expressions) => expressions
            .iter()
            .map(|expression| source_resolve_expression_alias(expression, alias_scope))
            .collect::<Option<Vec<_>>>()
            .map(SourceExpressionArrayAlias::Values),
    }
}

fn source_resolve_expression_alias(
    expression: &Expression,
    alias_scope: &SourceExpressionAliasScope,
) -> Option<Expression> {
    let kind = match &expression.kind {
        ExpressionKind::Name(name) => {
            if let Some(alias) = alias_scope.expressions.get(name) {
                return source_resolve_expression_alias(alias, alias_scope);
            }
            ExpressionKind::Name(name.clone())
        }
        ExpressionKind::Group(inner) => ExpressionKind::Group(Box::new(
            source_resolve_expression_alias(inner, alias_scope)?,
        )),
        ExpressionKind::Array(expressions) => ExpressionKind::Array(
            expressions
                .iter()
                .map(|expression| source_resolve_expression_alias(expression, alias_scope))
                .collect::<Option<Vec<_>>>()?,
        ),
        ExpressionKind::Unary { op, expr } => ExpressionKind::Unary {
            op: *op,
            expr: Box::new(source_resolve_expression_alias(expr, alias_scope)?),
        },
        ExpressionKind::Binary { op, left, right } => ExpressionKind::Binary {
            op: *op,
            left: Box::new(source_resolve_expression_alias(left, alias_scope)?),
            right: Box::new(source_resolve_expression_alias(right, alias_scope)?),
        },
        ExpressionKind::Index { target, index } => ExpressionKind::Index {
            target: Box::new(source_resolve_expression_alias(target, alias_scope)?),
            index: Box::new(source_resolve_expression_alias(index, alias_scope)?),
        },
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => ExpressionKind::RowOffset {
            target: Box::new(source_resolve_expression_alias(target, alias_scope)?),
            offset: Box::new(source_resolve_expression_alias(offset, alias_scope)?),
            prior: *prior,
        },
        ExpressionKind::Call { callee, args } => ExpressionKind::Call {
            callee: Box::new(source_resolve_expression_alias(callee, alias_scope)?),
            args: args
                .iter()
                .map(|arg| {
                    Some(CallArgument {
                        name: arg.name.clone(),
                        value: source_resolve_expression_alias(&arg.value, alias_scope)?,
                    })
                })
                .collect::<Option<Vec<_>>>()?,
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

fn source_function_returns_expr_array(
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
    tokens.len() == type_index + 3
        && tokens[type_index].kind == TokenKind::Expr
        && tokens[type_index].lexeme == "expr"
        && tokens[type_index + 1].kind == TokenKind::LBracket
        && tokens[type_index + 2].kind == TokenKind::RBracket
}

fn source_zero_expression_array(
    name: &str,
    source_name: &str,
    start: usize,
    lengths: &[usize],
) -> Option<Vec<Expression>> {
    let (&length, rest) = lengths.split_first()?;
    Some(
        (0..length)
            .map(|_| {
                if rest.is_empty() {
                    source_zero_expression(source_name, start)
                } else {
                    Expression {
                        kind: ExpressionKind::Array(
                            source_zero_expression_array(name, source_name, start, rest)
                                .unwrap_or_default(),
                        ),
                        source_name: source_name.to_owned(),
                        start,
                        end: start.saturating_add(name.len()),
                    }
                }
            })
            .collect(),
    )
}

fn source_zero_expression(source_name: &str, start: usize) -> Expression {
    Expression {
        kind: ExpressionKind::Integer("0".to_owned()),
        source_name: source_name.to_owned(),
        start,
        end: start,
    }
}

fn source_static_integer_value(value: Option<&FixedFileTemplateValue>) -> Option<i128> {
    match value {
        Some(FixedFileTemplateValue::Integer(value)) => Some(*value),
        Some(FixedFileTemplateValue::Boolean(value)) => Some(if *value { 1 } else { 0 }),
        _ => None,
    }
}

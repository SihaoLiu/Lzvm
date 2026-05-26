#![allow(clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    lex_source, BinaryOperator, CallArgument, Expression, ExpressionKind, FixedFileTemplateValue,
    FunctionDeclaration, FunctionStatement, FunctionStatementDeclaration, SourceProgram,
    SourceProgramModule, TokenKind,
};

use crate::{
    source_control_body_cache::{
        SourceControlBodyCache, SourceReturnedArrayCallKey, SourceReturnedArrayElementKey,
    },
    source_expression_aliases::source_alias_binding_name,
    source_expression_info::{source_call_expression, SourceExpressionAliasScope},
    source_expression_return_arrays::{
        source_function_returns_expr_array, source_returned_expression_array_alias,
    },
    source_statement_hints::SourceExpressionArrayAlias,
    source_static_values::{evaluate_source_static_expression, SourceStaticValueLookup},
    source_template_context::SourceTemplateLoweringContext,
};

mod alias_bindings;
mod alias_collection;
mod array_lengths;
mod destructuring;
mod import_scope;
mod returned_body;
mod returned_calls;
#[cfg(test)]
mod tests;

pub(crate) use alias_bindings::collect_source_returned_expression_alias_with_stack;
pub(crate) use alias_collection::{
    collect_source_template_expression_aliases_with_stack,
    collect_source_template_expression_aliases_with_static_state,
};
pub(crate) use array_lengths::insert_source_expr_array_alias_length;
pub(crate) use destructuring::collect_source_expr_destructuring_aliases;
pub(crate) use returned_calls::source_import_returned_expression_calls;
use returned_calls::source_returned_expression_alias;

pub(crate) fn source_resolved_expression_value(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
) -> Option<Expression> {
    if !source_expression_may_resolve(
        context.program,
        context.module,
        expression,
        values,
        alias_scope,
        resolve_aliases,
        true,
    ) {
        return Some(expression.clone());
    }
    let mut changed = false;
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    source_resolve_expression(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        resolve_aliases,
        true,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
        &mut changed,
    )
}

pub(crate) fn source_resolved_expression_value_without_returned_calls(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<Expression> {
    if !source_expression_may_resolve(
        context.program,
        context.module,
        expression,
        values,
        alias_scope,
        true,
        false,
    ) {
        return Some(expression.clone());
    }
    let mut changed = false;
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    source_resolve_expression(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        true,
        false,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
        &mut changed,
    )
}

pub(crate) fn source_resolved_indexed_returned_expression_array_value(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<Expression> {
    let ExpressionKind::Index { target, index } = &source_strip_group_expression(expression).kind
    else {
        return None;
    };
    if !source_indexed_returned_array_call_may_resolve(context.module, target) {
        return None;
    }
    let mut changed = false;
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    source_resolve_indexed_call_array_expression(
        context,
        expression,
        target,
        index,
        values,
        alias_scope,
        body_cache,
        call_stack,
        true,
        true,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
        &mut changed,
    )
}

pub(crate) fn source_returned_expression_array_call_alias_cached(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<SourceExpressionArrayAlias> {
    let key = source_returned_array_call_key(expression, values);
    if !source_returned_array_call_cacheable(expression, alias_scope) {
        return source_returned_expression_array_alias(
            context,
            expression,
            values,
            alias_scope,
            body_cache,
            call_stack,
        );
    }
    match body_cache.returned_expression_array_alias(&key) {
        Some(alias) => alias,
        None => {
            let alias = source_returned_expression_array_alias(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
            );
            body_cache.insert_returned_expression_array_alias(key, alias.clone());
            alias
        }
    }
}

pub(crate) fn source_returned_array_call_cacheable(
    expression: &Expression,
    alias_scope: &SourceExpressionAliasScope,
) -> bool {
    let mut names = BTreeSet::new();
    collect_source_expression_names(expression, &mut names);
    names.into_iter().all(|name| {
        !alias_scope.expressions.contains_key(name.as_str())
            && source_expression_array_alias_lookup(alias_scope, name.as_str()).is_none()
    })
}

pub(crate) fn source_expression_may_resolve(
    program: &SourceProgram,
    module: &SourceProgramModule,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    resolve_aliases: bool,
    resolve_calls: bool,
) -> bool {
    if source_expression_static_value_can_apply(program, expression, values) {
        return true;
    }
    match &expression.kind {
        ExpressionKind::Name(name) => {
            resolve_aliases && alias_scope.expressions.contains_key(name.as_str())
        }
        ExpressionKind::Call { callee, args } => {
            source_get_l1_expression(expression).is_some()
                || (resolve_calls
                    && source_call_expression(Some(expression)).is_some_and(|(name, _)| {
                        module
                            .functions
                            .iter()
                            .find(|function| function.name == name)
                            .is_some_and(|function| source_function_returns_expr(module, function))
                    }))
                || source_expression_may_resolve(
                    program,
                    module,
                    callee,
                    values,
                    alias_scope,
                    resolve_aliases,
                    resolve_calls,
                )
                || args.iter().any(|arg| {
                    source_expression_may_resolve(
                        program,
                        module,
                        &arg.value,
                        values,
                        alias_scope,
                        resolve_aliases,
                        resolve_calls,
                    )
                })
        }
        ExpressionKind::Group(inner) => source_expression_may_resolve(
            program,
            module,
            inner,
            values,
            alias_scope,
            resolve_aliases,
            resolve_calls,
        ),
        ExpressionKind::Array(expressions) => expressions.iter().any(|expression| {
            source_expression_may_resolve(
                program,
                module,
                expression,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            )
        }),
        ExpressionKind::Unary { expr, .. } => source_expression_may_resolve(
            program,
            module,
            expr,
            values,
            alias_scope,
            resolve_aliases,
            resolve_calls,
        ),
        ExpressionKind::Binary { left, right, .. } => {
            source_expression_may_resolve(
                program,
                module,
                left,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            ) || source_expression_may_resolve(
                program,
                module,
                right,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            )
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            source_expression_may_resolve(
                program,
                module,
                condition,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            ) || source_expression_may_resolve(
                program,
                module,
                then_expr,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            ) || source_expression_may_resolve(
                program,
                module,
                else_expr,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            )
        }
        ExpressionKind::Index { target, index } => {
            source_expression_array_index_may_resolve(program, expression, values, alias_scope)
                || source_indexed_call_may_resolve(module, target, resolve_calls)
                || source_expression_may_resolve(
                    program,
                    module,
                    target,
                    values,
                    alias_scope,
                    resolve_aliases,
                    resolve_calls,
                )
                || source_expression_may_resolve(
                    program,
                    module,
                    index,
                    values,
                    alias_scope,
                    resolve_aliases,
                    resolve_calls,
                )
        }
        ExpressionKind::RowOffset { target, offset, .. } => {
            source_expression_may_resolve(
                program,
                module,
                target,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            ) || source_expression_may_resolve(
                program,
                module,
                offset,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            )
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => false,
    }
}

fn source_expression_array_index_may_resolve(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
) -> bool {
    source_expression_index_chain(expression).is_some_and(|(name, indices)| {
        source_expression_array_alias_lookup(alias_scope, name).is_some()
            && indices
                .iter()
                .all(|index| source_expression_static_index(program, index, values).is_some())
    })
}

fn source_expression_static_index(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    let value = evaluate_source_static_expression(program, expression, values)?;
    usize::try_from(source_static_integer_value(&value)?).ok()
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct SourceExpressionArrayResolutionKey {
    name: String,
    indices: Vec<usize>,
}

impl SourceExpressionArrayResolutionKey {
    fn new(name: &str, indices: &[usize]) -> Self {
        Self {
            name: name.to_owned(),
            indices: indices.to_vec(),
        }
    }
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
    resolve_calls: bool,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<SourceExpressionArrayResolutionKey>,
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
                        resolve_calls,
                        resolving_aliases,
                        resolving_array_aliases,
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
            if let Some(expression) = source_get_l1_expression(expression) {
                *changed = true;
                return Some(expression);
            }
            if resolve_calls {
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
                    resolve_calls,
                    resolving_aliases,
                    resolving_array_aliases,
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
                                resolve_calls,
                                resolving_aliases,
                                resolving_array_aliases,
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
            resolve_calls,
            resolving_aliases,
            resolving_array_aliases,
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
                        resolve_calls,
                        resolving_aliases,
                        resolving_array_aliases,
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
                resolve_calls,
                resolving_aliases,
                resolving_array_aliases,
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
                resolve_calls,
                resolving_aliases,
                resolving_array_aliases,
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
                resolve_calls,
                resolving_aliases,
                resolving_array_aliases,
                changed,
            )?),
        },
        ExpressionKind::Index { target, index } => {
            if source_indexed_call_may_resolve(context.module, target, resolve_calls) {
                if let Some(expression) = source_resolve_indexed_call_array_expression(
                    context,
                    expression,
                    target,
                    index,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    resolve_aliases,
                    resolve_calls,
                    resolving_aliases,
                    resolving_array_aliases,
                    changed,
                ) {
                    *changed = true;
                    return Some(expression);
                }
            }
            if let Some((name, indices, expression)) = source_resolved_expression_array_element(
                expression,
                context,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolve_calls,
                resolving_array_aliases,
            ) {
                *changed = true;
                return source_resolve_array_lookup_expression(
                    &name,
                    &indices,
                    expression,
                    context,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    resolve_aliases,
                    resolve_calls,
                    resolving_aliases,
                    resolving_array_aliases,
                    changed,
                );
            }
            let target = source_resolve_index_target_expression(
                context,
                target,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolve_calls,
                resolving_aliases,
                resolving_array_aliases,
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
                resolve_calls,
                resolving_aliases,
                resolving_array_aliases,
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
            if let Some((name, indices, expression)) = source_resolved_expression_array_element(
                &indexed,
                context,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolve_calls,
                resolving_array_aliases,
            ) {
                *changed = true;
                return source_resolve_array_lookup_expression(
                    &name,
                    &indices,
                    expression,
                    context,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    resolve_aliases,
                    resolve_calls,
                    resolving_aliases,
                    resolving_array_aliases,
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
                resolve_calls,
                resolving_aliases,
                resolving_array_aliases,
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
                resolve_calls,
                resolving_aliases,
                resolving_array_aliases,
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

#[allow(clippy::too_many_arguments)]
fn source_resolve_index_target_expression(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolve_calls: bool,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<SourceExpressionArrayResolutionKey>,
    changed: &mut bool,
) -> Option<Expression> {
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Index { target, index } => Some(Expression {
            kind: ExpressionKind::Index {
                target: Box::new(source_resolve_index_target_expression(
                    context,
                    target,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    resolve_aliases,
                    resolve_calls,
                    resolving_aliases,
                    resolving_array_aliases,
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
                    resolve_calls,
                    resolving_aliases,
                    resolving_array_aliases,
                    changed,
                )?),
            },
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        }),
        _ => source_resolve_expression(
            context,
            expression,
            values,
            alias_scope,
            body_cache,
            call_stack,
            resolve_aliases,
            resolve_calls,
            resolving_aliases,
            resolving_array_aliases,
            changed,
        ),
    }
}

fn source_indexed_call_may_resolve(
    module: &SourceProgramModule,
    target: &Expression,
    resolve_calls: bool,
) -> bool {
    resolve_calls
        && source_call_expression(Some(target)).is_some_and(|(name, _)| {
            module
                .functions
                .iter()
                .find(|function| function.name == name)
                .is_some_and(|function| source_function_returns_expr_array(module, function))
        })
}

fn source_indexed_returned_array_call_may_resolve(
    module: &SourceProgramModule,
    target: &Expression,
) -> bool {
    source_call_expression(Some(target)).is_some_and(|(name, _)| {
        module
            .functions
            .iter()
            .find(|function| function.name == name)
            .is_some_and(|function| {
                source_function_returns_expr_array(module, function)
                    || source_function_returns_expr(module, function)
            })
    })
}

#[allow(clippy::too_many_arguments)]
fn source_resolve_indexed_call_array_expression(
    context: &SourceTemplateLoweringContext<'_>,
    source_expression: &Expression,
    target: &Expression,
    index: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolve_calls: bool,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<SourceExpressionArrayResolutionKey>,
    changed: &mut bool,
) -> Option<Expression> {
    let index = source_expression_index(context, index, values)?;
    let key = source_returned_array_call_key(target, values);
    let element_key = SourceReturnedArrayElementKey::new(key.clone(), vec![index]);
    let use_element_cache = resolve_aliases
        && resolve_calls
        && source_returned_array_call_cacheable(target, alias_scope);
    if use_element_cache {
        if let Some(expression) = body_cache.returned_expression_array_element(&element_key) {
            return expression;
        }
    }
    let alias = source_returned_expression_array_call_alias_cached(
        context,
        target,
        values,
        alias_scope,
        body_cache,
        call_stack,
    )?;
    let indices = [index];
    let element_resolve_aliases = true;
    let element = source_expression_array_alias_element(
        context,
        &alias,
        &indices,
        values,
        alias_scope,
        body_cache,
        call_stack,
        element_resolve_aliases,
        resolve_calls,
        source_expression,
    )?;
    let resolved = source_expression_array_element_expression(
        context,
        element,
        &indices,
        values,
        body_cache,
        call_stack,
        element_resolve_aliases,
        resolve_calls,
        resolving_array_aliases,
        &source_returned_array_call_name(target),
    )
    .and_then(|expression| {
        source_resolve_expression(
            context,
            &expression,
            values,
            alias_scope,
            body_cache,
            call_stack,
            resolve_aliases,
            resolve_calls,
            resolving_aliases,
            resolving_array_aliases,
            changed,
        )
    });
    if use_element_cache {
        body_cache.insert_returned_expression_array_element(element_key, resolved.clone());
    }
    resolved
}

pub(crate) fn source_returned_array_call_key(
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> SourceReturnedArrayCallKey {
    let mut names = BTreeSet::new();
    collect_source_expression_names(expression, &mut names);
    let static_values = names
        .into_iter()
        .filter_map(|name| {
            values
                .get(&name)
                .map(|value| (name, source_static_value_cache_key(value)))
        })
        .collect();
    SourceReturnedArrayCallKey::new(
        expression.source_name.clone(),
        expression.start,
        expression.end,
        static_values,
    )
}

fn source_returned_array_call_name(expression: &Expression) -> String {
    format!(
        "__lzvm_returned_array_call_{}_{}",
        expression.start, expression.end
    )
}

fn source_static_value_cache_key(value: &FixedFileTemplateValue) -> String {
    match value {
        FixedFileTemplateValue::Integer(value) => format!("i:{value}"),
        FixedFileTemplateValue::Boolean(value) => format!("b:{value}"),
        FixedFileTemplateValue::String(value) => format!("s:{value}"),
    }
}

fn collect_source_expression_names(expression: &Expression, names: &mut BTreeSet<String>) {
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => {
            names.insert(name.clone());
        }
        ExpressionKind::Group(inner) => collect_source_expression_names(inner, names),
        ExpressionKind::Array(expressions) => {
            for expression in expressions {
                collect_source_expression_names(expression, names);
            }
        }
        ExpressionKind::Unary { expr, .. } => collect_source_expression_names(expr, names),
        ExpressionKind::Binary { left, right, .. } => {
            collect_source_expression_names(left, names);
            collect_source_expression_names(right, names);
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            collect_source_expression_names(condition, names);
            collect_source_expression_names(then_expr, names);
            collect_source_expression_names(else_expr, names);
        }
        ExpressionKind::Call { callee, args } => {
            collect_source_expression_names(callee, names);
            for arg in args {
                collect_source_expression_names(&arg.value, names);
            }
        }
        ExpressionKind::Index { target, index } => {
            collect_source_expression_names(target, names);
            collect_source_expression_names(index, names);
        }
        ExpressionKind::RowOffset { target, offset, .. } => {
            collect_source_expression_names(target, names);
            collect_source_expression_names(offset, names);
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => {}
    }
}

fn source_get_l1_expression(expression: &Expression) -> Option<Expression> {
    let (name, arguments) = source_call_expression(Some(expression))?;
    if name != "get_L1" || !arguments.is_empty() {
        return None;
    }
    Some(Expression {
        kind: ExpressionKind::Name("air.__L1__".to_owned()),
        source_name: expression.source_name.clone(),
        start: expression.start,
        end: expression.end,
    })
}

#[allow(clippy::too_many_arguments)]
fn source_resolve_array_lookup_expression(
    name: &str,
    indices: &[usize],
    expression: Expression,
    context: &SourceTemplateLoweringContext<'_>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolve_calls: bool,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<SourceExpressionArrayResolutionKey>,
    changed: &mut bool,
) -> Option<Expression> {
    if !source_expression_may_resolve(
        context.program,
        context.module,
        &expression,
        values,
        alias_scope,
        resolve_aliases,
        resolve_calls,
    ) {
        return Some(expression);
    }
    let key = SourceExpressionArrayResolutionKey::new(name, indices);
    if !resolving_array_aliases.insert(key.clone()) {
        return Some(expression);
    }
    let resolved = source_resolve_expression(
        context,
        &expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        resolve_aliases,
        resolve_calls,
        resolving_aliases,
        resolving_array_aliases,
        changed,
    );
    resolving_array_aliases.remove(&key);
    resolved
}

fn source_static_value_expression(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Expression> {
    if !source_expression_static_value_can_apply(context.program, expression, values) {
        return None;
    }
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

fn source_expression_static_value_can_apply(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_) => true,
        ExpressionKind::Name(name) => values.source_static_value(name).is_some(),
        ExpressionKind::Group(inner) => {
            source_expression_static_value_can_apply(program, inner, values)
        }
        ExpressionKind::Unary { expr, .. } => {
            source_expression_static_value_can_apply(program, expr, values)
        }
        ExpressionKind::Binary { op, left, right } => match op {
            BinaryOperator::LogicalAnd | BinaryOperator::LogicalOr => {
                source_expression_static_value_can_apply(program, left, values)
            }
            _ => {
                source_expression_static_value_can_apply(program, left, values)
                    && source_expression_static_value_can_apply(program, right, values)
            }
        },
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            source_expression_static_value_can_apply(program, condition, values)
                && (source_expression_static_value_can_apply(program, then_expr, values)
                    || source_expression_static_value_can_apply(program, else_expr, values))
        }
        ExpressionKind::Call { .. } => true,
        ExpressionKind::Index { target, index } => {
            let Some(name) = source_expression_static_index_target_name(program, target, values)
            else {
                return false;
            };
            let Some(index) = source_expression_static_index(program, index, values) else {
                return false;
            };
            values.source_static_array_element(&name, index).is_some()
        }
        ExpressionKind::Array(_)
        | ExpressionKind::RowOffset { .. }
        | ExpressionKind::PositionalParam(_) => false,
    }
}

fn source_expression_static_index_target_name(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<String> {
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(name.clone()),
        ExpressionKind::Index { target, index } => {
            let name = source_expression_static_index_target_name(program, target, values)?;
            let index = source_expression_static_index(program, index, values)?;
            Some(format!("{name}[{index}]"))
        }
        _ => None,
    }
}

fn source_resolved_expression_array_element(
    expression: &Expression,
    context: &SourceTemplateLoweringContext<'_>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolve_calls: bool,
    resolving_array_aliases: &mut BTreeSet<SourceExpressionArrayResolutionKey>,
) -> Option<(String, Vec<usize>, Expression)> {
    let (name, index_expressions) = source_expression_index_chain(expression)?;
    let indices = index_expressions
        .iter()
        .map(|index| source_expression_index(context, index, values))
        .collect::<Option<Vec<_>>>()?;
    let key = SourceExpressionArrayResolutionKey::new(name, &indices);
    if resolving_array_aliases.contains(&key) {
        return None;
    }
    let alias = source_expression_array_alias_lookup(alias_scope, name)?;
    let element = source_expression_array_alias_element(
        context,
        alias,
        &indices,
        values,
        alias_scope,
        body_cache,
        call_stack,
        resolve_aliases,
        resolve_calls,
        expression,
    )?;
    let expression = source_expression_array_element_expression(
        context,
        element,
        &indices,
        values,
        body_cache,
        call_stack,
        resolve_aliases,
        resolve_calls,
        resolving_array_aliases,
        name,
    )?;
    Some((name.to_owned(), indices, expression))
}

enum SourceResolvedExpressionArrayElement<'a> {
    Borrowed {
        expression: &'a Expression,
        alias_scope: &'a SourceExpressionAliasScope,
        resolve_aliases: bool,
    },
    Owned(Expression),
}

#[allow(clippy::too_many_arguments)]
fn source_expression_array_element_expression(
    context: &SourceTemplateLoweringContext<'_>,
    element: SourceResolvedExpressionArrayElement<'_>,
    indices: &[usize],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolve_calls: bool,
    resolving_array_aliases: &mut BTreeSet<SourceExpressionArrayResolutionKey>,
    array_name: &str,
) -> Option<Expression> {
    match element {
        SourceResolvedExpressionArrayElement::Owned(expression) => Some(expression),
        SourceResolvedExpressionArrayElement::Borrowed {
            expression,
            alias_scope,
            resolve_aliases: element_resolve_aliases,
        } => {
            let resolve_aliases = resolve_aliases || element_resolve_aliases;
            if !source_expression_may_resolve(
                context.program,
                context.module,
                expression,
                values,
                alias_scope,
                resolve_aliases,
                resolve_calls,
            ) {
                return Some(expression.clone());
            }
            let mut changed = false;
            let mut resolving_aliases = BTreeSet::new();
            let key = SourceExpressionArrayResolutionKey::new(array_name, indices);
            if !resolving_array_aliases.insert(key.clone()) {
                return Some(expression.clone());
            }
            let resolved = source_resolve_expression(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                call_stack,
                resolve_aliases,
                resolve_calls,
                &mut resolving_aliases,
                resolving_array_aliases,
                &mut changed,
            );
            resolving_array_aliases.remove(&key);
            resolved
        }
    }
}

fn source_expression_array_alias_element<'a>(
    context: &SourceTemplateLoweringContext<'_>,
    alias: &'a SourceExpressionArrayAlias,
    indices: &[usize],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &'a SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolve_calls: bool,
    source_expression: &Expression,
) -> Option<SourceResolvedExpressionArrayElement<'a>> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if let Some(alias) = source_expression_array_alias_lookup(alias_scope, name) {
                source_expression_array_alias_element(
                    context,
                    alias,
                    indices,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    resolve_aliases,
                    resolve_calls,
                    source_expression,
                )
            } else {
                source_named_array_element(name, indices, source_expression)
                    .map(SourceResolvedExpressionArrayElement::Owned)
            }
        }
        SourceExpressionArrayAlias::Values(expressions) => source_expression_array_element(
            context,
            expressions,
            indices,
            values,
            alias_scope,
            body_cache,
            call_stack,
            resolve_aliases,
            resolve_calls,
            source_expression,
        ),
        SourceExpressionArrayAlias::ScopedValues { expressions, scope } => {
            source_expression_array_element(
                context,
                expressions,
                indices,
                values,
                scope.as_ref(),
                body_cache,
                call_stack,
                true,
                resolve_calls,
                source_expression,
            )
        }
        SourceExpressionArrayAlias::Call { expression, .. } => {
            let indexed =
                source_indexed_expression(expression.as_ref().clone(), indices, source_expression)?;
            if !resolve_calls {
                return Some(SourceResolvedExpressionArrayElement::Owned(indexed));
            }
            source_resolved_indexed_returned_expression_array_value(
                context,
                &indexed,
                values,
                alias_scope,
                body_cache,
                call_stack,
            )
            .map(SourceResolvedExpressionArrayElement::Owned)
        }
    }
}

fn source_expression_array_element<'a>(
    context: &SourceTemplateLoweringContext<'_>,
    expressions: &'a [Expression],
    indices: &[usize],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &'a SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
    resolve_calls: bool,
    source_expression: &Expression,
) -> Option<SourceResolvedExpressionArrayElement<'a>> {
    let (index, rest) = indices.split_first()?;
    let expression = expressions.get(*index)?;
    if rest.is_empty() {
        return Some(SourceResolvedExpressionArrayElement::Borrowed {
            expression,
            alias_scope,
            resolve_aliases,
        });
    }
    match &source_strip_group_expression(expression).kind {
        ExpressionKind::Array(expressions) => source_expression_array_element(
            context,
            expressions,
            rest,
            values,
            alias_scope,
            body_cache,
            call_stack,
            resolve_aliases,
            resolve_calls,
            source_expression,
        ),
        ExpressionKind::Name(name) => {
            if let Some(alias) = source_expression_array_alias_lookup(alias_scope, name) {
                source_expression_array_alias_element(
                    context,
                    alias,
                    rest,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    resolve_aliases,
                    resolve_calls,
                    source_expression,
                )
            } else {
                source_named_array_element(name, rest, source_expression)
                    .map(SourceResolvedExpressionArrayElement::Owned)
            }
        }
        _ => None,
    }
}

fn source_expression_array_alias_lookup<'a>(
    alias_scope: &'a SourceExpressionAliasScope,
    name: &str,
) -> Option<&'a SourceExpressionArrayAlias> {
    alias_scope.expression_arrays.get(name).or_else(|| {
        let binding_name = source_alias_binding_name(name);
        (binding_name != name)
            .then(|| alias_scope.expression_arrays.get(binding_name))
            .flatten()
    })
}

fn source_indexed_expression_scoped_array_scope<'a>(
    expression: &Expression,
    alias_scope: &'a SourceExpressionAliasScope,
) -> Option<&'a SourceExpressionAliasScope> {
    let (name, _) = source_expression_index_chain(expression)?;
    let alias = source_expression_array_alias_lookup(alias_scope, name)?;
    source_expression_array_alias_scope(alias, alias_scope)
}

fn source_expression_array_alias_scope<'a>(
    alias: &'a SourceExpressionArrayAlias,
    alias_scope: &'a SourceExpressionAliasScope,
) -> Option<&'a SourceExpressionAliasScope> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            let alias = source_expression_array_alias_lookup(alias_scope, name)?;
            source_expression_array_alias_scope(alias, alias_scope)
        }
        SourceExpressionArrayAlias::ScopedValues { scope, .. } => Some(scope.as_ref()),
        SourceExpressionArrayAlias::Values(_) | SourceExpressionArrayAlias::Call { .. } => None,
    }
}

fn source_named_array_element(
    name: &str,
    indices: &[usize],
    source_expression: &Expression,
) -> Option<Expression> {
    source_indexed_expression(
        Expression {
            kind: ExpressionKind::Name(name.to_owned()),
            source_name: source_expression.source_name.clone(),
            start: source_expression.start,
            end: source_expression.end,
        },
        indices,
        source_expression,
    )
}

fn source_indexed_expression(
    target: Expression,
    indices: &[usize],
    source_expression: &Expression,
) -> Option<Expression> {
    indices.iter().try_fold(target, |target, index| {
        Some(Expression {
            kind: ExpressionKind::Index {
                target: Box::new(target),
                index: Box::new(Expression {
                    kind: ExpressionKind::Integer(index.to_string()),
                    source_name: source_expression.source_name.clone(),
                    start: source_expression.start,
                    end: source_expression.end,
                }),
            },
            source_name: source_expression.source_name.clone(),
            start: source_expression.start,
            end: source_expression.end,
        })
    })
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

pub(crate) fn source_function_returns_expr(
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

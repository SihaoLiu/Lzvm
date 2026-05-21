use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    CallArgument, Expression, ExpressionKind, FixedFileTemplateValue, FunctionDeclaration,
    FunctionParameter, FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind,
    SourceProgram,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_aliases::collect_source_template_expression_alias,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{
        evaluate_source_static_expression, insert_source_static_array,
        source_static_array_expression,
    },
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

use super::{
    lower_top_level_global_constraint, source_global_expression_array_alias,
    SourceGlobalAliasScope, SourceGlobalConstraintBuilder, SourceGlobalExpressionAliases,
    SourceGlobalExpressionArrayAlias, SourceGlobalExpressionArrayAliases,
    SourceTopLevelGlobalConstraintContext,
};

pub(super) fn lower_top_level_function_call(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    expression: &Expression,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some((name, arguments)) = source_call_expression(expression) else {
        return Ok(false);
    };
    let Some(function) = context
        .module
        .functions
        .iter()
        .find(|function| function.name == name)
    else {
        return Ok(false);
    };
    if function.return_type.is_some() {
        return Ok(false);
    }
    let Some(mut bindings) = source_function_call_bindings(
        context.program,
        function,
        arguments,
        &context.alias_scope.static_values,
        context.alias_scope,
    ) else {
        return Ok(false);
    };

    let checkpoint = constraints.checkpoint();
    let mut body_alias_scope = bindings.alias_scope;
    let mut body_cache = SourceControlBodyCache::default();
    for statement in &function.statements {
        body_alias_scope.static_values = bindings.values.clone();
        if !lower_function_body_statement(
            context,
            statement,
            &mut bindings.values,
            &body_alias_scope,
            &mut body_cache,
            constraints,
        )? {
            constraints.rollback(checkpoint);
            return Ok(false);
        }
        body_alias_scope.static_values = bindings.values.clone();
        collect_source_template_expression_alias(statement, &mut body_alias_scope.expressions);
        collect_source_global_expression_array_alias(
            statement,
            &mut body_alias_scope.expression_arrays,
        );
    }
    Ok(true)
}

struct SourceFunctionCallBindings<'a> {
    values: BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: SourceGlobalAliasScope<'a>,
}

fn source_function_call_bindings<'a>(
    program: &'a SourceProgram,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalAliasScope<'a>,
) -> Option<SourceFunctionCallBindings<'a>> {
    let mut function_values = values.clone();
    let mut function_alias_scope = SourceGlobalAliasScope {
        program: alias_scope.program,
        expressions: alias_scope.expressions.clone(),
        expression_arrays: alias_scope.expression_arrays.clone(),
        static_values: alias_scope.static_values.clone(),
    };
    let mut provided = BTreeSet::new();
    let mut positional_index = 0;

    for argument in arguments {
        let parameter = if let Some(name) = argument.name.as_ref() {
            function
                .parameters
                .iter()
                .find(|parameter| parameter.name == *name)?
        } else {
            while function
                .parameters
                .get(positional_index)
                .is_some_and(|parameter| provided.contains(&parameter.name))
            {
                positional_index = positional_index.checked_add(1)?;
            }
            function.parameters.get(positional_index)?
        };
        if !provided.insert(parameter.name.clone()) {
            return None;
        }
        bind_source_function_argument(
            program,
            parameter,
            &argument.value,
            &mut function_values,
            &mut function_alias_scope.expressions,
            &mut function_alias_scope.expression_arrays,
        )?;
        if argument.name.is_none() {
            positional_index = positional_index.checked_add(1)?;
        }
    }

    for parameter in &function.parameters {
        if provided.contains(&parameter.name) {
            continue;
        }
        bind_source_function_default(
            program,
            parameter,
            &mut function_values,
            &mut function_alias_scope.expressions,
            &mut function_alias_scope.expression_arrays,
        )?;
    }

    function_alias_scope.static_values = function_values.clone();
    Some(SourceFunctionCallBindings {
        values: function_values,
        alias_scope: function_alias_scope,
    })
}

fn bind_source_function_argument(
    program: &SourceProgram,
    parameter: &FunctionParameter,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &mut SourceGlobalExpressionAliases,
    expression_array_aliases: &mut SourceGlobalExpressionArrayAliases,
) -> Option<()> {
    if source_expr_parameter(parameter) {
        if expression_name(expression) == Some(parameter.name.as_str()) {
            return Some(());
        }
        expression_aliases.insert(parameter.name.clone(), expression.clone());
        return Some(());
    }
    if source_expr_array_parameter(parameter) {
        insert_source_expr_array_static_values(program, expression, values, &parameter.name)?;
        let alias = source_global_expression_array_alias(expression)?;
        if matches!(&alias, SourceGlobalExpressionArrayAlias::Name(name) if name == &parameter.name)
        {
            return Some(());
        }
        expression_array_aliases.insert(parameter.name.clone(), alias);
        return Some(());
    }
    if source_const_parameter(parameter) && parameter.array_dims.is_empty() {
        let value = evaluate_source_static_expression(program, expression, values)?;
        values.insert(parameter.name.clone(), value);
        return Some(());
    }
    if !source_const_parameter(parameter) {
        return None;
    }
    let elements = source_static_array_expression(program, expression, values)?;
    insert_source_static_array(values, &parameter.name, elements)
}

fn bind_source_function_default(
    program: &SourceProgram,
    parameter: &FunctionParameter,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &mut SourceGlobalExpressionAliases,
    expression_array_aliases: &mut SourceGlobalExpressionArrayAliases,
) -> Option<()> {
    let expression = parameter.default_expression.as_ref()?;
    if source_expr_parameter(parameter) {
        expression_aliases.insert(parameter.name.clone(), expression.clone());
        return Some(());
    }
    if source_expr_array_parameter(parameter) {
        insert_source_expr_array_static_values(program, expression, values, &parameter.name)?;
        let alias = source_global_expression_array_alias(expression)?;
        expression_array_aliases.insert(parameter.name.clone(), alias);
        return Some(());
    }
    if source_const_parameter(parameter) && parameter.array_dims.is_empty() {
        let value = evaluate_source_static_expression(program, expression, values)?;
        values.insert(parameter.name.clone(), value);
        return Some(());
    }
    if !source_const_parameter(parameter) {
        return None;
    }
    let elements = source_static_array_expression(program, expression, values)?;
    insert_source_static_array(values, &parameter.name, elements)
}

fn lower_function_body_statement(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalAliasScope<'_>,
    body_cache: &mut SourceControlBodyCache,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        return Ok(apply_static_declaration(context.program, statement, values)
            || source_expr_alias_declaration(statement)
            || source_expr_array_alias_declaration(statement));
    }
    if statement.kind == FunctionStatementKind::If {
        return match source_static_if_body_statements_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                let mut body_alias_scope = clone_alias_scope(alias_scope);
                for body_statement in body_statements.iter() {
                    body_alias_scope.static_values = values.clone();
                    if !lower_function_body_statement(
                        context,
                        body_statement,
                        values,
                        &body_alias_scope,
                        body_cache,
                        constraints,
                    )? {
                        return Ok(false);
                    }
                    body_alias_scope.static_values = values.clone();
                    collect_source_template_expression_alias(
                        body_statement,
                        &mut body_alias_scope.expressions,
                    );
                    collect_source_global_expression_array_alias(
                        body_statement,
                        &mut body_alias_scope.expression_arrays,
                    );
                }
                Ok(true)
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                Ok(false)
            }
            Err(error) => Err(error),
        };
    }
    if statement.kind == FunctionStatementKind::For {
        return match source_static_for_loop_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                let variable_name = loop_info.variable_name.clone();
                let previous = values.get(&variable_name).cloned();
                for iteration_value in &loop_info.iteration_values {
                    values.insert(variable_name.clone(), iteration_value.clone());
                    let mut loop_alias_scope = clone_alias_scope(alias_scope);
                    for body_statement in loop_info.body_statements.iter() {
                        loop_alias_scope.static_values = values.clone();
                        if !lower_function_body_statement(
                            context,
                            body_statement,
                            values,
                            &loop_alias_scope,
                            body_cache,
                            constraints,
                        )? {
                            restore_static_value(values, &variable_name, previous.as_ref());
                            return Ok(false);
                        }
                        loop_alias_scope.static_values = values.clone();
                        collect_source_template_expression_alias(
                            body_statement,
                            &mut loop_alias_scope.expressions,
                        );
                        collect_source_global_expression_array_alias(
                            body_statement,
                            &mut loop_alias_scope.expression_arrays,
                        );
                    }
                }
                restore_static_value(values, &variable_name, previous.as_ref());
                Ok(true)
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                Ok(false)
            }
            Err(error) => Err(error),
        };
    }
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(false);
    }
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(false);
    };
    let source_line = &context.module.source.contents[expression.start..expression.end];
    match lower_top_level_global_constraint(
        expression,
        source_line,
        context.slots,
        alias_scope,
        constraints,
    ) {
        Ok(()) => Ok(true),
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

fn restore_static_value(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
    value: Option<&FixedFileTemplateValue>,
) {
    if let Some(value) = value {
        values.insert(name.to_owned(), value.clone());
    } else {
        values.remove(name);
    }
}

fn insert_source_expr_array_static_values(
    program: &SourceProgram,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    target_name: &str,
) -> Option<()> {
    if let Some(elements) = source_static_array_expression(program, expression, values) {
        return insert_source_static_array(values, target_name, elements);
    }
    Some(())
}

fn apply_static_declaration(
    program: &SourceProgram,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(FunctionStatementDeclaration::Constant(declaration)) = statement.declaration.as_ref()
    else {
        return false;
    };
    if declaration.type_name.as_deref() == Some("expr") || !declaration.array_dims.is_empty() {
        return false;
    }
    let Some(expression) = declaration.initializer_expression.as_ref() else {
        return false;
    };
    let Some(value) = evaluate_source_static_expression(program, expression, values) else {
        return false;
    };
    values.insert(declaration.name.clone(), value);
    true
}

fn clone_alias_scope<'a>(alias_scope: &SourceGlobalAliasScope<'a>) -> SourceGlobalAliasScope<'a> {
    SourceGlobalAliasScope {
        program: alias_scope.program,
        expressions: alias_scope.expressions.clone(),
        expression_arrays: alias_scope.expression_arrays.clone(),
        static_values: alias_scope.static_values.clone(),
    }
}

fn source_expr_alias_declaration(statement: &FunctionStatement) -> bool {
    let Some(FunctionStatementDeclaration::Constant(declaration)) = statement.declaration.as_ref()
    else {
        return false;
    };
    declaration.type_name.as_deref() == Some("expr")
        && declaration.array_dims.is_empty()
        && declaration.initializer_expression.is_some()
}

fn source_expr_array_alias_declaration(statement: &FunctionStatement) -> bool {
    let Some(FunctionStatementDeclaration::Constant(declaration)) = statement.declaration.as_ref()
    else {
        return false;
    };
    declaration.type_name.as_deref() == Some("expr")
        && !declaration.array_dims.is_empty()
        && declaration.initializer_expression.is_some()
}

fn collect_source_global_expression_array_alias(
    statement: &FunctionStatement,
    expression_array_aliases: &mut SourceGlobalExpressionArrayAliases,
) {
    let Some(FunctionStatementDeclaration::Constant(declaration)) = statement.declaration.as_ref()
    else {
        return;
    };
    if declaration.type_name.as_deref() != Some("expr") || declaration.array_dims.is_empty() {
        return;
    }
    let Some(expression) = declaration.initializer_expression.as_ref() else {
        return;
    };
    if let Some(alias) = source_global_expression_array_alias(expression) {
        expression_array_aliases.insert(declaration.name.clone(), alias);
    }
}

fn source_expr_parameter(parameter: &FunctionParameter) -> bool {
    !parameter.by_reference && parameter.array_dims.is_empty() && parameter.type_name == "expr"
}

fn source_expr_array_parameter(parameter: &FunctionParameter) -> bool {
    !parameter.by_reference && !parameter.array_dims.is_empty() && parameter.type_name == "expr"
}

fn source_const_parameter(parameter: &FunctionParameter) -> bool {
    parameter.is_const && !parameter.by_reference
}

fn source_call_expression(expression: &Expression) -> Option<(&str, &[CallArgument])> {
    let ExpressionKind::Call { callee, args } = &strip_group_expression(expression).kind else {
        return None;
    };
    let ExpressionKind::Name(name) = &strip_group_expression(callee).kind else {
        return None;
    };
    Some((name.as_str(), args.as_slice()))
}

fn expression_name(expression: &Expression) -> Option<&str> {
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

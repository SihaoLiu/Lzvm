use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    BinaryOperator, CallArgument, Expression, ExpressionKind, FixedFileTemplateValue,
    FunctionDeclaration, FunctionParameter, FunctionStatement, FunctionStatementDeclaration,
    FunctionStatementKind, SourceProgram, UnaryOperator,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_aliases::collect_source_template_expression_alias,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_statement_hints::{source_statement_is_source_directive, source_statement_line},
    source_static_values::{
        evaluate_source_static_expression, insert_source_static_array,
        source_static_array_expression, source_static_array_length,
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
    let mut body_cache = SourceControlBodyCache::default();
    let mut call_stack = BTreeSet::new();
    lower_function_call_expression(
        context,
        expression,
        &context.alias_scope.static_values,
        context.alias_scope,
        &mut body_cache,
        &mut call_stack,
        constraints,
    )
}

fn lower_function_call_expression(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalAliasScope<'_>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
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
    let Some(mut bindings) =
        source_function_call_bindings(context.program, function, arguments, values, alias_scope)
    else {
        return Ok(false);
    };

    let function_name = function.name.clone();
    if !call_stack.insert(function_name.clone()) {
        return Ok(false);
    }
    let result = lower_bound_function_call(
        context,
        function,
        &mut bindings,
        body_cache,
        call_stack,
        constraints,
    );
    call_stack.remove(&function_name);
    result
}

fn lower_bound_function_call(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    function: &FunctionDeclaration,
    bindings: &mut SourceFunctionCallBindings<'_>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let checkpoint = constraints.checkpoint();
    let mut body_alias_scope = clone_alias_scope(&bindings.alias_scope);
    for statement in &function.statements {
        body_alias_scope.static_values = bindings.values.clone();
        if !lower_function_body_statement(
            context,
            statement,
            &mut bindings.values,
            &body_alias_scope,
            body_cache,
            call_stack,
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
    call_stack: &mut BTreeSet<String>,
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
                        call_stack,
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
                            call_stack,
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
    if source_statement_is_source_directive(context.module, statement).map_err(|source| {
        SourceKeyDirectoryMetadataError::Lex {
            source_name: context.module.source_name.clone(),
            source,
        }
    })? {
        return Ok(true);
    }
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(false);
    }
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(false);
    };
    if apply_static_expression_statement(context.program, expression, values) {
        return Ok(true);
    }
    if source_static_assertion(context, statement, values, alias_scope)? {
        return Ok(true);
    }
    if lower_function_call_expression(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        constraints,
    )? {
        return Ok(true);
    }
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

fn apply_static_expression_statement(
    program: &SourceProgram,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Unary { op, expr } => {
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => return false,
            };
            let Some(name) = expression_name(expr) else {
                return false;
            };
            apply_static_delta(name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let Some(name) = expression_name(left) else {
                return false;
            };
            if !values.contains_key(name) {
                return false;
            }
            let Some(right) = evaluate_source_static_expression(program, right, values) else {
                return false;
            };
            let value = match op {
                BinaryOperator::Assign => right,
                BinaryOperator::PlusAssign => {
                    let Some(current) = static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_add(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                BinaryOperator::MinusAssign => {
                    let Some(current) = static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_sub(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                BinaryOperator::StarAssign => {
                    let Some(current) = static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_mul(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                _ => return false,
            };
            values.insert(name.to_owned(), value);
            true
        }
        _ => false,
    }
}

fn apply_static_delta(
    name: &str,
    delta: i128,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(current) = static_integer_value(values.get(name)) else {
        return false;
    };
    let Some(value) = current.checked_add(delta) else {
        return false;
    };
    values.insert(name.to_owned(), FixedFileTemplateValue::Integer(value));
    true
}

fn static_integer_value(value: Option<&FixedFileTemplateValue>) -> Option<i128> {
    match value {
        Some(FixedFileTemplateValue::Integer(value)) => Some(*value),
        Some(FixedFileTemplateValue::Boolean(value)) => Some(if *value { 1 } else { 0 }),
        _ => None,
    }
}

fn source_static_assertion(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalAliasScope<'_>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(false);
    };
    let Some((name, arguments)) = source_call_expression(expression) else {
        return Ok(false);
    };
    if name != "assert" || !(1..=2).contains(&arguments.len()) || arguments[0].name.is_some() {
        return Ok(false);
    }
    match source_static_condition(context, &arguments[0].value, values, alias_scope) {
        Some(true) => Ok(true),
        Some(false) => Err(SourceKeyDirectoryMetadataError::StaticAssertionFailed {
            line: source_statement_line(context.module, statement),
        }),
        None => Ok(false),
    }
}

fn source_static_condition(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalAliasScope<'_>,
) -> Option<bool> {
    if let Some(value) = evaluate_source_static_expression(context.program, expression, values) {
        return Some(source_static_truthy_value(&value));
    }
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return None;
    };
    let left = source_static_integer_expression(context, left, values, alias_scope)?;
    let right = source_static_integer_expression(context, right, values, alias_scope)?;
    match op {
        BinaryOperator::Less => Some(left < right),
        BinaryOperator::LessEqual => Some(left <= right),
        BinaryOperator::Greater => Some(left > right),
        BinaryOperator::GreaterEqual => Some(left >= right),
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => Some(left == right),
        BinaryOperator::NotEqual => Some(left != right),
        _ => None,
    }
}

fn source_static_integer_expression(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalAliasScope<'_>,
) -> Option<i128> {
    let expression = strip_group_expression(expression);
    if let ExpressionKind::Call { callee, args } = &expression.kind {
        if args.len() == 1 && args[0].name.is_none() {
            if let ExpressionKind::Name(callee) = &strip_group_expression(callee).kind {
                if callee == "length" {
                    let ExpressionKind::Name(name) = &strip_group_expression(&args[0].value).kind
                    else {
                        return None;
                    };
                    return source_global_array_length(
                        context,
                        values,
                        &alias_scope.expression_arrays,
                        name,
                        &mut BTreeSet::new(),
                    );
                }
            }
        }
    }
    let value = evaluate_source_static_expression(context.program, expression, values)?;
    static_integer_value(Some(&value))
}

fn source_global_array_length(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_array_aliases: &SourceGlobalExpressionArrayAliases,
    name: &str,
    resolving_aliases: &mut BTreeSet<String>,
) -> Option<i128> {
    if let Some(length) = source_static_array_length(values, name) {
        return Some(length);
    }
    if let Some(slot) = context.slots.public_values.get(name) {
        return Some(i128::from(slot.dimension));
    }
    if let Some(slot) = context.slots.proof_values.get(name) {
        return Some(i128::from(slot.dimension));
    }
    if !resolving_aliases.insert(name.to_owned()) {
        return None;
    }
    let length = match expression_array_aliases.get(name)? {
        SourceGlobalExpressionArrayAlias::Name(alias) => source_global_array_length(
            context,
            values,
            expression_array_aliases,
            alias,
            resolving_aliases,
        )?,
        SourceGlobalExpressionArrayAlias::Values(expressions) => {
            i128::try_from(expressions.len()).ok()?
        }
    };
    resolving_aliases.remove(name);
    Some(length)
}

fn source_static_truthy_value(value: &FixedFileTemplateValue) -> bool {
    match value {
        FixedFileTemplateValue::Integer(value) => *value != 0,
        FixedFileTemplateValue::Boolean(value) => *value,
        FixedFileTemplateValue::String(value) => !value.is_empty(),
    }
}

fn apply_static_declaration(
    program: &SourceProgram,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if declaration.type_name.as_deref() == Some("expr") {
                return false;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return false;
            };
            if !declaration.array_dims.is_empty() {
                let Some(elements) = source_static_array_expression(program, expression, values)
                else {
                    return false;
                };
                return insert_source_static_array(values, &declaration.name, elements).is_some();
            }
            let Some(value) = evaluate_source_static_expression(program, expression, values) else {
                return false;
            };
            values.insert(declaration.name.clone(), value);
            true
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            if declaration.type_name == "expr" {
                return false;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return false;
            };
            if !declaration.array_dims.is_empty() {
                let Some(elements) = source_static_array_expression(program, expression, values)
                else {
                    return false;
                };
                return insert_source_static_array(values, &declaration.name, elements).is_some();
            }
            let Some(value) = evaluate_source_static_expression(program, expression, values) else {
                return false;
            };
            values.insert(declaration.name.clone(), value);
            true
        }
        _ => false,
    }
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

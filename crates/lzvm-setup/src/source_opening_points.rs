use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    lex_source, AirInstanceDeclaration, AirTemplateDeclaration, CallArgument, Expression,
    ExpressionKind, FixedFileTemplateValue, FunctionDeclaration, FunctionStatement,
    FunctionStatementDeclaration, FunctionStatementKind, SourceProgram, SourceProgramModule, Token,
    UnaryOperator,
};

use crate::{
    source_constraint_lowering::SourceExpressionAliases,
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_statement_hints::{
        source_lookup_statement_expressions, SourceExpressionArrayAlias,
        SourceExpressionArrayAliases,
    },
    source_static_values::{
        evaluate_source_static_expression, source_declaration_constant_values_from_cache,
        static_value_integer, SourceTemplateConstantValueCache,
    },
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

pub(crate) fn source_opening_points(
    program: &SourceProgram,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<Vec<i64>, SourceKeyDirectoryMetadataError> {
    let mut points = vec![0_i64];
    let unit_instance = source_opening_unit_instance(program, unit_name);
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        for template in &module.air_templates {
            if !active_templates.contains(&template.name) {
                continue;
            }
            if unit_instance.is_some_and(|instance| template.name != instance.template) {
                continue;
            }
            let mut alias_scope = SourceOpeningAliasScope::default();
            let context = SourceOpeningPointContext {
                program,
                module,
                tokens: &tokens,
                constant_values,
                template_values,
            };
            let mut function_call_stack = BTreeSet::new();
            let mut statement_values = source_opening_template_values(
                context.program,
                context.module,
                template,
                unit_instance,
                context.constant_values,
                context.template_values,
            );
            for statement in &template.statements {
                collect_source_statement_opening_points(
                    &context,
                    statement,
                    &mut statement_values,
                    &alias_scope,
                    body_cache,
                    &mut function_call_stack,
                    &mut points,
                )?;
                collect_source_opening_point_expression_alias(
                    statement,
                    &mut alias_scope.expressions,
                );
                collect_source_opening_point_expression_array_alias(
                    statement,
                    &mut alias_scope.expression_arrays,
                );
            }
        }
    }
    Ok(points)
}

fn source_opening_unit_instance<'a>(
    program: &'a SourceProgram,
    unit_name: Option<(&str, &str)>,
) -> Option<&'a AirInstanceDeclaration> {
    let (group_name, unit_name) = unit_name?;
    let units = program
        .air_units()
        .into_iter()
        .filter(|unit| !unit.virtual_instance)
        .collect::<Vec<_>>();
    let instances = program
        .modules
        .iter()
        .flat_map(|module| module.air_instances.iter())
        .filter(|instance| !instance.virtual_instance)
        .collect::<Vec<_>>();
    units
        .into_iter()
        .zip(instances)
        .find_map(|(unit, instance)| {
            (unit.group_name == group_name && unit.unit_name == unit_name).then_some(instance)
        })
}

fn source_opening_template_values(
    program: &SourceProgram,
    module: &SourceProgramModule,
    template: &AirTemplateDeclaration,
    instance: Option<&AirInstanceDeclaration>,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
    template_values: &SourceTemplateConstantValueCache,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let cached = source_declaration_constant_values_from_cache(
        module,
        template.body.start,
        template.body.end,
        base_values,
        template_values,
    );
    let Some(instance) = instance else {
        return cached.clone();
    };
    let mut values = base_values.clone();
    let mut provided = BTreeSet::new();
    if let Some(arguments) = instance.args_expressions.as_ref() {
        apply_source_opening_instance_arguments(
            program,
            template,
            arguments,
            &mut values,
            &mut provided,
        );
    }
    bind_source_opening_template_defaults(program, template, &mut values, &provided);

    let parameter_names = template
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<BTreeSet<_>>();
    for (name, value) in cached {
        if !parameter_names.contains(name.as_str()) {
            values.insert(name.clone(), value.clone());
        }
    }

    values
}

fn bind_source_opening_template_defaults(
    program: &SourceProgram,
    template: &AirTemplateDeclaration,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    provided: &BTreeSet<String>,
) {
    for parameter in &template.parameters {
        if provided.contains(&parameter.name) {
            continue;
        }
        let Some(value) = parameter
            .default_expression
            .as_ref()
            .and_then(|expression| evaluate_source_static_expression(program, expression, values))
        else {
            values.remove(&parameter.name);
            continue;
        };
        values.insert(parameter.name.clone(), value);
    }
}

fn apply_source_opening_instance_arguments(
    program: &SourceProgram,
    template: &AirTemplateDeclaration,
    arguments: &[CallArgument],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    provided: &mut BTreeSet<String>,
) {
    let mut positional_index = 0;
    for argument in arguments {
        let Some(value) = evaluate_source_static_expression(program, &argument.value, values)
        else {
            continue;
        };
        let name = if let Some(name) = argument.name.as_ref() {
            name
        } else {
            while template
                .parameters
                .get(positional_index)
                .is_some_and(|parameter| provided.contains(&parameter.name))
            {
                let Some(next) = positional_index.checked_add(1) else {
                    return;
                };
                positional_index = next;
            }
            let Some(parameter) = template.parameters.get(positional_index) else {
                continue;
            };
            &parameter.name
        };
        if provided.insert(name.clone()) {
            values.insert(name.clone(), value);
        }
        if argument.name.is_none() {
            let Some(next) = positional_index.checked_add(1) else {
                return;
            };
            positional_index = next;
        }
    }
}

struct SourceOpeningPointContext<'a> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    template_values: &'a SourceTemplateConstantValueCache,
}

#[derive(Clone, Default)]
struct SourceOpeningAliasScope {
    expressions: SourceExpressionAliases,
    expression_arrays: SourceExpressionArrayAliases,
}

fn collect_source_statement_opening_points(
    context: &SourceOpeningPointContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceOpeningAliasScope,
    body_cache: &mut SourceControlBodyCache,
    function_call_stack: &mut BTreeSet<String>,
    points: &mut Vec<i64>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        apply_source_opening_static_declaration(context.program, statement, values);
        collect_source_declaration_initializer_opening_points(
            context,
            statement,
            values,
            alias_scope,
            body_cache,
            function_call_stack,
            points,
        )?;
        return Ok(());
    }
    if statement.kind == FunctionStatementKind::Return {
        if let Some(expression) = statement.value_expression.as_ref() {
            let mut resolving_aliases = BTreeSet::new();
            let mut resolving_array_aliases = BTreeSet::new();
            collect_source_opening_points(
                context.program,
                expression,
                values,
                alias_scope,
                points,
                &mut resolving_aliases,
                &mut resolving_array_aliases,
            )?;
            collect_source_returned_function_call_opening_points(
                context,
                expression,
                values,
                alias_scope,
                body_cache,
                function_call_stack,
                points,
            )?;
        }
        return Ok(());
    }
    if statement.kind == FunctionStatementKind::If {
        match source_static_if_body_statements_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                let mut body_alias_scope = alias_scope.clone();
                for body_statement in body_statements.iter() {
                    collect_source_statement_opening_points(
                        context,
                        body_statement,
                        values,
                        &body_alias_scope,
                        body_cache,
                        function_call_stack,
                        points,
                    )?;
                    collect_source_opening_point_expression_alias(
                        body_statement,
                        &mut body_alias_scope.expressions,
                    );
                    collect_source_opening_point_expression_array_alias(
                        body_statement,
                        &mut body_alias_scope.expression_arrays,
                    );
                }
                return Ok(());
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind == FunctionStatementKind::For {
        match source_static_for_loop_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                for iteration_value in &loop_info.iteration_values {
                    let mut loop_alias_scope = alias_scope.clone();
                    values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                    for body_statement in loop_info.body_statements.iter() {
                        collect_source_statement_opening_points(
                            context,
                            body_statement,
                            values,
                            &loop_alias_scope,
                            body_cache,
                            function_call_stack,
                            points,
                        )?;
                        collect_source_opening_point_expression_alias(
                            body_statement,
                            &mut loop_alias_scope.expressions,
                        );
                        collect_source_opening_point_expression_array_alias(
                            body_statement,
                            &mut loop_alias_scope.expression_arrays,
                        );
                    }
                }
                return Ok(());
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(());
    }
    if let Some(expression) = statement.value_expression.as_ref() {
        let mut resolving_aliases = BTreeSet::new();
        let mut resolving_array_aliases = BTreeSet::new();
        collect_source_opening_points(
            context.program,
            expression,
            values,
            alias_scope,
            points,
            &mut resolving_aliases,
            &mut resolving_array_aliases,
        )?;
    }
    if let Some(expressions) = source_lookup_statement_expressions(context.module, statement)
        .map_err(|source| SourceKeyDirectoryMetadataError::Lex {
            source_name: context.module.source_name.clone(),
            source,
        })?
    {
        for expression in expressions {
            let mut resolving_aliases = BTreeSet::new();
            let mut resolving_array_aliases = BTreeSet::new();
            collect_source_opening_points(
                context.program,
                &expression,
                values,
                alias_scope,
                points,
                &mut resolving_aliases,
                &mut resolving_array_aliases,
            )?;
        }
    }
    collect_source_function_call_opening_points(
        context,
        statement,
        values,
        alias_scope,
        body_cache,
        function_call_stack,
        points,
    )?;
    Ok(())
}

fn collect_source_declaration_initializer_opening_points(
    context: &SourceOpeningPointContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceOpeningAliasScope,
    body_cache: &mut SourceControlBodyCache,
    function_call_stack: &mut BTreeSet<String>,
    points: &mut Vec<i64>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let expression = match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            declaration.initializer_expression.as_ref()
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            declaration.initializer_expression.as_ref()
        }
        _ => None,
    };
    let Some(expression) = expression else {
        return Ok(());
    };
    collect_source_returned_function_call_opening_points(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        function_call_stack,
        points,
    )?;
    Ok(())
}

fn collect_source_returned_function_call_opening_points(
    context: &SourceOpeningPointContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceOpeningAliasScope,
    body_cache: &mut SourceControlBodyCache,
    function_call_stack: &mut BTreeSet<String>,
    points: &mut Vec<i64>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some((name, arguments)) = source_call_expression(Some(expression)) else {
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
    if function.return_type.is_none() {
        return Ok(false);
    }
    let mut call_state = SourceOpeningFunctionCallState {
        body_cache,
        function_call_stack,
        points,
    };
    collect_source_function_body_opening_points(
        context,
        function,
        arguments,
        values,
        alias_scope,
        &mut call_state,
    )
}

fn collect_source_function_call_opening_points(
    context: &SourceOpeningPointContext<'_>,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceOpeningAliasScope,
    body_cache: &mut SourceControlBodyCache,
    function_call_stack: &mut BTreeSet<String>,
    points: &mut Vec<i64>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some((name, arguments)) = source_call_expression(statement.value_expression.as_ref())
    else {
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

    let mut call_state = SourceOpeningFunctionCallState {
        body_cache,
        function_call_stack,
        points,
    };
    collect_source_function_body_opening_points(
        context,
        function,
        arguments,
        values,
        alias_scope,
        &mut call_state,
    )
}

struct SourceOpeningFunctionCallState<'a> {
    body_cache: &'a mut SourceControlBodyCache,
    function_call_stack: &'a mut BTreeSet<String>,
    points: &'a mut Vec<i64>,
}

fn collect_source_function_body_opening_points(
    context: &SourceOpeningPointContext<'_>,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceOpeningAliasScope,
    call_state: &mut SourceOpeningFunctionCallState<'_>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some(mut bindings) = source_opening_function_call_bindings(
        context.program,
        function,
        arguments,
        values,
        alias_scope,
    ) else {
        return Ok(false);
    };

    let function_name = function.name.clone();
    if !call_state.function_call_stack.insert(function_name.clone()) {
        return unsupported("source opening point function call cycle");
    }

    let mut body_alias_scope = bindings.alias_scope;
    let result: Result<(), SourceKeyDirectoryMetadataError> = (|| {
        for body_statement in &function.statements {
            collect_source_statement_opening_points(
                context,
                body_statement,
                &mut bindings.values,
                &body_alias_scope,
                call_state.body_cache,
                call_state.function_call_stack,
                call_state.points,
            )?;
            collect_source_opening_point_expression_alias(
                body_statement,
                &mut body_alias_scope.expressions,
            );
            collect_source_opening_point_expression_array_alias(
                body_statement,
                &mut body_alias_scope.expression_arrays,
            );
        }
        Ok(())
    })();
    call_state.function_call_stack.remove(&function_name);
    result?;
    Ok(true)
}

struct SourceOpeningFunctionCallBindings {
    values: BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: SourceOpeningAliasScope,
}

fn source_opening_function_call_bindings(
    program: &SourceProgram,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceOpeningAliasScope,
) -> Option<SourceOpeningFunctionCallBindings> {
    let mut function_values = values.clone();
    let mut function_alias_scope = alias_scope.clone();
    for parameter in &function.parameters {
        if source_opening_expr_scalar_parameter(parameter) {
            if let Some(expression) = parameter.default_expression.as_ref() {
                function_alias_scope
                    .expressions
                    .insert(parameter.name.clone(), expression.clone());
            }
            continue;
        }
        if source_opening_expr_array_parameter(parameter) {
            if let Some(expression) = parameter.default_expression.as_ref() {
                let alias = source_opening_expression_array_alias(expression)?;
                function_alias_scope
                    .expression_arrays
                    .insert(parameter.name.clone(), alias);
            }
            continue;
        }
        if source_opening_const_scalar_parameter(parameter) {
            if let Some(expression) = parameter.default_expression.as_ref() {
                let value =
                    evaluate_source_static_expression(program, expression, &function_values)?;
                function_values.insert(parameter.name.clone(), value);
            }
            continue;
        }
        return None;
    }

    let mut positional_index = 0;
    for argument in arguments {
        if let Some(name) = argument.name.as_ref() {
            let parameter = function
                .parameters
                .iter()
                .find(|parameter| parameter.name == *name)?;
            source_bind_opening_function_argument(
                program,
                parameter,
                &argument.value,
                &mut function_values,
                &mut function_alias_scope.expressions,
                &mut function_alias_scope.expression_arrays,
            )?;
            continue;
        }
        let parameter = function.parameters.get(positional_index)?;
        source_bind_opening_function_argument(
            program,
            parameter,
            &argument.value,
            &mut function_values,
            &mut function_alias_scope.expressions,
            &mut function_alias_scope.expression_arrays,
        )?;
        positional_index += 1;
    }

    Some(SourceOpeningFunctionCallBindings {
        values: function_values,
        alias_scope: function_alias_scope,
    })
}

fn source_bind_opening_function_argument(
    program: &SourceProgram,
    parameter: &lzvm_pil::FunctionParameter,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &mut SourceExpressionAliases,
    expression_array_aliases: &mut SourceExpressionArrayAliases,
) -> Option<()> {
    if source_opening_expr_scalar_parameter(parameter) {
        if source_expression_name(expression) == Some(parameter.name.as_str()) {
            return Some(());
        }
        expression_aliases.insert(parameter.name.clone(), expression.clone());
        return Some(());
    }
    if source_opening_expr_array_parameter(parameter) {
        let alias = source_opening_expression_array_alias(expression)?;
        expression_array_aliases.insert(parameter.name.clone(), alias);
        return Some(());
    }
    if source_opening_const_scalar_parameter(parameter) {
        let value = evaluate_source_static_expression(program, expression, values)?;
        values.insert(parameter.name.clone(), value);
        return Some(());
    }
    None
}

fn source_opening_const_scalar_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    parameter.is_const && !parameter.by_reference && parameter.array_dims.is_empty()
}

fn source_opening_expr_scalar_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    !parameter.by_reference && parameter.array_dims.is_empty() && parameter.type_name == "expr"
}

fn source_opening_expr_array_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    !parameter.by_reference && !parameter.array_dims.is_empty() && parameter.type_name == "expr"
}

fn source_opening_expression_array_alias(
    expression: &Expression,
) -> Option<SourceExpressionArrayAlias> {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(SourceExpressionArrayAlias::Name(name.clone())),
        ExpressionKind::Array(expressions) => {
            Some(SourceExpressionArrayAlias::Values(expressions.clone()))
        }
        _ => None,
    }
}

fn source_call_expression(expression: Option<&Expression>) -> Option<(&str, &[CallArgument])> {
    let ExpressionKind::Call { callee, args } = &strip_source_group_expression(expression?).kind
    else {
        return None;
    };
    let ExpressionKind::Name(name) = &strip_source_group_expression(callee).kind else {
        return None;
    };
    Some((name.as_str(), args.as_slice()))
}

fn strip_source_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_source_group_expression(inner),
        _ => expression,
    }
}

fn collect_source_opening_points(
    program: &SourceProgram,
    expression: &Expression,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceOpeningAliasScope,
    points: &mut Vec<i64>,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    match &expression.kind {
        ExpressionKind::Group(inner) => collect_source_opening_points(
            program,
            inner,
            constant_values,
            alias_scope,
            points,
            resolving_aliases,
            resolving_array_aliases,
        ),
        ExpressionKind::Unary { expr, .. } => collect_source_opening_points(
            program,
            expr,
            constant_values,
            alias_scope,
            points,
            resolving_aliases,
            resolving_array_aliases,
        ),
        ExpressionKind::Binary { left, right, .. } => {
            collect_source_opening_points(
                program,
                left,
                constant_values,
                alias_scope,
                points,
                resolving_aliases,
                resolving_array_aliases,
            )?;
            collect_source_opening_points(
                program,
                right,
                constant_values,
                alias_scope,
                points,
                resolving_aliases,
                resolving_array_aliases,
            )
        }
        ExpressionKind::Array(values) => {
            for value in values {
                collect_source_opening_points(
                    program,
                    value,
                    constant_values,
                    alias_scope,
                    points,
                    resolving_aliases,
                    resolving_array_aliases,
                )?;
            }
            Ok(())
        }
        ExpressionKind::Call { callee, args } => {
            collect_source_opening_points(
                program,
                callee,
                constant_values,
                alias_scope,
                points,
                resolving_aliases,
                resolving_array_aliases,
            )?;
            for arg in args {
                collect_source_opening_points(
                    program,
                    &arg.value,
                    constant_values,
                    alias_scope,
                    points,
                    resolving_aliases,
                    resolving_array_aliases,
                )?;
            }
            Ok(())
        }
        ExpressionKind::Index { target, index } => {
            if let Some(element) = source_opening_indexed_array_alias_element(
                program,
                expression,
                constant_values,
                alias_scope,
                resolving_array_aliases,
            ) {
                return match element {
                    SourceOpeningArrayAliasElement::Expression(expression) => {
                        collect_source_opening_points(
                            program,
                            expression,
                            constant_values,
                            alias_scope,
                            points,
                            resolving_aliases,
                            resolving_array_aliases,
                        )
                    }
                    SourceOpeningArrayAliasElement::NamedArray => Ok(()),
                };
            }
            collect_source_opening_points(
                program,
                target,
                constant_values,
                alias_scope,
                points,
                resolving_aliases,
                resolving_array_aliases,
            )?;
            collect_source_opening_points(
                program,
                index,
                constant_values,
                alias_scope,
                points,
                resolving_aliases,
                resolving_array_aliases,
            )
        }
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => {
            collect_source_opening_points(
                program,
                target,
                constant_values,
                alias_scope,
                points,
                resolving_aliases,
                resolving_array_aliases,
            )?;
            collect_source_opening_points(
                program,
                offset,
                constant_values,
                alias_scope,
                points,
                resolving_aliases,
                resolving_array_aliases,
            )?;
            let signed_offset = source_row_offset_value(program, offset, *prior, constant_values)?;
            if !points.contains(&signed_offset) {
                points.push(signed_offset);
            }
            Ok(())
        }
        ExpressionKind::Name(name) => {
            if let Some(alias) = alias_scope.expressions.get(name) {
                if !resolving_aliases.insert(name.clone()) {
                    return unsupported("source opening point expression alias cycle");
                }
                let result = collect_source_opening_points(
                    program,
                    alias,
                    constant_values,
                    alias_scope,
                    points,
                    resolving_aliases,
                    resolving_array_aliases,
                );
                resolving_aliases.remove(name);
                return result;
            }
            if let Some(alias) = alias_scope.expression_arrays.get(name) {
                if !resolving_array_aliases.insert(name.clone()) {
                    return unsupported("source opening point expression array alias cycle");
                }
                let result = collect_source_opening_points_from_array_alias(
                    program,
                    alias,
                    constant_values,
                    alias_scope,
                    points,
                    resolving_aliases,
                    resolving_array_aliases,
                );
                resolving_array_aliases.remove(name);
                return result;
            }
            Ok(())
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => Ok(()),
    }
}

enum SourceOpeningArrayAliasElement<'a> {
    Expression(&'a Expression),
    NamedArray,
}

fn source_opening_indexed_array_alias_element<'a>(
    program: &SourceProgram,
    expression: &Expression,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &'a SourceOpeningAliasScope,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceOpeningArrayAliasElement<'a>> {
    let (name, index_expressions) =
        source_opening_index_chain(strip_source_group_expression(expression))?;
    let indices = index_expressions
        .iter()
        .map(|index| source_opening_index_value(program, index, constant_values))
        .collect::<Option<Vec<_>>>()?;
    let alias = alias_scope.expression_arrays.get(name)?;
    source_opening_array_alias_path_element(alias, &indices, alias_scope, resolving_array_aliases)
}

fn source_opening_array_alias_path_element<'a>(
    alias: &'a SourceExpressionArrayAlias,
    indices: &[usize],
    alias_scope: &'a SourceOpeningAliasScope,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceOpeningArrayAliasElement<'a>> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if let Some(next_alias) = alias_scope.expression_arrays.get(name) {
                if !resolving_array_aliases.insert(name.clone()) {
                    return None;
                }
                let element = source_opening_array_alias_path_element(
                    next_alias,
                    indices,
                    alias_scope,
                    resolving_array_aliases,
                );
                resolving_array_aliases.remove(name);
                return element;
            }
            Some(SourceOpeningArrayAliasElement::NamedArray)
        }
        SourceExpressionArrayAlias::Values(expressions) => source_opening_expression_array_element(
            expressions,
            indices,
            alias_scope,
            resolving_array_aliases,
        ),
    }
}

fn source_opening_expression_array_element<'a>(
    expressions: &'a [Expression],
    indices: &[usize],
    alias_scope: &'a SourceOpeningAliasScope,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceOpeningArrayAliasElement<'a>> {
    let (index, rest) = indices.split_first()?;
    let expression = expressions.get(*index)?;
    if rest.is_empty() {
        return Some(SourceOpeningArrayAliasElement::Expression(expression));
    }
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Array(expressions) => source_opening_expression_array_element(
            expressions,
            rest,
            alias_scope,
            resolving_array_aliases,
        ),
        ExpressionKind::Name(name) => {
            let alias = alias_scope.expression_arrays.get(name)?;
            source_opening_array_alias_path_element(
                alias,
                rest,
                alias_scope,
                resolving_array_aliases,
            )
        }
        _ => None,
    }
}

fn source_opening_index_chain(expression: &Expression) -> Option<(&str, Vec<&Expression>)> {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some((name, Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (name, mut indices) = source_opening_index_chain(target)?;
            indices.push(index);
            Some((name, indices))
        }
        _ => None,
    }
}

fn source_expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => source_expression_name(inner),
        _ => None,
    }
}

fn source_opening_index_value(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<usize> {
    if let Some(value) = evaluate_source_static_expression(program, expression, values) {
        return static_value_integer(&value).and_then(|value| usize::try_from(value).ok());
    }
    usize::try_from(eval_i128_expression(expression).ok()?).ok()
}

fn collect_source_opening_points_from_array_alias(
    program: &SourceProgram,
    alias: &SourceExpressionArrayAlias,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceOpeningAliasScope,
    points: &mut Vec<i64>,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            let Some(next_alias) = alias_scope.expression_arrays.get(name) else {
                return Ok(());
            };
            if !resolving_array_aliases.insert(name.clone()) {
                return unsupported("source opening point expression array alias cycle");
            }
            let result = collect_source_opening_points_from_array_alias(
                program,
                next_alias,
                constant_values,
                alias_scope,
                points,
                resolving_aliases,
                resolving_array_aliases,
            );
            resolving_array_aliases.remove(name);
            result
        }
        SourceExpressionArrayAlias::Values(expressions) => {
            for expression in expressions {
                collect_source_opening_points(
                    program,
                    expression,
                    constant_values,
                    alias_scope,
                    points,
                    resolving_aliases,
                    resolving_array_aliases,
                )?;
            }
            Ok(())
        }
    }
}

fn collect_source_opening_point_expression_alias(
    statement: &FunctionStatement,
    expression_aliases: &mut SourceExpressionAliases,
) {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if declaration.type_name.as_deref() != Some("expr")
                || !declaration.array_dims.is_empty()
            {
                return;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return;
            };
            expression_aliases.insert(declaration.name.clone(), expression.clone());
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            if declaration.type_name != "expr" || !declaration.array_dims.is_empty() {
                return;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return;
            };
            expression_aliases.insert(declaration.name.clone(), expression.clone());
        }
        _ => {}
    }
}

fn collect_source_opening_point_expression_array_alias(
    statement: &FunctionStatement,
    expression_array_aliases: &mut SourceExpressionArrayAliases,
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
    if let Some(alias) = source_opening_expression_array_alias(expression) {
        expression_array_aliases.insert(declaration.name.clone(), alias);
    }
}

fn apply_source_opening_static_declaration(
    program: &SourceProgram,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if !declaration.array_dims.is_empty() {
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
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            if !declaration.array_dims.is_empty() {
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
        _ => false,
    }
}

fn source_row_offset_value(
    program: &SourceProgram,
    expression: &Expression,
    prior: bool,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<i64, SourceKeyDirectoryMetadataError> {
    let offset = eval_i128_expression_with_values(program, expression, values)?;
    let signed = if prior {
        offset
            .checked_neg()
            .ok_or_else(|| unsupported_source_message("source row offset overflow"))?
    } else {
        offset
    };
    i64::try_from(signed).map_err(|_| unsupported_source_message("source row offset overflow"))
}

fn eval_i128_expression_with_values(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<i128, SourceKeyDirectoryMetadataError> {
    if let Some(value) = evaluate_source_static_expression(program, expression, values) {
        if let Some(value) = static_value_integer(&value) {
            return Ok(value);
        }
    }
    eval_i128_expression(expression)
}

fn eval_i128_expression(expression: &Expression) -> Result<i128, SourceKeyDirectoryMetadataError> {
    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128_literal(value)
        }
        ExpressionKind::Group(value) => eval_i128_expression(value),
        ExpressionKind::Unary { op, expr } => {
            let value = eval_i128_expression(expr)?;
            match op {
                UnaryOperator::Plus => Ok(value),
                UnaryOperator::Minus => value
                    .checked_neg()
                    .ok_or_else(|| unsupported_source_message("source expression overflow")),
                _ => unsupported("unsupported source unary expression"),
            }
        }
        _ => unsupported("source row offset must be a static integer"),
    }
}

fn parse_i128_literal(value: &str) -> Result<i128, SourceKeyDirectoryMetadataError> {
    let value = value.trim().replace('_', "");
    if let Some(hex) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        let parsed = i128::from_str_radix(hex, 16)
            .map_err(|_| unsupported_source_message("invalid source integer literal"))?;
        parsed
            .checked_neg()
            .ok_or_else(|| unsupported_source_message("source integer literal overflow"))
    } else if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        i128::from_str_radix(hex, 16)
            .map_err(|_| unsupported_source_message("invalid source integer literal"))
    } else {
        value
            .parse::<i128>()
            .map_err(|_| unsupported_source_message("invalid source integer literal"))
    }
}

fn unsupported<T>(message: impl Into<String>) -> Result<T, SourceKeyDirectoryMetadataError> {
    Err(unsupported_source_message(message))
}

fn unsupported_source_message(message: impl Into<String>) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}

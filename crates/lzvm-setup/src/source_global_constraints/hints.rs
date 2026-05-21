use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::expression_info::{ExpressionInfo, HintInfo};
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::global_program::GlobalProgramError;
use lzvm_artifacts::hint_program::{global_hint_program_from_expression_info, HintProgram};
use lzvm_pil::{
    lex_source, BinaryOperator, CallArgument, Expression, ExpressionKind, FixedFileTemplateValue,
    FunctionDeclaration, FunctionParameter, FunctionStatement, FunctionStatementDeclaration,
    FunctionStatementKind, SourceProgram, SourceProgramModule, Token, UnaryOperator,
};

use crate::{
    source_constraint_lowering::SourceExpressionAliases,
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_expression_aliases::collect_source_template_expression_alias,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_scalar_slots::SourceScalarSlots,
    source_statement_hints::{
        lower_source_lookup_statement, source_statement_line, SourceExpressionArrayAlias,
        SourceExpressionArrayAliases, SourceLookupInputs,
    },
    source_static_values::{
        evaluate_source_static_expression, insert_source_static_array,
        source_static_array_expression, source_static_array_length, source_static_array_values,
    },
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

use super::{strip_group_expression, unsupported_source_message};

pub(super) fn source_global_hints(
    program: &SourceProgram,
    global_info: &GlobalInfo,
    static_values: &BTreeMap<String, FixedFileTemplateValue>,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<HintProgram, SourceKeyDirectoryMetadataError> {
    let scalar_slots =
        SourceScalarSlots::from_global(&global_info.publics_map, &global_info.proof_values_map)
            .map_err(|error| unsupported_source_message(error.to_string()))?;
    let mut hints = Vec::<HintInfo>::new();

    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        for group in &module.air_groups {
            let mut values = static_values.clone();
            let mut alias_scope = SourceGlobalHintAliasScope::default();
            let mut context = SourceGlobalHintLoweringContext {
                program,
                module,
                tokens: &tokens,
                scalar_slots: &scalar_slots,
                body_cache,
                hints: &mut hints,
            };
            for statement in &group.statements {
                lower_source_global_hint_statement(
                    &mut context,
                    statement,
                    &mut values,
                    &alias_scope,
                )?;
                collect_source_template_expression_alias(statement, &mut alias_scope.expressions);
                collect_source_global_hint_expression_array_alias(
                    statement,
                    &mut alias_scope.expression_arrays,
                );
            }
        }
    }

    let expression_info = ExpressionInfo {
        hints,
        expressions: Vec::new(),
        constraints: Vec::new(),
    };
    global_hint_program_from_expression_info(&expression_info).map_err(|error| {
        SourceKeyDirectoryMetadataError::GlobalProgram(GlobalProgramError::Hints(error))
    })
}

struct SourceGlobalHintLoweringContext<'a, 'b> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    scalar_slots: &'a SourceScalarSlots,
    body_cache: &'b mut SourceControlBodyCache,
    hints: &'b mut Vec<HintInfo>,
}

#[derive(Clone, Default)]
struct SourceGlobalHintAliasScope {
    expressions: SourceExpressionAliases,
    expression_arrays: SourceExpressionArrayAliases,
}

fn lower_source_global_hint_statement(
    context: &mut SourceGlobalHintLoweringContext<'_, '_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalHintAliasScope,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    match statement.kind {
        FunctionStatementKind::Declaration => {
            apply_source_global_hint_static_declaration(context.program, statement, values);
            Ok(())
        }
        FunctionStatementKind::If => {
            match source_static_if_body_statements_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                context.body_cache,
            ) {
                Ok(Some(body_statements)) => {
                    let mut body_alias_scope = alias_scope.clone();
                    for body_statement in body_statements.iter() {
                        lower_source_global_hint_statement(
                            context,
                            body_statement,
                            values,
                            &body_alias_scope,
                        )?;
                        collect_source_template_expression_alias(
                            body_statement,
                            &mut body_alias_scope.expressions,
                        );
                        collect_source_global_hint_expression_array_alias(
                            body_statement,
                            &mut body_alias_scope.expression_arrays,
                        );
                    }
                }
                Ok(None)
                | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
                Err(error) => return Err(error),
            }
            Ok(())
        }
        FunctionStatementKind::For => {
            match source_static_for_loop_with_tokens(
                context.program,
                context.module,
                context.tokens,
                statement,
                values,
                context.body_cache,
            ) {
                Ok(Some(loop_info)) => {
                    let previous = values.get(&loop_info.variable_name).cloned();
                    for iteration_value in &loop_info.iteration_values {
                        let mut loop_alias_scope = alias_scope.clone();
                        values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                        for body_statement in loop_info.body_statements.iter() {
                            lower_source_global_hint_statement(
                                context,
                                body_statement,
                                values,
                                &loop_alias_scope,
                            )?;
                            collect_source_template_expression_alias(
                                body_statement,
                                &mut loop_alias_scope.expressions,
                            );
                            collect_source_global_hint_expression_array_alias(
                                body_statement,
                                &mut loop_alias_scope.expression_arrays,
                            );
                        }
                    }
                    if let Some(previous) = previous {
                        values.insert(loop_info.variable_name, previous);
                    } else {
                        values.remove(&loop_info.variable_name);
                    }
                }
                Ok(None)
                | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
                Err(error) => return Err(error),
            }
            Ok(())
        }
        FunctionStatementKind::Expression => {
            if apply_source_global_hint_static_expression_statement(
                context.program,
                statement.value_expression.as_ref(),
                values,
            ) {
                return Ok(());
            }
            if source_global_hint_static_assertion(
                context.program,
                context.module,
                statement,
                values,
                alias_scope,
            )? {
                return Ok(());
            }
            if lower_source_global_hint_function_call(context, statement, values, alias_scope)? {
                return Ok(());
            }
            let lookup_inputs = SourceLookupInputs {
                program: context.program,
                module: context.module,
                values,
                expression_aliases: &alias_scope.expressions,
                expression_array_aliases: &alias_scope.expression_arrays,
                scalar_slots: context.scalar_slots,
                opening_points: &[],
            };
            if let Some(hint) =
                lower_source_lookup_statement(&lookup_inputs, statement).map_err(|source| {
                    SourceKeyDirectoryMetadataError::Lex {
                        source_name: context.module.source_name.clone(),
                        source,
                    }
                })?
            {
                if source_global_hint_is_structured(&hint) {
                    context.hints.push(hint);
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn lower_source_global_hint_function_call(
    context: &mut SourceGlobalHintLoweringContext<'_, '_>,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalHintAliasScope,
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
    let Some(mut bindings) = source_global_function_call_bindings(
        context.program,
        context.module,
        function,
        arguments,
        values,
        alias_scope,
    ) else {
        return Ok(false);
    };

    let mut function_hints = Vec::new();
    {
        let mut function_context = SourceGlobalHintLoweringContext {
            program: context.program,
            module: context.module,
            tokens: context.tokens,
            scalar_slots: context.scalar_slots,
            body_cache: context.body_cache,
            hints: &mut function_hints,
        };
        let mut body_alias_scope = bindings.alias_scope;
        for body_statement in &function.statements {
            lower_source_global_hint_statement(
                &mut function_context,
                body_statement,
                &mut bindings.values,
                &body_alias_scope,
            )?;
            collect_source_template_expression_alias(
                body_statement,
                &mut body_alias_scope.expressions,
            );
            collect_source_global_hint_expression_array_alias(
                body_statement,
                &mut body_alias_scope.expression_arrays,
            );
        }
    }

    context.hints.extend(function_hints);
    Ok(true)
}

struct SourceGlobalFunctionCallBindings {
    values: BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: SourceGlobalHintAliasScope,
}

fn source_global_function_call_bindings(
    program: &SourceProgram,
    module: &SourceProgramModule,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalHintAliasScope,
) -> Option<SourceGlobalFunctionCallBindings> {
    let mut function_values = values.clone();
    let mut function_alias_scope = alias_scope.clone();
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
        bind_source_global_function_argument(
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
        bind_source_global_function_default(
            program,
            module,
            parameter,
            &mut function_values,
            &mut function_alias_scope.expressions,
            &mut function_alias_scope.expression_arrays,
        )?;
    }

    Some(SourceGlobalFunctionCallBindings {
        values: function_values,
        alias_scope: function_alias_scope,
    })
}

fn bind_source_global_function_argument(
    program: &SourceProgram,
    parameter: &FunctionParameter,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &mut SourceExpressionAliases,
    expression_array_aliases: &mut SourceExpressionArrayAliases,
) -> Option<()> {
    if source_expr_parameter(parameter) {
        if source_expression_name(expression) == Some(parameter.name.as_str()) {
            return Some(());
        }
        expression_aliases.insert(parameter.name.clone(), expression.clone());
        return Some(());
    }
    if source_expr_array_parameter(parameter) {
        insert_source_expr_array_static_values(program, expression, values, &parameter.name)?;
        let alias = source_global_hint_expression_array_alias(expression)?;
        if matches!(&alias, SourceExpressionArrayAlias::Name(name) if name == &parameter.name) {
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
    if let Some(elements) = source_static_array_expression(program, expression, values) {
        return insert_source_static_array(values, &parameter.name, elements);
    }
    let name = source_expression_name(expression)?;
    let elements = source_static_array_values(values, name)?;
    insert_source_static_array(values, &parameter.name, elements)
}

fn bind_source_global_function_default(
    program: &SourceProgram,
    _module: &SourceProgramModule,
    parameter: &FunctionParameter,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &mut SourceExpressionAliases,
    expression_array_aliases: &mut SourceExpressionArrayAliases,
) -> Option<()> {
    if source_expr_parameter(parameter) {
        let expression = parameter.default_expression.as_ref()?;
        expression_aliases.insert(parameter.name.clone(), expression.clone());
        return Some(());
    }
    if source_expr_array_parameter(parameter) {
        let expression = parameter.default_expression.as_ref()?;
        insert_source_expr_array_static_values(program, expression, values, &parameter.name)?;
        let alias = source_global_hint_expression_array_alias(expression)?;
        expression_array_aliases.insert(parameter.name.clone(), alias);
        return Some(());
    }
    if source_const_parameter(parameter) && parameter.array_dims.is_empty() {
        let expression = parameter.default_expression.as_ref()?;
        let value = evaluate_source_static_expression(program, expression, values)?;
        values.insert(parameter.name.clone(), value);
        return Some(());
    }
    if !source_const_parameter(parameter) {
        return None;
    }
    None
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
    let Some(name) = source_expression_name(expression) else {
        return Some(());
    };
    let Some(elements) = source_static_array_values(values, name) else {
        return Some(());
    };
    insert_source_static_array(values, target_name, elements)
}

fn source_const_parameter(parameter: &FunctionParameter) -> bool {
    parameter.is_const && !parameter.by_reference
}

fn source_expr_parameter(parameter: &FunctionParameter) -> bool {
    !parameter.by_reference && parameter.array_dims.is_empty() && parameter.type_name == "expr"
}

fn source_expr_array_parameter(parameter: &FunctionParameter) -> bool {
    !parameter.by_reference && !parameter.array_dims.is_empty() && parameter.type_name == "expr"
}

fn source_call_expression(expression: Option<&Expression>) -> Option<(&str, &[CallArgument])> {
    let ExpressionKind::Call { callee, args } = &expression?.kind else {
        return None;
    };
    let ExpressionKind::Name(name) = &callee.kind else {
        return None;
    };
    Some((name.as_str(), args.as_slice()))
}

fn apply_source_global_hint_static_expression_statement(
    program: &SourceProgram,
    expression: Option<&Expression>,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(expression) = expression else {
        return false;
    };
    match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => return false,
            };
            let Some(name) = source_expression_name(expr) else {
                return false;
            };
            apply_source_global_hint_static_delta(name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let Some(name) = source_expression_name(left) else {
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
                    let Some(current) = source_global_hint_static_integer_value(values.get(name))
                    else {
                        return false;
                    };
                    let Some(right) = source_global_hint_static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_add(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                BinaryOperator::MinusAssign => {
                    let Some(current) = source_global_hint_static_integer_value(values.get(name))
                    else {
                        return false;
                    };
                    let Some(right) = source_global_hint_static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_sub(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                BinaryOperator::StarAssign => {
                    let Some(current) = source_global_hint_static_integer_value(values.get(name))
                    else {
                        return false;
                    };
                    let Some(right) = source_global_hint_static_integer_value(Some(&right)) else {
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

fn source_global_hint_static_assertion(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalHintAliasScope,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some((name, arguments)) = source_call_expression(statement.value_expression.as_ref())
    else {
        return Ok(false);
    };
    if name != "assert" || !(1..=2).contains(&arguments.len()) || arguments[0].name.is_some() {
        return Ok(false);
    }
    match source_global_hint_static_condition(program, &arguments[0].value, values, alias_scope) {
        Some(true) => Ok(true),
        Some(false) => Err(SourceKeyDirectoryMetadataError::StaticAssertionFailed {
            line: source_statement_line(module, statement),
        }),
        None => Ok(false),
    }
}

fn source_global_hint_static_condition(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalHintAliasScope,
) -> Option<bool> {
    if let Some(value) = evaluate_source_static_expression(program, expression, values) {
        return Some(source_global_hint_static_truthy_value(&value));
    }
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return None;
    };
    let left = source_global_hint_static_integer_expression(program, left, values, alias_scope)?;
    let right = source_global_hint_static_integer_expression(program, right, values, alias_scope)?;
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

fn source_global_hint_static_integer_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceGlobalHintAliasScope,
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
                    return source_global_hint_array_length(
                        values,
                        &alias_scope.expression_arrays,
                        name,
                        &mut BTreeSet::new(),
                    );
                }
            }
        }
    }
    let value = evaluate_source_static_expression(program, expression, values)?;
    source_global_hint_static_integer_value(Some(&value))
}

fn source_global_hint_array_length(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_array_aliases: &SourceExpressionArrayAliases,
    name: &str,
    resolving_aliases: &mut BTreeSet<String>,
) -> Option<i128> {
    if let Some(length) = source_static_array_length(values, name) {
        return Some(length);
    }
    if !resolving_aliases.insert(name.to_owned()) {
        return None;
    }
    let length = match expression_array_aliases.get(name)? {
        SourceExpressionArrayAlias::Name(alias) => source_global_hint_array_length(
            values,
            expression_array_aliases,
            alias,
            resolving_aliases,
        )?,
        SourceExpressionArrayAlias::Values(expressions) => {
            i128::try_from(expressions.len()).ok()?
        }
    };
    resolving_aliases.remove(name);
    Some(length)
}

fn source_global_hint_static_integer_value(value: Option<&FixedFileTemplateValue>) -> Option<i128> {
    match value {
        Some(FixedFileTemplateValue::Integer(value)) => Some(*value),
        Some(FixedFileTemplateValue::Boolean(value)) => Some(if *value { 1 } else { 0 }),
        _ => None,
    }
}

fn source_global_hint_static_truthy_value(value: &FixedFileTemplateValue) -> bool {
    match value {
        FixedFileTemplateValue::Integer(value) => *value != 0,
        FixedFileTemplateValue::Boolean(value) => *value,
        FixedFileTemplateValue::String(value) => !value.is_empty(),
    }
}

fn apply_source_global_hint_static_delta(
    name: &str,
    delta: i128,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(current) = source_global_hint_static_integer_value(values.get(name)) else {
        return false;
    };
    let Some(value) = current.checked_add(delta) else {
        return false;
    };
    values.insert(name.to_owned(), FixedFileTemplateValue::Integer(value));
    true
}

fn source_expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => source_expression_name(inner),
        _ => None,
    }
}

fn collect_source_global_hint_expression_array_alias(
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
    if let Some(alias) = source_global_hint_expression_array_alias(expression) {
        expression_array_aliases.insert(declaration.name.clone(), alias);
    }
}

fn source_global_hint_expression_array_alias(
    expression: &Expression,
) -> Option<SourceExpressionArrayAlias> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(SourceExpressionArrayAlias::Name(name.clone())),
        ExpressionKind::Array(expressions) => {
            Some(SourceExpressionArrayAlias::Values(expressions.clone()))
        }
        _ => None,
    }
}

fn source_global_hint_is_structured(hint: &HintInfo) -> bool {
    hint.fields.iter().all(|field| field.name != "line")
}

fn apply_source_global_hint_static_declaration(
    program: &SourceProgram,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) {
    let Some(declaration) = statement.declaration.as_ref() else {
        return;
    };
    match declaration {
        FunctionStatementDeclaration::Constant(declaration) => {
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return;
            };
            if !declaration.array_dims.is_empty() {
                if let Some(elements) = source_static_array_expression(program, expression, values)
                {
                    let _ = insert_source_static_array(values, &declaration.name, elements);
                }
                return;
            }
            if let Some(value) = evaluate_source_static_expression(program, expression, values) {
                values.insert(declaration.name.clone(), value);
            }
        }
        FunctionStatementDeclaration::Variable(declaration) => {
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return;
            };
            if !declaration.array_dims.is_empty() {
                if let Some(elements) = source_static_array_expression(program, expression, values)
                {
                    let _ = insert_source_static_array(values, &declaration.name, elements);
                }
                return;
            }
            if let Some(value) = evaluate_source_static_expression(program, expression, values) {
                values.insert(declaration.name.clone(), value);
            }
        }
        FunctionStatementDeclaration::Column(_) => {}
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use lzvm_artifacts::expression_info::{ConstraintCode, ExpressionInfo, HintInfo};
use lzvm_artifacts::global_info::{NamedStageValue, PublicValue};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_pil::{
    lex_source, parse_expression_tokens, BinaryOperator, CallArgument, ColumnKind, Expression,
    ExpressionKind, FixedFileTemplateValue, FunctionDeclaration, FunctionStatement,
    FunctionStatementDeclaration, FunctionStatementKind, SourceFile, SourceProgram,
    SourceProgramModule, SourceSpan, Token, TokenKind, UnaryOperator,
};

use crate::{
    source_constraint_lowering::{
        lower_source_template_boolean_constraint, SourceExpressionAliases,
    },
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_expression_aliases::collect_source_template_expression_alias,
    source_expression_filters::{
        source_expression_assigns_fixed_index, source_expression_is_assignment,
        source_expression_is_constrained_assignment, source_expression_is_equality_constraint,
    },
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_scalar_slots::{SourceChallengeSlotMetadata, SourceScalarSlots},
    source_scope::{
        concrete_template_names, declaration_in_function_body, declaration_in_inactive_template,
    },
    source_statement_hints::{
        lower_source_lookup_statement, lower_unsupported_source_assignment_statement,
        lower_unsupported_source_call_statement, lower_unsupported_source_constraint_statement,
        lower_unsupported_source_template_statement, source_statement_contains_assignment_operator,
        source_statement_first_token_kind, source_statement_line, SourceExpressionArrayAlias,
        SourceExpressionArrayAliases, SourceLookupInputs,
    },
    source_static_values::{
        evaluate_source_static_expression, insert_source_static_array,
        source_declaration_constant_values_from_cache, source_scalar_constant_values,
        source_static_array_length, source_static_array_values,
        source_static_assignment_expression, source_template_constant_value_cache,
    },
    source_template_context::SourceTemplateLoweringContext,
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

pub(crate) fn source_expression_info(
    program: &SourceProgram,
    setup: &UnitSetupInfo,
    publics: &[PublicValue],
    challenges: &[SourceChallengeSlotMetadata],
    proof_values: &[NamedStageValue],
    body_caches: &mut SourceControlBodyCaches,
) -> Result<ExpressionInfo, SourceKeyDirectoryMetadataError> {
    let scalar_slots = SourceScalarSlots::from_setup(setup, publics, challenges, proof_values)
        .map_err(|error| unsupported_source_message(error.to_string()))?;
    let fixed_assignment_columns = source_fixed_assignment_column_names(program);
    let active_templates = concrete_template_names(program);
    let constant_values = source_scalar_constant_values(program, 1_u64 << setup.stark.n_bits);
    let template_values = source_template_constant_value_cache(program, &constant_values);
    let mut hints = Vec::new();
    let mut constraints = Vec::new();
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
            let context = SourceTemplateLoweringContext {
                program,
                module,
                tokens: &tokens,
                scalar_slots: &scalar_slots,
                opening_points: &setup.opening_points,
                fixed_columns: &fixed_assignment_columns,
                constant_values: &constant_values,
                template_values: &template_values,
            };
            let mut expression_aliases = SourceExpressionAliases::new();
            let mut statement_values = source_declaration_constant_values_from_cache(
                context.module,
                template.body.start,
                template.body.end,
                context.constant_values,
                context.template_values,
            )
            .clone();
            for statement in &template.statements {
                lower_source_template_statement(
                    &context,
                    statement,
                    &mut statement_values,
                    &expression_aliases,
                    body_cache,
                    &mut hints,
                    &mut constraints,
                )?;
                collect_source_template_expression_alias(statement, &mut expression_aliases);
            }
        }
    }
    Ok(ExpressionInfo {
        hints,
        expressions: Vec::new(),
        constraints,
    })
}

fn source_fixed_assignment_column_names(program: &SourceProgram) -> BTreeSet<String> {
    let active_templates = concrete_template_names(program);
    program
        .modules
        .iter()
        .flat_map(|module| {
            module.columns.iter().filter(|declaration| {
                declaration.kind == ColumnKind::Fixed
                    && declaration.initializer.is_none()
                    && !declaration_in_function_body(module, declaration.start, declaration.end)
                    && !declaration_in_inactive_template(
                        module,
                        declaration.start,
                        declaration.end,
                        &active_templates,
                    )
            })
        })
        .flat_map(|declaration| declaration.items.iter().map(|item| item.name.clone()))
        .collect()
}

fn lower_source_template_statement(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    body_cache: &mut SourceControlBodyCache,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        apply_source_static_declaration(context.program, statement, values);
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
                let mut body_aliases = expression_aliases.clone();
                for body_statement in body_statements.iter() {
                    lower_source_template_statement(
                        context,
                        body_statement,
                        values,
                        &body_aliases,
                        body_cache,
                        hints,
                        constraints,
                    )?;
                    collect_source_template_expression_alias(body_statement, &mut body_aliases);
                }
                return Ok(());
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
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
                    let mut loop_aliases = expression_aliases.clone();
                    values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                    for body_statement in loop_info.body_statements.iter() {
                        lower_source_template_statement(
                            context,
                            body_statement,
                            values,
                            &loop_aliases,
                            body_cache,
                            hints,
                            constraints,
                        )?;
                        collect_source_template_expression_alias(body_statement, &mut loop_aliases);
                    }
                }
                return Ok(());
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind != FunctionStatementKind::Expression {
        hints.push(lower_unsupported_source_template_statement(
            context.module,
            statement,
        ));
        return Ok(());
    }

    if let Some(kind) =
        source_statement_first_token_kind(context.module, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        if matches!(kind, TokenKind::AirValue | TokenKind::Commit) {
            return Ok(());
        }
        if matches!(
            kind,
            TokenKind::Include | TokenKind::Require | TokenKind::Use
        ) {
            hints.push(lower_unsupported_source_template_statement(
                context.module,
                statement,
            ));
            return Ok(());
        }
    }
    if source_expression_assigns_fixed_index(
        statement.value_expression.as_ref(),
        context.fixed_columns,
    ) {
        return Ok(());
    }
    if apply_source_static_expression_statement(
        context.program,
        statement.value_expression.as_ref(),
        values,
    ) {
        return Ok(());
    }
    if let Some(update) =
        source_static_postfix_update(context.module, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        if apply_source_static_delta(&update.name, update.delta, values) {
            return Ok(());
        }
        if source_static_name(context.module, &update.name) {
            return Ok(());
        }
        hints.push(lower_unsupported_source_assignment_statement(
            context.module,
            statement,
        ));
        return Ok(());
    }
    if source_static_assignment_expression(context.module, statement.value_expression.as_ref()) {
        return Ok(());
    }
    let expression_array_aliases = SourceExpressionArrayAliases::new();
    let lookup_inputs = SourceLookupInputs {
        program: context.program,
        module: context.module,
        values,
        expression_aliases,
        expression_array_aliases: &expression_array_aliases,
        scalar_slots: context.scalar_slots,
        opening_points: context.opening_points,
    };
    if let Some(hint) =
        lower_source_lookup_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(());
    }
    if source_expression_is_constrained_assignment(statement.value_expression.as_ref()) {
        let lowered = lower_source_template_boolean_constraint(
            context.program,
            context.module,
            statement,
            context.scalar_slots,
            values,
            expression_aliases,
        );
        match lowered {
            Ok(Some(constraint)) => constraints.push(constraint),
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                hints.push(lower_unsupported_source_assignment_statement(
                    context.module,
                    statement,
                ));
            }
            Err(error) => return Err(error),
        }
        return Ok(());
    }
    let contains_assignment_operator =
        source_statement_contains_assignment_operator(context.module, statement).map_err(
            |source| SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            },
        )?;
    if source_expression_is_assignment(statement.value_expression.as_ref())
        || contains_assignment_operator
    {
        hints.push(lower_unsupported_source_assignment_statement(
            context.module,
            statement,
        ));
        return Ok(());
    }
    match lower_source_template_boolean_constraint(
        context.program,
        context.module,
        statement,
        context.scalar_slots,
        values,
        expression_aliases,
    ) {
        Ok(Some(constraint)) => {
            constraints.push(constraint);
            return Ok(());
        }
        Ok(None)
            if source_expression_is_equality_constraint(statement.value_expression.as_ref()) =>
        {
            hints.push(lower_unsupported_source_constraint_statement(
                context.module,
                statement,
            ));
            return Ok(());
        }
        Ok(None) => {}
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
            hints.push(lower_unsupported_source_constraint_statement(
                context.module,
                statement,
            ));
            return Ok(());
        }
        Err(error) => return Err(error),
    }
    if lower_source_template_function_call(
        context,
        statement,
        values,
        expression_aliases,
        hints,
        constraints,
    )? {
        return Ok(());
    }
    if let Some(hint) =
        lower_unsupported_source_call_statement(context.module, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(());
    }
    unsupported(format!(
        "air template statements need constraint lowering support: {}",
        source_statement_line(context.module, statement)
    ))
}

fn lower_source_template_function_call(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
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
    let Some(mut bindings) = source_function_call_bindings(
        context.program,
        context.module,
        function,
        arguments,
        values,
        expression_aliases,
    ) else {
        return Ok(false);
    };

    let mut function_hints = Vec::new();
    let mut function_constraints = Vec::new();
    for body_statement in &function.statements {
        if !lower_source_function_body_statement(
            context,
            body_statement,
            &mut bindings.values,
            &bindings.expression_aliases,
            &bindings.expression_array_aliases,
            &mut function_hints,
            &mut function_constraints,
        )? {
            return Ok(false);
        }
    }

    hints.extend(function_hints);
    constraints.extend(function_constraints);
    Ok(true)
}

struct SourceFunctionCallBindings {
    values: BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: SourceExpressionAliases,
    expression_array_aliases: SourceExpressionArrayAliases,
}

fn source_function_call_bindings(
    program: &SourceProgram,
    module: &SourceProgramModule,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
) -> Option<SourceFunctionCallBindings> {
    let mut function_values = values.clone();
    let mut function_aliases = expression_aliases.clone();
    let mut function_array_aliases = SourceExpressionArrayAliases::new();
    for parameter in &function.parameters {
        if source_const_parameter(parameter) && parameter.array_dims.is_empty() {
            let Some(expression) = parameter.default_expression.as_ref() else {
                continue;
            };
            let value = evaluate_source_static_expression(program, expression, &function_values)?;
            function_values.insert(parameter.name.clone(), value);
            continue;
        }
        if source_const_parameter(parameter) {
            if let Some(span) = parameter.default_value {
                let elements =
                    source_static_array_literal(program, module, span, &function_values)?;
                insert_source_static_array(&mut function_values, &parameter.name, elements)?;
            }
            continue;
        }
        if source_expr_parameter(parameter) {
            if let Some(expression) = parameter.default_expression.as_ref() {
                function_aliases.insert(parameter.name.clone(), expression.clone());
            }
            continue;
        }
        if source_expr_array_parameter(parameter) {
            if let Some(expression) = parameter.default_expression.as_ref() {
                let alias = source_expression_array_alias(expression)?;
                function_array_aliases.insert(parameter.name.clone(), alias);
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
            source_bind_function_argument(
                program,
                parameter,
                &argument.value,
                &mut function_values,
                &mut function_aliases,
                &mut function_array_aliases,
            )?;
            continue;
        }
        let parameter = function.parameters.get(positional_index)?;
        source_bind_function_argument(
            program,
            parameter,
            &argument.value,
            &mut function_values,
            &mut function_aliases,
            &mut function_array_aliases,
        )?;
        positional_index += 1;
    }

    Some(SourceFunctionCallBindings {
        values: function_values,
        expression_aliases: function_aliases,
        expression_array_aliases: function_array_aliases,
    })
}

fn source_bind_function_argument(
    program: &SourceProgram,
    parameter: &lzvm_pil::FunctionParameter,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &mut SourceExpressionAliases,
    expression_array_aliases: &mut SourceExpressionArrayAliases,
) -> Option<()> {
    if source_const_parameter(parameter) && parameter.array_dims.is_empty() {
        let value = evaluate_source_static_expression(program, expression, values)?;
        values.insert(parameter.name.clone(), value);
        return Some(());
    }
    if source_expr_parameter(parameter) {
        expression_aliases.insert(parameter.name.clone(), expression.clone());
        return Some(());
    }
    if source_expr_array_parameter(parameter) {
        let alias = source_expression_array_alias(expression)?;
        expression_array_aliases.insert(parameter.name.clone(), alias);
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

fn source_const_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    parameter.is_const && !parameter.by_reference
}

fn source_expr_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    !parameter.is_const
        && !parameter.by_reference
        && parameter.array_dims.is_empty()
        && parameter.type_name == "expr"
}

fn source_expr_array_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    !parameter.is_const
        && !parameter.by_reference
        && !parameter.array_dims.is_empty()
        && parameter.type_name == "expr"
}

fn source_expression_array_alias(expression: &Expression) -> Option<SourceExpressionArrayAlias> {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(SourceExpressionArrayAlias::Name(name.clone())),
        ExpressionKind::Array(expressions) => {
            Some(SourceExpressionArrayAlias::Values(expressions.clone()))
        }
        _ => None,
    }
}

fn source_static_array_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Vec<FixedFileTemplateValue>> {
    let expression = strip_source_group_expression(expression);
    let ExpressionKind::Array(elements) = &expression.kind else {
        return None;
    };
    elements
        .iter()
        .map(|element| evaluate_source_static_expression(program, element, values))
        .collect()
}

fn source_static_array_literal(
    program: &SourceProgram,
    module: &SourceProgramModule,
    span: SourceSpan,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Vec<FixedFileTemplateValue>> {
    let contents = module.source.contents.get(span.start..span.end)?;
    let tokens = lex_source(contents).ok()?;
    if tokens.first()?.kind != TokenKind::LBracket {
        return None;
    }
    let close_index = tokens
        .iter()
        .position(|token| token.kind == TokenKind::RBracket)?;
    let ranges = source_top_level_ranges(&tokens, 0, close_index)?;
    let source = SourceFile {
        contents: contents.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::new(),
        source_name: module.source_name.clone(),
    };
    ranges
        .into_iter()
        .map(|range| {
            let (expression, consumed) =
                parse_expression_tokens(&tokens, range.0, range.1, &source).ok()?;
            if consumed != range.1 {
                return None;
            }
            evaluate_source_static_expression(program, &expression, values)
        })
        .collect()
}

fn source_top_level_ranges(
    tokens: &[Token],
    open_index: usize,
    close_index: usize,
) -> Option<Vec<(usize, usize)>> {
    if open_index >= close_index {
        return None;
    }
    let mut ranges = Vec::new();
    let mut start = open_index + 1;
    let mut expected = Vec::<TokenKind>::new();
    for (index, token) in tokens
        .iter()
        .enumerate()
        .take(close_index)
        .skip(open_index + 1)
    {
        match token.kind {
            TokenKind::LParen => expected.push(TokenKind::RParen),
            TokenKind::LBracket => expected.push(TokenKind::RBracket),
            TokenKind::LBrace => expected.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if expected.pop()? != token.kind {
                    return None;
                }
            }
            TokenKind::Comma if expected.is_empty() => {
                if start == index {
                    return None;
                }
                ranges.push((start, index));
                start = index + 1;
            }
            _ => {}
        }
    }
    if !expected.is_empty() {
        return None;
    }
    if start < close_index {
        ranges.push((start, close_index));
    }
    Some(ranges)
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

fn source_static_assertion(
    program: &SourceProgram,
    expression: Option<&Expression>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some((name, arguments)) = source_call_expression(expression) else {
        return false;
    };
    if name != "assert" || arguments.len() != 1 || arguments[0].name.is_some() {
        return false;
    }
    source_static_condition(program, &arguments[0].value, values).unwrap_or(false)
}

fn source_static_condition(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<bool> {
    if let Some(value) = evaluate_source_static_expression(program, expression, values) {
        return Some(source_static_truthy_value(&value));
    }
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_group_expression(expression).kind
    else {
        return None;
    };
    let left = source_static_integer_expression(program, left, values)?;
    let right = source_static_integer_expression(program, right, values)?;
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
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<i128> {
    let expression = strip_source_group_expression(expression);
    if let ExpressionKind::Call { callee, args } = &expression.kind {
        if args.len() == 1 && args[0].name.is_none() {
            if let ExpressionKind::Name(callee) = &strip_source_group_expression(callee).kind {
                if callee == "length" {
                    let ExpressionKind::Name(name) =
                        &strip_source_group_expression(&args[0].value).kind
                    else {
                        return None;
                    };
                    return source_static_array_length(values, name);
                }
            }
        }
    }
    let value = evaluate_source_static_expression(program, expression, values)?;
    source_static_integer_value(Some(&value))
}

fn strip_source_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_source_group_expression(inner),
        _ => expression,
    }
}

fn source_static_truthy_value(value: &FixedFileTemplateValue) -> bool {
    match value {
        FixedFileTemplateValue::Integer(value) => *value != 0,
        FixedFileTemplateValue::Boolean(value) => *value,
        FixedFileTemplateValue::String(value) => !value.is_empty(),
    }
}

fn lower_source_function_body_statement(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        return Ok(apply_source_static_declaration(
            context.program,
            statement,
            values,
        ));
    }
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(false);
    }
    if apply_source_static_expression_statement(
        context.program,
        statement.value_expression.as_ref(),
        values,
    ) || source_static_assertion(context.program, statement.value_expression.as_ref(), values)
    {
        return Ok(true);
    }
    let lookup_inputs = SourceLookupInputs {
        program: context.program,
        module: context.module,
        values,
        expression_aliases,
        expression_array_aliases,
        scalar_slots: context.scalar_slots,
        opening_points: context.opening_points,
    };
    if let Some(hint) =
        lower_source_lookup_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(true);
    }
    match lower_source_template_boolean_constraint(
        context.program,
        context.module,
        statement,
        context.scalar_slots,
        values,
        expression_aliases,
    ) {
        Ok(Some(constraint)) => {
            constraints.push(constraint);
            Ok(true)
        }
        Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn apply_source_static_declaration(
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

fn apply_source_static_expression_statement(
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
            apply_source_static_delta(name, delta, values)
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
                    let Some(current) = source_static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = source_static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_add(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                BinaryOperator::MinusAssign => {
                    let Some(current) = source_static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = source_static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_sub(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                BinaryOperator::StarAssign => {
                    let Some(current) = source_static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = source_static_integer_value(Some(&right)) else {
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

struct SourceStaticPostfixUpdate {
    name: String,
    delta: i128,
}

fn source_static_postfix_update(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
) -> Result<Option<SourceStaticPostfixUpdate>, lzvm_pil::LexError> {
    let text = &module.source.contents[statement.start..statement.end];
    let tokens = lex_source(text)?;
    let tokens = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::EndOfInput)
        .collect::<Vec<_>>();
    let (name, update) = match tokens.as_slice() {
        [name, update] => (*name, *update),
        [name, update, semicolon] if semicolon.kind == TokenKind::Semicolon => (*name, *update),
        _ => return Ok(None),
    };
    if name.kind != TokenKind::Identifier {
        return Ok(None);
    }
    let delta = match update.kind {
        TokenKind::Increment => 1,
        TokenKind::Decrement => -1,
        _ => return Ok(None),
    };
    Ok(Some(SourceStaticPostfixUpdate {
        name: name.lexeme.clone(),
        delta,
    }))
}

fn apply_source_static_delta(
    name: &str,
    delta: i128,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(current) = source_static_integer_value(values.get(name)) else {
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

fn source_static_integer_value(value: Option<&FixedFileTemplateValue>) -> Option<i128> {
    match value {
        Some(FixedFileTemplateValue::Integer(value)) => Some(*value),
        Some(FixedFileTemplateValue::Boolean(value)) => Some(if *value { 1 } else { 0 }),
        _ => None,
    }
}

fn source_static_name(module: &SourceProgramModule, name: &str) -> bool {
    module
        .constants
        .iter()
        .any(|declaration| declaration.name == name)
        || module
            .variables
            .iter()
            .any(|declaration| declaration.name == name)
}

fn unsupported<T>(message: impl Into<String>) -> Result<T, SourceKeyDirectoryMetadataError> {
    Err(unsupported_source_message(message))
}

fn unsupported_source_message(message: impl Into<String>) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}

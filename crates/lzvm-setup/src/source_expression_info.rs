#![allow(clippy::too_many_arguments)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use lzvm_artifacts::expression_info::{ConstraintCode, ExpressionInfo, HintInfo};
use lzvm_artifacts::global_info::{NamedStageValue, PublicValue};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_pil::{
    lex_source, parse_expression_tokens, BinaryOperator, CallArgument, Expression, ExpressionKind,
    FixedFileTemplateValue, FunctionDeclaration, FunctionStatement, FunctionStatementDeclaration,
    FunctionStatementKind, SourceFile, SourceProgram, SourceProgramModule, SourceSpan, Token,
    TokenKind,
};

use crate::{
    source_constraint_lowering::{
        lower_source_template_boolean_constraint,
        lower_source_template_boolean_constraint_with_returned_calls, SourceExpressionAliases,
    },
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_expression_aliases::{
        insert_source_expression_array_alias_binding, source_alias_binding_name,
        source_expression_alias_assignment_target, source_template_expression_alias_can_apply,
    },
    source_expression_filters::{
        source_expression_assigns_fixed_index, source_expression_is_assignment,
        source_expression_is_constrained_assignment, source_expression_is_equality_constraint,
    },
    source_expression_return_arrays::{
        source_returned_expression_array_alias, source_returned_expression_array_call_alias,
    },
    source_expression_return_values::{
        collect_source_template_expression_aliases_with_stack,
        collect_source_template_expression_aliases_with_static_state,
        insert_source_expr_array_alias_length, source_expression_may_resolve,
        source_import_returned_expression_calls, source_resolved_expression_value,
        source_resolved_expression_value_without_returned_calls,
    },
    source_expression_statements::{
        apply_source_expression_string_assignment, apply_source_expression_string_declaration,
        apply_source_static_array_assignment_statement, apply_source_static_declaration,
        apply_source_static_expression_statement,
    },
    source_expression_static_assertions::source_static_assertion,
    source_expression_template_values::source_expression_template_values,
    source_expression_units::{
        source_expression_template_instances, source_expression_unit_instances,
        source_fixed_assignment_column_names,
    },
    source_final_calls::{source_final_statement_call, SourceFinalScope},
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_range_check_hints::{lower_source_range_check_statement, SourceRangeCheckIds},
    source_scalar_slots::{SourceChallengeSlotMetadata, SourceScalarSlots},
    source_statement_hints::{
        lower_source_annotation_statement, lower_source_arith_helper_statement,
        lower_source_assignment_statement, lower_source_lookup_statement,
        lower_source_memory_helper_statement, lower_source_operation_helper_statement,
        lower_unsupported_source_assignment_statement, lower_unsupported_source_call_statement,
        lower_unsupported_source_constraint_statement, lower_unsupported_source_template_statement,
        source_statement_contains_assignment_operator, source_statement_first_token_kind,
        source_statement_is_source_directive, source_statement_line, SourceExpressionArrayAlias,
        SourceExpressionArrayAliases, SourceLookupInputs,
    },
    source_static_values::{
        evaluate_source_static_expression, execute_static_template_range,
        insert_source_static_array, source_active_static_name, source_static_array_length,
        source_static_array_values, source_static_assignment_expression, static_value_truthy,
        SourceTemplateConstantValueCache,
    },
    source_template_context::SourceTemplateLoweringContext,
    source_template_do_while::{source_static_do_while_loop_with_tokens, SourceStaticDoWhileLoop},
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_aliases,
    source_template_switch::source_static_switch_body_statements,
    source_template_while::{
        source_static_while_loop_with_tokens, SourceStaticWhileLoop, STATIC_WHILE_LOOP_LIMIT,
    },
};

#[derive(Clone, Debug, Default)]
pub(crate) struct SourceExpressionAliasScope {
    pub(crate) expressions: Arc<SourceExpressionAliases>,
    pub(crate) expression_arrays: Arc<SourceExpressionArrayAliases>,
}

impl SourceExpressionAliasScope {
    fn from_maps(
        expressions: SourceExpressionAliases,
        expression_arrays: SourceExpressionArrayAliases,
    ) -> Self {
        Self {
            expressions: Arc::new(expressions),
            expression_arrays: Arc::new(expression_arrays),
        }
    }

    pub(crate) fn expressions_mut(&mut self) -> &mut SourceExpressionAliases {
        Arc::make_mut(&mut self.expressions)
    }

    pub(crate) fn expression_arrays_mut(&mut self) -> &mut SourceExpressionArrayAliases {
        Arc::make_mut(&mut self.expression_arrays)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceTemplateStatementFlow {
    Fallthrough,
    Break,
    Continue,
}

pub(crate) fn source_expression_info(
    program: &SourceProgram,
    setup: &UnitSetupInfo,
    group_name: Option<&str>,
    unit_name: Option<&str>,
    publics: &[PublicValue],
    challenges: &[SourceChallengeSlotMetadata],
    proof_values: &[NamedStageValue],
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
    range_checks: &RefCell<SourceRangeCheckIds>,
) -> Result<ExpressionInfo, SourceKeyDirectoryMetadataError> {
    let scalar_slots = SourceScalarSlots::from_setup(setup, publics, challenges, proof_values)
        .map_err(|error| unsupported_source_message(error.to_string()))?;
    let unit_instances = source_expression_unit_instances(program, group_name, unit_name);
    let fixed_assignment_columns = source_fixed_assignment_column_names(
        program,
        active_templates,
        constant_values,
        template_values,
    );
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
            let template_instances =
                source_expression_template_instances(unit_instances.as_deref(), &template.name);
            if template_instances.is_empty() {
                continue;
            }
            for unit_instance in template_instances {
                let context = SourceTemplateLoweringContext {
                    program,
                    module,
                    tokens: &tokens,
                    scalar_slots: &scalar_slots,
                    opening_points: &setup.opening_points,
                    fixed_columns: &fixed_assignment_columns,
                    range_checks,
                    active_templates,
                    constant_values,
                    template_values,
                    final_air_calls_enabled: unit_instance.is_some(),
                };
                let mut alias_scope = SourceExpressionAliasScope::default();
                let mut statement_values = source_expression_template_values(
                    context.program,
                    context.module,
                    template,
                    unit_instance,
                    context.constant_values,
                    context.template_values,
                );
                let mut alias_values = statement_values.clone();
                for statement in &template.statements {
                    let flow = lower_source_template_statement(
                        &context,
                        statement,
                        &mut statement_values,
                        &alias_scope,
                        body_cache,
                        &mut hints,
                        &mut constraints,
                    )?;
                    if flow != SourceTemplateStatementFlow::Fallthrough {
                        return Err(unsupported_source_message(
                            "source control statement outside static loop",
                        ));
                    }
                    collect_source_template_expression_aliases_with_static_state(
                        &context,
                        statement,
                        &mut alias_values,
                        body_cache,
                        &mut alias_scope,
                    );
                    sync_source_alias_static_lengths(&mut statement_values, &alias_values);
                }
                lower_source_template_final_air_calls(
                    &context,
                    &template.statements,
                    &mut statement_values,
                    &alias_scope,
                    body_cache,
                    &mut hints,
                    &mut constraints,
                )?;
            }
        }
    }
    Ok(ExpressionInfo {
        hints,
        expressions: Vec::new(),
        constraints,
    })
}

fn lower_source_template_statement(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
) -> Result<SourceTemplateStatementFlow, SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        if apply_source_static_container_statement(context, statement, values)? {
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        if !apply_source_expression_string_declaration(context, statement, values, alias_scope) {
            apply_source_static_declaration(context.program, statement, values);
        }
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if statement.kind == FunctionStatementKind::If {
        match source_static_if_body_statements_with_aliases(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            &alias_scope.expressions,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                let mut body_alias_scope = alias_scope.clone();
                let mut body_alias_values = values.clone();
                for body_statement in body_statements.iter() {
                    let flow = lower_source_template_statement(
                        context,
                        body_statement,
                        values,
                        &body_alias_scope,
                        body_cache,
                        hints,
                        constraints,
                    )?;
                    if flow != SourceTemplateStatementFlow::Fallthrough {
                        return Ok(flow);
                    }
                    collect_source_template_expression_aliases_with_static_state(
                        context,
                        body_statement,
                        &mut body_alias_values,
                        body_cache,
                        &mut body_alias_scope,
                    );
                    sync_source_alias_static_lengths(values, &body_alias_values);
                }
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
                return Ok(SourceTemplateStatementFlow::Fallthrough);
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
                let mut loop_alias_scope = alias_scope.clone();
                let mut loop_alias_values = values.clone();
                for iteration_value in &loop_info.iteration_values {
                    values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                    loop_alias_values
                        .insert(loop_info.variable_name.clone(), iteration_value.clone());
                    for body_statement in loop_info.body_statements.iter() {
                        let flow = lower_source_template_statement(
                            context,
                            body_statement,
                            values,
                            &loop_alias_scope,
                            body_cache,
                            hints,
                            constraints,
                        )?;
                        match flow {
                            SourceTemplateStatementFlow::Fallthrough => {}
                            SourceTemplateStatementFlow::Continue => break,
                            SourceTemplateStatementFlow::Break => {
                                return Ok(SourceTemplateStatementFlow::Fallthrough);
                            }
                        }
                        collect_source_template_expression_aliases_with_static_state(
                            context,
                            body_statement,
                            &mut loop_alias_values,
                            body_cache,
                            &mut loop_alias_scope,
                        );
                        sync_source_alias_static_lengths(values, &loop_alias_values);
                    }
                }
                loop_info.apply_final_variable_value(values);
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind == FunctionStatementKind::While {
        if statement.body.is_none() {
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        match source_static_while_loop_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                if let Some(flow) = lower_source_static_while_template_statement(
                    context,
                    loop_info,
                    values,
                    alias_scope,
                    body_cache,
                    hints,
                    constraints,
                )? {
                    return Ok(flow);
                }
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind == FunctionStatementKind::Do {
        match source_static_do_while_loop_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                if let Some(flow) = lower_source_static_do_while_template_statement(
                    context,
                    loop_info,
                    values,
                    alias_scope,
                    body_cache,
                    hints,
                    constraints,
                )? {
                    return Ok(flow);
                }
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind == FunctionStatementKind::Switch {
        match source_static_switch_body_statements(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                let mut switch_alias_scope = alias_scope.clone();
                let mut switch_alias_values = values.clone();
                for body_statement in body_statements.iter() {
                    let flow = lower_source_template_statement(
                        context,
                        body_statement,
                        values,
                        &switch_alias_scope,
                        body_cache,
                        hints,
                        constraints,
                    )?;
                    match flow {
                        SourceTemplateStatementFlow::Fallthrough => {}
                        SourceTemplateStatementFlow::Break => {
                            return Ok(SourceTemplateStatementFlow::Fallthrough);
                        }
                        SourceTemplateStatementFlow::Continue => {
                            return Ok(SourceTemplateStatementFlow::Continue);
                        }
                    }
                    collect_source_template_expression_aliases_with_static_state(
                        context,
                        body_statement,
                        &mut switch_alias_values,
                        body_cache,
                        &mut switch_alias_scope,
                    );
                    sync_source_alias_static_lengths(values, &switch_alias_values);
                }
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                hints.push(lower_unsupported_source_template_statement(
                    context.module,
                    statement,
                ));
                return Ok(SourceTemplateStatementFlow::Fallthrough);
            }
            Err(error) => return Err(error),
        }
    }
    if source_statement_is_source_directive(context.module, statement).map_err(|source| {
        SourceKeyDirectoryMetadataError::Lex {
            source_name: context.module.source_name.clone(),
            source,
        }
    })? {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if statement.kind == FunctionStatementKind::Break {
        return Ok(SourceTemplateStatementFlow::Break);
    }
    if statement.kind == FunctionStatementKind::Continue {
        return Ok(SourceTemplateStatementFlow::Continue);
    }
    if let Some(call) = source_final_statement_call(context.tokens, context.module, statement)? {
        match call.scope {
            SourceFinalScope::Proof => return Ok(SourceTemplateStatementFlow::Fallthrough),
            SourceFinalScope::Air => return Ok(SourceTemplateStatementFlow::Fallthrough),
            SourceFinalScope::AirGroup => return Ok(SourceTemplateStatementFlow::Fallthrough),
        }
    }
    if statement.kind != FunctionStatementKind::Expression {
        hints.push(lower_unsupported_source_template_statement(
            context.module,
            statement,
        ));
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if let Some(kind) =
        source_statement_first_token_kind(context.module, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        if matches!(
            kind,
            TokenKind::AirGroupValue
                | TokenKind::AirValue
                | TokenKind::Commit
                | TokenKind::Public
                | TokenKind::ProofValue
                | TokenKind::Challenge
        ) {
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
    }
    if source_expression_assigns_fixed_index(
        statement.value_expression.as_ref(),
        context.fixed_columns,
    ) {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if source_noop_call_statement(statement) {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if apply_source_static_expression_statement(
        context.program,
        statement.value_expression.as_ref(),
        values,
    ) {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if apply_source_expression_string_assignment(context, statement, values, alias_scope) {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if apply_source_static_array_assignment_statement(context, statement, values) {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if source_static_assertion(context.program, context.module, statement, values)? {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
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
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        if source_active_static_name(
            context.program,
            context.module,
            context.active_templates,
            &update.name,
            context.constant_values,
            context.template_values,
        ) {
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        hints.push(lower_unsupported_source_assignment_statement(
            context.module,
            statement,
        ));
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if source_template_expression_alias_can_apply(statement, &alias_scope.expressions) {
        if let Some(name) =
            source_expression_alias_assignment_target(statement.value_expression.as_ref())
        {
            values.remove(name);
        }
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if source_static_assignment_expression(
        context.program,
        context.module,
        context.active_templates,
        statement.value_expression.as_ref(),
        context.constant_values,
        context.template_values,
    ) {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    let lookup_inputs = SourceLookupInputs {
        program: context.program,
        module: context.module,
        values,
        constant_values: context.constant_values,
        expression_aliases: &alias_scope.expressions,
        expression_array_aliases: &alias_scope.expression_arrays,
        scalar_slots: context.scalar_slots,
        opening_points: context.opening_points,
    };
    let range_hints =
        lower_source_range_check_statement(&lookup_inputs, context.range_checks, statement)
            .map_err(|source| SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            })?;
    if !range_hints.is_empty() {
        hints.extend(range_hints);
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if let Some(hint) =
        lower_source_lookup_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if let Some(hint) =
        lower_source_memory_helper_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if let Some(hint) =
        lower_source_operation_helper_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if let Some(hint) =
        lower_source_arith_helper_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if let Some(hint) =
        lower_source_annotation_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if matches!(
        source_statement_first_token_kind(context.module, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?,
        Some(TokenKind::AtIdentifier)
    ) {
        hints.push(lower_unsupported_source_template_statement(
            context.module,
            statement,
        ));
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    if source_expression_is_constrained_assignment(statement.value_expression.as_ref()) {
        let mut call_stack = BTreeSet::new();
        let resolved_statement = source_statement_with_static_resolved_expression(
            context,
            statement,
            values,
            alias_scope,
            body_cache,
            &mut call_stack,
        );
        let lowering_statement = resolved_statement.as_ref().unwrap_or(statement);
        let lowered = lower_source_template_boolean_constraint(
            context.program,
            context.module,
            lowering_statement,
            context.scalar_slots,
            values,
            alias_scope,
        );
        match lowered {
            Ok(Some(constraint)) => constraints.push(constraint),
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                let mut call_stack = BTreeSet::new();
                let direct_lowered = lower_source_template_boolean_constraint_with_returned_calls(
                    context,
                    statement,
                    values,
                    alias_scope,
                    body_cache,
                    &mut call_stack,
                );
                match direct_lowered {
                    Ok(Some(constraint)) => constraints.push(constraint),
                    Ok(None) => {
                        hints.push(lower_unsupported_source_assignment_statement(
                            context.module,
                            statement,
                        ));
                    }
                    Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                        hints.push(lower_unsupported_source_assignment_statement(
                            context.module,
                            statement,
                        ));
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        }
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    let is_assignment = source_expression_is_assignment(statement.value_expression.as_ref());
    if is_assignment {
        if source_expression_array_alias_assignment_can_apply(
            context.program,
            statement.value_expression.as_ref(),
            values,
            &alias_scope.expression_arrays,
        ) {
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        if let Some(hint) =
            lower_source_assignment_statement(&lookup_inputs, statement).map_err(|source| {
                SourceKeyDirectoryMetadataError::Lex {
                    source_name: context.module.source_name.clone(),
                    source,
                }
            })?
        {
            hints.push(hint);
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
    }
    let contains_assignment_operator =
        source_statement_contains_assignment_operator(context.module, statement).map_err(
            |source| SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            },
        )?;
    if is_assignment || contains_assignment_operator {
        hints.push(lower_unsupported_source_assignment_statement(
            context.module,
            statement,
        ));
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    let mut call_stack = BTreeSet::new();
    let resolved_statement = source_statement_with_static_resolved_expression(
        context,
        statement,
        values,
        alias_scope,
        body_cache,
        &mut call_stack,
    );
    let lowering_statement = resolved_statement.as_ref().unwrap_or(statement);
    let fallback_lowered = lower_source_template_boolean_constraint(
        context.program,
        context.module,
        lowering_statement,
        context.scalar_slots,
        values,
        alias_scope,
    );
    let mut unsupported_constraint = false;
    match fallback_lowered {
        Ok(Some(constraint)) => {
            constraints.push(constraint);
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        Ok(None)
            if source_expression_is_equality_constraint(statement.value_expression.as_ref()) =>
        {
            unsupported_constraint = true;
        }
        Ok(None) => {}
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
            unsupported_constraint = true;
        }
        Err(error) => return Err(error),
    }
    let mut call_stack = BTreeSet::new();
    let mut output = SourceTemplateFunctionOutput { hints, constraints };
    if lower_source_template_function_call(
        context,
        statement,
        values,
        alias_scope,
        body_cache,
        &mut call_stack,
        &mut output,
        None,
    )? {
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    let direct_lowered = lower_source_template_boolean_constraint_with_returned_calls(
        context,
        statement,
        values,
        alias_scope,
        body_cache,
        &mut call_stack,
    );
    match direct_lowered {
        Ok(Some(constraint)) => {
            constraints.push(constraint);
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        Ok(None) => {}
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
            unsupported_constraint = true;
        }
        Err(error) => return Err(error),
    }
    if unsupported_constraint {
        hints.push(lower_unsupported_source_constraint_statement(
            context.module,
            statement,
        ));
        return Ok(SourceTemplateStatementFlow::Fallthrough);
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
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    unsupported(format!(
        "air template statements need constraint lowering support: {}",
        source_statement_line(context.module, statement)
    ))
}

fn lower_source_static_while_template_statement(
    context: &SourceTemplateLoweringContext<'_>,
    loop_info: SourceStaticWhileLoop,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
) -> Result<Option<SourceTemplateStatementFlow>, SourceKeyDirectoryMetadataError> {
    let mut loop_values = values.clone();
    let mut loop_alias_scope = alias_scope.clone();
    let mut loop_alias_values = values.clone();
    let mut loop_hints = Vec::new();
    let mut loop_constraints = Vec::new();
    for _ in 0..STATIC_WHILE_LOOP_LIMIT {
        let Some(condition_value) =
            evaluate_source_static_expression(context.program, &loop_info.condition, &loop_values)
        else {
            return Ok(None);
        };
        if !static_value_truthy(&condition_value) {
            *values = loop_values;
            hints.extend(loop_hints);
            constraints.extend(loop_constraints);
            return Ok(Some(SourceTemplateStatementFlow::Fallthrough));
        }
        for body_statement in loop_info.body_statements.iter() {
            let flow = lower_source_template_statement(
                context,
                body_statement,
                &mut loop_values,
                &loop_alias_scope,
                body_cache,
                &mut loop_hints,
                &mut loop_constraints,
            )?;
            match flow {
                SourceTemplateStatementFlow::Fallthrough => {}
                SourceTemplateStatementFlow::Continue => break,
                SourceTemplateStatementFlow::Break => {
                    *values = loop_values;
                    hints.extend(loop_hints);
                    constraints.extend(loop_constraints);
                    return Ok(Some(SourceTemplateStatementFlow::Fallthrough));
                }
            }
            collect_source_template_expression_aliases_with_static_state(
                context,
                body_statement,
                &mut loop_alias_values,
                body_cache,
                &mut loop_alias_scope,
            );
            sync_source_alias_static_lengths(&mut loop_values, &loop_alias_values);
        }
    }
    Ok(None)
}

fn lower_source_static_do_while_template_statement(
    context: &SourceTemplateLoweringContext<'_>,
    loop_info: SourceStaticDoWhileLoop,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
) -> Result<Option<SourceTemplateStatementFlow>, SourceKeyDirectoryMetadataError> {
    let mut loop_values = values.clone();
    let mut loop_alias_scope = alias_scope.clone();
    let mut loop_alias_values = values.clone();
    let mut loop_hints = Vec::new();
    let mut loop_constraints = Vec::new();
    for _ in 0..STATIC_WHILE_LOOP_LIMIT {
        for body_statement in loop_info.body_statements.iter() {
            let flow = lower_source_template_statement(
                context,
                body_statement,
                &mut loop_values,
                &loop_alias_scope,
                body_cache,
                &mut loop_hints,
                &mut loop_constraints,
            )?;
            match flow {
                SourceTemplateStatementFlow::Fallthrough => {}
                SourceTemplateStatementFlow::Continue => break,
                SourceTemplateStatementFlow::Break => {
                    *values = loop_values;
                    hints.extend(loop_hints);
                    constraints.extend(loop_constraints);
                    return Ok(Some(SourceTemplateStatementFlow::Fallthrough));
                }
            }
            collect_source_template_expression_aliases_with_static_state(
                context,
                body_statement,
                &mut loop_alias_values,
                body_cache,
                &mut loop_alias_scope,
            );
            sync_source_alias_static_lengths(&mut loop_values, &loop_alias_values);
        }
        let Some(condition_value) =
            evaluate_source_static_expression(context.program, &loop_info.condition, &loop_values)
        else {
            return Ok(None);
        };
        if !static_value_truthy(&condition_value) {
            *values = loop_values;
            hints.extend(loop_hints);
            constraints.extend(loop_constraints);
            return Ok(Some(SourceTemplateStatementFlow::Fallthrough));
        }
    }
    Ok(None)
}

fn sync_source_alias_static_lengths(
    target: &mut BTreeMap<String, FixedFileTemplateValue>,
    source: &BTreeMap<String, FixedFileTemplateValue>,
) {
    for (key, value) in source {
        if key.starts_with("__lzvm_array_len::") {
            target.insert(key.clone(), value.clone());
        }
    }
}

struct SourceTemplateFinalAirCall {
    source_line: String,
    priority: i128,
    source_order: usize,
    expression: Expression,
}

struct SourceTemplateFinalAirQueue {
    calls: Vec<SourceTemplateFinalAirCall>,
    current_priority: Option<i128>,
    next_source_order: usize,
}

impl SourceTemplateFinalAirQueue {
    fn new(next_source_order: usize) -> Self {
        Self {
            calls: Vec::new(),
            current_priority: None,
            next_source_order,
        }
    }

    fn push_call(
        &mut self,
        context: &SourceTemplateLoweringContext<'_>,
        statement: &FunctionStatement,
        call: crate::source_final_calls::SourceFinalCall,
        values: &BTreeMap<String, FixedFileTemplateValue>,
        source_order: usize,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        let priority =
            source_final_air_priority(context, statement, call.priority.as_ref(), values)?;
        self.calls.push(SourceTemplateFinalAirCall {
            source_line: source_statement_line(context.module, statement),
            priority,
            source_order,
            expression: call.expression,
        });
        Ok(())
    }

    fn push_reentrant_call(
        &mut self,
        context: &SourceTemplateLoweringContext<'_>,
        statement: &FunctionStatement,
        call: crate::source_final_calls::SourceFinalCall,
        values: &BTreeMap<String, FixedFileTemplateValue>,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        let priority =
            source_final_air_priority(context, statement, call.priority.as_ref(), values)?;
        if self
            .current_priority
            .is_some_and(|current_priority| priority >= current_priority)
        {
            return Ok(());
        }
        let source_order = self.next_source_order;
        self.next_source_order = self.next_source_order.saturating_add(1);
        self.calls.push(SourceTemplateFinalAirCall {
            source_line: source_statement_line(context.module, statement),
            priority,
            source_order,
            expression: call.expression,
        });
        Ok(())
    }

    fn pop_next(&mut self) -> Option<SourceTemplateFinalAirCall> {
        self.calls.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.source_order.cmp(&right.source_order))
        });
        if self.calls.is_empty() {
            None
        } else {
            Some(self.calls.remove(0))
        }
    }
}

fn source_final_air_priority(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    priority: Option<&Expression>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<i128, SourceKeyDirectoryMetadataError> {
    let Some(expression) = priority else {
        return Ok(0);
    };
    let Some(value) = evaluate_source_static_expression(context.program, expression, values) else {
        return unsupported(format!(
            "source final air priority needs static expression: {}",
            source_statement_line(context.module, statement)
        ));
    };
    let Some(priority) = source_static_integer_value(Some(&value)) else {
        return unsupported(format!(
            "source final air priority needs integer expression: {}",
            source_statement_line(context.module, statement)
        ));
    };
    Ok(priority)
}

fn lower_source_template_final_air_calls(
    context: &SourceTemplateLoweringContext<'_>,
    statements: &[FunctionStatement],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if !context.final_air_calls_enabled {
        return Ok(());
    }
    let mut queue = SourceTemplateFinalAirQueue::new(statements.len());
    for (source_order, statement) in statements.iter().enumerate() {
        let Some(call) = source_final_statement_call(context.tokens, context.module, statement)?
        else {
            continue;
        };
        if call.scope != SourceFinalScope::Air {
            continue;
        }
        queue.push_call(context, statement, call, values, source_order)?;
    }
    while let Some(call) = queue.pop_next() {
        let mut call_stack = BTreeSet::new();
        let mut output = SourceTemplateFunctionOutput { hints, constraints };
        queue.current_priority = Some(call.priority);
        if !lower_source_template_function_call_expression(
            context,
            &call.expression,
            values,
            alias_scope,
            body_cache,
            &mut call_stack,
            &mut output,
            true,
            Some(&mut queue),
        )? {
            queue.current_priority = None;
            return unsupported(format!(
                "air template statements need constraint lowering support: {}",
                call.source_line
            ));
        }
        queue.current_priority = None;
    }
    Ok(())
}

fn lower_source_template_function_call(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    output: &mut SourceTemplateFunctionOutput<'_>,
    final_air_queue: Option<&mut SourceTemplateFinalAirQueue>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(false);
    };
    lower_source_template_function_call_expression(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        output,
        false,
        final_air_queue,
    )
}

fn lower_source_template_function_call_expression(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    output: &mut SourceTemplateFunctionOutput<'_>,
    propagate_shared_values: bool,
    mut final_air_queue: Option<&mut SourceTemplateFinalAirQueue>,
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
    let shared_values = if propagate_shared_values {
        source_function_shared_static_values(values, function)
    } else {
        BTreeSet::new()
    };
    let Some(mut bindings) = source_function_call_bindings(
        context,
        function,
        arguments,
        values,
        alias_scope,
        body_cache,
        call_stack,
    ) else {
        return Ok(false);
    };

    if !call_stack.insert(function.name.clone()) {
        return Ok(false);
    }
    let mut function_hints = Vec::new();
    let mut function_constraints = Vec::new();
    let mut function_output = SourceTemplateFunctionOutput {
        hints: &mut function_hints,
        constraints: &mut function_constraints,
    };
    let mut body_alias_scope = bindings.alias_scope;
    let lowered: Result<bool, SourceKeyDirectoryMetadataError> = (|| {
        for body_statement in &function.statements {
            if !lower_source_function_body_statement(
                context,
                body_statement,
                &mut bindings.values,
                &body_alias_scope,
                body_cache,
                call_stack,
                &mut function_output,
                final_air_queue.as_deref_mut(),
            )? {
                return Ok(false);
            }
            collect_source_template_expression_aliases_with_stack(
                context,
                body_statement,
                &mut bindings.values,
                body_cache,
                call_stack,
                &mut body_alias_scope,
            );
        }
        Ok(true)
    })();
    call_stack.remove(&function.name);
    if !lowered? {
        return Ok(false);
    }

    output.hints.extend(function_hints);
    output.constraints.extend(function_constraints);
    for name in shared_values {
        if let Some(value) = bindings.values.get(&name).cloned() {
            values.insert(name, value);
        }
    }
    Ok(true)
}

fn source_noop_call_statement(statement: &FunctionStatement) -> bool {
    source_call_expression(statement.value_expression.as_ref()).is_some_and(|(name, _)| {
        matches!(
            name,
            "println" | "Tables.fill" | "Tables.copy" | "Tables.print"
        )
    })
}

fn source_statement_with_resolved_expression(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    resolve_aliases: bool,
) -> Option<FunctionStatement> {
    let expression = statement.value_expression.as_ref()?;
    if !source_expression_may_resolve(
        context.program,
        context.module,
        expression,
        values,
        alias_scope,
        resolve_aliases,
        true,
    ) {
        return None;
    }
    let resolved = source_resolved_expression_value(
        context,
        expression,
        values,
        alias_scope,
        body_cache,
        call_stack,
        resolve_aliases,
    )?;
    if &resolved == expression {
        return None;
    }
    let mut statement = statement.clone();
    statement.value_expression = Some(resolved);
    Some(statement)
}

fn source_statement_with_static_resolved_expression(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<FunctionStatement> {
    let expression = statement.value_expression.as_ref()?;
    if matches!(
        &strip_source_group_expression(expression).kind,
        ExpressionKind::Call { .. }
    ) {
        return None;
    }
    let mut alias_names = BTreeSet::new();
    collect_source_expression_referenced_aliases(expression, alias_scope, &mut alias_names);
    let filtered_values;
    let values = if alias_names.is_empty() {
        values
    } else {
        filtered_values = source_values_without_alias_names(values, &alias_names);
        &filtered_values
    };
    source_statement_with_resolved_expression(
        context,
        statement,
        values,
        alias_scope,
        body_cache,
        call_stack,
        false,
    )
}

fn collect_source_expression_referenced_aliases(
    expression: &Expression,
    alias_scope: &SourceExpressionAliasScope,
    names: &mut BTreeSet<String>,
) {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Name(name) => {
            if alias_scope.expressions.contains_key(name.as_str())
                || alias_scope.expression_arrays.contains_key(name.as_str())
            {
                names.insert(name.clone());
            }
            let binding_name = source_alias_binding_name(name);
            if binding_name != name
                && (alias_scope.expressions.contains_key(binding_name)
                    || alias_scope.expression_arrays.contains_key(binding_name))
            {
                names.insert(binding_name.to_owned());
            }
        }
        ExpressionKind::Group(inner) => {
            collect_source_expression_referenced_aliases(inner, alias_scope, names);
        }
        ExpressionKind::Array(expressions) => {
            for expression in expressions {
                collect_source_expression_referenced_aliases(expression, alias_scope, names);
            }
        }
        ExpressionKind::Unary { expr, .. } => {
            collect_source_expression_referenced_aliases(expr, alias_scope, names);
        }
        ExpressionKind::Binary { left, right, .. } => {
            collect_source_expression_referenced_aliases(left, alias_scope, names);
            collect_source_expression_referenced_aliases(right, alias_scope, names);
        }
        ExpressionKind::Call { callee, args } => {
            collect_source_expression_referenced_aliases(callee, alias_scope, names);
            for arg in args {
                collect_source_expression_referenced_aliases(&arg.value, alias_scope, names);
            }
        }
        ExpressionKind::Index { target, index } => {
            collect_source_expression_referenced_aliases(target, alias_scope, names);
            collect_source_expression_referenced_aliases(index, alias_scope, names);
        }
        ExpressionKind::RowOffset { target, offset, .. } => {
            collect_source_expression_referenced_aliases(target, alias_scope, names);
            collect_source_expression_referenced_aliases(offset, alias_scope, names);
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            collect_source_expression_referenced_aliases(condition, alias_scope, names);
            collect_source_expression_referenced_aliases(then_expr, alias_scope, names);
            collect_source_expression_referenced_aliases(else_expr, alias_scope, names);
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => {}
    }
}

fn source_values_without_alias_names(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_names: &BTreeSet<String>,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut filtered = values.clone();
    for name in alias_names {
        filtered.remove(name);
        let binding_name = source_alias_binding_name(name);
        if binding_name != name {
            filtered.remove(binding_name);
        }
    }
    filtered
}

fn source_function_shared_static_values(
    values: &BTreeMap<String, FixedFileTemplateValue>,
    function: &FunctionDeclaration,
) -> BTreeSet<String> {
    let parameters = function
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<BTreeSet<_>>();
    values
        .keys()
        .filter(|name| !parameters.contains(name.as_str()))
        .cloned()
        .collect()
}

struct SourceTemplateFunctionOutput<'a> {
    hints: &'a mut Vec<HintInfo>,
    constraints: &'a mut Vec<ConstraintCode>,
}

pub(crate) struct SourceFunctionCallBindings {
    pub(crate) values: BTreeMap<String, FixedFileTemplateValue>,
    pub(crate) alias_scope: SourceExpressionAliasScope,
}

pub(crate) fn source_function_call_bindings(
    context: &SourceTemplateLoweringContext<'_>,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<SourceFunctionCallBindings> {
    let mut function_values = values.clone();
    let mut function_alias_scope = SourceExpressionAliasScope::default();
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
        source_bind_function_argument(
            context,
            parameter,
            &argument.value,
            &mut function_values,
            alias_scope,
            &mut function_alias_scope,
            body_cache,
            call_stack,
        )?;
        if argument.name.is_none() {
            positional_index = positional_index.checked_add(1)?;
        }
    }

    for parameter in &function.parameters {
        if provided.contains(&parameter.name) {
            continue;
        }
        source_bind_function_default(
            context,
            parameter,
            &mut function_values,
            &mut function_alias_scope,
            body_cache,
            call_stack,
        )?;
    }
    collect_function_body_alias_dependencies(
        context,
        function,
        alias_scope,
        &mut function_alias_scope,
        body_cache,
    );

    Some(SourceFunctionCallBindings {
        values: function_values,
        alias_scope: function_alias_scope,
    })
}

fn source_bind_function_argument(
    context: &SourceTemplateLoweringContext<'_>,
    parameter: &lzvm_pil::FunctionParameter,
    expression: &Expression,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    caller_alias_scope: &SourceExpressionAliasScope,
    function_alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<()> {
    if source_expr_parameter(parameter) {
        if source_expression_name(expression) == Some(parameter.name.as_str()) {
            return Some(());
        }
        collect_expression_dependencies_into_scope(
            expression,
            caller_alias_scope,
            function_alias_scope,
        );
        function_alias_scope.expressions_mut().insert(
            parameter.name.clone(),
            source_expression_with_static_values(context.program, expression, values),
        );
        return Some(());
    }
    if source_expr_array_parameter(parameter) {
        let alias = source_expr_array_argument_alias(
            context,
            expression,
            values,
            caller_alias_scope.expressions.as_ref(),
            caller_alias_scope.expression_arrays.as_ref(),
            body_cache,
            call_stack,
        )?;
        collect_expression_dependencies_into_scope(
            expression,
            caller_alias_scope,
            function_alias_scope,
        );
        insert_source_expr_array_static_values(
            context.program,
            expression,
            values,
            &parameter.name,
        )?;
        let expression_array_aliases = function_alias_scope.expression_arrays_mut();
        let _ = insert_source_expr_array_alias_length(
            values,
            &parameter.name,
            &alias,
            expression_array_aliases,
        );
        if matches!(&alias, SourceExpressionArrayAlias::Name(name) if name == &parameter.name) {
            return Some(());
        }
        expression_array_aliases.insert(parameter.name.clone(), alias);
        return Some(());
    }
    if source_const_parameter(parameter) && parameter.array_dims.is_empty() {
        let value = evaluate_source_static_expression(context.program, expression, values)?;
        values.insert(parameter.name.clone(), value);
        return Some(());
    }
    if !source_const_parameter(parameter) {
        return None;
    }
    if let Some(elements) = source_static_array_expression(context.program, expression, values) {
        return insert_source_static_array(values, &parameter.name, elements);
    }
    let name = source_expression_name(expression)?;
    let elements = source_static_array_values(values, name)?;
    insert_source_static_array(values, &parameter.name, elements)
}

fn source_bind_function_default(
    context: &SourceTemplateLoweringContext<'_>,
    parameter: &lzvm_pil::FunctionParameter,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    function_alias_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<()> {
    if source_expr_parameter(parameter) {
        let expression = parameter.default_expression.as_ref()?;
        let source_alias_scope = function_alias_scope.clone();
        collect_expression_dependencies_into_scope(
            expression,
            &source_alias_scope,
            function_alias_scope,
        );
        function_alias_scope.expressions_mut().insert(
            parameter.name.clone(),
            source_expression_with_static_values(context.program, expression, values),
        );
        return Some(());
    }
    if source_expr_array_parameter(parameter) {
        let expression = parameter.default_expression.as_ref()?;
        let alias = source_expr_array_argument_alias(
            context,
            expression,
            values,
            function_alias_scope.expressions.as_ref(),
            function_alias_scope.expression_arrays.as_ref(),
            body_cache,
            call_stack,
        )?;
        let source_alias_scope = function_alias_scope.clone();
        collect_expression_dependencies_into_scope(
            expression,
            &source_alias_scope,
            function_alias_scope,
        );
        insert_source_expr_array_static_values(
            context.program,
            expression,
            values,
            &parameter.name,
        )?;
        let expression_array_aliases = function_alias_scope.expression_arrays_mut();
        let _ = insert_source_expr_array_alias_length(
            values,
            &parameter.name,
            &alias,
            expression_array_aliases,
        );
        expression_array_aliases.insert(parameter.name.clone(), alias);
        return Some(());
    }
    if source_const_parameter(parameter) && parameter.array_dims.is_empty() {
        let expression = parameter.default_expression.as_ref()?;
        let value = evaluate_source_static_expression(context.program, expression, values)?;
        values.insert(parameter.name.clone(), value);
        return Some(());
    }
    if !source_const_parameter(parameter) {
        return None;
    }
    let elements = source_static_array_literal(
        context.program,
        context.module,
        parameter.default_value?,
        values,
    )?;
    insert_source_static_array(values, &parameter.name, elements)
}

fn source_expr_array_argument_alias(
    context: &SourceTemplateLoweringContext<'_>,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Option<SourceExpressionArrayAlias> {
    source_expression_array_alias(expression)
        .map(|alias| {
            source_expression_array_alias_with_scope(
                alias,
                expression_aliases,
                expression_array_aliases,
            )
        })
        .or_else(|| {
            source_expression_array_slice_alias(
                context.program,
                expression,
                values,
                expression_array_aliases,
            )
            .map(|alias| {
                source_expression_array_alias_with_scope(
                    alias,
                    expression_aliases,
                    expression_array_aliases,
                )
            })
        })
        .or_else(|| {
            if source_expression_has_nested_call_argument(expression) {
                return source_returned_expression_array_call_alias(
                    context,
                    expression,
                    Vec::new(),
                );
            }
            let call_alias_scope = SourceExpressionAliasScope::from_maps(
                expression_aliases.clone(),
                expression_array_aliases.clone(),
            );
            source_returned_expression_array_alias(
                context,
                expression,
                values,
                &call_alias_scope,
                body_cache,
                call_stack,
            )
            .or_else(|| {
                source_returned_expression_array_call_alias(context, expression, Vec::new())
            })
        })
}

fn source_expression_array_alias_with_scope(
    alias: SourceExpressionArrayAlias,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> SourceExpressionArrayAlias {
    let mut resolving = BTreeSet::new();
    source_expression_array_alias_with_scope_inner(
        alias,
        expression_aliases,
        expression_array_aliases,
        &mut resolving,
    )
}

fn source_expression_array_alias_with_scope_inner(
    alias: SourceExpressionArrayAlias,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    resolving: &mut BTreeSet<String>,
) -> SourceExpressionArrayAlias {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if !resolving.insert(name.clone()) {
                return SourceExpressionArrayAlias::Name(name);
            }
            let resolved = expression_array_aliases
                .get(&name)
                .cloned()
                .map(|alias| {
                    source_expression_array_alias_with_scope_inner(
                        alias,
                        expression_aliases,
                        expression_array_aliases,
                        resolving,
                    )
                })
                .unwrap_or_else(|| SourceExpressionArrayAlias::Name(name.clone()));
            resolving.remove(&name);
            resolved
        }
        SourceExpressionArrayAlias::Values(expressions) => {
            SourceExpressionArrayAlias::ScopedValues {
                scope: Arc::new(source_expression_array_alias_dependency_scope(
                    &expressions,
                    expression_aliases,
                    expression_array_aliases,
                )),
                expressions,
            }
        }
        alias => alias,
    }
}

fn source_expression_array_alias_dependency_scope(
    expressions: &[Expression],
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> SourceExpressionAliasScope {
    let mut scope = SourceExpressionAliasScope::default();
    let mut visited_expressions = BTreeSet::new();
    let mut visited_arrays = BTreeSet::new();
    for expression in expressions {
        collect_expression_alias_dependencies(
            expression,
            expression_aliases,
            expression_array_aliases,
            &mut scope,
            &mut visited_expressions,
            &mut visited_arrays,
        );
    }
    scope
}

pub(crate) fn collect_expression_dependencies_into_scope(
    expression: &Expression,
    source_scope: &SourceExpressionAliasScope,
    target_scope: &mut SourceExpressionAliasScope,
) {
    let mut visited_expressions = BTreeSet::new();
    let mut visited_arrays = BTreeSet::new();
    collect_expression_alias_dependencies(
        expression,
        source_scope.expressions.as_ref(),
        source_scope.expression_arrays.as_ref(),
        target_scope,
        &mut visited_expressions,
        &mut visited_arrays,
    );
}

fn collect_expression_alias_dependencies(
    expression: &Expression,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    scope: &mut SourceExpressionAliasScope,
    visited_expressions: &mut BTreeSet<String>,
    visited_arrays: &mut BTreeSet<String>,
) {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Name(name) => {
            collect_named_expression_alias_dependency(
                name,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
            collect_named_array_alias_dependency(
                name,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
        }
        ExpressionKind::Group(inner) => collect_expression_alias_dependencies(
            inner,
            expression_aliases,
            expression_array_aliases,
            scope,
            visited_expressions,
            visited_arrays,
        ),
        ExpressionKind::Array(expressions) => {
            for expression in expressions {
                collect_expression_alias_dependencies(
                    expression,
                    expression_aliases,
                    expression_array_aliases,
                    scope,
                    visited_expressions,
                    visited_arrays,
                );
            }
        }
        ExpressionKind::Unary { expr, .. } => collect_expression_alias_dependencies(
            expr,
            expression_aliases,
            expression_array_aliases,
            scope,
            visited_expressions,
            visited_arrays,
        ),
        ExpressionKind::Binary { left, right, .. } => {
            collect_expression_alias_dependencies(
                left,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
            collect_expression_alias_dependencies(
                right,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            collect_expression_alias_dependencies(
                condition,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
            collect_expression_alias_dependencies(
                then_expr,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
            collect_expression_alias_dependencies(
                else_expr,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
        }
        ExpressionKind::Call { callee, args } => {
            collect_expression_alias_dependencies(
                callee,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
            for arg in args {
                collect_expression_alias_dependencies(
                    &arg.value,
                    expression_aliases,
                    expression_array_aliases,
                    scope,
                    visited_expressions,
                    visited_arrays,
                );
            }
        }
        ExpressionKind::Index { target, index } => {
            collect_expression_alias_dependencies(
                target,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
            collect_expression_alias_dependencies(
                index,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
        }
        ExpressionKind::RowOffset { target, offset, .. } => {
            collect_expression_alias_dependencies(
                target,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
            collect_expression_alias_dependencies(
                offset,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            );
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => {}
    }
}

fn collect_named_expression_alias_dependency(
    name: &str,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    scope: &mut SourceExpressionAliasScope,
    visited_expressions: &mut BTreeSet<String>,
    visited_arrays: &mut BTreeSet<String>,
) {
    for candidate in source_alias_name_candidates(name) {
        let Some(expression) = expression_aliases.get(candidate) else {
            continue;
        };
        if !visited_expressions.insert(candidate.to_owned()) {
            continue;
        }
        if !scope.expressions.contains_key(candidate) {
            scope
                .expressions_mut()
                .insert(candidate.to_owned(), expression.clone());
        }
        collect_expression_alias_dependencies(
            expression,
            expression_aliases,
            expression_array_aliases,
            scope,
            visited_expressions,
            visited_arrays,
        );
    }
}

fn collect_named_array_alias_dependency(
    name: &str,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    scope: &mut SourceExpressionAliasScope,
    visited_expressions: &mut BTreeSet<String>,
    visited_arrays: &mut BTreeSet<String>,
) {
    for candidate in source_alias_name_candidates(name) {
        let Some(alias) = expression_array_aliases.get(candidate) else {
            continue;
        };
        if !visited_arrays.insert(candidate.to_owned()) {
            continue;
        }
        if !scope.expression_arrays.contains_key(candidate) {
            scope
                .expression_arrays_mut()
                .insert(candidate.to_owned(), alias.clone());
        }
        collect_array_alias_dependencies(
            alias,
            expression_aliases,
            expression_array_aliases,
            scope,
            visited_expressions,
            visited_arrays,
        );
    }
}

fn collect_array_alias_dependencies(
    alias: &SourceExpressionArrayAlias,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
    scope: &mut SourceExpressionAliasScope,
    visited_expressions: &mut BTreeSet<String>,
    visited_arrays: &mut BTreeSet<String>,
) {
    match alias {
        SourceExpressionArrayAlias::Name(name) => collect_named_array_alias_dependency(
            name,
            expression_aliases,
            expression_array_aliases,
            scope,
            visited_expressions,
            visited_arrays,
        ),
        SourceExpressionArrayAlias::Values(expressions) => {
            for expression in expressions {
                collect_expression_alias_dependencies(
                    expression,
                    expression_aliases,
                    expression_array_aliases,
                    scope,
                    visited_expressions,
                    visited_arrays,
                );
            }
        }
        SourceExpressionArrayAlias::ScopedValues {
            expressions,
            scope: scoped,
        } => {
            for expression in expressions {
                collect_expression_alias_dependencies(
                    expression,
                    scoped.expressions.as_ref(),
                    scoped.expression_arrays.as_ref(),
                    scope,
                    visited_expressions,
                    visited_arrays,
                );
            }
        }
        SourceExpressionArrayAlias::Call { expression, .. } => {
            collect_expression_alias_dependencies(
                expression,
                expression_aliases,
                expression_array_aliases,
                scope,
                visited_expressions,
                visited_arrays,
            )
        }
    }
}

fn source_alias_name_candidates(name: &str) -> [&str; 2] {
    [name, source_alias_binding_name(name)]
}

fn collect_function_body_alias_dependencies(
    context: &SourceTemplateLoweringContext<'_>,
    function: &FunctionDeclaration,
    source_scope: &SourceExpressionAliasScope,
    target_scope: &mut SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
) {
    let Some((start, end)) = body_cache.span_token_bounds(context.tokens, function.body) else {
        return;
    };
    let mut visited_expressions = BTreeSet::new();
    let mut visited_arrays = BTreeSet::new();
    for token in &context.tokens[start..end] {
        if token.kind != TokenKind::Identifier {
            continue;
        }
        collect_named_expression_alias_dependency(
            &token.lexeme,
            source_scope.expressions.as_ref(),
            source_scope.expression_arrays.as_ref(),
            target_scope,
            &mut visited_expressions,
            &mut visited_arrays,
        );
        collect_named_array_alias_dependency(
            &token.lexeme,
            source_scope.expressions.as_ref(),
            source_scope.expression_arrays.as_ref(),
            target_scope,
            &mut visited_expressions,
            &mut visited_arrays,
        );
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
    let Some(name) = source_expression_name(expression) else {
        return Some(());
    };
    let Some(elements) = source_static_array_values(values, name) else {
        return Some(());
    };
    insert_source_static_array(values, target_name, elements)
}

fn source_const_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    !parameter.by_reference && (parameter.is_const || parameter.type_name == "int")
}

fn source_expr_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    !parameter.by_reference && parameter.array_dims.is_empty() && parameter.type_name == "expr"
}

fn source_expr_array_parameter(parameter: &lzvm_pil::FunctionParameter) -> bool {
    !parameter.by_reference && !parameter.array_dims.is_empty() && parameter.type_name == "expr"
}

pub(crate) fn source_expression_array_alias(
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

fn source_expression_array_slice_alias(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<SourceExpressionArrayAlias> {
    let (name, index_expressions) = source_expression_index_chain(expression)?;
    let indices = index_expressions
        .iter()
        .map(|index| {
            let value = evaluate_source_static_expression(program, index, values)?;
            usize::try_from(source_static_integer_value(Some(&value))?).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    if let Some(alias) =
        source_expression_array_assignment_alias_name(name, expression_array_aliases).and_then(
            |name| {
                expression_array_aliases.get(&name).and_then(|alias| {
                    source_expression_array_alias_slice(alias, &indices, expression_array_aliases)
                })
            },
        )
    {
        return Some(alias);
    }
    source_raw_expression_array_slice_alias(program, expression, values, name, &indices)
}

fn source_expression_array_alias_slice(
    alias: &SourceExpressionArrayAlias,
    indices: &[usize],
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<SourceExpressionArrayAlias> {
    if indices.is_empty() {
        return Some(alias.clone());
    }
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            expression_array_aliases.get(name).and_then(|alias| {
                source_expression_array_alias_slice(alias, indices, expression_array_aliases)
            })
        }
        SourceExpressionArrayAlias::Values(expressions) => {
            source_expression_array_values_slice(expressions, indices, expression_array_aliases)
        }
        SourceExpressionArrayAlias::ScopedValues { expressions, scope } => {
            source_expression_array_values_slice(expressions, indices, expression_array_aliases)
                .map(|alias| match alias {
                    SourceExpressionArrayAlias::Values(expressions) => {
                        SourceExpressionArrayAlias::ScopedValues {
                            expressions,
                            scope: Arc::clone(scope),
                        }
                    }
                    alias => alias,
                })
        }
        SourceExpressionArrayAlias::Call { .. } => None,
    }
}

fn source_raw_expression_array_slice_alias(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
    indices: &[usize],
) -> Option<SourceExpressionArrayAlias> {
    if indices.is_empty() {
        return None;
    }
    let slice_name = source_static_indexed_array_name(name, indices);
    let length = usize::try_from(source_static_array_length(values, &slice_name)?).ok()?;
    let target = source_expression_with_static_values(program, expression, values);
    Some(SourceExpressionArrayAlias::Values(
        (0..length)
            .map(|index| source_indexed_expression(&target, index))
            .collect(),
    ))
}

fn source_static_indexed_array_name(name: &str, indices: &[usize]) -> String {
    let mut name = name.to_owned();
    for index in indices {
        name.push_str(&format!("[{index}]"));
    }
    name
}

fn source_indexed_expression(target: &Expression, index: usize) -> Expression {
    Expression {
        source_name: target.source_name.clone(),
        start: target.start,
        end: target.end,
        kind: ExpressionKind::Index {
            target: Box::new(target.clone()),
            index: Box::new(Expression {
                source_name: target.source_name.clone(),
                start: target.start,
                end: target.end,
                kind: ExpressionKind::Integer(index.to_string()),
            }),
        },
    }
}

fn source_expression_array_values_slice(
    expressions: &[Expression],
    indices: &[usize],
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<SourceExpressionArrayAlias> {
    let (&index, rest) = indices.split_first()?;
    let expression = expressions.get(index)?;
    if rest.is_empty() {
        return source_expression_array_alias(expression);
    }
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Array(expressions) => {
            source_expression_array_values_slice(expressions, rest, expression_array_aliases)
        }
        ExpressionKind::Name(name) => expression_array_aliases.get(name).and_then(|alias| {
            source_expression_array_alias_slice(alias, rest, expression_array_aliases)
        }),
        _ => None,
    }
}

fn source_expression_has_nested_call_argument(expression: &Expression) -> bool {
    let Some((_, arguments)) = source_call_expression(Some(expression)) else {
        return false;
    };
    arguments
        .iter()
        .any(|argument| source_expression_contains_call(&argument.value))
}

fn source_expression_contains_call(expression: &Expression) -> bool {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Call { .. } => true,
        ExpressionKind::Array(expressions) => {
            expressions.iter().any(source_expression_contains_call)
        }
        ExpressionKind::Unary { expr, .. } => source_expression_contains_call(expr),
        ExpressionKind::Binary { left, right, .. } => {
            source_expression_contains_call(left) || source_expression_contains_call(right)
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            source_expression_contains_call(condition)
                || source_expression_contains_call(then_expr)
                || source_expression_contains_call(else_expr)
        }
        ExpressionKind::Index { target, index } => {
            source_expression_contains_call(target) || source_expression_contains_call(index)
        }
        ExpressionKind::RowOffset { target, offset, .. } => {
            source_expression_contains_call(target) || source_expression_contains_call(offset)
        }
        _ => false,
    }
}

pub(crate) fn collect_source_template_expression_array_alias(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    expression_aliases: &SourceExpressionAliases,
    alias_scope: &mut SourceExpressionAliasScope,
) {
    if collect_source_expr_array_multi_declaration_aliases(
        context,
        statement,
        values,
        body_cache,
        call_stack,
        expression_aliases,
        alias_scope,
    ) {
        return;
    }
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if declaration.type_name.as_deref() != Some("expr") || declaration.array_dims.is_empty()
            {
                return;
            }
            let current_array_aliases = alias_scope.expression_arrays.clone();
            if let Some(alias) = source_declaration_expression_array_alias(
                context,
                SourceExpressionArrayDeclaration {
                    name: &declaration.name,
                    dim_expressions: &declaration.array_dim_expressions,
                    initializer: declaration.initializer_expression.as_ref(),
                    source_name: &declaration.source_name,
                    start: declaration.start,
                },
                values,
                body_cache,
                expression_aliases,
                &current_array_aliases,
            ) {
                let expression_array_aliases = alias_scope.expression_arrays_mut();
                insert_source_expression_array_alias_binding(
                    expression_array_aliases,
                    &declaration.name,
                    alias.clone(),
                );
                insert_source_expression_array_alias_length_values(
                    values,
                    &declaration.name,
                    &alias,
                    expression_array_aliases,
                );
            }
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            if declaration.type_name != "expr" || declaration.array_dims.is_empty() {
                return;
            }
            let current_array_aliases = alias_scope.expression_arrays.clone();
            if let Some(alias) = source_declaration_expression_array_alias(
                context,
                SourceExpressionArrayDeclaration {
                    name: &declaration.name,
                    dim_expressions: &declaration.array_dim_expressions,
                    initializer: declaration.initializer_expression.as_ref(),
                    source_name: &declaration.source_name,
                    start: declaration.start,
                },
                values,
                body_cache,
                expression_aliases,
                &current_array_aliases,
            ) {
                let expression_array_aliases = alias_scope.expression_arrays_mut();
                insert_source_expression_array_alias_binding(
                    expression_array_aliases,
                    &declaration.name,
                    alias.clone(),
                );
                insert_source_expression_array_alias_length_values(
                    values,
                    &declaration.name,
                    &alias,
                    expression_array_aliases,
                );
            }
        }
        _ => {
            if source_expression_array_alias_assignment_can_apply(
                context.program,
                statement.value_expression.as_ref(),
                values,
                &alias_scope.expression_arrays,
            ) {
                source_expression_array_alias_assignment_with_returned_calls(
                    context,
                    statement.value_expression.as_ref(),
                    values,
                    body_cache,
                    call_stack,
                    alias_scope,
                );
            }
        }
    }
}

fn collect_source_expr_array_multi_declaration_aliases(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    _call_stack: &mut BTreeSet<String>,
    expression_aliases: &SourceExpressionAliases,
    alias_scope: &mut SourceExpressionAliasScope,
) -> bool {
    let Some(declarations) = source_expr_array_multi_declarations(context, statement, body_cache)
    else {
        return false;
    };
    for declaration in declarations {
        let current_array_aliases = alias_scope.expression_arrays.clone();
        if let Some(alias) = source_declaration_expression_array_alias(
            context,
            SourceExpressionArrayDeclaration {
                name: &declaration.name,
                dim_expressions: &declaration.dim_expressions,
                initializer: None,
                source_name: &declaration.source_name,
                start: declaration.start,
            },
            values,
            body_cache,
            expression_aliases,
            &current_array_aliases,
        ) {
            let expression_array_aliases = alias_scope.expression_arrays_mut();
            insert_source_expression_array_alias_binding(
                expression_array_aliases,
                &declaration.name,
                alias.clone(),
            );
            insert_source_expression_array_alias_length_values(
                values,
                &declaration.name,
                &alias,
                expression_array_aliases,
            );
        }
    }
    true
}

fn source_expression_array_alias_assignment_with_returned_calls(
    context: &SourceTemplateLoweringContext<'_>,
    expression: Option<&Expression>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    alias_scope: &mut SourceExpressionAliasScope,
) -> bool {
    let Some(Expression {
        kind: ExpressionKind::Binary { op, left, right },
        source_name,
        start,
        end,
    }) = expression.map(strip_source_group_expression)
    else {
        return false;
    };
    if !source_expression_contains_call(right) {
        return source_expression_array_alias_assignment(
            context.program,
            expression,
            values,
            alias_scope.expression_arrays_mut(),
        );
    }
    let resolved_right = match source_import_returned_expression_calls(
        context,
        right,
        values,
        alias_scope,
        body_cache,
        call_stack,
    ) {
        Some(imported_right) if imported_right != **right => imported_right,
        _ => {
            let Some(resolved_right) = source_resolved_expression_value_without_returned_calls(
                context,
                right,
                values,
                alias_scope,
                body_cache,
                call_stack,
            ) else {
                return source_expression_array_alias_assignment(
                    context.program,
                    expression,
                    values,
                    alias_scope.expression_arrays_mut(),
                );
            };
            source_import_returned_expression_calls(
                context,
                &resolved_right,
                values,
                alias_scope,
                body_cache,
                call_stack,
            )
            .unwrap_or(resolved_right)
        }
    };
    let resolved_expression = Expression {
        kind: ExpressionKind::Binary {
            op: *op,
            left: left.clone(),
            right: Box::new(resolved_right),
        },
        source_name: source_name.clone(),
        start: *start,
        end: *end,
    };
    source_expression_array_alias_assignment(
        context.program,
        Some(&resolved_expression),
        values,
        alias_scope.expression_arrays_mut(),
    )
}

struct SourceExprArrayMultiDeclaration {
    name: String,
    dim_expressions: Vec<Option<Expression>>,
    source_name: String,
    start: usize,
}

fn source_expr_array_multi_declarations(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    body_cache: &mut SourceControlBodyCache,
) -> Option<Vec<SourceExprArrayMultiDeclaration>> {
    if statement.kind != FunctionStatementKind::Declaration || statement.declaration.is_some() {
        return None;
    }
    let (start_index, end_index) = body_cache.span_token_bounds(
        context.tokens,
        SourceSpan {
            start: statement.start,
            end: statement.end,
        },
    )?;
    let mut cursor = start_index;
    if context
        .tokens
        .get(cursor)
        .is_some_and(|token| matches!(token.kind, TokenKind::Const | TokenKind::Constant))
    {
        cursor += 1;
    }
    if !context
        .tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::Expr)
    {
        return None;
    }
    cursor += 1;
    let mut declarations = Vec::new();
    loop {
        let name = context.tokens.get(cursor)?;
        if name.kind != TokenKind::Identifier {
            return None;
        }
        cursor += 1;
        let mut dim_expressions = Vec::new();
        while context
            .tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::LBracket)
        {
            let close = source_delimited_token_end(context.tokens, cursor)?;
            dim_expressions.push(source_array_dim_expression(context, cursor + 1, close)?);
            cursor = close + 1;
        }
        if dim_expressions.is_empty() {
            return None;
        }
        declarations.push(SourceExprArrayMultiDeclaration {
            name: name.lexeme.clone(),
            dim_expressions,
            source_name: context.module.source_name.clone(),
            start: name.start,
        });
        match context.tokens.get(cursor).map(|token| token.kind) {
            Some(TokenKind::Comma) => {
                cursor += 1;
            }
            Some(TokenKind::Semicolon) if cursor + 1 == end_index => break,
            _ => return None,
        }
    }
    (declarations.len() > 1).then_some(declarations)
}

fn source_array_dim_expression(
    context: &SourceTemplateLoweringContext<'_>,
    start_index: usize,
    end_index: usize,
) -> Option<Option<Expression>> {
    if start_index == end_index {
        return Some(None);
    }
    let (expression, next_index) = parse_expression_tokens(
        context.tokens,
        start_index,
        end_index,
        &context.module.source,
    )
    .ok()?;
    (next_index == end_index).then_some(Some(expression))
}

fn source_delimited_token_end(tokens: &[Token], open_index: usize) -> Option<usize> {
    let open = tokens.get(open_index)?;
    let close_kind = match open.kind {
        TokenKind::LParen => TokenKind::RParen,
        TokenKind::LBracket => TokenKind::RBracket,
        TokenKind::LBrace => TokenKind::RBrace,
        _ => return None,
    };
    let mut depth = 1_usize;
    let mut cursor = open_index + 1;
    while let Some(token) = tokens.get(cursor) {
        if token.kind == open.kind {
            depth = depth.checked_add(1)?;
        } else if token.kind == close_kind {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(cursor);
            }
        }
        cursor += 1;
    }
    None
}

fn insert_source_expression_array_alias_length_values(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
    alias: &SourceExpressionArrayAlias,
    expression_array_aliases: &SourceExpressionArrayAliases,
) {
    let _ = insert_source_expr_array_alias_length(values, name, alias, expression_array_aliases);
    let binding_name = source_alias_binding_name(name);
    if binding_name != name {
        let _ = insert_source_expr_array_alias_length(
            values,
            binding_name,
            alias,
            expression_array_aliases,
        );
    }
}

struct SourceExpressionArrayDeclaration<'a> {
    name: &'a str,
    dim_expressions: &'a [Option<Expression>],
    initializer: Option<&'a Expression>,
    source_name: &'a str,
    start: usize,
}

fn source_declaration_expression_array_alias(
    context: &SourceTemplateLoweringContext<'_>,
    declaration: SourceExpressionArrayDeclaration<'_>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<SourceExpressionArrayAlias> {
    let declared_lengths = source_expression_array_declaration_lengths(
        context.program,
        declaration.dim_expressions,
        values,
    );
    if let Some(expression) = declaration.initializer {
        if let Some(alias) = source_expression_array_alias(expression) {
            return Some(alias);
        }
        let alias_scope = SourceExpressionAliasScope::from_maps(
            expression_aliases.clone(),
            expression_array_aliases.clone(),
        );
        let mut call_stack = BTreeSet::new();
        return source_returned_expression_array_alias(
            context,
            expression,
            values,
            &alias_scope,
            body_cache,
            &mut call_stack,
        )
        .or_else(|| {
            source_returned_expression_array_call_alias(
                context,
                expression,
                declared_lengths.clone().unwrap_or_default(),
            )
        });
    }
    let lengths = declared_lengths?;
    if lengths.is_empty() {
        return None;
    }
    Some(SourceExpressionArrayAlias::Values(
        source_zero_expression_array(
            declaration.name,
            declaration.source_name,
            declaration.start,
            &lengths,
        )?,
    ))
}

fn source_expression_array_declaration_lengths(
    program: &SourceProgram,
    dim_expressions: &[Option<Expression>],
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Vec<usize>> {
    dim_expressions
        .iter()
        .map(|expression| {
            let value = evaluate_source_static_expression(program, expression.as_ref()?, values)?;
            usize::try_from(source_static_integer_value(Some(&value))?).ok()
        })
        .collect()
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

pub(crate) fn source_expression_array_alias_assignment(
    program: &SourceProgram,
    expression: Option<&Expression>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_array_aliases: &mut SourceExpressionArrayAliases,
) -> bool {
    let Some(Expression {
        kind: ExpressionKind::Binary { op, left, right },
        ..
    }) = expression.map(strip_source_group_expression)
    else {
        return false;
    };
    let Some((name, index_expressions)) = source_expression_index_chain(left) else {
        return false;
    };
    let Some(name) = source_expression_array_assignment_alias_name(name, expression_array_aliases)
    else {
        return false;
    };
    let Some(indices) = index_expressions
        .iter()
        .map(|index| {
            let value = evaluate_source_static_expression(program, index, values)?;
            usize::try_from(source_static_integer_value(Some(&value))?).ok()
        })
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let right = source_expression_with_static_values(program, right, values);
    let value = match op {
        BinaryOperator::Assign => right,
        BinaryOperator::PlusAssign | BinaryOperator::MinusAssign | BinaryOperator::StarAssign => {
            let Some(current) = expression_array_aliases
                .get(&name)
                .and_then(|alias| source_expression_array_alias_current(alias, &indices))
            else {
                return false;
            };
            let binary_op = match op {
                BinaryOperator::PlusAssign => BinaryOperator::Add,
                BinaryOperator::MinusAssign => BinaryOperator::Subtract,
                BinaryOperator::StarAssign => BinaryOperator::Multiply,
                _ => return false,
            };
            source_expression_binary(binary_op, current, right)
        }
        _ => return false,
    };
    let Some(alias) = expression_array_aliases.get_mut(&name) else {
        return false;
    };
    assign_source_expression_array_alias(alias, &indices, value)
}

fn source_expression_array_alias_assignment_can_apply(
    program: &SourceProgram,
    expression: Option<&Expression>,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> bool {
    let Some(Expression {
        kind: ExpressionKind::Binary { op, left, .. },
        ..
    }) = expression.map(strip_source_group_expression)
    else {
        return false;
    };
    let Some((name, index_expressions)) = source_expression_index_chain(left) else {
        return false;
    };
    let Some(name) = source_expression_array_assignment_alias_name(name, expression_array_aliases)
    else {
        return false;
    };
    let Some(indices) = index_expressions
        .iter()
        .map(|index| {
            let value = evaluate_source_static_expression(program, index, values)?;
            usize::try_from(source_static_integer_value(Some(&value))?).ok()
        })
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    match op {
        BinaryOperator::Assign => true,
        BinaryOperator::PlusAssign | BinaryOperator::MinusAssign | BinaryOperator::StarAssign => {
            expression_array_aliases
                .get(&name)
                .and_then(|alias| source_expression_array_alias_current(alias, &indices))
                .is_some()
        }
        _ => false,
    }
}

fn source_expression_with_static_values(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Expression {
    if let Some(value) = evaluate_source_static_expression(program, expression, values) {
        return source_static_value_expression(value, expression);
    }
    let kind = match &expression.kind {
        ExpressionKind::Group(inner) => ExpressionKind::Group(Box::new(
            source_expression_with_static_values(program, inner, values),
        )),
        ExpressionKind::Array(items) => ExpressionKind::Array(
            items
                .iter()
                .map(|item| source_expression_with_static_values(program, item, values))
                .collect(),
        ),
        ExpressionKind::Unary { op, expr } => ExpressionKind::Unary {
            op: *op,
            expr: Box::new(source_expression_with_static_values(program, expr, values)),
        },
        ExpressionKind::Binary { op, left, right } => ExpressionKind::Binary {
            op: *op,
            left: Box::new(source_expression_with_static_values(program, left, values)),
            right: Box::new(source_expression_with_static_values(program, right, values)),
        },
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => ExpressionKind::Ternary {
            condition: Box::new(source_expression_with_static_values(
                program, condition, values,
            )),
            then_expr: Box::new(source_expression_with_static_values(
                program, then_expr, values,
            )),
            else_expr: Box::new(source_expression_with_static_values(
                program, else_expr, values,
            )),
        },
        ExpressionKind::Call { callee, args } => ExpressionKind::Call {
            callee: Box::new(source_expression_with_static_values(
                program, callee, values,
            )),
            args: args
                .iter()
                .map(|arg| CallArgument {
                    name: arg.name.clone(),
                    value: source_expression_with_static_values(program, &arg.value, values),
                })
                .collect(),
        },
        ExpressionKind::Index { target, index } => ExpressionKind::Index {
            target: Box::new(source_index_target_with_static_values(
                program, target, values,
            )),
            index: Box::new(source_expression_with_static_values(program, index, values)),
        },
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => ExpressionKind::RowOffset {
            target: Box::new(source_expression_with_static_values(
                program, target, values,
            )),
            offset: Box::new(source_expression_with_static_values(
                program, offset, values,
            )),
            prior: *prior,
        },
        _ => expression.kind.clone(),
    };
    Expression {
        kind,
        source_name: expression.source_name.clone(),
        start: expression.start,
        end: expression.end,
    }
}

fn source_index_target_with_static_values(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Expression {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Index { target, index } => Expression {
            kind: ExpressionKind::Index {
                target: Box::new(source_index_target_with_static_values(
                    program, target, values,
                )),
                index: Box::new(source_expression_with_static_values(program, index, values)),
            },
            source_name: expression.source_name.clone(),
            start: expression.start,
            end: expression.end,
        },
        _ => source_expression_with_static_values(program, expression, values),
    }
}

fn source_static_value_expression(
    value: FixedFileTemplateValue,
    source_expression: &Expression,
) -> Expression {
    let kind = match value {
        FixedFileTemplateValue::Integer(value) => ExpressionKind::Integer(value.to_string()),
        FixedFileTemplateValue::Boolean(value) => {
            ExpressionKind::Integer(if value { "1" } else { "0" }.to_owned())
        }
        FixedFileTemplateValue::String(value) => ExpressionKind::StringLiteral(value),
    };
    Expression {
        kind,
        source_name: source_expression.source_name.clone(),
        start: source_expression.start,
        end: source_expression.end,
    }
}

fn source_expression_array_assignment_alias_name(
    name: &str,
    expression_array_aliases: &SourceExpressionArrayAliases,
) -> Option<String> {
    let alias = expression_array_aliases.get(name)?;
    if let SourceExpressionArrayAlias::Name(target) = alias {
        if target != name && expression_array_aliases.contains_key(target) {
            return Some(target.clone());
        }
    }
    Some(name.to_owned())
}

fn assign_source_expression_array_alias(
    alias: &mut SourceExpressionArrayAlias,
    indices: &[usize],
    value: Expression,
) -> bool {
    let SourceExpressionArrayAlias::Values(expressions) = alias else {
        return false;
    };
    assign_source_expression_array_values(expressions, indices, value)
}

fn source_expression_array_alias_current(
    alias: &SourceExpressionArrayAlias,
    indices: &[usize],
) -> Option<Expression> {
    let SourceExpressionArrayAlias::Values(expressions) = alias else {
        return None;
    };
    source_expression_array_values_current(expressions, indices)
}

fn source_expression_array_values_current(
    expressions: &[Expression],
    indices: &[usize],
) -> Option<Expression> {
    let (&index, rest) = indices.split_first()?;
    let expression = expressions.get(index)?;
    if rest.is_empty() {
        return Some(expression.clone());
    }
    let ExpressionKind::Array(inner) = &strip_source_group_expression(expression).kind else {
        return None;
    };
    source_expression_array_values_current(inner, rest)
}

fn source_expression_binary(op: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression {
        source_name: right.source_name.clone(),
        start: left.start,
        end: right.end,
        kind: ExpressionKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

fn assign_source_expression_array_values(
    expressions: &mut Vec<Expression>,
    indices: &[usize],
    value: Expression,
) -> bool {
    let Some((&index, rest)) = indices.split_first() else {
        return false;
    };
    while expressions.len() <= index {
        expressions.push(source_zero_expression(&value.source_name, value.start));
    }
    if rest.is_empty() {
        expressions[index] = value;
        return true;
    }
    if !matches!(expressions[index].kind, ExpressionKind::Array(_)) {
        expressions[index].kind = ExpressionKind::Array(Vec::new());
    }
    let ExpressionKind::Array(inner) = &mut expressions[index].kind else {
        return false;
    };
    assign_source_expression_array_values(inner, rest, value)
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

pub(crate) fn source_call_expression(
    expression: Option<&Expression>,
) -> Option<(&str, &[CallArgument])> {
    let ExpressionKind::Call { callee, args } = &expression?.kind else {
        return None;
    };
    let ExpressionKind::Name(name) = &callee.kind else {
        return None;
    };
    Some((name.as_str(), args.as_slice()))
}

fn apply_source_static_container_statement(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some(body) = statement.body else {
        return Ok(false);
    };
    let Some(TokenKind::Container) = source_statement_first_token_kind(context.module, statement)
        .map_err(|source| SourceKeyDirectoryMetadataError::Lex {
        source_name: context.module.source_name.clone(),
        source,
    })?
    else {
        return Ok(false);
    };
    Ok(execute_static_template_range(
        context.program,
        context.module,
        body.start,
        body.end,
        values,
    )
    .is_some())
}

fn strip_source_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_source_group_expression(inner),
        _ => expression,
    }
}

fn lower_source_function_body_statement(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    output: &mut SourceTemplateFunctionOutput<'_>,
    mut final_air_queue: Option<&mut SourceTemplateFinalAirQueue>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        if apply_source_static_container_statement(context, statement, values)? {
            return Ok(true);
        }
        let applied =
            apply_source_expression_string_declaration(context, statement, values, alias_scope)
                || apply_source_static_declaration(context.program, statement, values);
        return Ok(applied
            || source_expr_alias_declaration(statement)
            || source_expr_array_multi_declarations(context, statement, body_cache).is_some());
    }
    if statement.kind == FunctionStatementKind::If {
        match source_static_if_body_statements_with_aliases(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            &alias_scope.expressions,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                let mut body_alias_scope = alias_scope.clone();
                for body_statement in body_statements.iter() {
                    if !lower_source_function_body_statement(
                        context,
                        body_statement,
                        values,
                        &body_alias_scope,
                        body_cache,
                        call_stack,
                        output,
                        final_air_queue.as_deref_mut(),
                    )? {
                        return Ok(false);
                    }
                    collect_source_template_expression_aliases_with_stack(
                        context,
                        body_statement,
                        values,
                        body_cache,
                        call_stack,
                        &mut body_alias_scope,
                    );
                }
                return Ok(true);
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                return Ok(false);
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
                let mut loop_alias_scope = alias_scope.clone();
                for iteration_value in &loop_info.iteration_values {
                    values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                    for body_statement in loop_info.body_statements.iter() {
                        if !lower_source_function_body_statement(
                            context,
                            body_statement,
                            values,
                            &loop_alias_scope,
                            body_cache,
                            call_stack,
                            output,
                            final_air_queue.as_deref_mut(),
                        )? {
                            return Ok(false);
                        }
                        collect_source_template_expression_aliases_with_stack(
                            context,
                            body_statement,
                            values,
                            body_cache,
                            call_stack,
                            &mut loop_alias_scope,
                        );
                    }
                }
                loop_info.apply_final_variable_value(values);
                return Ok(true);
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind == FunctionStatementKind::While {
        if statement.body.is_none() {
            return Ok(true);
        }
        match source_static_while_loop_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                return lower_source_function_body_static_while(
                    context,
                    loop_info,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    output,
                    final_air_queue,
                );
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind == FunctionStatementKind::Do {
        match source_static_do_while_loop_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                return lower_source_function_body_static_do_while(
                    context,
                    loop_info,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    output,
                    final_air_queue,
                );
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
    }
    if statement.kind == FunctionStatementKind::Switch {
        match source_static_switch_body_statements(
            context.program,
            context.module,
            context.tokens,
            statement,
            values,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                let mut switch_alias_scope = alias_scope.clone();
                for body_statement in body_statements.iter() {
                    if !lower_source_function_body_statement(
                        context,
                        body_statement,
                        values,
                        &switch_alias_scope,
                        body_cache,
                        call_stack,
                        output,
                        final_air_queue.as_deref_mut(),
                    )? {
                        return Ok(false);
                    }
                    collect_source_template_expression_aliases_with_stack(
                        context,
                        body_statement,
                        values,
                        body_cache,
                        call_stack,
                        &mut switch_alias_scope,
                    );
                }
                return Ok(true);
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
    }
    if source_statement_is_source_directive(context.module, statement).map_err(|source| {
        SourceKeyDirectoryMetadataError::Lex {
            source_name: context.module.source_name.clone(),
            source,
        }
    })? {
        return Ok(true);
    }
    if let Some(call) = source_final_statement_call(context.tokens, context.module, statement)? {
        return match call.scope {
            SourceFinalScope::Proof => Ok(true),
            SourceFinalScope::Air => {
                if !context.final_air_calls_enabled {
                    return Ok(true);
                }
                if let Some(queue) = final_air_queue.as_deref_mut() {
                    queue.push_reentrant_call(context, statement, call, values)?;
                    return Ok(true);
                }
                lower_source_template_function_call_expression(
                    context,
                    &call.expression,
                    values,
                    alias_scope,
                    body_cache,
                    call_stack,
                    output,
                    true,
                    None,
                )
            }
            SourceFinalScope::AirGroup => Ok(true),
        };
    }
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(false);
    }
    if source_noop_call_statement(statement) {
        return Ok(true);
    }
    if apply_source_static_expression_statement(
        context.program,
        statement.value_expression.as_ref(),
        values,
    ) {
        return Ok(true);
    }
    if apply_source_expression_string_assignment(context, statement, values, alias_scope) {
        return Ok(true);
    }
    if apply_source_static_array_assignment_statement(context, statement, values) {
        return Ok(true);
    }
    if source_static_assertion(context.program, context.module, statement, values)? {
        return Ok(true);
    }
    if source_template_expression_alias_can_apply(statement, &alias_scope.expressions) {
        if let Some(name) =
            source_expression_alias_assignment_target(statement.value_expression.as_ref())
        {
            values.remove(name);
        }
        return Ok(true);
    }
    if source_expression_array_alias_assignment_can_apply(
        context.program,
        statement.value_expression.as_ref(),
        values,
        &alias_scope.expression_arrays,
    ) {
        return Ok(true);
    }
    let lookup_inputs = SourceLookupInputs {
        program: context.program,
        module: context.module,
        values,
        constant_values: context.constant_values,
        expression_aliases: &alias_scope.expressions,
        expression_array_aliases: &alias_scope.expression_arrays,
        scalar_slots: context.scalar_slots,
        opening_points: context.opening_points,
    };
    let range_hints =
        lower_source_range_check_statement(&lookup_inputs, context.range_checks, statement)
            .map_err(|source| SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            })?;
    if !range_hints.is_empty() {
        output.hints.extend(range_hints);
        return Ok(true);
    }
    if let Some(hint) =
        lower_source_lookup_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        output.hints.push(hint);
        return Ok(true);
    }
    if let Some(hint) =
        lower_source_memory_helper_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        output.hints.push(hint);
        return Ok(true);
    }
    if let Some(hint) =
        lower_source_operation_helper_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        output.hints.push(hint);
        return Ok(true);
    }
    if let Some(hint) =
        lower_source_arith_helper_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        output.hints.push(hint);
        return Ok(true);
    }
    if let Some(hint) =
        lower_source_annotation_statement(&lookup_inputs, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        output.hints.push(hint);
        return Ok(true);
    }
    if lower_source_template_function_call(
        context,
        statement,
        values,
        alias_scope,
        body_cache,
        call_stack,
        output,
        final_air_queue,
    )? {
        return Ok(true);
    }
    let direct_lowered = lower_source_template_boolean_constraint_with_returned_calls(
        context,
        statement,
        values,
        alias_scope,
        body_cache,
        call_stack,
    );
    match direct_lowered {
        Ok(Some(constraint)) => {
            output.constraints.push(constraint);
            return Ok(true);
        }
        Ok(None) => {}
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
        Err(error) => return Err(error),
    }
    let resolved_statement = source_statement_with_resolved_expression(
        context,
        statement,
        values,
        alias_scope,
        body_cache,
        call_stack,
        true,
    );
    let Some(lowering_statement) = resolved_statement.as_ref() else {
        return Ok(false);
    };
    let fallback_lowered = lower_source_template_boolean_constraint(
        context.program,
        context.module,
        lowering_statement,
        context.scalar_slots,
        values,
        alias_scope,
    );
    match fallback_lowered {
        Ok(Some(constraint)) => {
            output.constraints.push(constraint);
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

fn lower_source_function_body_static_while(
    context: &SourceTemplateLoweringContext<'_>,
    loop_info: SourceStaticWhileLoop,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    output: &mut SourceTemplateFunctionOutput<'_>,
    mut final_air_queue: Option<&mut SourceTemplateFinalAirQueue>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let mut loop_values = values.clone();
    let mut loop_alias_scope = alias_scope.clone();
    let mut loop_hints = Vec::new();
    let mut loop_constraints = Vec::new();
    let mut loop_output = SourceTemplateFunctionOutput {
        hints: &mut loop_hints,
        constraints: &mut loop_constraints,
    };
    for _ in 0..STATIC_WHILE_LOOP_LIMIT {
        let Some(condition_value) =
            evaluate_source_static_expression(context.program, &loop_info.condition, &loop_values)
        else {
            return Ok(false);
        };
        if !static_value_truthy(&condition_value) {
            *values = loop_values;
            output.hints.extend(loop_hints);
            output.constraints.extend(loop_constraints);
            return Ok(true);
        }
        for body_statement in loop_info.body_statements.iter() {
            if !lower_source_function_body_statement(
                context,
                body_statement,
                &mut loop_values,
                &loop_alias_scope,
                body_cache,
                call_stack,
                &mut loop_output,
                final_air_queue.as_deref_mut(),
            )? {
                return Ok(false);
            }
            collect_source_template_expression_aliases_with_stack(
                context,
                body_statement,
                &mut loop_values,
                body_cache,
                call_stack,
                &mut loop_alias_scope,
            );
        }
    }
    Ok(false)
}

fn lower_source_function_body_static_do_while(
    context: &SourceTemplateLoweringContext<'_>,
    loop_info: SourceStaticDoWhileLoop,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
    output: &mut SourceTemplateFunctionOutput<'_>,
    mut final_air_queue: Option<&mut SourceTemplateFinalAirQueue>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let mut loop_values = values.clone();
    let mut loop_alias_scope = alias_scope.clone();
    let mut loop_hints = Vec::new();
    let mut loop_constraints = Vec::new();
    let mut loop_output = SourceTemplateFunctionOutput {
        hints: &mut loop_hints,
        constraints: &mut loop_constraints,
    };
    for _ in 0..STATIC_WHILE_LOOP_LIMIT {
        for body_statement in loop_info.body_statements.iter() {
            if !lower_source_function_body_statement(
                context,
                body_statement,
                &mut loop_values,
                &loop_alias_scope,
                body_cache,
                call_stack,
                &mut loop_output,
                final_air_queue.as_deref_mut(),
            )? {
                return Ok(false);
            }
            collect_source_template_expression_aliases_with_stack(
                context,
                body_statement,
                &mut loop_values,
                body_cache,
                call_stack,
                &mut loop_alias_scope,
            );
        }
        let Some(condition_value) =
            evaluate_source_static_expression(context.program, &loop_info.condition, &loop_values)
        else {
            return Ok(false);
        };
        if !static_value_truthy(&condition_value) {
            *values = loop_values;
            output.hints.extend(loop_hints);
            output.constraints.extend(loop_constraints);
            return Ok(true);
        }
    }
    Ok(false)
}

fn source_expr_alias_declaration(statement: &FunctionStatement) -> bool {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            declaration.type_name.as_deref() == Some("expr")
                && (declaration.initializer_expression.is_some()
                    || !declaration.array_dims.is_empty())
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            declaration.type_name == "expr"
                && (declaration.initializer_expression.is_some()
                    || !declaration.array_dims.is_empty())
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

fn source_expression_index_chain(expression: &Expression) -> Option<(&str, Vec<&Expression>)> {
    match &strip_source_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some((name, Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (name, mut indices) = source_expression_index_chain(target)?;
            indices.push(index);
            Some((name, indices))
        }
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

fn unsupported<T>(message: impl Into<String>) -> Result<T, SourceKeyDirectoryMetadataError> {
    Err(unsupported_source_message(message))
}

fn unsupported_source_message(message: impl Into<String>) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}

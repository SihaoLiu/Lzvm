use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

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
        lower_source_template_boolean_constraint, SourceExpressionAliases,
    },
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_expression_aliases::{
        source_expression_alias_assignment, source_expression_alias_assignment_target,
    },
    source_expression_filters::{
        source_expression_assigns_fixed_index, source_expression_is_assignment,
        source_expression_is_constrained_assignment, source_expression_is_equality_constraint,
    },
    source_expression_return_arrays::source_returned_expression_array_alias,
    source_expression_return_values::{
        collect_source_template_expression_aliases,
        collect_source_template_expression_aliases_with_stack,
        insert_source_expr_array_alias_length,
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
    source_scalar_slots::{SourceChallengeSlotMetadata, SourceScalarSlots},
    source_scope::concrete_template_names,
    source_statement_hints::{
        lower_source_annotation_statement, lower_source_assignment_statement,
        lower_source_lookup_statement, lower_unsupported_source_assignment_statement,
        lower_unsupported_source_call_statement, lower_unsupported_source_constraint_statement,
        lower_unsupported_source_template_statement, source_statement_contains_assignment_operator,
        source_statement_first_token_kind, source_statement_is_source_directive,
        source_statement_line, SourceExpressionArrayAlias, SourceExpressionArrayAliases,
        SourceLookupInputs,
    },
    source_static_values::{
        evaluate_source_static_expression, insert_source_static_array, source_active_static_name,
        source_scalar_constant_values, source_static_array_values,
        source_static_assignment_expression, source_template_constant_value_cache,
    },
    source_template_context::SourceTemplateLoweringContext,
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

#[derive(Clone, Default)]
pub(crate) struct SourceExpressionAliasScope {
    pub(crate) expressions: SourceExpressionAliases,
    pub(crate) expression_arrays: SourceExpressionArrayAliases,
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
    body_caches: &mut SourceControlBodyCaches,
) -> Result<ExpressionInfo, SourceKeyDirectoryMetadataError> {
    let scalar_slots = SourceScalarSlots::from_setup(setup, publics, challenges, proof_values)
        .map_err(|error| unsupported_source_message(error.to_string()))?;
    let active_templates = concrete_template_names(program);
    let constant_values = source_scalar_constant_values(program, 1_u64 << setup.stark.n_bits);
    let template_values = source_template_constant_value_cache(program, &constant_values);
    let unit_instances = source_expression_unit_instances(program, group_name, unit_name);
    let fixed_assignment_columns = source_fixed_assignment_column_names(
        program,
        &active_templates,
        &constant_values,
        &template_values,
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
                    active_templates: &active_templates,
                    constant_values: &constant_values,
                    template_values: &template_values,
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
                    collect_source_template_expression_aliases(
                        &context,
                        statement,
                        &mut statement_values,
                        body_cache,
                        &mut alias_scope,
                    );
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
        if !apply_source_expression_string_declaration(context, statement, values, alias_scope) {
            apply_source_static_declaration(context.program, statement, values);
        }
        return Ok(SourceTemplateStatementFlow::Fallthrough);
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
                    collect_source_template_expression_aliases(
                        context,
                        body_statement,
                        values,
                        body_cache,
                        &mut body_alias_scope,
                    );
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
                for iteration_value in &loop_info.iteration_values {
                    let mut loop_alias_scope = alias_scope.clone();
                    values.insert(loop_info.variable_name.clone(), iteration_value.clone());
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
                        collect_source_template_expression_aliases(
                            context,
                            body_statement,
                            values,
                            body_cache,
                            &mut loop_alias_scope,
                        );
                    }
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
    let mut expression_aliases = alias_scope.expressions.clone();
    if source_expression_alias_assignment(
        statement.value_expression.as_ref(),
        &mut expression_aliases,
    ) {
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
        expression_aliases: &alias_scope.expressions,
        expression_array_aliases: &alias_scope.expression_arrays,
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
    if source_expression_is_constrained_assignment(statement.value_expression.as_ref()) {
        let lowered = lower_source_template_boolean_constraint(
            context.program,
            context.module,
            statement,
            context.scalar_slots,
            values,
            &alias_scope.expressions,
            &alias_scope.expression_arrays,
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
        return Ok(SourceTemplateStatementFlow::Fallthrough);
    }
    let is_assignment = source_expression_is_assignment(statement.value_expression.as_ref());
    if is_assignment {
        let mut expression_arrays = alias_scope.expression_arrays.clone();
        if source_expression_array_alias_assignment(
            context.program,
            statement.value_expression.as_ref(),
            values,
            &mut expression_arrays,
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
    match lower_source_template_boolean_constraint(
        context.program,
        context.module,
        statement,
        context.scalar_slots,
        values,
        &alias_scope.expressions,
        &alias_scope.expression_arrays,
    ) {
        Ok(Some(constraint)) => {
            constraints.push(constraint);
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        Ok(None)
            if source_expression_is_equality_constraint(statement.value_expression.as_ref()) =>
        {
            hints.push(lower_unsupported_source_constraint_statement(
                context.module,
                statement,
            ));
            return Ok(SourceTemplateStatementFlow::Fallthrough);
        }
        Ok(None) => {}
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
            hints.push(lower_unsupported_source_constraint_statement(
                context.module,
                statement,
            ));
            return Ok(SourceTemplateStatementFlow::Fallthrough);
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
    if function.return_type.is_some() {
        return Ok(false);
    }
    let shared_values = if propagate_shared_values {
        source_function_shared_static_values(values, function)
    } else {
        BTreeSet::new()
    };
    let Some(mut bindings) = source_function_call_bindings(
        context.program,
        context.module,
        function,
        arguments,
        values,
        alias_scope,
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
    source_call_expression(statement.value_expression.as_ref())
        .is_some_and(|(name, _)| name == "println")
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
    program: &SourceProgram,
    module: &SourceProgramModule,
    function: &FunctionDeclaration,
    arguments: &[CallArgument],
    values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
) -> Option<SourceFunctionCallBindings> {
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
        source_bind_function_argument(
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
        source_bind_function_default(
            program,
            module,
            parameter,
            &mut function_values,
            &mut function_alias_scope.expressions,
            &mut function_alias_scope.expression_arrays,
        )?;
    }

    Some(SourceFunctionCallBindings {
        values: function_values,
        alias_scope: function_alias_scope,
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
    if source_expr_parameter(parameter) {
        if source_expression_name(expression) == Some(parameter.name.as_str()) {
            return Some(());
        }
        expression_aliases.insert(parameter.name.clone(), expression.clone());
        return Some(());
    }
    if source_expr_array_parameter(parameter) {
        let alias = source_expression_array_alias(expression)?;
        insert_source_expr_array_static_values(program, expression, values, &parameter.name)?;
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

fn source_bind_function_default(
    program: &SourceProgram,
    module: &SourceProgramModule,
    parameter: &lzvm_pil::FunctionParameter,
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
        let alias = source_expression_array_alias(expression)?;
        insert_source_expr_array_static_values(program, expression, values, &parameter.name)?;
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
        let value = evaluate_source_static_expression(program, expression, values)?;
        values.insert(parameter.name.clone(), value);
        return Some(());
    }
    if !source_const_parameter(parameter) {
        return None;
    }
    let elements = source_static_array_literal(program, module, parameter.default_value?, values)?;
    insert_source_static_array(values, &parameter.name, elements)
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
    parameter.is_const && !parameter.by_reference
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

pub(crate) fn collect_source_template_expression_array_alias(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &BTreeMap<String, FixedFileTemplateValue>,
    body_cache: &mut SourceControlBodyCache,
    expression_aliases: &SourceExpressionAliases,
    expression_array_aliases: &mut SourceExpressionArrayAliases,
) {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if declaration.type_name.as_deref() != Some("expr") || declaration.array_dims.is_empty()
            {
                return;
            }
            let current_array_aliases = expression_array_aliases.clone();
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
                expression_array_aliases.insert(declaration.name.clone(), alias);
            }
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            if declaration.type_name != "expr" || declaration.array_dims.is_empty() {
                return;
            }
            let current_array_aliases = expression_array_aliases.clone();
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
                expression_array_aliases.insert(declaration.name.clone(), alias);
            }
        }
        _ => {
            source_expression_array_alias_assignment(
                context.program,
                statement.value_expression.as_ref(),
                values,
                expression_array_aliases,
            );
        }
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
    if let Some(expression) = declaration.initializer {
        if let Some(alias) = source_expression_array_alias(expression) {
            return Some(alias);
        }
        let alias_scope = SourceExpressionAliasScope {
            expressions: expression_aliases.clone(),
            expression_arrays: expression_array_aliases.clone(),
        };
        let mut call_stack = BTreeSet::new();
        return source_returned_expression_array_alias(
            context,
            expression,
            values,
            &alias_scope,
            body_cache,
            &mut call_stack,
        );
    }
    let lengths = declaration
        .dim_expressions
        .iter()
        .map(|expression| {
            let value =
                evaluate_source_static_expression(context.program, expression.as_ref()?, values)?;
            usize::try_from(source_static_integer_value(Some(&value))?).ok()
        })
        .collect::<Option<Vec<_>>>()?;
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
    if *op != BinaryOperator::Assign {
        return false;
    }
    let Some((name, index_expressions)) = source_expression_index_chain(left) else {
        return false;
    };
    let Some(alias) = expression_array_aliases.get_mut(name) else {
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
    assign_source_expression_array_alias(alias, &indices, (**right).clone())
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
        let applied =
            apply_source_expression_string_declaration(context, statement, values, alias_scope)
                || apply_source_static_declaration(context.program, statement, values);
        return Ok(applied || source_expr_alias_declaration(statement));
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
                for iteration_value in &loop_info.iteration_values {
                    let mut loop_alias_scope = alias_scope.clone();
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
    let mut expression_aliases = alias_scope.expressions.clone();
    if source_expression_alias_assignment(
        statement.value_expression.as_ref(),
        &mut expression_aliases,
    ) {
        if let Some(name) =
            source_expression_alias_assignment_target(statement.value_expression.as_ref())
        {
            values.remove(name);
        }
        return Ok(true);
    }
    let mut expression_arrays = alias_scope.expression_arrays.clone();
    if source_expression_array_alias_assignment(
        context.program,
        statement.value_expression.as_ref(),
        values,
        &mut expression_arrays,
    ) {
        return Ok(true);
    }
    let lookup_inputs = SourceLookupInputs {
        program: context.program,
        module: context.module,
        values,
        expression_aliases: &alias_scope.expressions,
        expression_array_aliases: &alias_scope.expression_arrays,
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
        final_air_queue.as_deref_mut(),
    )? {
        return Ok(true);
    }
    match lower_source_template_boolean_constraint(
        context.program,
        context.module,
        statement,
        context.scalar_slots,
        values,
        &alias_scope.expressions,
        &alias_scope.expression_arrays,
    ) {
        Ok(Some(constraint)) => {
            output.constraints.push(constraint);
            Ok(true)
        }
        Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
            Ok(false)
        }
        Err(error) => Err(error),
    }
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

#![allow(clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    lex_source, parse_expression_tokens, AirInstanceDeclaration, AirTemplateDeclaration,
    BinaryOperator, CallArgument, Expression, ExpressionKind, FixedFileTemplateValue,
    FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind, SourceProgram,
    SourceProgramModule, SourceSpan, Token, TokenKind, UnaryOperator,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_expression_statements::{
        apply_source_static_declaration, apply_source_static_expression_statement,
    },
    source_fixed_columns::SourceFixedColumnsWriteError,
    source_fixed_expression::{
        evaluate_source_fixed_template_value_expression_with_parts, SourceFixedConstantValues,
    },
    source_fixed_sequence::{
        canonical_fixed_value, pad_short_literal_sequence, parse_literal_sequence,
    },
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_scope::concrete_template_names,
    source_static_values::{
        evaluate_source_static_expression, evaluate_source_static_expression_with_lookup,
        static_value_truthy, SourceStaticValueLookup,
    },
    source_template_do_while::source_static_do_while_loop_with_tokens,
    source_template_for::source_static_for_loop_with_lookup,
    source_template_if::source_static_if_body_statements_with_lookup,
    source_template_switch::source_static_switch_body_statements,
    source_template_while::{source_static_while_loop_with_tokens, STATIC_WHILE_LOOP_LIMIT},
};

mod dynamic;

use dynamic::collect_source_fixed_dynamic_for_assignment;
pub(crate) use dynamic::{apply_source_fixed_dynamic_operations, SourceFixedDynamicOperation};

pub(crate) struct SourceFixedTemplateAssignments {
    pub(crate) values: BTreeMap<String, Vec<u64>>,
    pub(crate) copy_operations: Vec<SourceFixedCopyOperation>,
    pub(crate) dynamic_operations: Vec<SourceFixedDynamicOperation>,
}

pub(crate) struct SourceFixedCopyOperation {
    pub(crate) source_name: String,
    pub(crate) source_span: SourceSpan,
    pub(crate) source_column: String,
    pub(crate) source_offset: usize,
    pub(crate) target_column: String,
    pub(crate) target_offset: usize,
    pub(crate) count: usize,
}

pub(crate) fn source_fixed_values_from_template_assignments(
    program: &SourceProgram,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    row_count: usize,
    constant_values: &SourceFixedConstantValues,
    unit_instance: Option<&AirInstanceDeclaration>,
) -> Result<SourceFixedTemplateAssignments, SourceFixedColumnsWriteError> {
    let mut partial_values = BTreeMap::<String, Vec<Option<u64>>>::new();
    let mut zero_default_columns = BTreeSet::<String>::new();
    let mut copy_operations = Vec::<SourceFixedCopyOperation>::new();
    let mut dynamic_operations = Vec::<SourceFixedDynamicOperation>::new();
    let active_templates = concrete_template_names(program);
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceFixedColumnsWriteError::Lex {
                source_name: module.source_name.clone(),
                source_span: SourceSpan {
                    start: 0,
                    end: module.source.contents.len(),
                },
                source,
            }
        })?;
        let mut body_cache = SourceControlBodyCache::default();
        for template in &module.air_templates {
            let mut assignment_values = if let Some(instance) = unit_instance {
                if template.name != instance.template {
                    continue;
                }
                SourceFixedAssignmentValues::for_template(
                    program,
                    template,
                    instance,
                    constant_values,
                )
            } else {
                if !active_templates.contains(&template.name) {
                    continue;
                }
                SourceFixedAssignmentValues::for_template_defaults(
                    program,
                    template,
                    constant_values,
                )
            };
            if !active_templates.contains(&template.name) {
                continue;
            }
            let context = SourceFixedTemplateAssignmentContext {
                program,
                module,
                tokens: &tokens,
                expected_columns,
                logical_dimensions,
                row_count,
            };
            for statement in &template.statements {
                collect_source_fixed_template_assignment(
                    &context,
                    statement,
                    &mut assignment_values,
                    &mut body_cache,
                    &mut partial_values,
                    &mut zero_default_columns,
                    &mut copy_operations,
                    &mut dynamic_operations,
                )?;
            }
        }
    }

    let values = partial_values
        .into_iter()
        .filter_map(|(name, values)| {
            if zero_default_columns.contains(&name) {
                return Some((
                    name,
                    values.into_iter().map(|value| value.unwrap_or(0)).collect(),
                ));
            }
            values
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .map(|values| (name, values))
        })
        .collect();
    Ok(SourceFixedTemplateAssignments {
        values,
        copy_operations,
        dynamic_operations,
    })
}

struct SourceFixedTemplateAssignmentContext<'a> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    expected_columns: &'a BTreeSet<String>,
    logical_dimensions: &'a BTreeMap<String, Vec<u32>>,
    row_count: usize,
}

struct SourceFixedAssignmentValues<'a> {
    base_scalars: &'a BTreeMap<String, FixedFileTemplateValue>,
    overlays: Vec<(String, FixedFileTemplateValue)>,
    arrays: &'a BTreeMap<String, Vec<u64>>,
}

impl<'a> SourceFixedAssignmentValues<'a> {
    fn for_template(
        program: &SourceProgram,
        template: &AirTemplateDeclaration,
        instance: &AirInstanceDeclaration,
        constant_values: &'a SourceFixedConstantValues,
    ) -> Self {
        let mut scalars = constant_values.scalars.clone();
        bind_source_fixed_instance_arguments(program, template, instance, &mut scalars);
        Self::with_scalars(constant_values, scalars)
    }

    fn for_template_defaults(
        program: &SourceProgram,
        template: &AirTemplateDeclaration,
        constant_values: &'a SourceFixedConstantValues,
    ) -> Self {
        let mut scalars = constant_values.scalars.clone();
        bind_source_fixed_template_defaults(program, template, &mut scalars, &BTreeSet::new());
        Self::with_scalars(constant_values, scalars)
    }

    fn with_scalars(
        constant_values: &'a SourceFixedConstantValues,
        scalars: BTreeMap<String, FixedFileTemplateValue>,
    ) -> Self {
        let overlays = scalars
            .into_iter()
            .filter(|(name, value)| constant_values.scalars.get(name) != Some(value))
            .collect();
        Self {
            base_scalars: &constant_values.scalars,
            overlays,
            arrays: &constant_values.arrays,
        }
    }

    fn with_loop_value(
        base: &SourceFixedAssignmentValues<'a>,
        variable_name: &str,
        value: &FixedFileTemplateValue,
    ) -> Self {
        let mut overlays = base.overlays.clone();
        overlays.push((variable_name.to_owned(), value.clone()));
        Self {
            base_scalars: base.base_scalars,
            overlays,
            arrays: base.arrays,
        }
    }

    fn scalar_value(&self, name: &str) -> Option<FixedFileTemplateValue> {
        self.source_static_value(name).cloned()
    }

    fn fixed_constant_values(&self) -> SourceFixedConstantValues {
        let mut scalars = self.base_scalars.clone();
        for (name, value) in &self.overlays {
            scalars.insert(name.clone(), value.clone());
        }
        SourceFixedConstantValues {
            scalars,
            arrays: self.arrays.clone(),
        }
    }

    fn replace_scalars(&mut self, scalars: BTreeMap<String, FixedFileTemplateValue>) {
        self.overlays = scalars
            .into_iter()
            .filter(|(name, value)| self.base_scalars.get(name) != Some(value))
            .collect();
    }

    fn set_scalar_value(&mut self, name: &str, value: FixedFileTemplateValue) {
        self.overlays.push((name.to_owned(), value));
    }

    fn apply_static_statement(&mut self, program: &SourceProgram, statement: &FunctionStatement) {
        if !self.static_statement_can_update_values(statement) {
            return;
        }
        let mut scalars = self.fixed_constant_values().scalars;
        let updated = if statement.kind == FunctionStatementKind::Declaration {
            apply_source_static_declaration(program, statement, &mut scalars)
        } else if statement.kind == FunctionStatementKind::Expression {
            apply_source_static_expression_statement(
                program,
                statement.value_expression.as_ref(),
                &mut scalars,
            )
        } else {
            false
        };
        if updated {
            self.replace_scalars(scalars);
        }
    }

    fn static_statement_can_update_values(&self, statement: &FunctionStatement) -> bool {
        if statement.kind == FunctionStatementKind::Declaration {
            return matches!(
                statement.declaration.as_ref(),
                Some(
                    FunctionStatementDeclaration::Constant(_)
                        | FunctionStatementDeclaration::Variable(_)
                )
            );
        }
        if statement.kind != FunctionStatementKind::Expression {
            return false;
        }
        source_fixed_static_expression_target_name(statement.value_expression.as_ref())
            .is_some_and(|name| self.source_static_value(name).is_some())
    }
}

fn bind_source_fixed_instance_arguments(
    program: &SourceProgram,
    template: &AirTemplateDeclaration,
    instance: &AirInstanceDeclaration,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) {
    let mut provided = BTreeSet::new();
    if let Some(arguments) = instance.args_expressions.as_ref() {
        apply_source_fixed_instance_arguments(program, template, arguments, values, &mut provided);
    }
    bind_source_fixed_template_defaults(program, template, values, &provided);
}

fn bind_source_fixed_template_defaults(
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

fn apply_source_fixed_instance_arguments(
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

impl SourceStaticValueLookup for SourceFixedAssignmentValues<'_> {
    fn source_static_value(&self, name: &str) -> Option<&FixedFileTemplateValue> {
        self.overlays
            .iter()
            .rev()
            .find_map(|(candidate, value)| (candidate == name).then_some(value))
            .or_else(|| self.base_scalars.get(name))
    }

    fn source_static_array_element(
        &self,
        name: &str,
        index: usize,
    ) -> Option<FixedFileTemplateValue> {
        self.arrays
            .get(name)
            .and_then(|values| values.get(index))
            .map(|value| FixedFileTemplateValue::Integer(i128::from(*value)))
    }

    fn source_static_integer_values(&self) -> BTreeMap<String, i128> {
        let mut values = self
            .base_scalars
            .iter()
            .filter_map(|(name, value)| match value {
                FixedFileTemplateValue::Integer(value) => Some((name.clone(), *value)),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        for (name, value) in &self.overlays {
            match value {
                FixedFileTemplateValue::Integer(value) => {
                    values.insert(name.clone(), *value);
                }
                _ => {
                    values.remove(name);
                }
            }
        }
        values
    }
}

fn source_fixed_static_expression_target_name(expression: Option<&Expression>) -> Option<&str> {
    let expression = strip_source_fixed_group_expression(expression?);
    match &expression.kind {
        ExpressionKind::Unary {
            op: UnaryOperator::Increment | UnaryOperator::Decrement,
            expr,
        } => source_fixed_static_lvalue_name(expr),
        ExpressionKind::Binary {
            op:
                BinaryOperator::Assign
                | BinaryOperator::PlusAssign
                | BinaryOperator::MinusAssign
                | BinaryOperator::StarAssign,
            left,
            ..
        } => source_fixed_static_lvalue_name(left),
        _ => None,
    }
}

fn source_fixed_static_lvalue_name(expression: &Expression) -> Option<&str> {
    match &strip_source_fixed_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(name),
        _ => None,
    }
}

fn collect_source_fixed_template_assignment(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &mut SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
    zero_default_columns: &mut BTreeSet<String>,
    copy_operations: &mut Vec<SourceFixedCopyOperation>,
    dynamic_operations: &mut Vec<SourceFixedDynamicOperation>,
) -> Result<(), SourceFixedColumnsWriteError> {
    if statement.kind == FunctionStatementKind::If {
        match source_static_if_body_statements_with_lookup(
            context.program,
            context.module,
            context.tokens,
            statement,
            assignment_values,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                for body_statement in body_statements.iter() {
                    collect_source_fixed_template_assignment(
                        context,
                        body_statement,
                        assignment_values,
                        body_cache,
                        partial_values,
                        zero_default_columns,
                        copy_operations,
                        dynamic_operations,
                    )?;
                }
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
            Err(error) => {
                return Err(source_fixed_template_assignment_error(statement, error));
            }
        }
        return Ok(());
    }
    if statement.kind == FunctionStatementKind::Switch {
        let scalars = assignment_values.fixed_constant_values().scalars;
        match source_static_switch_body_statements(
            context.program,
            context.module,
            context.tokens,
            statement,
            &scalars,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                for body_statement in body_statements.iter() {
                    collect_source_fixed_template_assignment(
                        context,
                        body_statement,
                        assignment_values,
                        body_cache,
                        partial_values,
                        zero_default_columns,
                        copy_operations,
                        dynamic_operations,
                    )?;
                }
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
            Err(error) => {
                return Err(source_fixed_template_assignment_error(statement, error));
            }
        }
        return Ok(());
    }
    if statement.kind == FunctionStatementKind::For {
        let dynamic_count_before = dynamic_operations.len();
        collect_source_fixed_dynamic_for_assignment(
            context,
            statement,
            assignment_values,
            body_cache,
            dynamic_operations,
        )?;
        if dynamic_count_before != dynamic_operations.len() {
            return Ok(());
        }
        let mut static_loop_applied = false;
        let partial_value_count_before = source_fixed_partial_value_count(partial_values);
        let zero_default_count_before = zero_default_columns.len();
        let copy_count_before = copy_operations.len();
        let dynamic_count_before = dynamic_operations.len();
        let mut static_loop_state_updated = false;
        match source_static_for_loop_with_lookup(
            context.program,
            context.module,
            context.tokens,
            statement,
            assignment_values,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                for iteration_value in &loop_info.iteration_values {
                    let mut iteration_assignment_values =
                        SourceFixedAssignmentValues::with_loop_value(
                            assignment_values,
                            &loop_info.variable_name,
                            iteration_value,
                        );
                    for body_statement in loop_info.body_statements.iter() {
                        collect_source_fixed_template_assignment(
                            context,
                            body_statement,
                            &mut iteration_assignment_values,
                            body_cache,
                            partial_values,
                            zero_default_columns,
                            copy_operations,
                            dynamic_operations,
                        )?;
                    }
                }
                if let Some(final_value) = loop_info.final_variable_value {
                    assignment_values.set_scalar_value(&loop_info.variable_name, final_value);
                    static_loop_state_updated = true;
                }
                static_loop_applied = true;
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
            Err(error) => {
                return Err(source_fixed_template_assignment_error(statement, error));
            }
        }
        let static_loop_progressed = partial_value_count_before
            != source_fixed_partial_value_count(partial_values)
            || zero_default_count_before != zero_default_columns.len()
            || copy_count_before != copy_operations.len()
            || dynamic_count_before != dynamic_operations.len();
        if static_loop_applied && (static_loop_progressed || static_loop_state_updated) {
            return Ok(());
        }
        collect_source_fixed_dynamic_for_assignment(
            context,
            statement,
            assignment_values,
            body_cache,
            dynamic_operations,
        )?;
        return Ok(());
    }
    if statement.kind == FunctionStatementKind::While {
        let scalars = assignment_values.fixed_constant_values().scalars;
        match source_static_while_loop_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            &scalars,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                for _ in 0..STATIC_WHILE_LOOP_LIMIT {
                    let Some(condition_value) = evaluate_source_static_expression_with_lookup(
                        context.program,
                        &loop_info.condition,
                        assignment_values,
                    ) else {
                        return Ok(());
                    };
                    if !static_value_truthy(&condition_value) {
                        return Ok(());
                    }
                    for body_statement in loop_info.body_statements.iter() {
                        collect_source_fixed_template_assignment(
                            context,
                            body_statement,
                            assignment_values,
                            body_cache,
                            partial_values,
                            zero_default_columns,
                            copy_operations,
                            dynamic_operations,
                        )?;
                    }
                }
                return Ok(());
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
            Err(error) => {
                return Err(source_fixed_template_assignment_error(statement, error));
            }
        }
        return Ok(());
    }
    if statement.kind == FunctionStatementKind::Do {
        let scalars = assignment_values.fixed_constant_values().scalars;
        match source_static_do_while_loop_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            &scalars,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                for _ in 0..STATIC_WHILE_LOOP_LIMIT {
                    for body_statement in loop_info.body_statements.iter() {
                        collect_source_fixed_template_assignment(
                            context,
                            body_statement,
                            assignment_values,
                            body_cache,
                            partial_values,
                            zero_default_columns,
                            copy_operations,
                            dynamic_operations,
                        )?;
                    }
                    let Some(condition_value) = evaluate_source_static_expression_with_lookup(
                        context.program,
                        &loop_info.condition,
                        assignment_values,
                    ) else {
                        return Ok(());
                    };
                    if !static_value_truthy(&condition_value) {
                        return Ok(());
                    }
                }
                return Ok(());
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
            Err(error) => {
                return Err(source_fixed_template_assignment_error(statement, error));
            }
        }
        return Ok(());
    }
    if statement.kind == FunctionStatementKind::Declaration {
        assignment_values.apply_static_statement(context.program, statement);
        return Ok(());
    }
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(());
    }
    if collect_source_fixed_table_fill_statement(
        context,
        statement,
        assignment_values,
        partial_values,
        zero_default_columns,
    )? {
        return Ok(());
    }
    if collect_source_fixed_table_copy_statement(
        context,
        statement,
        assignment_values,
        copy_operations,
    )? {
        return Ok(());
    }
    if collect_source_fixed_sequence_assignment_statement(
        context,
        statement,
        assignment_values,
        partial_values,
    )? {
        return Ok(());
    }
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(());
    };
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        assignment_values.apply_static_statement(context.program, statement);
        return Ok(());
    };
    if *op != BinaryOperator::Assign {
        assignment_values.apply_static_statement(context.program, statement);
        return Ok(());
    }
    if collect_source_fixed_element_sequence_assignment(
        context,
        left,
        right,
        assignment_values,
        partial_values,
    )? {
        return Ok(());
    }
    let Some((column_name, row)) = source_fixed_index_assignment_target(
        &context.module.source_name,
        left,
        context.expected_columns,
        context.logical_dimensions,
        context.row_count,
        assignment_values,
    )?
    else {
        assignment_values.apply_static_statement(context.program, statement);
        return Ok(());
    };
    let Some(value) = evaluate_source_fixed_assignment_value_expression(right, assignment_values)
        .as_ref()
        .and_then(source_fixed_assignment_integer)
    else {
        assignment_values.apply_static_statement(context.program, statement);
        return Ok(());
    };
    let value = canonical_fixed_value(
        value,
        &context.module.source_name,
        SourceSpan {
            start: right.start,
            end: right.end,
        },
    )?;
    let values = partial_values
        .entry(column_name.clone())
        .or_insert_with(|| vec![None; context.row_count]);
    match values[row] {
        Some(existing) if existing != value => {
            Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                source_name: context.module.source_name.clone(),
                column: column_name,
            })
        }
        Some(_) => Ok(()),
        None => {
            values[row] = Some(value);
            Ok(())
        }
    }
}

fn source_fixed_partial_value_count(values: &BTreeMap<String, Vec<Option<u64>>>) -> usize {
    values
        .values()
        .map(|values| values.iter().filter(|value| value.is_some()).count())
        .sum()
}

fn collect_source_fixed_table_fill_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
    zero_default_columns: &mut BTreeSet<String>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some((name, arguments)) = source_fixed_call_statement(statement) else {
        return Ok(false);
    };
    if name != "Tables.fill" {
        return Ok(false);
    }
    if arguments.len() != 4 || arguments.iter().any(|argument| argument.name.is_some()) {
        return Err(SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: context.module.source_name.clone(),
            source_span: SourceSpan {
                start: statement.start,
                end: statement.end,
            },
            expression: source_fixed_statement_text(context, statement),
        });
    }

    let Some(column_name) = source_fixed_physical_assignment_column_name(
        &arguments[1].value,
        context.expected_columns,
        context.logical_dimensions,
        assignment_values,
    ) else {
        return Ok(false);
    };
    let value =
        source_fixed_static_integer_argument(context, &arguments[0].value, assignment_values)?;
    let offset =
        source_fixed_static_usize_argument(context, &arguments[2].value, assignment_values)?;
    let count =
        source_fixed_static_usize_argument(context, &arguments[3].value, assignment_values)?;
    let end = offset.checked_add(count).ok_or_else(|| {
        SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: context.module.source_name.clone(),
            source_span: SourceSpan {
                start: arguments[3].value.start,
                end: arguments[3].value.end,
            },
            expression: count.to_string(),
        }
    })?;
    if end > context.row_count {
        return Err(SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: context.module.source_name.clone(),
            source_span: SourceSpan {
                start: arguments[3].value.start,
                end: arguments[3].value.end,
            },
            expression: end.to_string(),
        });
    }
    let value = canonical_fixed_value(
        value,
        &context.module.source_name,
        SourceSpan {
            start: arguments[0].value.start,
            end: arguments[0].value.end,
        },
    )?;
    let values = partial_values
        .entry(column_name.clone())
        .or_insert_with(|| vec![None; context.row_count]);
    zero_default_columns.insert(column_name.clone());
    for value_slot in values.iter_mut().take(end).skip(offset) {
        match *value_slot {
            Some(existing) if existing != value => {
                return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                    source_name: context.module.source_name.clone(),
                    column: column_name,
                });
            }
            Some(_) => {}
            None => *value_slot = Some(value),
        }
    }
    Ok(true)
}

fn collect_source_fixed_table_copy_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    copy_operations: &mut Vec<SourceFixedCopyOperation>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some((name, arguments)) = source_fixed_call_statement(statement) else {
        return Ok(false);
    };
    if name != "Tables.copy" {
        return Ok(false);
    }
    if arguments.len() != 5 || arguments.iter().any(|argument| argument.name.is_some()) {
        return Err(SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: context.module.source_name.clone(),
            source_span: SourceSpan {
                start: statement.start,
                end: statement.end,
            },
            expression: source_fixed_statement_text(context, statement),
        });
    }

    let Some(source_column) = source_fixed_physical_assignment_column_name(
        &arguments[0].value,
        context.expected_columns,
        context.logical_dimensions,
        assignment_values,
    ) else {
        return Ok(false);
    };
    let source_offset =
        source_fixed_static_usize_argument(context, &arguments[1].value, assignment_values)?;
    let Some(target_column) = source_fixed_physical_assignment_column_name(
        &arguments[2].value,
        context.expected_columns,
        context.logical_dimensions,
        assignment_values,
    ) else {
        return Ok(false);
    };
    let target_offset =
        source_fixed_static_usize_argument(context, &arguments[3].value, assignment_values)?;
    let count =
        source_fixed_static_usize_argument(context, &arguments[4].value, assignment_values)?;
    source_fixed_checked_range_end(
        context,
        &arguments[3].value,
        target_offset,
        count,
        context.row_count,
    )?;
    copy_operations.push(SourceFixedCopyOperation {
        source_name: context.module.source_name.clone(),
        source_span: SourceSpan {
            start: statement.start,
            end: statement.end,
        },
        source_column,
        source_offset,
        target_column,
        target_offset,
        count,
    });
    Ok(true)
}

fn source_fixed_checked_range_end(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    expression: &Expression,
    offset: usize,
    count: usize,
    row_count: usize,
) -> Result<usize, SourceFixedColumnsWriteError> {
    let end = offset.checked_add(count).ok_or_else(|| {
        SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: context.module.source_name.clone(),
            source_span: SourceSpan {
                start: expression.start,
                end: expression.end,
            },
            expression: count.to_string(),
        }
    })?;
    if end > row_count {
        return Err(SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: context.module.source_name.clone(),
            source_span: SourceSpan {
                start: expression.start,
                end: expression.end,
            },
            expression: end.to_string(),
        });
    }
    Ok(end)
}

fn source_fixed_call_statement(statement: &FunctionStatement) -> Option<(&str, &[CallArgument])> {
    let ExpressionKind::Call { callee, args } =
        &strip_source_fixed_group_expression(statement.value_expression.as_ref()?).kind
    else {
        return None;
    };
    let ExpressionKind::Name(name) = &strip_source_fixed_group_expression(callee).kind else {
        return None;
    };
    Some((name.as_str(), args.as_slice()))
}

fn source_fixed_static_integer_argument(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    expression: &Expression,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Result<i128, SourceFixedColumnsWriteError> {
    let Some(value) =
        evaluate_source_fixed_assignment_value_expression(expression, assignment_values)
    else {
        return Err(SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: context.module.source_name.clone(),
            source_span: SourceSpan {
                start: expression.start,
                end: expression.end,
            },
            expression: source_fixed_expression_text(context, expression),
        });
    };
    source_fixed_assignment_integer(&value).ok_or_else(|| {
        SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: context.module.source_name.clone(),
            source_span: SourceSpan {
                start: expression.start,
                end: expression.end,
            },
            expression: source_fixed_expression_text(context, expression),
        }
    })
}

fn source_fixed_static_usize_argument(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    expression: &Expression,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Result<usize, SourceFixedColumnsWriteError> {
    let value = source_fixed_static_integer_argument(context, expression, assignment_values)?;
    usize::try_from(value).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
        source_name: context.module.source_name.clone(),
        source_span: SourceSpan {
            start: expression.start,
            end: expression.end,
        },
        expression: value.to_string(),
    })
}

fn source_fixed_statement_text(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
) -> String {
    context
        .module
        .source
        .contents
        .get(statement.start..statement.end)
        .unwrap_or("Tables.fill")
        .trim()
        .to_owned()
}

fn source_fixed_expression_text(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    expression: &Expression,
) -> String {
    context
        .module
        .source
        .contents
        .get(expression.start..expression.end)
        .unwrap_or("<expression>")
        .trim()
        .to_owned()
}

fn collect_source_fixed_sequence_assignment_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    if let Some(expression) = statement.value_expression.as_ref() {
        let ExpressionKind::Binary {
            op: BinaryOperator::Assign,
            right,
            ..
        } = &strip_source_fixed_group_expression(expression).kind
        else {
            return Ok(false);
        };
        let Some(source) = context.module.source.contents.get(right.start..right.end) else {
            return Ok(false);
        };
        if !source.trim_start().starts_with('[') {
            return Ok(false);
        }
    }
    let Some(value_span) = statement.value else {
        return Ok(false);
    };
    let Some((start_index, end_index)) = source_fixed_token_span_bounds(context.tokens, value_span)
    else {
        return Ok(false);
    };
    let Some(assign_index) =
        source_fixed_top_level_assign_index(context.tokens, start_index, end_index)
    else {
        return Ok(false);
    };
    let right_index = assign_index + 1;
    if !context
        .tokens
        .get(right_index)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        return Ok(false);
    }
    let Ok((left, next_index)) = parse_expression_tokens(
        context.tokens,
        start_index,
        assign_index,
        &context.module.source,
    ) else {
        return Ok(false);
    };
    if next_index != assign_index {
        return Ok(false);
    }
    collect_source_fixed_element_sequence_assignment_from_span(
        context,
        &left,
        SourceSpan {
            start: context.tokens[right_index].start,
            end: value_span.end,
        },
        assignment_values,
        partial_values,
    )
}

fn collect_source_fixed_element_sequence_assignment(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    left: &Expression,
    right: &Expression,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some(source) = context.module.source.contents.get(right.start..right.end) else {
        return Ok(false);
    };
    if !source.trim_start().starts_with('[') {
        return Ok(false);
    }
    collect_source_fixed_element_sequence_assignment_from_span(
        context,
        left,
        SourceSpan {
            start: right.start,
            end: right.end,
        },
        assignment_values,
        partial_values,
    )
}

fn collect_source_fixed_element_sequence_assignment_from_span(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    left: &Expression,
    right_span: SourceSpan,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some(column_name) = source_fixed_element_assignment_target(
        left,
        context.expected_columns,
        context.logical_dimensions,
        assignment_values,
    ) else {
        return Ok(false);
    };
    let Some(source) = context
        .module
        .source
        .contents
        .get(right_span.start..right_span.end)
    else {
        return Ok(false);
    };
    let constant_values = assignment_values.fixed_constant_values();
    let mut values = parse_literal_sequence(
        context.program,
        &context.module.source_name,
        right_span,
        source,
        context.row_count,
        &constant_values,
    )?;
    pad_short_literal_sequence(&mut values, context.row_count);
    merge_source_fixed_complete_values(
        &context.module.source_name,
        &column_name,
        context.row_count,
        values,
        partial_values,
    )?;
    Ok(true)
}

fn source_fixed_token_span_bounds(tokens: &[Token], span: SourceSpan) -> Option<(usize, usize)> {
    let start_index = tokens.iter().position(|token| token.start == span.start)?;
    let end_index = tokens
        .iter()
        .position(|token| token.end == span.end)?
        .checked_add(1)?;
    Some((start_index, end_index))
}

fn source_fixed_top_level_assign_index(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
) -> Option<usize> {
    let mut depth = 0_u32;
    for index in start_index..end_index {
        match tokens.get(index)?.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                depth = depth.checked_add(1)?;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.checked_sub(1)?;
            }
            TokenKind::Assign if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn source_fixed_element_assignment_target(
    expression: &Expression,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<String> {
    source_fixed_physical_assignment_column_name(
        expression,
        expected_columns,
        logical_dimensions,
        values,
    )
}

fn merge_source_fixed_complete_values(
    source_name: &str,
    column_name: &str,
    row_count: usize,
    values: Vec<u64>,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
) -> Result<(), SourceFixedColumnsWriteError> {
    if values.len() != row_count {
        return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
            source_name: source_name.to_owned(),
            column: column_name.to_owned(),
        });
    }
    let partial = partial_values
        .entry(column_name.to_owned())
        .or_insert_with(|| vec![None; row_count]);
    for (row, value) in values.into_iter().enumerate() {
        match partial[row] {
            Some(existing) if existing != value => {
                return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                    source_name: source_name.to_owned(),
                    column: column_name.to_owned(),
                });
            }
            Some(_) => {}
            None => partial[row] = Some(value),
        }
    }
    Ok(())
}

fn evaluate_source_fixed_assignment_value_expression(
    expression: &Expression,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<FixedFileTemplateValue> {
    if values.overlays.is_empty() {
        return evaluate_source_fixed_template_value_expression_with_parts(
            expression,
            values.base_scalars,
            values.arrays,
        );
    }

    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_source_fixed_assignment_integer(value).map(FixedFileTemplateValue::Integer)
        }
        ExpressionKind::StringLiteral(value) | ExpressionKind::TemplateLiteral(value) => {
            Some(FixedFileTemplateValue::String(value.clone()))
        }
        ExpressionKind::Name(name) => values.scalar_value(name),
        ExpressionKind::Group(inner) => {
            evaluate_source_fixed_assignment_value_expression(inner, values)
        }
        ExpressionKind::Unary { op, expr } => {
            let value = evaluate_source_fixed_assignment_value_expression(expr, values)?;
            match op {
                UnaryOperator::Plus => {
                    source_fixed_assignment_integer(&value).map(FixedFileTemplateValue::Integer)
                }
                UnaryOperator::Minus => source_fixed_assignment_integer(&value)
                    .and_then(i128::checked_neg)
                    .map(FixedFileTemplateValue::Integer),
                UnaryOperator::Not => Some(FixedFileTemplateValue::Boolean(
                    !source_fixed_assignment_truthy(&value),
                )),
                UnaryOperator::Increment | UnaryOperator::Decrement => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let left = evaluate_source_fixed_assignment_value_expression(left, values)?;
            if *op == BinaryOperator::LogicalAnd {
                if source_fixed_assignment_truthy(&left) {
                    return evaluate_source_fixed_assignment_value_expression(right, values);
                }
                return Some(left);
            }
            if *op == BinaryOperator::LogicalOr {
                if source_fixed_assignment_truthy(&left) {
                    return Some(left);
                }
                return evaluate_source_fixed_assignment_value_expression(right, values);
            }
            let right = evaluate_source_fixed_assignment_value_expression(right, values)?;
            evaluate_source_fixed_assignment_binary(*op, left, right)
        }
        ExpressionKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            let condition = evaluate_source_fixed_assignment_value_expression(condition, values)?;
            if source_fixed_assignment_truthy(&condition) {
                evaluate_source_fixed_assignment_value_expression(then_expr, values)
            } else {
                evaluate_source_fixed_assignment_value_expression(else_expr, values)
            }
        }
        ExpressionKind::Index { target, index } => {
            let ExpressionKind::Name(array_name) =
                &strip_source_fixed_group_expression(target).kind
            else {
                return None;
            };
            let array_values = values.arrays.get(array_name)?;
            let index = evaluate_source_fixed_assignment_value_expression(index, values)?;
            let index = usize::try_from(source_fixed_assignment_integer(&index)?).ok()?;
            array_values
                .get(index)
                .copied()
                .map(|value| FixedFileTemplateValue::Integer(i128::from(value)))
        }
        ExpressionKind::Call { .. }
        | ExpressionKind::Array(_)
        | ExpressionKind::RowOffset { .. }
        | ExpressionKind::PositionalParam(_) => None,
    }
}

fn evaluate_source_fixed_assignment_binary(
    op: BinaryOperator,
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
) -> Option<FixedFileTemplateValue> {
    match op {
        BinaryOperator::Add => match (left, right) {
            (FixedFileTemplateValue::Integer(left), FixedFileTemplateValue::Integer(right)) => {
                left.checked_add(right).map(FixedFileTemplateValue::Integer)
            }
            (left, right) => Some(FixedFileTemplateValue::String(format!(
                "{}{}",
                source_fixed_assignment_string(left),
                source_fixed_assignment_string(right)
            ))),
        },
        BinaryOperator::Subtract => {
            source_fixed_assignment_integer_op(left, right, i128::checked_sub)
        }
        BinaryOperator::Multiply => {
            source_fixed_assignment_integer_op(left, right, i128::checked_mul)
        }
        BinaryOperator::Divide | BinaryOperator::Backslash => {
            let left = source_fixed_assignment_integer(&left)?;
            let right = source_fixed_assignment_integer(&right)?;
            (right != 0).then(|| FixedFileTemplateValue::Integer(left / right))
        }
        BinaryOperator::Modulo => {
            let left = source_fixed_assignment_integer(&left)?;
            let right = source_fixed_assignment_integer(&right)?;
            (right != 0).then(|| FixedFileTemplateValue::Integer(left % right))
        }
        BinaryOperator::Power => {
            let left = source_fixed_assignment_integer(&left)?;
            let right = u32::try_from(source_fixed_assignment_integer(&right)?).ok()?;
            left.checked_pow(right).map(FixedFileTemplateValue::Integer)
        }
        BinaryOperator::ShiftLeft => source_fixed_assignment_shift(left, right, true),
        BinaryOperator::ShiftRight => source_fixed_assignment_shift(left, right, false),
        BinaryOperator::BitAnd => {
            source_fixed_assignment_bitwise(left, right, |left, right| left & right)
        }
        BinaryOperator::BitXor => {
            source_fixed_assignment_bitwise(left, right, |left, right| left ^ right)
        }
        BinaryOperator::BitOr => {
            source_fixed_assignment_bitwise(left, right, |left, right| left | right)
        }
        BinaryOperator::Less => {
            source_fixed_assignment_cmp(left, right, |left, right| left < right)
        }
        BinaryOperator::LessEqual => {
            source_fixed_assignment_cmp(left, right, |left, right| left <= right)
        }
        BinaryOperator::Greater => {
            source_fixed_assignment_cmp(left, right, |left, right| left > right)
        }
        BinaryOperator::GreaterEqual => {
            source_fixed_assignment_cmp(left, right, |left, right| left >= right)
        }
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => Some(
            FixedFileTemplateValue::Boolean(source_fixed_assignment_value_eq(&left, &right)),
        ),
        BinaryOperator::NotEqual => Some(FixedFileTemplateValue::Boolean(
            !source_fixed_assignment_value_eq(&left, &right),
        )),
        _ => None,
    }
}

fn source_fixed_assignment_integer_op(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl Fn(i128, i128) -> Option<i128>,
) -> Option<FixedFileTemplateValue> {
    let left = source_fixed_assignment_integer(&left)?;
    let right = source_fixed_assignment_integer(&right)?;
    op(left, right).map(FixedFileTemplateValue::Integer)
}

fn source_fixed_assignment_shift(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    left_shift: bool,
) -> Option<FixedFileTemplateValue> {
    let left = source_fixed_assignment_integer(&left)?;
    let right = u32::try_from(source_fixed_assignment_integer(&right)?).ok()?;
    if left_shift {
        left.checked_shl(right).map(FixedFileTemplateValue::Integer)
    } else {
        left.checked_shr(right).map(FixedFileTemplateValue::Integer)
    }
}

fn source_fixed_assignment_bitwise(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl Fn(i128, i128) -> i128,
) -> Option<FixedFileTemplateValue> {
    let left = source_fixed_assignment_integer(&left)?;
    let right = source_fixed_assignment_integer(&right)?;
    Some(FixedFileTemplateValue::Integer(op(left, right)))
}

fn source_fixed_assignment_cmp(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl Fn(i128, i128) -> bool,
) -> Option<FixedFileTemplateValue> {
    let left = source_fixed_assignment_integer(&left)?;
    let right = source_fixed_assignment_integer(&right)?;
    Some(FixedFileTemplateValue::Boolean(op(left, right)))
}

fn source_fixed_assignment_value_eq(
    left: &FixedFileTemplateValue,
    right: &FixedFileTemplateValue,
) -> bool {
    match (left, right) {
        (FixedFileTemplateValue::Integer(left), FixedFileTemplateValue::Integer(right)) => {
            left == right
        }
        (FixedFileTemplateValue::Boolean(left), FixedFileTemplateValue::Boolean(right)) => {
            left == right
        }
        (FixedFileTemplateValue::String(left), FixedFileTemplateValue::String(right)) => {
            left == right
        }
        _ => false,
    }
}

fn source_fixed_assignment_integer(value: &FixedFileTemplateValue) -> Option<i128> {
    match value {
        FixedFileTemplateValue::Integer(value) => Some(*value),
        FixedFileTemplateValue::Boolean(value) => Some(if *value { 1 } else { 0 }),
        _ => None,
    }
}

fn source_fixed_assignment_truthy(value: &FixedFileTemplateValue) -> bool {
    match value {
        FixedFileTemplateValue::Integer(value) => *value != 0,
        FixedFileTemplateValue::Boolean(value) => *value,
        FixedFileTemplateValue::String(value) => !value.is_empty(),
    }
}

fn source_fixed_assignment_string(value: FixedFileTemplateValue) -> String {
    match value {
        FixedFileTemplateValue::Integer(value) => value.to_string(),
        FixedFileTemplateValue::Boolean(value) => value.to_string(),
        FixedFileTemplateValue::String(value) => value,
    }
}

fn parse_source_fixed_assignment_integer(value: &str) -> Option<i128> {
    let value = value.trim().replace('_', "");
    if let Some(hex) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        return i128::from_str_radix(hex, 16)
            .ok()
            .and_then(i128::checked_neg);
    }
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return i128::from_str_radix(hex, 16).ok();
    }
    value.parse::<i128>().ok()
}

fn source_fixed_template_assignment_error(
    statement: &FunctionStatement,
    error: SourceKeyDirectoryMetadataError,
) -> SourceFixedColumnsWriteError {
    let source_span = SourceSpan {
        start: statement.start,
        end: statement.end,
    };
    match error {
        SourceKeyDirectoryMetadataError::SourceProgram(error) => {
            SourceFixedColumnsWriteError::SourceProgram(error)
        }
        SourceKeyDirectoryMetadataError::SetupInfo(error) => {
            SourceFixedColumnsWriteError::SetupInfo(error)
        }
        SourceKeyDirectoryMetadataError::Setup(error) => SourceFixedColumnsWriteError::Setup(error),
        SourceKeyDirectoryMetadataError::Parse(error) => {
            SourceFixedColumnsWriteError::ExpressionParse {
                source_name: statement.source_name.clone(),
                source_span,
                source: error,
            }
        }
        SourceKeyDirectoryMetadataError::Lex {
            source_name,
            source,
        } => SourceFixedColumnsWriteError::Lex {
            source_name,
            source_span,
            source,
        },
        other => SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: statement.source_name.clone(),
            source_span,
            expression: other.to_string(),
        },
    }
}

fn source_fixed_index_assignment_target(
    source_name: &str,
    expression: &Expression,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    row_count: usize,
    constant_values: &SourceFixedAssignmentValues<'_>,
) -> Result<Option<(String, usize)>, SourceFixedColumnsWriteError> {
    let ExpressionKind::Index { target, index } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return Ok(None);
    };
    let Some(column_name) = source_fixed_index_assignment_column_name(
        target,
        expected_columns,
        logical_dimensions,
        constant_values,
    ) else {
        return Ok(None);
    };
    let Some(row_value) = evaluate_source_fixed_assignment_value_expression(index, constant_values)
    else {
        return Ok(None);
    };
    let Some(row) = source_fixed_assignment_integer(&row_value) else {
        return Ok(None);
    };
    let row =
        usize::try_from(row).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: source_name.to_owned(),
            source_span: SourceSpan {
                start: index.start,
                end: index.end,
            },
            expression: row.to_string(),
        })?;
    if row >= row_count {
        return Err(SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: source_name.to_owned(),
            source_span: SourceSpan {
                start: index.start,
                end: index.end,
            },
            expression: row.to_string(),
        });
    }
    Ok(Some((column_name, row)))
}

fn source_fixed_index_assignment_column_name(
    expression: &Expression,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<String> {
    source_fixed_physical_assignment_column_name(
        expression,
        expected_columns,
        logical_dimensions,
        values,
    )
}

fn source_fixed_physical_assignment_column_name(
    expression: &Expression,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<String> {
    let (column_name, indices) = source_fixed_assignment_index_path(expression, values)?;
    if indices.is_empty() {
        return expected_columns
            .contains(&column_name)
            .then_some(column_name);
    }
    if let Some(dimensions) = logical_dimensions.get(&column_name) {
        let index = source_fixed_linear_element_index(&indices, dimensions)?;
        let physical_name = format!("{column_name}[{index}]");
        return expected_columns
            .contains(&physical_name)
            .then_some(physical_name);
    }
    if indices.len() == 1 {
        let physical_name = format!("{}[{}]", column_name, indices[0]);
        return expected_columns
            .contains(&physical_name)
            .then_some(physical_name);
    }
    None
}

fn source_fixed_assignment_index_path(
    expression: &Expression,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<(String, Vec<u32>)> {
    match &strip_source_fixed_group_expression(expression).kind {
        ExpressionKind::Name(column_name) => Some((column_name.clone(), Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (column_name, mut indices) = source_fixed_assignment_index_path(target, values)?;
            let index = evaluate_source_fixed_assignment_value_expression(index, values)?;
            let index = source_fixed_assignment_integer(&index)?;
            let index = u32::try_from(index).ok()?;
            indices.push(index);
            Some((column_name, indices))
        }
        _ => None,
    }
}

fn source_fixed_linear_element_index(indices: &[u32], dimensions: &[u32]) -> Option<u32> {
    if indices.len() != dimensions.len() {
        return None;
    }
    indices
        .iter()
        .zip(dimensions)
        .try_fold(0_u32, |acc, (index, dimension)| {
            if index >= dimension {
                return None;
            }
            acc.checked_mul(*dimension)?.checked_add(*index)
        })
}

fn strip_source_fixed_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_source_fixed_group_expression(inner),
        _ => expression,
    }
}

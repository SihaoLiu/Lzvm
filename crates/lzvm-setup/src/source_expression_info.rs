use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::expression_info::{ConstraintCode, ExpressionInfo, HintInfo};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_pil::{
    ColumnKind, FixedFileTemplateValue, FunctionStatement, FunctionStatementKind, SourceProgram,
    TokenKind,
};

use crate::{
    source_constraint_lowering::{
        lower_source_template_boolean_constraint, SourceExpressionAliases,
    },
    source_expression_aliases::collect_source_template_expression_alias,
    source_expression_filters::{
        source_expression_assigns_fixed_index, source_expression_is_assignment,
        source_expression_is_constrained_assignment, source_expression_is_equality_constraint,
    },
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_scalar_slots::SourceScalarSlots,
    source_scope::{
        concrete_template_names, declaration_in_function_body, declaration_in_inactive_template,
    },
    source_statement_hints::{
        lower_source_lookup_statement, lower_unsupported_source_assignment_statement,
        lower_unsupported_source_call_statement, lower_unsupported_source_constraint_statement,
        lower_unsupported_source_template_statement, source_statement_contains_assignment_operator,
        source_statement_first_token_kind, source_statement_line,
    },
    source_static_values::{
        source_declaration_constant_values_from_cache, source_scalar_constant_values,
        source_static_assignment_expression, source_template_constant_value_cache,
    },
    source_template_context::SourceTemplateLoweringContext,
    source_template_for::source_static_for_loop,
    source_template_if::source_static_if_body_statements,
};

pub(crate) fn source_expression_info(
    program: &SourceProgram,
    setup: &UnitSetupInfo,
) -> Result<ExpressionInfo, SourceKeyDirectoryMetadataError> {
    let scalar_slots = SourceScalarSlots::from_setup(setup)
        .map_err(|error| unsupported_source_message(error.to_string()))?;
    let fixed_assignment_columns = source_fixed_assignment_column_names(program);
    let active_templates = concrete_template_names(program);
    let constant_values = source_scalar_constant_values(program, 1_u64 << setup.stark.n_bits);
    let template_values = source_template_constant_value_cache(program, &constant_values);
    let mut hints = Vec::new();
    let mut constraints = Vec::new();
    for module in &program.modules {
        for template in &module.air_templates {
            if !active_templates.contains(&template.name) {
                continue;
            }
            let context = SourceTemplateLoweringContext {
                program,
                module,
                scalar_slots: &scalar_slots,
                fixed_columns: &fixed_assignment_columns,
                constant_values: &constant_values,
                template_values: &template_values,
            };
            let mut expression_aliases = SourceExpressionAliases::new();
            let local_values = BTreeMap::new();
            for statement in &template.statements {
                lower_source_template_statement(
                    &context,
                    statement,
                    &local_values,
                    &expression_aliases,
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
    local_values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        return Ok(());
    }
    let declaration_values = source_declaration_constant_values_from_cache(
        context.module,
        statement.start,
        statement.end,
        context.constant_values,
        context.template_values,
    );
    let merged_declaration_values;
    let declaration_values = if local_values.is_empty() {
        declaration_values
    } else {
        merged_declaration_values = {
            let mut values = declaration_values.clone();
            values.extend(local_values.clone());
            values
        };
        &merged_declaration_values
    };
    if statement.kind == FunctionStatementKind::If {
        match source_static_if_body_statements(
            context.program,
            context.module,
            statement,
            declaration_values,
        ) {
            Ok(Some(body_statements)) => {
                let mut body_aliases = expression_aliases.clone();
                for body_statement in &body_statements {
                    lower_source_template_statement(
                        context,
                        body_statement,
                        local_values,
                        &body_aliases,
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
        match source_static_for_loop(
            context.program,
            context.module,
            statement,
            declaration_values,
        ) {
            Ok(Some(loop_info)) => {
                for iteration_values in &loop_info.iteration_values {
                    let mut loop_aliases = expression_aliases.clone();
                    for body_statement in &loop_info.body_statements {
                        lower_source_template_statement(
                            context,
                            body_statement,
                            iteration_values,
                            &loop_aliases,
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
    ) || source_static_assignment_expression(context.module, statement.value_expression.as_ref())
    {
        return Ok(());
    }
    if let Some(hint) =
        lower_source_lookup_statement(context.module, statement).map_err(|source| {
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
            declaration_values,
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
        declaration_values,
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

fn unsupported<T>(message: impl Into<String>) -> Result<T, SourceKeyDirectoryMetadataError> {
    Err(unsupported_source_message(message))
}

fn unsupported_source_message(message: impl Into<String>) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}

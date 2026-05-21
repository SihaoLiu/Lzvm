use std::collections::BTreeMap;

use lzvm_artifacts::expression_info::{ExpressionInfo, HintInfo};
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::global_program::GlobalProgramError;
use lzvm_artifacts::hint_program::{global_hint_program_from_expression_info, HintProgram};
use lzvm_pil::{
    lex_source, Expression, ExpressionKind, FixedFileTemplateValue, FunctionStatement,
    FunctionStatementDeclaration, FunctionStatementKind, SourceProgram, SourceProgramModule, Token,
};

use crate::{
    source_constraint_lowering::SourceExpressionAliases,
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_expression_aliases::collect_source_template_expression_alias,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_scalar_slots::SourceScalarSlots,
    source_statement_hints::{
        lower_source_lookup_statement, SourceExpressionArrayAlias, SourceExpressionArrayAliases,
        SourceLookupInputs,
    },
    source_static_values::{
        evaluate_source_static_expression, insert_source_static_array,
        source_static_array_expression,
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

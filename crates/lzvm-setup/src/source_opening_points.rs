use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    lex_source, Expression, ExpressionKind, FixedFileTemplateValue, FunctionStatement,
    FunctionStatementDeclaration, FunctionStatementKind, SourceProgram, SourceProgramModule, Token,
    UnaryOperator,
};

use crate::{
    source_constraint_lowering::SourceExpressionAliases,
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{
        evaluate_source_static_expression, source_declaration_constant_values_from_cache,
        SourceTemplateConstantValueCache,
    },
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_tokens,
};

pub(crate) fn source_opening_points(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<Vec<i64>, SourceKeyDirectoryMetadataError> {
    let mut points = vec![0_i64];
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
            let mut expression_aliases = SourceExpressionAliases::new();
            let context = SourceOpeningPointContext {
                program,
                module,
                tokens: &tokens,
                constant_values,
                template_values,
            };
            let mut statement_values = source_declaration_constant_values_from_cache(
                context.module,
                template.body.start,
                template.body.end,
                context.constant_values,
                context.template_values,
            )
            .clone();
            for statement in &template.statements {
                collect_source_statement_opening_points(
                    &context,
                    statement,
                    &mut statement_values,
                    &expression_aliases,
                    body_cache,
                    &mut points,
                )?;
                collect_source_opening_point_expression_alias(statement, &mut expression_aliases);
            }
        }
    }
    Ok(points)
}

struct SourceOpeningPointContext<'a> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    template_values: &'a SourceTemplateConstantValueCache,
}

fn collect_source_statement_opening_points(
    context: &SourceOpeningPointContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    body_cache: &mut SourceControlBodyCache,
    points: &mut Vec<i64>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        apply_source_opening_static_declaration(context.program, statement, values);
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
                    collect_source_statement_opening_points(
                        context,
                        body_statement,
                        values,
                        &body_aliases,
                        body_cache,
                        points,
                    )?;
                    collect_source_opening_point_expression_alias(
                        body_statement,
                        &mut body_aliases,
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
                    let mut loop_aliases = expression_aliases.clone();
                    values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                    for body_statement in loop_info.body_statements.iter() {
                        collect_source_statement_opening_points(
                            context,
                            body_statement,
                            values,
                            &loop_aliases,
                            body_cache,
                            points,
                        )?;
                        collect_source_opening_point_expression_alias(
                            body_statement,
                            &mut loop_aliases,
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
        collect_source_opening_points(
            context.program,
            expression,
            values,
            expression_aliases,
            points,
            &mut resolving_aliases,
        )?;
    }
    Ok(())
}

fn collect_source_opening_points(
    program: &SourceProgram,
    expression: &Expression,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
    points: &mut Vec<i64>,
    resolving_aliases: &mut BTreeSet<String>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    match &expression.kind {
        ExpressionKind::Group(inner) => collect_source_opening_points(
            program,
            inner,
            constant_values,
            expression_aliases,
            points,
            resolving_aliases,
        ),
        ExpressionKind::Unary { expr, .. } => collect_source_opening_points(
            program,
            expr,
            constant_values,
            expression_aliases,
            points,
            resolving_aliases,
        ),
        ExpressionKind::Binary { left, right, .. } => {
            collect_source_opening_points(
                program,
                left,
                constant_values,
                expression_aliases,
                points,
                resolving_aliases,
            )?;
            collect_source_opening_points(
                program,
                right,
                constant_values,
                expression_aliases,
                points,
                resolving_aliases,
            )
        }
        ExpressionKind::Call { callee, args } => {
            collect_source_opening_points(
                program,
                callee,
                constant_values,
                expression_aliases,
                points,
                resolving_aliases,
            )?;
            for arg in args {
                collect_source_opening_points(
                    program,
                    &arg.value,
                    constant_values,
                    expression_aliases,
                    points,
                    resolving_aliases,
                )?;
            }
            Ok(())
        }
        ExpressionKind::Index { target, index } => {
            collect_source_opening_points(
                program,
                target,
                constant_values,
                expression_aliases,
                points,
                resolving_aliases,
            )?;
            collect_source_opening_points(
                program,
                index,
                constant_values,
                expression_aliases,
                points,
                resolving_aliases,
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
                expression_aliases,
                points,
                resolving_aliases,
            )?;
            collect_source_opening_points(
                program,
                offset,
                constant_values,
                expression_aliases,
                points,
                resolving_aliases,
            )?;
            let signed_offset = source_row_offset_value(program, offset, *prior, constant_values)?;
            if !points.contains(&signed_offset) {
                points.push(signed_offset);
            }
            Ok(())
        }
        ExpressionKind::Name(name) => {
            let Some(alias) = expression_aliases.get(name) else {
                return Ok(());
            };
            if !resolving_aliases.insert(name.clone()) {
                return unsupported("source opening point expression alias cycle");
            }
            let result = collect_source_opening_points(
                program,
                alias,
                constant_values,
                expression_aliases,
                points,
                resolving_aliases,
            );
            resolving_aliases.remove(name);
            result
        }
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_) => Ok(()),
    }
}

fn collect_source_opening_point_expression_alias(
    statement: &FunctionStatement,
    expression_aliases: &mut SourceExpressionAliases,
) {
    let Some(FunctionStatementDeclaration::Constant(declaration)) = statement.declaration.as_ref()
    else {
        return;
    };
    if declaration.type_name.as_deref() != Some("expr") || !declaration.array_dims.is_empty() {
        return;
    }
    let Some(expression) = declaration.initializer_expression.as_ref() else {
        return;
    };
    expression_aliases.insert(declaration.name.clone(), expression.clone());
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
    if let Some(FixedFileTemplateValue::Integer(value)) =
        evaluate_source_static_expression(program, expression, values)
    {
        return Ok(value);
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

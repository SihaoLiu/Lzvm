use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    parse_expression_tokens, parse_function_statement_tokens, BinaryOperator, Expression,
    ExpressionKind, FixedFileTemplateValue, FunctionStatement, FunctionStatementDeclaration,
    FunctionStatementKind, SourceProgram, SourceSpan, Token, TokenKind, UnaryOperator,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_fixed_columns::SourceFixedColumnsWriteError,
    source_fixed_expression::{
        source_fixed_expression_value_at_row, SourceFixedConstantValues,
        SourceFixedExpressionValueAtRowRequest,
    },
};

use super::{
    evaluate_source_fixed_assignment_value_expression, source_fixed_assignment_integer,
    source_fixed_physical_assignment_column_name, strip_source_fixed_group_expression,
    SourceFixedAssignmentValues, SourceFixedTemplateAssignmentContext,
};

const SOURCE_FIXED_DYNAMIC_LOCAL_FOR_LIMIT: usize = 256;

pub(crate) struct SourceFixedDynamicOperation {
    source_name: String,
    source: String,
    source_span: SourceSpan,
    target_column: String,
    loop_variable: String,
    start: usize,
    count: usize,
    prefix_statements: Vec<SourceFixedDynamicLocalStatement>,
    conditions: Vec<SourceFixedDynamicCondition>,
    guarded_prefix_statements: Vec<SourceFixedDynamicLocalStatement>,
    expression: Expression,
    constant_values: SourceFixedConstantValues,
}

#[derive(Clone)]
enum SourceFixedDynamicLocalStatement {
    Declaration {
        name: String,
        expression: Expression,
    },
    Assignment {
        name: String,
        expression: Expression,
    },
    Increment {
        name: String,
        delta: i128,
    },
    DeclarationBatch {
        declarations: Vec<(String, Expression)>,
    },
    If {
        branches: Vec<SourceFixedDynamicIfBranch>,
    },
    Switch {
        expression: Expression,
        branches: Vec<SourceFixedDynamicSwitchBranch>,
    },
}

#[derive(Clone)]
struct SourceFixedDynamicIfBranch {
    condition: Option<Expression>,
    statements: Vec<SourceFixedDynamicLocalStatement>,
}

#[derive(Clone)]
struct SourceFixedDynamicSwitchBranch {
    matches: Vec<Expression>,
    statements: Option<Vec<SourceFixedDynamicLocalStatement>>,
}

#[derive(Clone)]
struct SourceFixedDynamicCondition {
    expression: Expression,
    negate: bool,
}

pub(crate) fn apply_source_fixed_dynamic_operations(
    program: &SourceProgram,
    operations: &[SourceFixedDynamicOperation],
    row_count: usize,
    column_values: &mut BTreeMap<String, Vec<u64>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let mut staged_columns = BTreeMap::<String, Vec<Option<u64>>>::new();
    for operation in operations {
        source_fixed_dynamic_range_end(operation, row_count)?;
        if operation.start != 0 || operation.count != row_count {
            return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                source_name: operation.source_name.clone(),
                column: operation.target_column.clone(),
            });
        }
        let mut values = vec![None; row_count];
        let mut unresolved = false;
        let mut constant_values = operation.constant_values.clone();
        for (row, value_slot) in values
            .iter_mut()
            .enumerate()
            .skip(operation.start)
            .take(operation.count)
        {
            let row_value = i128::try_from(row).map_err(|_| {
                SourceFixedColumnsWriteError::IntegerOutOfRange {
                    source_name: operation.source_name.clone(),
                    source_span: operation.source_span,
                    expression: row.to_string(),
                }
            })?;
            constant_values.scalars.insert(
                operation.loop_variable.clone(),
                FixedFileTemplateValue::Integer(row_value),
            );
            for statement in &operation.prefix_statements {
                if !apply_source_fixed_dynamic_local_statement(
                    program,
                    operation,
                    statement,
                    row,
                    row_count,
                    &mut constant_values,
                    column_values,
                )? {
                    unresolved = true;
                    break;
                }
            }
            if unresolved {
                break;
            }
            if !source_fixed_dynamic_conditions_match(
                program,
                operation,
                &operation.conditions,
                row,
                row_count,
                &constant_values,
                column_values,
            )? {
                continue;
            }
            for statement in &operation.guarded_prefix_statements {
                if !apply_source_fixed_dynamic_local_statement(
                    program,
                    operation,
                    statement,
                    row,
                    row_count,
                    &mut constant_values,
                    column_values,
                )? {
                    unresolved = true;
                    break;
                }
            }
            if unresolved {
                break;
            }
            let Some(value) = source_fixed_dynamic_expression_value(
                program,
                operation,
                &operation.expression,
                row,
                row_count,
                &constant_values,
                column_values,
            )?
            else {
                unresolved = true;
                break;
            };
            *value_slot = Some(value);
        }
        if unresolved {
            continue;
        }
        source_fixed_dynamic_merge_values(&mut staged_columns, operation, values)?;
    }
    let mut progressed = false;
    for (target_column, values) in staged_columns {
        let Some(values) = values.into_iter().collect::<Option<Vec<_>>>() else {
            continue;
        };
        let source_name = operations
            .iter()
            .find(|operation| operation.target_column == target_column)
            .map_or("", |operation| operation.source_name.as_str());
        match column_values.get(&target_column) {
            Some(existing) if existing != &values => {
                return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                    source_name: source_name.to_owned(),
                    column: target_column,
                });
            }
            Some(_) => {}
            None => {
                column_values.insert(target_column, values);
                progressed = true;
            }
        }
    }
    Ok(progressed)
}

fn source_fixed_dynamic_conditions_match(
    program: &SourceProgram,
    operation: &SourceFixedDynamicOperation,
    conditions: &[SourceFixedDynamicCondition],
    row: usize,
    row_count: usize,
    constant_values: &SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    for condition in conditions {
        let Some(value) = source_fixed_dynamic_expression_value(
            program,
            operation,
            &condition.expression,
            row,
            row_count,
            constant_values,
            column_values,
        )?
        else {
            return Ok(false);
        };
        let selected = value != 0;
        if selected == condition.negate {
            return Ok(false);
        }
    }
    Ok(true)
}

fn source_fixed_dynamic_merge_values(
    staged_columns: &mut BTreeMap<String, Vec<Option<u64>>>,
    operation: &SourceFixedDynamicOperation,
    values: Vec<Option<u64>>,
) -> Result<(), SourceFixedColumnsWriteError> {
    let target = staged_columns
        .entry(operation.target_column.clone())
        .or_insert_with(|| vec![None; values.len()]);
    for (row, value) in values.into_iter().enumerate() {
        let Some(value) = value else {
            continue;
        };
        match target.get_mut(row) {
            Some(slot @ None) => *slot = Some(value),
            Some(Some(existing)) if *existing == value => {}
            Some(Some(_)) | None => {
                return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                    source_name: operation.source_name.clone(),
                    column: operation.target_column.clone(),
                });
            }
        }
    }
    Ok(())
}

pub(super) fn collect_source_fixed_dynamic_for_assignment(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
    dynamic_operations: &mut Vec<SourceFixedDynamicOperation>,
) -> Result<(), SourceFixedColumnsWriteError> {
    let Some(loop_info) =
        source_fixed_dynamic_for_loop(context, statement, assignment_values, body_cache)?
    else {
        return Ok(());
    };
    let mut operations = Vec::new();
    let mut prefix_statements = Vec::new();
    for body_statement in loop_info.body_statements.iter() {
        let Some((target_column, expression)) = source_fixed_dynamic_assignment_statement(
            context,
            body_statement,
            &loop_info.variable_name,
            assignment_values,
        )?
        else {
            if collect_source_fixed_dynamic_guarded_assignment_statements(
                context,
                body_statement,
                &loop_info,
                assignment_values,
                body_cache,
                &prefix_statements,
                &mut operations,
            )? {
                continue;
            }
            if let Some(local_statement) =
                collect_source_fixed_dynamic_local_statement(context, body_statement, body_cache)?
            {
                prefix_statements.push(local_statement);
                continue;
            }
            if !operations.is_empty()
                && source_fixed_dynamic_ignorable_post_assignment_statement(body_statement)
            {
                continue;
            }
            return Ok(());
        };
        operations.push(SourceFixedDynamicOperation {
            source_name: context.module.source_name.clone(),
            source: context.module.source.contents.clone(),
            source_span: SourceSpan {
                start: body_statement.start,
                end: body_statement.end,
            },
            target_column,
            loop_variable: loop_info.variable_name.clone(),
            start: loop_info.start,
            count: loop_info.count,
            prefix_statements: prefix_statements.clone(),
            conditions: Vec::new(),
            guarded_prefix_statements: Vec::new(),
            expression,
            constant_values: assignment_values.fixed_constant_values(),
        });
    }
    dynamic_operations.extend(operations);
    Ok(())
}

fn collect_source_fixed_dynamic_guarded_assignment_statements(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    loop_info: &SourceFixedDynamicForLoop,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
    common_prefix: &[SourceFixedDynamicLocalStatement],
    operations: &mut Vec<SourceFixedDynamicOperation>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let mut collected = Vec::new();
    let conditions = Vec::new();
    let mut guarded_prefix = Vec::new();
    if !collect_source_fixed_dynamic_guarded_statement_list(
        context,
        std::slice::from_ref(statement),
        loop_info,
        assignment_values,
        body_cache,
        common_prefix,
        &conditions,
        &mut guarded_prefix,
        &mut collected,
    )? {
        return Ok(false);
    }
    operations.extend(collected);
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn collect_source_fixed_dynamic_guarded_if_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    loop_info: &SourceFixedDynamicForLoop,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
    common_prefix: &[SourceFixedDynamicLocalStatement],
    parent_conditions: &[SourceFixedDynamicCondition],
    guarded_prefix: &[SourceFixedDynamicLocalStatement],
    operations: &mut Vec<SourceFixedDynamicOperation>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some(branches) =
        source_fixed_dynamic_if_assignment_branches(context, statement, body_cache)?
    else {
        return Ok(false);
    };
    let mut progressed = false;
    for branch in branches {
        let body_statements = body_cache
            .body_statements(context.tokens, branch.body, &context.module.source)
            .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
                source_name: context.module.source_name.clone(),
                source_span: branch.body,
                source,
            })?;
        let mut conditions = parent_conditions.to_vec();
        conditions.extend(branch.conditions);
        let mut branch_prefix = guarded_prefix.to_vec();
        if !collect_source_fixed_dynamic_guarded_statement_list(
            context,
            body_statements.as_ref(),
            loop_info,
            assignment_values,
            body_cache,
            common_prefix,
            &conditions,
            &mut branch_prefix,
            operations,
        )? {
            return Ok(false);
        }
        progressed = true;
    }
    Ok(progressed)
}

#[allow(clippy::too_many_arguments)]
fn collect_source_fixed_dynamic_guarded_statement_list(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statements: &[FunctionStatement],
    loop_info: &SourceFixedDynamicForLoop,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
    common_prefix: &[SourceFixedDynamicLocalStatement],
    conditions: &[SourceFixedDynamicCondition],
    guarded_prefix: &mut Vec<SourceFixedDynamicLocalStatement>,
    operations: &mut Vec<SourceFixedDynamicOperation>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let mut progressed = false;
    for statement in statements {
        if let Some((target_column, expression)) = source_fixed_dynamic_assignment_statement(
            context,
            statement,
            &loop_info.variable_name,
            assignment_values,
        )? {
            operations.push(SourceFixedDynamicOperation {
                source_name: context.module.source_name.clone(),
                source: context.module.source.contents.clone(),
                source_span: SourceSpan {
                    start: statement.start,
                    end: statement.end,
                },
                target_column,
                loop_variable: loop_info.variable_name.clone(),
                start: loop_info.start,
                count: loop_info.count,
                prefix_statements: common_prefix.to_vec(),
                conditions: conditions.to_vec(),
                guarded_prefix_statements: guarded_prefix.clone(),
                expression,
                constant_values: assignment_values.fixed_constant_values(),
            });
            progressed = true;
            continue;
        }
        let before = operations.len();
        if collect_source_fixed_dynamic_guarded_if_statement(
            context,
            statement,
            loop_info,
            assignment_values,
            body_cache,
            common_prefix,
            conditions,
            guarded_prefix,
            operations,
        )? {
            progressed = true;
            continue;
        }
        operations.truncate(before);
        let before = operations.len();
        if collect_source_fixed_dynamic_guarded_for_statement(
            context,
            statement,
            loop_info,
            assignment_values,
            body_cache,
            common_prefix,
            conditions,
            guarded_prefix,
            operations,
        )? {
            progressed = true;
            continue;
        }
        operations.truncate(before);
        if let Some(local_statement) =
            collect_source_fixed_dynamic_local_statement(context, statement, body_cache)?
        {
            guarded_prefix.push(local_statement);
            continue;
        }
        return Ok(false);
    }
    Ok(progressed)
}

#[allow(clippy::too_many_arguments)]
fn collect_source_fixed_dynamic_guarded_for_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    loop_info: &SourceFixedDynamicForLoop,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
    common_prefix: &[SourceFixedDynamicLocalStatement],
    conditions: &[SourceFixedDynamicCondition],
    guarded_prefix: &[SourceFixedDynamicLocalStatement],
    operations: &mut Vec<SourceFixedDynamicOperation>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some(local_loop) =
        source_fixed_dynamic_local_for_loop(context, statement, assignment_values, body_cache)?
    else {
        return Ok(false);
    };
    let mut progressed = false;
    for value in local_loop.start..local_loop.end {
        let value =
            i128::try_from(value).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: context.module.source_name.clone(),
                source_span: SourceSpan {
                    start: statement.start,
                    end: statement.end,
                },
                expression: value.to_string(),
            })?;
        let iteration_value = FixedFileTemplateValue::Integer(value);
        let iteration_assignment_values = SourceFixedAssignmentValues::with_loop_value(
            assignment_values,
            &local_loop.variable_name,
            &iteration_value,
        );
        let mut iteration_prefix = guarded_prefix.to_vec();
        if !collect_source_fixed_dynamic_guarded_statement_list(
            context,
            local_loop.body_statements.as_ref(),
            loop_info,
            &iteration_assignment_values,
            body_cache,
            common_prefix,
            conditions,
            &mut iteration_prefix,
            operations,
        )? {
            return Ok(false);
        }
        progressed = true;
    }
    Ok(progressed)
}

struct SourceFixedDynamicIfAssignmentBranch {
    conditions: Vec<SourceFixedDynamicCondition>,
    body: SourceSpan,
}

fn source_fixed_dynamic_if_assignment_branches(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Vec<SourceFixedDynamicIfAssignmentBranch>>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::If {
        return Ok(None);
    }
    let Some(condition) = statement.header_expression.as_ref() else {
        return Ok(None);
    };
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some((_, statement_end)) = body_cache.span_token_bounds(
        context.tokens,
        SourceSpan {
            start: statement.start,
            end: statement.end,
        },
    ) else {
        return Ok(None);
    };
    let Some((_, mut cursor)) = body_cache.span_token_bounds(context.tokens, body) else {
        return Ok(None);
    };
    let mut branches = vec![SourceFixedDynamicIfAssignmentBranch {
        conditions: vec![SourceFixedDynamicCondition {
            expression: condition.clone(),
            negate: false,
        }],
        body,
    }];
    let mut prior_conditions = vec![condition.clone()];
    while cursor < statement_end {
        let Some(tail) = source_fixed_dynamic_if_assignment_tail(context, cursor, statement_end)?
        else {
            return Ok(None);
        };
        let mut conditions = prior_conditions
            .iter()
            .cloned()
            .map(|expression| SourceFixedDynamicCondition {
                expression,
                negate: true,
            })
            .collect::<Vec<_>>();
        if let Some(condition) = tail.condition {
            prior_conditions.push(condition.clone());
            conditions.push(SourceFixedDynamicCondition {
                expression: condition,
                negate: false,
            });
        }
        branches.push(SourceFixedDynamicIfAssignmentBranch {
            conditions,
            body: tail.body,
        });
        cursor = tail.next_index;
    }
    Ok(Some(branches))
}

struct SourceFixedDynamicIfAssignmentTail {
    condition: Option<Expression>,
    body: SourceSpan,
    next_index: usize,
}

fn source_fixed_dynamic_if_assignment_tail(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    cursor: usize,
    statement_end: usize,
) -> Result<Option<SourceFixedDynamicIfAssignmentTail>, SourceFixedColumnsWriteError> {
    match context.tokens.get(cursor).map(|token| token.kind) {
        Some(TokenKind::ElseIf) => {
            source_fixed_dynamic_if_assignment_conditional_tail(context, cursor, statement_end)
        }
        Some(TokenKind::Else)
            if context
                .tokens
                .get(cursor + 1)
                .is_some_and(|token| token.kind == TokenKind::If) =>
        {
            source_fixed_dynamic_if_assignment_conditional_tail(context, cursor + 1, statement_end)
        }
        Some(TokenKind::Else) => {
            let next = cursor + 1;
            let Some((body, next_index)) =
                source_fixed_dynamic_braced_body_at(context.tokens, next, statement_end)
            else {
                return Ok(None);
            };
            Ok(Some(SourceFixedDynamicIfAssignmentTail {
                condition: None,
                body,
                next_index,
            }))
        }
        _ => Ok(None),
    }
}

fn source_fixed_dynamic_if_assignment_conditional_tail(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    keyword_index: usize,
    statement_end: usize,
) -> Result<Option<SourceFixedDynamicIfAssignmentTail>, SourceFixedColumnsWriteError> {
    let open_index = keyword_index + 1;
    if !context
        .tokens
        .get(open_index)
        .is_some_and(|token| token.kind == TokenKind::LParen)
    {
        return Ok(None);
    }
    let Some(close_index) = source_fixed_dynamic_delimited_end(context.tokens, open_index) else {
        return Ok(None);
    };
    let Some((body, next_index)) =
        source_fixed_dynamic_braced_body_at(context.tokens, close_index + 1, statement_end)
    else {
        return Ok(None);
    };
    let (condition, consumed) = parse_expression_tokens(
        context.tokens,
        open_index,
        close_index + 1,
        &context.module.source,
    )
    .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
        source_name: context.module.source_name.clone(),
        source_span: SourceSpan {
            start: context.tokens[open_index].start,
            end: context.tokens[close_index].end,
        },
        source,
    })?;
    if consumed != close_index + 1 {
        return Ok(None);
    }
    Ok(Some(SourceFixedDynamicIfAssignmentTail {
        condition: Some(condition),
        body,
        next_index,
    }))
}

fn apply_source_fixed_dynamic_local_statement(
    program: &SourceProgram,
    operation: &SourceFixedDynamicOperation,
    statement: &SourceFixedDynamicLocalStatement,
    row: usize,
    row_count: usize,
    constant_values: &mut SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    match statement {
        SourceFixedDynamicLocalStatement::Declaration { name, expression }
        | SourceFixedDynamicLocalStatement::Assignment { name, expression } => {
            apply_source_fixed_dynamic_scalar_statement(
                program,
                operation,
                (name, expression),
                row,
                row_count,
                constant_values,
                column_values,
            )
        }
        SourceFixedDynamicLocalStatement::Increment { name, delta } => {
            apply_source_fixed_dynamic_increment_statement(name, *delta, constant_values)
        }
        SourceFixedDynamicLocalStatement::DeclarationBatch { declarations } => {
            apply_source_fixed_dynamic_declaration_batch(
                program,
                operation,
                declarations,
                row,
                row_count,
                constant_values,
                column_values,
            )
        }
        SourceFixedDynamicLocalStatement::If { branches } => {
            apply_source_fixed_dynamic_if_statement(
                program,
                operation,
                branches,
                row,
                row_count,
                constant_values,
                column_values,
            )
        }
        SourceFixedDynamicLocalStatement::Switch {
            expression,
            branches,
        } => apply_source_fixed_dynamic_switch_statement(
            program,
            operation,
            (expression, branches),
            row,
            row_count,
            constant_values,
            column_values,
        ),
    }
}

fn apply_source_fixed_dynamic_scalar_statement(
    program: &SourceProgram,
    operation: &SourceFixedDynamicOperation,
    assignment: (&str, &Expression),
    row: usize,
    row_count: usize,
    constant_values: &mut SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let (name, expression) = assignment;
    let Some(value) = source_fixed_dynamic_expression_value(
        program,
        operation,
        expression,
        row,
        row_count,
        constant_values,
        column_values,
    )?
    else {
        return Ok(false);
    };
    constant_values.scalars.insert(
        name.to_owned(),
        FixedFileTemplateValue::Integer(i128::from(value)),
    );
    Ok(true)
}

fn apply_source_fixed_dynamic_increment_statement(
    name: &str,
    delta: i128,
    constant_values: &mut SourceFixedConstantValues,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some(current) = constant_values
        .scalars
        .get(name)
        .and_then(source_fixed_assignment_integer)
    else {
        return Ok(false);
    };
    let Some(value) = current.checked_add(delta) else {
        return Ok(false);
    };
    constant_values
        .scalars
        .insert(name.to_owned(), FixedFileTemplateValue::Integer(value));
    Ok(true)
}

fn apply_source_fixed_dynamic_declaration_batch(
    program: &SourceProgram,
    operation: &SourceFixedDynamicOperation,
    declarations: &[(String, Expression)],
    row: usize,
    row_count: usize,
    constant_values: &mut SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let mut values = Vec::with_capacity(declarations.len());
    for (name, expression) in declarations {
        let Some(value) = source_fixed_dynamic_expression_value(
            program,
            operation,
            expression,
            row,
            row_count,
            constant_values,
            column_values,
        )?
        else {
            return Ok(false);
        };
        values.push((name.clone(), value));
    }
    for (name, value) in values {
        constant_values
            .scalars
            .insert(name, FixedFileTemplateValue::Integer(i128::from(value)));
    }
    Ok(true)
}

fn apply_source_fixed_dynamic_if_statement(
    program: &SourceProgram,
    operation: &SourceFixedDynamicOperation,
    branches: &[SourceFixedDynamicIfBranch],
    row: usize,
    row_count: usize,
    constant_values: &mut SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    for branch in branches {
        let selected = match branch.condition.as_ref() {
            Some(condition) => {
                let Some(value) = source_fixed_dynamic_expression_value(
                    program,
                    operation,
                    condition,
                    row,
                    row_count,
                    constant_values,
                    column_values,
                )?
                else {
                    return Ok(false);
                };
                value != 0
            }
            None => true,
        };
        if !selected {
            continue;
        }
        for statement in &branch.statements {
            if !apply_source_fixed_dynamic_local_statement(
                program,
                operation,
                statement,
                row,
                row_count,
                constant_values,
                column_values,
            )? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    Ok(true)
}

fn apply_source_fixed_dynamic_switch_statement(
    program: &SourceProgram,
    operation: &SourceFixedDynamicOperation,
    switch: (&Expression, &[SourceFixedDynamicSwitchBranch]),
    row: usize,
    row_count: usize,
    constant_values: &mut SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let (expression, branches) = switch;
    let Some(selector) = source_fixed_dynamic_expression_value(
        program,
        operation,
        expression,
        row,
        row_count,
        constant_values,
        column_values,
    )?
    else {
        return Ok(false);
    };
    let mut default_branch = None;
    for branch in branches {
        if branch.matches.is_empty() {
            default_branch = default_branch.or(Some(branch));
            continue;
        }
        for candidate in &branch.matches {
            let Some(value) = source_fixed_dynamic_expression_value(
                program,
                operation,
                candidate,
                row,
                row_count,
                constant_values,
                column_values,
            )?
            else {
                return Ok(false);
            };
            if value == selector {
                return apply_source_fixed_dynamic_switch_branch(
                    program,
                    operation,
                    branch,
                    row,
                    row_count,
                    constant_values,
                    column_values,
                );
            }
        }
    }
    if let Some(branch) = default_branch {
        return apply_source_fixed_dynamic_switch_branch(
            program,
            operation,
            branch,
            row,
            row_count,
            constant_values,
            column_values,
        );
    }
    Ok(true)
}

fn apply_source_fixed_dynamic_switch_branch(
    program: &SourceProgram,
    operation: &SourceFixedDynamicOperation,
    branch: &SourceFixedDynamicSwitchBranch,
    row: usize,
    row_count: usize,
    constant_values: &mut SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some(statements) = branch.statements.as_ref() else {
        return Ok(false);
    };
    for statement in statements {
        if !apply_source_fixed_dynamic_local_statement(
            program,
            operation,
            statement,
            row,
            row_count,
            constant_values,
            column_values,
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn source_fixed_dynamic_expression_value(
    program: &SourceProgram,
    operation: &SourceFixedDynamicOperation,
    expression: &Expression,
    row: usize,
    row_count: usize,
    constant_values: &SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<Option<u64>, SourceFixedColumnsWriteError> {
    source_fixed_expression_value_at_row(
        &SourceFixedExpressionValueAtRowRequest {
            program,
            source_name: &operation.source_name,
            source: &operation.source,
            column_name: &operation.target_column,
            expression,
            row_count,
            constant_values,
            column_values,
        },
        row,
    )
}

fn collect_source_fixed_dynamic_local_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicLocalStatement>, SourceFixedColumnsWriteError> {
    if let Some((name, expression)) = source_fixed_dynamic_local_declaration(statement) {
        return Ok(Some(SourceFixedDynamicLocalStatement::Declaration {
            name: name.to_owned(),
            expression: expression.clone(),
        }));
    }
    if let Some((name, expression)) = source_fixed_dynamic_local_assignment(statement) {
        return Ok(Some(SourceFixedDynamicLocalStatement::Assignment {
            name: name.to_owned(),
            expression: expression.clone(),
        }));
    }
    if let Some((name, delta)) = source_fixed_dynamic_local_increment(statement) {
        return Ok(Some(SourceFixedDynamicLocalStatement::Increment {
            name: name.to_owned(),
            delta,
        }));
    }
    if let Some(declarations) =
        source_fixed_dynamic_local_destructuring_declaration(context, statement, body_cache)?
    {
        return Ok(Some(SourceFixedDynamicLocalStatement::DeclarationBatch {
            declarations,
        }));
    }
    if let Some(switch_statement) =
        source_fixed_dynamic_local_switch_statement(context, statement, body_cache)?
    {
        return Ok(Some(switch_statement));
    }
    source_fixed_dynamic_local_if_statement(context, statement, body_cache)
}

fn source_fixed_dynamic_local_declaration(
    statement: &FunctionStatement,
) -> Option<(&str, &Expression)> {
    if statement.kind != FunctionStatementKind::Declaration {
        return None;
    }
    let Some(FunctionStatementDeclaration::Variable(declaration)) = statement.declaration.as_ref()
    else {
        return None;
    };
    if declaration.type_name != "int" || !declaration.array_dims.is_empty() {
        return None;
    }
    declaration
        .initializer_expression
        .as_ref()
        .map(|expression| (declaration.name.as_str(), expression))
}

fn source_fixed_dynamic_local_assignment(
    statement: &FunctionStatement,
) -> Option<(&str, &Expression)> {
    if statement.kind != FunctionStatementKind::Expression {
        return None;
    }
    let expression = statement.value_expression.as_ref()?;
    let ExpressionKind::Binary {
        op: BinaryOperator::Assign,
        left,
        right,
    } = &strip_source_fixed_group_expression(expression).kind
    else {
        return None;
    };
    let ExpressionKind::Name(name) = &strip_source_fixed_group_expression(left).kind else {
        return None;
    };
    Some((name.as_str(), right))
}

fn source_fixed_dynamic_local_increment(statement: &FunctionStatement) -> Option<(&str, i128)> {
    if statement.kind != FunctionStatementKind::Expression {
        return None;
    }
    let expression = statement.value_expression.as_ref()?;
    let ExpressionKind::Unary { op, expr } = &strip_source_fixed_group_expression(expression).kind
    else {
        return None;
    };
    let ExpressionKind::Name(name) = &strip_source_fixed_group_expression(expr).kind else {
        return None;
    };
    match op {
        UnaryOperator::Increment => Some((name.as_str(), 1)),
        UnaryOperator::Decrement => Some((name.as_str(), -1)),
        _ => None,
    }
}

fn source_fixed_dynamic_ignorable_post_assignment_statement(statement: &FunctionStatement) -> bool {
    if statement.kind != FunctionStatementKind::Expression {
        return false;
    }
    let Some(expression) = statement.value_expression.as_ref() else {
        return false;
    };
    let ExpressionKind::Call { callee, .. } = &strip_source_fixed_group_expression(expression).kind
    else {
        return false;
    };
    matches!(
        &strip_source_fixed_group_expression(callee).kind,
        ExpressionKind::Name(name) if name == "println"
    )
}

fn source_fixed_dynamic_local_destructuring_declaration(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Vec<(String, Expression)>>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::Declaration || statement.declaration.is_some() {
        return Ok(None);
    }
    let Some((start_index, end_index)) = body_cache.span_token_bounds(
        context.tokens,
        SourceSpan {
            start: statement.start,
            end: statement.end,
        },
    ) else {
        return Ok(None);
    };
    if !context
        .tokens
        .get(start_index)
        .is_some_and(|token| token.kind == TokenKind::Int)
    {
        return Ok(None);
    }
    let names_open = start_index + 1;
    if !context
        .tokens
        .get(names_open)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        return Ok(None);
    }
    let Some(names_close) = source_fixed_dynamic_delimited_end(context.tokens, names_open) else {
        return Ok(None);
    };
    let Some(names) = source_fixed_dynamic_name_list(context.tokens, names_open + 1, names_close)
    else {
        return Ok(None);
    };
    let assign_index = names_close + 1;
    if !context
        .tokens
        .get(assign_index)
        .is_some_and(|token| token.kind == TokenKind::Assign)
    {
        return Ok(None);
    }
    let values_open = assign_index + 1;
    if !context
        .tokens
        .get(values_open)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        return Ok(None);
    }
    let Some(values_close) = source_fixed_dynamic_delimited_end(context.tokens, values_open) else {
        return Ok(None);
    };
    let Some(semicolon_index) = end_index.checked_sub(1) else {
        return Ok(None);
    };
    if values_close + 1 != semicolon_index
        || !context
            .tokens
            .get(semicolon_index)
            .is_some_and(|token| token.kind == TokenKind::Semicolon)
    {
        return Ok(None);
    }
    let Some(expressions) =
        source_fixed_dynamic_expression_list(context, values_open + 1, values_close)?
    else {
        return Ok(None);
    };
    if names.len() != expressions.len() {
        return Ok(None);
    }
    Ok(Some(names.into_iter().zip(expressions).collect()))
}

fn source_fixed_dynamic_local_if_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicLocalStatement>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::If {
        return Ok(None);
    }
    let Some(condition) = statement.header_expression.as_ref() else {
        return Ok(None);
    };
    let Some((_, statement_end)) = body_cache.span_token_bounds(
        context.tokens,
        SourceSpan {
            start: statement.start,
            end: statement.end,
        },
    ) else {
        return Ok(None);
    };
    let Some(header) = statement.header else {
        return Ok(None);
    };
    let Some((_, body_start)) = body_cache.span_token_bounds(context.tokens, header) else {
        return Ok(None);
    };
    let Some((statements, mut cursor)) = collect_source_fixed_dynamic_local_if_body_at(
        context,
        body_start,
        statement_end,
        body_cache,
    )?
    else {
        return Ok(None);
    };
    let mut branches = vec![SourceFixedDynamicIfBranch {
        condition: Some(condition.clone()),
        statements,
    }];
    while cursor < statement_end {
        let Some(branch) =
            source_fixed_dynamic_local_if_tail(context, cursor, statement_end, body_cache)?
        else {
            return Ok(None);
        };
        cursor = branch.next_index;
        branches.push(branch.branch);
    }
    Ok(Some(SourceFixedDynamicLocalStatement::If { branches }))
}

struct SourceFixedDynamicIfTail {
    branch: SourceFixedDynamicIfBranch,
    next_index: usize,
}

fn source_fixed_dynamic_local_if_tail(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    cursor: usize,
    statement_end: usize,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicIfTail>, SourceFixedColumnsWriteError> {
    let Some(token) = context.tokens.get(cursor) else {
        return Ok(None);
    };
    match token.kind {
        TokenKind::ElseIf => {
            source_fixed_dynamic_conditional_branch(context, cursor, statement_end, body_cache)
        }
        TokenKind::Else => {
            let next = cursor + 1;
            match context.tokens.get(next).map(|token| token.kind) {
                Some(TokenKind::If) => source_fixed_dynamic_conditional_branch(
                    context,
                    next,
                    statement_end,
                    body_cache,
                ),
                Some(_) => {
                    let Some((statements, next_index)) =
                        collect_source_fixed_dynamic_local_if_body_at(
                            context,
                            next,
                            statement_end,
                            body_cache,
                        )?
                    else {
                        return Ok(None);
                    };
                    Ok(Some(SourceFixedDynamicIfTail {
                        branch: SourceFixedDynamicIfBranch {
                            condition: None,
                            statements,
                        },
                        next_index,
                    }))
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

fn source_fixed_dynamic_conditional_branch(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    keyword_index: usize,
    statement_end: usize,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicIfTail>, SourceFixedColumnsWriteError> {
    if !matches!(
        context.tokens.get(keyword_index).map(|token| token.kind),
        Some(TokenKind::If | TokenKind::ElseIf)
    ) {
        return Ok(None);
    }
    let open_index = keyword_index + 1;
    if !context
        .tokens
        .get(open_index)
        .is_some_and(|token| token.kind == TokenKind::LParen)
    {
        return Ok(None);
    }
    let Some(close_index) = source_fixed_dynamic_delimited_end(context.tokens, open_index) else {
        return Ok(None);
    };
    let body_index = close_index + 1;
    let (condition, consumed) = parse_expression_tokens(
        context.tokens,
        open_index,
        close_index + 1,
        &context.module.source,
    )
    .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
        source_name: context.module.source_name.clone(),
        source_span: SourceSpan {
            start: context.tokens[open_index].start,
            end: context.tokens[close_index].end,
        },
        source,
    })?;
    if consumed != close_index + 1 {
        return Ok(None);
    }
    let Some((statements, next_index)) = collect_source_fixed_dynamic_local_if_body_at(
        context,
        body_index,
        statement_end,
        body_cache,
    )?
    else {
        return Ok(None);
    };
    Ok(Some(SourceFixedDynamicIfTail {
        branch: SourceFixedDynamicIfBranch {
            condition: Some(condition),
            statements,
        },
        next_index,
    }))
}

fn collect_source_fixed_dynamic_local_if_body_at(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    start_index: usize,
    statement_end: usize,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<(Vec<SourceFixedDynamicLocalStatement>, usize)>, SourceFixedColumnsWriteError> {
    match context.tokens.get(start_index).map(|token| token.kind) {
        Some(TokenKind::LBrace) => {
            let Some((body, next_index)) =
                source_fixed_dynamic_braced_body_at(context.tokens, start_index, statement_end)
            else {
                return Ok(None);
            };
            let Some(statements) =
                collect_source_fixed_dynamic_local_body(context, body, body_cache)?
            else {
                return Ok(None);
            };
            Ok(Some((statements, next_index)))
        }
        Some(_) => {
            let Some(next_index) = source_fixed_dynamic_unbraced_statement_end(
                context.tokens,
                start_index,
                statement_end,
            ) else {
                return Ok(None);
            };
            let Some(statements) = collect_source_fixed_dynamic_local_statement_range(
                context,
                start_index,
                next_index,
                body_cache,
            )?
            else {
                return Ok(None);
            };
            Ok(Some((statements, next_index)))
        }
        None => Ok(None),
    }
}

fn source_fixed_dynamic_unbraced_statement_end(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
) -> Option<usize> {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(end_index).skip(start_index) {
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Some(index + 1);
        }
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if stack.pop()? != token.kind {
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

fn collect_source_fixed_dynamic_local_body(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    body: SourceSpan,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Vec<SourceFixedDynamicLocalStatement>>, SourceFixedColumnsWriteError> {
    let body_statements = body_cache
        .body_statements(context.tokens, body, &context.module.source)
        .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
            source_name: context.module.source_name.clone(),
            source_span: body,
            source,
        })?;
    let mut statements = Vec::with_capacity(body_statements.len());
    for statement in body_statements.iter() {
        let Some(local_statement) =
            collect_source_fixed_dynamic_local_statement(context, statement, body_cache)?
        else {
            return Ok(None);
        };
        statements.push(local_statement);
    }
    Ok(Some(statements))
}

fn source_fixed_dynamic_local_switch_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicLocalStatement>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::Switch {
        return Ok(None);
    }
    let Some(expression) = statement.header_expression.as_ref() else {
        return Ok(None);
    };
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some(branches) = source_fixed_dynamic_switch_branches(context, body, body_cache)? else {
        return Ok(None);
    };
    Ok(Some(SourceFixedDynamicLocalStatement::Switch {
        expression: expression.clone(),
        branches,
    }))
}

fn source_fixed_dynamic_switch_branches(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    body: SourceSpan,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Vec<SourceFixedDynamicSwitchBranch>>, SourceFixedColumnsWriteError> {
    let Some((open_index, close_after)) = body_cache.span_token_bounds(context.tokens, body) else {
        return Ok(None);
    };
    let Some(body_end) = close_after.checked_sub(1) else {
        return Ok(None);
    };
    let mut cursor = open_index + 1;
    let mut branches = Vec::new();
    while cursor < body_end {
        while cursor < body_end
            && context
                .tokens
                .get(cursor)
                .is_some_and(|token| token.kind == TokenKind::Semicolon)
        {
            cursor += 1;
        }
        if cursor >= body_end {
            break;
        }
        let Some(token) = context.tokens.get(cursor) else {
            return Ok(None);
        };
        let (matches, statements_start) = match token.kind {
            TokenKind::Case => {
                let Some(colon) =
                    source_fixed_dynamic_switch_label_colon(context.tokens, cursor + 1, body_end)
                else {
                    return Ok(None);
                };
                let Some(expressions) =
                    source_fixed_dynamic_expression_list(context, cursor + 1, colon)?
                else {
                    return Ok(None);
                };
                (expressions, colon + 1)
            }
            TokenKind::Default => {
                let Some(colon) =
                    source_fixed_dynamic_switch_label_colon(context.tokens, cursor + 1, body_end)
                else {
                    return Ok(None);
                };
                (Vec::new(), colon + 1)
            }
            _ => return Ok(None),
        };
        let statements_end =
            source_fixed_dynamic_next_switch_label(context.tokens, statements_start, body_end)
                .unwrap_or(body_end);
        let statements = collect_source_fixed_dynamic_local_statement_range(
            context,
            statements_start,
            statements_end,
            body_cache,
        )?;
        branches.push(SourceFixedDynamicSwitchBranch {
            matches,
            statements,
        });
        cursor = statements_end;
    }
    Ok(Some(branches))
}

fn collect_source_fixed_dynamic_local_statement_range(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    start_index: usize,
    end_index: usize,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<Vec<SourceFixedDynamicLocalStatement>>, SourceFixedColumnsWriteError> {
    let statements = parse_function_statement_tokens(
        context.tokens,
        start_index,
        end_index,
        &context.module.source,
    )
    .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
        source_name: context.module.source_name.clone(),
        source_span: SourceSpan {
            start: context
                .tokens
                .get(start_index)
                .map_or(0, |token| token.start),
            end: context
                .tokens
                .get(end_index.saturating_sub(1))
                .map_or(0, |token| token.end),
        },
        source,
    })?;
    let mut local_statements = Vec::new();
    for statement in statements.iter() {
        if statement.kind == FunctionStatementKind::Break {
            break;
        }
        let Some(local_statement) =
            collect_source_fixed_dynamic_local_statement(context, statement, body_cache)?
        else {
            return Ok(None);
        };
        local_statements.push(local_statement);
    }
    Ok(Some(local_statements))
}

fn source_fixed_dynamic_switch_label_colon(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
) -> Option<usize> {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(end_index).skip(start_index) {
        if stack.is_empty() && token.kind == TokenKind::Colon {
            return Some(index);
        }
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if stack.pop()? != token.kind {
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

fn source_fixed_dynamic_next_switch_label(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
) -> Option<usize> {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(end_index).skip(start_index) {
        if stack.is_empty() && matches!(token.kind, TokenKind::Case | TokenKind::Default) {
            return Some(index);
        }
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if stack.pop()? != token.kind {
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

fn source_fixed_dynamic_braced_body_at(
    tokens: &[Token],
    open_index: usize,
    limit_index: usize,
) -> Option<(SourceSpan, usize)> {
    if open_index >= limit_index
        || !tokens
            .get(open_index)
            .is_some_and(|token| token.kind == TokenKind::LBrace)
    {
        return None;
    }
    let close_index = source_fixed_dynamic_delimited_end(tokens, open_index)?;
    if close_index >= limit_index {
        return None;
    }
    Some((
        SourceSpan {
            start: tokens[open_index].start,
            end: tokens[close_index].end,
        },
        close_index + 1,
    ))
}

fn source_fixed_dynamic_delimited_end(tokens: &[Token], open_index: usize) -> Option<usize> {
    let expected = match tokens.get(open_index)?.kind {
        TokenKind::LParen => TokenKind::RParen,
        TokenKind::LBracket => TokenKind::RBracket,
        TokenKind::LBrace => TokenKind::RBrace,
        _ => return None,
    };
    let mut stack = vec![expected];
    for (index, token) in tokens.iter().enumerate().skip(open_index + 1) {
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if stack.pop()? != token.kind {
                    return None;
                }
                if stack.is_empty() {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn source_fixed_dynamic_name_list(
    tokens: &[Token],
    start_index: usize,
    end_index: usize,
) -> Option<Vec<String>> {
    if start_index >= end_index {
        return None;
    }
    let mut names = Vec::new();
    let mut cursor = start_index;
    loop {
        let token = tokens.get(cursor)?;
        if token.kind != TokenKind::Identifier {
            return None;
        }
        names.push(token.lexeme.clone());
        cursor += 1;
        if cursor == end_index {
            return Some(names);
        }
        if !tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::Comma)
        {
            return None;
        }
        cursor += 1;
        if cursor >= end_index {
            return None;
        }
    }
}

fn source_fixed_dynamic_expression_list(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    start_index: usize,
    end_index: usize,
) -> Result<Option<Vec<Expression>>, SourceFixedColumnsWriteError> {
    if start_index >= end_index {
        return Ok(Some(Vec::new()));
    }
    let mut expressions = Vec::new();
    let mut segment_start = start_index;
    let mut stack = Vec::new();
    let mut cursor = start_index;
    while cursor < end_index {
        let Some(token) = context.tokens.get(cursor) else {
            return Ok(None);
        };
        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return Ok(None);
                };
                if expected != token.kind {
                    return Ok(None);
                }
            }
            TokenKind::Comma if stack.is_empty() => {
                if segment_start >= cursor {
                    return Ok(None);
                }
                let Some(expression) =
                    source_fixed_dynamic_expression_segment(context, segment_start, cursor)?
                else {
                    return Ok(None);
                };
                expressions.push(expression);
                segment_start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    if !stack.is_empty() || segment_start >= end_index {
        return Ok(None);
    }
    let Some(expression) =
        source_fixed_dynamic_expression_segment(context, segment_start, end_index)?
    else {
        return Ok(None);
    };
    expressions.push(expression);
    Ok(Some(expressions))
}

fn source_fixed_dynamic_expression_segment(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    start_index: usize,
    end_index: usize,
) -> Result<Option<Expression>, SourceFixedColumnsWriteError> {
    if start_index >= end_index {
        return Ok(None);
    }
    let (expression, consumed) = parse_expression_tokens(
        context.tokens,
        start_index,
        end_index,
        &context.module.source,
    )
    .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
        source_name: context.module.source_name.clone(),
        source_span: SourceSpan {
            start: context.tokens[start_index].start,
            end: context.tokens[end_index - 1].end,
        },
        source,
    })?;
    Ok((consumed == end_index).then_some(expression))
}

struct SourceFixedDynamicForLoop {
    body_statements: std::sync::Arc<[FunctionStatement]>,
    variable_name: String,
    start: usize,
    count: usize,
}

struct SourceFixedDynamicLocalForLoop {
    body_statements: std::sync::Arc<[FunctionStatement]>,
    variable_name: String,
    start: usize,
    end: usize,
}

fn source_fixed_dynamic_local_for_loop(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicLocalForLoop>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::For {
        return Ok(None);
    }
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some(variable) = source_fixed_dynamic_for_variable(statement) else {
        return Ok(None);
    };
    let Some(initializer) = variable.initializer_expression.as_ref() else {
        return Ok(None);
    };
    let Some(start) = source_fixed_dynamic_usize_expression(initializer, assignment_values) else {
        return Ok(None);
    };
    let Some(header) = statement.header else {
        return Ok(None);
    };
    let Some(condition) = source_fixed_dynamic_for_header(
        context,
        header,
        body_cache,
        &variable.name,
        assignment_values,
    )?
    else {
        return Ok(None);
    };
    let Some(end) =
        source_fixed_dynamic_for_condition_end(&condition, &variable.name, assignment_values)
    else {
        return Ok(None);
    };
    let Some(count) = end.checked_sub(start) else {
        return Ok(None);
    };
    if count > SOURCE_FIXED_DYNAMIC_LOCAL_FOR_LIMIT {
        return Ok(None);
    }
    let body_statements = body_cache
        .body_statements(context.tokens, body, &context.module.source)
        .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
            source_name: context.module.source_name.clone(),
            source_span: body,
            source,
        })?;
    Ok(Some(SourceFixedDynamicLocalForLoop {
        body_statements,
        variable_name: variable.name.clone(),
        start,
        end,
    }))
}

fn source_fixed_dynamic_for_loop(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
) -> Result<Option<SourceFixedDynamicForLoop>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::For {
        return Ok(None);
    }
    let Some(body) = statement.body else {
        return Ok(None);
    };
    let Some(variable) = source_fixed_dynamic_for_variable(statement) else {
        return Ok(None);
    };
    let Some(initializer) = variable.initializer_expression.as_ref() else {
        return Ok(None);
    };
    let Some(start) = source_fixed_dynamic_usize_expression(initializer, assignment_values) else {
        return Ok(None);
    };
    let Some(header) = statement.header else {
        return Ok(None);
    };
    let Some(condition) = source_fixed_dynamic_for_header(
        context,
        header,
        body_cache,
        &variable.name,
        assignment_values,
    )?
    else {
        return Ok(None);
    };
    let Some(end) =
        source_fixed_dynamic_for_condition_end(&condition, &variable.name, assignment_values)
    else {
        return Ok(None);
    };
    if end < start {
        return Ok(None);
    }
    let count = end - start;
    if start != 0 || count != context.row_count {
        return Ok(None);
    }
    let body_statements = body_cache
        .body_statements(context.tokens, body, &context.module.source)
        .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
            source_name: context.module.source_name.clone(),
            source_span: body,
            source,
        })?;
    Ok(Some(SourceFixedDynamicForLoop {
        body_statements,
        variable_name: variable.name.clone(),
        start,
        count,
    }))
}

fn source_fixed_dynamic_for_variable(
    statement: &FunctionStatement,
) -> Option<&lzvm_pil::VariableDeclaration> {
    let Some(FunctionStatementDeclaration::Variable(declaration)) =
        statement.header_declaration.as_ref()
    else {
        return None;
    };
    if declaration.type_name != "int" || !declaration.array_dims.is_empty() {
        return None;
    }
    Some(declaration)
}

fn source_fixed_dynamic_for_header(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    header: SourceSpan,
    body_cache: &mut SourceControlBodyCache,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Result<Option<Expression>, SourceFixedColumnsWriteError> {
    let Some((open, close)) = source_fixed_header_token_bounds(context.tokens, header, body_cache)
    else {
        return Ok(None);
    };
    let semicolons = source_fixed_for_header_semicolons(context.tokens, open + 1, close);
    let [first_semicolon, second_semicolon] = semicolons.as_slice() else {
        return Ok(None);
    };
    let (condition, consumed) = parse_expression_tokens(
        context.tokens,
        first_semicolon + 1,
        *second_semicolon,
        &context.module.source,
    )
    .map_err(|source| SourceFixedColumnsWriteError::ExpressionParse {
        source_name: context.module.source_name.clone(),
        source_span: header,
        source,
    })?;
    if consumed != *second_semicolon {
        return Ok(None);
    }
    if !source_fixed_dynamic_for_update_is_unit_increment(
        context,
        second_semicolon + 1,
        close,
        variable_name,
        assignment_values,
    )? {
        return Ok(None);
    }
    Ok(Some(condition))
}

fn source_fixed_header_token_bounds(
    tokens: &[Token],
    header: SourceSpan,
    body_cache: &mut SourceControlBodyCache,
) -> Option<(usize, usize)> {
    let (open, close) = body_cache.span_token_bounds(tokens, header)?;
    let close = close.checked_sub(1)?;
    (open < close).then_some((open, close))
}

fn source_fixed_for_header_semicolons(tokens: &[Token], start: usize, end: usize) -> Vec<usize> {
    let mut semicolons = Vec::new();
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                depth = depth.saturating_add(1);
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Semicolon if depth == 0 => semicolons.push(index),
            _ => {}
        }
    }
    semicolons
}

fn source_fixed_dynamic_for_update_is_unit_increment(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    start: usize,
    end: usize,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    if source_fixed_dynamic_for_postfix_delta(context.tokens, start, end, variable_name) == Some(1)
    {
        return Ok(true);
    }
    let (expression, consumed) =
        parse_expression_tokens(context.tokens, start, end, &context.module.source).map_err(
            |source| SourceFixedColumnsWriteError::ExpressionParse {
                source_name: context.module.source_name.clone(),
                source_span: SourceSpan {
                    start: context.tokens.get(start).map_or(0, |token| token.start),
                    end: context
                        .tokens
                        .get(end.saturating_sub(1))
                        .map_or(0, |token| token.end),
                },
                source,
            },
        )?;
    if consumed != end {
        return Ok(false);
    }
    Ok(
        source_fixed_dynamic_for_update_delta(&expression, variable_name, assignment_values)
            == Some(1),
    )
}

fn source_fixed_dynamic_for_postfix_delta(
    tokens: &[Token],
    start: usize,
    end: usize,
    variable_name: &str,
) -> Option<i128> {
    let first = tokens.get(start)?;
    let second = tokens.get(start + 1)?;
    if start + 2 != end || first.lexeme != variable_name {
        return None;
    }
    match second.kind {
        TokenKind::Increment => Some(1),
        TokenKind::Decrement => Some(-1),
        _ => None,
    }
}

fn source_fixed_dynamic_for_update_delta(
    expression: &Expression,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<i128> {
    match &strip_source_fixed_group_expression(expression).kind {
        ExpressionKind::Unary { op, expr } => {
            if !source_fixed_expression_is_loop_variable(expr, variable_name) {
                return None;
            }
            match op {
                UnaryOperator::Increment => Some(1),
                UnaryOperator::Decrement => Some(-1),
                _ => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            if !source_fixed_expression_is_loop_variable(left, variable_name) {
                return None;
            }
            match op {
                BinaryOperator::Assign => {
                    source_fixed_assignment_update_delta(right, variable_name, assignment_values)
                }
                BinaryOperator::PlusAssign => {
                    source_fixed_dynamic_integer_expression(right, assignment_values)
                }
                BinaryOperator::MinusAssign => {
                    source_fixed_dynamic_integer_expression(right, assignment_values)?.checked_neg()
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn source_fixed_assignment_update_delta(
    expression: &Expression,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<i128> {
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return None;
    };
    if !source_fixed_expression_is_loop_variable(left, variable_name) {
        return None;
    }
    let delta = source_fixed_dynamic_integer_expression(right, assignment_values)?;
    match op {
        BinaryOperator::Add => Some(delta),
        BinaryOperator::Subtract => delta.checked_neg(),
        _ => None,
    }
}

fn source_fixed_dynamic_for_condition_end(
    expression: &Expression,
    variable_name: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<usize> {
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return None;
    };
    if !source_fixed_expression_is_loop_variable(left, variable_name) {
        return None;
    }
    let right = source_fixed_dynamic_usize_expression(right, assignment_values)?;
    match op {
        BinaryOperator::Less => Some(right),
        BinaryOperator::LessEqual => right.checked_add(1),
        _ => None,
    }
}

fn source_fixed_dynamic_assignment_statement(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    loop_variable: &str,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Result<Option<(String, Expression)>, SourceFixedColumnsWriteError> {
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(None);
    }
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(None);
    };
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return Ok(None);
    };
    if *op != BinaryOperator::Assign {
        return Ok(None);
    }
    let Some(target_column) = source_fixed_dynamic_assignment_target(
        left,
        loop_variable,
        context.expected_columns,
        context.logical_dimensions,
        assignment_values,
    ) else {
        return Ok(None);
    };
    Ok(Some((target_column, (**right).clone())))
}

fn source_fixed_dynamic_assignment_target(
    expression: &Expression,
    loop_variable: &str,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<String> {
    let ExpressionKind::Index { target, index } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return None;
    };
    if !source_fixed_expression_is_loop_variable(index, loop_variable) {
        return None;
    }
    source_fixed_physical_assignment_column_name(
        target,
        expected_columns,
        logical_dimensions,
        values,
    )
}

fn source_fixed_expression_is_loop_variable(expression: &Expression, variable_name: &str) -> bool {
    match &strip_source_fixed_group_expression(expression).kind {
        ExpressionKind::Name(name) => name == variable_name,
        _ => false,
    }
}

fn source_fixed_dynamic_usize_expression(
    expression: &Expression,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<usize> {
    let value = source_fixed_dynamic_integer_expression(expression, assignment_values)?;
    usize::try_from(value).ok()
}

fn source_fixed_dynamic_integer_expression(
    expression: &Expression,
    assignment_values: &SourceFixedAssignmentValues<'_>,
) -> Option<i128> {
    let value = evaluate_source_fixed_assignment_value_expression(expression, assignment_values)?;
    source_fixed_assignment_integer(&value)
}

fn source_fixed_dynamic_range_end(
    operation: &SourceFixedDynamicOperation,
    row_count: usize,
) -> Result<usize, SourceFixedColumnsWriteError> {
    let end = operation
        .start
        .checked_add(operation.count)
        .ok_or_else(|| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: operation.source_name.clone(),
            source_span: operation.source_span,
            expression: operation.count.to_string(),
        })?;
    if end > row_count {
        return Err(SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: operation.source_name.clone(),
            source_span: operation.source_span,
            expression: end.to_string(),
        });
    }
    Ok(end)
}

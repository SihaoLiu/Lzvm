use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::expression_info::{
    BoundaryKind, CodeDestination, CodeOperand, CodeOperation, ConstraintCode, OperationKind,
};
use lzvm_field::{Felt, MODULUS};
use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, FunctionStatement,
    FunctionStatementKind, SourceProgram, SourceProgramModule, UnaryOperator,
};

use crate::{
    source_control_body_cache::{
        SourceConstraintFragment, SourceControlBodyCache, SourceReturnedConstraintElementKey,
    },
    source_expression_info::{
        source_call_expression, source_function_call_bindings, SourceExpressionAliasScope,
    },
    source_expression_return_values::{
        collect_source_expr_destructuring_aliases,
        collect_source_template_expression_aliases_with_stack, source_function_returns_expr,
        source_returned_array_call_cacheable, source_returned_array_call_key,
        source_returned_expression_array_call_alias_cached,
    },
    source_expression_statements::{
        apply_source_static_declaration, apply_source_static_expression_statement,
    },
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_scalar_slots::SourceScalarSlots,
    source_statement_hints::{SourceExpressionArrayAlias, SourceExpressionArrayAliases},
    source_static_values::{
        evaluate_source_static_expression, static_value_integer, static_value_truthy,
    },
    source_template_context::SourceTemplateLoweringContext,
    source_template_for::source_static_for_loop_with_tokens,
    source_template_if::source_static_if_body_statements_with_aliases,
    source_template_while::{source_static_while_loop_with_tokens, STATIC_WHILE_LOOP_LIMIT},
};

pub(crate) type SourceExpressionAliases = BTreeMap<String, Expression>;

pub(crate) fn lower_source_template_boolean_constraint(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    scalar_slots: &SourceScalarSlots,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
) -> Result<Option<ConstraintCode>, SourceKeyDirectoryMetadataError> {
    lower_source_template_boolean_constraint_inner(
        program,
        module,
        statement,
        scalar_slots,
        constant_values,
        alias_scope,
        None,
    )
}

pub(crate) fn lower_source_template_boolean_constraint_with_returned_calls(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    body_cache: &mut SourceControlBodyCache,
    call_stack: &mut BTreeSet<String>,
) -> Result<Option<ConstraintCode>, SourceKeyDirectoryMetadataError> {
    lower_source_template_boolean_constraint_inner(
        context.program,
        context.module,
        statement,
        context.scalar_slots,
        constant_values,
        alias_scope,
        Some(SourceConstraintReturnedCallContext {
            context,
            body_cache,
            call_stack,
        }),
    )
}

fn lower_source_template_boolean_constraint_inner(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    scalar_slots: &SourceScalarSlots,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
    returned_call_context: Option<SourceConstraintReturnedCallContext<'_>>,
) -> Result<Option<ConstraintCode>, SourceKeyDirectoryMetadataError> {
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(None);
    };
    let mut state = SourceConstraintLoweringState {
        program,
        scalar_slots,
        constant_values,
        alias_scope,
        operations: Vec::new(),
        next_temporary: 0,
        frame_offsets: SourceConstraintFrameOffsets::default(),
        resolving_aliases: BTreeSet::new(),
        resolving_array_aliases: BTreeSet::new(),
        operand_cache: BTreeMap::new(),
        returned_call_context,
    };
    let Some(result) = lower_source_constraint_residual(expression, &mut state)? else {
        return Ok(None);
    };
    if !matches!(result, CodeOperand::Temporary { .. }) {
        push_source_copy_operation(&mut state, result)?;
    }
    if state.operations.is_empty() {
        return Ok(None);
    }
    let (boundary, offset_min, offset_max) = state.frame_offsets.boundary()?;
    Ok(Some(ConstraintCode {
        stage: 1,
        boundary,
        offset_min,
        offset_max,
        line: module.source.contents[statement.start..statement.end]
            .trim()
            .to_owned(),
        intermediate: false,
        temporary_count: state.next_temporary,
        operations: state.operations,
    }))
}

struct SourceConstraintLoweringState<'a> {
    program: &'a SourceProgram,
    scalar_slots: &'a SourceScalarSlots,
    constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &'a SourceExpressionAliasScope,
    operations: Vec<CodeOperation>,
    next_temporary: u32,
    frame_offsets: SourceConstraintFrameOffsets,
    resolving_aliases: BTreeSet<String>,
    resolving_array_aliases: BTreeSet<SourceConstraintArrayResolutionKey>,
    operand_cache: BTreeMap<SourceConstraintOperandCacheKey, CodeOperand>,
    returned_call_context: Option<SourceConstraintReturnedCallContext<'a>>,
}

struct SourceConstraintReturnedCallContext<'a> {
    context: &'a SourceTemplateLoweringContext<'a>,
    body_cache: &'a mut SourceControlBodyCache,
    call_stack: &'a mut BTreeSet<String>,
}

#[derive(Clone, Copy)]
struct SourceConstraintAliasEnvironment<'a> {
    expression_aliases: &'a SourceExpressionAliases,
    expression_array_aliases: &'a SourceExpressionArrayAliases,
    scope: Option<&'a SourceExpressionAliasScope>,
}

impl SourceConstraintAliasEnvironment<'_> {
    fn expression_alias_id(self) -> usize {
        self.expression_aliases as *const SourceExpressionAliases as usize
    }

    fn expression_array_alias_id(self) -> usize {
        self.expression_array_aliases as *const SourceExpressionArrayAliases as usize
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
enum SourceConstraintOperandCacheKey {
    ExpressionAlias {
        name: String,
        row_offset: i64,
        expression_alias_id: usize,
        expression_array_alias_id: usize,
    },
    ArrayAliasElement {
        name: String,
        indices: Vec<u32>,
        row_offset: i64,
        expression_alias_id: usize,
        expression_array_alias_id: usize,
    },
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct SourceConstraintArrayResolutionKey {
    name: String,
    indices: Vec<u32>,
    expression_alias_id: usize,
    expression_array_alias_id: usize,
}

fn lower_source_constraint_residual(
    expression: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    let expression = strip_group_expression(expression);
    if let ExpressionKind::Binary { op, left, right } = &expression.kind {
        if matches!(
            op,
            BinaryOperator::TripleEqual | BinaryOperator::ConstrainedAssign
        ) {
            if expression_is_zero(right) {
                return lower_source_scalar_expression_at(left, state, 0).map(Some);
            } else if expression_is_zero(left) {
                return lower_source_scalar_expression_at(right, state, 0).map(Some);
            }

            let left = lower_source_scalar_expression_at(left, state, 0)?;
            let right = lower_source_scalar_expression_at(right, state, 0)?;
            let dimension = source_binary_result_dimension(&left, &right)?;
            let id = state.next_temporary;
            state.next_temporary = state.next_temporary.checked_add(1).ok_or_else(|| {
                unsupported_source_message("source scalar constraint temporary overflow")
            })?;
            state.operations.push(CodeOperation {
                op: OperationKind::Sub,
                destination: CodeDestination::temporary(id, dimension),
                sources: vec![left, right],
            });
            return Ok(Some(CodeOperand::temporary(id, dimension)));
        }
    }

    if source_expression_can_be_bare_constraint(expression) {
        lower_source_scalar_expression_at(expression, state, 0).map(Some)
    } else {
        Ok(None)
    }
}

fn source_expression_can_be_bare_constraint(expression: &Expression) -> bool {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Integer(_)
        | ExpressionKind::HexInteger(_)
        | ExpressionKind::Name(_)
        | ExpressionKind::Unary { .. }
        | ExpressionKind::Binary { .. }
        | ExpressionKind::Index { .. }
        | ExpressionKind::RowOffset { .. } => true,
        ExpressionKind::StringLiteral(_)
        | ExpressionKind::TemplateLiteral(_)
        | ExpressionKind::PositionalParam(_)
        | ExpressionKind::Group(_)
        | ExpressionKind::Array(_)
        | ExpressionKind::Ternary { .. }
        | ExpressionKind::Call { .. } => false,
    }
}

fn lower_source_scalar_expression_at(
    expression: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let aliases = SourceConstraintAliasEnvironment {
        expression_aliases: state.alias_scope.expressions.as_ref(),
        expression_array_aliases: state.alias_scope.expression_arrays.as_ref(),
        scope: Some(state.alias_scope),
    };
    lower_source_scalar_expression_in_env(expression, state, row_offset, aliases)
}

fn lower_source_scalar_expression_in_env(
    expression: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
    aliases: SourceConstraintAliasEnvironment<'_>,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let expression = strip_group_expression(expression);
    if let Some(value) = static_scalar_integer(expression, state)? {
        return Ok(CodeOperand::number(canonical_field_value(value)?, 1));
    }
    match &expression.kind {
        ExpressionKind::Name(name) => {
            if let Some(alias) = aliases.expression_aliases.get(name) {
                let key = SourceConstraintOperandCacheKey::ExpressionAlias {
                    name: name.clone(),
                    row_offset,
                    expression_alias_id: aliases.expression_alias_id(),
                    expression_array_alias_id: aliases.expression_array_alias_id(),
                };
                if let Some(operand) = state.operand_cache.get(&key) {
                    return Ok(operand.clone());
                }
                if !state.resolving_aliases.insert(name.clone()) {
                    return unsupported("source scalar constraint expression alias cycle");
                }
                let operand =
                    lower_source_scalar_expression_in_env(alias, state, row_offset, aliases);
                state.resolving_aliases.remove(name);
                if let Ok(operand) = operand.as_ref() {
                    state.operand_cache.insert(key, operand.clone());
                }
                return operand;
            }
            if row_offset != 0 {
                state.frame_offsets.include(row_offset);
                return state
                    .scalar_slots
                    .operand_at(name, row_offset)
                    .map_err(|error| unsupported_source_message(error.to_string()));
            }
            state
                .scalar_slots
                .operand(name)
                .map_err(|error| unsupported_source_message(error.to_string()))
        }
        ExpressionKind::Unary { op, expr } => {
            let value = lower_source_scalar_expression_in_env(expr, state, row_offset, aliases)?;
            match op {
                UnaryOperator::Plus => Ok(value),
                UnaryOperator::Minus => {
                    let dimension = source_operand_dimension(&value)?;
                    let id = state.next_temporary;
                    state.next_temporary =
                        state.next_temporary.checked_add(1).ok_or_else(|| {
                            unsupported_source_message(
                                "source scalar constraint temporary overflow",
                            )
                        })?;
                    state.operations.push(CodeOperation {
                        op: OperationKind::Sub,
                        destination: CodeDestination::temporary(id, dimension),
                        sources: vec![CodeOperand::number(0, 1), value],
                    });
                    Ok(CodeOperand::temporary(id, dimension))
                }
                _ => unsupported("unsupported source scalar constraint expression"),
            }
        }
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => {
            let signed_offset = source_row_offset_value(offset, *prior, state)?;
            let combined_offset = row_offset
                .checked_add(signed_offset)
                .ok_or_else(|| unsupported_source_message("source row offset overflow"))?;
            lower_source_scalar_expression_in_env(target, state, combined_offset, aliases)
        }
        ExpressionKind::Index { .. } => {
            let Some((name, index_expressions)) =
                source_constraint_index_chain(strip_group_expression(expression))
            else {
                if let Some(operand) = lower_source_returned_array_call_expression(
                    expression, state, row_offset, aliases,
                )? {
                    return Ok(operand);
                }
                return Err(unsupported_source_message(
                    "unsupported source indexed constraint target",
                ));
            };
            let indices = index_expressions
                .iter()
                .map(|index| source_scalar_index_value(index, state))
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(alias) = aliases.expression_array_aliases.get(name) {
                let key = SourceConstraintOperandCacheKey::ArrayAliasElement {
                    name: name.to_owned(),
                    indices: indices.clone(),
                    row_offset,
                    expression_alias_id: aliases.expression_alias_id(),
                    expression_array_alias_id: aliases.expression_array_alias_id(),
                };
                if let Some(operand) = state.operand_cache.get(&key) {
                    return Ok(operand.clone());
                }
                let resolution_key = SourceConstraintArrayResolutionKey {
                    name: name.to_owned(),
                    indices: indices.clone(),
                    expression_alias_id: aliases.expression_alias_id(),
                    expression_array_alias_id: aliases.expression_array_alias_id(),
                };
                if !state.resolving_array_aliases.insert(resolution_key.clone()) {
                    return unsupported("source scalar constraint expression array alias cycle");
                }
                let mut resolving_array_alias_names = BTreeSet::new();
                let element = source_constraint_array_alias_path_element(
                    alias,
                    &indices,
                    aliases,
                    &mut resolving_array_alias_names,
                );
                let element = element.ok_or_else(|| {
                    unsupported_source_message("unsupported source indexed constraint target")
                });
                let result = match element? {
                    SourceConstraintArrayAliasElement::Expression {
                        expression,
                        aliases,
                    } => lower_source_scalar_expression_in_env(
                        expression, state, row_offset, aliases,
                    ),
                    SourceConstraintArrayAliasElement::ReturnedCall {
                        expression,
                        aliases,
                    } => lower_source_returned_array_call_expression(
                        &expression,
                        state,
                        row_offset,
                        aliases,
                    )?
                    .ok_or_else(|| {
                        unsupported_source_message("unsupported source indexed constraint target")
                    }),
                    SourceConstraintArrayAliasElement::NamedArray(name) => {
                        if row_offset != 0 {
                            state.frame_offsets.include(row_offset);
                        }
                        state
                            .scalar_slots
                            .operand_indices_at(name, &indices, row_offset)
                            .map_err(|error| unsupported_source_message(error.to_string()))
                    }
                };
                state.resolving_array_aliases.remove(&resolution_key);
                if let Ok(operand) = result.as_ref() {
                    state.operand_cache.insert(key, operand.clone());
                }
                return result;
            }
            if row_offset != 0 {
                state.frame_offsets.include(row_offset);
            }
            state
                .scalar_slots
                .operand_indices_at(name, &indices, row_offset)
                .map_err(|error| unsupported_source_message(error.to_string()))
        }
        ExpressionKind::Binary { op, left, right } => {
            if matches!(op, BinaryOperator::Divide | BinaryOperator::Backslash) {
                return lower_source_static_divisor_expression(
                    left, right, state, row_offset, aliases,
                );
            }
            if *op == BinaryOperator::Power {
                return lower_source_static_exponent_expression(
                    left, right, state, row_offset, aliases,
                );
            }
            let op = match op {
                BinaryOperator::Add => OperationKind::Add,
                BinaryOperator::Subtract => OperationKind::Sub,
                BinaryOperator::Multiply => OperationKind::Mul,
                _ => return unsupported("unsupported source scalar constraint expression"),
            };
            let left = lower_source_scalar_expression_in_env(left, state, row_offset, aliases)?;
            let right = lower_source_scalar_expression_in_env(right, state, row_offset, aliases)?;
            let dimension = source_binary_result_dimension(&left, &right)?;
            let id = state.next_temporary;
            state.next_temporary = state.next_temporary.checked_add(1).ok_or_else(|| {
                unsupported_source_message("source scalar constraint temporary overflow")
            })?;
            state.operations.push(CodeOperation {
                op,
                destination: CodeDestination::temporary(id, dimension),
                sources: vec![left, right],
            });
            Ok(CodeOperand::temporary(id, dimension))
        }
        ExpressionKind::Call { .. } => {
            if let Some(operand) = lower_source_returned_scalar_call_expression(
                expression, state, row_offset, aliases,
            )? {
                return Ok(operand);
            }
            unsupported("unsupported source scalar constraint expression")
        }
        _ => unsupported("unsupported source scalar constraint expression"),
    }
}

enum SourceConstraintArrayAliasElement<'a> {
    Expression {
        expression: &'a Expression,
        aliases: SourceConstraintAliasEnvironment<'a>,
    },
    ReturnedCall {
        expression: Expression,
        aliases: SourceConstraintAliasEnvironment<'a>,
    },
    NamedArray(&'a str),
}

fn lower_source_returned_array_call_expression(
    expression: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
    aliases: SourceConstraintAliasEnvironment<'_>,
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    let ExpressionKind::Index { target, index } = &strip_group_expression(expression).kind else {
        return Ok(None);
    };
    let index = source_scalar_index_value(index, state)?;
    let use_fragment_cache = aliases
        .scope
        .is_some_and(|scope| source_returned_array_call_cacheable(target, scope));
    let key = SourceReturnedConstraintElementKey::new(
        source_returned_array_call_key(target, state.constant_values),
        vec![index],
        row_offset,
    );
    if use_fragment_cache {
        let Some(returned) = state.returned_call_context.as_ref() else {
            return Ok(None);
        };
        if let Some(fragment) = returned.body_cache.returned_constraint_array_element(&key) {
            return append_source_constraint_fragment(state, fragment.as_ref());
        }
    }

    let start_operation = state.operations.len();
    let start_temporary = state.next_temporary;
    let result = lower_source_returned_array_call_expression_uncached(
        expression, state, row_offset, aliases,
    );
    let fragment = match result.as_ref() {
        Ok(Some(result)) => Some(source_constraint_fragment_from_operations(
            &state.operations[start_operation..],
            result,
            start_temporary,
            state.next_temporary,
        )?),
        Ok(None) => None,
        Err(_) => None,
    };
    if use_fragment_cache {
        let Some(returned) = state.returned_call_context.as_mut() else {
            return result;
        };
        returned
            .body_cache
            .insert_returned_constraint_array_element(key, fragment);
    }
    result
}

fn lower_source_returned_array_call_expression_uncached(
    expression: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
    aliases: SourceConstraintAliasEnvironment<'_>,
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    let Some(alias_scope) = aliases.scope else {
        return Ok(None);
    };
    let ExpressionKind::Index { target, index } = &strip_group_expression(expression).kind else {
        return Ok(None);
    };
    let index = source_scalar_index_value(index, state)?;
    let Some(alias) = ({
        let Some(returned) = state.returned_call_context.as_mut() else {
            return Ok(None);
        };
        source_returned_expression_array_call_alias_cached(
            returned.context,
            target,
            state.constant_values,
            alias_scope,
            returned.body_cache,
            returned.call_stack,
        )
    }) else {
        return Ok(None);
    };
    let indices = [index];
    let mut resolving_array_alias_names = BTreeSet::new();
    let Some(element) = source_constraint_array_alias_path_element(
        &alias,
        &indices,
        aliases,
        &mut resolving_array_alias_names,
    ) else {
        return Ok(None);
    };
    match element {
        SourceConstraintArrayAliasElement::Expression {
            expression,
            aliases,
        } => {
            lower_source_scalar_expression_in_env(expression, state, row_offset, aliases).map(Some)
        }
        SourceConstraintArrayAliasElement::ReturnedCall {
            expression,
            aliases,
        } => lower_source_returned_array_call_expression(&expression, state, row_offset, aliases),
        SourceConstraintArrayAliasElement::NamedArray(name) => {
            if row_offset != 0 {
                state.frame_offsets.include(row_offset);
            }
            state
                .scalar_slots
                .operand_indices_at(name, &indices, row_offset)
                .map(Some)
                .map_err(|error| unsupported_source_message(error.to_string()))
        }
    }
}

fn append_source_constraint_fragment(
    state: &mut SourceConstraintLoweringState<'_>,
    fragment: Option<&SourceConstraintFragment>,
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    let Some(fragment) = fragment else {
        return Ok(None);
    };
    let temporary_base = state.next_temporary;
    state.next_temporary = state
        .next_temporary
        .checked_add(fragment.temporary_count)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    state.frame_offsets.include(fragment.offset_min);
    state.frame_offsets.include(fragment.offset_max);
    state.operations.extend(
        fragment
            .operations
            .iter()
            .map(|operation| remap_source_constraint_operation(operation, temporary_base))
            .collect::<Result<Vec<_>, _>>()?,
    );
    remap_source_constraint_operand(&fragment.result, temporary_base).map(Some)
}

fn source_constraint_fragment_from_operations(
    operations: &[CodeOperation],
    result: &CodeOperand,
    start_temporary: u32,
    next_temporary: u32,
) -> Result<SourceConstraintFragment, SourceKeyDirectoryMetadataError> {
    let temporary_count = next_temporary
        .checked_sub(start_temporary)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    let operations = operations
        .iter()
        .map(|operation| normalize_source_constraint_operation(operation, start_temporary))
        .collect::<Result<Vec<_>, _>>()?;
    let result = normalize_source_constraint_operand(result, start_temporary)?;
    let mut frame_offsets = SourceConstraintFrameOffsets::default();
    for operation in &operations {
        include_source_destination_offsets(&operation.destination, &mut frame_offsets);
        for source in &operation.sources {
            include_source_operand_offsets(source, &mut frame_offsets);
        }
    }
    include_source_operand_offsets(&result, &mut frame_offsets);
    Ok(SourceConstraintFragment {
        operations,
        result,
        temporary_count,
        offset_min: frame_offsets.min,
        offset_max: frame_offsets.max,
    })
}

fn normalize_source_constraint_operation(
    operation: &CodeOperation,
    temporary_base: u32,
) -> Result<CodeOperation, SourceKeyDirectoryMetadataError> {
    Ok(CodeOperation {
        op: operation.op,
        destination: normalize_source_constraint_destination(
            &operation.destination,
            temporary_base,
        )?,
        sources: operation
            .sources
            .iter()
            .map(|source| normalize_source_constraint_operand(source, temporary_base))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn remap_source_constraint_operation(
    operation: &CodeOperation,
    temporary_base: u32,
) -> Result<CodeOperation, SourceKeyDirectoryMetadataError> {
    Ok(CodeOperation {
        op: operation.op,
        destination: remap_source_constraint_destination(&operation.destination, temporary_base)?,
        sources: operation
            .sources
            .iter()
            .map(|source| remap_source_constraint_operand(source, temporary_base))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn normalize_source_constraint_destination(
    destination: &CodeDestination,
    temporary_base: u32,
) -> Result<CodeDestination, SourceKeyDirectoryMetadataError> {
    match destination {
        CodeDestination::Temporary { id, dimension } => Ok(CodeDestination::temporary(
            id.checked_sub(temporary_base).ok_or_else(|| {
                unsupported_source_message("source scalar constraint temporary overflow")
            })?,
            *dimension,
        )),
        _ => unsupported("unsupported source scalar constraint destination"),
    }
}

fn remap_source_constraint_destination(
    destination: &CodeDestination,
    temporary_base: u32,
) -> Result<CodeDestination, SourceKeyDirectoryMetadataError> {
    match destination {
        CodeDestination::Temporary { id, dimension } => Ok(CodeDestination::temporary(
            id.checked_add(temporary_base).ok_or_else(|| {
                unsupported_source_message("source scalar constraint temporary overflow")
            })?,
            *dimension,
        )),
        _ => unsupported("unsupported source scalar constraint destination"),
    }
}

fn normalize_source_constraint_operand(
    operand: &CodeOperand,
    temporary_base: u32,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    remap_source_constraint_operand_with(operand, |id| {
        id.checked_sub(temporary_base).ok_or_else(|| {
            unsupported_source_message("source scalar constraint temporary overflow")
        })
    })
}

fn remap_source_constraint_operand(
    operand: &CodeOperand,
    temporary_base: u32,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    remap_source_constraint_operand_with(operand, |id| {
        id.checked_add(temporary_base).ok_or_else(|| {
            unsupported_source_message("source scalar constraint temporary overflow")
        })
    })
}

fn remap_source_constraint_operand_with(
    operand: &CodeOperand,
    remap_temporary: impl Fn(u32) -> Result<u32, SourceKeyDirectoryMetadataError>,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    match operand {
        CodeOperand::Temporary { id, dimension } => {
            Ok(CodeOperand::temporary(remap_temporary(*id)?, *dimension))
        }
        _ => Ok(operand.clone()),
    }
}

fn include_source_destination_offsets(
    _destination: &CodeDestination,
    _frame_offsets: &mut SourceConstraintFrameOffsets,
) {
}

fn include_source_operand_offsets(
    operand: &CodeOperand,
    frame_offsets: &mut SourceConstraintFrameOffsets,
) {
    match operand {
        CodeOperand::ConstantAt { prime, .. }
        | CodeOperand::Commitment { prime, .. }
        | CodeOperand::CommitmentElement { prime, .. }
        | CodeOperand::CustomCommitment { prime, .. } => {
            if let Some(offset) = prime {
                frame_offsets.include(*offset);
            }
        }
        _ => {}
    }
}

fn lower_source_returned_scalar_call_expression(
    expression: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
    aliases: SourceConstraintAliasEnvironment<'_>,
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    let Some(alias_scope) = aliases.scope else {
        return Ok(None);
    };
    let Some((name, arguments)) = source_call_expression(Some(expression)) else {
        return Ok(None);
    };
    let Some(context) = state
        .returned_call_context
        .as_ref()
        .map(|returned| returned.context)
    else {
        return Ok(None);
    };
    let Some(function) = context
        .module
        .functions
        .iter()
        .find(|function| function.name == name)
    else {
        return Ok(None);
    };
    if !source_function_returns_expr(context.module, function) {
        return Ok(None);
    }
    let Some(mut bindings) = ({
        let Some(returned) = state.returned_call_context.as_mut() else {
            return Ok(None);
        };
        source_function_call_bindings(
            context,
            function,
            arguments,
            state.constant_values,
            alias_scope,
            returned.body_cache,
            returned.call_stack,
        )
    }) else {
        return Ok(None);
    };
    {
        let Some(returned) = state.returned_call_context.as_mut() else {
            return Ok(None);
        };
        if !returned.call_stack.insert(function.name.clone()) {
            return Ok(None);
        }
    }
    let mut body_alias_scope = bindings.alias_scope;
    let result = lower_source_returned_scalar_body(
        context,
        &function.statements,
        &mut bindings.values,
        &mut body_alias_scope,
        state,
        row_offset,
    );
    if let Some(returned) = state.returned_call_context.as_mut() {
        returned.call_stack.remove(&function.name);
    }
    result
}

fn lower_source_returned_scalar_body(
    context: &SourceTemplateLoweringContext<'_>,
    statements: &[FunctionStatement],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &mut SourceExpressionAliasScope,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    for statement in statements {
        if statement.kind == FunctionStatementKind::Return {
            let Some(expression) = statement.value_expression.as_ref() else {
                return Ok(None);
            };
            let aliases = SourceConstraintAliasEnvironment {
                expression_aliases: alias_scope.expressions.as_ref(),
                expression_array_aliases: alias_scope.expression_arrays.as_ref(),
                scope: Some(alias_scope),
            };
            return lower_source_scalar_expression_in_env(expression, state, row_offset, aliases)
                .map(Some);
        }
        if statement.kind == FunctionStatementKind::If {
            let body = {
                let Some(returned) = state.returned_call_context.as_mut() else {
                    return Ok(None);
                };
                source_static_if_body_statements_with_aliases(
                    context.program,
                    context.module,
                    context.tokens,
                    statement,
                    values,
                    &alias_scope.expressions,
                    returned.body_cache,
                )
            };
            match body {
                Ok(Some(body)) => {
                    if let Some(operand) = lower_source_returned_scalar_body(
                        context,
                        &body,
                        values,
                        alias_scope,
                        state,
                        row_offset,
                    )? {
                        return Ok(Some(operand));
                    }
                }
                Ok(None)
                | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
                Err(error) => return Err(error),
            }
            continue;
        }
        if statement.kind == FunctionStatementKind::For {
            let loop_info = {
                let Some(returned) = state.returned_call_context.as_mut() else {
                    return Ok(None);
                };
                source_static_for_loop_with_tokens(
                    context.program,
                    context.module,
                    context.tokens,
                    statement,
                    values,
                    returned.body_cache,
                )
            };
            match loop_info {
                Ok(Some(loop_info)) => {
                    for iteration_value in &loop_info.iteration_values {
                        values.insert(loop_info.variable_name.clone(), iteration_value.clone());
                        if let Some(operand) = lower_source_returned_scalar_body(
                            context,
                            &loop_info.body_statements,
                            values,
                            alias_scope,
                            state,
                            row_offset,
                        )? {
                            return Ok(Some(operand));
                        }
                    }
                    loop_info.apply_final_variable_value(values);
                }
                Ok(None)
                | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
                Err(error) => return Err(error),
            }
            continue;
        }
        if statement.kind == FunctionStatementKind::While {
            let loop_info = {
                let Some(returned) = state.returned_call_context.as_mut() else {
                    return Ok(None);
                };
                source_static_while_loop_with_tokens(
                    context.program,
                    context.module,
                    context.tokens,
                    statement,
                    values,
                    returned.body_cache,
                )
            };
            match loop_info {
                Ok(Some(loop_info)) => {
                    for _ in 0..STATIC_WHILE_LOOP_LIMIT {
                        let Some(condition_value) = evaluate_source_static_expression(
                            context.program,
                            &loop_info.condition,
                            values,
                        ) else {
                            return Ok(None);
                        };
                        if !static_value_truthy(&condition_value) {
                            break;
                        }
                        if let Some(operand) = lower_source_returned_scalar_body(
                            context,
                            &loop_info.body_statements,
                            values,
                            alias_scope,
                            state,
                            row_offset,
                        )? {
                            return Ok(Some(operand));
                        }
                    }
                }
                Ok(None)
                | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
                Err(error) => return Err(error),
            }
            continue;
        }
        apply_source_static_declaration(context.program, statement, values);
        apply_source_static_expression_statement(
            context.program,
            statement.value_expression.as_ref(),
            values,
        );
        let collected_destructuring = {
            let Some(returned) = state.returned_call_context.as_mut() else {
                return Ok(None);
            };
            collect_source_expr_destructuring_aliases(
                context,
                statement,
                values,
                returned.body_cache,
                returned.call_stack,
                alias_scope,
            )
        };
        if collected_destructuring {
            continue;
        }
        let Some(returned) = state.returned_call_context.as_mut() else {
            return Ok(None);
        };
        collect_source_template_expression_aliases_with_stack(
            context,
            statement,
            values,
            returned.body_cache,
            returned.call_stack,
            alias_scope,
        );
    }
    Ok(None)
}

fn source_constraint_array_alias_path_element<'a>(
    alias: &'a SourceExpressionArrayAlias,
    indices: &[u32],
    aliases: SourceConstraintAliasEnvironment<'a>,
    resolving_array_alias_names: &mut BTreeSet<String>,
) -> Option<SourceConstraintArrayAliasElement<'a>> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if let Some(next_alias) = aliases.expression_array_aliases.get(name) {
                if !resolving_array_alias_names.insert(name.clone()) {
                    return None;
                }
                let element = source_constraint_array_alias_path_element(
                    next_alias,
                    indices,
                    aliases,
                    resolving_array_alias_names,
                );
                resolving_array_alias_names.remove(name);
                return element;
            }
            Some(SourceConstraintArrayAliasElement::NamedArray(name))
        }
        SourceExpressionArrayAlias::Values(expressions) => {
            source_constraint_expression_array_element(
                expressions,
                indices,
                aliases,
                resolving_array_alias_names,
            )
        }
        SourceExpressionArrayAlias::ScopedValues { expressions, scope } => {
            source_constraint_expression_array_element(
                expressions,
                indices,
                SourceConstraintAliasEnvironment {
                    expression_aliases: scope.expressions.as_ref(),
                    expression_array_aliases: scope.expression_arrays.as_ref(),
                    scope: Some(scope.as_ref()),
                },
                resolving_array_alias_names,
            )
        }
        SourceExpressionArrayAlias::Call { expression, .. } => {
            Some(SourceConstraintArrayAliasElement::ReturnedCall {
                expression: source_constraint_indexed_expression(
                    expression.as_ref().clone(),
                    indices,
                )?,
                aliases,
            })
        }
    }
}

fn source_constraint_indexed_expression(target: Expression, indices: &[u32]) -> Option<Expression> {
    let source_name = target.source_name.clone();
    let start = target.start;
    let end = target.end;
    indices.iter().try_fold(target, |target, index| {
        Some(Expression {
            kind: ExpressionKind::Index {
                target: Box::new(target),
                index: Box::new(Expression {
                    kind: ExpressionKind::Integer(index.to_string()),
                    source_name: source_name.clone(),
                    start,
                    end,
                }),
            },
            source_name: source_name.clone(),
            start,
            end,
        })
    })
}

fn source_constraint_expression_array_element<'a>(
    expressions: &'a [Expression],
    indices: &[u32],
    aliases: SourceConstraintAliasEnvironment<'a>,
    resolving_array_alias_names: &mut BTreeSet<String>,
) -> Option<SourceConstraintArrayAliasElement<'a>> {
    let (index, rest) = indices.split_first()?;
    let expression = expressions.get(usize::try_from(*index).ok()?)?;
    if rest.is_empty() {
        return Some(SourceConstraintArrayAliasElement::Expression {
            expression,
            aliases,
        });
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Array(expressions) => source_constraint_expression_array_element(
            expressions,
            rest,
            aliases,
            resolving_array_alias_names,
        ),
        ExpressionKind::Name(name) => {
            let alias = aliases.expression_array_aliases.get(name)?;
            source_constraint_array_alias_path_element(
                alias,
                rest,
                aliases,
                resolving_array_alias_names,
            )
        }
        _ => None,
    }
}

fn lower_source_static_divisor_expression(
    left: &Expression,
    right: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
    aliases: SourceConstraintAliasEnvironment<'_>,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let Some(divisor) = static_scalar_integer(right, state)? else {
        return unsupported("unsupported source scalar constraint expression");
    };
    let divisor = Felt::from_u64(canonical_field_value(divisor)?);
    let inverse = divisor
        .inverse()
        .ok_or_else(|| unsupported_source_message("source scalar constraint division by zero"))?;
    let left = lower_source_scalar_expression_in_env(left, state, row_offset, aliases)?;
    let dimension = source_operand_dimension(&left)?;
    let id = state.next_temporary;
    state.next_temporary = state
        .next_temporary
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    state.operations.push(CodeOperation {
        op: OperationKind::Mul,
        destination: CodeDestination::temporary(id, dimension),
        sources: vec![left, CodeOperand::number(inverse.to_u64(), 1)],
    });
    Ok(CodeOperand::temporary(id, dimension))
}

fn lower_source_static_exponent_expression(
    left: &Expression,
    right: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
    aliases: SourceConstraintAliasEnvironment<'_>,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let Some(exponent) = static_scalar_integer(right, state)? else {
        return unsupported("unsupported source scalar constraint expression");
    };
    let mut exponent = u64::try_from(exponent)
        .map_err(|_| unsupported_source_message("source scalar constraint exponent overflow"))?;
    if exponent == 0 {
        return Ok(CodeOperand::number(1, 1));
    }
    let mut power = lower_source_scalar_expression_in_env(left, state, row_offset, aliases)?;
    let mut result = None;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = Some(match result {
                Some(value) => push_source_mul_operation(state, value, power.clone())?,
                None => power.clone(),
            });
        }
        exponent >>= 1;
        if exponent > 0 {
            power = push_source_mul_operation(state, power.clone(), power)?;
        }
    }
    Ok(result.expect("nonzero exponent should produce a result"))
}

fn push_source_mul_operation(
    state: &mut SourceConstraintLoweringState<'_>,
    left: CodeOperand,
    right: CodeOperand,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let dimension = source_binary_result_dimension(&left, &right)?;
    let id = state.next_temporary;
    state.next_temporary = state
        .next_temporary
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    state.operations.push(CodeOperation {
        op: OperationKind::Mul,
        destination: CodeDestination::temporary(id, dimension),
        sources: vec![left, right],
    });
    Ok(CodeOperand::temporary(id, dimension))
}

fn push_source_copy_operation(
    state: &mut SourceConstraintLoweringState<'_>,
    source: CodeOperand,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let dimension = source_operand_dimension(&source)?;
    let id = state.next_temporary;
    state.next_temporary = state
        .next_temporary
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    state.operations.push(CodeOperation {
        op: OperationKind::Copy,
        destination: CodeDestination::temporary(id, dimension),
        sources: vec![source],
    });
    Ok(CodeOperand::temporary(id, dimension))
}

fn source_binary_result_dimension(
    left: &CodeOperand,
    right: &CodeOperand,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let left = source_operand_dimension(left)?;
    let right = source_operand_dimension(right)?;
    match (left, right) {
        (1, 1) => Ok(1),
        (1, 3) | (3, 1) | (3, 3) => Ok(3),
        _ => unsupported("unsupported source scalar constraint expression dimension"),
    }
}

fn source_operand_dimension(operand: &CodeOperand) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let dimension = match operand {
        CodeOperand::Temporary { dimension, .. }
        | CodeOperand::Number { dimension, .. }
        | CodeOperand::Evaluation { dimension, .. }
        | CodeOperand::Challenge { dimension, .. }
        | CodeOperand::Public { dimension, .. }
        | CodeOperand::Constant { dimension, .. }
        | CodeOperand::ConstantAt { dimension, .. }
        | CodeOperand::Commitment { dimension, .. }
        | CodeOperand::CommitmentElement { dimension, .. }
        | CodeOperand::BoundaryZerofier { dimension, .. }
        | CodeOperand::ProofValue { dimension, .. }
        | CodeOperand::OpeningDenominator { dimension, .. }
        | CodeOperand::CustomCommitment { dimension, .. }
        | CodeOperand::AirGroupValue { dimension, .. }
        | CodeOperand::AirValue { dimension, .. } => *dimension,
    };
    match dimension {
        1 | 3 => Ok(dimension),
        _ => unsupported("unsupported source scalar constraint expression dimension"),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SourceConstraintFrameOffsets {
    min: i64,
    max: i64,
}

impl SourceConstraintFrameOffsets {
    fn include(&mut self, offset: i64) {
        self.min = self.min.min(offset);
        self.max = self.max.max(offset);
    }

    fn boundary(
        &self,
    ) -> Result<(BoundaryKind, Option<i64>, Option<i64>), SourceKeyDirectoryMetadataError> {
        if self.min == 0 && self.max == 0 {
            Ok((BoundaryKind::EveryRow, None, None))
        } else {
            let leading_rows = if self.min < 0 {
                self.min
                    .checked_neg()
                    .ok_or_else(|| unsupported_source_message("source row offset overflow"))?
            } else {
                0
            };
            let trailing_rows = self.max.max(0);
            Ok((
                BoundaryKind::EveryFrame,
                Some(leading_rows),
                Some(trailing_rows),
            ))
        }
    }
}

fn strip_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_group_expression(inner),
        _ => expression,
    }
}

fn source_constraint_index_chain(expression: &Expression) -> Option<(&str, Vec<&Expression>)> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some((name, Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (name, mut indices) = source_constraint_index_chain(target)?;
            indices.push(index);
            Some((name, indices))
        }
        _ => None,
    }
}

fn expression_is_zero(expression: &Expression) -> bool {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128_literal(value).is_ok_and(|value| value == 0)
        }
        ExpressionKind::Unary {
            op: UnaryOperator::Plus | UnaryOperator::Minus,
            expr,
        } => expression_is_zero(expr),
        _ => false,
    }
}

fn source_row_offset_value(
    expression: &Expression,
    prior: bool,
    state: &SourceConstraintLoweringState<'_>,
) -> Result<i64, SourceKeyDirectoryMetadataError> {
    let offset = eval_i128_expression_with_values(expression, state)?;
    let signed = if prior {
        offset
            .checked_neg()
            .ok_or_else(|| unsupported_source_message("source row offset overflow"))?
    } else {
        offset
    };
    i64::try_from(signed).map_err(|_| unsupported_source_message("source row offset overflow"))
}

fn source_scalar_index_value(
    expression: &Expression,
    state: &SourceConstraintLoweringState<'_>,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let index = eval_i128_expression_with_values(expression, state)?;
    if index < 0 {
        return unsupported("source scalar constraint index must be nonnegative");
    }
    u32::try_from(index).map_err(|_| unsupported_source_message("source scalar index overflow"))
}

fn eval_i128_expression_with_values(
    expression: &Expression,
    state: &SourceConstraintLoweringState<'_>,
) -> Result<i128, SourceKeyDirectoryMetadataError> {
    if let Some(value) =
        evaluate_source_static_expression(state.program, expression, state.constant_values)
    {
        if let Some(value) = static_value_integer(&value) {
            return Ok(value);
        }
    }
    eval_i128_expression(expression)
}

fn eval_i128_expression(expression: &Expression) -> Result<i128, SourceKeyDirectoryMetadataError> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128_literal(value)
        }
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

fn static_scalar_integer(
    expression: &Expression,
    state: &SourceConstraintLoweringState<'_>,
) -> Result<Option<i128>, SourceKeyDirectoryMetadataError> {
    if let Some(value) =
        evaluate_source_static_expression(state.program, expression, state.constant_values)
    {
        return Ok(static_value_integer(&value));
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128_literal(value).map(Some)
        }
        ExpressionKind::Unary { op, expr } => {
            let Some(value) = static_scalar_integer(expr, state)? else {
                return Ok(None);
            };
            match op {
                UnaryOperator::Plus => Ok(Some(value)),
                UnaryOperator::Minus => value
                    .checked_neg()
                    .map(Some)
                    .ok_or_else(|| unsupported_source_message("source scalar literal overflow")),
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

fn canonical_field_value(value: i128) -> Result<u64, SourceKeyDirectoryMetadataError> {
    let modulus = i128::from(MODULUS);
    let canonical = value.rem_euclid(modulus);
    u64::try_from(canonical)
        .map_err(|_| unsupported_source_message("source scalar literal overflow"))
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

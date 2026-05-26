use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::expression_info::{CodeOperand, HintFieldInfo, HintPayload, HintValueInfo};
use lzvm_field::MODULUS;
use lzvm_pil::{Expression, ExpressionKind, FixedFileTemplateValue, SourceProgram};

use crate::{
    source_expression_info::SourceExpressionAliasScope,
    source_static_values::{evaluate_source_static_expression, static_value_integer},
};

use super::{
    canonical_hint_number_from_value, SourceExpressionArrayAlias, SourceExpressionArrayAliases,
    SourceLookupLowering,
};

pub(super) fn source_lookup_scalar_operand(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
    row_offset: i64,
) -> Option<CodeOperand> {
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    source_lookup_scalar_operand_inner(
        context,
        expression,
        row_offset,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
    )
}

fn source_lookup_scalar_operand_inner(
    context: &SourceLookupLowering<'_>,
    expression: &Expression,
    row_offset: i64,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<CodeOperand> {
    if let Some(value) =
        evaluate_source_static_expression(context.program, expression, context.values)
    {
        return Some(CodeOperand::number(
            canonical_hint_number_from_value(value)?,
            1,
        ));
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => {
            if let Some(alias) = context.expression_aliases.get(name) {
                if !resolving_aliases.insert(name.clone()) {
                    return None;
                }
                let operand = source_lookup_scalar_operand_inner(
                    context,
                    alias,
                    row_offset,
                    resolving_aliases,
                    resolving_array_aliases,
                );
                resolving_aliases.remove(name);
                return operand;
            }
            if row_offset == 0 {
                context.scalar_slots.operand(name).ok()
            } else {
                context.scalar_slots.operand_at(name, row_offset).ok()
            }
        }
        ExpressionKind::Index { .. } => {
            let (name, index_expressions) =
                source_lookup_index_chain(strip_group_expression(expression))?;
            let indices = index_expressions
                .iter()
                .map(|index| source_lookup_index(context.program, index, context.values))
                .collect::<Option<Vec<_>>>()?;
            if let Some(alias) = context.expression_array_aliases.get(name) {
                let element = source_lookup_array_alias_path_element(
                    alias,
                    &indices,
                    context.expression_array_aliases,
                    resolving_array_aliases,
                )?;
                return match element {
                    SourceLookupArrayAliasElement::Expression(expression) => {
                        source_lookup_scalar_operand_inner(
                            context,
                            expression,
                            row_offset,
                            resolving_aliases,
                            resolving_array_aliases,
                        )
                    }
                    SourceLookupArrayAliasElement::ScopedExpression { expression, scope } => {
                        let scoped_context = source_lookup_scoped_context(context, scope);
                        source_lookup_scalar_operand_inner(
                            &scoped_context,
                            expression,
                            row_offset,
                            resolving_aliases,
                            resolving_array_aliases,
                        )
                    }
                    SourceLookupArrayAliasElement::NamedArray(name) => context
                        .scalar_slots
                        .operand_indices_at(name, &indices, row_offset)
                        .ok(),
                };
            }
            context
                .scalar_slots
                .operand_indices_at(name, &indices, row_offset)
                .ok()
        }
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } => {
            let signed_offset =
                source_lookup_row_offset_value(context.program, offset, *prior, context.values)?;
            let combined_offset = row_offset.checked_add(signed_offset)?;
            source_lookup_scalar_operand_inner(
                context,
                target,
                combined_offset,
                resolving_aliases,
                resolving_array_aliases,
            )
        }
        _ => None,
    }
}

enum SourceLookupArrayAliasElement<'a> {
    Expression(&'a Expression),
    ScopedExpression {
        expression: &'a Expression,
        scope: &'a SourceExpressionAliasScope,
    },
    NamedArray(&'a str),
}

fn source_lookup_array_alias_path_element<'a>(
    alias: &'a SourceExpressionArrayAlias,
    indices: &[u32],
    expression_array_aliases: &'a SourceExpressionArrayAliases,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceLookupArrayAliasElement<'a>> {
    match alias {
        SourceExpressionArrayAlias::Name(name) => {
            if let Some(next_alias) = expression_array_aliases.get(name) {
                if !resolving_array_aliases.insert(name.clone()) {
                    return None;
                }
                let element = source_lookup_array_alias_path_element(
                    next_alias,
                    indices,
                    expression_array_aliases,
                    resolving_array_aliases,
                );
                resolving_array_aliases.remove(name);
                return element;
            }
            Some(SourceLookupArrayAliasElement::NamedArray(name))
        }
        SourceExpressionArrayAlias::Values(expressions) => source_lookup_expression_array_element(
            expressions,
            indices,
            expression_array_aliases,
            resolving_array_aliases,
        ),
        SourceExpressionArrayAlias::ScopedValues { expressions, scope } => {
            source_lookup_expression_array_element(
                expressions,
                indices,
                scope.expression_arrays.as_ref(),
                resolving_array_aliases,
            )
            .map(|element| source_lookup_array_element_with_scope(element, scope.as_ref()))
        }
        SourceExpressionArrayAlias::Call { .. } => None,
    }
}

fn source_lookup_array_element_with_scope<'a>(
    element: SourceLookupArrayAliasElement<'a>,
    scope: &'a SourceExpressionAliasScope,
) -> SourceLookupArrayAliasElement<'a> {
    match element {
        SourceLookupArrayAliasElement::Expression(expression) => {
            SourceLookupArrayAliasElement::ScopedExpression { expression, scope }
        }
        element => element,
    }
}

pub(super) fn source_lookup_scoped_context<'a>(
    context: &SourceLookupLowering<'a>,
    scope: &'a SourceExpressionAliasScope,
) -> SourceLookupLowering<'a> {
    SourceLookupLowering {
        program: context.program,
        module: context.module,
        line: context.line,
        tokens: context.tokens,
        values: context.values,
        expression_aliases: scope.expressions.as_ref(),
        expression_array_aliases: scope.expression_arrays.as_ref(),
        scalar_slots: context.scalar_slots,
        opening_points: context.opening_points,
    }
}

fn source_lookup_expression_array_element<'a>(
    expressions: &'a [Expression],
    indices: &[u32],
    expression_array_aliases: &'a SourceExpressionArrayAliases,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceLookupArrayAliasElement<'a>> {
    let (index, rest) = indices.split_first()?;
    let expression = expressions.get(usize::try_from(*index).ok()?)?;
    if rest.is_empty() {
        return Some(SourceLookupArrayAliasElement::Expression(expression));
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Array(expressions) => source_lookup_expression_array_element(
            expressions,
            rest,
            expression_array_aliases,
            resolving_array_aliases,
        ),
        ExpressionKind::Name(name) => {
            let alias = expression_array_aliases.get(name)?;
            source_lookup_array_alias_path_element(
                alias,
                rest,
                expression_array_aliases,
                resolving_array_aliases,
            )
        }
        _ => None,
    }
}

pub(super) fn strip_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_group_expression(inner),
        _ => expression,
    }
}

fn source_lookup_index_chain(expression: &Expression) -> Option<(&str, Vec<&Expression>)> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some((name, Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (name, mut indices) = source_lookup_index_chain(target)?;
            indices.push(index);
            Some((name, indices))
        }
        _ => None,
    }
}

pub(super) fn source_lookup_index(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<u32> {
    let value = evaluate_source_static_expression(program, expression, values)?;
    u32::try_from(static_value_integer(&value)?).ok()
}

fn source_lookup_row_offset_value(
    program: &SourceProgram,
    expression: &Expression,
    prior: bool,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<i64> {
    let value = evaluate_source_static_expression(program, expression, values)?;
    let offset = static_value_integer(&value)?;
    let signed = if prior { offset.checked_neg()? } else { offset };
    i64::try_from(signed).ok()
}

pub(super) fn canonical_hint_number(value: i128) -> Option<u64> {
    let modulus = i128::from(MODULUS);
    u64::try_from(value.rem_euclid(modulus)).ok()
}

pub(super) fn hint_payload_from_code_operand(
    operand: CodeOperand,
    opening_points: &[i64],
) -> Option<HintPayload> {
    match operand {
        CodeOperand::Number { value, .. } => Some(HintPayload::number(value)),
        CodeOperand::Commitment {
            id,
            prime,
            dimension,
        } => {
            let row_offset = prime.unwrap_or(0);
            Some(HintPayload::Commitment {
                id,
                row_offset_index: Some(opening_point_index(opening_points, row_offset)?),
                row_offset: Some(row_offset),
                stage: None,
                stage_id: None,
                dimension: Some(dimension),
                air_group_id: None,
                air_id: None,
            })
        }
        CodeOperand::CommitmentElement {
            id,
            element,
            prime,
            dimension,
        } => {
            let row_offset = prime.unwrap_or(0);
            Some(HintPayload::commitment_element(
                id,
                element,
                Some(opening_point_index(opening_points, row_offset)?),
                Some(row_offset),
                Some(dimension),
            ))
        }
        CodeOperand::Constant { id, dimension } => Some(HintPayload::constant(
            id,
            Some(opening_point_index(opening_points, 0)?),
            Some(0),
            Some(dimension),
            None,
            None,
        )),
        CodeOperand::ConstantAt {
            id,
            prime,
            dimension,
        } => {
            let row_offset = prime.unwrap_or(0);
            Some(HintPayload::constant(
                id,
                Some(opening_point_index(opening_points, row_offset)?),
                Some(row_offset),
                Some(dimension),
                None,
                None,
            ))
        }
        CodeOperand::AirValue {
            id,
            stage,
            dimension,
            ..
        } => Some(HintPayload::air_value(id, stage, Some(dimension))),
        CodeOperand::AirGroupValue {
            id,
            stage,
            air_group_id,
            dimension,
        } => Some(HintPayload::air_group_value(
            id,
            air_group_id,
            stage,
            Some(dimension),
        )),
        CodeOperand::Public { id, .. } => Some(HintPayload::public(id, None)),
        CodeOperand::Challenge {
            id,
            stage,
            stage_id,
            ..
        } => Some(HintPayload::challenge(id, stage, stage_id)),
        CodeOperand::ProofValue {
            id,
            stage,
            dimension,
        } => Some(HintPayload::proof_value(id, stage, Some(dimension))),
        _ => None,
    }
}

pub(super) fn source_assignment_target_payload(
    operand: CodeOperand,
    opening_points: &[i64],
) -> Option<HintPayload> {
    match operand {
        CodeOperand::Commitment { .. } | CodeOperand::CommitmentElement { .. } => {
            hint_payload_from_code_operand(operand, opening_points)
        }
        _ => None,
    }
}

fn opening_point_index(opening_points: &[i64], row_offset: i64) -> Option<u32> {
    opening_points
        .iter()
        .position(|point| *point == row_offset)
        .and_then(|index| u32::try_from(index).ok())
}

pub(super) fn hint_number_field(name: &str, value: u64) -> HintFieldInfo {
    HintFieldInfo {
        name: name.to_owned(),
        values: vec![HintValueInfo {
            positions: Vec::new(),
            payload: HintPayload::number(value),
        }],
    }
}

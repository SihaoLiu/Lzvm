use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::expression_info::{
    BoundaryKind, CodeDestination, CodeOperand, CodeOperation, ConstraintCode, OperationKind,
};
use lzvm_field::{Felt, MODULUS};
use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, FunctionStatement,
    SourceProgram, SourceProgramModule, UnaryOperator,
};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError, source_scalar_slots::SourceScalarSlots,
    source_static_values::evaluate_source_static_expression,
};

pub(crate) type SourceExpressionAliases = BTreeMap<String, Expression>;

pub(crate) fn lower_source_template_boolean_constraint(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    scalar_slots: &SourceScalarSlots,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    expression_aliases: &SourceExpressionAliases,
) -> Result<Option<ConstraintCode>, SourceKeyDirectoryMetadataError> {
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(None);
    };
    let mut state = SourceConstraintLoweringState {
        program,
        scalar_slots,
        constant_values,
        expression_aliases,
        operations: Vec::new(),
        next_temporary: 0,
        frame_offsets: SourceConstraintFrameOffsets::default(),
        resolving_aliases: BTreeSet::new(),
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
    expression_aliases: &'a SourceExpressionAliases,
    operations: Vec<CodeOperation>,
    next_temporary: u32,
    frame_offsets: SourceConstraintFrameOffsets,
    resolving_aliases: BTreeSet<String>,
}

fn lower_source_constraint_residual(
    expression: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return Ok(None);
    };
    if !matches!(
        op,
        BinaryOperator::TripleEqual | BinaryOperator::ConstrainedAssign
    ) {
        return Ok(None);
    }
    if expression_is_zero(right) {
        return lower_source_scalar_expression_at(left, state, 0).map(Some);
    } else if expression_is_zero(left) {
        return lower_source_scalar_expression_at(right, state, 0).map(Some);
    }

    let left = lower_source_scalar_expression_at(left, state, 0)?;
    let right = lower_source_scalar_expression_at(right, state, 0)?;
    let id = state.next_temporary;
    state.next_temporary = state
        .next_temporary
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    state.operations.push(CodeOperation {
        op: OperationKind::Sub,
        destination: CodeDestination::temporary(id, 1),
        sources: vec![left, right],
    });
    Ok(Some(CodeOperand::temporary(id, 1)))
}

fn lower_source_scalar_expression_at(
    expression: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let expression = strip_group_expression(expression);
    if let Some(value) = static_scalar_integer(expression, state)? {
        return Ok(CodeOperand::number(canonical_field_value(value)?, 1));
    }
    match &expression.kind {
        ExpressionKind::Name(name) => {
            if let Some(alias) = state.expression_aliases.get(name) {
                if !state.resolving_aliases.insert(name.clone()) {
                    return unsupported("source scalar constraint expression alias cycle");
                }
                let operand = lower_source_scalar_expression_at(alias, state, row_offset);
                state.resolving_aliases.remove(name);
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
            let value = lower_source_scalar_expression_at(expr, state, row_offset)?;
            match op {
                UnaryOperator::Plus => Ok(value),
                UnaryOperator::Minus => {
                    let id = state.next_temporary;
                    state.next_temporary =
                        state.next_temporary.checked_add(1).ok_or_else(|| {
                            unsupported_source_message(
                                "source scalar constraint temporary overflow",
                            )
                        })?;
                    state.operations.push(CodeOperation {
                        op: OperationKind::Sub,
                        destination: CodeDestination::temporary(id, 1),
                        sources: vec![CodeOperand::number(0, 1), value],
                    });
                    Ok(CodeOperand::temporary(id, 1))
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
            lower_source_scalar_expression_at(target, state, combined_offset)
        }
        ExpressionKind::Index { target, index } => {
            let ExpressionKind::Name(name) = &strip_group_expression(target).kind else {
                return unsupported("unsupported source indexed constraint target");
            };
            let index = source_scalar_index_value(index, state)?;
            if row_offset != 0 {
                state.frame_offsets.include(row_offset);
            }
            state
                .scalar_slots
                .operand_index_at(name, index, row_offset)
                .map_err(|error| unsupported_source_message(error.to_string()))
        }
        ExpressionKind::Binary { op, left, right } => {
            if *op == BinaryOperator::Divide {
                return lower_source_static_divisor_expression(left, right, state, row_offset);
            }
            if *op == BinaryOperator::Power {
                return lower_source_static_exponent_expression(left, right, state, row_offset);
            }
            let op = match op {
                BinaryOperator::Add => OperationKind::Add,
                BinaryOperator::Subtract => OperationKind::Sub,
                BinaryOperator::Multiply => OperationKind::Mul,
                _ => return unsupported("unsupported source scalar constraint expression"),
            };
            let left = lower_source_scalar_expression_at(left, state, row_offset)?;
            let right = lower_source_scalar_expression_at(right, state, row_offset)?;
            let id = state.next_temporary;
            state.next_temporary = state.next_temporary.checked_add(1).ok_or_else(|| {
                unsupported_source_message("source scalar constraint temporary overflow")
            })?;
            state.operations.push(CodeOperation {
                op,
                destination: CodeDestination::temporary(id, 1),
                sources: vec![left, right],
            });
            Ok(CodeOperand::temporary(id, 1))
        }
        _ => unsupported("unsupported source scalar constraint expression"),
    }
}

fn lower_source_static_divisor_expression(
    left: &Expression,
    right: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let Some(divisor) = static_scalar_integer(right, state)? else {
        return unsupported("unsupported source scalar constraint expression");
    };
    let divisor = Felt::from_u64(canonical_field_value(divisor)?);
    let inverse = divisor
        .inverse()
        .ok_or_else(|| unsupported_source_message("source scalar constraint division by zero"))?;
    let left = lower_source_scalar_expression_at(left, state, row_offset)?;
    let id = state.next_temporary;
    state.next_temporary = state
        .next_temporary
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    state.operations.push(CodeOperation {
        op: OperationKind::Mul,
        destination: CodeDestination::temporary(id, 1),
        sources: vec![left, CodeOperand::number(inverse.to_u64(), 1)],
    });
    Ok(CodeOperand::temporary(id, 1))
}

fn lower_source_static_exponent_expression(
    left: &Expression,
    right: &Expression,
    state: &mut SourceConstraintLoweringState<'_>,
    row_offset: i64,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let Some(exponent) = static_scalar_integer(right, state)? else {
        return unsupported("unsupported source scalar constraint expression");
    };
    let mut exponent = u64::try_from(exponent)
        .map_err(|_| unsupported_source_message("source scalar constraint exponent overflow"))?;
    if exponent == 0 {
        return Ok(CodeOperand::number(1, 1));
    }
    let mut power = lower_source_scalar_expression_at(left, state, row_offset)?;
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
    let id = state.next_temporary;
    state.next_temporary = state
        .next_temporary
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    state.operations.push(CodeOperation {
        op: OperationKind::Mul,
        destination: CodeDestination::temporary(id, 1),
        sources: vec![left, right],
    });
    Ok(CodeOperand::temporary(id, 1))
}

fn push_source_copy_operation(
    state: &mut SourceConstraintLoweringState<'_>,
    source: CodeOperand,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let id = state.next_temporary;
    state.next_temporary = state
        .next_temporary
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    state.operations.push(CodeOperation {
        op: OperationKind::Copy,
        destination: CodeDestination::temporary(id, 1),
        sources: vec![source],
    });
    Ok(CodeOperand::temporary(id, 1))
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
    if let Some(FixedFileTemplateValue::Integer(value)) =
        evaluate_source_static_expression(state.program, expression, state.constant_values)
    {
        return Ok(value);
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
    if let Some(FixedFileTemplateValue::Integer(value)) =
        evaluate_source_static_expression(state.program, expression, state.constant_values)
    {
        return Ok(Some(value));
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

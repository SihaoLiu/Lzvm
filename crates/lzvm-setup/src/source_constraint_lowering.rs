use lzvm_artifacts::expression_info::{
    BoundaryKind, CodeDestination, CodeOperand, CodeOperation, ConstraintCode, OperationKind,
};
use lzvm_field::MODULUS;
use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FunctionStatement, SourceProgramModule,
    UnaryOperator,
};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError, source_scalar_slots::SourceScalarSlots,
};

pub(crate) fn lower_source_template_boolean_constraint(
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    scalar_slots: &SourceScalarSlots,
) -> Result<Option<ConstraintCode>, SourceKeyDirectoryMetadataError> {
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(None);
    };
    let mut operations = Vec::new();
    let mut next_temporary = 0_u32;
    let mut frame_offsets = SourceConstraintFrameOffsets::default();
    let Some(result) = lower_source_constraint_residual(
        expression,
        scalar_slots,
        &mut operations,
        &mut next_temporary,
        &mut frame_offsets,
    )?
    else {
        return Ok(None);
    };
    if operations.is_empty() {
        return Ok(None);
    }
    if !matches!(result, CodeOperand::Temporary { .. }) {
        return Ok(None);
    }
    let (boundary, offset_min, offset_max) = frame_offsets.boundary()?;
    Ok(Some(ConstraintCode {
        stage: 1,
        boundary,
        offset_min,
        offset_max,
        line: module.source.contents[statement.start..statement.end]
            .trim()
            .to_owned(),
        intermediate: false,
        temporary_count: next_temporary,
        operations,
    }))
}

fn lower_source_constraint_residual(
    expression: &Expression,
    scalar_slots: &SourceScalarSlots,
    operations: &mut Vec<CodeOperation>,
    next_temporary: &mut u32,
    frame_offsets: &mut SourceConstraintFrameOffsets,
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return Ok(None);
    };
    if *op != BinaryOperator::TripleEqual {
        return Ok(None);
    }
    if expression_is_zero(right) {
        return lower_source_scalar_expression(
            left,
            scalar_slots,
            operations,
            next_temporary,
            frame_offsets,
        )
        .map(Some);
    } else if expression_is_zero(left) {
        return lower_source_scalar_expression(
            right,
            scalar_slots,
            operations,
            next_temporary,
            frame_offsets,
        )
        .map(Some);
    }

    let left = lower_source_scalar_expression(
        left,
        scalar_slots,
        operations,
        next_temporary,
        frame_offsets,
    )?;
    let right = lower_source_scalar_expression(
        right,
        scalar_slots,
        operations,
        next_temporary,
        frame_offsets,
    )?;
    let id = *next_temporary;
    *next_temporary = next_temporary
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source scalar constraint temporary overflow"))?;
    operations.push(CodeOperation {
        op: OperationKind::Sub,
        destination: CodeDestination::temporary(id, 1),
        sources: vec![left, right],
    });
    Ok(Some(CodeOperand::temporary(id, 1)))
}

fn lower_source_scalar_expression(
    expression: &Expression,
    scalar_slots: &SourceScalarSlots,
    operations: &mut Vec<CodeOperation>,
    next_temporary: &mut u32,
    frame_offsets: &mut SourceConstraintFrameOffsets,
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    let expression = strip_group_expression(expression);
    if let Some(value) = static_scalar_integer(expression)? {
        return Ok(CodeOperand::number(canonical_field_value(value)?, 1));
    }
    match &expression.kind {
        ExpressionKind::Name(name) => scalar_slots
            .operand(name)
            .map_err(|error| unsupported_source_message(error.to_string())),
        ExpressionKind::Unary { op, expr } => {
            let value = lower_source_scalar_expression(
                expr,
                scalar_slots,
                operations,
                next_temporary,
                frame_offsets,
            )?;
            match op {
                UnaryOperator::Plus => Ok(value),
                UnaryOperator::Minus => {
                    let id = *next_temporary;
                    *next_temporary = next_temporary.checked_add(1).ok_or_else(|| {
                        unsupported_source_message("source scalar constraint temporary overflow")
                    })?;
                    operations.push(CodeOperation {
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
            let signed_offset = source_row_offset_value(offset, *prior)?;
            let ExpressionKind::Name(name) = &strip_group_expression(target).kind else {
                return unsupported("source row offsets require named scalar values");
            };
            frame_offsets.include(signed_offset);
            scalar_slots
                .operand_at(name, signed_offset)
                .map_err(|error| unsupported_source_message(error.to_string()))
        }
        ExpressionKind::Binary { op, left, right } => {
            let op = match op {
                BinaryOperator::Add => OperationKind::Add,
                BinaryOperator::Subtract => OperationKind::Sub,
                BinaryOperator::Multiply => OperationKind::Mul,
                _ => return unsupported("unsupported source scalar constraint expression"),
            };
            let left = lower_source_scalar_expression(
                left,
                scalar_slots,
                operations,
                next_temporary,
                frame_offsets,
            )?;
            let right = lower_source_scalar_expression(
                right,
                scalar_slots,
                operations,
                next_temporary,
                frame_offsets,
            )?;
            let id = *next_temporary;
            *next_temporary = next_temporary.checked_add(1).ok_or_else(|| {
                unsupported_source_message("source scalar constraint temporary overflow")
            })?;
            operations.push(CodeOperation {
                op,
                destination: CodeDestination::temporary(id, 1),
                sources: vec![left, right],
            });
            Ok(CodeOperand::temporary(id, 1))
        }
        _ => unsupported("unsupported source scalar constraint expression"),
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
) -> Result<i64, SourceKeyDirectoryMetadataError> {
    let offset = eval_i128_expression(expression)?;
    let signed = if prior {
        offset
            .checked_neg()
            .ok_or_else(|| unsupported_source_message("source row offset overflow"))?
    } else {
        offset
    };
    i64::try_from(signed).map_err(|_| unsupported_source_message("source row offset overflow"))
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
) -> Result<Option<i128>, SourceKeyDirectoryMetadataError> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128_literal(value).map(Some)
        }
        ExpressionKind::Unary { op, expr } => {
            let Some(value) = static_scalar_integer(expr)? else {
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

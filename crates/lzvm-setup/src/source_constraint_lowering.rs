use lzvm_artifacts::expression_info::{
    BoundaryKind, CodeDestination, CodeOperand, CodeOperation, ConstraintCode, OperationKind,
};
use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FunctionStatement, SourceProgramModule,
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
    let Some(result) = lower_source_constraint_residual(
        expression,
        scalar_slots,
        &mut operations,
        &mut next_temporary,
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
    Ok(Some(ConstraintCode {
        stage: 1,
        boundary: BoundaryKind::EveryRow,
        offset_min: None,
        offset_max: None,
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
) -> Result<Option<CodeOperand>, SourceKeyDirectoryMetadataError> {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return Ok(None);
    };
    if *op != BinaryOperator::TripleEqual {
        return Ok(None);
    }
    if expression_is_zero(right) {
        return lower_source_scalar_expression(left, scalar_slots, operations, next_temporary)
            .map(Some);
    } else if expression_is_zero(left) {
        return lower_source_scalar_expression(right, scalar_slots, operations, next_temporary)
            .map(Some);
    }

    let left = lower_source_scalar_expression(left, scalar_slots, operations, next_temporary)?;
    let right = lower_source_scalar_expression(right, scalar_slots, operations, next_temporary)?;
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
) -> Result<CodeOperand, SourceKeyDirectoryMetadataError> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => scalar_slots
            .operand(name)
            .map_err(|error| unsupported_source_message(error.to_string())),
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            let value = parse_i128_literal(value)
                .ok()
                .and_then(|value| u64::try_from(value).ok())
                .ok_or_else(|| unsupported_source_message("source scalar literal overflow"))?;
            Ok(CodeOperand::number(value, 1))
        }
        ExpressionKind::Binary { op, left, right } => {
            let op = match op {
                BinaryOperator::Add => OperationKind::Add,
                BinaryOperator::Subtract => OperationKind::Sub,
                BinaryOperator::Multiply => OperationKind::Mul,
                _ => return unsupported("unsupported source scalar constraint expression"),
            };
            let left =
                lower_source_scalar_expression(left, scalar_slots, operations, next_temporary)?;
            let right =
                lower_source_scalar_expression(right, scalar_slots, operations, next_temporary)?;
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
        _ => false,
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

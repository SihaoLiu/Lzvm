use std::collections::BTreeSet;

use lzvm_field::Felt;
use lzvm_pil::{BinaryOperator, Expression, ExpressionKind, UnaryOperator};

use super::*;

#[derive(Debug, Clone, Copy)]
struct SourceGlobalExtOperand {
    buffer: u16,
    offset: u32,
}

#[derive(Debug, Clone, Copy)]
enum SourceGlobalMixedOperand {
    Base(SourceGlobalBaseOperand),
    Ext(SourceGlobalExtOperand),
}

pub(super) fn append_ext_residual_constraint(
    builder: &mut SourceGlobalConstraintBuilder,
    expression: &Expression,
    slots: &SourceGlobalSlots<'_>,
    alias_scope: &SourceGlobalAliasScope<'_>,
    source_line: String,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let ops_offset = source_usize_to_u32(builder.ops.len(), "source global op offset overflow")?;
    let args_offset =
        source_usize_to_u32(builder.args.len(), "source global argument offset overflow")?;
    let ops_start = builder.ops.len();
    let args_start = builder.args.len();
    let numbers_start = builder.numbers.len();
    let (destination_id, temp1_count, temp3_count) = {
        let mut context = SourceGlobalExtLoweringContext {
            builder,
            slots,
            alias_scope,
            resolving_aliases: BTreeSet::new(),
            resolving_array_aliases: BTreeSet::new(),
            next_base_temp: 0,
            next_ext_temp: 0,
        };
        let Some(SourceGlobalMixedOperand::Ext(operand)) =
            lower_global_mixed_residual_operand(expression, &mut context)?
        else {
            builder.ops.truncate(ops_start);
            builder.args.truncate(args_start);
            builder.numbers.truncate(numbers_start);
            return Ok(false);
        };
        let destination = context.ensure_ext_temp_operand(operand)?;
        let destination_id = destination
            .offset
            .checked_div(3)
            .ok_or_else(|| unsupported_source_message("source global temporary overflow"))?;
        (
            destination_id,
            context.next_base_temp,
            context.next_ext_temp,
        )
    };
    let ops_count = source_usize_to_u32(
        builder.ops.len().saturating_sub(ops_start),
        "source global op count overflow",
    )?;
    let args_count = source_usize_to_u32(
        builder.args.len().saturating_sub(args_start),
        "source global argument count overflow",
    )?;
    builder.entries.push(GlobalConstraintEntry {
        destination_dimension: 3,
        destination_id,
        temp1_count,
        temp3_count,
        ops_count,
        ops_offset,
        args_count,
        args_offset,
        source_line,
    });
    Ok(true)
}

struct SourceGlobalExtLoweringContext<'a, 'b> {
    builder: &'a mut SourceGlobalConstraintBuilder,
    slots: &'b SourceGlobalSlots<'b>,
    alias_scope: &'b SourceGlobalAliasScope<'b>,
    resolving_aliases: BTreeSet<String>,
    resolving_array_aliases: BTreeSet<String>,
    next_base_temp: u32,
    next_ext_temp: u32,
}

impl SourceGlobalExtLoweringContext<'_, '_> {
    fn number_operand(
        &mut self,
        value: u64,
    ) -> Result<SourceGlobalBaseOperand, SourceKeyDirectoryMetadataError> {
        Ok(SourceGlobalBaseOperand {
            buffer: 2,
            offset: self.builder.intern_number(value)?,
        })
    }

    fn zero_operand(&mut self) -> Result<SourceGlobalBaseOperand, SourceKeyDirectoryMetadataError> {
        self.number_operand(0)
    }

    fn append_base_binary_op(
        &mut self,
        kind: u16,
        left: SourceGlobalBaseOperand,
        right: SourceGlobalBaseOperand,
    ) -> Result<SourceGlobalBaseOperand, SourceKeyDirectoryMetadataError> {
        let destination = self.next_base_temp;
        self.next_base_temp = self
            .next_base_temp
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source global temporary overflow"))?;
        let destination =
            source_u32_to_u16(destination, "source global temporary offset overflow")?;
        let left_offset = source_u32_to_u16(left.offset, "source global operand offset overflow")?;
        let right_offset =
            source_u32_to_u16(right.offset, "source global operand offset overflow")?;

        self.builder.ops.push(0);
        self.builder.args.extend([
            kind,
            destination,
            left.buffer,
            left_offset,
            right.buffer,
            right_offset,
        ]);
        Ok(SourceGlobalBaseOperand {
            buffer: 0,
            offset: u32::from(destination),
        })
    }

    fn append_ext_base_binary_op(
        &mut self,
        kind: u16,
        left: SourceGlobalExtOperand,
        right: SourceGlobalBaseOperand,
    ) -> Result<SourceGlobalExtOperand, SourceKeyDirectoryMetadataError> {
        let destination = self.next_ext_temp;
        self.next_ext_temp = self
            .next_ext_temp
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source global temporary overflow"))?;
        let destination_offset = destination
            .checked_mul(3)
            .ok_or_else(|| unsupported_source_message("source global temporary overflow"))?;
        let destination_offset = source_u32_to_u16(
            destination_offset,
            "source global temporary offset overflow",
        )?;
        let left_offset = source_u32_to_u16(left.offset, "source global operand offset overflow")?;
        let right_offset =
            source_u32_to_u16(right.offset, "source global operand offset overflow")?;

        self.builder.ops.push(1);
        self.builder.args.extend([
            kind,
            destination_offset,
            left.buffer,
            left_offset,
            right.buffer,
            right_offset,
        ]);
        Ok(SourceGlobalExtOperand {
            buffer: 4,
            offset: u32::from(destination_offset),
        })
    }

    fn append_ext_ext_binary_op(
        &mut self,
        kind: u16,
        left: SourceGlobalExtOperand,
        right: SourceGlobalExtOperand,
    ) -> Result<SourceGlobalExtOperand, SourceKeyDirectoryMetadataError> {
        let destination = self.next_ext_temp;
        self.next_ext_temp = self
            .next_ext_temp
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source global temporary overflow"))?;
        let destination_offset = destination
            .checked_mul(3)
            .ok_or_else(|| unsupported_source_message("source global temporary overflow"))?;
        let destination_offset = source_u32_to_u16(
            destination_offset,
            "source global temporary offset overflow",
        )?;
        let left_offset = source_u32_to_u16(left.offset, "source global operand offset overflow")?;
        let right_offset =
            source_u32_to_u16(right.offset, "source global operand offset overflow")?;

        self.builder.ops.push(2);
        self.builder.args.extend([
            kind,
            destination_offset,
            left.buffer,
            left_offset,
            right.buffer,
            right_offset,
        ]);
        Ok(SourceGlobalExtOperand {
            buffer: 4,
            offset: u32::from(destination_offset),
        })
    }

    fn ensure_ext_temp_operand(
        &mut self,
        operand: SourceGlobalExtOperand,
    ) -> Result<SourceGlobalExtOperand, SourceKeyDirectoryMetadataError> {
        if operand.buffer == 4 && operand.offset.is_multiple_of(3) {
            return Ok(operand);
        }
        let zero = self.zero_operand()?;
        self.append_ext_base_binary_op(0, operand, zero)
    }
}

fn lower_global_mixed_residual_operand(
    expression: &Expression,
    context: &mut SourceGlobalExtLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalMixedOperand>, SourceKeyDirectoryMetadataError> {
    if let Some(value) = evaluate_source_static_expression(
        context.alias_scope.program,
        expression,
        &context.alias_scope.static_values,
    ) {
        return Ok(Some(SourceGlobalMixedOperand::Base(
            context.number_operand(source_public_initializer_field_value(&value)?)?,
        )));
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Group(inner) => lower_global_mixed_residual_operand(inner, context),
        ExpressionKind::Name(name) => lower_global_mixed_name_operand(name, &[], context),
        ExpressionKind::Index { target, index } => {
            lower_global_mixed_index_operand(target, index, context)
        }
        ExpressionKind::Unary { op, expr } => match op {
            UnaryOperator::Plus => lower_global_mixed_residual_operand(expr, context),
            UnaryOperator::Minus => {
                let Some(value) = lower_global_mixed_residual_operand(expr, context)? else {
                    return Ok(None);
                };
                let zero = context.zero_operand()?;
                match value {
                    SourceGlobalMixedOperand::Base(value) => context
                        .append_base_binary_op(1, zero, value)
                        .map(SourceGlobalMixedOperand::Base)
                        .map(Some),
                    SourceGlobalMixedOperand::Ext(value) => context
                        .append_ext_base_binary_op(3, value, zero)
                        .map(SourceGlobalMixedOperand::Ext)
                        .map(Some),
                }
            }
            _ => Ok(None),
        },
        ExpressionKind::Binary { op, left, right } => {
            lower_global_mixed_binary_operand(op, left, right, context)
        }
        _ => Ok(None),
    }
}

fn lower_global_mixed_binary_operand(
    op: &BinaryOperator,
    left: &Expression,
    right: &Expression,
    context: &mut SourceGlobalExtLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalMixedOperand>, SourceKeyDirectoryMetadataError> {
    if matches!(op, BinaryOperator::Divide | BinaryOperator::Backslash) {
        return lower_global_mixed_static_divisor_operand(left, right, context);
    }
    if *op == BinaryOperator::Power {
        return lower_global_mixed_static_exponent_operand(left, right, context);
    }
    let kind = match op {
        BinaryOperator::Add => 0,
        BinaryOperator::Subtract => 1,
        BinaryOperator::Multiply => 2,
        _ => return Ok(None),
    };
    let Some(left) = lower_global_mixed_residual_operand(left, context)? else {
        return Ok(None);
    };
    let Some(right) = lower_global_mixed_residual_operand(right, context)? else {
        return Ok(None);
    };
    lower_global_mixed_binary_operands(kind, left, right, context).map(Some)
}

fn lower_global_mixed_static_divisor_operand(
    left: &Expression,
    right: &Expression,
    context: &mut SourceGlobalExtLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalMixedOperand>, SourceKeyDirectoryMetadataError> {
    let Some(value) = evaluate_source_static_expression(
        context.alias_scope.program,
        right,
        &context.alias_scope.static_values,
    ) else {
        return Ok(None);
    };
    let divisor = Felt::from_u64(source_public_initializer_field_value(&value)?);
    let inverse = divisor
        .inverse()
        .ok_or_else(|| unsupported_source_message("source global constraint division by zero"))?;
    let Some(left) = lower_global_mixed_residual_operand(left, context)? else {
        return Ok(None);
    };
    let inverse = SourceGlobalMixedOperand::Base(context.number_operand(inverse.to_u64())?);
    lower_global_mixed_binary_operands(2, left, inverse, context).map(Some)
}

fn lower_global_mixed_static_exponent_operand(
    left: &Expression,
    right: &Expression,
    context: &mut SourceGlobalExtLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalMixedOperand>, SourceKeyDirectoryMetadataError> {
    let Some(mut exponent) = static_u32_expression(right, context.alias_scope) else {
        return Ok(None);
    };
    if exponent == 0 {
        return context
            .number_operand(1)
            .map(SourceGlobalMixedOperand::Base)
            .map(Some);
    }
    let Some(mut power) = lower_global_mixed_residual_operand(left, context)? else {
        return Ok(None);
    };
    let mut result = None;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = Some(match result {
                Some(value) => lower_global_mixed_binary_operands(2, value, power, context)?,
                None => power,
            });
        }
        exponent >>= 1;
        if exponent > 0 {
            power = lower_global_mixed_binary_operands(2, power, power, context)?;
        }
    }
    Ok(result)
}

fn lower_global_mixed_binary_operands(
    kind: u16,
    left: SourceGlobalMixedOperand,
    right: SourceGlobalMixedOperand,
    context: &mut SourceGlobalExtLoweringContext<'_, '_>,
) -> Result<SourceGlobalMixedOperand, SourceKeyDirectoryMetadataError> {
    match (left, right) {
        (SourceGlobalMixedOperand::Base(left), SourceGlobalMixedOperand::Base(right)) => context
            .append_base_binary_op(kind, left, right)
            .map(SourceGlobalMixedOperand::Base),
        (SourceGlobalMixedOperand::Ext(left), SourceGlobalMixedOperand::Base(right)) => context
            .append_ext_base_binary_op(kind, left, right)
            .map(SourceGlobalMixedOperand::Ext),
        (SourceGlobalMixedOperand::Base(left), SourceGlobalMixedOperand::Ext(right)) => {
            let kind = if kind == 1 { 3 } else { kind };
            context
                .append_ext_base_binary_op(kind, right, left)
                .map(SourceGlobalMixedOperand::Ext)
        }
        (SourceGlobalMixedOperand::Ext(left), SourceGlobalMixedOperand::Ext(right)) => context
            .append_ext_ext_binary_op(kind, left, right)
            .map(SourceGlobalMixedOperand::Ext),
    }
}

fn lower_global_mixed_index_operand(
    target: &Expression,
    index: &Expression,
    context: &mut SourceGlobalExtLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalMixedOperand>, SourceKeyDirectoryMetadataError> {
    let Some((name, indices)) =
        indices::source_global_index_chain(target, index, context.alias_scope)
    else {
        return Ok(None);
    };
    if let Some(alias) = context.alias_scope.expression_arrays.get(&name) {
        let element = source_global_expression_array_alias_path_element(
            alias,
            &indices,
            &context.alias_scope.expression_arrays,
            &mut context.resolving_array_aliases,
        );
        return match element {
            Some(SourceGlobalExpressionArrayAliasElement::Expression(expression)) => {
                lower_global_mixed_residual_operand(expression, context)
            }
            Some(SourceGlobalExpressionArrayAliasElement::NamedArray(name)) => {
                lower_global_mixed_name_operand(name, &indices, context)
            }
            None => Ok(None),
        };
    }
    lower_global_mixed_name_operand(&name, &indices, context)
}

fn lower_global_mixed_name_operand(
    name: &str,
    indices: &[u32],
    context: &mut SourceGlobalExtLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalMixedOperand>, SourceKeyDirectoryMetadataError> {
    if indices.is_empty() {
        if let Some(alias) = context.alias_scope.expressions.get(name) {
            if !context.resolving_aliases.insert(name.to_owned()) {
                return Ok(None);
            }
            let operand = lower_global_mixed_residual_operand(alias, context);
            context.resolving_aliases.remove(name);
            return operand;
        }
    }
    if let Some(slot) = context.slots.challenges.get(name) {
        let offset = challenge_target_offset(slot, indices)?;
        return Ok(Some(SourceGlobalMixedOperand::Ext(
            SourceGlobalExtOperand { buffer: 6, offset },
        )));
    }
    if let Some(slot) = context.slots.group_values.get(name) {
        let offset = group_value_target_offset(slot, indices)?;
        return match proof_value_operand_dimension(u64::from(slot.stage)) {
            1 => Ok(Some(SourceGlobalMixedOperand::Base(
                SourceGlobalBaseOperand { buffer: 5, offset },
            ))),
            3 => Ok(Some(SourceGlobalMixedOperand::Ext(
                SourceGlobalExtOperand { buffer: 5, offset },
            ))),
            _ => unsupported("unsupported source group value dimension"),
        };
    }
    if let Some(slot) = context.slots.public_values.get(name) {
        if slot.stage != 1 {
            return unsupported("top-level base residuals require base-field public values");
        }
        let offset = public_value_target_offset(slot, indices)?;
        return Ok(Some(SourceGlobalMixedOperand::Base(
            SourceGlobalBaseOperand { buffer: 1, offset },
        )));
    }
    if let Some(slot) = context.slots.proof_values.get(name) {
        let offset = proof_value_target_offset(slot, indices)?;
        let dimension = proof_value_operand_dimension(slot.stage);
        return match dimension {
            1 => Ok(Some(SourceGlobalMixedOperand::Base(
                SourceGlobalBaseOperand { buffer: 3, offset },
            ))),
            3 => Ok(Some(SourceGlobalMixedOperand::Ext(
                SourceGlobalExtOperand { buffer: 3, offset },
            ))),
            _ => unsupported("unsupported source proof value dimension"),
        };
    }
    Ok(None)
}

use lzvm_artifacts::constraint_program::ConstraintProgram;
use lzvm_field::Felt;

use super::regular_constraints_support::{
    find_stage_columns, scalar_ext, BufferKind, BufferLayout,
};
use super::{
    entry_args, entry_ops, read_operation_args, validate_inputs, RegularConstraintEvalError,
    RegularConstraintInputs, RegularConstraintResult, RegularConstraintViolation, SourceRef,
};

pub(crate) fn try_evaluate_regular_constraints_cuda_base(
    program: &ConstraintProgram,
    inputs: RegularConstraintInputs<'_>,
    fixed_values_device_buffer: Option<&lzvm_accel::CudaDeviceBuffer>,
) -> Result<Option<Vec<RegularConstraintResult>>, RegularConstraintEvalError> {
    validate_inputs(inputs)?;
    if inputs.domain_size == 0 || !inputs.custom_fixed_columns.is_empty() {
        return Ok(None);
    }
    if program
        .numbers
        .iter()
        .any(|value| *value >= lzvm_field::MODULUS)
    {
        return Ok(None);
    }

    let layout = BufferLayout::new(inputs);
    let mut cuda_entries = Vec::with_capacity(program.entries.len());
    for (constraint_index, entry) in program.entries.iter().enumerate() {
        if entry.destination_dimension != 1 {
            return Ok(None);
        }
        let ops = entry_ops(constraint_index, entry, program)?;
        let args = entry_args(constraint_index, entry, program)?;
        if args.len() != ops.len().saturating_mul(8) {
            return Ok(None);
        }
        for (operation_index, op) in ops.iter().enumerate() {
            if *op != 0 {
                return Ok(None);
            }
            let cursor = operation_index
                .checked_mul(8)
                .ok_or(RegularConstraintEvalError::LengthOverflow)?;
            let operation = read_operation_args(constraint_index, args, cursor)?;
            if operation.kind > 3
                || !cuda_base_source_supported(operation.src0, inputs, layout)
                || !cuda_base_source_supported(operation.src1, inputs, layout)
            {
                return Ok(None);
            }
        }
        cuda_entries.push(lzvm_accel::CudaRegularConstraintEntry {
            destination_id: entry.destination_id,
            first_row: entry.first_row,
            last_row: entry.last_row,
            temp1_count: entry.temp1_count,
            ops_count: entry.ops_count,
            ops_offset: entry.ops_offset,
            args_count: entry.args_count,
            args_offset: entry.args_offset,
        });
    }

    let fixed_values = Felt::as_u64_slice(inputs.fixed_columns.values);
    let stages = inputs
        .stage_columns
        .iter()
        .map(|stage| lzvm_accel::CudaRegularStage {
            stage_index: u32::from(stage.stage_index),
            column_count: stage.column_count,
            values: Felt::as_u64_slice(stage.values),
        })
        .collect::<Vec<_>>();
    let unit_values = Felt::as_u64_slice(inputs.unit_values);

    let cuda_results = match lzvm_accel::cuda_regular_constraints_base(
        &cuda_entries,
        &program.ops,
        &program.args,
        lzvm_accel::CudaRegularConstraintInputs {
            domain_size: inputs.domain_size,
            stage_count: usize::from(inputs.stage_count),
            fixed_column_count: inputs.fixed_columns.column_count,
            fixed_values,
            fixed_values_device: fixed_values_device_buffer,
            stages: &stages,
            opening_point_offsets: inputs.opening_point_offsets,
            numbers: &program.numbers,
            unit_values,
        },
    ) {
        Ok(results) => results,
        Err(_) => return Ok(None),
    };

    let results = program
        .entries
        .iter()
        .enumerate()
        .zip(cuda_results)
        .map(|((constraint_index, entry), result)| {
            let invalid_rows = result
                .invalid_row
                .map(|row| RegularConstraintViolation {
                    row,
                    value: scalar_ext(Felt::from_u64(result.value)),
                })
                .into_iter()
                .collect();
            RegularConstraintResult {
                constraint_index,
                stage: entry.stage,
                intermediate: entry.intermediate,
                invalid_rows,
            }
        })
        .collect();
    Ok(Some(results))
}

fn cuda_base_source_supported(
    source: SourceRef,
    inputs: RegularConstraintInputs<'_>,
    layout: BufferLayout,
) -> bool {
    let Ok(kind) = layout.resolve(source.buffer) else {
        return false;
    };
    match kind {
        BufferKind::Fixed => inputs
            .opening_point_offsets
            .get(source.row_offset_index)
            .is_some_and(|_| source.offset < inputs.fixed_columns.column_count),
        BufferKind::Stage(stage_index) => inputs
            .opening_point_offsets
            .get(source.row_offset_index)
            .is_some_and(|_| {
                find_stage_columns(inputs, stage_index)
                    .is_ok_and(|stage| source.offset < stage.column_count)
            }),
        BufferKind::Tmp1 | BufferKind::Number | BufferKind::UnitValue => true,
        BufferKind::CustomFixed(_)
        | BufferKind::DomainOrZerofier
        | BufferKind::Tmp3
        | BufferKind::Public
        | BufferKind::ProofValue
        | BufferKind::GroupValue
        | BufferKind::Challenge
        | BufferKind::Evaluation => false,
    }
}

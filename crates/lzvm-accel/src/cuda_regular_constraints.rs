use std::ptr;

use super::{cuda_status, u64_word_byte_len, AccelError, CudaDeviceBuffer};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CudaRegularConstraintEntry {
    pub destination_id: u32,
    pub first_row: u32,
    pub last_row: u32,
    pub temp1_count: u32,
    pub ops_count: u32,
    pub ops_offset: u32,
    pub args_count: u32,
    pub args_offset: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct CudaRegularStage<'a> {
    pub stage_index: u32,
    pub column_count: usize,
    pub values: &'a [u64],
}

#[derive(Debug, Clone, Copy)]
pub struct CudaRegularConstraintInputs<'a> {
    pub domain_size: usize,
    pub stage_count: usize,
    pub fixed_column_count: usize,
    pub fixed_values: &'a [u64],
    pub fixed_values_device: Option<&'a CudaDeviceBuffer>,
    pub stages: &'a [CudaRegularStage<'a>],
    pub opening_point_offsets: &'a [i64],
    pub numbers: &'a [u64],
    pub unit_values: &'a [u64],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CudaRegularConstraintResult {
    pub invalid_row: Option<usize>,
    pub value: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CudaRegularStageRaw {
    stage_index: u32,
    column_count: usize,
    values: *const u64,
    value_count: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CudaRegularConstraintOutputRaw {
    row: u64,
    value: u64,
    found: u32,
}

unsafe extern "C" {
    fn lzvm_cuda_regular_constraints_base(
        entries: *const CudaRegularConstraintEntry,
        entry_count: usize,
        ops: *const u8,
        ops_count: usize,
        args: *const u16,
        args_count: usize,
        numbers: *const u64,
        number_count: usize,
        fixed_values: *const u64,
        fixed_value_count: usize,
        fixed_values_device: *const u64,
        fixed_column_count: usize,
        stages: *const CudaRegularStageRaw,
        stage_input_count: usize,
        stage_count: usize,
        opening_point_offsets: *const i64,
        opening_point_offset_count: usize,
        unit_values: *const u64,
        unit_value_count: usize,
        domain_size: usize,
        out: *mut CudaRegularConstraintOutputRaw,
    ) -> i32;
}

pub fn cuda_regular_constraints_base(
    entries: &[CudaRegularConstraintEntry],
    ops: &[u8],
    args: &[u16],
    inputs: CudaRegularConstraintInputs<'_>,
) -> Result<Vec<CudaRegularConstraintResult>, AccelError> {
    let stages = inputs
        .stages
        .iter()
        .map(|stage| CudaRegularStageRaw {
            stage_index: stage.stage_index,
            column_count: stage.column_count,
            values: stage.values.as_ptr(),
            value_count: stage.values.len(),
        })
        .collect::<Vec<_>>();
    let fixed_values_device = if let Some(buffer) = inputs.fixed_values_device {
        let expected_len = u64_word_byte_len(inputs.fixed_values.len())?;
        if buffer.len() != expected_len {
            return Err(AccelError::LengthMismatch {
                lhs: buffer.len(),
                rhs: expected_len,
            });
        }
        buffer.as_raw_ptr().cast::<u64>() as *const u64
    } else {
        ptr::null()
    };
    let mut raw_results = vec![
        CudaRegularConstraintOutputRaw {
            row: u64::MAX,
            value: 0,
            found: 0,
        };
        entries.len()
    ];

    let code = unsafe {
        lzvm_cuda_regular_constraints_base(
            entries.as_ptr(),
            entries.len(),
            ops.as_ptr(),
            ops.len(),
            args.as_ptr(),
            args.len(),
            inputs.numbers.as_ptr(),
            inputs.numbers.len(),
            inputs.fixed_values.as_ptr(),
            inputs.fixed_values.len(),
            fixed_values_device,
            inputs.fixed_column_count,
            stages.as_ptr(),
            stages.len(),
            inputs.stage_count,
            inputs.opening_point_offsets.as_ptr(),
            inputs.opening_point_offsets.len(),
            inputs.unit_values.as_ptr(),
            inputs.unit_values.len(),
            inputs.domain_size,
            raw_results.as_mut_ptr(),
        )
    };
    cuda_status(code)?;

    raw_results
        .into_iter()
        .map(|result| {
            let invalid_row = if result.found == 0 {
                None
            } else {
                Some(
                    usize::try_from(result.row).map_err(|_| AccelError::InvalidDomain {
                        bits: usize::BITS as usize,
                        len: entries.len(),
                    })?,
                )
            };
            Ok(CudaRegularConstraintResult {
                invalid_row,
                value: result.value,
            })
        })
        .collect()
}

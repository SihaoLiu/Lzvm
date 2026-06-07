use super::{
    coset_extend_domain, coset_extend_row_weights, cuda_status, ensure_cuda_setup, AccelError,
    CudaDeviceBuffer, CudaRowMajorColumnView,
};

unsafe extern "C" {
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device_raw(
        values: *const u64,
        weights: *const u64,
        out: *mut u64,
        source_len: usize,
        column_count: usize,
        target_row_count: usize,
    ) -> i32;

    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device_raw(
        values: *const u64,
        weights: *const u64,
        out: *mut u64,
        source_len: usize,
        source_row_stride: usize,
        column_offset: usize,
        column_count: usize,
        target_row_count: usize,
    ) -> i32;
}

fn selected_row_weights(
    source_len: usize,
    target_len: usize,
    source_root: u64,
    target_root: u64,
    target_bits: usize,
    target_rows: &[usize],
) -> Result<Vec<u64>, AccelError> {
    let weight_count =
        source_len
            .checked_mul(target_rows.len())
            .ok_or(AccelError::InvalidDomain {
                bits: target_bits,
                len: target_rows.len(),
            })?;
    let mut weights = Vec::with_capacity(weight_count);
    for &target_row in target_rows {
        weights.extend(coset_extend_row_weights(
            source_len,
            target_len,
            source_root,
            target_root,
            target_bits,
            target_row,
        )?);
    }
    Ok(weights)
}

fn validate_output_len(
    out: &CudaDeviceBuffer,
    column_count: usize,
    target_bits: usize,
    target_row_count: usize,
) -> Result<(), AccelError> {
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }
    let out_words =
        target_row_count
            .checked_mul(column_count)
            .ok_or(AccelError::InvalidDomain {
                bits: target_bits,
                len: target_row_count,
            })?;
    let out_bytes = out_words.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits: target_bits,
        len: out_words,
    })?;
    if out.len() != out_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: out_bytes,
            rhs: out.len(),
        });
    }
    Ok(())
}

pub fn cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    target_rows: &[usize],
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() && target_rows.is_empty() {
            return Ok(());
        }
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }
    validate_output_len(out, column_count, target_bits, target_rows.len())?;

    let source_words = values.len() / 8;
    if !source_words.is_multiple_of(column_count) {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_words,
        });
    }
    let source_rows = source_words / column_count;
    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_words,
        });
    }
    if target_rows.is_empty() {
        return Ok(());
    }
    let weights = selected_row_weights(
        source_len,
        target_len,
        source_root,
        target_root,
        target_bits,
        target_rows,
    )?;
    ensure_cuda_setup(target_bits)?;
    let weights_buffer = CudaDeviceBuffer::from_u64_words(&weights)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device_raw(
            values.as_raw_ptr() as *const u64,
            weights_buffer.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            column_count,
            target_rows.len(),
        )
    };
    cuda_status(code)
}

pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    target_rows: &[usize],
) -> Result<(), AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
        if values.is_empty() && out.is_empty() && target_rows.is_empty() {
            return Ok(());
        }
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    if column_offset > source_row_stride || column_count > source_row_stride - column_offset {
        return Err(AccelError::InvalidDomain {
            bits: source_row_stride,
            len: column_count,
        });
    }
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }
    validate_output_len(out, column_count, target_bits, target_rows.len())?;

    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_rows,
        });
    }
    let required_source_words = source_rows
        .checked_sub(1)
        .and_then(|last_row| last_row.checked_mul(source_row_stride))
        .and_then(|base| base.checked_add(column_offset))
        .and_then(|base| base.checked_add(column_count))
        .ok_or(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_rows,
        })?;
    if values.len() / 8 < required_source_words {
        return Err(AccelError::LengthMismatch {
            lhs: values.len() / 8,
            rhs: required_source_words,
        });
    }
    if target_rows.is_empty() {
        return Ok(());
    }
    let weights = selected_row_weights(
        source_len,
        target_len,
        source_root,
        target_root,
        target_bits,
        target_rows,
    )?;
    ensure_cuda_setup(target_bits)?;
    let weights_buffer = CudaDeviceBuffer::from_u64_words(&weights)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device_raw(
            values.as_raw_ptr() as *const u64,
            weights_buffer.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_row_stride,
            column_offset,
            column_count,
            target_rows.len(),
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_coset_extend_row_major_columns_device, AccelError, CudaDeviceBuffer,
};
#[cfg(not(feature = "cuda"))]
use lzvm_field::coset_extend_evaluations;
use lzvm_field::Felt;
#[cfg(feature = "cuda")]
use lzvm_field::MAX_ROOT_OF_UNITY_BITS;

#[cfg(feature = "cuda")]
use crate::gpu_setup::prepare_gpu_setup;
use crate::witness_layout::WitnessTraceStageValues;

use super::{WitnessStageLeafError, WitnessStageLeaves, WORD_BYTES};

pub fn extend_witness_stage_leaves(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
) -> Result<WitnessStageLeaves, WitnessStageLeafError> {
    let columns = stage.column_count();
    let rows = stage.row_count();
    let bytes =
        extend_witness_stage_row_major_bytes(stage.values(), columns, source_bits, target_bits)?;
    let extended_rows = if columns == 0 {
        0
    } else {
        bytes
            .len()
            .checked_div(WORD_BYTES)
            .ok_or(WitnessStageLeafError::LengthOverflow)?
            .checked_div(columns)
            .ok_or(WitnessStageLeafError::LengthOverflow)?
    };

    Ok(WitnessStageLeaves::new(
        stage.stage_index(),
        rows,
        extended_rows,
        columns,
        bytes,
    ))
}

#[cfg(feature = "cuda")]
pub fn extend_witness_stage_leaves_with_cuda(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
) -> Result<WitnessStageLeaves, WitnessStageLeafError> {
    extend_witness_stage_leaves(stage, source_bits, target_bits)
}

#[cfg(feature = "cuda")]
fn extend_witness_stage_row_major_bytes(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<u8>, WitnessStageLeafError> {
    let out_byte_count =
        validate_cuda_extension_domain(values.len(), column_count, source_bits, target_bits)?;
    prepare_gpu_setup(target_bits)?;

    let source_bytes = row_major_felt_bytes(values)?;
    let mut source_buffer = CudaDeviceBuffer::new(source_bytes.len())?;
    source_buffer.copy_from(&source_bytes)?;
    let mut output_buffer = CudaDeviceBuffer::new(out_byte_count)?;

    cuda_goldilocks_coset_extend_row_major_columns_device(
        &source_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
    )?;
    let bytes = output_buffer.to_vec()?;
    validate_row_major_word_bytes(&bytes)?;
    Ok(bytes)
}

#[cfg(feature = "cuda")]
fn validate_cuda_extension_domain(
    value_count: usize,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<usize, WitnessStageLeafError> {
    let source_rows = validate_cuda_source_shape(value_count, column_count, source_bits)?;
    if column_count == 0 {
        return Ok(0);
    }
    if target_bits < source_bits {
        return Err(cuda_invalid_domain(target_bits, source_rows));
    }
    let source_len = cuda_domain_len(source_bits, source_rows)?;
    if source_rows != source_len {
        return Err(cuda_invalid_domain(source_bits, value_count));
    }
    let target_rows = cuda_domain_len(target_bits, source_rows)?;
    let target_words = target_rows
        .checked_mul(column_count)
        .ok_or_else(|| cuda_invalid_domain(target_bits, value_count))?;
    target_words
        .checked_mul(WORD_BYTES)
        .ok_or_else(|| cuda_invalid_domain(target_bits, target_words))
}

#[cfg(feature = "cuda")]
fn validate_cuda_source_shape(
    value_count: usize,
    column_count: usize,
    source_bits: usize,
) -> Result<usize, WitnessStageLeafError> {
    if column_count == 0 {
        if value_count == 0 {
            return Ok(0);
        }
        return Err(cuda_invalid_domain(source_bits, value_count));
    }
    if !value_count.is_multiple_of(column_count) {
        return Err(cuda_invalid_domain(source_bits, value_count));
    }
    Ok(value_count / column_count)
}

#[cfg(feature = "cuda")]
fn cuda_domain_len(bits: usize, len: usize) -> Result<usize, WitnessStageLeafError> {
    if bits > MAX_ROOT_OF_UNITY_BITS {
        return Err(cuda_invalid_domain(bits, len));
    }
    let shift = u32::try_from(bits).map_err(|_| cuda_invalid_domain(bits, len))?;
    1_usize
        .checked_shl(shift)
        .ok_or_else(|| cuda_invalid_domain(bits, len))
}

#[cfg(feature = "cuda")]
fn cuda_invalid_domain(bits: usize, len: usize) -> WitnessStageLeafError {
    AccelError::InvalidDomain { bits, len }.into()
}

#[cfg(feature = "cuda")]
fn row_major_felt_bytes(values: &[Felt]) -> Result<Vec<u8>, WitnessStageLeafError> {
    let mut bytes = Vec::with_capacity(
        values
            .len()
            .checked_mul(WORD_BYTES)
            .ok_or(WitnessStageLeafError::LengthOverflow)?,
    );
    for value in values {
        bytes.extend_from_slice(&value.to_u64().to_le_bytes());
    }
    Ok(bytes)
}

#[cfg(feature = "cuda")]
fn validate_row_major_word_bytes(bytes: &[u8]) -> Result<(), WitnessStageLeafError> {
    if !bytes.len().is_multiple_of(WORD_BYTES) {
        return Err(WitnessStageLeafError::LengthOverflow);
    }
    for chunk in bytes.chunks_exact(WORD_BYTES) {
        let word = u64::from_le_bytes(chunk.try_into().expect("chunk length checked"));
        Felt::from_canonical(word)?;
    }
    Ok(())
}

#[cfg(not(feature = "cuda"))]
fn extend_witness_stage_row_major_bytes(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<u8>, WitnessStageLeafError> {
    let extended_values =
        extend_witness_stage_row_major_values(values, column_count, source_bits, target_bits)?;
    let byte_count = extended_values
        .len()
        .checked_mul(WORD_BYTES)
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    let mut bytes = Vec::with_capacity(byte_count);
    for value in extended_values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    Ok(bytes)
}

#[cfg(not(feature = "cuda"))]
fn extend_witness_stage_row_major_values(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<Felt>, WitnessStageLeafError> {
    if column_count == 0 {
        return Ok(Vec::new());
    }
    let source_rows = values
        .len()
        .checked_div(column_count)
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    if source_rows
        .checked_mul(column_count)
        .ok_or(WitnessStageLeafError::LengthOverflow)?
        != values.len()
    {
        return Err(WitnessStageLeafError::LengthOverflow);
    }

    let mut extended_columns = Vec::with_capacity(column_count);
    for column in 0..column_count {
        let mut source = Vec::with_capacity(source_rows);
        for row in 0..source_rows {
            source.push(values[row * column_count + column]);
        }
        extended_columns.push(coset_extend_evaluations(&source, source_bits, target_bits)?);
    }

    let extended_rows = extended_columns.first().map_or(0, Vec::len);
    Ok((0..extended_rows)
        .flat_map(|row| extended_columns.iter().map(move |column| column[row]))
        .collect())
}

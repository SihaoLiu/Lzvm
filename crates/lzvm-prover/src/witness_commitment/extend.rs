#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_coset_extend_row_major_columns_device,
    cuda_goldilocks_coset_extend_row_major_columns_output_bytes, CudaDeviceBuffer,
};
#[cfg(not(feature = "cuda"))]
use lzvm_field::coset_extend_evaluations;
use lzvm_field::Felt;

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
    let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
        values.len(),
        column_count,
        source_bits,
        target_bits,
    )?;
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

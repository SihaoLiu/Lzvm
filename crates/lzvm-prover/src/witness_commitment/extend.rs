#[cfg(feature = "cuda")]
use lzvm_accel::{cuda_goldilocks_coset_extend_device, CudaDeviceBuffer};
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
    #[cfg(feature = "cuda")]
    prepare_gpu_setup(target_bits)?;

    let columns = stage.column_count();
    let rows = stage.row_count();
    let mut extended_columns = Vec::with_capacity(columns);
    for column in 0..columns {
        let mut source = Vec::with_capacity(rows);
        for row in 0..rows {
            let index = row
                .checked_mul(columns)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(WitnessStageLeafError::LengthOverflow)?;
            source.push(stage.values()[index]);
        }
        extended_columns.push(extend_witness_stage_column_values(
            &source,
            source_bits,
            target_bits,
        )?);
    }

    let extended_rows = extended_columns.first().map_or(0, Vec::len);
    let byte_count = extended_rows
        .checked_mul(columns)
        .and_then(|count| count.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    let mut bytes = Vec::with_capacity(byte_count);
    for row in 0..extended_rows {
        for column_values in &extended_columns {
            bytes.extend_from_slice(&column_values[row].to_le_bytes());
        }
    }

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
fn extend_witness_stage_column_values(
    source: &[Felt],
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<Felt>, WitnessStageLeafError> {
    let source_words = source
        .iter()
        .map(|value| value.to_u64())
        .collect::<Vec<_>>();
    if target_bits < source_bits {
        return Err(WitnessStageLeafError::Accel(
            lzvm_accel::AccelError::InvalidDomain {
                bits: target_bits,
                len: source_words.len(),
            },
        ));
    }
    let source_word_count = 1_usize
        .checked_shl(u32::try_from(source_bits).map_err(|_| WitnessStageLeafError::LengthOverflow)?)
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    if source_words.len() != source_word_count {
        return Err(WitnessStageLeafError::Accel(
            lzvm_accel::AccelError::InvalidDomain {
                bits: source_bits,
                len: source_words.len(),
            },
        ));
    }
    let target_words = 1_usize
        .checked_shl(u32::try_from(target_bits).map_err(|_| WitnessStageLeafError::LengthOverflow)?)
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    let source_buffer = CudaDeviceBuffer::from_u64_words(&source_words)?;
    let mut target_buffer = CudaDeviceBuffer::new(
        target_words
            .checked_mul(8)
            .ok_or(WitnessStageLeafError::LengthOverflow)?,
    )?;
    cuda_goldilocks_coset_extend_device(
        &source_buffer,
        &mut target_buffer,
        source_bits,
        target_bits,
    )?;
    target_buffer
        .to_u64_words()?
        .into_iter()
        .map(Felt::from_canonical)
        .collect::<Result<Vec<_>, _>>()
        .map_err(WitnessStageLeafError::from)
}

#[cfg(not(feature = "cuda"))]
fn extend_witness_stage_column_values(
    source: &[Felt],
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<Felt>, WitnessStageLeafError> {
    Ok(coset_extend_evaluations(source, source_bits, target_bits)?)
}

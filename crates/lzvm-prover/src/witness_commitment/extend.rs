use std::time::Duration;
#[cfg(feature = "cuda")]
use std::time::Instant;

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
#[cfg(feature = "cuda")]
use crate::merkle_hash::{
    linear_hashes_from_validated_wide_row_major_device_buffer,
    padded_hashes_from_validated_row_major_bytes,
};
use crate::witness_layout::WitnessTraceStageValues;

#[cfg(feature = "cuda")]
use super::{WitnessStageCommitmentError, WitnessTraceCommitmentError, HASH_WORDS};
use super::{WitnessStageLeafError, WitnessStageLeaves, WORD_BYTES};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WitnessStageLeafExtendTiming {
    setup_duration: Duration,
    upload_duration: Duration,
    kernel_duration: Duration,
    download_duration: Duration,
    validate_duration: Duration,
    leaf_hash_duration: Duration,
}

impl WitnessStageLeafExtendTiming {
    pub(crate) fn accumulate(&mut self, other: Self) {
        self.setup_duration += other.setup_duration;
        self.upload_duration += other.upload_duration;
        self.kernel_duration += other.kernel_duration;
        self.download_duration += other.download_duration;
        self.validate_duration += other.validate_duration;
        self.leaf_hash_duration += other.leaf_hash_duration;
    }

    pub(crate) fn setup_duration(&self) -> Duration {
        self.setup_duration
    }

    pub(crate) fn upload_duration(&self) -> Duration {
        self.upload_duration
    }

    pub(crate) fn kernel_duration(&self) -> Duration {
        self.kernel_duration
    }

    pub(crate) fn download_duration(&self) -> Duration {
        self.download_duration
    }

    pub(crate) fn validate_duration(&self) -> Duration {
        self.validate_duration
    }

    pub(crate) fn leaf_hash_duration(&self) -> Duration {
        self.leaf_hash_duration
    }
}

pub fn extend_witness_stage_leaves(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
) -> Result<WitnessStageLeaves, WitnessStageLeafError> {
    let columns = stage.column_count();
    let rows = stage.row_count();
    let bytes =
        extend_witness_stage_row_major_bytes(stage.values(), columns, source_bits, target_bits)?;
    let extended_rows = extended_row_count_from_bytes(bytes.len(), columns)?;

    Ok(WitnessStageLeaves::new(
        stage.stage_index(),
        rows,
        extended_rows,
        columns,
        bytes,
    ))
}

#[cfg(feature = "cuda")]
pub(crate) fn extend_witness_stage_leaves_with_leaf_hashes(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
) -> Result<(WitnessStageLeaves, Vec<[Felt; HASH_WORDS]>), WitnessTraceCommitmentError> {
    let columns = stage.column_count();
    let rows = stage.row_count();
    let (bytes, leaf_hashes) = extend_witness_stage_row_major_bytes_with_leaf_hashes(
        stage.values(),
        columns,
        source_bits,
        target_bits,
        arity,
    )?;
    let extended_rows = extended_row_count_from_bytes(bytes.len(), columns)?;

    Ok((
        WitnessStageLeaves::new(stage.stage_index(), rows, extended_rows, columns, bytes),
        leaf_hashes,
    ))
}

#[cfg(feature = "cuda")]
pub(crate) fn extend_witness_stage_leaves_with_leaf_hashes_and_timing(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<(WitnessStageLeaves, Vec<[Felt; HASH_WORDS]>), WitnessTraceCommitmentError> {
    let columns = stage.column_count();
    let rows = stage.row_count();
    let (bytes, leaf_hashes) = extend_witness_stage_row_major_bytes_with_leaf_hashes_timed(
        stage.values(),
        columns,
        source_bits,
        target_bits,
        arity,
        timing,
    )?;
    let extended_rows = extended_row_count_from_bytes(bytes.len(), columns)?;

    Ok((
        WitnessStageLeaves::new(stage.stage_index(), rows, extended_rows, columns, bytes),
        leaf_hashes,
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
    )
    .map_err(WitnessStageLeafError::from)?;
    prepare_gpu_setup(target_bits).map_err(WitnessStageLeafError::from)?;

    let source_buffer = CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(values))
        .map_err(WitnessStageLeafError::from)?;
    let mut output_buffer =
        CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from)?;

    cuda_goldilocks_coset_extend_row_major_columns_device(
        &source_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
    )
    .map_err(WitnessStageLeafError::from)?;
    let bytes = output_buffer
        .to_vec()
        .map_err(WitnessStageLeafError::from)?;
    validate_row_major_word_bytes(&bytes)?;
    Ok(bytes)
}

#[cfg(feature = "cuda")]
fn extend_witness_stage_row_major_bytes_with_leaf_hashes(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
) -> Result<(Vec<u8>, Vec<[Felt; HASH_WORDS]>), WitnessTraceCommitmentError> {
    let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
        values.len(),
        column_count,
        source_bits,
        target_bits,
    )
    .map_err(WitnessStageLeafError::from)?;
    prepare_gpu_setup(target_bits).map_err(WitnessStageLeafError::from)?;

    let source_buffer = CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(values))
        .map_err(WitnessStageLeafError::from)?;
    let mut output_buffer =
        CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from)?;

    cuda_goldilocks_coset_extend_row_major_columns_device(
        &source_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
    )
    .map_err(WitnessStageLeafError::from)?;
    let bytes = output_buffer
        .to_vec()
        .map_err(WitnessStageLeafError::from)?;
    validate_row_major_word_bytes(&bytes)?;
    let extended_rows = extended_row_count_from_bytes(bytes.len(), column_count)?;
    let leaf_hashes = if column_count <= HASH_WORDS {
        padded_hashes_from_validated_row_major_bytes(&bytes, extended_rows, column_count, arity)
    } else {
        linear_hashes_from_validated_wide_row_major_device_buffer(
            &output_buffer,
            extended_rows,
            column_count,
            arity,
        )
    }
    .map_err(WitnessStageCommitmentError::from)?;
    Ok((bytes, leaf_hashes))
}

#[cfg(feature = "cuda")]
fn extend_witness_stage_row_major_bytes_with_leaf_hashes_timed(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<(Vec<u8>, Vec<[Felt; HASH_WORDS]>), WitnessTraceCommitmentError> {
    let out_byte_count = record_duration(&mut timing.setup_duration, || {
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            target_bits,
        )
        .map_err(WitnessStageLeafError::from)?;
        prepare_gpu_setup(target_bits).map_err(WitnessStageLeafError::from)?;
        Ok::<_, WitnessStageLeafError>(out_byte_count)
    })?;

    let source_buffer = record_duration(&mut timing.upload_duration, || {
        CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(values))
            .map_err(WitnessStageLeafError::from)
    })?;
    let mut output_buffer = record_duration(&mut timing.setup_duration, || {
        CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from)
    })?;

    record_duration(&mut timing.kernel_duration, || {
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source_buffer,
            &mut output_buffer,
            column_count,
            source_bits,
            target_bits,
        )
        .map_err(WitnessStageLeafError::from)
    })?;
    let bytes = record_duration(&mut timing.download_duration, || {
        output_buffer.to_vec().map_err(WitnessStageLeafError::from)
    })?;
    record_duration(&mut timing.validate_duration, || {
        validate_row_major_word_bytes(&bytes)
    })?;
    let extended_rows = extended_row_count_from_bytes(bytes.len(), column_count)?;
    let leaf_hashes = record_duration(&mut timing.leaf_hash_duration, || {
        if column_count <= HASH_WORDS {
            padded_hashes_from_validated_row_major_bytes(&bytes, extended_rows, column_count, arity)
        } else {
            linear_hashes_from_validated_wide_row_major_device_buffer(
                &output_buffer,
                extended_rows,
                column_count,
                arity,
            )
        }
        .map_err(WitnessStageCommitmentError::from)
    })?;
    Ok((bytes, leaf_hashes))
}

#[cfg(feature = "cuda")]
fn record_duration<T, E>(
    duration: &mut Duration,
    run: impl FnOnce() -> Result<T, E>,
) -> Result<T, WitnessTraceCommitmentError>
where
    WitnessTraceCommitmentError: From<E>,
{
    let started = Instant::now();
    let result = run().map_err(WitnessTraceCommitmentError::from)?;
    *duration += started.elapsed();
    Ok(result)
}

fn extended_row_count_from_bytes(
    byte_count: usize,
    column_count: usize,
) -> Result<usize, WitnessStageLeafError> {
    if column_count == 0 {
        return Ok(0);
    }
    byte_count
        .checked_div(WORD_BYTES)
        .ok_or(WitnessStageLeafError::LengthOverflow)?
        .checked_div(column_count)
        .ok_or(WitnessStageLeafError::LengthOverflow)
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

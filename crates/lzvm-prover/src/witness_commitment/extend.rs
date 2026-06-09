use std::time::Duration;
#[cfg(feature = "cuda")]
use std::time::Instant;

#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_begin_validate_canonical_words_device,
    cuda_goldilocks_coset_extend_row_major_columns_device,
    cuda_goldilocks_coset_extend_row_major_columns_device_unsynced,
    cuda_goldilocks_coset_extend_row_major_columns_output_bytes,
    cuda_goldilocks_coset_extend_row_major_columns_strided_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced,
    cuda_goldilocks_validate_canonical_words_device, AccelError, CudaCanonicalCheck,
    CudaDeviceBuffer, CudaRowMajorColumnView,
};
#[cfg(not(feature = "cuda"))]
use lzvm_field::coset_extend_evaluations;
use lzvm_field::Felt;

#[cfg(feature = "cuda")]
use crate::gpu_setup::prepare_gpu_setup;
#[cfg(feature = "cuda")]
use crate::merkle_hash::{
    linear_hash_level_from_validated_row_major_device_buffer,
    linear_hashes_from_validated_wide_row_major_device_buffer, CudaDigestCheckpointLevel,
    CudaDigestLevel, CudaDigestRoot,
};
use crate::witness_layout::WitnessTraceStageValues;

use super::{coset_extend_launch_work, WitnessStageLeafError, WitnessStageLeaves, WORD_BYTES};
#[cfg(feature = "cuda")]
use super::{
    WitnessStageCommitmentError, WitnessStageSourceDeviceView, WitnessTraceCommitmentError,
    HASH_WORDS,
};

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) struct PendingCanonicalCudaDigestLevel {
    level: CudaDigestLevel,
    canonical_check: CudaCanonicalCheck,
}

#[cfg(feature = "cuda")]
#[derive(Default)]
pub(crate) struct WitnessStageLeafWorkspaceCache {
    buffer: Option<CudaDeviceBuffer>,
}

#[cfg(feature = "cuda")]
impl WitnessStageLeafWorkspaceCache {
    fn workspace(
        &mut self,
        byte_count: usize,
        timing: &mut WitnessStageLeafExtendTiming,
    ) -> Result<&mut CudaDeviceBuffer, WitnessTraceCommitmentError> {
        let needs_alloc = self
            .buffer
            .as_ref()
            .is_none_or(|buffer| buffer.len() < byte_count);
        if needs_alloc {
            let buffer = record_setup_duration(
                &mut timing.setup_duration,
                &mut timing.leaf_setup_workspace_alloc_duration,
                || CudaDeviceBuffer::new(byte_count).map_err(WitnessStageLeafError::from),
            )?;
            timing.record_workspace_alloc(byte_count);
            self.buffer = Some(buffer);
        }
        Ok(self
            .buffer
            .as_mut()
            .expect("workspace cache should contain an allocated buffer"))
    }
}

#[cfg(feature = "cuda")]
impl PendingCanonicalCudaDigestLevel {
    fn new(level: CudaDigestLevel, canonical_check: CudaCanonicalCheck) -> Self {
        Self {
            level,
            canonical_check,
        }
    }

    pub(crate) fn state_count(&self) -> usize {
        self.level.state_count()
    }

    pub(crate) fn arity(&self) -> usize {
        self.level.arity()
    }

    pub(crate) fn root(&self) -> Result<[Felt; HASH_WORDS], crate::merkle_hash::MerkleHashError> {
        self.level.root()
    }

    pub(crate) fn root_device(
        &self,
    ) -> Result<CudaDigestRoot, crate::merkle_hash::MerkleHashError> {
        self.level.root_device()
    }

    pub(crate) fn parent_checkpoint_level(
        &self,
        max_state_count: usize,
    ) -> Result<Option<CudaDigestCheckpointLevel>, crate::merkle_hash::MerkleHashError> {
        self.level.parent_checkpoint_level(max_state_count)
    }

    pub(crate) fn finish_canonical_check(&self) -> Result<(), WitnessStageLeafError> {
        if self
            .canonical_check
            .is_canonical()
            .map_err(WitnessStageLeafError::from)?
        {
            Ok(())
        } else {
            Err(WitnessStageLeafError::NonCanonicalDeviceWord)
        }
    }

    pub(crate) fn into_validated_level(self) -> Result<CudaDigestLevel, WitnessStageLeafError> {
        self.finish_canonical_check()?;
        Ok(self.level)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WitnessStageLeafExtendTiming {
    setup_duration: Duration,
    leaf_setup_prepare_duration: Duration,
    leaf_setup_output_alloc_duration: Duration,
    leaf_setup_workspace_alloc_duration: Duration,
    leaf_setup_output_alloc_byte_count: usize,
    leaf_setup_workspace_alloc_byte_count: usize,
    leaf_setup_output_alloc_count: usize,
    leaf_setup_workspace_alloc_count: usize,
    upload_duration: Duration,
    kernel_duration: Duration,
    download_duration: Duration,
    validate_duration: Duration,
    leaf_hash_duration: Duration,
    leaf_hash_row_count: usize,
    leaf_hash_byte_count: usize,
    leaf_hash_arity2_row_count: usize,
    leaf_hash_arity2_byte_count: usize,
    leaf_hash_arity4_row_count: usize,
    leaf_hash_arity4_byte_count: usize,
    leaf_coset_extend_call_count: usize,
    leaf_coset_extend_output_byte_count: usize,
    leaf_coset_extend_column_count: usize,
    leaf_coset_extend_max_column_count: usize,
    leaf_coset_extend_ntt_launch_count: usize,
    leaf_coset_extend_bit_reverse_launch_count: usize,
    leaf_coset_extend_ntt_stage_launch_count: usize,
    leaf_coset_extend_ntt_block_twiddle_launch_count: usize,
    leaf_coset_extend_normalize_launch_count: usize,
    leaf_coset_extend_pack_launch_count: usize,
    leaf_coset_extend_unpack_launch_count: usize,
}

impl WitnessStageLeafExtendTiming {
    pub(crate) fn accumulate(&mut self, other: Self) {
        self.setup_duration += other.setup_duration;
        self.leaf_setup_prepare_duration += other.leaf_setup_prepare_duration;
        self.leaf_setup_output_alloc_duration += other.leaf_setup_output_alloc_duration;
        self.leaf_setup_workspace_alloc_duration += other.leaf_setup_workspace_alloc_duration;
        self.leaf_setup_output_alloc_byte_count += other.leaf_setup_output_alloc_byte_count;
        self.leaf_setup_workspace_alloc_byte_count += other.leaf_setup_workspace_alloc_byte_count;
        self.leaf_setup_output_alloc_count += other.leaf_setup_output_alloc_count;
        self.leaf_setup_workspace_alloc_count += other.leaf_setup_workspace_alloc_count;
        self.upload_duration += other.upload_duration;
        self.kernel_duration += other.kernel_duration;
        self.download_duration += other.download_duration;
        self.validate_duration += other.validate_duration;
        self.leaf_hash_duration += other.leaf_hash_duration;
        self.leaf_hash_row_count += other.leaf_hash_row_count;
        self.leaf_hash_byte_count += other.leaf_hash_byte_count;
        self.leaf_hash_arity2_row_count += other.leaf_hash_arity2_row_count;
        self.leaf_hash_arity2_byte_count += other.leaf_hash_arity2_byte_count;
        self.leaf_hash_arity4_row_count += other.leaf_hash_arity4_row_count;
        self.leaf_hash_arity4_byte_count += other.leaf_hash_arity4_byte_count;
        self.leaf_coset_extend_call_count += other.leaf_coset_extend_call_count;
        self.leaf_coset_extend_output_byte_count += other.leaf_coset_extend_output_byte_count;
        self.leaf_coset_extend_column_count += other.leaf_coset_extend_column_count;
        self.leaf_coset_extend_max_column_count = self
            .leaf_coset_extend_max_column_count
            .max(other.leaf_coset_extend_max_column_count);
        self.leaf_coset_extend_ntt_launch_count += other.leaf_coset_extend_ntt_launch_count;
        self.leaf_coset_extend_bit_reverse_launch_count +=
            other.leaf_coset_extend_bit_reverse_launch_count;
        self.leaf_coset_extend_ntt_stage_launch_count +=
            other.leaf_coset_extend_ntt_stage_launch_count;
        self.leaf_coset_extend_ntt_block_twiddle_launch_count +=
            other.leaf_coset_extend_ntt_block_twiddle_launch_count;
        self.leaf_coset_extend_normalize_launch_count +=
            other.leaf_coset_extend_normalize_launch_count;
        self.leaf_coset_extend_pack_launch_count += other.leaf_coset_extend_pack_launch_count;
        self.leaf_coset_extend_unpack_launch_count += other.leaf_coset_extend_unpack_launch_count;
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    fn record_leaf_hash_work(&mut self, row_count: usize, byte_count: usize, arity: usize) {
        self.leaf_hash_row_count += row_count;
        self.leaf_hash_byte_count += byte_count;
        match arity {
            2 => {
                self.leaf_hash_arity2_row_count += row_count;
                self.leaf_hash_arity2_byte_count += byte_count;
            }
            4 => {
                self.leaf_hash_arity4_row_count += row_count;
                self.leaf_hash_arity4_byte_count += byte_count;
            }
            _ => {}
        }
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    fn record_coset_extend_work(
        &mut self,
        output_byte_count: usize,
        column_count: usize,
        source_bits: usize,
        target_bits: usize,
    ) {
        self.leaf_coset_extend_call_count += 1;
        self.leaf_coset_extend_output_byte_count += output_byte_count;
        self.leaf_coset_extend_column_count += column_count;
        self.leaf_coset_extend_max_column_count =
            self.leaf_coset_extend_max_column_count.max(column_count);
        let work = coset_extend_launch_work(column_count, source_bits, target_bits);
        self.leaf_coset_extend_ntt_launch_count += work.ntt_launch_count;
        self.leaf_coset_extend_bit_reverse_launch_count += work.bit_reverse_launch_count;
        self.leaf_coset_extend_ntt_stage_launch_count += work.ntt_stage_launch_count;
        self.leaf_coset_extend_ntt_block_twiddle_launch_count +=
            work.ntt_block_twiddle_launch_count;
        self.leaf_coset_extend_normalize_launch_count += work.normalize_launch_count;
        self.leaf_coset_extend_pack_launch_count += work.pack_launch_count;
        self.leaf_coset_extend_unpack_launch_count += work.unpack_launch_count;
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    fn record_output_alloc(&mut self, byte_count: usize) {
        self.leaf_setup_output_alloc_byte_count += byte_count;
        self.leaf_setup_output_alloc_count += 1;
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    fn record_workspace_alloc(&mut self, byte_count: usize) {
        self.leaf_setup_workspace_alloc_byte_count += byte_count;
        self.leaf_setup_workspace_alloc_count += 1;
    }

    pub(crate) fn setup_duration(&self) -> Duration {
        self.setup_duration
    }

    pub(crate) fn leaf_setup_prepare_duration(&self) -> Duration {
        self.leaf_setup_prepare_duration
    }

    pub(crate) fn leaf_setup_output_alloc_duration(&self) -> Duration {
        self.leaf_setup_output_alloc_duration
    }

    pub(crate) fn leaf_setup_workspace_alloc_duration(&self) -> Duration {
        self.leaf_setup_workspace_alloc_duration
    }

    pub(crate) fn leaf_setup_output_alloc_byte_count(&self) -> usize {
        self.leaf_setup_output_alloc_byte_count
    }

    pub(crate) fn leaf_setup_workspace_alloc_byte_count(&self) -> usize {
        self.leaf_setup_workspace_alloc_byte_count
    }

    pub(crate) fn leaf_setup_output_alloc_count(&self) -> usize {
        self.leaf_setup_output_alloc_count
    }

    pub(crate) fn leaf_setup_workspace_alloc_count(&self) -> usize {
        self.leaf_setup_workspace_alloc_count
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

    pub(crate) fn leaf_hash_row_count(&self) -> usize {
        self.leaf_hash_row_count
    }

    pub(crate) fn leaf_hash_byte_count(&self) -> usize {
        self.leaf_hash_byte_count
    }

    pub(crate) fn leaf_hash_arity2_row_count(&self) -> usize {
        self.leaf_hash_arity2_row_count
    }

    pub(crate) fn leaf_hash_arity2_byte_count(&self) -> usize {
        self.leaf_hash_arity2_byte_count
    }

    pub(crate) fn leaf_hash_arity4_row_count(&self) -> usize {
        self.leaf_hash_arity4_row_count
    }

    pub(crate) fn leaf_hash_arity4_byte_count(&self) -> usize {
        self.leaf_hash_arity4_byte_count
    }

    pub(crate) fn leaf_coset_extend_call_count(&self) -> usize {
        self.leaf_coset_extend_call_count
    }

    pub(crate) fn leaf_coset_extend_output_byte_count(&self) -> usize {
        self.leaf_coset_extend_output_byte_count
    }

    pub(crate) fn leaf_coset_extend_column_count(&self) -> usize {
        self.leaf_coset_extend_column_count
    }

    pub(crate) fn leaf_coset_extend_max_column_count(&self) -> usize {
        self.leaf_coset_extend_max_column_count
    }

    pub(crate) fn leaf_coset_extend_ntt_launch_count(&self) -> usize {
        self.leaf_coset_extend_ntt_launch_count
    }

    pub(crate) fn leaf_coset_extend_bit_reverse_launch_count(&self) -> usize {
        self.leaf_coset_extend_bit_reverse_launch_count
    }

    pub(crate) fn leaf_coset_extend_ntt_stage_launch_count(&self) -> usize {
        self.leaf_coset_extend_ntt_stage_launch_count
    }

    pub(crate) fn leaf_coset_extend_ntt_block_twiddle_launch_count(&self) -> usize {
        self.leaf_coset_extend_ntt_block_twiddle_launch_count
    }

    pub(crate) fn leaf_coset_extend_normalize_launch_count(&self) -> usize {
        self.leaf_coset_extend_normalize_launch_count
    }

    pub(crate) fn leaf_coset_extend_pack_launch_count(&self) -> usize {
        self.leaf_coset_extend_pack_launch_count
    }

    pub(crate) fn leaf_coset_extend_unpack_launch_count(&self) -> usize {
        self.leaf_coset_extend_unpack_launch_count
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
pub(crate) fn extend_witness_stage_leaves_from_source_device_view(
    stage_index: usize,
    row_count: usize,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    source_device: &WitnessStageSourceDeviceView,
) -> Result<WitnessStageLeaves, WitnessStageLeafError> {
    if !source_device.has_matching_shape(row_count, column_count) {
        return Err(WitnessStageLeafError::LengthOverflow);
    }
    let required_source_bytes = source_device
        .required_byte_len()
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    if source_device.buffer().len() < required_source_bytes {
        return Err(WitnessStageLeafError::Accel(AccelError::LengthMismatch {
            lhs: source_device.buffer().len(),
            rhs: required_source_bytes,
        }));
    }
    let value_count = row_count
        .checked_mul(column_count)
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
        value_count,
        column_count,
        source_bits,
        target_bits,
    )
    .map_err(WitnessStageLeafError::from)?;
    prepare_gpu_setup(target_bits).map_err(WitnessStageLeafError::from)?;
    let mut output_buffer =
        CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from)?;
    if source_device.row_stride() == column_count && source_device.column_offset() == 0 {
        cuda_goldilocks_coset_extend_row_major_columns_device(
            source_device.buffer(),
            &mut output_buffer,
            column_count,
            source_bits,
            target_bits,
        )
    } else {
        cuda_goldilocks_coset_extend_row_major_columns_strided_device(
            source_device.buffer(),
            &mut output_buffer,
            CudaRowMajorColumnView {
                source_rows: row_count,
                source_row_stride: source_device.row_stride(),
                column_offset: source_device.column_offset(),
                column_count,
            },
            source_bits,
            target_bits,
        )
    }
    .map_err(WitnessStageLeafError::from)?;
    validate_row_major_device_words(&output_buffer, out_byte_count)?;
    let bytes = output_buffer
        .to_vec()
        .map_err(WitnessStageLeafError::from)?;
    let extended_rows = extended_row_count_from_bytes(bytes.len(), column_count)?;
    Ok(WitnessStageLeaves::new(
        stage_index,
        row_count,
        extended_rows,
        column_count,
        bytes,
    ))
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) fn compact_witness_stage_leaf_hashes_with_source_device_timing(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_device: Option<&CudaDeviceBuffer>,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<Vec<[Felt; HASH_WORDS]>, WitnessTraceCommitmentError> {
    compact_witness_stage_leaf_hashes_timed(
        stage.values(),
        stage.column_count(),
        source_bits,
        target_bits,
        arity,
        source_device,
        timing,
    )
}

#[cfg(feature = "cuda")]
pub(crate) fn compact_witness_stage_leaf_hash_level_with_source_device_timing(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_device: Option<&CudaDeviceBuffer>,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<PendingCanonicalCudaDigestLevel, WitnessTraceCommitmentError> {
    compact_witness_stage_leaf_hash_level_timed(
        stage.values(),
        stage.column_count(),
        source_bits,
        target_bits,
        arity,
        source_device,
        timing,
    )
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) fn compact_witness_stage_leaf_hash_level_from_source_device_timing(
    row_count: usize,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_device: &CudaDeviceBuffer,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<PendingCanonicalCudaDigestLevel, WitnessTraceCommitmentError> {
    let value_count = row_count
        .checked_mul(column_count)
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    let expected_source_bytes = value_count
        .checked_mul(WORD_BYTES)
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    if source_device.len() != expected_source_bytes {
        return Err(WitnessStageLeafError::Accel(AccelError::LengthMismatch {
            lhs: source_device.len(),
            rhs: expected_source_bytes,
        })
        .into());
    }
    compact_witness_stage_leaf_hash_level_from_source_device_timed(
        source_device,
        CudaRowMajorColumnView {
            source_rows: row_count,
            source_row_stride: column_count,
            column_offset: 0,
            column_count,
        },
        source_bits,
        target_bits,
        arity,
        None,
        timing,
    )
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) fn compact_witness_stage_leaf_hash_level_from_source_device_view_timing(
    row_count: usize,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_device: &WitnessStageSourceDeviceView,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<PendingCanonicalCudaDigestLevel, WitnessTraceCommitmentError> {
    compact_witness_stage_leaf_hash_level_from_source_device_view_with_workspace_cache_timing(
        row_count,
        column_count,
        source_bits,
        target_bits,
        arity,
        source_device,
        None,
        timing,
    )
}

#[cfg(feature = "cuda")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn compact_witness_stage_leaf_hash_level_from_source_device_view_with_workspace_cache_timing(
    row_count: usize,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_device: &WitnessStageSourceDeviceView,
    workspace_cache: Option<&mut WitnessStageLeafWorkspaceCache>,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<PendingCanonicalCudaDigestLevel, WitnessTraceCommitmentError> {
    if !source_device.has_matching_shape(row_count, column_count) {
        return Err(WitnessStageLeafError::LengthOverflow.into());
    }
    let required_source_bytes = source_device
        .required_byte_len()
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    if source_device.buffer().len() < required_source_bytes {
        return Err(WitnessStageLeafError::Accel(AccelError::LengthMismatch {
            lhs: source_device.buffer().len(),
            rhs: required_source_bytes,
        })
        .into());
    }
    compact_witness_stage_leaf_hash_level_from_source_device_timed(
        source_device.buffer(),
        CudaRowMajorColumnView {
            source_rows: row_count,
            source_row_stride: source_device.row_stride(),
            column_offset: source_device.column_offset(),
            column_count,
        },
        source_bits,
        target_bits,
        arity,
        workspace_cache,
        timing,
    )
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
    extend_witness_stage_row_major_bytes_with_source_device(
        values,
        column_count,
        source_bits,
        target_bits,
        None,
    )
}

#[cfg(feature = "cuda")]
fn extend_witness_stage_row_major_bytes_with_source_device(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    source_device: Option<&CudaDeviceBuffer>,
) -> Result<Vec<u8>, WitnessStageLeafError> {
    let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
        values.len(),
        column_count,
        source_bits,
        target_bits,
    )
    .map_err(WitnessStageLeafError::from)?;
    prepare_gpu_setup(target_bits).map_err(WitnessStageLeafError::from)?;

    let source_buffer_storage;
    let source_buffer = if let Some(source_device) = source_device {
        validate_source_device_buffer(source_device, values)?;
        source_device
    } else {
        source_buffer_storage = CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(values))
            .map_err(WitnessStageLeafError::from)?;
        &source_buffer_storage
    };
    let mut output_buffer =
        CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from)?;

    cuda_goldilocks_coset_extend_row_major_columns_device(
        source_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
    )
    .map_err(WitnessStageLeafError::from)?;
    validate_row_major_device_words(&output_buffer, out_byte_count)?;
    let bytes = output_buffer
        .to_vec()
        .map_err(WitnessStageLeafError::from)?;
    Ok(bytes)
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
fn compact_witness_stage_leaf_hashes_timed(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_device: Option<&CudaDeviceBuffer>,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<Vec<[Felt; HASH_WORDS]>, WitnessTraceCommitmentError> {
    if column_count <= HASH_WORDS {
        return Err(WitnessStageCommitmentError::LengthOverflow.into());
    }
    let out_byte_count = record_setup_duration(
        &mut timing.setup_duration,
        &mut timing.leaf_setup_prepare_duration,
        || {
            let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
                values.len(),
                column_count,
                source_bits,
                target_bits,
            )
            .map_err(WitnessStageLeafError::from)?;
            prepare_gpu_setup(target_bits).map_err(WitnessStageLeafError::from)?;
            Ok::<_, WitnessStageLeafError>(out_byte_count)
        },
    )?;

    let source_buffer_storage;
    let source_buffer = if let Some(source_device) = source_device {
        validate_source_device_buffer(source_device, values)?;
        source_device
    } else {
        source_buffer_storage = record_duration(&mut timing.upload_duration, || {
            CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(values))
                .map_err(WitnessStageLeafError::from)
        })?;
        &source_buffer_storage
    };
    let mut output_buffer = record_setup_duration(
        &mut timing.setup_duration,
        &mut timing.leaf_setup_output_alloc_duration,
        || CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from),
    )?;
    timing.record_output_alloc(out_byte_count);
    let mut extension_workspace = record_setup_duration(
        &mut timing.setup_duration,
        &mut timing.leaf_setup_workspace_alloc_duration,
        || CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from),
    )?;
    timing.record_workspace_alloc(out_byte_count);

    record_duration(&mut timing.kernel_duration, || {
        cuda_goldilocks_coset_extend_row_major_columns_device_unsynced(
            source_buffer,
            &mut output_buffer,
            &mut extension_workspace,
            column_count,
            source_bits,
            target_bits,
        )
        .map_err(WitnessStageLeafError::from)
    })?;
    timing.record_coset_extend_work(out_byte_count, column_count, source_bits, target_bits);
    let extended_rows = extended_row_count_from_bytes(out_byte_count, column_count)?;
    record_duration(&mut timing.validate_duration, || {
        validate_row_major_device_words(&output_buffer, out_byte_count)
    })?;
    let leaf_hashes = record_duration(&mut timing.leaf_hash_duration, || {
        linear_hashes_from_validated_wide_row_major_device_buffer(
            &output_buffer,
            extended_rows,
            column_count,
            arity,
        )
        .map_err(WitnessStageCommitmentError::from)
    })?;
    timing.record_leaf_hash_work(extended_rows, out_byte_count, arity);
    Ok(leaf_hashes)
}

#[cfg(feature = "cuda")]
fn compact_witness_stage_leaf_hash_level_timed(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_device: Option<&CudaDeviceBuffer>,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<PendingCanonicalCudaDigestLevel, WitnessTraceCommitmentError> {
    let out_byte_count = record_setup_duration(
        &mut timing.setup_duration,
        &mut timing.leaf_setup_prepare_duration,
        || {
            let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
                values.len(),
                column_count,
                source_bits,
                target_bits,
            )
            .map_err(WitnessStageLeafError::from)?;
            prepare_gpu_setup(target_bits).map_err(WitnessStageLeafError::from)?;
            Ok::<_, WitnessStageLeafError>(out_byte_count)
        },
    )?;

    let source_buffer_storage;
    let source_buffer = if let Some(source_device) = source_device {
        validate_source_device_buffer(source_device, values)?;
        source_device
    } else {
        source_buffer_storage = record_duration(&mut timing.upload_duration, || {
            CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(values))
                .map_err(WitnessStageLeafError::from)
        })?;
        &source_buffer_storage
    };
    let mut output_buffer = record_setup_duration(
        &mut timing.setup_duration,
        &mut timing.leaf_setup_output_alloc_duration,
        || CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from),
    )?;
    timing.record_output_alloc(out_byte_count);

    record_duration(&mut timing.kernel_duration, || {
        cuda_goldilocks_coset_extend_row_major_columns_device(
            source_buffer,
            &mut output_buffer,
            column_count,
            source_bits,
            target_bits,
        )
        .map_err(WitnessStageLeafError::from)
    })?;
    timing.record_coset_extend_work(out_byte_count, column_count, source_bits, target_bits);
    let extended_rows = extended_row_count_from_bytes(out_byte_count, column_count)?;
    let canonical_check = record_duration(&mut timing.validate_duration, || {
        begin_validate_row_major_device_words(&output_buffer, out_byte_count)
    })?;
    let level = record_duration(&mut timing.leaf_hash_duration, || {
        linear_hash_level_from_validated_row_major_device_buffer(
            &output_buffer,
            extended_rows,
            column_count,
            arity,
        )
        .map_err(WitnessStageCommitmentError::from)
    })?;
    timing.record_leaf_hash_work(extended_rows, out_byte_count, arity);
    Ok(PendingCanonicalCudaDigestLevel::new(level, canonical_check))
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
fn compact_witness_stage_leaf_hash_level_from_source_device_timed(
    source_device: &CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    workspace_cache: Option<&mut WitnessStageLeafWorkspaceCache>,
    timing: &mut WitnessStageLeafExtendTiming,
) -> Result<PendingCanonicalCudaDigestLevel, WitnessTraceCommitmentError> {
    let out_byte_count = record_setup_duration(
        &mut timing.setup_duration,
        &mut timing.leaf_setup_prepare_duration,
        || {
            let value_count = view
                .source_rows
                .checked_mul(view.column_count)
                .ok_or(WitnessStageLeafError::LengthOverflow)?;
            let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
                value_count,
                view.column_count,
                source_bits,
                target_bits,
            )
            .map_err(WitnessStageLeafError::from)?;
            prepare_gpu_setup(target_bits).map_err(WitnessStageLeafError::from)?;
            Ok::<_, WitnessStageLeafError>(out_byte_count)
        },
    )?;

    let mut output_buffer = record_setup_duration(
        &mut timing.setup_duration,
        &mut timing.leaf_setup_output_alloc_duration,
        || CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from),
    )?;
    timing.record_output_alloc(out_byte_count);
    let mut extension_workspace_storage;
    let extension_workspace = if let Some(workspace_cache) = workspace_cache {
        workspace_cache.workspace(out_byte_count, timing)?
    } else {
        extension_workspace_storage = record_setup_duration(
            &mut timing.setup_duration,
            &mut timing.leaf_setup_workspace_alloc_duration,
            || CudaDeviceBuffer::new(out_byte_count).map_err(WitnessStageLeafError::from),
        )?;
        timing.record_workspace_alloc(out_byte_count);
        &mut extension_workspace_storage
    };

    record_duration(&mut timing.kernel_duration, || {
        if view.source_row_stride == view.column_count && view.column_offset == 0 {
            cuda_goldilocks_coset_extend_row_major_columns_device_unsynced(
                source_device,
                &mut output_buffer,
                extension_workspace,
                view.column_count,
                source_bits,
                target_bits,
            )
        } else {
            cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced(
                source_device,
                &mut output_buffer,
                extension_workspace,
                view,
                source_bits,
                target_bits,
            )
        }
        .map_err(WitnessStageLeafError::from)
    })?;
    timing.record_coset_extend_work(out_byte_count, view.column_count, source_bits, target_bits);
    let extended_rows = extended_row_count_from_bytes(out_byte_count, view.column_count)?;
    let canonical_check = record_duration(&mut timing.validate_duration, || {
        begin_validate_row_major_device_words(&output_buffer, out_byte_count)
    })?;
    let level = record_duration(&mut timing.leaf_hash_duration, || {
        linear_hash_level_from_validated_row_major_device_buffer(
            &output_buffer,
            extended_rows,
            view.column_count,
            arity,
        )
        .map_err(WitnessStageCommitmentError::from)
    })?;
    timing.record_leaf_hash_work(extended_rows, out_byte_count, arity);
    Ok(PendingCanonicalCudaDigestLevel::new(level, canonical_check))
}

#[cfg(feature = "cuda")]
fn validate_source_device_buffer(
    buffer: &CudaDeviceBuffer,
    values: &[Felt],
) -> Result<(), WitnessStageLeafError> {
    let expected = values
        .len()
        .checked_mul(WORD_BYTES)
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    if buffer.len() != expected {
        return Err(WitnessStageLeafError::Accel(AccelError::LengthMismatch {
            lhs: buffer.len(),
            rhs: expected,
        }));
    }
    Ok(())
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

#[cfg(feature = "cuda")]
fn record_setup_duration<T, E>(
    total: &mut Duration,
    part: &mut Duration,
    run: impl FnOnce() -> Result<T, E>,
) -> Result<T, WitnessTraceCommitmentError>
where
    WitnessTraceCommitmentError: From<E>,
{
    let started = Instant::now();
    let result = run().map_err(WitnessTraceCommitmentError::from)?;
    let elapsed = started.elapsed();
    *total += elapsed;
    *part += elapsed;
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
fn validate_row_major_device_words(
    buffer: &CudaDeviceBuffer,
    byte_count: usize,
) -> Result<(), WitnessStageLeafError> {
    if !byte_count.is_multiple_of(WORD_BYTES) || buffer.len() != byte_count {
        return Err(WitnessStageLeafError::LengthOverflow);
    }
    let word_count = byte_count / WORD_BYTES;
    if cuda_goldilocks_validate_canonical_words_device(buffer, word_count)
        .map_err(WitnessStageLeafError::from)?
    {
        Ok(())
    } else {
        Err(WitnessStageLeafError::NonCanonicalDeviceWord)
    }
}

#[cfg(feature = "cuda")]
fn begin_validate_row_major_device_words(
    buffer: &CudaDeviceBuffer,
    byte_count: usize,
) -> Result<CudaCanonicalCheck, WitnessStageLeafError> {
    if !byte_count.is_multiple_of(WORD_BYTES) || buffer.len() != byte_count {
        return Err(WitnessStageLeafError::LengthOverflow);
    }
    let word_count = byte_count / WORD_BYTES;
    cuda_goldilocks_begin_validate_canonical_words_device(buffer, word_count)
        .map_err(WitnessStageLeafError::from)
}

#[cfg(all(test, feature = "cuda"))]
fn validate_row_major_word_bytes_and_padded_hashes(
    bytes: &[u8],
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, WitnessTraceCommitmentError> {
    if column_count > HASH_WORDS {
        return Err(WitnessStageLeafError::LengthOverflow.into());
    }
    let expected = row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageLeafError::LengthOverflow)?;
    if bytes.len() != expected {
        return Err(WitnessStageLeafError::LengthOverflow.into());
    }

    let mut out = Vec::with_capacity(row_count);
    for row in 0..row_count {
        let mut digest = [Felt::ZERO; HASH_WORDS];
        for (column, slot) in digest.iter_mut().enumerate().take(column_count) {
            let word_index = row
                .checked_mul(column_count)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(WitnessStageLeafError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(WORD_BYTES)
                .ok_or(WitnessStageLeafError::LengthOverflow)?;
            let word = u64::from_le_bytes(
                bytes[byte_index..byte_index + WORD_BYTES]
                    .try_into()
                    .expect("row-major byte length checked"),
            );
            *slot = Felt::from_canonical(word).map_err(WitnessStageLeafError::from)?;
        }
        out.push(digest);
    }
    validate_leaf_hash_arity(arity)?;
    Ok(out)
}

#[cfg(all(test, feature = "cuda"))]
fn validate_leaf_hash_arity(arity: usize) -> Result<(), WitnessStageCommitmentError> {
    match arity {
        2 | 4 => Ok(()),
        _ => Err(WitnessStageCommitmentError::UnsupportedArity { arity }),
    }
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::{validate_row_major_device_words, validate_row_major_word_bytes_and_padded_hashes};
    use crate::witness_commitment::WitnessStageLeafError;
    use lzvm_accel::CudaDeviceBuffer;
    use lzvm_field::{Felt, MODULUS};

    fn encode_words(words: &[u64]) -> Vec<u8> {
        let mut out = Vec::with_capacity(words.len() * 8);
        for word in words {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out
    }

    #[test]
    fn narrow_leaf_validation_builds_padded_digests() {
        let bytes = encode_words(&[1, 2, 3, 4, 5, 6]);

        let digests = validate_row_major_word_bytes_and_padded_hashes(&bytes, 2, 3, 2)
            .expect("narrow digests should validate");

        assert_eq!(
            digests,
            vec![
                [
                    Felt::from_u64(1),
                    Felt::from_u64(2),
                    Felt::from_u64(3),
                    Felt::ZERO
                ],
                [
                    Felt::from_u64(4),
                    Felt::from_u64(5),
                    Felt::from_u64(6),
                    Felt::ZERO
                ],
            ]
        );
    }

    #[test]
    fn narrow_leaf_validation_rejects_noncanonical_words() {
        let bytes = encode_words(&[1, MODULUS]);

        let error = validate_row_major_word_bytes_and_padded_hashes(&bytes, 1, 2, 2)
            .expect_err("non-canonical words should be rejected");

        assert!(error.to_string().contains("non-canonical field element"));
    }

    #[test]
    fn device_word_validation_rejects_noncanonical_words() {
        let buffer = CudaDeviceBuffer::from_u64_words(&[1, MODULUS]).expect("buffer should upload");

        let error = validate_row_major_device_words(&buffer, 2 * super::WORD_BYTES)
            .expect_err("non-canonical device words should be rejected");

        assert_eq!(error, WitnessStageLeafError::NonCanonicalDeviceWord);
    }
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

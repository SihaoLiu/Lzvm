use std::fmt;
#[cfg(feature = "cuda")]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(feature = "cuda")]
use std::time::Instant;

#[cfg(all(test, feature = "cuda"))]
static CUDA_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(feature = "cuda")]
mod cuda_allocator;
#[cfg(feature = "cuda")]
mod cuda_buffer;
#[cfg(feature = "cuda")]
mod cuda_canonical;
#[cfg(feature = "cuda")]
mod cuda_copy_sites;
#[cfg(feature = "cuda")]
mod cuda_device;
#[cfg(feature = "cuda")]
mod cuda_graph_extension;
#[cfg(feature = "cuda")]
mod cuda_regular_constraints;
#[cfg(feature = "cuda")]
mod cuda_row_selected;
#[cfg(feature = "cuda")]
mod cuda_setup;
#[cfg(feature = "cuda")]
mod cuda_stream;
#[cfg(feature = "cuda")]
pub use cuda_allocator::{cuda_allocator_clear_cache, cuda_allocator_stats, CudaAllocatorStats};
#[cfg(feature = "cuda")]
pub use cuda_buffer::{CudaDeviceBuffer, CudaPinnedHostBuffer, MainTraceDeviceLayout};
#[cfg(feature = "cuda")]
pub use cuda_canonical::{
    cuda_goldilocks_begin_validate_canonical_words_device,
    cuda_goldilocks_begin_validate_canonical_words_device_on_stream,
    cuda_goldilocks_validate_canonical_words_device, CudaCanonicalCheck,
};
#[cfg(feature = "cuda")]
pub use cuda_copy_sites::{
    cuda_copy_site_stats_clear, cuda_copy_site_stats_snapshot, CudaCopyDirection, CudaCopySiteStat,
};
#[cfg(feature = "cuda")]
pub use cuda_device::{cuda_device_synchronize, cuda_memory_info, CudaMemoryInfo};
#[cfg(feature = "cuda")]
pub use cuda_graph_extension::{
    CudaRowMajorCosetExtensionGraphRunner, CudaStridedRowMajorCosetExtensionGraphRunner,
};
#[cfg(feature = "cuda")]
pub use cuda_regular_constraints::{
    cuda_regular_constraints_base, CudaRegularConstraintEntry, CudaRegularConstraintInputs,
    CudaRegularConstraintResult, CudaRegularStage,
};
#[cfg(feature = "cuda")]
pub use cuda_row_selected::{
    cuda_goldilocks_coset_extend_main_trace_compact_descriptors_shifted_rows_device,
    cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device,
};
#[cfg(feature = "cuda")]
pub use cuda_setup::{cuda_setup_init, cuda_setup_stats, CudaSetupStats};
#[cfg(feature = "cuda")]
pub use cuda_stream::{CudaEvent, CudaGraph, CudaGraphCapture, CudaGraphExec, CudaStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccelError {
    LengthMismatch { lhs: usize, rhs: usize },
    InvalidDomain { bits: usize, len: usize },
    CudaUnavailable,
    Cuda { code: i32 },
}

const CUDA_ERROR_OUT_OF_MEMORY: i32 = 2;

impl fmt::Display for AccelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { lhs, rhs } => {
                write!(f, "length mismatch: lhs {lhs}, rhs {rhs}")
            }
            Self::InvalidDomain { bits, len } => {
                write!(f, "invalid field domain: bits {bits}, len {len}")
            }
            Self::CudaUnavailable => write!(f, "cuda backend is not enabled"),
            Self::Cuda { code } if *code == CUDA_ERROR_OUT_OF_MEMORY => {
                write!(f, "cuda backend out of memory: error code {code}")
            }
            Self::Cuda { code } if *code < 0 => write!(f, "invalid cuda input: {code}"),
            Self::Cuda { code } => write!(f, "cuda backend error: {code}"),
        }
    }
}

impl std::error::Error for AccelError {}

#[cfg(feature = "cuda")]
unsafe extern "C" {
    fn lzvm_cuda_current_device(out: *mut i32) -> i32;
    fn lzvm_cuda_goldilocks_add(lhs: *const u64, rhs: *const u64, out: *mut u64, len: usize)
        -> i32;
    fn lzvm_cuda_goldilocks_mul(lhs: *const u64, rhs: *const u64, out: *mut u64, len: usize)
        -> i32;
    fn lzvm_cuda_goldilocks_butterfly(
        even: *const u64,
        odd: *const u64,
        twiddle: *const u64,
        out_even: *mut u64,
        out_odd: *mut u64,
        len: usize,
    ) -> i32;
    fn lzvm_cuda_goldilocks_ntt(
        values: *const u64,
        out: *mut u64,
        len: usize,
        bits: usize,
        root: u64,
    ) -> i32;
    fn lzvm_cuda_goldilocks_intt(
        values: *const u64,
        out: *mut u64,
        len: usize,
        bits: usize,
        root: u64,
    ) -> i32;
    fn lzvm_cuda_goldilocks_coset_extend(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_raw(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_unsynced"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_unsynced_raw(
        values: *const u64,
        out: *mut u64,
        workspace: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_on_stream"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_on_stream_raw(
        values: *const u64,
        out: *mut u64,
        workspace: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_raw(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        source_row_stride: usize,
        column_offset: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced_raw(
        values: *const u64,
        out: *mut u64,
        workspace: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        source_row_stride: usize,
        column_offset: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream_raw(
        values: *const u64,
        out: *mut u64,
        workspace: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        source_row_stride: usize,
        column_offset: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_row_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_row_device_raw(
        values: *const u64,
        weights: *const u64,
        out: *mut u64,
        source_len: usize,
        column_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_shifted_row_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_shifted_row_device_raw(
        values: *const u64,
        weights: *const u64,
        out: *mut u64,
        source_len: usize,
        column_count: usize,
        weight_shift: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_shifted_rows_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_shifted_rows_device_raw(
        values: *const u64,
        weights: *const u64,
        weight_shifts: *const u64,
        output_rows: *const u64,
        out: *mut u64,
        source_len: usize,
        column_count: usize,
        target_row_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_rows_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_rows_device_raw(
        values: *const u64,
        weights: *const u64,
        out: *mut u64,
        source_len: usize,
        column_count: usize,
        target_row_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_row_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_row_device_raw(
        values: *const u64,
        weights: *const u64,
        out: *mut u64,
        source_len: usize,
        source_row_stride: usize,
        column_offset: usize,
        column_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_row_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_row_device_raw(
        values: *const u64,
        weights: *const u64,
        out: *mut u64,
        source_len: usize,
        source_row_stride: usize,
        column_offset: usize,
        column_count: usize,
        weight_shift: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_rows_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_rows_device_raw(
        values: *const u64,
        weights: *const u64,
        weight_shifts: *const u64,
        output_rows: *const u64,
        out: *mut u64,
        source_len: usize,
        source_row_stride: usize,
        column_offset: usize,
        column_count: usize,
        target_row_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device_raw(
        values: *const u64,
        weights: *const u64,
        out: *mut u64,
        source_len: usize,
        source_row_stride: usize,
        column_offset: usize,
        column_count: usize,
        target_row_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_device"]
    fn lzvm_cuda_goldilocks_coset_extend_device_raw(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    fn lzvm_cuda_poseidon2_width4(values: *const u64, out: *mut u64, state_count: usize) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width4_device"]
    fn lzvm_cuda_poseidon2_width4_device_raw(
        values: *const u64,
        out: *mut u64,
        state_count: usize,
    ) -> i32;
    fn lzvm_cuda_poseidon2_width4_find_nonce(
        challenge: *const u64,
        start: u64,
        count: usize,
        target: u64,
        out: *mut u64,
        found: *mut u32,
    ) -> i32;
    fn lzvm_cuda_poseidon2_width8(values: *const u64, out: *mut u64, state_count: usize) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_device"]
    fn lzvm_cuda_poseidon2_width8_device_raw(
        values: *const u64,
        out: *mut u64,
        state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_parent_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_parent_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_root_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_root_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_opening_path_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_opening_path_device_raw(
        values: *const u64,
        root_out: *mut u64,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_index: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_digest_root_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_digest_root_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_digest_parent_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_digest_parent_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_digest_selected_parent_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_digest_selected_parent_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
        parent_index: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_digest_opening_path_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_digest_opening_path_device_raw(
        values: *const u64,
        root_out: *mut u64,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_index: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_device_raw(
        values: *const u64,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_index: usize,
        prefix_level_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_raw(
        values: *const u64,
        query_indices: *const usize,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_count: usize,
        prefix_level_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_to_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_to_device_raw(
        values: *const u64,
        query_indices: *const usize,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_count: usize,
        prefix_level_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_digest_opening_suffixes_batch_to_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_digest_opening_suffixes_batch_to_device_raw(
        values: *const *const u64,
        child_state_counts: *const usize,
        query_indices: *const usize,
        siblings_out: *const *mut u64,
        group_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_device"]
    fn lzvm_cuda_poseidon2_width8_linear_round_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_row_major_device"]
    fn lzvm_cuda_poseidon2_width8_linear_round_row_major_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_row_major_digest_device"]
    fn lzvm_cuda_poseidon2_width8_linear_round_row_major_digest_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_row_major_digest_device_on_stream"]
    fn lzvm_cuda_poseidon2_width8_linear_round_row_major_digest_device_on_stream_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_device"]
    fn lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_device_raw(
        current_states: *const u64,
        column_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_checked_device"]
    fn lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_checked_device_raw(
        current_states: *const u64,
        column_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
        noncanonical_found: *mut u32,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_device_on_stream"]
    fn lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_device_on_stream_raw(
        current_states: *const u64,
        column_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    fn lzvm_cuda_poseidon2_width16(values: *const u64, out: *mut u64, state_count: usize) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_device"]
    fn lzvm_cuda_poseidon2_width16_device_raw(
        values: *const u64,
        out: *mut u64,
        state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_parent_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_parent_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_root_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_root_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_opening_path_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_opening_path_device_raw(
        values: *const u64,
        root_out: *mut u64,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_index: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_digest_root_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_digest_root_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_digest_parent_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_digest_parent_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_digest_selected_parent_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_digest_selected_parent_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
        parent_index: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_digest_opening_path_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_digest_opening_path_device_raw(
        values: *const u64,
        root_out: *mut u64,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_index: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_device_raw(
        values: *const u64,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_index: usize,
        prefix_level_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_raw(
        values: *const u64,
        query_indices: *const usize,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_count: usize,
        prefix_level_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_to_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_to_device_raw(
        values: *const u64,
        query_indices: *const usize,
        siblings_out: *mut u64,
        child_state_count: usize,
        query_count: usize,
        prefix_level_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_digest_opening_suffixes_batch_to_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_digest_opening_suffixes_batch_to_device_raw(
        values: *const *const u64,
        child_state_counts: *const usize,
        query_indices: *const usize,
        siblings_out: *const *mut u64,
        group_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_device"]
    fn lzvm_cuda_poseidon2_width16_linear_round_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_row_major_device"]
    fn lzvm_cuda_poseidon2_width16_linear_round_row_major_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_row_major_digest_device"]
    fn lzvm_cuda_poseidon2_width16_linear_round_row_major_digest_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_row_major_digest_device_on_stream"]
    fn lzvm_cuda_poseidon2_width16_linear_round_row_major_digest_device_on_stream_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_device"]
    fn lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_device_raw(
        current_states: *const u64,
        column_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_checked_device"]
    fn lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_checked_device_raw(
        current_states: *const u64,
        column_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
        noncanonical_found: *mut u32,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_device_on_stream"]
    fn lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_device_on_stream_raw(
        current_states: *const u64,
        column_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    fn lzvm_cuda_keccak256_fixed(
        input: *const u8,
        message_len: usize,
        out: *mut u8,
        message_count: usize,
    ) -> i32;
}

#[cfg(feature = "cuda")]
const ROOTS_OF_UNITY: [u64; 33] = [
    1,
    18_446_744_069_414_584_320,
    281_474_976_710_656,
    16_777_216,
    4096,
    64,
    8,
    2_198_989_700_608,
    4_404_853_092_538_523_347,
    6_434_636_298_004_421_797,
    4_255_134_452_441_852_017,
    9_113_133_275_150_391_358,
    4_355_325_209_153_869_931,
    4_308_460_244_895_131_701,
    7_126_024_226_993_609_386,
    1_873_558_160_482_552_414,
    8_167_150_655_112_846_419,
    5_718_075_921_287_398_682,
    3_411_401_055_030_829_696,
    8_982_441_859_486_529_725,
    1_971_462_654_193_939_361,
    6_553_637_399_136_210_105,
    8_124_823_329_697_072_476,
    5_936_499_541_590_631_774,
    2_709_866_199_236_980_323,
    8_877_499_657_461_974_390,
    3_757_607_247_483_852_735,
    4_969_973_714_567_017_225,
    2_147_253_751_702_802_259,
    2_530_564_950_562_219_707,
    1_905_180_297_017_055_339,
    3_524_815_499_551_269_279,
    7_277_203_076_849_721_926,
];

#[cfg(feature = "cuda")]
const SHIFT: u64 = 7;
#[cfg(feature = "cuda")]
const GOLDILOCKS_MODULUS: u64 = 0xffff_ffff_0000_0001;

#[cfg(feature = "cuda")]
fn sub_mod(lhs: u64, rhs: u64) -> u64 {
    if lhs >= rhs {
        lhs - rhs
    } else {
        GOLDILOCKS_MODULUS - (rhs - lhs)
    }
}

#[cfg(feature = "cuda")]
fn mul_mod(lhs: u64, rhs: u64) -> u64 {
    ((lhs as u128 * rhs as u128) % GOLDILOCKS_MODULUS as u128) as u64
}

#[cfg(feature = "cuda")]
fn pow_mod(mut base: u64, mut exponent: u64) -> u64 {
    let mut result = 1_u64;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = mul_mod(result, base);
        }
        base = mul_mod(base, base);
        exponent >>= 1;
    }
    result
}

#[cfg(feature = "cuda")]
pub(crate) fn ensure_cuda_setup(max_bits_ext: usize) -> Result<(), AccelError> {
    cuda_setup_init(max_bits_ext)
}

#[cfg(feature = "cuda")]
pub(crate) fn cuda_status(code: i32) -> Result<(), AccelError> {
    if code == 0 {
        Ok(())
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
fn u64_word_byte_len(word_count: usize) -> Result<usize, AccelError> {
    word_count.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits: 64,
        len: word_count,
    })
}

#[cfg(feature = "cuda")]
pub(crate) fn coset_extend_domain(
    len: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<(usize, usize, u64, u64), AccelError> {
    if target_bits < source_bits {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len,
        });
    }
    let Some(source_root) = ROOTS_OF_UNITY.get(source_bits).copied() else {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len,
        });
    };
    let Some(target_root) = ROOTS_OF_UNITY.get(target_bits).copied() else {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len,
        });
    };
    let source_len = 1_usize
        .checked_shl(
            u32::try_from(source_bits).map_err(|_| AccelError::InvalidDomain {
                bits: source_bits,
                len,
            })?,
        )
        .ok_or(AccelError::InvalidDomain {
            bits: source_bits,
            len,
        })?;
    let target_len = 1_usize
        .checked_shl(
            u32::try_from(target_bits).map_err(|_| AccelError::InvalidDomain {
                bits: target_bits,
                len,
            })?,
        )
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len,
        })?;

    Ok((source_len, target_len, source_root, target_root))
}

#[cfg(feature = "cuda")]
pub(crate) fn coset_extend_row_weights(
    source_len: usize,
    target_len: usize,
    source_root: u64,
    target_root: u64,
    target_bits: usize,
    target_row: usize,
) -> Result<Vec<u64>, AccelError> {
    if target_row >= target_len {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row,
        });
    }
    let source_len_u64 = u64::try_from(source_len).map_err(|_| AccelError::InvalidDomain {
        bits: target_bits,
        len: source_len,
    })?;
    let target_row_u64 = u64::try_from(target_row).map_err(|_| AccelError::InvalidDomain {
        bits: target_bits,
        len: target_row,
    })?;
    let inv_source_len = pow_mod(source_len_u64, GOLDILOCKS_MODULUS - 2);
    let source_root_inverse = pow_mod(source_root, GOLDILOCKS_MODULUS - 2);
    let target_point = mul_mod(SHIFT, pow_mod(target_root, target_row_u64));
    let numerator = sub_mod(pow_mod(target_point, source_len_u64), 1);

    let mut denominators = Vec::with_capacity(source_len);
    let mut prefix_products = Vec::with_capacity(source_len);
    let mut denominator_product = 1_u64;
    let mut source_inverse_power = 1_u64;
    for _ in 0..source_len {
        let denominator = sub_mod(mul_mod(target_point, source_inverse_power), 1);
        denominators.push(denominator);
        prefix_products.push(denominator_product);
        if denominator != 0 {
            denominator_product = mul_mod(denominator_product, denominator);
        }
        source_inverse_power = mul_mod(source_inverse_power, source_root_inverse);
    }

    let mut inverse_product = pow_mod(denominator_product, GOLDILOCKS_MODULUS - 2);
    let mut weights = vec![1_u64; source_len];
    for index in (0..source_len).rev() {
        let denominator = denominators[index];
        if denominator == 0 {
            continue;
        }
        let inverse_denominator = mul_mod(inverse_product, prefix_products[index]);
        weights[index] = mul_mod(mul_mod(numerator, inverse_denominator), inv_source_len);
        inverse_product = mul_mod(inverse_product, denominator);
    }
    Ok(weights)
}

#[cfg(feature = "cuda")]
fn row_weight_shift_for_target_row(
    source_bits: usize,
    target_bits: usize,
    target_row: usize,
) -> Result<(usize, usize), AccelError> {
    if target_bits < source_bits {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row,
        });
    }
    let blowup_bits = target_bits - source_bits;
    let source_len = 1_usize
        .checked_shl(
            u32::try_from(source_bits).map_err(|_| AccelError::InvalidDomain {
                bits: source_bits,
                len: target_row,
            })?,
        )
        .ok_or(AccelError::InvalidDomain {
            bits: source_bits,
            len: target_row,
        })?;
    let blowup = 1_usize
        .checked_shl(
            u32::try_from(blowup_bits).map_err(|_| AccelError::InvalidDomain {
                bits: target_bits,
                len: target_row,
            })?,
        )
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row,
        })?;
    let target_len = source_len
        .checked_mul(blowup)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row,
        })?;
    if target_row >= target_len {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row,
        });
    }
    let residue_row = target_row % blowup;
    let source_row = target_row >> blowup_bits;
    let weight_shift = (source_len - source_row) % source_len;
    Ok((residue_row, weight_shift))
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RowWeightCacheKey {
    device: i32,
    source_len: usize,
    target_len: usize,
    source_root: u64,
    target_root: u64,
    source_bits: usize,
    target_bits: usize,
    residue_row: usize,
}

#[cfg(feature = "cuda")]
struct RowWeightCacheEntry {
    key: RowWeightCacheKey,
    weights: Arc<CudaDeviceBuffer>,
}

#[cfg(feature = "cuda")]
static ROW_WEIGHT_CACHE: OnceLock<Mutex<Vec<RowWeightCacheEntry>>> = OnceLock::new();

#[cfg(feature = "cuda")]
const ROW_WEIGHT_CACHE_MAX_ENTRIES: usize = 64;

#[cfg(feature = "cuda")]
fn row_weight_cache() -> &'static Mutex<Vec<RowWeightCacheEntry>> {
    ROW_WEIGHT_CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "cuda")]
fn current_cuda_device() -> Result<i32, AccelError> {
    let started = Instant::now();
    let mut device = 0_i32;
    let code = unsafe { lzvm_cuda_current_device(&mut device) };
    cuda_setup::record_cuda_current_device_duration(started.elapsed());
    cuda_status(code)?;
    Ok(device)
}

#[cfg(feature = "cuda")]
fn cached_coset_extend_residue_row_weights_device(
    source_len: usize,
    target_len: usize,
    source_root: u64,
    target_root: u64,
    source_bits: usize,
    target_bits: usize,
    target_row: usize,
) -> Result<(Arc<CudaDeviceBuffer>, usize), AccelError> {
    if target_bits < source_bits {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row,
        });
    }
    let device = current_cuda_device()?;
    let (residue_row, weight_shift) =
        row_weight_shift_for_target_row(source_bits, target_bits, target_row)?;
    let key = RowWeightCacheKey {
        device,
        source_len,
        target_len,
        source_root,
        target_root,
        source_bits,
        target_bits,
        residue_row,
    };
    if let Ok(cache) = row_weight_cache().lock() {
        if let Some(entry) = cache.iter().find(|entry| entry.key == key) {
            return Ok((Arc::clone(&entry.weights), weight_shift));
        }
    }
    let weights = coset_extend_row_weights(
        source_len,
        target_len,
        source_root,
        target_root,
        target_bits,
        residue_row,
    )?;
    let weights = Arc::new(CudaDeviceBuffer::from_u64_words(&weights)?);
    if let Ok(mut cache) = row_weight_cache().lock() {
        if let Some(entry) = cache.iter().find(|entry| entry.key == key) {
            return Ok((Arc::clone(&entry.weights), weight_shift));
        }
        if cache.len() >= ROW_WEIGHT_CACHE_MAX_ENTRIES {
            cache.clear();
        }
        cache.push(RowWeightCacheEntry {
            key,
            weights: Arc::clone(&weights),
        });
    }
    Ok((weights, weight_shift))
}

#[cfg(feature = "cuda")]
struct ShiftedRowWeightGroup {
    residue_row: usize,
    output_rows: Vec<u64>,
    weight_shifts: Vec<u64>,
}

#[cfg(feature = "cuda")]
fn shifted_row_weight_groups(
    source_bits: usize,
    target_bits: usize,
    target_rows: &[usize],
) -> Result<Vec<ShiftedRowWeightGroup>, AccelError> {
    let mut groups: Vec<ShiftedRowWeightGroup> = Vec::new();
    for (output_row, &target_row) in target_rows.iter().enumerate() {
        let (residue_row, weight_shift) =
            row_weight_shift_for_target_row(source_bits, target_bits, target_row)?;
        let output_row = u64::try_from(output_row).map_err(|_| AccelError::InvalidDomain {
            bits: target_bits,
            len: target_rows.len(),
        })?;
        let weight_shift = u64::try_from(weight_shift).map_err(|_| AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row,
        })?;
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.residue_row == residue_row)
        {
            group.output_rows.push(output_row);
            group.weight_shifts.push(weight_shift);
        } else {
            groups.push(ShiftedRowWeightGroup {
                residue_row,
                output_rows: vec![output_row],
                weight_shifts: vec![weight_shift],
            });
        }
    }
    Ok(groups)
}

#[cfg(feature = "cuda")]
fn coset_extend_row_range_weights(
    source_len: usize,
    target_len: usize,
    source_root: u64,
    target_root: u64,
    target_bits: usize,
    target_row_start: usize,
    target_row_count: usize,
) -> Result<Vec<u64>, AccelError> {
    let target_row_end =
        target_row_start
            .checked_add(target_row_count)
            .ok_or(AccelError::InvalidDomain {
                bits: target_bits,
                len: target_row_start,
            })?;
    if target_row_end > target_len {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row_end,
        });
    }
    let weight_count =
        source_len
            .checked_mul(target_row_count)
            .ok_or(AccelError::InvalidDomain {
                bits: target_bits,
                len: target_row_count,
            })?;
    let mut weights = Vec::with_capacity(weight_count);
    for target_row in target_row_start..target_row_end {
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

#[cfg(feature = "cuda")]
type CudaBinaryOp = unsafe extern "C" fn(*const u64, *const u64, *mut u64, usize) -> i32;

#[cfg(feature = "cuda")]
fn run_cuda_binary_op(
    lhs: &[u64],
    rhs: &[u64],
    operation: CudaBinaryOp,
) -> Result<Vec<u64>, AccelError> {
    if lhs.len() != rhs.len() {
        return Err(AccelError::LengthMismatch {
            lhs: lhs.len(),
            rhs: rhs.len(),
        });
    }

    let mut out = vec![0_u64; lhs.len()];
    let code = if lhs.is_empty() {
        0
    } else {
        unsafe { operation(lhs.as_ptr(), rhs.as_ptr(), out.as_mut_ptr(), lhs.len()) }
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_add(lhs: &[u64], rhs: &[u64]) -> Result<Vec<u64>, AccelError> {
    run_cuda_binary_op(lhs, rhs, lzvm_cuda_goldilocks_add)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_mul(lhs: &[u64], rhs: &[u64]) -> Result<Vec<u64>, AccelError> {
    run_cuda_binary_op(lhs, rhs, lzvm_cuda_goldilocks_mul)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_butterfly(
    even: &[u64],
    odd: &[u64],
    twiddle: &[u64],
) -> Result<(Vec<u64>, Vec<u64>), AccelError> {
    if even.len() != odd.len() {
        return Err(AccelError::LengthMismatch {
            lhs: even.len(),
            rhs: odd.len(),
        });
    }
    if even.len() != twiddle.len() {
        return Err(AccelError::LengthMismatch {
            lhs: even.len(),
            rhs: twiddle.len(),
        });
    }

    let mut out_even = vec![0_u64; even.len()];
    let mut out_odd = vec![0_u64; even.len()];
    let code = if even.is_empty() {
        0
    } else {
        unsafe {
            lzvm_cuda_goldilocks_butterfly(
                even.as_ptr(),
                odd.as_ptr(),
                twiddle.as_ptr(),
                out_even.as_mut_ptr(),
                out_odd.as_mut_ptr(),
                even.len(),
            )
        }
    };
    if code == 0 {
        Ok((out_even, out_odd))
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_ntt(values: &[u64], bits: usize) -> Result<Vec<u64>, AccelError> {
    let Some(root) = ROOTS_OF_UNITY.get(bits).copied() else {
        return Err(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        });
    };
    let expected_len = 1_usize
        .checked_shl(u32::try_from(bits).map_err(|_| AccelError::InvalidDomain {
            bits,
            len: values.len(),
        })?)
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        })?;
    if values.len() != expected_len {
        return Err(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        });
    }
    ensure_cuda_setup(bits)?;

    let mut out = vec![0_u64; values.len()];
    let code = unsafe {
        lzvm_cuda_goldilocks_ntt(values.as_ptr(), out.as_mut_ptr(), values.len(), bits, root)
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_intt(values: &[u64], bits: usize) -> Result<Vec<u64>, AccelError> {
    let Some(root) = ROOTS_OF_UNITY.get(bits).copied() else {
        return Err(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        });
    };
    let expected_len = 1_usize
        .checked_shl(u32::try_from(bits).map_err(|_| AccelError::InvalidDomain {
            bits,
            len: values.len(),
        })?)
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        })?;
    if values.len() != expected_len {
        return Err(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        });
    }
    ensure_cuda_setup(bits)?;

    let mut out = vec![0_u64; values.len()];
    let code = unsafe {
        lzvm_cuda_goldilocks_intt(values.as_ptr(), out.as_mut_ptr(), values.len(), bits, root)
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend(
    values: &[u64],
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<u64>, AccelError> {
    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(values.len(), source_bits, target_bits)?;
    if values.len() != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let mut out = vec![0_u64; target_len];
    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend(
            values.as_ptr(),
            out.as_mut_ptr(),
            source_len,
            source_bits,
            target_len,
            target_bits,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns(
    values: &[u64],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<u64>, AccelError> {
    if column_count == 0 {
        if values.is_empty() {
            return Ok(Vec::new());
        }
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    if !values.len().is_multiple_of(column_count) {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }

    let source_rows = values.len() / column_count;
    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let out_len = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: values.len(),
        })?;
    let mut out = vec![0_u64; out_len];
    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns(
            values.as_ptr(),
            out.as_mut_ptr(),
            source_len,
            source_bits,
            target_len,
            target_bits,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
    value_count: usize,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<usize, AccelError> {
    if column_count == 0 {
        if value_count == 0 {
            return Ok(0);
        }
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: value_count,
        });
    }
    if !value_count.is_multiple_of(column_count) {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: value_count,
        });
    }

    let source_rows = value_count / column_count;
    let (source_len, target_len, _, _) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: value_count,
        });
    }
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: value_count,
        })?;
    target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

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
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_words,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_device_unsynced(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    workspace: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() && workspace.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }
    if workspace.len() < out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: workspace.len(),
        });
    }

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
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_words,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_unsynced_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            workspace.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_device_on_stream(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    workspace: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() && workspace.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }
    if workspace.len() < out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: workspace.len(),
        });
    }

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
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_words,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_on_stream_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            workspace.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
            stream.as_raw(),
        )
    };
    cuda_status(code)?;
    stream.synchronize()
}

#[cfg(feature = "cuda")]
/// Enqueues row-major coset extension on `stream` and returns after launch.
///
/// # Safety
///
/// The caller must keep `values`, `out`, `workspace`, and `stream` alive and
/// must not read or reuse `out` or `workspace` until the queued stream work has
/// completed.
pub unsafe fn cuda_goldilocks_begin_coset_extend_row_major_columns_device_on_stream(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    workspace: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() && workspace.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }
    if workspace.len() < out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: workspace.len(),
        });
    }

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
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_words,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_on_stream_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            workspace.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
            stream.as_raw(),
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CudaRowMajorColumnView {
    pub source_rows: usize,
    pub source_row_stride: usize,
    pub column_offset: usize,
    pub column_count: usize,
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
        if values.is_empty() && out.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

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
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_rows,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            source_row_stride,
            column_offset,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    workspace: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
        if values.is_empty() && out.is_empty() && workspace.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }
    if workspace.len() < out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: workspace.len(),
        });
    }

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
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_rows,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            workspace.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            source_row_stride,
            column_offset,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    workspace: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
        if values.is_empty() && out.is_empty() && workspace.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }
    if workspace.len() < out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: workspace.len(),
        });
    }

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
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_rows,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            workspace.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            source_row_stride,
            column_offset,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
            stream.as_raw(),
        )
    };
    cuda_status(code)?;
    stream.synchronize()
}

#[cfg(feature = "cuda")]
/// Enqueues strided row-major coset extension on `stream` and returns after launch.
///
/// # Safety
///
/// The caller must keep `values`, `out`, `workspace`, and `stream` alive and
/// must not read or reuse `out` or `workspace` until the queued stream work has
/// completed.
pub unsafe fn cuda_goldilocks_begin_coset_extend_row_major_columns_strided_device_on_stream(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    workspace: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
        if values.is_empty() && out.is_empty() && workspace.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }
    if workspace.len() < out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: workspace.len(),
        });
    }

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
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_rows,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            workspace.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            source_row_stride,
            column_offset,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
            stream.as_raw(),
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy)]
struct CudaColumnMajorCosetExtendDomain {
    source_len: usize,
    target_len: usize,
    source_root_inverse: u64,
    target_root: u64,
}

#[cfg(feature = "cuda")]
fn validate_cuda_column_major_coset_extension(
    values: &CudaDeviceBuffer,
    columns: &CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
) -> Result<CudaColumnMajorCosetExtendDomain, AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
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
    if !columns.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: columns.len(),
            rhs: columns.len() / 8 * 8,
        });
    }

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
    let target_bytes = target_len
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_rows,
        })?;
    if columns.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: columns.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    Ok(CudaColumnMajorCosetExtendDomain {
        source_len,
        target_len,
        source_root_inverse: pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
        target_root,
    })
}

#[cfg(feature = "cuda")]
fn enqueue_cuda_column_major_coset_extension(
    values: &CudaDeviceBuffer,
    columns: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    stream_raw: *mut std::ffi::c_void,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && columns.is_empty() {
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
    let source_words = values.len() / 8;
    if !source_words.is_multiple_of(column_count) {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_words,
        });
    }
    let view = CudaRowMajorColumnView {
        source_rows: source_words / column_count,
        source_row_stride: column_count,
        column_offset: 0,
        column_count,
    };
    let domain = validate_cuda_column_major_coset_extension(
        values,
        columns,
        view,
        source_bits,
        target_bits,
    )?;
    let columns_ptr = columns.as_raw_ptr() as *mut u64;
    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_on_stream_raw(
            values.as_raw_ptr() as *const u64,
            columns_ptr,
            columns_ptr,
            domain.source_len,
            source_bits,
            domain.target_len,
            target_bits,
            column_count,
            domain.source_root_inverse,
            domain.target_root,
            SHIFT,
            stream_raw,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn enqueue_cuda_strided_column_major_coset_extension(
    values: &CudaDeviceBuffer,
    columns: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    stream_raw: *mut std::ffi::c_void,
) -> Result<(), AccelError> {
    if view.column_count == 0 || view.source_row_stride == 0 {
        if values.is_empty() && columns.is_empty() {
            return Ok(());
        }
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    let domain = validate_cuda_column_major_coset_extension(
        values,
        columns,
        view,
        source_bits,
        target_bits,
    )?;
    let columns_ptr = columns.as_raw_ptr() as *mut u64;
    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream_raw(
            values.as_raw_ptr() as *const u64,
            columns_ptr,
            columns_ptr,
            domain.source_len,
            source_bits,
            domain.target_len,
            target_bits,
            view.source_row_stride,
            view.column_offset,
            view.column_count,
            domain.source_root_inverse,
            domain.target_root,
            SHIFT,
            stream_raw,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_to_column_major_device_unsynced(
    values: &CudaDeviceBuffer,
    columns: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    enqueue_cuda_column_major_coset_extension(
        values,
        columns,
        column_count,
        source_bits,
        target_bits,
        std::ptr::null_mut(),
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_to_column_major_device_on_stream(
    values: &CudaDeviceBuffer,
    columns: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    enqueue_cuda_column_major_coset_extension(
        values,
        columns,
        column_count,
        source_bits,
        target_bits,
        stream.as_raw(),
    )?;
    stream.synchronize()
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_to_column_major_device_unsynced(
    values: &CudaDeviceBuffer,
    columns: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    enqueue_cuda_strided_column_major_coset_extension(
        values,
        columns,
        view,
        source_bits,
        target_bits,
        std::ptr::null_mut(),
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_to_column_major_device_on_stream(
    values: &CudaDeviceBuffer,
    columns: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    enqueue_cuda_strided_column_major_coset_extension(
        values,
        columns,
        view,
        source_bits,
        target_bits,
        stream.as_raw(),
    )?;
    stream.synchronize()
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_row_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    target_row: usize,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

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
    let out_bytes = column_count
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: column_count,
        })?;
    if out.len() != out_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: out_bytes,
            rhs: out.len(),
        });
    }
    let weights = coset_extend_row_weights(
        source_len,
        target_len,
        source_root,
        target_root,
        target_bits,
        target_row,
    )?;
    ensure_cuda_setup(target_bits)?;
    let weights_buffer = CudaDeviceBuffer::from_u64_words(&weights)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_row_device_raw(
            values.as_raw_ptr() as *const u64,
            weights_buffer.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            column_count,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_shifted_row_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    target_row: usize,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

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
    let out_bytes = column_count
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: column_count,
        })?;
    if out.len() != out_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: out_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;
    let (weights_buffer, weight_shift) = cached_coset_extend_residue_row_weights_device(
        source_len,
        target_len,
        source_root,
        target_root,
        source_bits,
        target_bits,
        target_row,
    )?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_shifted_row_device_raw(
            values.as_raw_ptr() as *const u64,
            weights_buffer.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            column_count,
            weight_shift,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_shifted_rows_device(
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

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
    let out_words =
        target_rows
            .len()
            .checked_mul(column_count)
            .ok_or(AccelError::InvalidDomain {
                bits: target_bits,
                len: target_rows.len(),
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
    if target_rows.is_empty() {
        return Ok(());
    }
    ensure_cuda_setup(target_bits)?;
    for group in shifted_row_weight_groups(source_bits, target_bits, target_rows)? {
        let (weights_buffer, _) = cached_coset_extend_residue_row_weights_device(
            source_len,
            target_len,
            source_root,
            target_root,
            source_bits,
            target_bits,
            group.residue_row,
        )?;
        let shifts = CudaDeviceBuffer::from_u64_words(&group.weight_shifts)?;
        let output_rows = CudaDeviceBuffer::from_u64_words(&group.output_rows)?;
        let code = unsafe {
            lzvm_cuda_goldilocks_coset_extend_row_major_columns_shifted_rows_device_raw(
                values.as_raw_ptr() as *const u64,
                weights_buffer.as_raw_ptr() as *const u64,
                shifts.as_raw_ptr() as *const u64,
                output_rows.as_raw_ptr() as *const u64,
                out.as_raw_ptr() as *mut u64,
                source_len,
                column_count,
                group.output_rows.len(),
            )
        };
        cuda_status(code)?;
    }
    Ok(())
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_rows_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    target_row_start: usize,
    target_row_count: usize,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() && target_row_count == 0 {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

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
    let target_row_end =
        target_row_start
            .checked_add(target_row_count)
            .ok_or(AccelError::InvalidDomain {
                bits: target_bits,
                len: target_row_start,
            })?;
    if target_row_end > target_len {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row_end,
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
    if target_row_count == 0 {
        return Ok(());
    }
    let weights = coset_extend_row_range_weights(
        source_len,
        target_len,
        source_root,
        target_root,
        target_bits,
        target_row_start,
        target_row_count,
    )?;
    ensure_cuda_setup(target_bits)?;
    let weights_buffer = CudaDeviceBuffer::from_u64_words(&weights)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_rows_device_raw(
            values.as_raw_ptr() as *const u64,
            weights_buffer.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            column_count,
            target_row_count,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_row_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    target_row: usize,
) -> Result<(), AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
        if values.is_empty() && out.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_rows,
        });
    }
    if target_row >= target_len {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row,
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
    let out_bytes = column_count
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: column_count,
        })?;
    if out.len() != out_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: out_bytes,
            rhs: out.len(),
        });
    }
    let weights = coset_extend_row_weights(
        source_len,
        target_len,
        source_root,
        target_root,
        target_bits,
        target_row,
    )?;
    ensure_cuda_setup(target_bits)?;
    let weights_buffer = CudaDeviceBuffer::from_u64_words(&weights)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_row_device_raw(
            values.as_raw_ptr() as *const u64,
            weights_buffer.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_row_stride,
            column_offset,
            column_count,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_row_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    target_row: usize,
) -> Result<(), AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
        if values.is_empty() && out.is_empty() {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

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
    let out_bytes = column_count
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: column_count,
        })?;
    if out.len() != out_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: out_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;
    let (weights_buffer, weight_shift) = cached_coset_extend_residue_row_weights_device(
        source_len,
        target_len,
        source_root,
        target_root,
        source_bits,
        target_bits,
        target_row,
    )?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_row_device_raw(
            values.as_raw_ptr() as *const u64,
            weights_buffer.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_row_stride,
            column_offset,
            column_count,
            weight_shift,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_rows_device(
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

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
    let out_words =
        target_rows
            .len()
            .checked_mul(column_count)
            .ok_or(AccelError::InvalidDomain {
                bits: target_bits,
                len: target_rows.len(),
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
    if target_rows.is_empty() {
        return Ok(());
    }
    ensure_cuda_setup(target_bits)?;
    for group in shifted_row_weight_groups(source_bits, target_bits, target_rows)? {
        let (weights_buffer, _) = cached_coset_extend_residue_row_weights_device(
            source_len,
            target_len,
            source_root,
            target_root,
            source_bits,
            target_bits,
            group.residue_row,
        )?;
        let shifts = CudaDeviceBuffer::from_u64_words(&group.weight_shifts)?;
        let output_rows = CudaDeviceBuffer::from_u64_words(&group.output_rows)?;
        let code = unsafe {
            lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_rows_device_raw(
                values.as_raw_ptr() as *const u64,
                weights_buffer.as_raw_ptr() as *const u64,
                shifts.as_raw_ptr() as *const u64,
                output_rows.as_raw_ptr() as *const u64,
                out.as_raw_ptr() as *mut u64,
                source_len,
                source_row_stride,
                column_offset,
                column_count,
                group.output_rows.len(),
            )
        };
        cuda_status(code)?;
    }
    Ok(())
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    target_row_start: usize,
    target_row_count: usize,
) -> Result<(), AccelError> {
    let CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    } = view;
    if column_count == 0 || source_row_stride == 0 {
        if values.is_empty() && out.is_empty() && target_row_count == 0 {
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
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_rows,
        });
    }
    let target_row_end =
        target_row_start
            .checked_add(target_row_count)
            .ok_or(AccelError::InvalidDomain {
                bits: target_bits,
                len: target_row_start,
            })?;
    if target_row_end > target_len {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_row_end,
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
    if target_row_count == 0 {
        return Ok(());
    }
    let weights = coset_extend_row_range_weights(
        source_len,
        target_len,
        source_root,
        target_root,
        target_bits,
        target_row_start,
        target_row_count,
    )?;
    ensure_cuda_setup(target_bits)?;
    let weights_buffer = CudaDeviceBuffer::from_u64_words(&weights)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device_raw(
            values.as_raw_ptr() as *const u64,
            weights_buffer.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_row_stride,
            column_offset,
            column_count,
            target_row_count,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(values.len(), source_bits, target_bits)?;
    let source_bytes = source_len.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits: source_bits,
        len: values.len(),
    })?;
    let target_bytes = target_len.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits: target_bits,
        len: out.len(),
    })?;
    if values.len() != source_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: source_bytes,
            rhs: values.len(),
        });
    }
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_device_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width4(values: &[u64]) -> Result<Vec<u64>, AccelError> {
    const WIDTH: usize = 4;

    if !values.len().is_multiple_of(WIDTH) {
        return Err(AccelError::InvalidDomain {
            bits: 2,
            len: values.len(),
        });
    }
    let mut out = vec![0_u64; values.len()];
    let code = if values.is_empty() {
        0
    } else {
        unsafe {
            lzvm_cuda_poseidon2_width4(values.as_ptr(), out.as_mut_ptr(), values.len() / WIDTH)
        }
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
type CudaPoseidon2DeviceOp = unsafe extern "C" fn(*const u64, *mut u64, usize) -> i32;

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_device_op(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    width: usize,
    bits: usize,
    operation: CudaPoseidon2DeviceOp,
) -> Result<(), AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }
    if values.len() != out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: out.len(),
        });
    }

    let word_count = values.len() / 8;
    if !word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: word_count,
        });
    }
    if word_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            word_count / width,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
type CudaPoseidon2MerkleParentDeviceOp = unsafe extern "C" fn(*const u64, *mut u64, usize) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2MerkleSelectedParentDeviceOp =
    unsafe extern "C" fn(*const u64, *mut u64, usize, usize) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2MerkleOpeningPathDeviceOp =
    unsafe extern "C" fn(*const u64, *mut u64, *mut u64, usize, usize) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2MerkleOpeningPrefixDeviceOp =
    unsafe extern "C" fn(*const u64, *mut u64, usize, usize, usize) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2MerkleOpeningPrefixBatchDeviceOp =
    unsafe extern "C" fn(*const u64, *const usize, *mut u64, usize, usize, usize) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2MerkleOpeningSuffixesBatchDeviceOp = unsafe extern "C" fn(
    *const *const u64,
    *const usize,
    *const usize,
    *const *mut u64,
    usize,
) -> i32;

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CudaMerkleOpeningPathWords {
    pub root: [u64; 4],
    pub siblings: Vec<u64>,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Copy)]
pub struct CudaMerkleDigestOpeningSuffixSource<'a> {
    pub values: &'a CudaDeviceBuffer,
    pub query_index: usize,
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_parent_device_op(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    width: usize,
    arity: usize,
    bits: usize,
    operation: CudaPoseidon2MerkleParentDeviceOp,
) -> Result<(), AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / width;
    let parent_state_count = child_state_count.div_ceil(arity);
    let expected_out_bytes = parent_state_count
        .checked_mul(width)
        .and_then(|word_count| word_count.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        })?;

    if out.len() != expected_out_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: expected_out_bytes,
            rhs: out.len(),
        });
    }
    if child_state_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            child_state_count,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_root_device_op(
    values: &CudaDeviceBuffer,
    width: usize,
    bits: usize,
    operation: CudaPoseidon2MerkleParentDeviceOp,
) -> Result<[u64; 4], AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / width;
    if child_state_count == 0 {
        return Ok([0; 4]);
    }

    let out = CudaDeviceBuffer::new(width.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits,
        len: child_word_count,
    })?)?;
    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            child_state_count,
        )
    };
    cuda_status(code)?;

    let root = out.to_u64_words()?;
    Ok([root[0], root[1], root[2], root[3]])
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_opening_path_device_op(
    values: &CudaDeviceBuffer,
    width: usize,
    arity: usize,
    bits: usize,
    query_index: usize,
    operation: CudaPoseidon2MerkleOpeningPathDeviceOp,
) -> Result<CudaMerkleOpeningPathWords, AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / width;
    if child_state_count == 0 || query_index >= child_state_count {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }

    let level_count = merkle_opening_level_count(child_state_count, arity);
    let sibling_word_count = level_count
        .checked_mul(arity.saturating_sub(1))
        .and_then(|count| count.checked_mul(4))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        })?;
    let mut root = [0_u64; 4];
    let mut siblings = vec![0_u64; sibling_word_count];
    let host_output_bytes = sibling_word_count
        .checked_add(4)
        .and_then(|word_count| word_count.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        })?;
    cuda_copy_sites::record_d2h_copy_site_timing("merkle_opening_path", host_output_bytes, || {
        let code = unsafe {
            operation(
                values.as_raw_ptr() as *const u64,
                root.as_mut_ptr(),
                siblings.as_mut_ptr(),
                child_state_count,
                query_index,
            )
        };
        cuda_status(code)
    })?;

    Ok(CudaMerkleOpeningPathWords { root, siblings })
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_root_device_op(
    values: &CudaDeviceBuffer,
    bits: usize,
    operation: CudaPoseidon2MerkleParentDeviceOp,
) -> Result<[u64; 4], AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(4) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / 4;
    if child_state_count == 0 {
        return Ok([0; 4]);
    }

    let out = CudaDeviceBuffer::new(u64_word_byte_len(4)?)?;
    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            child_state_count,
        )
    };
    cuda_status(code)?;

    let root = out.to_u64_words()?;
    Ok([root[0], root[1], root[2], root[3]])
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_root_device_buffer_op(
    values: &CudaDeviceBuffer,
    bits: usize,
    operation: CudaPoseidon2MerkleParentDeviceOp,
) -> Result<CudaDeviceBuffer, AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(4) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / 4;
    if child_state_count == 0 {
        return CudaDeviceBuffer::from_u64_words(&[0; 4]);
    }

    let out = CudaDeviceBuffer::new(u64_word_byte_len(4)?)?;
    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            child_state_count,
        )
    };
    cuda_status(code)?;
    Ok(out)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_parent_device_op(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    arity: usize,
    bits: usize,
    operation: CudaPoseidon2MerkleParentDeviceOp,
) -> Result<(), AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(4) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / 4;
    let parent_state_count = child_state_count.div_ceil(arity);
    let expected_out_bytes = parent_state_count
        .checked_mul(4)
        .and_then(|word_count| word_count.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        })?;
    if out.len() != expected_out_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: expected_out_bytes,
            rhs: out.len(),
        });
    }
    if child_state_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            child_state_count,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_selected_parent_device_op(
    values: &CudaDeviceBuffer,
    arity: usize,
    bits: usize,
    parent_index: usize,
    operation: CudaPoseidon2MerkleSelectedParentDeviceOp,
) -> Result<[u64; 4], AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(4) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / 4;
    let parent_state_count = child_state_count.div_ceil(arity);
    if child_state_count == 0 || parent_index >= parent_state_count {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }

    let out = CudaDeviceBuffer::new(u64_word_byte_len(4)?)?;
    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            child_state_count,
            parent_index,
        )
    };
    cuda_status(code)?;

    let digest = out.to_u64_words()?;
    Ok([digest[0], digest[1], digest[2], digest[3]])
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_opening_path_device_op(
    values: &CudaDeviceBuffer,
    arity: usize,
    bits: usize,
    query_index: usize,
    operation: CudaPoseidon2MerkleOpeningPathDeviceOp,
) -> Result<CudaMerkleOpeningPathWords, AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(4) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / 4;
    if child_state_count == 0 || query_index >= child_state_count {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }

    let level_count = merkle_opening_level_count(child_state_count, arity);
    let sibling_word_count = level_count
        .checked_mul(arity.saturating_sub(1))
        .and_then(|count| count.checked_mul(4))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        })?;
    let mut root = [0_u64; 4];
    let mut siblings = vec![0_u64; sibling_word_count];
    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            root.as_mut_ptr(),
            siblings.as_mut_ptr(),
            child_state_count,
            query_index,
        )
    };
    cuda_status(code)?;

    Ok(CudaMerkleOpeningPathWords { root, siblings })
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_opening_prefix_device_op(
    values: &CudaDeviceBuffer,
    arity: usize,
    bits: usize,
    query_index: usize,
    prefix_level_count: usize,
    operation: CudaPoseidon2MerkleOpeningPrefixDeviceOp,
) -> Result<Vec<u64>, AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(4) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / 4;
    if child_state_count == 0 || query_index >= child_state_count {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }

    let level_count = merkle_opening_level_count(child_state_count, arity);
    if prefix_level_count > level_count {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }
    let sibling_word_count = prefix_level_count
        .checked_mul(arity.saturating_sub(1))
        .and_then(|count| count.checked_mul(4))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        })?;
    let mut siblings = vec![0_u64; sibling_word_count];
    let host_output_bytes = u64_word_byte_len(sibling_word_count)?;
    cuda_copy_sites::record_d2h_copy_site_timing(
        "merkle_opening_prefix",
        host_output_bytes,
        || {
            let code = unsafe {
                operation(
                    values.as_raw_ptr() as *const u64,
                    siblings.as_mut_ptr(),
                    child_state_count,
                    query_index,
                    prefix_level_count,
                )
            };
            cuda_status(code)
        },
    )?;

    Ok(siblings)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_opening_prefix_batch_device_op(
    values: &CudaDeviceBuffer,
    arity: usize,
    bits: usize,
    query_indices: &[usize],
    prefix_level_count: usize,
    operation: CudaPoseidon2MerkleOpeningPrefixBatchDeviceOp,
) -> Result<Vec<u64>, AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(4) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / 4;
    if child_state_count == 0
        || query_indices
            .iter()
            .any(|query_index| *query_index >= child_state_count)
    {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }

    let level_count = merkle_opening_level_count(child_state_count, arity);
    if prefix_level_count > level_count {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }
    let sibling_word_count = query_indices
        .len()
        .checked_mul(prefix_level_count)
        .and_then(|count| count.checked_mul(arity.saturating_sub(1)))
        .and_then(|count| count.checked_mul(4))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        })?;
    let mut siblings = vec![0_u64; sibling_word_count];
    let host_output_bytes = u64_word_byte_len(sibling_word_count)?;
    cuda_copy_sites::record_d2h_copy_site_timing(
        "merkle_opening_prefix_batch",
        host_output_bytes,
        || {
            let code = unsafe {
                operation(
                    values.as_raw_ptr() as *const u64,
                    query_indices.as_ptr(),
                    siblings.as_mut_ptr(),
                    child_state_count,
                    query_indices.len(),
                    prefix_level_count,
                )
            };
            cuda_status(code)
        },
    )?;

    Ok(siblings)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_opening_prefix_batch_device_buffer_op(
    values: &CudaDeviceBuffer,
    arity: usize,
    bits: usize,
    query_indices: &[usize],
    prefix_level_count: usize,
    operation: CudaPoseidon2MerkleOpeningPrefixBatchDeviceOp,
) -> Result<CudaDeviceBuffer, AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(4) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / 4;
    if child_state_count == 0
        || query_indices
            .iter()
            .any(|query_index| *query_index >= child_state_count)
    {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }

    let level_count = merkle_opening_level_count(child_state_count, arity);
    if prefix_level_count > level_count {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        });
    }
    let sibling_word_count = query_indices
        .len()
        .checked_mul(prefix_level_count)
        .and_then(|count| count.checked_mul(arity.saturating_sub(1)))
        .and_then(|count| count.checked_mul(4))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_state_count,
        })?;
    let out = CudaDeviceBuffer::new(u64_word_byte_len(sibling_word_count)?)?;
    if sibling_word_count == 0 {
        return Ok(out);
    }

    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            query_indices.as_ptr(),
            out.as_raw_ptr() as *mut u64,
            child_state_count,
            query_indices.len(),
            prefix_level_count,
        )
    };
    cuda_status(code)?;

    Ok(out)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_digest_opening_suffixes_batch_device_buffer_op(
    sources: &[CudaMerkleDigestOpeningSuffixSource<'_>],
    arity: usize,
    bits: usize,
    operation: CudaPoseidon2MerkleOpeningSuffixesBatchDeviceOp,
) -> Result<Vec<CudaDeviceBuffer>, AccelError> {
    let mut values = Vec::with_capacity(sources.len());
    let mut child_state_counts = Vec::with_capacity(sources.len());
    let mut query_indices = Vec::with_capacity(sources.len());
    let mut outputs = Vec::with_capacity(sources.len());
    for source in sources {
        if !source.values.len().is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: source.values.len(),
                rhs: source.values.len() / 8 * 8,
            });
        }
        let child_word_count = source.values.len() / 8;
        if !child_word_count.is_multiple_of(4) {
            return Err(AccelError::InvalidDomain {
                bits,
                len: child_word_count,
            });
        }
        let child_state_count = child_word_count / 4;
        if child_state_count == 0 || source.query_index >= child_state_count {
            return Err(AccelError::InvalidDomain {
                bits,
                len: child_state_count,
            });
        }
        let level_count = merkle_opening_level_count(child_state_count, arity);
        let sibling_word_count = level_count
            .checked_mul(arity.saturating_sub(1))
            .and_then(|count| count.checked_mul(4))
            .ok_or(AccelError::InvalidDomain {
                bits,
                len: child_state_count,
            })?;
        values.push(source.values.as_raw_ptr() as *const u64);
        child_state_counts.push(child_state_count);
        query_indices.push(source.query_index);
        outputs.push(CudaDeviceBuffer::new(u64_word_byte_len(
            sibling_word_count,
        )?)?);
    }
    if sources.is_empty() {
        return Ok(outputs);
    }
    let output_ptrs = outputs
        .iter()
        .map(|output| output.as_raw_ptr() as *mut u64)
        .collect::<Vec<_>>();
    let code = unsafe {
        operation(
            values.as_ptr(),
            child_state_counts.as_ptr(),
            query_indices.as_ptr(),
            output_ptrs.as_ptr(),
            sources.len(),
        )
    };
    cuda_status(code)?;
    Ok(outputs)
}

#[cfg(feature = "cuda")]
fn merkle_opening_level_count(mut state_count: usize, arity: usize) -> usize {
    let mut level_count = 0;
    while state_count > 1 {
        state_count = state_count.div_ceil(arity);
        level_count += 1;
    }
    level_count
}

#[cfg(feature = "cuda")]
type CudaPoseidon2LinearRoundDeviceOp =
    unsafe extern "C" fn(*const u64, *const u64, *mut u64, usize, usize) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2LinearRoundRowMajorDeviceOp =
    unsafe extern "C" fn(*const u64, *const u64, *mut u64, usize, usize, usize, usize) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2LinearRoundRowMajorCheckedDeviceOp = unsafe extern "C" fn(
    *const u64,
    *const u64,
    *mut u64,
    usize,
    usize,
    usize,
    usize,
    *mut u32,
) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2LinearRoundRowMajorDeviceOnStreamOp = unsafe extern "C" fn(
    *const u64,
    *const u64,
    *mut u64,
    usize,
    usize,
    usize,
    usize,
    *mut std::ffi::c_void,
) -> i32;

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy)]
struct CudaLinearRoundRowMajorParams {
    width: usize,
    rate: usize,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_linear_round_device_op(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    width: usize,
    rate: usize,
    chunk_len: usize,
    operation: CudaPoseidon2LinearRoundDeviceOp,
) -> Result<(), AccelError> {
    if !current_states.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: current_states.len(),
            rhs: current_states.len() / 8 * 8,
        });
    }
    if !row_values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: row_values.len(),
            rhs: row_values.len() / 8 * 8,
        });
    }
    if current_states.len() != out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: current_states.len(),
            rhs: out.len(),
        });
    }
    if chunk_len == 0 || chunk_len > rate {
        return Err(AccelError::InvalidDomain {
            bits: width,
            len: chunk_len,
        });
    }

    let current_word_count = current_states.len() / 8;
    if !current_word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits: width,
            len: current_word_count,
        });
    }
    let row_count = current_word_count / width;
    let expected_row_bytes = row_count
        .checked_mul(chunk_len)
        .and_then(|word_count| word_count.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits: width,
            len: current_word_count,
        })?;
    if row_values.len() != expected_row_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: expected_row_bytes,
            rhs: row_values.len(),
        });
    }
    if row_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            current_states.as_raw_ptr() as *const u64,
            row_values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            row_count,
            chunk_len,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_linear_round_row_major_device_op(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    params: CudaLinearRoundRowMajorParams,
    operation: CudaPoseidon2LinearRoundRowMajorDeviceOp,
) -> Result<(), AccelError> {
    let row_count = validate_cuda_poseidon2_linear_round_row_major_buffers(
        current_states,
        row_values,
        out,
        params,
    )?;
    if row_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            current_states.as_raw_ptr() as *const u64,
            row_values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            row_count,
            params.column_count,
            params.offset,
            params.chunk_len,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_linear_round_row_major_checked_device_op(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    canonical_check: &CudaCanonicalCheck,
    params: CudaLinearRoundRowMajorParams,
    operation: CudaPoseidon2LinearRoundRowMajorCheckedDeviceOp,
) -> Result<(), AccelError> {
    let row_count = validate_cuda_poseidon2_linear_round_row_major_buffers(
        current_states,
        row_values,
        out,
        params,
    )?;
    if row_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            current_states.as_raw_ptr() as *const u64,
            row_values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            row_count,
            params.column_count,
            params.offset,
            params.chunk_len,
            canonical_check.as_raw_device_ptr(),
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_linear_round_row_major_device_op_on_stream(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    params: CudaLinearRoundRowMajorParams,
    stream: &CudaStream,
    operation: CudaPoseidon2LinearRoundRowMajorDeviceOnStreamOp,
) -> Result<(), AccelError> {
    let row_count = validate_cuda_poseidon2_linear_round_row_major_buffers(
        current_states,
        row_values,
        out,
        params,
    )?;
    if row_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            current_states.as_raw_ptr() as *const u64,
            row_values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            row_count,
            params.column_count,
            params.offset,
            params.chunk_len,
            stream.as_raw(),
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn validate_cuda_poseidon2_linear_round_row_major_buffers(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &CudaDeviceBuffer,
    params: CudaLinearRoundRowMajorParams,
) -> Result<usize, AccelError> {
    if !current_states.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: current_states.len(),
            rhs: current_states.len() / 8 * 8,
        });
    }
    if !row_values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: row_values.len(),
            rhs: row_values.len() / 8 * 8,
        });
    }
    if current_states.len() != out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: current_states.len(),
            rhs: out.len(),
        });
    }
    if params.chunk_len == 0 || params.chunk_len > params.rate {
        return Err(AccelError::InvalidDomain {
            bits: params.width,
            len: params.chunk_len,
        });
    }
    if params
        .offset
        .checked_add(params.chunk_len)
        .is_none_or(|end| end > params.column_count)
    {
        return Err(AccelError::InvalidDomain {
            bits: params.width,
            len: params.column_count,
        });
    }

    let current_word_count = current_states.len() / 8;
    if !current_word_count.is_multiple_of(params.width) {
        return Err(AccelError::InvalidDomain {
            bits: params.width,
            len: current_word_count,
        });
    }
    let row_count = current_word_count / params.width;
    let expected_row_bytes = row_count
        .checked_mul(params.column_count)
        .and_then(|word_count| word_count.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits: params.width,
            len: current_word_count,
        })?;
    if row_values.len() != expected_row_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: expected_row_bytes,
            rhs: row_values.len(),
        });
    }
    Ok(row_count)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width4_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_device_op(values, out, 4, 2, lzvm_cuda_poseidon2_width4_device_raw)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width4_find_nonce(
    challenge: [u64; 3],
    start: u64,
    count: usize,
    target: u64,
) -> Result<Option<u64>, AccelError> {
    if count == 0 {
        return Ok(None);
    }
    let count_u64 = u64::try_from(count).map_err(|_| AccelError::InvalidDomain {
        bits: 2,
        len: count,
    })?;
    start
        .checked_add(count_u64 - 1)
        .ok_or(AccelError::InvalidDomain {
            bits: 2,
            len: count,
        })?;

    let mut out = 0_u64;
    let mut found = 0_u32;
    let code = unsafe {
        lzvm_cuda_poseidon2_width4_find_nonce(
            challenge.as_ptr(),
            start,
            count,
            target,
            &mut out,
            &mut found,
        )
    };
    if code == 0 {
        Ok((found != 0).then_some(out))
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8(values: &[u64]) -> Result<Vec<u64>, AccelError> {
    const WIDTH: usize = 8;

    if !values.len().is_multiple_of(WIDTH) {
        return Err(AccelError::InvalidDomain {
            bits: 3,
            len: values.len(),
        });
    }
    let mut out = vec![0_u64; values.len()];
    let code = if values.is_empty() {
        0
    } else {
        unsafe {
            lzvm_cuda_poseidon2_width8(values.as_ptr(), out.as_mut_ptr(), values.len() / WIDTH)
        }
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_device_op(values, out, 8, 3, lzvm_cuda_poseidon2_width8_device_raw)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_parent_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_merkle_parent_device_op(
        values,
        out,
        8,
        2,
        3,
        lzvm_cuda_poseidon2_width8_merkle_parent_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_root_device(
    values: &CudaDeviceBuffer,
) -> Result<[u64; 4], AccelError> {
    run_cuda_poseidon2_merkle_root_device_op(
        values,
        8,
        3,
        lzvm_cuda_poseidon2_width8_merkle_root_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_opening_path_device(
    values: &CudaDeviceBuffer,
    query_index: usize,
) -> Result<CudaMerkleOpeningPathWords, AccelError> {
    run_cuda_poseidon2_merkle_opening_path_device_op(
        values,
        8,
        2,
        3,
        query_index,
        lzvm_cuda_poseidon2_width8_merkle_opening_path_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_root_device(
    values: &CudaDeviceBuffer,
) -> Result<[u64; 4], AccelError> {
    run_cuda_poseidon2_merkle_digest_root_device_op(
        values,
        3,
        lzvm_cuda_poseidon2_width8_merkle_digest_root_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_root_device_buffer(
    values: &CudaDeviceBuffer,
) -> Result<CudaDeviceBuffer, AccelError> {
    run_cuda_poseidon2_merkle_digest_root_device_buffer_op(
        values,
        3,
        lzvm_cuda_poseidon2_width8_merkle_digest_root_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_parent_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_merkle_digest_parent_device_op(
        values,
        out,
        2,
        3,
        lzvm_cuda_poseidon2_width8_merkle_digest_parent_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_selected_parent_device(
    values: &CudaDeviceBuffer,
    parent_index: usize,
) -> Result<[u64; 4], AccelError> {
    run_cuda_poseidon2_merkle_digest_selected_parent_device_op(
        values,
        2,
        3,
        parent_index,
        lzvm_cuda_poseidon2_width8_merkle_digest_selected_parent_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_opening_path_device(
    values: &CudaDeviceBuffer,
    query_index: usize,
) -> Result<CudaMerkleOpeningPathWords, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_path_device_op(
        values,
        2,
        3,
        query_index,
        lzvm_cuda_poseidon2_width8_merkle_digest_opening_path_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_opening_prefix_device(
    values: &CudaDeviceBuffer,
    query_index: usize,
    prefix_level_count: usize,
) -> Result<Vec<u64>, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_prefix_device_op(
        values,
        2,
        3,
        query_index,
        prefix_level_count,
        lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device(
    values: &CudaDeviceBuffer,
    query_indices: &[usize],
    prefix_level_count: usize,
) -> Result<Vec<u64>, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_prefix_batch_device_op(
        values,
        2,
        3,
        query_indices,
        prefix_level_count,
        lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_buffer(
    values: &CudaDeviceBuffer,
    query_indices: &[usize],
    prefix_level_count: usize,
) -> Result<CudaDeviceBuffer, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_prefix_batch_device_buffer_op(
        values,
        2,
        3,
        query_indices,
        prefix_level_count,
        lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_to_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_digest_opening_suffixes_batch_device_buffers(
    sources: &[CudaMerkleDigestOpeningSuffixSource<'_>],
) -> Result<Vec<CudaDeviceBuffer>, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_suffixes_batch_device_buffer_op(
        sources,
        2,
        3,
        lzvm_cuda_poseidon2_width8_merkle_digest_opening_suffixes_batch_to_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16(values: &[u64]) -> Result<Vec<u64>, AccelError> {
    const WIDTH: usize = 16;

    if !values.len().is_multiple_of(WIDTH) {
        return Err(AccelError::InvalidDomain {
            bits: 4,
            len: values.len(),
        });
    }
    let mut out = vec![0_u64; values.len()];
    let code = if values.is_empty() {
        0
    } else {
        unsafe {
            lzvm_cuda_poseidon2_width16(values.as_ptr(), out.as_mut_ptr(), values.len() / WIDTH)
        }
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_device_op(values, out, 16, 4, lzvm_cuda_poseidon2_width16_device_raw)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_parent_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_merkle_parent_device_op(
        values,
        out,
        16,
        4,
        4,
        lzvm_cuda_poseidon2_width16_merkle_parent_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_root_device(
    values: &CudaDeviceBuffer,
) -> Result<[u64; 4], AccelError> {
    run_cuda_poseidon2_merkle_root_device_op(
        values,
        16,
        4,
        lzvm_cuda_poseidon2_width16_merkle_root_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_opening_path_device(
    values: &CudaDeviceBuffer,
    query_index: usize,
) -> Result<CudaMerkleOpeningPathWords, AccelError> {
    run_cuda_poseidon2_merkle_opening_path_device_op(
        values,
        16,
        4,
        4,
        query_index,
        lzvm_cuda_poseidon2_width16_merkle_opening_path_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_root_device(
    values: &CudaDeviceBuffer,
) -> Result<[u64; 4], AccelError> {
    run_cuda_poseidon2_merkle_digest_root_device_op(
        values,
        4,
        lzvm_cuda_poseidon2_width16_merkle_digest_root_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_root_device_buffer(
    values: &CudaDeviceBuffer,
) -> Result<CudaDeviceBuffer, AccelError> {
    run_cuda_poseidon2_merkle_digest_root_device_buffer_op(
        values,
        4,
        lzvm_cuda_poseidon2_width16_merkle_digest_root_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_parent_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_merkle_digest_parent_device_op(
        values,
        out,
        4,
        4,
        lzvm_cuda_poseidon2_width16_merkle_digest_parent_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_selected_parent_device(
    values: &CudaDeviceBuffer,
    parent_index: usize,
) -> Result<[u64; 4], AccelError> {
    run_cuda_poseidon2_merkle_digest_selected_parent_device_op(
        values,
        4,
        4,
        parent_index,
        lzvm_cuda_poseidon2_width16_merkle_digest_selected_parent_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_opening_path_device(
    values: &CudaDeviceBuffer,
    query_index: usize,
) -> Result<CudaMerkleOpeningPathWords, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_path_device_op(
        values,
        4,
        4,
        query_index,
        lzvm_cuda_poseidon2_width16_merkle_digest_opening_path_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_opening_prefix_device(
    values: &CudaDeviceBuffer,
    query_index: usize,
    prefix_level_count: usize,
) -> Result<Vec<u64>, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_prefix_device_op(
        values,
        4,
        4,
        query_index,
        prefix_level_count,
        lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device(
    values: &CudaDeviceBuffer,
    query_indices: &[usize],
    prefix_level_count: usize,
) -> Result<Vec<u64>, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_prefix_batch_device_op(
        values,
        4,
        4,
        query_indices,
        prefix_level_count,
        lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_buffer(
    values: &CudaDeviceBuffer,
    query_indices: &[usize],
    prefix_level_count: usize,
) -> Result<CudaDeviceBuffer, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_prefix_batch_device_buffer_op(
        values,
        4,
        4,
        query_indices,
        prefix_level_count,
        lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_to_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_digest_opening_suffixes_batch_device_buffers(
    sources: &[CudaMerkleDigestOpeningSuffixSource<'_>],
) -> Result<Vec<CudaDeviceBuffer>, AccelError> {
    run_cuda_poseidon2_merkle_digest_opening_suffixes_batch_device_buffer_op(
        sources,
        4,
        4,
        lzvm_cuda_poseidon2_width16_merkle_digest_opening_suffixes_batch_to_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_linear_round_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_device_op(
        current_states,
        row_values,
        out,
        8,
        4,
        chunk_len,
        lzvm_cuda_poseidon2_width8_linear_round_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_linear_round_row_major_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op(
        current_states,
        row_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 8,
            rate: 4,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width8_linear_round_row_major_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_linear_round_row_major_digest_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op(
        current_states,
        row_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 8,
            rate: 4,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width8_linear_round_row_major_digest_device_raw,
    )
}

/// Enqueues a width-8 row-major digest round on `stream` and returns after launch.
///
/// # Safety
///
/// The caller must keep `current_states`, `row_values`, `out`, and `stream`
/// alive until the queued stream work has completed, and must not read or
/// reuse `out` until that work has completed.
#[cfg(feature = "cuda")]
pub unsafe fn cuda_poseidon2_begin_width8_linear_round_row_major_digest_device_on_stream(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op_on_stream(
        current_states,
        row_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 8,
            rate: 4,
            column_count,
            offset,
            chunk_len,
        },
        stream,
        lzvm_cuda_poseidon2_width8_linear_round_row_major_digest_device_on_stream_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_linear_round_column_major_digest_device(
    current_states: &CudaDeviceBuffer,
    column_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op(
        current_states,
        column_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 8,
            rate: 4,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_linear_round_column_major_digest_checked_device(
    current_states: &CudaDeviceBuffer,
    column_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
    canonical_check: &CudaCanonicalCheck,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_checked_device_op(
        current_states,
        column_values,
        out,
        canonical_check,
        CudaLinearRoundRowMajorParams {
            width: 8,
            rate: 4,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_checked_device_raw,
    )
}

/// Enqueues a width-8 column-major digest round on `stream` and returns after launch.
///
/// # Safety
///
/// The caller must keep `current_states`, `column_values`, `out`, and `stream`
/// alive until the queued stream work has completed, and must not read or
/// reuse `out` until that work has completed.
#[cfg(feature = "cuda")]
pub unsafe fn cuda_poseidon2_begin_width8_linear_round_column_major_digest_device_on_stream(
    current_states: &CudaDeviceBuffer,
    column_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op_on_stream(
        current_states,
        column_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 8,
            rate: 4,
            column_count,
            offset,
            chunk_len,
        },
        stream,
        lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_device_on_stream_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_linear_round_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_device_op(
        current_states,
        row_values,
        out,
        16,
        12,
        chunk_len,
        lzvm_cuda_poseidon2_width16_linear_round_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_linear_round_row_major_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op(
        current_states,
        row_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 16,
            rate: 12,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width16_linear_round_row_major_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_linear_round_row_major_digest_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op(
        current_states,
        row_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 16,
            rate: 12,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width16_linear_round_row_major_digest_device_raw,
    )
}

/// Enqueues a width-16 row-major digest round on `stream` and returns after launch.
///
/// # Safety
///
/// The caller must keep `current_states`, `row_values`, `out`, and `stream`
/// alive until the queued stream work has completed, and must not read or
/// reuse `out` until that work has completed.
#[cfg(feature = "cuda")]
pub unsafe fn cuda_poseidon2_begin_width16_linear_round_row_major_digest_device_on_stream(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op_on_stream(
        current_states,
        row_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 16,
            rate: 12,
            column_count,
            offset,
            chunk_len,
        },
        stream,
        lzvm_cuda_poseidon2_width16_linear_round_row_major_digest_device_on_stream_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_linear_round_column_major_digest_device(
    current_states: &CudaDeviceBuffer,
    column_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op(
        current_states,
        column_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 16,
            rate: 12,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_linear_round_column_major_digest_checked_device(
    current_states: &CudaDeviceBuffer,
    column_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
    canonical_check: &CudaCanonicalCheck,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_checked_device_op(
        current_states,
        column_values,
        out,
        canonical_check,
        CudaLinearRoundRowMajorParams {
            width: 16,
            rate: 12,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_checked_device_raw,
    )
}

/// Enqueues a width-16 column-major digest round on `stream` and returns after launch.
///
/// # Safety
///
/// The caller must keep `current_states`, `column_values`, `out`, and `stream`
/// alive until the queued stream work has completed, and must not read or
/// reuse `out` until that work has completed.
#[cfg(feature = "cuda")]
pub unsafe fn cuda_poseidon2_begin_width16_linear_round_column_major_digest_device_on_stream(
    current_states: &CudaDeviceBuffer,
    column_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
    stream: &CudaStream,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op_on_stream(
        current_states,
        column_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 16,
            rate: 12,
            column_count,
            offset,
            chunk_len,
        },
        stream,
        lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_device_on_stream_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_keccak256_fixed(input: &[u8], message_len: usize) -> Result<Vec<[u8; 32]>, AccelError> {
    if message_len == 0 || !input.len().is_multiple_of(message_len) {
        return Err(AccelError::InvalidDomain {
            bits: 0,
            len: input.len(),
        });
    }

    let message_count = input.len() / message_len;
    let output_len = message_count
        .checked_mul(32)
        .ok_or(AccelError::InvalidDomain {
            bits: 0,
            len: input.len(),
        })?;
    let mut out = vec![0_u8; output_len];
    let code = if message_count == 0 {
        0
    } else {
        unsafe {
            lzvm_cuda_keccak256_fixed(input.as_ptr(), message_len, out.as_mut_ptr(), message_count)
        }
    };
    if code == 0 {
        Ok(out
            .chunks_exact(32)
            .map(|chunk| {
                let mut digest = [0_u8; 32];
                digest.copy_from_slice(chunk);
                digest
            })
            .collect())
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_setup_init(_max_bits_ext: usize) -> Result<(), AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_add(_lhs: &[u64], _rhs: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_mul(_lhs: &[u64], _rhs: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_butterfly(
    _even: &[u64],
    _odd: &[u64],
    _twiddle: &[u64],
) -> Result<(Vec<u64>, Vec<u64>), AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_ntt(_values: &[u64], _bits: usize) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_intt(_values: &[u64], _bits: usize) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_coset_extend(
    _values: &[u64],
    _source_bits: usize,
    _target_bits: usize,
) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_poseidon2_width4(_values: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_poseidon2_width4_find_nonce(
    _challenge: [u64; 3],
    _start: u64,
    _count: usize,
    _target: u64,
) -> Result<Option<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_poseidon2_width8(_values: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_poseidon2_width16(_values: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_keccak256_fixed(
    _input: &[u8],
    _message_len: usize,
) -> Result<Vec<[u8; 32]>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(test)]
mod error_tests {
    use super::AccelError;

    #[test]
    fn cuda_error_code_two_reports_out_of_memory() {
        assert_eq!(
            AccelError::Cuda { code: 2 }.to_string(),
            "cuda backend out of memory: error code 2"
        );
    }

    #[test]
    fn cuda_non_memory_error_keeps_raw_code() {
        assert_eq!(
            AccelError::Cuda { code: 700 }.to_string(),
            "cuda backend error: 700"
        );
    }
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::{
        coset_extend_domain, coset_extend_row_weights,
        cuda_goldilocks_begin_coset_extend_row_major_columns_device_on_stream,
        cuda_goldilocks_coset_extend_row_major_columns_device,
        cuda_goldilocks_coset_extend_row_major_columns_device_on_stream,
        cuda_goldilocks_coset_extend_row_major_columns_output_bytes,
        cuda_goldilocks_coset_extend_row_major_columns_strided_device,
        cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream,
        cuda_goldilocks_coset_extend_row_major_columns_strided_to_column_major_device_on_stream,
        cuda_goldilocks_coset_extend_row_major_columns_to_column_major_device_unsynced,
        cuda_poseidon2_begin_width16_linear_round_column_major_digest_device_on_stream,
        cuda_poseidon2_begin_width16_linear_round_row_major_digest_device_on_stream,
        cuda_poseidon2_width16_linear_round_column_major_digest_device,
        cuda_poseidon2_width16_linear_round_row_major_digest_device,
        cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_buffer,
        cuda_poseidon2_width16_merkle_digest_opening_suffixes_batch_device_buffers,
        cuda_poseidon2_width8_linear_round_column_major_digest_device,
        cuda_poseidon2_width8_linear_round_row_major_digest_device,
        cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_buffer,
        cuda_poseidon2_width8_merkle_digest_opening_suffixes_batch_device_buffers, cuda_setup_init,
        cuda_status, lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_on_stream_raw,
        lzvm_cuda_goldilocks_intt, lzvm_cuda_goldilocks_ntt, merkle_opening_level_count, mul_mod,
        pow_mod, row_weight_shift_for_target_row, sub_mod, CudaDeviceBuffer, CudaEvent,
        CudaMerkleDigestOpeningSuffixSource, CudaRowMajorColumnView,
        CudaRowMajorCosetExtensionGraphRunner, CudaStream,
        CudaStridedRowMajorCosetExtensionGraphRunner, GOLDILOCKS_MODULUS, ROOTS_OF_UNITY, SHIFT,
    };

    fn reference_ntt_with_root(mut values: Vec<u64>, bits: usize, root: u64) -> Vec<u64> {
        let len = values.len();
        for index in 0..len {
            let reverse = index.reverse_bits() >> (usize::BITS as usize - bits);
            if index < reverse {
                values.swap(index, reverse);
            }
        }
        let mut stage_len = 2;
        while stage_len <= len {
            let half = stage_len / 2;
            let stage_twiddle = pow_mod(root, (len / stage_len) as u64);
            for group in (0..len).step_by(stage_len) {
                for offset in 0..half {
                    let even_index = group + offset;
                    let odd_index = even_index + half;
                    let even = values[even_index];
                    let odd = mul_mod(values[odd_index], pow_mod(stage_twiddle, offset as u64));
                    values[even_index] =
                        ((even as u128 + odd as u128) % GOLDILOCKS_MODULUS as u128) as u64;
                    values[odd_index] = sub_mod(even, odd);
                }
            }
            stage_len <<= 1;
        }
        values
    }

    fn transpose_row_major_words(
        values: &[u64],
        row_count: usize,
        column_count: usize,
    ) -> Vec<u64> {
        let mut columns = vec![0_u64; values.len()];
        for row in 0..row_count {
            for column in 0..column_count {
                columns[column * row_count + row] = values[row * column_count + column];
            }
        }
        columns
    }

    #[test]
    fn digest_opening_suffixes_batch_matches_binary_per_buffer_paths() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state_counts = [1usize, 2, 3, 17, 70];
        let query_indices = [0usize, 1, 2, 16, 69];
        let buffers = state_counts
            .iter()
            .enumerate()
            .map(|(group, state_count)| {
                let words = (0..state_count * 4)
                    .map(|index| 1000 + group as u64 * 1000 + index as u64)
                    .collect::<Vec<_>>();
                CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload")
            })
            .collect::<Vec<_>>();
        let sources = buffers
            .iter()
            .zip(query_indices)
            .map(
                |(values, query_index)| CudaMerkleDigestOpeningSuffixSource {
                    values,
                    query_index,
                },
            )
            .collect::<Vec<_>>();

        let actual =
            cuda_poseidon2_width8_merkle_digest_opening_suffixes_batch_device_buffers(&sources)
                .expect("batched paths should launch");
        for (((values, state_count), query_index), actual) in buffers
            .iter()
            .zip(state_counts)
            .zip(query_indices)
            .zip(actual)
        {
            let level_count = merkle_opening_level_count(state_count, 2);
            let expected = cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_buffer(
                values,
                &[query_index],
                level_count,
            )
            .expect("single-buffer path should launch")
            .to_u64_words()
            .expect("single-buffer path should download");
            assert_eq!(
                actual.to_u64_words().expect("batched path should download"),
                expected
            );
        }
    }

    #[test]
    fn digest_opening_suffixes_batch_matches_quaternary_per_buffer_paths() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state_counts = [1usize, 4, 5, 19, 70];
        let query_indices = [0usize, 3, 4, 18, 69];
        let buffers = state_counts
            .iter()
            .enumerate()
            .map(|(group, state_count)| {
                let words = (0..state_count * 4)
                    .map(|index| 8000 + group as u64 * 1000 + index as u64)
                    .collect::<Vec<_>>();
                CudaDeviceBuffer::from_u64_words(&words).expect("digests should upload")
            })
            .collect::<Vec<_>>();
        let sources = buffers
            .iter()
            .zip(query_indices)
            .map(
                |(values, query_index)| CudaMerkleDigestOpeningSuffixSource {
                    values,
                    query_index,
                },
            )
            .collect::<Vec<_>>();

        let actual =
            cuda_poseidon2_width16_merkle_digest_opening_suffixes_batch_device_buffers(&sources)
                .expect("batched paths should launch");
        for (((values, state_count), query_index), actual) in buffers
            .iter()
            .zip(state_counts)
            .zip(query_indices)
            .zip(actual)
        {
            let level_count = merkle_opening_level_count(state_count, 4);
            let expected = cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_buffer(
                values,
                &[query_index],
                level_count,
            )
            .expect("single-buffer path should launch")
            .to_u64_words()
            .expect("single-buffer path should download");
            assert_eq!(
                actual.to_u64_words().expect("batched path should download"),
                expected
            );
        }
    }

    #[test]
    fn native_ntt_honors_noncanonical_root_argument() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let bits = 3;
        let input = vec![3, 5, 8, 13, 21, 34, 55, 89];
        cuda_setup_init(bits).expect("CUDA setup should initialize");
        let root = pow_mod(ROOTS_OF_UNITY[bits], 3);
        assert_ne!(root, ROOTS_OF_UNITY[bits]);
        let expected = reference_ntt_with_root(input.clone(), bits, root);
        let mut actual = vec![0_u64; input.len()];

        let code = unsafe {
            lzvm_cuda_goldilocks_ntt(input.as_ptr(), actual.as_mut_ptr(), input.len(), bits, root)
        };

        assert_eq!(code, 0);
        assert_eq!(actual, expected);

        let mut recovered = vec![0_u64; input.len()];
        let code = unsafe {
            lzvm_cuda_goldilocks_intt(
                actual.as_ptr(),
                recovered.as_mut_ptr(),
                actual.len(),
                bits,
                root,
            )
        };

        assert_eq!(code, 0);
        assert_eq!(recovered, input);
    }

    #[test]
    fn copy_from_u64_words_on_stream_matches_blocking_upload() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let values = (0..4096)
            .map(|index| (index as u64 + 17) * 19)
            .collect::<Vec<_>>();
        let stream = CudaStream::new().expect("CUDA stream should create");
        let mut streamed = CudaDeviceBuffer::new(values.len() * std::mem::size_of::<u64>())
            .expect("streamed upload buffer should allocate");

        unsafe {
            streamed
                .copy_from_u64_words_on_stream(&values, &stream)
                .expect("stream upload should enqueue");
        }
        stream.synchronize().expect("stream upload should finish");

        let blocking =
            CudaDeviceBuffer::from_u64_words(&values).expect("blocking upload should run");
        assert_eq!(
            streamed
                .to_u64_words()
                .expect("streamed upload should download"),
            blocking
                .to_u64_words()
                .expect("blocking upload should download")
        );

        let mut too_small = CudaDeviceBuffer::new((values.len() - 1) * std::mem::size_of::<u64>())
            .expect("short buffer should allocate");
        assert!(
            unsafe {
                too_small
                    .copy_from_u64_words_on_stream(&values, &stream)
                    .is_err()
            },
            "stream upload should reject length mismatches"
        );
    }

    #[test]
    fn row_weights_for_matching_blowup_residue_are_cyclic_shifts() {
        let source_bits = 5;
        let target_bits = 8;
        let source_rows = 1_usize << source_bits;
        let target_rows = 1_usize << target_bits;
        let (_, _, source_root, target_root) =
            coset_extend_domain(source_rows, source_bits, target_bits).expect("domain");
        let residue_row = 3;
        let shifted_row = residue_row + (11 << (target_bits - source_bits));

        let base = coset_extend_row_weights(
            source_rows,
            target_rows,
            source_root,
            target_root,
            target_bits,
            residue_row,
        )
        .expect("base weights");
        let shifted = coset_extend_row_weights(
            source_rows,
            target_rows,
            source_root,
            target_root,
            target_bits,
            shifted_row,
        )
        .expect("shifted weights");
        let (residue, shift) =
            row_weight_shift_for_target_row(source_bits, target_bits, shifted_row)
                .expect("row shift");
        assert_eq!(residue, residue_row);

        for index in 0..source_rows {
            assert_eq!(shifted[index], base[(index + shift) % source_rows]);
        }
    }

    #[test]
    fn row_major_coset_extension_on_stream_matches_default_stream() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let column_count = 3;
        let source_rows = 1_usize << source_bits;
        let values = (0..source_rows * column_count)
            .map(|index| (index as u64 + 1) * 17)
            .collect::<Vec<_>>();
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");

        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut default_out =
            CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source,
            &mut default_out,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("default stream extension should run");

        let mut stream_out =
            CudaDeviceBuffer::new(out_byte_count).expect("stream output should allocate");
        let mut workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("stream workspace should allocate");
        let stream = CudaStream::new().expect("CUDA stream should create");
        cuda_goldilocks_coset_extend_row_major_columns_device_on_stream(
            &source,
            &mut stream_out,
            &mut workspace,
            column_count,
            source_bits,
            target_bits,
            &stream,
        )
        .expect("explicit stream extension should enqueue");
        stream
            .synchronize()
            .expect("explicit stream extension should finish");

        assert_eq!(
            stream_out
                .to_u64_words()
                .expect("stream output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    #[test]
    fn column_major_coset_extension_matches_transposed_row_major_output() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let target_rows = 1_usize << target_bits;
        let column_count = 3;
        let values = (0..source_rows * column_count)
            .map(|index| (index as u64 + 1) * 41)
            .collect::<Vec<_>>();
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");
        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut row_major =
            CudaDeviceBuffer::new(out_byte_count).expect("row-major output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source,
            &mut row_major,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("row-major extension should run");
        let mut column_major =
            CudaDeviceBuffer::new(out_byte_count).expect("column-major output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_to_column_major_device_unsynced(
            &source,
            &mut column_major,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("column-major extension should enqueue");

        let expected = transpose_row_major_words(
            &row_major
                .to_u64_words()
                .expect("row-major output should download"),
            target_rows,
            column_count,
        );
        assert_eq!(
            column_major
                .to_u64_words()
                .expect("column-major output should download"),
            expected
        );

        let source_row_stride = 5;
        let column_offset = 1;
        let strided_values = (0..source_rows * source_row_stride)
            .map(|index| (index as u64 + 7) * 43)
            .collect::<Vec<_>>();
        let view = CudaRowMajorColumnView {
            source_rows,
            source_row_stride,
            column_offset,
            column_count,
        };
        let strided_source =
            CudaDeviceBuffer::from_u64_words(&strided_values).expect("source should upload");
        let mut strided_row_major =
            CudaDeviceBuffer::new(out_byte_count).expect("row-major output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_strided_device(
            &strided_source,
            &mut strided_row_major,
            view,
            source_bits,
            target_bits,
        )
        .expect("strided row-major extension should run");
        let mut strided_column_major =
            CudaDeviceBuffer::new(out_byte_count).expect("column-major output should allocate");
        let stream = CudaStream::new().expect("CUDA stream should create");
        cuda_goldilocks_coset_extend_row_major_columns_strided_to_column_major_device_on_stream(
            &strided_source,
            &mut strided_column_major,
            view,
            source_bits,
            target_bits,
            &stream,
        )
        .expect("strided column-major extension should run");
        let expected = transpose_row_major_words(
            &strided_row_major
                .to_u64_words()
                .expect("strided row-major output should download"),
            target_rows,
            column_count,
        );
        assert_eq!(
            strided_column_major
                .to_u64_words()
                .expect("strided column-major output should download"),
            expected
        );
    }

    #[test]
    fn row_major_coset_extension_graph_replay_matches_default_stream() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let column_count = 3;
        let source_rows = 1_usize << source_bits;
        let values = (0..source_rows * column_count)
            .map(|index| (index as u64 + 1) * 17)
            .collect::<Vec<_>>();
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");

        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut default_out =
            CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source,
            &mut default_out,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("default stream extension should run");

        let mut graph_out =
            CudaDeviceBuffer::new(out_byte_count).expect("graph output should allocate");
        let mut graph_workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("graph workspace should allocate");
        let stream = CudaStream::new().expect("CUDA stream should create");
        let capture = stream
            .begin_capture()
            .expect("CUDA graph capture should begin");
        unsafe {
            cuda_goldilocks_begin_coset_extend_row_major_columns_device_on_stream(
                &source,
                &mut graph_out,
                &mut graph_workspace,
                column_count,
                source_bits,
                target_bits,
                &stream,
            )
        }
        .expect("explicit stream extension should enqueue during capture");
        let graph = capture.end().expect("CUDA graph capture should end");
        let exec = graph.instantiate().expect("CUDA graph should instantiate");
        exec.launch(&stream).expect("CUDA graph should launch");
        stream.synchronize().expect("CUDA graph should finish");

        assert_eq!(
            graph_out
                .to_u64_words()
                .expect("graph output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    #[test]
    fn row_major_coset_extension_graph_update_retargets_output_buffer() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let column_count = 3;
        let source_rows = 1_usize << source_bits;
        let values = (0..source_rows * column_count)
            .map(|index| (index as u64 + 1) * 19)
            .collect::<Vec<_>>();
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");

        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut default_out =
            CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source,
            &mut default_out,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("default stream extension should run");

        let stream = CudaStream::new().expect("CUDA stream should create");
        let mut first_out =
            CudaDeviceBuffer::new(out_byte_count).expect("first output should allocate");
        let mut first_workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("first workspace should allocate");
        let first_capture = stream
            .begin_capture()
            .expect("first CUDA graph capture should begin");
        unsafe {
            cuda_goldilocks_begin_coset_extend_row_major_columns_device_on_stream(
                &source,
                &mut first_out,
                &mut first_workspace,
                column_count,
                source_bits,
                target_bits,
                &stream,
            )
        }
        .expect("first extension should enqueue during capture");
        let first_graph = first_capture
            .end()
            .expect("first CUDA graph capture should end");
        let mut exec = first_graph
            .instantiate()
            .expect("first CUDA graph should instantiate");

        let mut second_out =
            CudaDeviceBuffer::new(out_byte_count).expect("second output should allocate");
        let mut second_workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("second workspace should allocate");
        let second_capture = stream
            .begin_capture()
            .expect("second CUDA graph capture should begin");
        unsafe {
            cuda_goldilocks_begin_coset_extend_row_major_columns_device_on_stream(
                &source,
                &mut second_out,
                &mut second_workspace,
                column_count,
                source_bits,
                target_bits,
                &stream,
            )
        }
        .expect("second extension should enqueue during capture");
        let second_graph = second_capture
            .end()
            .expect("second CUDA graph capture should end");
        exec.update(&second_graph)
            .expect("CUDA graph exec should accept same-topology update");
        exec.launch(&stream)
            .expect("updated CUDA graph should launch");
        stream
            .synchronize()
            .expect("updated CUDA graph should finish");

        assert_eq!(
            second_out
                .to_u64_words()
                .expect("updated graph output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    #[test]
    fn row_major_coset_extension_graph_runner_replays_same_buffers() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let column_count = 3;
        let source_rows = 1_usize << source_bits;
        let mut values = (0..source_rows * column_count)
            .map(|index| (index as u64 + 1) * 23)
            .collect::<Vec<_>>();
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");

        let mut source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut graph_out =
            CudaDeviceBuffer::new(out_byte_count).expect("graph output should allocate");
        let mut graph_workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("graph workspace should allocate");
        let mut runner =
            CudaRowMajorCosetExtensionGraphRunner::new(column_count, source_bits, target_bits)
                .expect("graph runner should create");

        runner
            .run(&source, &mut graph_out, &mut graph_workspace)
            .expect("first graph run should succeed");
        assert_eq!(runner.capture_count(), 1);
        assert_eq!(runner.launch_count(), 1);

        values.iter_mut().for_each(|value| *value += 97);
        source
            .copy_from_u64_words(&values)
            .expect("source should reupload");

        let mut default_out =
            CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source,
            &mut default_out,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("default stream extension should run");
        runner
            .run(&source, &mut graph_out, &mut graph_workspace)
            .expect("second graph run should succeed");

        assert_eq!(runner.capture_count(), 1);
        assert_eq!(runner.launch_count(), 2);
        assert_eq!(
            graph_out
                .to_u64_words()
                .expect("graph output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    #[test]
    fn row_major_coset_extension_graph_runner_can_defer_synchronization() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let column_count = 3;
        let source_rows = 1_usize << source_bits;
        let values = (0..source_rows * column_count)
            .map(|index| (index as u64 + 5) * 31)
            .collect::<Vec<_>>();
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");

        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut default_out =
            CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source,
            &mut default_out,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("default stream extension should run");

        let mut graph_out =
            CudaDeviceBuffer::new(out_byte_count).expect("graph output should allocate");
        let mut graph_workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("graph workspace should allocate");
        let mut runner =
            CudaRowMajorCosetExtensionGraphRunner::new(column_count, source_bits, target_bits)
                .expect("graph runner should create");

        unsafe {
            runner
                .enqueue(&source, &mut graph_out, &mut graph_workspace)
                .expect("graph run should enqueue");
        }
        assert_eq!(runner.capture_count(), 1);
        assert_eq!(runner.launch_count(), 1);
        runner.synchronize().expect("graph stream should finish");

        assert_eq!(
            graph_out
                .to_u64_words()
                .expect("graph output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    #[test]
    fn strided_row_major_coset_extension_on_stream_matches_default_stream() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let source_row_stride = 5;
        let column_offset = 1;
        let column_count = 3;
        let values = (0..source_rows * source_row_stride)
            .map(|index| (index as u64 + 11) * 23)
            .collect::<Vec<_>>();
        let view = CudaRowMajorColumnView {
            source_rows,
            source_row_stride,
            column_offset,
            column_count,
        };
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            source_rows * column_count,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");

        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut default_out =
            CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_strided_device(
            &source,
            &mut default_out,
            view,
            source_bits,
            target_bits,
        )
        .expect("default strided extension should run");

        let mut stream_out =
            CudaDeviceBuffer::new(out_byte_count).expect("stream output should allocate");
        let mut workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("stream workspace should allocate");
        let stream = CudaStream::new().expect("CUDA stream should create");
        cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream(
            &source,
            &mut stream_out,
            &mut workspace,
            view,
            source_bits,
            target_bits,
            &stream,
        )
        .expect("explicit stream strided extension should enqueue");
        stream
            .synchronize()
            .expect("explicit stream strided extension should finish");

        assert_eq!(
            stream_out
                .to_u64_words()
                .expect("stream output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    #[test]
    fn strided_row_major_coset_extension_graph_runner_replays_same_buffers() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let source_row_stride = 6;
        let column_offset = 2;
        let column_count = 3;
        let mut values = (0..source_rows * source_row_stride)
            .map(|index| (index as u64 + 13) * 29)
            .collect::<Vec<_>>();
        let view = CudaRowMajorColumnView {
            source_rows,
            source_row_stride,
            column_offset,
            column_count,
        };
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            source_rows * column_count,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");

        let mut source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut graph_out =
            CudaDeviceBuffer::new(out_byte_count).expect("graph output should allocate");
        let mut graph_workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("graph workspace should allocate");
        let mut runner =
            CudaStridedRowMajorCosetExtensionGraphRunner::new(view, source_bits, target_bits)
                .expect("strided graph runner should create");

        runner
            .run(&source, &mut graph_out, &mut graph_workspace)
            .expect("first strided graph run should succeed");
        assert_eq!(runner.capture_count(), 1);
        assert_eq!(runner.launch_count(), 1);

        values.iter_mut().for_each(|value| *value += 101);
        source
            .copy_from_u64_words(&values)
            .expect("source should reupload");

        let mut default_out =
            CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_strided_device(
            &source,
            &mut default_out,
            view,
            source_bits,
            target_bits,
        )
        .expect("default strided extension should run");
        runner
            .run(&source, &mut graph_out, &mut graph_workspace)
            .expect("second strided graph run should succeed");

        assert_eq!(runner.capture_count(), 1);
        assert_eq!(runner.launch_count(), 2);
        assert_eq!(
            graph_out
                .to_u64_words()
                .expect("graph output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    #[test]
    fn strided_row_major_coset_extension_graph_runner_can_defer_synchronization() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let source_row_stride = 6;
        let column_offset = 1;
        let column_count = 4;
        let values = (0..source_rows * source_row_stride)
            .map(|index| (index as u64 + 7) * 37)
            .collect::<Vec<_>>();
        let view = CudaRowMajorColumnView {
            source_rows,
            source_row_stride,
            column_offset,
            column_count,
        };
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            source_rows * column_count,
            column_count,
            source_bits,
            target_bits,
        )
        .expect("output shape should be valid");

        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut default_out =
            CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_strided_device(
            &source,
            &mut default_out,
            view,
            source_bits,
            target_bits,
        )
        .expect("default strided extension should run");

        let mut graph_out =
            CudaDeviceBuffer::new(out_byte_count).expect("graph output should allocate");
        let mut graph_workspace =
            CudaDeviceBuffer::new(out_byte_count).expect("graph workspace should allocate");
        let mut runner =
            CudaStridedRowMajorCosetExtensionGraphRunner::new(view, source_bits, target_bits)
                .expect("strided graph runner should create");

        unsafe {
            runner
                .enqueue(&source, &mut graph_out, &mut graph_workspace)
                .expect("strided graph run should enqueue");
        }
        assert_eq!(runner.capture_count(), 1);
        assert_eq!(runner.launch_count(), 1);
        runner.synchronize().expect("graph stream should finish");

        assert_eq!(
            graph_out
                .to_u64_words()
                .expect("graph output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    #[test]
    fn column_major_digest_rounds_match_row_major_layout() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let row_count = 16;

        let width8 = 8;
        let column_count8 = 9;
        let current_words8 = (0..row_count * width8)
            .map(|index| (index as u64 + 3) * 17)
            .collect::<Vec<_>>();
        let row_words8 = (0..row_count * column_count8)
            .map(|index| (index as u64 + 5) * 19)
            .collect::<Vec<_>>();
        let column_words8 = transpose_row_major_words(&row_words8, row_count, column_count8);
        let current_states8 =
            CudaDeviceBuffer::from_u64_words(&current_words8).expect("states should upload");
        let row_values8 =
            CudaDeviceBuffer::from_u64_words(&row_words8).expect("row values should upload");
        let column_values8 =
            CudaDeviceBuffer::from_u64_words(&column_words8).expect("column values should upload");
        let byte_count8 = current_words8.len() * std::mem::size_of::<u64>();
        let mut row_out8 =
            CudaDeviceBuffer::zeroed(byte_count8).expect("row output should allocate");
        let mut column_out8 =
            CudaDeviceBuffer::zeroed(byte_count8).expect("column output should allocate");
        cuda_poseidon2_width8_linear_round_row_major_digest_device(
            &current_states8,
            &row_values8,
            &mut row_out8,
            column_count8,
            3,
            4,
        )
        .expect("row-major width-8 digest should run");
        cuda_poseidon2_width8_linear_round_column_major_digest_device(
            &current_states8,
            &column_values8,
            &mut column_out8,
            column_count8,
            3,
            4,
        )
        .expect("column-major width-8 digest should run");
        assert_eq!(
            column_out8
                .to_u64_words()
                .expect("column output should download"),
            row_out8.to_u64_words().expect("row output should download")
        );

        let width16 = 16;
        let column_count16 = 19;
        let current_words16 = (0..row_count * width16)
            .map(|index| (index as u64 + 7) * 23)
            .collect::<Vec<_>>();
        let row_words16 = (0..row_count * column_count16)
            .map(|index| (index as u64 + 11) * 29)
            .collect::<Vec<_>>();
        let column_words16 = transpose_row_major_words(&row_words16, row_count, column_count16);
        let current_states16 =
            CudaDeviceBuffer::from_u64_words(&current_words16).expect("states should upload");
        let row_values16 =
            CudaDeviceBuffer::from_u64_words(&row_words16).expect("row values should upload");
        let column_values16 =
            CudaDeviceBuffer::from_u64_words(&column_words16).expect("column values should upload");
        let byte_count16 = current_words16.len() * std::mem::size_of::<u64>();
        let mut row_out16 =
            CudaDeviceBuffer::zeroed(byte_count16).expect("row output should allocate");
        cuda_poseidon2_width16_linear_round_row_major_digest_device(
            &current_states16,
            &row_values16,
            &mut row_out16,
            column_count16,
            5,
            12,
        )
        .expect("row-major width-16 digest should run");
        let mut column_default_out16 =
            CudaDeviceBuffer::zeroed(byte_count16).expect("column output should allocate");
        cuda_poseidon2_width16_linear_round_column_major_digest_device(
            &current_states16,
            &column_values16,
            &mut column_default_out16,
            column_count16,
            5,
            12,
        )
        .expect("column-major width-16 digest should run");
        assert_eq!(
            column_default_out16
                .to_u64_words()
                .expect("column output should download"),
            row_out16
                .to_u64_words()
                .expect("row output should download")
        );
        let stream = CudaStream::new().expect("CUDA stream should create");
        let mut column_out16 =
            CudaDeviceBuffer::zeroed(byte_count16).expect("column output should allocate");
        unsafe {
            cuda_poseidon2_begin_width16_linear_round_column_major_digest_device_on_stream(
                &current_states16,
                &column_values16,
                &mut column_out16,
                column_count16,
                5,
                12,
                &stream,
            )
        }
        .expect("column-major width-16 digest should enqueue");
        stream
            .synchronize()
            .expect("column-major width-16 digest should finish");
        assert_eq!(
            column_out16
                .to_u64_words()
                .expect("column output should download"),
            row_out16
                .to_u64_words()
                .expect("row output should download")
        );
    }

    #[test]
    fn row_major_digest_on_stream_matches_default_stream() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let row_count = 16;
        let width = 16;
        let column_count = 19;
        let offset = 5;
        let chunk_len = 12;
        let current_words = (0..row_count * width)
            .map(|index| (index as u64 + 3) * 29)
            .collect::<Vec<_>>();
        let row_words = (0..row_count * column_count)
            .map(|index| (index as u64 + 7) * 31)
            .collect::<Vec<_>>();
        let current_states =
            CudaDeviceBuffer::from_u64_words(&current_words).expect("states should upload");
        let row_values =
            CudaDeviceBuffer::from_u64_words(&row_words).expect("row values should upload");
        let byte_count = current_words.len() * std::mem::size_of::<u64>();
        let mut default_out =
            CudaDeviceBuffer::zeroed(byte_count).expect("default output should allocate");
        cuda_poseidon2_width16_linear_round_row_major_digest_device(
            &current_states,
            &row_values,
            &mut default_out,
            column_count,
            offset,
            chunk_len,
        )
        .expect("default row-major digest round should launch");

        let stream = CudaStream::new().expect("CUDA stream should create");
        let mut stream_out =
            CudaDeviceBuffer::zeroed(byte_count).expect("stream output should allocate");
        unsafe {
            cuda_poseidon2_begin_width16_linear_round_row_major_digest_device_on_stream(
                &current_states,
                &row_values,
                &mut stream_out,
                column_count,
                offset,
                chunk_len,
                &stream,
            )
            .expect("stream row-major digest round should enqueue");
        }
        stream
            .synchronize()
            .expect("stream row-major digest round should finish");

        assert_eq!(
            stream_out
                .to_u64_words()
                .expect("stream output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }

    fn enqueue_row_major_extension_raw_on_stream(
        values: &CudaDeviceBuffer,
        out: &mut CudaDeviceBuffer,
        workspace: &mut CudaDeviceBuffer,
        column_count: usize,
        source_bits: usize,
        target_bits: usize,
        stream: &CudaStream,
    ) {
        let source_words = values.len() / 8;
        let source_rows = source_words / column_count;
        let (source_len, target_len, source_root, target_root) =
            coset_extend_domain(source_rows, source_bits, target_bits)
                .expect("domain shape should be valid");
        let code = unsafe {
            lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_on_stream_raw(
                values.as_raw_ptr() as *const u64,
                out.as_raw_ptr() as *mut u64,
                workspace.as_raw_ptr() as *mut u64,
                source_len,
                source_bits,
                target_len,
                target_bits,
                column_count,
                pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
                target_root,
                SHIFT,
                stream.as_raw(),
            )
        };
        cuda_status(code).expect("raw stream extension should enqueue");
    }

    #[test]
    fn stream_wait_event_orders_cross_stream_raw_extension() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let column_count = 3;
        let source_bits = 2;
        let mid_bits = 3;
        let target_bits = 4;
        let source_rows = 1_usize << source_bits;
        let values = (0..source_rows * column_count)
            .map(|index| (index as u64 + 5) * 31)
            .collect::<Vec<_>>();
        let mid_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            mid_bits,
        )
        .expect("mid shape should be valid");
        let target_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            (1_usize << mid_bits) * column_count,
            column_count,
            mid_bits,
            target_bits,
        )
        .expect("target shape should be valid");

        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let mut default_mid =
            CudaDeviceBuffer::new(mid_byte_count).expect("default mid should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source,
            &mut default_mid,
            column_count,
            source_bits,
            mid_bits,
        )
        .expect("default mid extension should run");
        let mut default_out =
            CudaDeviceBuffer::new(target_byte_count).expect("default output should allocate");
        cuda_goldilocks_coset_extend_row_major_columns_device(
            &default_mid,
            &mut default_out,
            column_count,
            mid_bits,
            target_bits,
        )
        .expect("default target extension should run");

        let producer = CudaStream::new().expect("producer stream should create");
        let consumer = CudaStream::new().expect("consumer stream should create");
        let ready = CudaEvent::new().expect("event should create");
        let mut stream_mid =
            CudaDeviceBuffer::new(mid_byte_count).expect("stream mid should allocate");
        let mut stream_mid_workspace =
            CudaDeviceBuffer::new(mid_byte_count).expect("stream mid workspace should allocate");
        enqueue_row_major_extension_raw_on_stream(
            &source,
            &mut stream_mid,
            &mut stream_mid_workspace,
            column_count,
            source_bits,
            mid_bits,
            &producer,
        );
        ready.record(&producer).expect("event should record");
        consumer
            .wait_event(&ready)
            .expect("consumer stream should wait for producer event");

        let mut stream_out =
            CudaDeviceBuffer::new(target_byte_count).expect("stream output should allocate");
        let mut stream_out_workspace = CudaDeviceBuffer::new(target_byte_count)
            .expect("stream output workspace should allocate");
        enqueue_row_major_extension_raw_on_stream(
            &stream_mid,
            &mut stream_out,
            &mut stream_out_workspace,
            column_count,
            mid_bits,
            target_bits,
            &consumer,
        );
        consumer
            .synchronize()
            .expect("consumer extension should finish");

        assert_eq!(
            stream_out
                .to_u64_words()
                .expect("stream output should download"),
            default_out
                .to_u64_words()
                .expect("default output should download")
        );
    }
}

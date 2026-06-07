#[cfg(feature = "cuda")]
use std::collections::HashMap;
#[cfg(feature = "cuda")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
#[cfg(feature = "cuda")]
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(feature = "cuda")]
use std::time::Instant;

#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_coset_extend_row_major_columns_device,
    cuda_goldilocks_coset_extend_row_major_columns_device_unsynced,
    cuda_goldilocks_coset_extend_row_major_columns_row_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced,
    cuda_goldilocks_coset_extend_row_major_columns_strided_row_device, cuda_memory_info,
    CudaDeviceBuffer, CudaRowMajorColumnView,
};
#[cfg(not(feature = "cuda"))]
use lzvm_field::coset_extend_evaluations;
use lzvm_field::Felt;

use super::{coset_extend_launch_work, errors::WitnessStageOpeningError, HASH_WORDS, WORD_BYTES};
#[cfg(feature = "cuda")]
use crate::gpu_setup::prepare_gpu_setup;
#[cfg(feature = "cuda")]
use crate::merkle_hash::{
    linear_hash_level_from_validated_row_major_device_buffer, CudaDigestCheckpointLevel,
    CudaDigestLevel,
};
use crate::merkle_hash::{linear_hashes_from_row_major_bytes, parent_levels_from_digest_level};

type CompactOnDemandOpening = (Vec<Felt>, Vec<Vec<[Felt; HASH_WORDS]>>);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
pub(crate) struct WitnessStageOpeningWorkTiming {
    pub(crate) setup: Duration,
    pub(crate) leaf_extend: Duration,
    pub(crate) leaf_hash: Duration,
    pub(crate) leaf_hash_rows: usize,
    pub(crate) leaf_hash_bytes: usize,
    pub(crate) leaf_hash_arity2_row_count: usize,
    pub(crate) leaf_hash_arity2_byte_count: usize,
    pub(crate) leaf_hash_arity4_row_count: usize,
    pub(crate) leaf_hash_arity4_byte_count: usize,
    pub(crate) leaf_coset_extend_call_count: usize,
    pub(crate) leaf_coset_extend_output_byte_count: usize,
    pub(crate) leaf_coset_extend_column_count: usize,
    pub(crate) leaf_coset_extend_max_column_count: usize,
    pub(crate) leaf_coset_extend_ntt_launch_count: usize,
    pub(crate) leaf_coset_extend_bit_reverse_launch_count: usize,
    pub(crate) leaf_coset_extend_ntt_stage_launch_count: usize,
    pub(crate) leaf_coset_extend_ntt_block_twiddle_launch_count: usize,
    pub(crate) leaf_coset_extend_normalize_launch_count: usize,
    pub(crate) leaf_coset_extend_pack_launch_count: usize,
    pub(crate) leaf_coset_extend_unpack_launch_count: usize,
    pub(crate) retained_leaf_digest_opening_count: usize,
    pub(crate) retained_leaf_digest_opening_row_count: usize,
    pub(crate) retained_parent_checkpoint_opening_count: usize,
    pub(crate) retained_parent_checkpoint_opening_row_count: usize,
    pub(crate) path_parent_hash_row_count: usize,
    pub(crate) path_parent_hash_byte_count: usize,
    pub(crate) path_parent_hash_launch_count: usize,
    pub(crate) row_values_device_row_count: usize,
    pub(crate) row_values_source_row_count: usize,
    pub(crate) row_values_word_count: usize,
    pub(crate) row_values_byte_count: usize,
    pub(crate) path: Duration,
    pub(crate) row_values: Duration,
}

impl WitnessStageOpeningWorkTiming {
    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn record_leaf_hash_work(
        &mut self,
        row_count: usize,
        byte_count: usize,
        arity: usize,
    ) {
        self.leaf_hash_rows += row_count;
        self.leaf_hash_bytes += byte_count;
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
    pub(crate) fn record_coset_extend_work(
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
    pub(crate) fn record_retained_leaf_digest_opening(&mut self, row_count: usize) {
        self.retained_leaf_digest_opening_count += 1;
        self.retained_leaf_digest_opening_row_count += row_count;
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn record_retained_parent_checkpoint_opening(&mut self, row_count: usize) {
        self.retained_parent_checkpoint_opening_count += 1;
        self.retained_parent_checkpoint_opening_row_count += row_count;
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn record_path_parent_hash_work(
        &mut self,
        row_count: usize,
        byte_count: usize,
        launch_count: usize,
    ) {
        self.path_parent_hash_row_count += row_count;
        self.path_parent_hash_byte_count += byte_count;
        self.path_parent_hash_launch_count += launch_count;
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn record_device_row_values(&mut self, row_count: usize, column_count: usize) {
        self.row_values_device_row_count += row_count;
        self.record_row_values(row_count, column_count);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn record_source_row_values(&mut self, row_count: usize, column_count: usize) {
        self.row_values_source_row_count += row_count;
        self.record_row_values(row_count, column_count);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    fn record_row_values(&mut self, row_count: usize, column_count: usize) {
        let word_count = row_count * column_count;
        self.row_values_word_count += word_count;
        self.row_values_byte_count += word_count * WORD_BYTES;
    }
}

#[cfg(feature = "cuda")]
fn merkle_opening_path_parent_work(
    mut state_count: usize,
    arity: usize,
) -> Option<(usize, usize, usize)> {
    if state_count <= 1 {
        return Some((0, 0, 0));
    }
    if arity <= 1 {
        return None;
    }

    let mut row_count = 0usize;
    let mut byte_count = 0usize;
    let mut launch_count = 0usize;
    while state_count > 1 {
        let parent_count = state_count.div_ceil(arity);
        let level_bytes = parent_count
            .checked_mul(arity)?
            .checked_mul(HASH_WORDS)?
            .checked_mul(WORD_BYTES)?;
        row_count = row_count.checked_add(parent_count)?;
        byte_count = byte_count.checked_add(level_bytes)?;
        launch_count = launch_count.checked_add(1)?;
        state_count = parent_count;
    }
    Some((row_count, byte_count, launch_count))
}

#[cfg(feature = "cuda")]
fn merkle_opening_path_prefix_parent_work(
    mut state_count: usize,
    arity: usize,
    prefix_level_count: usize,
) -> Option<(usize, usize, usize)> {
    if state_count <= 1 || prefix_level_count <= 1 {
        return Some((0, 0, 0));
    }
    if arity <= 1 {
        return None;
    }

    let mut row_count = 0usize;
    let mut byte_count = 0usize;
    let mut launch_count = 0usize;
    for _ in 1..prefix_level_count {
        if state_count <= 1 {
            break;
        }
        let parent_count = state_count.div_ceil(arity);
        let level_bytes = parent_count
            .checked_mul(arity)?
            .checked_mul(HASH_WORDS)?
            .checked_mul(WORD_BYTES)?;
        row_count = row_count.checked_add(parent_count)?;
        byte_count = byte_count.checked_add(level_bytes)?;
        launch_count = launch_count.checked_add(1)?;
        state_count = parent_count;
    }
    Some((row_count, byte_count, launch_count))
}

#[cfg(feature = "cuda")]
const DEFAULT_RETAINED_SOURCE_DEVICE_BYTES: usize = 10_000_000_000;
#[cfg(feature = "cuda")]
const RETAINED_SOURCE_DEVICE_RESERVE_BYTES: usize = 11 * 1024 * 1024 * 1024;
#[cfg(feature = "cuda")]
const RETAINED_COMBINED_DEVICE_CACHE_RESERVE_BYTES: usize = 10 * 1024 * 1024 * 1024;
#[cfg(feature = "cuda")]
const MAX_DEFAULT_RETAINED_SOURCE_DEVICE_BYTES: usize = DEFAULT_RETAINED_SOURCE_DEVICE_BYTES;
#[cfg(feature = "cuda")]
static RETAINED_SOURCE_DEVICE_BYTES: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "cuda")]
static RETAINED_SOURCE_DEVICE_LIMIT: OnceLock<usize> = OnceLock::new();
#[cfg(feature = "cuda")]
static RETAINED_COMBINED_DEVICE_CACHE_LIMIT: OnceLock<Option<usize>> = OnceLock::new();
#[cfg(feature = "cuda")]
static RETAINED_SOURCE_DEVICE_REGISTRY: OnceLock<Mutex<HashMap<usize, RetainedSourceDeviceEntry>>> =
    OnceLock::new();
#[cfg(feature = "cuda")]
const DEFAULT_RETAINED_LEAF_DIGEST_BYTES: usize = 14_000_000_000;
#[cfg(feature = "cuda")]
const RETAINED_LEAF_DIGEST_RESERVE_BYTES: usize = 12 * 1024 * 1024 * 1024;
#[cfg(feature = "cuda")]
const MAX_DEFAULT_RETAINED_LEAF_DIGEST_BYTES: usize = DEFAULT_RETAINED_LEAF_DIGEST_BYTES;
#[cfg(feature = "cuda")]
static RETAINED_LEAF_DIGEST_BYTES: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "cuda")]
static RETAINED_LEAF_DIGEST_LIMIT: OnceLock<usize> = OnceLock::new();

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) struct RetainedCudaSourceDevice {
    view: WitnessStageSourceDeviceView,
    key: usize,
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) struct RetainedCudaLeafDigestLevel {
    level: CudaDigestLevel,
    bytes: usize,
}

#[cfg(feature = "cuda")]
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub(crate) struct RetainedCudaParentCheckpointLevel {
    level: CudaDigestCheckpointLevel,
    bytes: usize,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy)]
struct RetainedSourceDeviceEntry {
    bytes: usize,
    refs: usize,
}

#[cfg(feature = "cuda")]
impl RetainedCudaSourceDevice {
    pub(crate) fn source_view(&self) -> &WitnessStageSourceDeviceView {
        &self.view
    }
}

#[cfg(feature = "cuda")]
impl Drop for RetainedCudaSourceDevice {
    fn drop(&mut self) {
        release_retained_device_buffer(self.key);
    }
}

#[cfg(feature = "cuda")]
impl RetainedCudaLeafDigestLevel {
    fn state_count(&self) -> usize {
        self.level.state_count()
    }

    fn arity(&self) -> usize {
        self.level.arity()
    }

    fn opening_path(
        &self,
        query_row: usize,
    ) -> Result<crate::merkle_hash::CudaMerkleOpeningPath, crate::merkle_hash::MerkleHashError>
    {
        self.level.opening_path(query_row)
    }
}

#[cfg(feature = "cuda")]
#[cfg_attr(not(test), allow(dead_code))]
impl RetainedCudaParentCheckpointLevel {
    fn source_state_count(&self) -> usize {
        self.level.source_state_count()
    }

    fn folded_level_count(&self) -> usize {
        self.level.folded_level_count()
    }

    fn state_count(&self) -> usize {
        self.level.state_count()
    }

    fn arity(&self) -> usize {
        self.level.arity()
    }

    fn opening_path_for_source_row(
        &self,
        source_row: usize,
    ) -> Result<crate::merkle_hash::CudaMerkleOpeningPath, crate::merkle_hash::MerkleHashError>
    {
        self.level.opening_path_for_source_row(source_row)
    }
}

#[cfg(feature = "cuda")]
impl Drop for RetainedCudaLeafDigestLevel {
    fn drop(&mut self) {
        release_retained_leaf_digest_bytes(self.bytes);
    }
}

#[cfg(feature = "cuda")]
impl Drop for RetainedCudaParentCheckpointLevel {
    fn drop(&mut self) {
        release_retained_leaf_digest_bytes(self.bytes);
    }
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
pub(crate) struct WitnessStageSourceDeviceView {
    buffer: Arc<CudaDeviceBuffer>,
    row_count: usize,
    column_count: usize,
    row_stride: usize,
    column_offset: usize,
}

#[cfg(feature = "cuda")]
impl WitnessStageSourceDeviceView {
    pub(crate) fn new(
        row_count: usize,
        column_count: usize,
        row_stride: usize,
        column_offset: usize,
        buffer: Arc<CudaDeviceBuffer>,
    ) -> Self {
        Self {
            buffer,
            row_count,
            column_count,
            row_stride,
            column_offset,
        }
    }

    pub(crate) fn buffer(&self) -> &CudaDeviceBuffer {
        self.buffer.as_ref()
    }

    fn buffer_key(&self) -> usize {
        Arc::as_ptr(&self.buffer) as usize
    }

    pub(crate) fn retained_byte_len(&self) -> usize {
        self.buffer().len()
    }

    pub(crate) fn row_stride(&self) -> usize {
        self.row_stride
    }

    pub(crate) fn column_offset(&self) -> usize {
        self.column_offset
    }

    pub(crate) fn logical_byte_len(&self) -> Option<usize> {
        self.row_count
            .checked_mul(self.column_count)?
            .checked_mul(WORD_BYTES)
    }

    pub(crate) fn required_byte_len(&self) -> Option<usize> {
        if self.row_count == 0 {
            return Some(0);
        }
        let end_word = self
            .row_count
            .checked_sub(1)?
            .checked_mul(self.row_stride)?
            .checked_add(self.column_offset)?
            .checked_add(self.column_count)?;
        end_word.checked_mul(WORD_BYTES)
    }

    pub(crate) fn has_matching_shape(&self, row_count: usize, column_count: usize) -> bool {
        self.row_count == row_count && self.column_count == column_count
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn retain_source_device_view(
    view: WitnessStageSourceDeviceView,
) -> Option<Arc<RetainedCudaSourceDevice>> {
    let key = view.buffer_key();
    let bytes = view.retained_byte_len();
    reserve_retained_device_buffer(key, bytes)?;
    Some(Arc::new(RetainedCudaSourceDevice { view, key }))
}

#[cfg(feature = "cuda")]
pub(crate) fn retain_leaf_digest_level(
    level: CudaDigestLevel,
    column_count: usize,
) -> Option<Arc<RetainedCudaLeafDigestLevel>> {
    if column_count <= HASH_WORDS {
        return None;
    }
    let bytes = level.byte_len();
    reserve_retained_leaf_digest_bytes(bytes)?;
    Some(Arc::new(RetainedCudaLeafDigestLevel { level, bytes }))
}

#[cfg(feature = "cuda")]
pub(crate) fn retain_parent_checkpoint_level(
    checkpoint: Option<CudaDigestCheckpointLevel>,
    column_count: usize,
) -> Option<Arc<RetainedCudaParentCheckpointLevel>> {
    if column_count <= HASH_WORDS {
        return None;
    }
    let level = checkpoint?;
    let bytes = level.byte_len();
    reserve_retained_leaf_digest_bytes(bytes)?;
    Some(Arc::new(RetainedCudaParentCheckpointLevel { level, bytes }))
}

#[cfg(feature = "cuda")]
fn retained_source_device_registry() -> &'static Mutex<HashMap<usize, RetainedSourceDeviceEntry>> {
    RETAINED_SOURCE_DEVICE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(feature = "cuda")]
fn reserve_retained_device_buffer(key: usize, bytes: usize) -> Option<()> {
    let registry = retained_source_device_registry();
    let mut registry = registry.lock().ok()?;
    if let Some(entry) = registry.get_mut(&key) {
        entry.refs = entry.refs.checked_add(1)?;
        return Some(());
    }
    reserve_retained_device_bytes(bytes)?;
    registry.insert(key, RetainedSourceDeviceEntry { bytes, refs: 1 });
    Some(())
}

#[cfg(feature = "cuda")]
fn release_retained_device_buffer(key: usize) {
    let registry = retained_source_device_registry();
    let Ok(mut registry) = registry.lock() else {
        return;
    };
    let Some(entry) = registry.get_mut(&key) else {
        return;
    };
    entry.refs = entry.refs.saturating_sub(1);
    if entry.refs == 0 {
        let bytes = entry.bytes;
        registry.remove(&key);
        release_retained_device_bytes(bytes);
    }
}

#[cfg(feature = "cuda")]
fn reserve_retained_device_bytes(bytes: usize) -> Option<()> {
    if bytes == 0 {
        return Some(());
    }
    let limit = retained_source_device_limit();
    if bytes > limit {
        return None;
    }
    let mut current = RETAINED_SOURCE_DEVICE_BYTES.load(Ordering::Acquire);
    loop {
        let next = current.checked_add(bytes)?;
        if next > limit {
            return None;
        }
        let leaf_bytes = RETAINED_LEAF_DIGEST_BYTES.load(Ordering::Acquire);
        retained_combined_device_cache_allows(next, leaf_bytes)?;
        match RETAINED_SOURCE_DEVICE_BYTES.compare_exchange_weak(
            current,
            next,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return Some(()),
            Err(observed) => current = observed,
        }
    }
}

#[cfg(feature = "cuda")]
fn release_retained_device_bytes(bytes: usize) {
    RETAINED_SOURCE_DEVICE_BYTES.fetch_sub(bytes, Ordering::AcqRel);
}

#[cfg(feature = "cuda")]
fn reserve_retained_leaf_digest_bytes(bytes: usize) -> Option<()> {
    if bytes == 0 {
        return Some(());
    }
    let limit = retained_leaf_digest_limit();
    if bytes > limit {
        return None;
    }
    let mut current = RETAINED_LEAF_DIGEST_BYTES.load(Ordering::Acquire);
    loop {
        let next = current.checked_add(bytes)?;
        if next > limit {
            return None;
        }
        let source_bytes = RETAINED_SOURCE_DEVICE_BYTES.load(Ordering::Acquire);
        retained_combined_device_cache_allows(source_bytes, next)?;
        match RETAINED_LEAF_DIGEST_BYTES.compare_exchange_weak(
            current,
            next,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return Some(()),
            Err(observed) => current = observed,
        }
    }
}

#[cfg(feature = "cuda")]
fn release_retained_leaf_digest_bytes(bytes: usize) {
    RETAINED_LEAF_DIGEST_BYTES.fetch_sub(bytes, Ordering::AcqRel);
}

#[cfg(feature = "cuda")]
pub(crate) fn retained_source_device_limit() -> usize {
    std::env::var("LZVM_CUDA_RETAINED_SOURCE_BYTES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_else(|| {
            *RETAINED_SOURCE_DEVICE_LIMIT.get_or_init(default_retained_source_device_limit)
        })
}

#[cfg(feature = "cuda")]
fn default_retained_source_device_limit() -> usize {
    cuda_memory_info()
        .ok()
        .map(|info| {
            info.total_bytes
                .saturating_sub(RETAINED_SOURCE_DEVICE_RESERVE_BYTES)
                .min(MAX_DEFAULT_RETAINED_SOURCE_DEVICE_BYTES)
        })
        .filter(|limit| *limit > 0)
        .unwrap_or(DEFAULT_RETAINED_SOURCE_DEVICE_BYTES)
}

#[cfg(feature = "cuda")]
fn retained_leaf_digest_limit() -> usize {
    std::env::var("LZVM_CUDA_RETAINED_LEAF_DIGEST_BYTES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_else(|| {
            *RETAINED_LEAF_DIGEST_LIMIT.get_or_init(default_retained_leaf_digest_limit)
        })
}

#[cfg(feature = "cuda")]
fn default_retained_leaf_digest_limit() -> usize {
    cuda_memory_info()
        .ok()
        .map(|info| {
            info.total_bytes
                .saturating_sub(RETAINED_LEAF_DIGEST_RESERVE_BYTES)
                .min(MAX_DEFAULT_RETAINED_LEAF_DIGEST_BYTES)
        })
        .filter(|limit| *limit > 0)
        .unwrap_or(DEFAULT_RETAINED_LEAF_DIGEST_BYTES)
}

#[cfg(feature = "cuda")]
fn retained_combined_device_cache_limit() -> Option<usize> {
    *RETAINED_COMBINED_DEVICE_CACHE_LIMIT.get_or_init(|| {
        cuda_memory_info()
            .ok()
            .map(|info| {
                info.total_bytes
                    .saturating_sub(RETAINED_COMBINED_DEVICE_CACHE_RESERVE_BYTES)
            })
            .filter(|limit| *limit > 0)
    })
}

#[cfg(feature = "cuda")]
fn retained_combined_device_cache_allows(source_bytes: usize, leaf_bytes: usize) -> Option<()> {
    if let Some(limit) = retained_combined_device_cache_limit() {
        let combined = source_bytes.checked_add(leaf_bytes)?;
        if combined > limit {
            return None;
        }
    }
    Some(())
}

#[cfg(feature = "cuda")]
enum SourceDeviceBuffer<'a> {
    Borrowed(&'a WitnessStageSourceDeviceView),
    Owned {
        buffer: CudaDeviceBuffer,
        row_stride: usize,
        column_offset: usize,
    },
}

#[cfg(feature = "cuda")]
impl<'a> SourceDeviceBuffer<'a> {
    fn as_buffer(&self) -> &CudaDeviceBuffer {
        match self {
            Self::Borrowed(view) => view.buffer(),
            Self::Owned { buffer, .. } => buffer,
        }
    }

    fn row_stride(&self) -> usize {
        match self {
            Self::Borrowed(view) => view.row_stride(),
            Self::Owned { row_stride, .. } => *row_stride,
        }
    }

    fn column_offset(&self) -> usize {
        match self {
            Self::Borrowed(view) => view.column_offset(),
            Self::Owned { column_offset, .. } => *column_offset,
        }
    }

    fn is_compact_for(&self, columns: usize) -> bool {
        self.row_stride() == columns && self.column_offset() == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessStageLeaves {
    stage_index: usize,
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    bytes: Vec<u8>,
}

impl WitnessStageLeaves {
    pub(crate) fn new(
        stage_index: usize,
        source_rows: usize,
        extended_rows: usize,
        columns: usize,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            stage_index,
            source_rows,
            extended_rows,
            columns,
            bytes,
        }
    }

    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn source_row_count(&self) -> usize {
        self.source_rows
    }

    pub fn extended_row_count(&self) -> usize {
        self.extended_rows
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug, Clone)]
pub struct WitnessStageCommitment {
    stage_index: usize,
    arity: usize,
    root: [Felt; HASH_WORDS],
    tree: WitnessStageTreeStorage,
}

#[derive(Debug, Clone)]
pub(crate) struct WitnessStageCompactTreeParts {
    pub(crate) source_rows: usize,
    pub(crate) extended_rows: usize,
    pub(crate) columns: usize,
    pub(crate) source_bits: usize,
    pub(crate) target_bits: usize,
    pub(crate) arity: usize,
    pub(crate) source_values: Vec<Felt>,
    pub(crate) raw_leaf_bytes: usize,
    pub(crate) logical_tree_bytes: usize,
    pub(crate) digest_tree: Option<Vec<u8>>,
    pub(crate) external_source_required: bool,
    #[cfg(feature = "cuda")]
    pub(crate) retained_source_device: Option<Arc<RetainedCudaSourceDevice>>,
    #[cfg(feature = "cuda")]
    pub(crate) retained_leaf_digest_level: Option<Arc<RetainedCudaLeafDigestLevel>>,
    #[cfg(feature = "cuda")]
    pub(crate) retained_parent_checkpoint_level: Option<Arc<RetainedCudaParentCheckpointLevel>>,
}

#[derive(Debug, Clone)]
enum WitnessStageTreeStorage {
    Host(Vec<u8>),
    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    Compact(Box<WitnessStageCompactTreeStorage>),
}

#[derive(Debug)]
#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
struct WitnessStageCompactTreeStorage {
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_values: Vec<Felt>,
    raw_leaf_bytes: usize,
    logical_tree_bytes: usize,
    digest_tree: Option<Vec<u8>>,
    external_source_required: bool,
    #[cfg(feature = "cuda")]
    retained_source_device: Option<Arc<RetainedCudaSourceDevice>>,
    #[cfg(feature = "cuda")]
    retained_leaf_digest_level: Option<Arc<RetainedCudaLeafDigestLevel>>,
    #[cfg(feature = "cuda")]
    retained_parent_checkpoint_level: Option<Arc<RetainedCudaParentCheckpointLevel>>,
    materialized_tree: OnceLock<Vec<u8>>,
}

impl Clone for WitnessStageCompactTreeStorage {
    fn clone(&self) -> Self {
        let materialized_tree = OnceLock::new();
        if let Some(bytes) = self.materialized_tree.get() {
            let _ = materialized_tree.set(bytes.clone());
        }
        Self {
            source_rows: self.source_rows,
            extended_rows: self.extended_rows,
            columns: self.columns,
            source_bits: self.source_bits,
            target_bits: self.target_bits,
            arity: self.arity,
            source_values: self.source_values.clone(),
            raw_leaf_bytes: self.raw_leaf_bytes,
            logical_tree_bytes: self.logical_tree_bytes,
            digest_tree: self.digest_tree.clone(),
            external_source_required: self.external_source_required,
            #[cfg(feature = "cuda")]
            retained_source_device: self.retained_source_device.clone(),
            #[cfg(feature = "cuda")]
            retained_leaf_digest_level: self.retained_leaf_digest_level.clone(),
            #[cfg(feature = "cuda")]
            retained_parent_checkpoint_level: self.retained_parent_checkpoint_level.clone(),
            materialized_tree,
        }
    }
}

impl WitnessStageCommitment {
    pub(crate) fn new(
        stage_index: usize,
        arity: usize,
        root: [Felt; HASH_WORDS],
        tree_bytes: Vec<u8>,
    ) -> Self {
        Self {
            stage_index,
            arity,
            root,
            tree: WitnessStageTreeStorage::Host(tree_bytes),
        }
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn new_compact(
        stage_index: usize,
        arity: usize,
        root: [Felt; HASH_WORDS],
        parts: WitnessStageCompactTreeParts,
    ) -> Self {
        Self {
            stage_index,
            arity,
            root,
            tree: WitnessStageTreeStorage::Compact(Box::new(WitnessStageCompactTreeStorage {
                source_rows: parts.source_rows,
                extended_rows: parts.extended_rows,
                columns: parts.columns,
                source_bits: parts.source_bits,
                target_bits: parts.target_bits,
                arity: parts.arity,
                source_values: parts.source_values,
                raw_leaf_bytes: parts.raw_leaf_bytes,
                logical_tree_bytes: parts.logical_tree_bytes,
                digest_tree: parts.digest_tree,
                external_source_required: parts.external_source_required,
                #[cfg(feature = "cuda")]
                retained_source_device: parts.retained_source_device,
                #[cfg(feature = "cuda")]
                retained_leaf_digest_level: parts.retained_leaf_digest_level,
                #[cfg(feature = "cuda")]
                retained_parent_checkpoint_level: parts.retained_parent_checkpoint_level,
                materialized_tree: OnceLock::new(),
            })),
        }
    }

    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn arity(&self) -> usize {
        self.arity
    }

    pub fn root(&self) -> [Felt; HASH_WORDS] {
        self.root
    }

    pub fn tree_bytes(&self) -> &[u8] {
        match &self.tree {
            WitnessStageTreeStorage::Host(bytes) => bytes,
            WitnessStageTreeStorage::Compact(storage) => storage.materialized_tree_bytes(),
        }
    }

    pub fn tree_byte_count(&self) -> usize {
        match &self.tree {
            WitnessStageTreeStorage::Host(bytes) => bytes.len(),
            WitnessStageTreeStorage::Compact(storage) => storage.logical_tree_bytes,
        }
    }

    #[cfg(all(test, feature = "cuda"))]
    pub(crate) fn retained_parent_checkpoint_shape_for_test(
        &self,
    ) -> Option<(usize, usize, usize, usize)> {
        match &self.tree {
            WitnessStageTreeStorage::Host(_) => None,
            WitnessStageTreeStorage::Compact(storage) => storage
                .retained_parent_checkpoint_level
                .as_ref()
                .map(|checkpoint| {
                    (
                        checkpoint.source_state_count(),
                        checkpoint.folded_level_count(),
                        checkpoint.state_count(),
                        checkpoint.arity(),
                    )
                }),
        }
    }

    #[cfg(all(test, feature = "cuda"))]
    pub(crate) fn retained_parent_checkpoint_opening_suffix_for_test(
        &self,
        source_row: usize,
    ) -> Result<Vec<Vec<[Felt; HASH_WORDS]>>, WitnessStageOpeningError> {
        match &self.tree {
            WitnessStageTreeStorage::Host(_) => Err(WitnessStageOpeningError::LengthOverflow),
            WitnessStageTreeStorage::Compact(storage) => {
                let checkpoint = storage
                    .retained_parent_checkpoint_level
                    .as_ref()
                    .ok_or(WitnessStageOpeningError::LengthOverflow)?;
                let path = checkpoint
                    .opening_path_for_source_row(source_row)
                    .map_err(WitnessStageOpeningError::from)?;
                Ok(path.siblings)
            }
        }
    }

    #[cfg(all(test, feature = "cuda"))]
    pub(crate) fn drop_retained_leaf_digest_level_for_test(&mut self) -> bool {
        match &mut self.tree {
            WitnessStageTreeStorage::Host(_) => false,
            WitnessStageTreeStorage::Compact(storage) => {
                storage.retained_leaf_digest_level.take().is_some()
            }
        }
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn requires_external_source(&self) -> bool {
        match &self.tree {
            WitnessStageTreeStorage::Host(_) => false,
            WitnessStageTreeStorage::Compact(storage) => storage.external_source_required,
        }
    }

    pub(crate) fn read_opening_values(
        &self,
        row_offset: usize,
        row_byte_count: usize,
    ) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        let end = row_offset
            .checked_add(row_byte_count)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let row = match &self.tree {
            WitnessStageTreeStorage::Host(tree_bytes) => tree_bytes.get(row_offset..end).ok_or(
                WitnessStageOpeningError::InvalidTreeByteLength {
                    expected: end,
                    found: self.tree_byte_count(),
                },
            )?,
            WitnessStageTreeStorage::Compact(storage) => {
                return storage.read_opening_values(row_offset, row_byte_count);
            }
        };
        row.chunks_exact(WORD_BYTES)
            .map(|chunk| {
                let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
                Felt::from_canonical(value).map_err(WitnessStageOpeningError::Field)
            })
            .collect()
    }

    #[cfg(not(feature = "cuda"))]
    pub(crate) fn open_compact_on_demand(
        &self,
        row_index: usize,
        row_count: usize,
        column_count: usize,
    ) -> Result<Option<CompactOnDemandOpening>, WitnessStageOpeningError> {
        match &self.tree {
            WitnessStageTreeStorage::Host(_) => Ok(None),
            WitnessStageTreeStorage::Compact(storage) => storage.open_compact_on_demand(
                row_index,
                row_count,
                column_count,
                self.arity,
                self.root,
            ),
        }
    }

    #[cfg(not(feature = "cuda"))]
    pub(crate) fn open_compact_batch_on_demand(
        &self,
        row_indices: &[usize],
        row_count: usize,
        column_count: usize,
    ) -> Result<Option<Vec<CompactOnDemandOpening>>, WitnessStageOpeningError> {
        match &self.tree {
            WitnessStageTreeStorage::Host(_) => Ok(None),
            WitnessStageTreeStorage::Compact(storage) => storage.open_compact_batch_on_demand(
                row_indices,
                row_count,
                column_count,
                self.arity,
                self.root,
            ),
        }
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn open_compact_on_demand_with_source_device(
        &self,
        row_index: usize,
        row_count: usize,
        column_count: usize,
        source_device: Option<&WitnessStageSourceDeviceView>,
        timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<Option<CompactOnDemandOpening>, WitnessStageOpeningError> {
        match &self.tree {
            WitnessStageTreeStorage::Host(_) => Ok(None),
            WitnessStageTreeStorage::Compact(storage) => storage
                .open_compact_on_demand_with_source_device(
                    row_index,
                    row_count,
                    column_count,
                    self.arity,
                    self.root,
                    source_device,
                    timing,
                ),
        }
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn open_compact_batch_on_demand_with_source_device(
        &self,
        row_indices: &[usize],
        row_count: usize,
        column_count: usize,
        source_device: Option<&WitnessStageSourceDeviceView>,
        timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<Option<Vec<CompactOnDemandOpening>>, WitnessStageOpeningError> {
        match &self.tree {
            WitnessStageTreeStorage::Host(_) => Ok(None),
            WitnessStageTreeStorage::Compact(storage) => storage
                .open_compact_batch_on_demand_with_source_device(
                    row_indices,
                    row_count,
                    column_count,
                    self.arity,
                    self.root,
                    source_device,
                    timing,
                ),
        }
    }

    pub(crate) fn read_digest_at(
        &self,
        level_offset: usize,
        index: usize,
    ) -> Result<[Felt; HASH_WORDS], WitnessStageOpeningError> {
        let digest_offset = index
            .checked_mul(HASH_WORDS * WORD_BYTES)
            .and_then(|offset| offset.checked_add(level_offset))
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let digest_end = digest_offset
            .checked_add(HASH_WORDS * WORD_BYTES)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let digest_bytes = match &self.tree {
            WitnessStageTreeStorage::Host(tree_bytes) => tree_bytes.get(digest_offset..digest_end),
            WitnessStageTreeStorage::Compact(storage) => {
                storage.read_digest_bytes(digest_offset, digest_end)
            }
        }
        .ok_or(WitnessStageOpeningError::InvalidTreeByteLength {
            expected: digest_end,
            found: self.tree_byte_count(),
        })?;
        let mut digest = [Felt::ZERO; HASH_WORDS];
        for (word, chunk) in digest.iter_mut().zip(digest_bytes.chunks_exact(WORD_BYTES)) {
            let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
            *word = Felt::from_canonical(value)?;
        }
        Ok(digest)
    }
}

impl PartialEq for WitnessStageCommitment {
    fn eq(&self, other: &Self) -> bool {
        self.stage_index == other.stage_index
            && self.arity == other.arity
            && self.root == other.root
            && self.tree_byte_count() == other.tree_byte_count()
            && self.tree_bytes() == other.tree_bytes()
    }
}

impl Eq for WitnessStageCommitment {}

impl WitnessStageCompactTreeStorage {
    fn read_opening_values(
        &self,
        row_offset: usize,
        row_byte_count: usize,
    ) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        if row_byte_count != self.columns * WORD_BYTES || !row_offset.is_multiple_of(row_byte_count)
        {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        let row = row_offset / row_byte_count;
        if row >= self.extended_rows {
            return Err(WitnessStageOpeningError::InvalidTreeByteLength {
                expected: row_offset + row_byte_count,
                found: self.logical_tree_bytes,
            });
        }
        self.extended_row_values(row)
    }

    fn read_digest_bytes(&self, digest_offset: usize, digest_end: usize) -> Option<&[u8]> {
        if digest_offset < self.raw_leaf_bytes || digest_end < self.raw_leaf_bytes {
            return None;
        }
        let start = digest_offset - self.raw_leaf_bytes;
        let end = digest_end - self.raw_leaf_bytes;
        self.digest_tree.as_ref()?.get(start..end)
    }

    fn materialized_tree_bytes(&self) -> &[u8] {
        self.materialized_tree
            .get_or_init(|| {
                let mut bytes = self
                    .extended_leaf_bytes()
                    .expect("compact witness leaves should materialize");
                let digest_tree = self
                    .materialized_digest_tree_bytes(&bytes)
                    .expect("compact witness digest tree should materialize");
                bytes.extend_from_slice(&digest_tree);
                bytes
            })
            .as_slice()
    }

    fn materialized_digest_tree_bytes(
        &self,
        leaf_bytes: &[u8],
    ) -> Result<Vec<u8>, WitnessStageOpeningError> {
        if let Some(bytes) = &self.digest_tree {
            return Ok(bytes.clone());
        }
        let leaf_hashes = linear_hashes_from_row_major_bytes(
            leaf_bytes,
            self.extended_rows,
            self.columns,
            self.arity,
        )?;
        materialize_digest_tree_bytes(leaf_hashes, self.arity)
    }

    #[cfg(not(feature = "cuda"))]
    fn open_compact_on_demand(
        &self,
        row_index: usize,
        row_count: usize,
        column_count: usize,
        arity: usize,
        expected_root: [Felt; HASH_WORDS],
    ) -> Result<Option<CompactOnDemandOpening>, WitnessStageOpeningError> {
        self.open_compact_on_demand_with_source_device(
            row_index,
            row_count,
            column_count,
            arity,
            expected_root,
            #[cfg(feature = "cuda")]
            None,
            #[cfg(feature = "cuda")]
            None,
        )
    }

    #[cfg(not(feature = "cuda"))]
    fn open_compact_batch_on_demand(
        &self,
        row_indices: &[usize],
        row_count: usize,
        column_count: usize,
        arity: usize,
        expected_root: [Felt; HASH_WORDS],
    ) -> Result<Option<Vec<CompactOnDemandOpening>>, WitnessStageOpeningError> {
        self.open_compact_batch_on_demand_with_source_device(
            row_indices,
            row_count,
            column_count,
            arity,
            expected_root,
            #[cfg(feature = "cuda")]
            None,
            #[cfg(feature = "cuda")]
            None,
        )
    }

    #[cfg_attr(feature = "cuda", allow(clippy::too_many_arguments))]
    fn open_compact_on_demand_with_source_device(
        &self,
        row_index: usize,
        row_count: usize,
        column_count: usize,
        arity: usize,
        expected_root: [Felt; HASH_WORDS],
        #[cfg(feature = "cuda")] source_device: Option<&WitnessStageSourceDeviceView>,
        #[cfg(feature = "cuda")] timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<Option<CompactOnDemandOpening>, WitnessStageOpeningError> {
        let should_open_on_demand = self.digest_tree.is_none();
        if !should_open_on_demand {
            return Ok(None);
        }
        if row_count != self.extended_rows || column_count != self.columns || arity != self.arity {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }

        #[cfg(feature = "cuda")]
        {
            self.open_on_demand_cuda(row_index, expected_root, source_device, timing)
                .map(Some)
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = row_index;
            let _ = expected_root;
            Err(WitnessStageOpeningError::LengthOverflow)
        }
    }

    #[cfg_attr(feature = "cuda", allow(clippy::too_many_arguments))]
    fn open_compact_batch_on_demand_with_source_device(
        &self,
        row_indices: &[usize],
        row_count: usize,
        column_count: usize,
        arity: usize,
        expected_root: [Felt; HASH_WORDS],
        #[cfg(feature = "cuda")] source_device: Option<&WitnessStageSourceDeviceView>,
        #[cfg(feature = "cuda")] timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<Option<Vec<CompactOnDemandOpening>>, WitnessStageOpeningError> {
        let should_open_on_demand = self.digest_tree.is_none();
        if !should_open_on_demand {
            return Ok(None);
        }
        if row_count != self.extended_rows || column_count != self.columns || arity != self.arity {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }

        #[cfg(feature = "cuda")]
        {
            self.open_batch_on_demand_cuda(row_indices, expected_root, source_device, timing)
                .map(Some)
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = row_indices;
            let _ = expected_root;
            Err(WitnessStageOpeningError::LengthOverflow)
        }
    }

    fn extended_row_values(&self, row: usize) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        #[cfg(feature = "cuda")]
        {
            self.extended_row_values_cuda(row)
        }
        #[cfg(not(feature = "cuda"))]
        {
            self.extended_row_values_cpu(row)
        }
    }

    #[cfg(not(feature = "cuda"))]
    fn extended_row_values_cpu(&self, row: usize) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        let mut out = Vec::with_capacity(self.columns);
        for column in 0..self.columns {
            let source = self.source_column_values(column)?;
            let extended = coset_extend_evaluations(&source, self.source_bits, self.target_bits)
                .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
            out.push(extended[row]);
        }
        Ok(out)
    }

    fn extended_leaf_bytes(&self) -> Result<Vec<u8>, WitnessStageOpeningError> {
        #[cfg(feature = "cuda")]
        {
            self.extended_leaf_bytes_cuda()
        }
        #[cfg(not(feature = "cuda"))]
        {
            self.extended_leaf_bytes_cpu()
        }
    }

    #[cfg(not(feature = "cuda"))]
    fn extended_leaf_bytes_cpu(&self) -> Result<Vec<u8>, WitnessStageOpeningError> {
        let mut extended_columns = Vec::with_capacity(self.columns);
        for column in 0..self.columns {
            let source = self.source_column_values(column)?;
            extended_columns.push(
                coset_extend_evaluations(&source, self.source_bits, self.target_bits)
                    .map_err(|_| WitnessStageOpeningError::LengthOverflow)?,
            );
        }
        let mut bytes = Vec::with_capacity(self.raw_leaf_bytes);
        for row in 0..self.extended_rows {
            for column_values in &extended_columns {
                bytes.extend_from_slice(&column_values[row].to_le_bytes());
            }
        }
        Ok(bytes)
    }

    #[cfg(feature = "cuda")]
    fn expected_source_value_count(&self) -> Result<usize, WitnessStageOpeningError> {
        self.source_rows
            .checked_mul(self.columns)
            .ok_or(WitnessStageOpeningError::LengthOverflow)
    }

    #[cfg(feature = "cuda")]
    fn source_device_buffer<'a>(
        &'a self,
        source_device: Option<&'a WitnessStageSourceDeviceView>,
    ) -> Result<SourceDeviceBuffer<'a>, WitnessStageOpeningError> {
        let expected_source_values = self.expected_source_value_count()?;
        let expected_source_bytes = expected_source_values
            .checked_mul(WORD_BYTES)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        if let Some(retained_source_device) = &self.retained_source_device {
            let view = retained_source_device.source_view();
            let Some(required_source_bytes) = view.required_byte_len() else {
                return Err(WitnessStageOpeningError::LengthOverflow);
            };
            let Some(source_column_end) = view.column_offset().checked_add(self.columns) else {
                return Err(WitnessStageOpeningError::LengthOverflow);
            };
            if !view.has_matching_shape(self.source_rows, self.columns)
                || view.row_stride() < self.columns
                || source_column_end > view.row_stride()
                || view.buffer().len() < required_source_bytes
            {
                return Err(WitnessStageOpeningError::LengthOverflow);
            }
            return Ok(SourceDeviceBuffer::Borrowed(view));
        }
        if let Some(view) = source_device {
            let Some(required_source_bytes) = view.required_byte_len() else {
                return Err(WitnessStageOpeningError::LengthOverflow);
            };
            let Some(source_column_end) = view.column_offset().checked_add(self.columns) else {
                return Err(WitnessStageOpeningError::LengthOverflow);
            };
            if !view.has_matching_shape(self.source_rows, self.columns)
                || view.row_stride() < self.columns
                || source_column_end > view.row_stride()
                || view.buffer().len() < required_source_bytes
            {
                return Err(WitnessStageOpeningError::LengthOverflow);
            }
            return Ok(SourceDeviceBuffer::Borrowed(view));
        }
        if self.external_source_required {
            return Err(WitnessStageOpeningError::ExternalSourceUnavailable);
        }
        if self.source_values.len() != expected_source_values {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        let buffer = CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&self.source_values))
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        if buffer.len() != expected_source_bytes {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        Ok(SourceDeviceBuffer::Owned {
            buffer,
            row_stride: self.columns,
            column_offset: 0,
        })
    }

    #[cfg(feature = "cuda")]
    fn extend_source_device_buffer_cuda(
        &self,
        source_buffer: &SourceDeviceBuffer<'_>,
        output_buffer: &mut CudaDeviceBuffer,
    ) -> Result<(), WitnessStageOpeningError> {
        if source_buffer.is_compact_for(self.columns) {
            cuda_goldilocks_coset_extend_row_major_columns_device(
                source_buffer.as_buffer(),
                output_buffer,
                self.columns,
                self.source_bits,
                self.target_bits,
            )
        } else {
            cuda_goldilocks_coset_extend_row_major_columns_strided_device(
                source_buffer.as_buffer(),
                output_buffer,
                CudaRowMajorColumnView {
                    source_rows: self.source_rows,
                    source_row_stride: source_buffer.row_stride(),
                    column_offset: source_buffer.column_offset(),
                    column_count: self.columns,
                },
                self.source_bits,
                self.target_bits,
            )
        }
        .map_err(|_| WitnessStageOpeningError::LengthOverflow)
    }

    #[cfg(feature = "cuda")]
    fn extend_source_device_buffer_cuda_unsynced(
        &self,
        source_buffer: &SourceDeviceBuffer<'_>,
        output_buffer: &mut CudaDeviceBuffer,
    ) -> Result<CudaDeviceBuffer, WitnessStageOpeningError> {
        let mut workspace = CudaDeviceBuffer::new(output_buffer.len())
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        if source_buffer.is_compact_for(self.columns) {
            cuda_goldilocks_coset_extend_row_major_columns_device_unsynced(
                source_buffer.as_buffer(),
                output_buffer,
                &mut workspace,
                self.columns,
                self.source_bits,
                self.target_bits,
            )
        } else {
            cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced(
                source_buffer.as_buffer(),
                output_buffer,
                &mut workspace,
                CudaRowMajorColumnView {
                    source_rows: self.source_rows,
                    source_row_stride: source_buffer.row_stride(),
                    column_offset: source_buffer.column_offset(),
                    column_count: self.columns,
                },
                self.source_bits,
                self.target_bits,
            )
        }
        .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        Ok(workspace)
    }

    #[cfg(feature = "cuda")]
    fn extended_row_values_cuda(&self, row: usize) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        prepare_gpu_setup(self.target_bits)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        let source_buffer = self.source_device_buffer(None)?;
        self.extended_row_values_from_source_cuda(row, &source_buffer)
    }

    #[cfg(feature = "cuda")]
    fn extended_row_values_from_source_cuda(
        &self,
        row: usize,
        source_buffer: &SourceDeviceBuffer<'_>,
    ) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        let row_byte_count = self
            .columns
            .checked_mul(WORD_BYTES)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let mut row_buffer = CudaDeviceBuffer::new(row_byte_count)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        if source_buffer.is_compact_for(self.columns) {
            cuda_goldilocks_coset_extend_row_major_columns_row_device(
                source_buffer.as_buffer(),
                &mut row_buffer,
                self.columns,
                self.source_bits,
                self.target_bits,
                row,
            )
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        } else {
            cuda_goldilocks_coset_extend_row_major_columns_strided_row_device(
                source_buffer.as_buffer(),
                &mut row_buffer,
                CudaRowMajorColumnView {
                    source_rows: self.source_rows,
                    source_row_stride: source_buffer.row_stride(),
                    column_offset: source_buffer.column_offset(),
                    column_count: self.columns,
                },
                self.source_bits,
                self.target_bits,
                row,
            )
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        }
        row_buffer
            .to_u64_words()
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?
            .into_iter()
            .map(|value| Felt::from_canonical(value).map_err(WitnessStageOpeningError::Field))
            .collect()
    }

    #[cfg(feature = "cuda")]
    fn open_with_recomputed_leaf_level_cuda(
        &self,
        row: usize,
        expected_root: [Felt; HASH_WORDS],
        source_buffer: &SourceDeviceBuffer<'_>,
        timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<CompactOnDemandOpening, WitnessStageOpeningError> {
        let mut openings = self.open_batch_with_recomputed_leaf_level_cuda(
            std::slice::from_ref(&row),
            expected_root,
            source_buffer,
            timing,
        )?;
        openings
            .pop()
            .ok_or(WitnessStageOpeningError::LengthOverflow)
    }

    #[cfg(feature = "cuda")]
    fn open_batch_with_recomputed_leaf_level_cuda(
        &self,
        rows: &[usize],
        expected_root: [Felt; HASH_WORDS],
        source_buffer: &SourceDeviceBuffer<'_>,
        mut timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<Vec<CompactOnDemandOpening>, WitnessStageOpeningError> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        for row in rows {
            if *row >= self.extended_rows {
                return Err(WitnessStageOpeningError::LengthOverflow);
            }
        }
        if let Some(retained_leaf_digest_level) = &self.retained_leaf_digest_level {
            match self.open_batch_with_retained_leaf_digest_level_cuda(
                rows,
                expected_root,
                source_buffer,
                retained_leaf_digest_level,
                timing.as_deref_mut(),
            ) {
                Ok(openings) => return Ok(openings),
                Err(WitnessStageOpeningError::LengthOverflow) => {}
                Err(error) => {
                    return Err(WitnessStageOpeningError::context(
                        "compact retained leaf digest",
                        error,
                    ));
                }
            }
        }

        let mut output_buffer = record_opening_duration(
            timing.as_deref_mut().map(|timing| &mut timing.setup),
            || {
                CudaDeviceBuffer::new(self.raw_leaf_bytes)
                    .map_err(|_| WitnessStageOpeningError::LengthOverflow)
            },
        )
        .map_err(|source| {
            WitnessStageOpeningError::context("compact full leaf allocation", source)
        })?;
        let _extension_workspace = record_opening_duration(
            timing.as_deref_mut().map(|timing| &mut timing.leaf_extend),
            || self.extend_source_device_buffer_cuda_unsynced(source_buffer, &mut output_buffer),
        )
        .map_err(|source| WitnessStageOpeningError::context("compact leaf extension", source))?;
        if let Some(timing) = timing.as_deref_mut() {
            timing.record_coset_extend_work(
                self.raw_leaf_bytes,
                self.columns,
                self.source_bits,
                self.target_bits,
            );
        }
        let leaf_level = record_opening_duration(
            timing.as_deref_mut().map(|timing| &mut timing.leaf_hash),
            || {
                linear_hash_level_from_validated_row_major_device_buffer(
                    &output_buffer,
                    self.extended_rows,
                    self.columns,
                    self.arity,
                )
                .map_err(WitnessStageOpeningError::from)
            },
        )
        .map_err(|source| WitnessStageOpeningError::context("compact leaf hash", source))?;
        if let Some(timing) = timing.as_deref_mut() {
            timing.record_leaf_hash_work(self.extended_rows, self.raw_leaf_bytes, self.arity);
        }

        if let Some(retained_parent_checkpoint_level) = &self.retained_parent_checkpoint_level {
            match self.open_batch_with_retained_parent_checkpoint_level_cuda(
                rows,
                expected_root,
                &output_buffer,
                &leaf_level,
                retained_parent_checkpoint_level,
                timing.as_deref_mut(),
            ) {
                Ok(openings) => return Ok(openings),
                Err(WitnessStageOpeningError::LengthOverflow) => {}
                Err(error) => {
                    return Err(WitnessStageOpeningError::context(
                        "compact parent checkpoint",
                        error,
                    ));
                }
            }
        }

        let path_parent_work = if timing.is_some() {
            Some(
                merkle_opening_path_parent_work(self.extended_rows, self.arity)
                    .ok_or(WitnessStageOpeningError::LengthOverflow)?,
            )
        } else {
            None
        };
        let mut openings = Vec::with_capacity(rows.len());
        for row in rows {
            let path = record_opening_duration(
                timing.as_deref_mut().map(|timing| &mut timing.path),
                || {
                    leaf_level
                        .opening_path(*row)
                        .map_err(WitnessStageOpeningError::from)
                },
            )
            .map_err(|source| WitnessStageOpeningError::context("compact full path", source))?;
            if path.root != expected_root {
                return Err(WitnessStageOpeningError::InvalidTreeByteLength {
                    expected: self.logical_tree_bytes,
                    found: 0,
                });
            }
            if let (Some(timing), Some((row_count, byte_count, launch_count))) =
                (timing.as_deref_mut(), path_parent_work)
            {
                timing.record_path_parent_hash_work(row_count, byte_count, launch_count);
            }
            let values = record_opening_duration(
                timing.as_deref_mut().map(|timing| &mut timing.row_values),
                || self.copy_extended_row_values_from_device(&output_buffer, *row),
            )
            .map_err(|source| WitnessStageOpeningError::context("compact row values", source))?;
            if let Some(timing) = timing.as_deref_mut() {
                timing.record_device_row_values(1, self.columns);
            }
            openings.push((values, path.siblings));
        }
        Ok(openings)
    }

    #[cfg(feature = "cuda")]
    fn open_batch_with_retained_parent_checkpoint_level_cuda(
        &self,
        rows: &[usize],
        expected_root: [Felt; HASH_WORDS],
        output_buffer: &CudaDeviceBuffer,
        leaf_level: &CudaDigestLevel,
        checkpoint: &RetainedCudaParentCheckpointLevel,
        mut timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<Vec<CompactOnDemandOpening>, WitnessStageOpeningError> {
        if leaf_level.state_count() != self.extended_rows
            || leaf_level.arity() != self.arity
            || checkpoint.source_state_count() != self.extended_rows
            || checkpoint.arity() != self.arity
            || checkpoint.folded_level_count() == 0
        {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        if let Some(timing) = timing.as_deref_mut() {
            timing.record_retained_parent_checkpoint_opening(rows.len());
        }
        let lower_prefix_parent_work = if timing.is_some() {
            Some(
                merkle_opening_path_prefix_parent_work(
                    self.extended_rows,
                    self.arity,
                    checkpoint.folded_level_count(),
                )
                .ok_or(WitnessStageOpeningError::LengthOverflow)?,
            )
        } else {
            None
        };
        let upper_suffix_parent_work = if timing.is_some() {
            Some(
                merkle_opening_path_parent_work(checkpoint.state_count(), self.arity)
                    .ok_or(WitnessStageOpeningError::LengthOverflow)?,
            )
        } else {
            None
        };
        let lower_prefixes =
            record_opening_duration(timing.as_deref_mut().map(|timing| &mut timing.path), || {
                leaf_level
                    .opening_path_prefix_batch_for_source_rows(
                        rows,
                        checkpoint.folded_level_count(),
                    )
                    .map_err(WitnessStageOpeningError::from)
            })
            .map_err(|source| {
                WitnessStageOpeningError::context("compact parent checkpoint prefix path", source)
            })?;
        if let (Some(timing), Some((row_count, byte_count, launch_count))) =
            (timing.as_deref_mut(), lower_prefix_parent_work)
        {
            timing.record_path_parent_hash_work(row_count, byte_count, launch_count);
        }
        if lower_prefixes.len() != rows.len() {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        let mut openings = Vec::with_capacity(rows.len());
        for (row, lower_prefix) in rows.iter().copied().zip(lower_prefixes.into_iter()) {
            let upper_suffix = record_opening_duration(
                timing.as_deref_mut().map(|timing| &mut timing.path),
                || {
                    checkpoint
                        .opening_path_for_source_row(row)
                        .map_err(WitnessStageOpeningError::from)
                },
            )
            .map_err(|source| {
                WitnessStageOpeningError::context("compact parent checkpoint suffix path", source)
            })?;
            if upper_suffix.root != expected_root {
                return Err(WitnessStageOpeningError::InvalidTreeByteLength {
                    expected: self.logical_tree_bytes,
                    found: 0,
                });
            }
            if let (Some(timing), Some((row_count, byte_count, launch_count))) =
                (timing.as_deref_mut(), upper_suffix_parent_work)
            {
                timing.record_path_parent_hash_work(row_count, byte_count, launch_count);
            }
            let values = record_opening_duration(
                timing.as_deref_mut().map(|timing| &mut timing.row_values),
                || self.copy_extended_row_values_from_device(output_buffer, row),
            )
            .map_err(|source| {
                WitnessStageOpeningError::context("compact parent checkpoint row values", source)
            })?;
            if let Some(timing) = timing.as_deref_mut() {
                timing.record_device_row_values(1, self.columns);
            }
            let mut siblings = lower_prefix;
            siblings.extend(upper_suffix.siblings);
            openings.push((values, siblings));
        }
        Ok(openings)
    }

    #[cfg(feature = "cuda")]
    fn open_batch_with_retained_leaf_digest_level_cuda(
        &self,
        rows: &[usize],
        expected_root: [Felt; HASH_WORDS],
        source_buffer: &SourceDeviceBuffer<'_>,
        leaf_level: &RetainedCudaLeafDigestLevel,
        mut timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<Vec<CompactOnDemandOpening>, WitnessStageOpeningError> {
        if leaf_level.state_count() != self.extended_rows || leaf_level.arity() != self.arity {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        if let Some(timing) = timing.as_deref_mut() {
            timing.record_retained_leaf_digest_opening(rows.len());
        }
        let path_parent_work = if timing.is_some() {
            Some(
                merkle_opening_path_parent_work(self.extended_rows, self.arity)
                    .ok_or(WitnessStageOpeningError::LengthOverflow)?,
            )
        } else {
            None
        };
        let mut openings = Vec::with_capacity(rows.len());
        for row in rows {
            let path = record_opening_duration(
                timing.as_deref_mut().map(|timing| &mut timing.path),
                || {
                    leaf_level
                        .opening_path(*row)
                        .map_err(WitnessStageOpeningError::from)
                },
            )
            .map_err(|source| {
                WitnessStageOpeningError::context("compact retained leaf digest path", source)
            })?;
            if path.root != expected_root {
                return Err(WitnessStageOpeningError::InvalidTreeByteLength {
                    expected: self.logical_tree_bytes,
                    found: 0,
                });
            }
            if let (Some(timing), Some((row_count, byte_count, launch_count))) =
                (timing.as_deref_mut(), path_parent_work)
            {
                timing.record_path_parent_hash_work(row_count, byte_count, launch_count);
            }
            let values = record_opening_duration(
                timing.as_deref_mut().map(|timing| &mut timing.row_values),
                || self.extended_row_values_from_source_cuda(*row, source_buffer),
            )
            .map_err(|source| {
                WitnessStageOpeningError::context("compact retained leaf digest row values", source)
            })?;
            if let Some(timing) = timing.as_deref_mut() {
                timing.record_source_row_values(1, self.columns);
            }
            openings.push((values, path.siblings));
        }
        Ok(openings)
    }

    #[cfg(feature = "cuda")]
    fn copy_extended_row_values_from_device(
        &self,
        output_buffer: &CudaDeviceBuffer,
        row: usize,
    ) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        let row_byte_count = self
            .columns
            .checked_mul(WORD_BYTES)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let row_offset = row
            .checked_mul(row_byte_count)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let mut bytes = vec![0_u8; row_byte_count];
        output_buffer
            .copy_range_to(row_offset, &mut bytes)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        bytes
            .chunks_exact(WORD_BYTES)
            .map(|chunk| {
                let value = u64::from_le_bytes(chunk.try_into().expect("slice length checked"));
                Felt::from_canonical(value).map_err(WitnessStageOpeningError::Field)
            })
            .collect()
    }

    #[cfg(feature = "cuda")]
    fn extended_leaf_bytes_cuda(&self) -> Result<Vec<u8>, WitnessStageOpeningError> {
        self.extended_rows_device()?
            .to_vec()
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)
    }

    #[cfg(feature = "cuda")]
    fn open_on_demand_cuda(
        &self,
        row: usize,
        expected_root: [Felt; HASH_WORDS],
        source_device: Option<&WitnessStageSourceDeviceView>,
        timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<CompactOnDemandOpening, WitnessStageOpeningError> {
        prepare_gpu_setup(self.target_bits)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        let source_buffer = self.source_device_buffer(source_device)?;
        self.open_with_recomputed_leaf_level_cuda(row, expected_root, &source_buffer, timing)
    }

    #[cfg(feature = "cuda")]
    fn open_batch_on_demand_cuda(
        &self,
        rows: &[usize],
        expected_root: [Felt; HASH_WORDS],
        source_device: Option<&WitnessStageSourceDeviceView>,
        timing: Option<&mut WitnessStageOpeningWorkTiming>,
    ) -> Result<Vec<CompactOnDemandOpening>, WitnessStageOpeningError> {
        prepare_gpu_setup(self.target_bits)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        let source_buffer = self.source_device_buffer(source_device)?;
        self.open_batch_with_recomputed_leaf_level_cuda(rows, expected_root, &source_buffer, timing)
    }

    #[cfg(feature = "cuda")]
    fn extended_rows_device(&self) -> Result<CudaDeviceBuffer, WitnessStageOpeningError> {
        prepare_gpu_setup(self.target_bits)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        let source_buffer = self.source_device_buffer(None)?;
        let mut output_buffer = CudaDeviceBuffer::new(self.raw_leaf_bytes)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        self.extend_source_device_buffer_cuda(&source_buffer, &mut output_buffer)?;
        Ok(output_buffer)
    }

    #[cfg(not(feature = "cuda"))]
    fn source_column_values(&self, column: usize) -> Result<Vec<Felt>, WitnessStageOpeningError> {
        if column >= self.columns {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        if self.source_values.len() != self.source_rows * self.columns {
            return Err(WitnessStageOpeningError::LengthOverflow);
        }
        Ok((0..self.source_rows)
            .map(|row| self.source_values[row * self.columns + column])
            .collect())
    }
}

fn materialize_digest_tree_bytes(
    level: Vec<[Felt; HASH_WORDS]>,
    arity: usize,
) -> Result<Vec<u8>, WitnessStageOpeningError> {
    if level.is_empty() {
        return Err(WitnessStageOpeningError::EmptyValues);
    }
    let mut out = Vec::new();
    for digest in &level {
        append_digest_bytes(&mut out, *digest);
    }

    for parent_level in parent_levels_from_digest_level(&level, arity)? {
        for _ in 0..parent_level.padding_count {
            append_digest_bytes(&mut out, [Felt::ZERO; HASH_WORDS]);
        }
        for digest in &parent_level.parents {
            append_digest_bytes(&mut out, *digest);
        }
    }
    Ok(out)
}

fn append_digest_bytes(out: &mut Vec<u8>, digest: [Felt; HASH_WORDS]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(feature = "cuda")]
fn record_opening_duration<T>(
    duration: Option<&mut Duration>,
    operation: impl FnOnce() -> Result<T, WitnessStageOpeningError>,
) -> Result<T, WitnessStageOpeningError> {
    let started = Instant::now();
    let result = operation();
    if let Some(duration) = duration {
        *duration += started.elapsed();
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessStageOpening {
    row_index: u64,
    values: Vec<Felt>,
    siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
}

impl WitnessStageOpening {
    pub fn new(
        row_index: u64,
        values: Vec<Felt>,
        siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
    ) -> Result<Self, WitnessStageOpeningError> {
        if values.is_empty() {
            return Err(WitnessStageOpeningError::EmptyValues);
        }
        Ok(Self {
            row_index,
            values,
            siblings,
        })
    }

    pub fn row_index(&self) -> u64 {
        self.row_index
    }

    pub fn values(&self) -> &[Felt] {
        &self.values
    }

    pub fn siblings(&self) -> &[Vec<[Felt; HASH_WORDS]>] {
        &self.siblings
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceCommitments {
    commitments: Vec<WitnessStageCommitment>,
}

impl WitnessTraceCommitments {
    pub(crate) fn new(commitments: Vec<WitnessStageCommitment>) -> Self {
        Self { commitments }
    }

    pub fn stage_count(&self) -> usize {
        self.commitments.len()
    }

    pub fn commitments(&self) -> &[WitnessStageCommitment] {
        &self.commitments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessStageExtendedValues {
    stage_index: usize,
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    values: Vec<Felt>,
}

impl WitnessStageExtendedValues {
    pub(crate) fn new(
        stage_index: usize,
        source_rows: usize,
        extended_rows: usize,
        columns: usize,
        values: Vec<Felt>,
    ) -> Self {
        Self {
            stage_index,
            source_rows,
            extended_rows,
            columns,
            values,
        }
    }

    pub fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub fn source_row_count(&self) -> usize {
        self.source_rows
    }

    pub fn extended_row_count(&self) -> usize {
        self.extended_rows
    }

    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub fn values(&self) -> &[Felt] {
        &self.values
    }
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::*;

    #[test]
    fn default_retained_leaf_digest_limit_stays_within_static_cache_cap() {
        assert_eq!(
            DEFAULT_RETAINED_LEAF_DIGEST_BYTES, 14_000_000_000,
            "default retained leaf digest cache should match the measured source/leaf split"
        );
        assert!(
            default_retained_leaf_digest_limit() <= DEFAULT_RETAINED_LEAF_DIGEST_BYTES,
            "default retained leaf digest cache should stay within the measured static cap"
        );
    }
}

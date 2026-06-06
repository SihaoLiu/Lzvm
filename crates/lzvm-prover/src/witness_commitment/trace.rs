#[cfg(feature = "cuda")]
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::witness_layout::derive_witness_trace_layout;
use crate::witness_layout::WitnessTraceStageValues;
use crate::witness_trace::WitnessTraceBuffer;
use crate::ProveUnitSchedule;
#[cfg(feature = "cuda")]
use lzvm_field::Felt;

#[cfg(feature = "cuda")]
use lzvm_accel::CudaDeviceBuffer;

#[cfg(feature = "cuda")]
type WitnessStageSourceDeviceRef<'a> = &'a WitnessStageSourceDevice;
#[cfg(not(feature = "cuda"))]
type WitnessStageSourceDeviceRef<'a> = &'a ();

#[cfg(not(feature = "cuda"))]
use super::commit_witness_stage_leaves_owned;
#[cfg(feature = "cuda")]
use super::{
    commit_witness_stage_device_compact_with_leaf_hash_level,
    commit_witness_stage_leaves_compact_with_leaf_hash_level,
    compact_witness_stage_leaf_hash_level_from_source_device_view_timing,
    compact_witness_stage_leaf_hash_level_with_source_device_timing,
    extend_witness_stage_leaves_from_source_device_view, retain_source_device_view,
    RetainedCudaSourceDevice, WitnessStageCommitment, WitnessStageDeviceCompactCommitInput,
    WitnessStageSourceDeviceView, WORD_BYTES,
};
use super::{
    decode_witness_stage_leaf_values, extend_witness_stage_leaves, WitnessStageExtendedValues,
    WitnessStageLeafExtendTiming, WitnessTraceCommitmentError, WitnessTraceCommitments,
};

#[cfg(feature = "cuda")]
const MAX_REUSE_SOURCE_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WitnessStageCommitTiming {
    leaf_extend_duration: Duration,
    leaf_extend_timing: WitnessStageLeafExtendTiming,
    tree_commit_duration: Duration,
}

#[cfg(feature = "cuda")]
#[derive(Default)]
pub(crate) struct WitnessStageCommitmentReuseCache {
    entries: Vec<WitnessStageCommitmentReuseEntry>,
}

#[cfg(feature = "cuda")]
struct WitnessStageCommitmentReuseEntry {
    stage_index: usize,
    source_rows: usize,
    columns: usize,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    source_values: Vec<Felt>,
    commitment: WitnessStageCommitment,
}

#[cfg(feature = "cuda")]
impl WitnessStageCommitmentReuseCache {
    fn get(
        &self,
        stage: &WitnessTraceStageValues,
        params: WitnessStageCommitParams,
    ) -> Option<WitnessStageCommitment> {
        self.entries
            .iter()
            .find(|entry| {
                entry.stage_index == stage.stage_index()
                    && entry.source_rows == stage.row_count()
                    && entry.columns == stage.column_count()
                    && entry.source_bits == params.source_bits
                    && entry.target_bits == params.target_bits
                    && entry.arity == params.arity
                    && entry.source_values == stage.values()
            })
            .map(|entry| entry.commitment.clone())
    }

    fn insert(
        &mut self,
        stage: &WitnessTraceStageValues,
        params: WitnessStageCommitParams,
        commitment: &WitnessStageCommitment,
    ) {
        let Some(source_bytes) = stage
            .values()
            .len()
            .checked_mul(WORD_BYTES)
            .filter(|bytes| *bytes <= MAX_REUSE_SOURCE_BYTES)
        else {
            return;
        };
        let _ = source_bytes;
        if self.get(stage, params).is_some() {
            return;
        }
        self.entries.push(WitnessStageCommitmentReuseEntry {
            stage_index: stage.stage_index(),
            source_rows: stage.row_count(),
            columns: stage.column_count(),
            source_bits: params.source_bits,
            target_bits: params.target_bits,
            arity: params.arity,
            source_values: stage.values().to_vec(),
            commitment: commitment.clone(),
        });
    }
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
pub(crate) struct WitnessStageSourceDevice {
    stage_index: usize,
    row_count: usize,
    column_count: usize,
    row_stride: usize,
    column_offset: usize,
    values: Arc<CudaDeviceBuffer>,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
pub(crate) struct WitnessStageRetainedSourceDevice {
    stage_index: usize,
    source_device: Arc<RetainedCudaSourceDevice>,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
pub(crate) struct WitnessRetainedDeviceBuffer {
    source_device: Arc<RetainedCudaSourceDevice>,
}

#[cfg(feature = "cuda")]
impl WitnessStageRetainedSourceDevice {
    pub(crate) fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub(crate) fn source_view(&self) -> &WitnessStageSourceDeviceView {
        self.source_device.source_view()
    }
}

#[cfg(feature = "cuda")]
impl WitnessRetainedDeviceBuffer {
    pub(crate) fn buffer(&self) -> &CudaDeviceBuffer {
        self.source_device.source_view().buffer()
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn retain_device_buffer(
    buffer: &Arc<CudaDeviceBuffer>,
) -> Option<WitnessRetainedDeviceBuffer> {
    if !buffer.len().is_multiple_of(WORD_BYTES) {
        return None;
    }
    let word_count = buffer.len() / WORD_BYTES;
    retain_source_device_view(WitnessStageSourceDeviceView::new(
        1,
        word_count,
        word_count,
        0,
        Arc::clone(buffer),
    ))
    .map(|source_device| WitnessRetainedDeviceBuffer { source_device })
}

#[cfg(feature = "cuda")]
impl WitnessStageSourceDevice {
    pub(crate) fn from_row_major_column_window(
        stage_index: usize,
        row_count: usize,
        column_count: usize,
        row_stride: usize,
        column_offset: usize,
        values: &Arc<CudaDeviceBuffer>,
    ) -> Self {
        Self {
            stage_index,
            row_count,
            column_count,
            row_stride,
            column_offset,
            values: Arc::clone(values),
        }
    }

    pub(crate) fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub(crate) fn row_count(&self) -> usize {
        self.row_count
    }

    pub(crate) fn column_count(&self) -> usize {
        self.column_count
    }

    pub(crate) fn source_view(&self) -> WitnessStageSourceDeviceView {
        WitnessStageSourceDeviceView::new(
            self.row_count,
            self.column_count,
            self.row_stride,
            self.column_offset,
            Arc::clone(&self.values),
        )
    }

    pub(crate) fn retain(&self) -> Option<WitnessStageRetainedSourceDevice> {
        retain_source_device_view(self.source_view()).map(|source_device| {
            WitnessStageRetainedSourceDevice {
                stage_index: self.stage_index,
                source_device,
            }
        })
    }
}

#[cfg(feature = "cuda")]
fn find_stage_source_device(
    source_devices: &[WitnessStageSourceDevice],
    stage_index: usize,
) -> Option<&WitnessStageSourceDevice> {
    source_devices
        .iter()
        .find(|source| source.stage_index == stage_index)
}

#[cfg(feature = "cuda")]
fn find_retained_stage_source_device(
    source_devices: &[WitnessStageRetainedSourceDevice],
    stage_index: usize,
) -> Option<&WitnessStageRetainedSourceDevice> {
    source_devices
        .iter()
        .find(|source| source.stage_index() == stage_index)
}

impl WitnessStageCommitTiming {
    pub(crate) fn accumulate(&mut self, other: Self) {
        self.leaf_extend_duration += other.leaf_extend_duration;
        self.leaf_extend_timing.accumulate(other.leaf_extend_timing);
        self.tree_commit_duration += other.tree_commit_duration;
    }

    pub(crate) fn leaf_extend_duration(&self) -> Duration {
        self.leaf_extend_duration
    }

    pub(crate) fn tree_commit_duration(&self) -> Duration {
        self.tree_commit_duration
    }

    pub(crate) fn leaf_setup_duration(&self) -> Duration {
        self.leaf_extend_timing.setup_duration()
    }

    pub(crate) fn leaf_upload_duration(&self) -> Duration {
        self.leaf_extend_timing.upload_duration()
    }

    pub(crate) fn leaf_kernel_duration(&self) -> Duration {
        self.leaf_extend_timing.kernel_duration()
    }

    pub(crate) fn leaf_download_duration(&self) -> Duration {
        self.leaf_extend_timing.download_duration()
    }

    pub(crate) fn leaf_validate_duration(&self) -> Duration {
        self.leaf_extend_timing.validate_duration()
    }

    pub(crate) fn leaf_hash_duration(&self) -> Duration {
        self.leaf_extend_timing.leaf_hash_duration()
    }

    pub(crate) fn leaf_hash_row_count(&self) -> usize {
        self.leaf_extend_timing.leaf_hash_row_count()
    }

    pub(crate) fn leaf_hash_byte_count(&self) -> usize {
        self.leaf_extend_timing.leaf_hash_byte_count()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WitnessIndexedStageCommitTiming {
    stage_index: usize,
    timing: WitnessStageCommitTiming,
}

impl WitnessIndexedStageCommitTiming {
    pub(crate) fn new(stage_index: usize, timing: WitnessStageCommitTiming) -> Self {
        Self {
            stage_index,
            timing,
        }
    }

    pub(crate) fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub(crate) fn timing(&self) -> WitnessStageCommitTiming {
        self.timing
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WitnessStageCommitParams {
    source_bits: usize,
    target_bits: usize,
    arity: usize,
}

impl WitnessStageCommitParams {
    fn from_unit(unit: &ProveUnitSchedule) -> Result<Self, WitnessTraceCommitmentError> {
        Ok(Self {
            source_bits: usize::try_from(unit.base_domain_bits)
                .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?,
            target_bits: usize::try_from(unit.extended_domain_bits)
                .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?,
            arity: usize::try_from(unit.merkle_tree_arity)
                .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?,
        })
    }
}

pub fn commit_witness_trace_stages(
    trace: &WitnessTraceBuffer,
    unit: &ProveUnitSchedule,
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    let layout = derive_witness_trace_layout(unit)?;
    let params = WitnessStageCommitParams::from_unit(unit)?;

    let mut commitments = Vec::with_capacity(layout.stage_count());
    for stage_info in layout.stages() {
        let stage = layout.stage_trace(trace, stage_info.stage_index)?;
        let commitment = commit_extended_witness_stage(&stage, params, None, None)?;
        commitments.push(commitment);
    }

    Ok(WitnessTraceCommitments::new(commitments))
}

pub fn commit_witness_trace_stages_with_workers(
    trace: &WitnessTraceBuffer,
    unit: &ProveUnitSchedule,
    worker_count: usize,
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    let worker_count = worker_count.max(1);
    if worker_count == 1 || unit.stage_commit_widths.len() <= 1 {
        return commit_witness_trace_stages(trace, unit);
    }

    let layout = derive_witness_trace_layout(unit)?;
    let params = WitnessStageCommitParams::from_unit(unit)?;
    let stage_indices = layout
        .stages()
        .iter()
        .map(|stage| stage.stage_index)
        .collect::<Vec<_>>();
    let worker_count = worker_count.min(stage_indices.len());
    let chunk_size = stage_indices.len().div_ceil(worker_count);

    let mut commitments = Vec::with_capacity(stage_indices.len());
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in stage_indices.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            let layout = &layout;
            handles.push(scope.spawn(move || {
                let mut out = Vec::with_capacity(chunk.len());
                for stage_index in chunk {
                    let stage = layout.stage_trace(trace, stage_index)?;
                    let commitment = commit_extended_witness_stage(&stage, params, None, None)?;
                    out.push((stage_index, commitment));
                }
                Ok::<_, WitnessTraceCommitmentError>(out)
            }));
        }

        for handle in handles {
            let chunk = handle
                .join()
                .map_err(|_| WitnessTraceCommitmentError::WorkerPanic)??;
            commitments.extend(chunk);
        }
        Ok::<(), WitnessTraceCommitmentError>(())
    })?;

    commitments.sort_by_key(|(stage_index, _)| *stage_index);
    Ok(WitnessTraceCommitments::new(
        commitments
            .into_iter()
            .map(|(_, commitment)| commitment)
            .collect(),
    ))
}

#[cfg_attr(feature = "cuda", allow(dead_code))]
pub(crate) fn commit_witness_stage_values_with_workers(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    #[cfg(feature = "cuda")]
    {
        commit_witness_stage_values_with_source_devices_and_workers(stages, unit, worker_count, &[])
    }
    #[cfg(not(feature = "cuda"))]
    {
        commit_witness_stage_values_with_source_devices_and_workers(stages, unit, worker_count)
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_values_with_source_devices_and_workers(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
    source_devices: &[WitnessStageSourceDevice],
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    commit_witness_stage_values_with_workers_inner(stages, unit, worker_count, source_devices)
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_values_with_source_devices_reusing_cached_stages_and_workers(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    source_devices: &[WitnessStageSourceDevice],
    reuse_cache: &mut WitnessStageCommitmentReuseCache,
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    let params = WitnessStageCommitParams::from_unit(unit)?;
    let mut commitments = Vec::with_capacity(stages.len());
    for stage in stages {
        if let Some(commitment) = reuse_cache.get(stage, params) {
            commitments.push(commitment);
            continue;
        }
        let commitment = commit_extended_witness_stage(
            stage,
            params,
            find_stage_source_device(source_devices, stage.stage_index()),
            None,
        )?;
        reuse_cache.insert(stage, params, &commitment);
        commitments.push(commitment);
    }
    Ok(WitnessTraceCommitments::new(commitments))
}

#[cfg(not(feature = "cuda"))]
fn commit_witness_stage_values_with_source_devices_and_workers(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    commit_witness_stage_values_with_workers_inner(stages, unit, worker_count)
}

#[cfg(feature = "cuda")]
fn commit_witness_stage_values_with_workers_inner(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
    source_devices: &[WitnessStageSourceDevice],
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    let worker_count = worker_count.max(1);
    let params = WitnessStageCommitParams::from_unit(unit)?;

    if worker_count == 1 || stages.len() <= 1 {
        let mut commitments = Vec::with_capacity(stages.len());
        for stage in stages {
            commitments.push(commit_extended_witness_stage(
                stage,
                params,
                find_stage_source_device(source_devices, stage.stage_index()),
                None,
            )?);
        }
        return Ok(WitnessTraceCommitments::new(commitments));
    }

    let worker_count = worker_count.min(stages.len());
    let chunk_size = stages.len().div_ceil(worker_count);
    let mut commitments = Vec::with_capacity(stages.len());
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in stages.chunks(chunk_size) {
            handles.push(scope.spawn(move || {
                let mut out = Vec::with_capacity(chunk.len());
                for stage in chunk {
                    let commitment = commit_extended_witness_stage(
                        stage,
                        params,
                        find_stage_source_device(source_devices, stage.stage_index()),
                        None,
                    )?;
                    out.push((stage.stage_index(), commitment));
                }
                Ok::<_, WitnessTraceCommitmentError>(out)
            }));
        }

        for handle in handles {
            let chunk = handle
                .join()
                .map_err(|_| WitnessTraceCommitmentError::WorkerPanic)??;
            commitments.extend(chunk);
        }
        Ok::<(), WitnessTraceCommitmentError>(())
    })?;

    commitments.sort_by_key(|(stage_index, _)| *stage_index);
    Ok(WitnessTraceCommitments::new(
        commitments
            .into_iter()
            .map(|(_, commitment)| commitment)
            .collect(),
    ))
}

#[cfg(not(feature = "cuda"))]
fn commit_witness_stage_values_with_workers_inner(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
) -> Result<WitnessTraceCommitments, WitnessTraceCommitmentError> {
    let worker_count = worker_count.max(1);
    let params = WitnessStageCommitParams::from_unit(unit)?;

    if worker_count == 1 || stages.len() <= 1 {
        let mut commitments = Vec::with_capacity(stages.len());
        for stage in stages {
            commitments.push(commit_extended_witness_stage(stage, params, None, None)?);
        }
        return Ok(WitnessTraceCommitments::new(commitments));
    }

    let worker_count = worker_count.min(stages.len());
    let chunk_size = stages.len().div_ceil(worker_count);
    let mut commitments = Vec::with_capacity(stages.len());
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in stages.chunks(chunk_size) {
            handles.push(scope.spawn(move || {
                let mut out = Vec::with_capacity(chunk.len());
                for stage in chunk {
                    let commitment = commit_extended_witness_stage(stage, params, None, None)?;
                    out.push((stage.stage_index(), commitment));
                }
                Ok::<_, WitnessTraceCommitmentError>(out)
            }));
        }

        for handle in handles {
            let chunk = handle
                .join()
                .map_err(|_| WitnessTraceCommitmentError::WorkerPanic)??;
            commitments.extend(chunk);
        }
        Ok::<(), WitnessTraceCommitmentError>(())
    })?;

    commitments.sort_by_key(|(stage_index, _)| *stage_index);
    Ok(WitnessTraceCommitments::new(
        commitments
            .into_iter()
            .map(|(_, commitment)| commitment)
            .collect(),
    ))
}

#[cfg_attr(feature = "cuda", allow(dead_code))]
pub(crate) fn commit_witness_stage_values_with_workers_and_indexed_timing(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
    timing: &mut WitnessStageCommitTiming,
) -> Result<
    (
        WitnessTraceCommitments,
        Vec<WitnessIndexedStageCommitTiming>,
    ),
    WitnessTraceCommitmentError,
> {
    #[cfg(feature = "cuda")]
    {
        commit_witness_stage_values_with_source_devices_and_indexed_timing(
            stages,
            unit,
            worker_count,
            &[],
            timing,
        )
    }
    #[cfg(not(feature = "cuda"))]
    {
        commit_witness_stage_values_with_workers_and_timing_inner(
            stages,
            unit,
            worker_count,
            timing,
        )
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_values_with_source_devices_and_indexed_timing(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
    source_devices: &[WitnessStageSourceDevice],
    timing: &mut WitnessStageCommitTiming,
) -> Result<
    (
        WitnessTraceCommitments,
        Vec<WitnessIndexedStageCommitTiming>,
    ),
    WitnessTraceCommitmentError,
> {
    commit_witness_stage_values_with_workers_and_timing_inner(
        stages,
        unit,
        worker_count,
        source_devices,
        timing,
    )
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_values_with_source_devices_reusing_cached_stages_and_indexed_timing(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    source_devices: &[WitnessStageSourceDevice],
    reuse_cache: &mut WitnessStageCommitmentReuseCache,
    timing: &mut WitnessStageCommitTiming,
) -> Result<
    (
        WitnessTraceCommitments,
        Vec<WitnessIndexedStageCommitTiming>,
    ),
    WitnessTraceCommitmentError,
> {
    let params = WitnessStageCommitParams::from_unit(unit)?;
    let mut commitments = Vec::with_capacity(stages.len());
    let mut stage_timings = Vec::with_capacity(stages.len());
    for stage in stages {
        let mut stage_timing = WitnessStageCommitTiming::default();
        let commitment = if let Some(commitment) = reuse_cache.get(stage, params) {
            commitment
        } else {
            let commitment = commit_extended_witness_stage(
                stage,
                params,
                find_stage_source_device(source_devices, stage.stage_index()),
                Some(&mut stage_timing),
            )?;
            reuse_cache.insert(stage, params, &commitment);
            commitment
        };
        timing.accumulate(stage_timing);
        stage_timings.push(WitnessIndexedStageCommitTiming::new(
            stage.stage_index(),
            stage_timing,
        ));
        commitments.push(commitment);
    }
    Ok((WitnessTraceCommitments::new(commitments), stage_timings))
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_source_devices_and_indexed_timing(
    source_devices: &[WitnessStageSourceDevice],
    unit: &ProveUnitSchedule,
    timing: &mut WitnessStageCommitTiming,
) -> Result<
    (
        WitnessTraceCommitments,
        Vec<WitnessIndexedStageCommitTiming>,
    ),
    WitnessTraceCommitmentError,
> {
    commit_witness_stage_source_devices_and_indexed_timing_inner(
        source_devices,
        unit,
        timing,
        false,
    )
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_source_devices_and_indexed_timing_external_source(
    source_devices: &[WitnessStageSourceDevice],
    unit: &ProveUnitSchedule,
    timing: &mut WitnessStageCommitTiming,
) -> Result<
    (
        WitnessTraceCommitments,
        Vec<WitnessIndexedStageCommitTiming>,
    ),
    WitnessTraceCommitmentError,
> {
    commit_witness_stage_source_devices_and_indexed_timing_inner(source_devices, unit, timing, true)
}

#[cfg(feature = "cuda")]
fn commit_witness_stage_source_devices_and_indexed_timing_inner(
    source_devices: &[WitnessStageSourceDevice],
    unit: &ProveUnitSchedule,
    timing: &mut WitnessStageCommitTiming,
    external_source_required: bool,
) -> Result<
    (
        WitnessTraceCommitments,
        Vec<WitnessIndexedStageCommitTiming>,
    ),
    WitnessTraceCommitmentError,
> {
    let params = WitnessStageCommitParams::from_unit(unit)?;
    let mut commitments = Vec::with_capacity(source_devices.len());
    let mut stage_timings = Vec::with_capacity(source_devices.len());
    for source_device in source_devices {
        let mut stage_timing = WitnessStageCommitTiming::default();
        let commitment = commit_extended_witness_stage_source_device(
            source_device,
            params,
            Some(&mut stage_timing),
            external_source_required,
        )?;
        timing.accumulate(stage_timing);
        stage_timings.push(WitnessIndexedStageCommitTiming::new(
            source_device.stage_index(),
            stage_timing,
        ));
        commitments.push((source_device.stage_index(), commitment));
    }
    commitments.sort_by_key(|(stage_index, _)| *stage_index);
    stage_timings.sort_by_key(|timing| timing.stage_index());
    Ok((
        WitnessTraceCommitments::new(
            commitments
                .into_iter()
                .map(|(_, commitment)| commitment)
                .collect(),
        ),
        stage_timings,
    ))
}

#[cfg(feature = "cuda")]
fn commit_witness_stage_values_with_workers_and_timing_inner(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
    source_devices: &[WitnessStageSourceDevice],
    timing: &mut WitnessStageCommitTiming,
) -> Result<
    (
        WitnessTraceCommitments,
        Vec<WitnessIndexedStageCommitTiming>,
    ),
    WitnessTraceCommitmentError,
> {
    let worker_count = worker_count.max(1);
    let params = WitnessStageCommitParams::from_unit(unit)?;

    if worker_count == 1 || stages.len() <= 1 {
        let mut commitments = Vec::with_capacity(stages.len());
        let mut stage_timings = Vec::with_capacity(stages.len());
        for stage in stages {
            let mut stage_timing = WitnessStageCommitTiming::default();
            let commitment = commit_extended_witness_stage(
                stage,
                params,
                find_stage_source_device(source_devices, stage.stage_index()),
                Some(&mut stage_timing),
            )?;
            timing.accumulate(stage_timing);
            stage_timings.push(WitnessIndexedStageCommitTiming::new(
                stage.stage_index(),
                stage_timing,
            ));
            commitments.push(commitment);
        }
        return Ok((WitnessTraceCommitments::new(commitments), stage_timings));
    }

    let worker_count = worker_count.min(stages.len());
    let chunk_size = stages.len().div_ceil(worker_count);
    let mut commitments = Vec::with_capacity(stages.len());
    let mut stage_timings = Vec::with_capacity(stages.len());
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in stages.chunks(chunk_size) {
            handles.push(scope.spawn(move || {
                let mut out = Vec::with_capacity(chunk.len());
                let mut out_timings = Vec::with_capacity(chunk.len());
                let mut chunk_timing = WitnessStageCommitTiming::default();
                for stage in chunk {
                    let mut stage_timing = WitnessStageCommitTiming::default();
                    let commitment = commit_extended_witness_stage(
                        stage,
                        params,
                        find_stage_source_device(source_devices, stage.stage_index()),
                        Some(&mut stage_timing),
                    )?;
                    chunk_timing.accumulate(stage_timing);
                    out.push((stage.stage_index(), commitment));
                    out_timings.push(WitnessIndexedStageCommitTiming::new(
                        stage.stage_index(),
                        stage_timing,
                    ));
                }
                Ok::<_, WitnessTraceCommitmentError>((out, out_timings, chunk_timing))
            }));
        }

        for handle in handles {
            let (chunk, chunk_stage_timings, chunk_timing) = handle
                .join()
                .map_err(|_| WitnessTraceCommitmentError::WorkerPanic)??;
            commitments.extend(chunk);
            stage_timings.extend(chunk_stage_timings);
            timing.accumulate(chunk_timing);
        }
        Ok::<(), WitnessTraceCommitmentError>(())
    })?;

    commitments.sort_by_key(|(stage_index, _)| *stage_index);
    stage_timings.sort_by_key(|timing| timing.stage_index());
    Ok((
        WitnessTraceCommitments::new(
            commitments
                .into_iter()
                .map(|(_, commitment)| commitment)
                .collect(),
        ),
        stage_timings,
    ))
}

#[cfg(not(feature = "cuda"))]
fn commit_witness_stage_values_with_workers_and_timing_inner(
    stages: &[WitnessTraceStageValues],
    unit: &ProveUnitSchedule,
    worker_count: usize,
    timing: &mut WitnessStageCommitTiming,
) -> Result<
    (
        WitnessTraceCommitments,
        Vec<WitnessIndexedStageCommitTiming>,
    ),
    WitnessTraceCommitmentError,
> {
    let worker_count = worker_count.max(1);
    let params = WitnessStageCommitParams::from_unit(unit)?;

    if worker_count == 1 || stages.len() <= 1 {
        let mut commitments = Vec::with_capacity(stages.len());
        let mut stage_timings = Vec::with_capacity(stages.len());
        for stage in stages {
            let mut stage_timing = WitnessStageCommitTiming::default();
            let commitment =
                commit_extended_witness_stage(stage, params, None, Some(&mut stage_timing))?;
            timing.accumulate(stage_timing);
            stage_timings.push(WitnessIndexedStageCommitTiming::new(
                stage.stage_index(),
                stage_timing,
            ));
            commitments.push(commitment);
        }
        return Ok((WitnessTraceCommitments::new(commitments), stage_timings));
    }

    let worker_count = worker_count.min(stages.len());
    let chunk_size = stages.len().div_ceil(worker_count);
    let mut commitments = Vec::with_capacity(stages.len());
    let mut stage_timings = Vec::with_capacity(stages.len());
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in stages.chunks(chunk_size) {
            handles.push(scope.spawn(move || {
                let mut out = Vec::with_capacity(chunk.len());
                let mut out_timings = Vec::with_capacity(chunk.len());
                let mut chunk_timing = WitnessStageCommitTiming::default();
                for stage in chunk {
                    let mut stage_timing = WitnessStageCommitTiming::default();
                    let commitment = commit_extended_witness_stage(
                        stage,
                        params,
                        None,
                        Some(&mut stage_timing),
                    )?;
                    chunk_timing.accumulate(stage_timing);
                    out.push((stage.stage_index(), commitment));
                    out_timings.push(WitnessIndexedStageCommitTiming::new(
                        stage.stage_index(),
                        stage_timing,
                    ));
                }
                Ok::<_, WitnessTraceCommitmentError>((out, out_timings, chunk_timing))
            }));
        }

        for handle in handles {
            let (chunk, chunk_stage_timings, chunk_timing) = handle
                .join()
                .map_err(|_| WitnessTraceCommitmentError::WorkerPanic)??;
            commitments.extend(chunk);
            stage_timings.extend(chunk_stage_timings);
            timing.accumulate(chunk_timing);
        }
        Ok::<(), WitnessTraceCommitmentError>(())
    })?;

    commitments.sort_by_key(|(stage_index, _)| *stage_index);
    stage_timings.sort_by_key(|timing| timing.stage_index());
    Ok((
        WitnessTraceCommitments::new(
            commitments
                .into_iter()
                .map(|(_, commitment)| commitment)
                .collect(),
        ),
        stage_timings,
    ))
}

#[cfg(feature = "cuda")]
fn commit_extended_witness_stage_source_device(
    source_device: &WitnessStageSourceDevice,
    params: WitnessStageCommitParams,
    mut timing: Option<&mut WitnessStageCommitTiming>,
    external_source_required: bool,
) -> Result<super::WitnessStageCommitment, WitnessTraceCommitmentError> {
    let source_view = source_device.source_view();
    let retained_source_device = if external_source_required {
        None
    } else {
        Some(source_view.clone())
    };
    let leaf_level = if let Some(timing) = timing.as_deref_mut() {
        let leaf_extend_duration = &mut timing.leaf_extend_duration;
        let leaf_extend_timing = &mut timing.leaf_extend_timing;
        record_optional_duration(Some(leaf_extend_duration), || {
            compact_witness_stage_leaf_hash_level_from_source_device_view_timing(
                source_device.row_count(),
                source_device.column_count(),
                params.source_bits,
                params.target_bits,
                params.arity,
                &source_view,
                leaf_extend_timing,
            )
        })?
    } else {
        let mut leaf_extend_timing = WitnessStageLeafExtendTiming::default();
        compact_witness_stage_leaf_hash_level_from_source_device_view_timing(
            source_device.row_count(),
            source_device.column_count(),
            params.source_bits,
            params.target_bits,
            params.arity,
            &source_view,
            &mut leaf_extend_timing,
        )?
    };
    record_optional_duration(
        timing
            .as_mut()
            .map(|timing| &mut timing.tree_commit_duration),
        || {
            commit_witness_stage_device_compact_with_leaf_hash_level(
                WitnessStageDeviceCompactCommitInput {
                    stage_index: source_device.stage_index(),
                    source_rows: source_device.row_count(),
                    column_count: source_device.column_count(),
                    source_bits: params.source_bits,
                    target_bits: params.target_bits,
                    arity: params.arity,
                    external_source_required,
                },
                leaf_level,
                retained_source_device,
            )
            .map_err(WitnessTraceCommitmentError::from)
        },
    )
}

fn commit_extended_witness_stage(
    stage: &WitnessTraceStageValues,
    params: WitnessStageCommitParams,
    source_device: Option<WitnessStageSourceDeviceRef<'_>>,
    mut timing: Option<&mut WitnessStageCommitTiming>,
) -> Result<super::WitnessStageCommitment, WitnessTraceCommitmentError> {
    #[cfg(not(feature = "cuda"))]
    let _ = source_device;

    #[cfg(feature = "cuda")]
    {
        let leaf_level = if let Some(timing) = timing.as_deref_mut() {
            let leaf_extend_duration = &mut timing.leaf_extend_duration;
            let leaf_extend_timing = &mut timing.leaf_extend_timing;
            record_optional_duration(Some(leaf_extend_duration), || {
                if let Some(source_device) = source_device {
                    compact_witness_stage_leaf_hash_level_from_source_device_view_timing(
                        stage.row_count(),
                        stage.column_count(),
                        params.source_bits,
                        params.target_bits,
                        params.arity,
                        &source_device.source_view(),
                        leaf_extend_timing,
                    )
                } else {
                    compact_witness_stage_leaf_hash_level_with_source_device_timing(
                        stage,
                        params.source_bits,
                        params.target_bits,
                        params.arity,
                        None,
                        leaf_extend_timing,
                    )
                }
            })?
        } else {
            let mut leaf_extend_timing = WitnessStageLeafExtendTiming::default();
            if let Some(source_device) = source_device {
                compact_witness_stage_leaf_hash_level_from_source_device_view_timing(
                    stage.row_count(),
                    stage.column_count(),
                    params.source_bits,
                    params.target_bits,
                    params.arity,
                    &source_device.source_view(),
                    &mut leaf_extend_timing,
                )?
            } else {
                compact_witness_stage_leaf_hash_level_with_source_device_timing(
                    stage,
                    params.source_bits,
                    params.target_bits,
                    params.arity,
                    None,
                    &mut leaf_extend_timing,
                )?
            }
        };
        record_optional_duration(
            timing
                .as_mut()
                .map(|timing| &mut timing.tree_commit_duration),
            || {
                commit_witness_stage_leaves_compact_with_leaf_hash_level(
                    stage,
                    params.source_bits,
                    params.target_bits,
                    params.arity,
                    leaf_level,
                    source_device.map(WitnessStageSourceDevice::source_view),
                )
                .map_err(WitnessTraceCommitmentError::from)
            },
        )
    }

    #[cfg(not(feature = "cuda"))]
    {
        let leaves = record_optional_duration(
            timing
                .as_mut()
                .map(|timing| &mut timing.leaf_extend_duration),
            || {
                extend_witness_stage_leaves(stage, params.source_bits, params.target_bits)
                    .map_err(WitnessTraceCommitmentError::from)
            },
        )?;
        record_optional_duration(
            timing
                .as_mut()
                .map(|timing| &mut timing.tree_commit_duration),
            || {
                commit_witness_stage_leaves_owned(leaves, params.arity)
                    .map_err(WitnessTraceCommitmentError::from)
            },
        )
    }
}

fn record_optional_duration<T>(
    duration: Option<&mut Duration>,
    run: impl FnOnce() -> Result<T, WitnessTraceCommitmentError>,
) -> Result<T, WitnessTraceCommitmentError> {
    if let Some(duration) = duration {
        let started = Instant::now();
        let result = run()?;
        *duration += started.elapsed();
        Ok(result)
    } else {
        run()
    }
}

pub fn extend_witness_trace_stage_values(
    trace: &WitnessTraceBuffer,
    unit: &ProveUnitSchedule,
) -> Result<Vec<WitnessStageExtendedValues>, WitnessTraceCommitmentError> {
    let layout = derive_witness_trace_layout(unit)?;
    let source_bits = usize::try_from(unit.base_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;
    let target_bits = usize::try_from(unit.extended_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;

    let mut stages = Vec::with_capacity(layout.stage_count());
    for stage_info in layout.stages() {
        let stage = layout.stage_trace(trace, stage_info.stage_index)?;
        let leaves = extend_witness_stage_leaves(&stage, source_bits, target_bits)?;
        let values = decode_witness_stage_leaf_values(&leaves)?;
        stages.push(WitnessStageExtendedValues::new(
            leaves.stage_index(),
            leaves.source_row_count(),
            leaves.extended_row_count(),
            leaves.column_count(),
            values,
        ));
    }

    Ok(stages)
}

#[cfg(feature = "cuda")]
pub(crate) fn extend_witness_trace_stage_values_with_source_devices(
    trace: &WitnessTraceBuffer,
    unit: &ProveUnitSchedule,
    source_devices: &[WitnessStageRetainedSourceDevice],
) -> Result<Vec<WitnessStageExtendedValues>, WitnessTraceCommitmentError> {
    let layout = derive_witness_trace_layout(unit)?;
    if trace.row_count() != layout.row_count() || trace.column_count() != layout.column_count() {
        return Err(WitnessTraceCommitmentError::from(
            crate::witness_layout::WitnessTraceLayoutError::TraceShapeMismatch {
                expected_rows: layout.row_count(),
                expected_columns: layout.column_count(),
                found_rows: trace.row_count(),
                found_columns: trace.column_count(),
            },
        ));
    }
    let source_bits = usize::try_from(unit.base_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;
    let target_bits = usize::try_from(unit.extended_domain_bits)
        .map_err(|_| WitnessTraceCommitmentError::LengthOverflow)?;

    let mut stages = Vec::with_capacity(layout.stage_count());
    let mut source_hits = 0_usize;
    let mut source_misses = 0_usize;
    for stage_info in layout.stages() {
        let leaves = if let Some(source_device) =
            find_retained_stage_source_device(source_devices, stage_info.stage_index)
        {
            source_hits += 1;
            extend_witness_stage_leaves_from_source_device_view(
                stage_info.stage_index,
                layout.row_count(),
                stage_info.width,
                source_bits,
                target_bits,
                source_device.source_view(),
            )?
        } else {
            source_misses += 1;
            let stage = layout.stage_trace(trace, stage_info.stage_index)?;
            extend_witness_stage_leaves(&stage, source_bits, target_bits)?
        };
        let values = decode_witness_stage_leaf_values(&leaves)?;
        stages.push(WitnessStageExtendedValues::new(
            leaves.stage_index(),
            leaves.source_row_count(),
            leaves.extended_row_count(),
            leaves.column_count(),
            values,
        ));
    }

    if debug_fri_stage_source_devices() {
        eprintln!("lzvm_cuda_fri_stage_source_hits={source_hits} misses={source_misses}");
    }
    Ok(stages)
}

#[cfg(feature = "cuda")]
fn debug_fri_stage_source_devices() -> bool {
    matches!(
        std::env::var("LZVM_CUDA_FRI_STAGE_SOURCE_DEBUG").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::{
        commit_extended_witness_stage, commit_witness_stage_values_with_workers,
        commit_witness_trace_stages_with_workers, extend_witness_stage_leaves,
        WitnessStageCommitParams,
    };
    use crate::witness_commitment::commit_witness_stage_leaves;
    use crate::witness_layout::derive_witness_trace_layout;
    use crate::witness_trace::WitnessTraceBuffer;
    use crate::{KeyUnitKind, PcsFriLayer, ProveUnitSchedule};
    use lzvm_field::Felt;

    #[test]
    fn cuda_combined_witness_stage_commitment_matches_separate_path() {
        assert_combined_witness_stage_commitment_matches_separate_path(3);
        assert_combined_witness_stage_commitment_matches_separate_path(6);
    }

    #[test]
    fn cuda_cached_multi_worker_stage_commitments_match_trace_path() {
        let unit = sample_unit(4, vec![2, 3]);
        let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
        let value_count = 4 * 5;
        let values = (0..value_count)
            .map(|value| Felt::from_u64(value as u64))
            .collect::<Vec<_>>();
        let trace =
            WitnessTraceBuffer::from_values(4, 5, values).expect("trace shape should be valid");
        let stages = layout
            .stages()
            .iter()
            .map(|stage| layout.stage_trace(&trace, stage.stage_index))
            .collect::<Result<Vec<_>, _>>()
            .expect("stages should extract");

        let cached = commit_witness_stage_values_with_workers(&stages, &unit, 2)
            .expect("cached commitments should build");
        let trace_based = commit_witness_trace_stages_with_workers(&trace, &unit, 2)
            .expect("trace commitments should build");

        assert_eq!(cached, trace_based);
    }

    fn assert_combined_witness_stage_commitment_matches_separate_path(width: u32) {
        let unit = sample_unit(4, vec![width]);
        let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
        let value_count = 4 * usize::try_from(width).expect("stage width fits");
        let values = (0..value_count)
            .map(|value| Felt::from_u64(value as u64))
            .collect::<Vec<_>>();
        let trace = WitnessTraceBuffer::from_values(4, usize::try_from(width).unwrap(), values)
            .expect("trace shape should be valid");
        let stage = layout
            .stage_trace(&trace, 1)
            .expect("stage should be present");
        let arity = usize::try_from(unit.merkle_tree_arity).expect("arity fits");
        let params = WitnessStageCommitParams::from_unit(&unit).expect("params should derive");

        let separate_leaves =
            extend_witness_stage_leaves(&stage, params.source_bits, params.target_bits)
                .expect("stage extends");
        let separate = commit_witness_stage_leaves(&separate_leaves, arity)
            .expect("separate commitment should build");
        let combined = commit_extended_witness_stage(&stage, params, None, None)
            .expect("combined commitment should build");

        assert_eq!(combined.stage_index(), separate.stage_index());
        assert_eq!(combined.arity(), separate.arity());
        assert_eq!(combined.root(), separate.root());
        assert_eq!(combined.tree_bytes(), separate.tree_bytes());
    }

    fn sample_unit(rows: u64, stage_commit_widths: Vec<u32>) -> ProveUnitSchedule {
        let mut transcript_root_challenge_draws = vec![1; stage_commit_widths.len()];
        if let Some(first) = transcript_root_challenge_draws.first_mut() {
            *first = 2;
        }
        ProveUnitSchedule {
            kind: KeyUnitKind::Basic,
            group_id: Some(0),
            unit_id: Some(0),
            group_name: Some("group-a".to_owned()),
            unit_name: Some("unit-a".to_owned()),
            base_domain_bits: 2,
            extended_domain_bits: 3,
            base_domain_size: rows,
            extended_domain_size: 8,
            blowup_factor: 2,
            query_count: 2,
            proof_of_work_bits: 0,
            merkle_tree_arity: 4,
            last_level_verification: 0,
            transcript_arity: Some(4),
            hash_commits: true,
            transcript_root_challenge_draws,
            challenge_count: 6,
            evaluation_value_count: 2,
            evaluation_map: Vec::new(),
            transcript_evaluation_challenge_draws: 2,
            constant_width: 1,
            stage_commit_widths,
            commitment_columns: Vec::new(),
            unit_value_map: Vec::new(),
            group_value_map: Vec::new(),
            opening_points: vec![0],
            fri_layers: vec![PcsFriLayer {
                input_bits: 3,
                output_bits: 1,
                folding_factor: 4,
            }],
            final_layer_bits: 1,
            fixed_bytes: 16,
            constant_tree_root: None,
            pcs_material_bytes: None,
            pcs_material_plan_digest: None,
            pcs_material_fixed_column_digest: None,
            pcs_material_constant_tree_digest: None,
            pcs_material_constant_tree_root: None,
            pcs_material_fixed_byte_count: None,
            pcs_material_constant_tree_byte_count: None,
            pcs_material_leaf_byte_count: None,
            pcs_material_node_byte_count: None,
        }
    }
}

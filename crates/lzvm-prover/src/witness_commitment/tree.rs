use lzvm_field::Felt;

#[cfg(feature = "cuda")]
use std::time::{Duration, Instant};

#[cfg(feature = "cuda")]
use super::PendingCanonicalCudaDigestLevel;
use crate::merkle_hash::{
    linear_hash, linear_hashes_from_row_major_bytes, parent_hash, parent_levels_from_digest_level,
};
#[cfg(feature = "cuda")]
use crate::merkle_hash::{
    synchronize_cuda_digest_root_materializations, CudaDigestCheckpointLevel, CudaDigestRoot,
    PendingCudaDigestRootMaterialization,
};
use crate::witness_layout::WitnessTraceStageValues;

#[cfg(feature = "cuda")]
use super::WitnessStageOpeningWorkTiming;
#[cfg(feature = "cuda")]
use super::{
    retain_leaf_digest_level, retain_parent_checkpoint_level, retain_source_device_view,
    WitnessStageSourceDeviceView,
};
use super::{
    WitnessStageCommitment, WitnessStageCommitmentError, WitnessStageCompactTreeParts,
    WitnessStageLeaves, WitnessStageOpening, WitnessStageOpeningError, HASH_WORDS, WORD_BYTES,
};

#[cfg(feature = "cuda")]
const RETAINED_PARENT_CHECKPOINT_MAX_STATES: usize = 524288;

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WitnessStageTreeCommitTiming {
    pub(crate) checkpoint_duration: Duration,
    pub(crate) root_duration: Duration,
    pub(crate) root_count: usize,
    pub(crate) root_byte_count: usize,
    pub(crate) root_materialization_group_count: usize,
    pub(crate) root_materialization_max_group_size: usize,
    pub(crate) retain_duration: Duration,
}

#[cfg(feature = "cuda")]
pub(crate) struct PendingCudaWitnessStageCommitment {
    stage_index: usize,
    arity: usize,
    root: CudaDigestRoot,
    leaf_level: PendingCanonicalCudaDigestLevel,
    parent_checkpoint: Option<CudaDigestCheckpointLevel>,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    source_bits: usize,
    target_bits: usize,
    raw_leaf_bytes: usize,
    logical_tree_bytes: usize,
    external_source_required: bool,
    expected_source_bytes: usize,
}

#[cfg(feature = "cuda")]
pub(crate) struct PendingCudaWitnessStageCommitmentMaterialization {
    stage_index: usize,
    arity: usize,
    root: PendingCudaDigestRootMaterialization,
    leaf_level: PendingCanonicalCudaDigestLevel,
    parent_checkpoint: Option<CudaDigestCheckpointLevel>,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
    source_rows: usize,
    extended_rows: usize,
    columns: usize,
    source_bits: usize,
    target_bits: usize,
    raw_leaf_bytes: usize,
    logical_tree_bytes: usize,
    external_source_required: bool,
    expected_source_bytes: usize,
}

#[cfg(feature = "cuda")]
impl PendingCudaWitnessStageCommitment {
    pub(crate) fn begin_materialize_with_timing(
        self,
        timing: &mut WitnessStageTreeCommitTiming,
    ) -> Result<PendingCudaWitnessStageCommitmentMaterialization, WitnessStageCommitmentError> {
        let Self {
            stage_index,
            arity,
            root,
            leaf_level,
            parent_checkpoint,
            retained_source_device,
            source_rows,
            extended_rows,
            columns,
            source_bits,
            target_bits,
            raw_leaf_bytes,
            logical_tree_bytes,
            external_source_required,
            expected_source_bytes,
        } = self;
        let root = record_stage_tree_commit_duration(Some(&mut timing.root_duration), || {
            root.begin_materialize_on_default_stream()
                .map_err(WitnessStageCommitmentError::from)
        })?;
        timing.root_count += 1;
        timing.root_byte_count += HASH_WORDS * WORD_BYTES;
        Ok(PendingCudaWitnessStageCommitmentMaterialization {
            stage_index,
            arity,
            root,
            leaf_level,
            parent_checkpoint,
            retained_source_device,
            source_rows,
            extended_rows,
            columns,
            source_bits,
            target_bits,
            raw_leaf_bytes,
            logical_tree_bytes,
            external_source_required,
            expected_source_bytes,
        })
    }
}

#[cfg(feature = "cuda")]
impl PendingCudaWitnessStageCommitmentMaterialization {
    pub(crate) fn finish_after_root_synchronize(
        self,
        timing: &mut WitnessStageTreeCommitTiming,
    ) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
        let root = record_stage_tree_commit_duration(Some(&mut timing.root_duration), || {
            self.root
                .finish_after_device_synchronize()
                .map_err(WitnessStageCommitmentError::from)
        })?;
        let (retained_parent_checkpoint_level, retained_leaf_digest_level, retained_source_device) =
            record_stage_tree_commit_duration(Some(&mut timing.retain_duration), || {
                let validated_level = self.leaf_level.into_validated_level();
                let leaf_level = validated_level.map_err(WitnessStageCommitmentError::from)?;
                let retained_parent_checkpoint_level =
                    retain_parent_checkpoint_level(self.parent_checkpoint, self.columns);
                let retained_leaf_digest_level = retain_leaf_digest_level(leaf_level, self.columns);
                let retained_source_device = match self.retained_source_device {
                    Some(view) => {
                        validate_source_device_view(&view, self.source_rows, self.columns)?;
                        retain_source_device_view(view)
                    }
                    None => None,
                };
                Ok::<_, WitnessStageCommitmentError>((
                    retained_parent_checkpoint_level,
                    retained_leaf_digest_level,
                    retained_source_device,
                ))
            })?;
        if retained_source_device.is_none() && !self.external_source_required {
            return Err(
                WitnessStageCommitmentError::SourceDeviceRetentionUnavailable {
                    bytes: self.expected_source_bytes,
                },
            );
        }
        Ok(WitnessStageCommitment::new_compact(
            self.stage_index,
            self.arity,
            root,
            WitnessStageCompactTreeParts {
                source_rows: self.source_rows,
                extended_rows: self.extended_rows,
                columns: self.columns,
                source_bits: self.source_bits,
                target_bits: self.target_bits,
                arity: self.arity,
                source_values: Vec::new(),
                raw_leaf_bytes: self.raw_leaf_bytes,
                logical_tree_bytes: self.logical_tree_bytes,
                digest_tree: None,
                zero_source: false,
                external_source_required: retained_source_device.is_none()
                    && self.external_source_required,
                retained_source_device,
                retained_leaf_digest_level,
                retained_parent_checkpoint_level,
            },
        ))
    }
}

#[cfg(feature = "cuda")]
pub(crate) fn synchronize_cuda_witness_stage_root_materializations(
    timing: &mut WitnessStageTreeCommitTiming,
) -> Result<(), WitnessStageCommitmentError> {
    record_stage_tree_commit_duration(Some(&mut timing.root_duration), || {
        synchronize_cuda_digest_root_materializations().map_err(WitnessStageCommitmentError::from)
    })
}

#[cfg(feature = "cuda")]
pub(crate) fn record_cuda_witness_stage_root_materialization_group(
    timing: &mut WitnessStageTreeCommitTiming,
    group_size: usize,
) {
    if group_size == 0 {
        return;
    }
    timing.root_materialization_group_count += 1;
    timing.root_materialization_max_group_size =
        timing.root_materialization_max_group_size.max(group_size);
}

pub fn commit_witness_stage_leaves(
    leaves: &WitnessStageLeaves,
    arity: usize,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let expected_leaf_bytes = validate_witness_stage_leaves(leaves, arity)?;

    let mut out = Vec::with_capacity(expected_leaf_bytes);
    out.extend_from_slice(leaves.bytes());

    commit_validated_witness_stage_bytes(
        leaves.stage_index(),
        leaves.extended_row_count(),
        leaves.column_count(),
        arity,
        expected_leaf_bytes,
        out,
    )
}

#[cfg_attr(feature = "cuda", allow(dead_code))]
pub(crate) fn commit_witness_stage_leaves_owned(
    leaves: WitnessStageLeaves,
    arity: usize,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let expected_leaf_bytes = validate_witness_stage_leaves(&leaves, arity)?;
    let stage_index = leaves.stage_index();
    let extended_row_count = leaves.extended_row_count();
    let column_count = leaves.column_count();
    let out = leaves.into_bytes();

    commit_validated_witness_stage_bytes(
        stage_index,
        extended_row_count,
        column_count,
        arity,
        expected_leaf_bytes,
        out,
    )
}

#[cfg(test)]
pub(crate) fn commit_witness_stage_leaves_owned_with_leaf_hashes(
    leaves: WitnessStageLeaves,
    arity: usize,
    leaf_hashes: Vec<[Felt; HASH_WORDS]>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    validate_witness_stage_leaves(&leaves, arity)?;
    let expected_leaf_hashes = leaves.extended_row_count();
    if leaf_hashes.len() != expected_leaf_hashes {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: expected_leaf_hashes,
            found: leaf_hashes.len(),
        });
    }
    let stage_index = leaves.stage_index();
    let extended_row_count = leaves.extended_row_count();
    let column_count = leaves.column_count();
    let out = leaves.into_bytes();

    commit_validated_witness_stage_bytes_with_leaf_hashes(
        stage_index,
        extended_row_count,
        column_count,
        arity,
        out,
        leaf_hashes,
    )
}

#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
#[cfg_attr(feature = "cuda", allow(dead_code))]
pub(crate) fn commit_witness_stage_leaves_compact_with_leaf_hashes(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    leaf_hashes: Vec<[Felt; HASH_WORDS]>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    validate_witness_commitment_arity(arity)?;
    let source_rows = checked_domain_rows(source_bits)?;
    let extended_rows = checked_domain_rows(target_bits)?;
    if target_bits < source_bits {
        return Err(WitnessStageCommitmentError::LengthOverflow);
    }
    if stage.row_count() != source_rows {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: source_rows,
            found: stage.row_count(),
        });
    }
    let column_count = stage.column_count();
    if column_count == 0 || extended_rows == 0 {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }
    let expected_source_values = source_rows
        .checked_mul(column_count)
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if stage.values().len() != expected_source_values {
        return Err(WitnessStageCommitmentError::InvalidLeafByteLength {
            expected: expected_source_values,
            found: stage.values().len(),
        });
    }
    if leaf_hashes.len() != extended_rows {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: extended_rows,
            found: leaf_hashes.len(),
        });
    }
    let raw_leaf_bytes = extended_rows
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let logical_tree_bytes =
        expected_witness_stage_commitment_tree_byte_count(extended_rows, column_count, arity)?;
    let mut digest_tree = Vec::with_capacity(logical_tree_bytes.saturating_sub(raw_leaf_bytes));
    let root = append_digest_tree_bytes(&mut digest_tree, leaf_hashes, arity)?;
    if raw_leaf_bytes
        .checked_add(digest_tree.len())
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?
        != logical_tree_bytes
    {
        return Err(WitnessStageCommitmentError::LengthOverflow);
    }
    Ok(WitnessStageCommitment::new_compact(
        stage.stage_index(),
        arity,
        root,
        WitnessStageCompactTreeParts {
            source_rows,
            extended_rows,
            columns: column_count,
            source_bits,
            target_bits,
            arity,
            source_values: stage.values().to_vec(),
            raw_leaf_bytes,
            logical_tree_bytes,
            digest_tree: Some(digest_tree),
            zero_source: false,
            external_source_required: false,
            #[cfg(feature = "cuda")]
            retained_source_device: None,
            #[cfg(feature = "cuda")]
            retained_leaf_digest_level: None,
            #[cfg(feature = "cuda")]
            retained_parent_checkpoint_level: None,
        },
    ))
}

#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
pub(crate) fn commit_witness_stage_zero_compact(
    stage_index: usize,
    source_bits: usize,
    target_bits: usize,
    column_count: usize,
    arity: usize,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    validate_witness_commitment_arity(arity)?;
    let source_rows = checked_domain_rows(source_bits)?;
    let extended_rows = checked_domain_rows(target_bits)?;
    if target_bits < source_bits {
        return Err(WitnessStageCommitmentError::LengthOverflow);
    }
    if column_count == 0 || extended_rows == 0 {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }
    let raw_leaf_bytes = extended_rows
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let logical_tree_bytes =
        expected_witness_stage_commitment_tree_byte_count(extended_rows, column_count, arity)?;
    let root = zero_compact_stage_root(extended_rows, column_count, arity)?;
    Ok(WitnessStageCommitment::new_compact(
        stage_index,
        arity,
        root,
        WitnessStageCompactTreeParts {
            source_rows,
            extended_rows,
            columns: column_count,
            source_bits,
            target_bits,
            arity,
            source_values: Vec::new(),
            raw_leaf_bytes,
            logical_tree_bytes,
            digest_tree: None,
            zero_source: true,
            external_source_required: false,
            #[cfg(feature = "cuda")]
            retained_source_device: None,
            #[cfg(feature = "cuda")]
            retained_leaf_digest_level: None,
            #[cfg(feature = "cuda")]
            retained_parent_checkpoint_level: None,
        },
    ))
}

#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
fn zero_compact_stage_root(
    extended_rows: usize,
    column_count: usize,
    arity: usize,
) -> Result<[Felt; HASH_WORDS], WitnessStageCommitmentError> {
    let mut digest = linear_hash(&vec![Felt::ZERO; column_count], arity)?;
    let mut level_len = extended_rows;
    while level_len > 1 {
        let padded_len = round_up_to_arity(
            level_len,
            arity,
            WitnessStageCommitmentError::LengthOverflow,
        )?;
        let mut children = vec![[Felt::ZERO; HASH_WORDS]; arity];
        let active = level_len.min(arity);
        for child in children.iter_mut().take(active) {
            *child = digest;
        }
        digest = parent_hash(&children, arity)?;
        level_len = padded_len / arity;
    }
    Ok(digest)
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_leaves_compact_with_leaf_hash_level(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    leaf_level: PendingCanonicalCudaDigestLevel,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    commit_witness_stage_leaves_compact_with_leaf_hash_level_inner(
        stage,
        source_bits,
        target_bits,
        arity,
        leaf_level,
        retained_source_device,
        None,
    )
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_leaves_compact_with_leaf_hash_level_timing(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    leaf_level: PendingCanonicalCudaDigestLevel,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
    timing: &mut WitnessStageTreeCommitTiming,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    commit_witness_stage_leaves_compact_with_leaf_hash_level_inner(
        stage,
        source_bits,
        target_bits,
        arity,
        leaf_level,
        retained_source_device,
        Some(timing),
    )
}

#[cfg(feature = "cuda")]
fn commit_witness_stage_leaves_compact_with_leaf_hash_level_inner(
    stage: &WitnessTraceStageValues,
    source_bits: usize,
    target_bits: usize,
    arity: usize,
    leaf_level: PendingCanonicalCudaDigestLevel,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
    mut timing: Option<&mut WitnessStageTreeCommitTiming>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    validate_witness_commitment_arity(arity)?;
    let source_rows = checked_domain_rows(source_bits)?;
    let extended_rows = checked_domain_rows(target_bits)?;
    if target_bits < source_bits {
        return Err(WitnessStageCommitmentError::LengthOverflow);
    }
    if stage.row_count() != source_rows {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: source_rows,
            found: stage.row_count(),
        });
    }
    let column_count = stage.column_count();
    if column_count == 0 || extended_rows == 0 {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }
    let expected_source_values = source_rows
        .checked_mul(column_count)
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if stage.values().len() != expected_source_values {
        return Err(WitnessStageCommitmentError::InvalidLeafByteLength {
            expected: expected_source_values,
            found: stage.values().len(),
        });
    }
    if leaf_level.arity() != arity || leaf_level.state_count() != extended_rows {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: extended_rows,
            found: leaf_level.state_count(),
        });
    }
    let raw_leaf_bytes = extended_rows
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let logical_tree_bytes =
        expected_witness_stage_commitment_tree_byte_count(extended_rows, column_count, arity)?;
    let parent_checkpoint = record_stage_tree_commit_duration(
        timing
            .as_mut()
            .map(|timing| &mut timing.checkpoint_duration),
        || {
            leaf_level
                .parent_checkpoint_level(RETAINED_PARENT_CHECKPOINT_MAX_STATES)
                .map_err(WitnessStageCommitmentError::from)
        },
    )?;
    let root = record_stage_tree_commit_duration(
        timing.as_mut().map(|timing| &mut timing.root_duration),
        || {
            match parent_checkpoint.as_ref() {
                Some(checkpoint) => checkpoint.root(),
                None => leaf_level.root(),
            }
            .map_err(WitnessStageCommitmentError::from)
        },
    )?;
    if let Some(timing) = timing.as_mut() {
        timing.root_count += 1;
        timing.root_byte_count += HASH_WORDS * WORD_BYTES;
    }
    let (retained_parent_checkpoint_level, retained_leaf_digest_level, retained_source_device) =
        record_stage_tree_commit_duration(
            timing.as_mut().map(|timing| &mut timing.retain_duration),
            || {
                let validated_level = leaf_level.into_validated_level();
                let leaf_level = validated_level.map_err(WitnessStageCommitmentError::from)?;
                let retained_parent_checkpoint_level =
                    retain_parent_checkpoint_level(parent_checkpoint, column_count);
                let retained_leaf_digest_level = retain_leaf_digest_level(leaf_level, column_count);
                let retained_source_device = retained_source_device
                    .filter(|view| {
                        validate_source_device_view(view, source_rows, column_count).is_ok()
                    })
                    .and_then(retain_source_device_view);
                Ok::<_, WitnessStageCommitmentError>((
                    retained_parent_checkpoint_level,
                    retained_leaf_digest_level,
                    retained_source_device,
                ))
            },
        )?;
    let source_values = if retained_source_device.as_ref().is_some() {
        Vec::new()
    } else {
        stage.values().to_vec()
    };
    Ok(WitnessStageCommitment::new_compact(
        stage.stage_index(),
        arity,
        root,
        WitnessStageCompactTreeParts {
            source_rows,
            extended_rows,
            columns: column_count,
            source_bits,
            target_bits,
            arity,
            source_values,
            raw_leaf_bytes,
            logical_tree_bytes,
            digest_tree: None,
            zero_source: false,
            external_source_required: false,
            retained_source_device,
            retained_leaf_digest_level,
            retained_parent_checkpoint_level,
        },
    ))
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WitnessStageDeviceCompactCommitInput {
    pub(crate) stage_index: usize,
    pub(crate) source_rows: usize,
    pub(crate) column_count: usize,
    pub(crate) source_bits: usize,
    pub(crate) target_bits: usize,
    pub(crate) arity: usize,
    pub(crate) external_source_required: bool,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) fn commit_witness_stage_device_compact_with_leaf_hash_level(
    input: WitnessStageDeviceCompactCommitInput,
    leaf_level: PendingCanonicalCudaDigestLevel,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    commit_witness_stage_device_compact_with_leaf_hash_level_inner(
        input,
        leaf_level,
        retained_source_device,
        None,
    )
}

#[cfg(feature = "cuda")]
pub(crate) fn commit_witness_stage_device_compact_with_leaf_hash_level_pending_timing(
    input: WitnessStageDeviceCompactCommitInput,
    leaf_level: PendingCanonicalCudaDigestLevel,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
    timing: &mut WitnessStageTreeCommitTiming,
) -> Result<PendingCudaWitnessStageCommitment, WitnessStageCommitmentError> {
    commit_witness_stage_device_compact_with_leaf_hash_level_pending_inner(
        input,
        leaf_level,
        retained_source_device,
        Some(timing),
    )
}

#[cfg(feature = "cuda")]
fn commit_witness_stage_device_compact_with_leaf_hash_level_pending_inner(
    input: WitnessStageDeviceCompactCommitInput,
    leaf_level: PendingCanonicalCudaDigestLevel,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
    mut timing: Option<&mut WitnessStageTreeCommitTiming>,
) -> Result<PendingCudaWitnessStageCommitment, WitnessStageCommitmentError> {
    let WitnessStageDeviceCompactCommitInput {
        stage_index,
        source_rows,
        column_count,
        source_bits,
        target_bits,
        arity,
        external_source_required,
    } = input;
    validate_witness_commitment_arity(arity)?;
    let expected_source_rows = checked_domain_rows(source_bits)?;
    let extended_rows = checked_domain_rows(target_bits)?;
    if target_bits < source_bits {
        return Err(WitnessStageCommitmentError::LengthOverflow);
    }
    if source_rows != expected_source_rows {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: expected_source_rows,
            found: source_rows,
        });
    }
    if column_count == 0 || extended_rows == 0 {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }
    let expected_source_values = source_rows
        .checked_mul(column_count)
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if leaf_level.arity() != arity || leaf_level.state_count() != extended_rows {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: extended_rows,
            found: leaf_level.state_count(),
        });
    }
    let raw_leaf_bytes = extended_rows
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let logical_tree_bytes =
        expected_witness_stage_commitment_tree_byte_count(extended_rows, column_count, arity)?;
    let expected_source_bytes = expected_source_values
        .checked_mul(WORD_BYTES)
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let parent_checkpoint = record_stage_tree_commit_duration(
        timing
            .as_mut()
            .map(|timing| &mut timing.checkpoint_duration),
        || {
            leaf_level
                .parent_checkpoint_level(RETAINED_PARENT_CHECKPOINT_MAX_STATES)
                .map_err(WitnessStageCommitmentError::from)
        },
    )?;
    let root = record_stage_tree_commit_duration(
        timing.as_mut().map(|timing| &mut timing.root_duration),
        || {
            match parent_checkpoint.as_ref() {
                Some(checkpoint) => checkpoint.root_device(),
                None => leaf_level.root_device(),
            }
            .map_err(WitnessStageCommitmentError::from)
        },
    )?;
    Ok(PendingCudaWitnessStageCommitment {
        stage_index,
        arity,
        root,
        leaf_level,
        parent_checkpoint,
        retained_source_device,
        source_rows,
        extended_rows,
        columns: column_count,
        source_bits,
        target_bits,
        raw_leaf_bytes,
        logical_tree_bytes,
        external_source_required,
        expected_source_bytes,
    })
}

#[cfg(feature = "cuda")]
fn commit_witness_stage_device_compact_with_leaf_hash_level_inner(
    input: WitnessStageDeviceCompactCommitInput,
    leaf_level: PendingCanonicalCudaDigestLevel,
    retained_source_device: Option<WitnessStageSourceDeviceView>,
    mut timing: Option<&mut WitnessStageTreeCommitTiming>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let WitnessStageDeviceCompactCommitInput {
        stage_index,
        source_rows,
        column_count,
        source_bits,
        target_bits,
        arity,
        external_source_required,
    } = input;
    validate_witness_commitment_arity(arity)?;
    let expected_source_rows = checked_domain_rows(source_bits)?;
    let extended_rows = checked_domain_rows(target_bits)?;
    if target_bits < source_bits {
        return Err(WitnessStageCommitmentError::LengthOverflow);
    }
    if source_rows != expected_source_rows {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: expected_source_rows,
            found: source_rows,
        });
    }
    if column_count == 0 || extended_rows == 0 {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }
    let expected_source_values = source_rows
        .checked_mul(column_count)
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if leaf_level.arity() != arity || leaf_level.state_count() != extended_rows {
        return Err(WitnessStageCommitmentError::InvalidLeafDigestCount {
            expected: extended_rows,
            found: leaf_level.state_count(),
        });
    }
    let raw_leaf_bytes = extended_rows
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let logical_tree_bytes =
        expected_witness_stage_commitment_tree_byte_count(extended_rows, column_count, arity)?;
    let expected_source_bytes = expected_source_values
        .checked_mul(WORD_BYTES)
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let parent_checkpoint = record_stage_tree_commit_duration(
        timing
            .as_mut()
            .map(|timing| &mut timing.checkpoint_duration),
        || {
            leaf_level
                .parent_checkpoint_level(RETAINED_PARENT_CHECKPOINT_MAX_STATES)
                .map_err(WitnessStageCommitmentError::from)
        },
    )?;
    let root = record_stage_tree_commit_duration(
        timing.as_mut().map(|timing| &mut timing.root_duration),
        || {
            match parent_checkpoint.as_ref() {
                Some(checkpoint) => checkpoint.root(),
                None => leaf_level.root(),
            }
            .map_err(WitnessStageCommitmentError::from)
        },
    )?;
    if let Some(timing) = timing.as_mut() {
        timing.root_count += 1;
        timing.root_byte_count += HASH_WORDS * WORD_BYTES;
    }
    let (retained_parent_checkpoint_level, retained_leaf_digest_level, retained_source_device) =
        record_stage_tree_commit_duration(
            timing.as_mut().map(|timing| &mut timing.retain_duration),
            || {
                let validated_level = leaf_level.into_validated_level();
                let leaf_level = validated_level.map_err(WitnessStageCommitmentError::from)?;
                let retained_parent_checkpoint_level =
                    retain_parent_checkpoint_level(parent_checkpoint, column_count);
                let retained_leaf_digest_level = retain_leaf_digest_level(leaf_level, column_count);
                let retained_source_device = match retained_source_device {
                    Some(view) => {
                        validate_source_device_view(&view, source_rows, column_count)?;
                        retain_source_device_view(view)
                    }
                    None => None,
                };
                Ok::<_, WitnessStageCommitmentError>((
                    retained_parent_checkpoint_level,
                    retained_leaf_digest_level,
                    retained_source_device,
                ))
            },
        )?;
    if retained_source_device.is_none() && !external_source_required {
        return Err(
            WitnessStageCommitmentError::SourceDeviceRetentionUnavailable {
                bytes: expected_source_bytes,
            },
        );
    }
    Ok(WitnessStageCommitment::new_compact(
        stage_index,
        arity,
        root,
        WitnessStageCompactTreeParts {
            source_rows,
            extended_rows,
            columns: column_count,
            source_bits,
            target_bits,
            arity,
            source_values: Vec::new(),
            raw_leaf_bytes,
            logical_tree_bytes,
            digest_tree: None,
            zero_source: false,
            external_source_required: retained_source_device.is_none() && external_source_required,
            retained_source_device,
            retained_leaf_digest_level,
            retained_parent_checkpoint_level,
        },
    ))
}

#[cfg(feature = "cuda")]
fn record_stage_tree_commit_duration<T>(
    duration: Option<&mut Duration>,
    run: impl FnOnce() -> Result<T, WitnessStageCommitmentError>,
) -> Result<T, WitnessStageCommitmentError> {
    match duration {
        Some(duration) => {
            let started = Instant::now();
            let result = run();
            *duration += started.elapsed();
            result
        }
        None => run(),
    }
}

fn validate_witness_stage_leaves(
    leaves: &WitnessStageLeaves,
    arity: usize,
) -> Result<usize, WitnessStageCommitmentError> {
    validate_witness_commitment_arity(arity)?;
    if leaves.extended_row_count() == 0 {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }
    let expected_leaf_bytes = leaves
        .extended_row_count()
        .checked_mul(leaves.column_count())
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if leaves.bytes().len() != expected_leaf_bytes {
        return Err(WitnessStageCommitmentError::InvalidLeafByteLength {
            expected: expected_leaf_bytes,
            found: leaves.bytes().len(),
        });
    }

    Ok(expected_leaf_bytes)
}

fn commit_validated_witness_stage_bytes(
    stage_index: usize,
    extended_row_count: usize,
    column_count: usize,
    arity: usize,
    leaf_byte_count: usize,
    out: Vec<u8>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let level = linear_hashes_from_row_major_bytes(
        &out[..leaf_byte_count],
        extended_row_count,
        column_count,
        arity,
    )?;
    commit_validated_witness_stage_bytes_with_leaf_hashes(
        stage_index,
        extended_row_count,
        column_count,
        arity,
        out,
        level,
    )
}

fn commit_validated_witness_stage_bytes_with_leaf_hashes(
    stage_index: usize,
    extended_row_count: usize,
    column_count: usize,
    arity: usize,
    mut out: Vec<u8>,
    level: Vec<[Felt; HASH_WORDS]>,
) -> Result<WitnessStageCommitment, WitnessStageCommitmentError> {
    let tree_byte_count =
        expected_witness_stage_commitment_tree_byte_count(extended_row_count, column_count, arity)?;
    out.reserve_exact(tree_byte_count.saturating_sub(out.len()));
    let root = append_digest_tree_bytes(&mut out, level, arity)?;

    Ok(WitnessStageCommitment::new(stage_index, arity, root, out))
}

fn append_digest_tree_bytes(
    out: &mut Vec<u8>,
    mut level: Vec<[Felt; HASH_WORDS]>,
    arity: usize,
) -> Result<[Felt; HASH_WORDS], WitnessStageCommitmentError> {
    if level.is_empty() {
        return Err(WitnessStageCommitmentError::EmptyStage);
    }
    for digest in &level {
        append_digest(out, *digest);
    }

    for parent_level in parent_levels_from_digest_level(&level, arity)? {
        for _ in 0..parent_level.padding_count {
            append_digest(out, [Felt::ZERO; HASH_WORDS]);
        }

        for digest in &parent_level.parents {
            append_digest(out, *digest);
        }
        level = parent_level.parents;
    }

    Ok(level[0])
}

#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
fn checked_domain_rows(bits: usize) -> Result<usize, WitnessStageCommitmentError> {
    1_usize
        .checked_shl(u32::try_from(bits).map_err(|_| WitnessStageCommitmentError::LengthOverflow)?)
        .ok_or(WitnessStageCommitmentError::LengthOverflow)
}

#[cfg(feature = "cuda")]
fn validate_source_device_view(
    view: &WitnessStageSourceDeviceView,
    row_count: usize,
    column_count: usize,
) -> Result<(), WitnessStageCommitmentError> {
    let expected_source_bytes = row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let required_source_bytes = view
        .required_byte_len()
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    let source_column_end = view
        .column_offset()
        .checked_add(column_count)
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if !view.has_matching_shape(row_count, column_count)
        || view.row_stride() < column_count
        || source_column_end > view.row_stride()
    {
        return Err(WitnessStageCommitmentError::InvalidLeafByteLength {
            expected: expected_source_bytes,
            found: view.logical_byte_len().unwrap_or(0),
        });
    }
    if view.buffer().len() < required_source_bytes {
        return Err(WitnessStageCommitmentError::InvalidLeafByteLength {
            expected: required_source_bytes,
            found: view.buffer().len(),
        });
    }
    Ok(())
}

pub fn open_witness_stage_commitment(
    commitment: &WitnessStageCommitment,
    row_index: u64,
    row_count: u64,
    column_count: usize,
) -> Result<WitnessStageOpening, WitnessStageOpeningError> {
    open_witness_stage_commitment_inner(
        commitment,
        row_index,
        row_count,
        column_count,
        #[cfg(feature = "cuda")]
        None,
        #[cfg(feature = "cuda")]
        None,
    )
}

pub(crate) fn open_witness_stage_commitments(
    commitment: &WitnessStageCommitment,
    row_indices: &[u64],
    row_count: u64,
    column_count: usize,
) -> Result<Vec<WitnessStageOpening>, WitnessStageOpeningError> {
    open_witness_stage_commitments_inner(
        commitment,
        row_indices,
        row_count,
        column_count,
        #[cfg(feature = "cuda")]
        None,
        #[cfg(feature = "cuda")]
        None,
    )
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
pub(crate) fn open_witness_stage_commitment_with_source_device(
    commitment: &WitnessStageCommitment,
    row_index: u64,
    row_count: u64,
    column_count: usize,
    source_device: Option<&WitnessStageSourceDeviceView>,
) -> Result<WitnessStageOpening, WitnessStageOpeningError> {
    open_witness_stage_commitment_inner(
        commitment,
        row_index,
        row_count,
        column_count,
        source_device,
        None,
    )
}

#[cfg(feature = "cuda")]
pub(crate) fn open_witness_stage_commitments_with_source_device_timing(
    commitment: &WitnessStageCommitment,
    row_indices: &[u64],
    row_count: u64,
    column_count: usize,
    source_device: Option<&WitnessStageSourceDeviceView>,
    timing: &mut WitnessStageOpeningWorkTiming,
) -> Result<Vec<WitnessStageOpening>, WitnessStageOpeningError> {
    open_witness_stage_commitments_inner(
        commitment,
        row_indices,
        row_count,
        column_count,
        source_device,
        Some(timing),
    )
}

fn open_witness_stage_commitments_inner(
    commitment: &WitnessStageCommitment,
    row_indices: &[u64],
    row_count: u64,
    column_count: usize,
    #[cfg(feature = "cuda")] source_device: Option<&WitnessStageSourceDeviceView>,
    #[cfg(feature = "cuda")] mut timing: Option<&mut WitnessStageOpeningWorkTiming>,
) -> Result<Vec<WitnessStageOpening>, WitnessStageOpeningError> {
    if row_indices.is_empty() {
        return Ok(Vec::new());
    }
    validate_witness_commitment_arity(commitment.arity())?;
    if row_count == 0 {
        return Err(WitnessStageOpeningError::ZeroRows);
    }
    if column_count == 0 {
        return Err(WitnessStageOpeningError::ZeroColumns);
    }

    let rows = usize::try_from(row_count).map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
    let expected_tree_bytes =
        expected_witness_stage_opening_tree_byte_count(rows, column_count, commitment.arity())?;
    if commitment.tree_byte_count() != expected_tree_bytes {
        return Err(WitnessStageOpeningError::InvalidTreeByteLength {
            expected: expected_tree_bytes,
            found: commitment.tree_byte_count(),
        });
    }

    let mut query_rows = Vec::with_capacity(row_indices.len());
    for row_index in row_indices {
        if *row_index >= row_count {
            return Err(WitnessStageOpeningError::RowOutOfRange {
                row_index: *row_index,
                row_count,
            });
        }
        query_rows.push(
            usize::try_from(*row_index).map_err(|_| WitnessStageOpeningError::LengthOverflow)?,
        );
    }

    #[cfg(feature = "cuda")]
    let compact_openings = commitment.open_compact_batch_on_demand_with_source_device(
        &query_rows,
        rows,
        column_count,
        source_device,
        timing.as_deref_mut(),
    )?;
    #[cfg(not(feature = "cuda"))]
    let compact_openings =
        commitment.open_compact_batch_on_demand(&query_rows, rows, column_count)?;
    if let Some(openings) = compact_openings {
        return row_indices
            .iter()
            .copied()
            .zip(openings)
            .map(|(row_index, (values, siblings))| {
                WitnessStageOpening::new(row_index, values, siblings)
            })
            .collect();
    }

    let mut openings = Vec::with_capacity(row_indices.len());
    for row_index in row_indices {
        let opening = open_witness_stage_commitment_inner(
            commitment,
            *row_index,
            row_count,
            column_count,
            #[cfg(feature = "cuda")]
            source_device,
            #[cfg(feature = "cuda")]
            timing.as_deref_mut(),
        )?;
        openings.push(opening);
    }
    Ok(openings)
}

fn open_witness_stage_commitment_inner(
    commitment: &WitnessStageCommitment,
    row_index: u64,
    row_count: u64,
    column_count: usize,
    #[cfg(feature = "cuda")] source_device: Option<&WitnessStageSourceDeviceView>,
    #[cfg(feature = "cuda")] timing: Option<&mut WitnessStageOpeningWorkTiming>,
) -> Result<WitnessStageOpening, WitnessStageOpeningError> {
    validate_witness_commitment_arity(commitment.arity())?;
    if row_count == 0 {
        return Err(WitnessStageOpeningError::ZeroRows);
    }
    if column_count == 0 {
        return Err(WitnessStageOpeningError::ZeroColumns);
    }
    if row_index >= row_count {
        return Err(WitnessStageOpeningError::RowOutOfRange {
            row_index,
            row_count,
        });
    }

    let rows = usize::try_from(row_count).map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
    let query_row =
        usize::try_from(row_index).map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
    let row_byte_count = column_count
        .checked_mul(WORD_BYTES)
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let expected_tree_bytes =
        expected_witness_stage_opening_tree_byte_count(rows, column_count, commitment.arity())?;
    if commitment.tree_byte_count() != expected_tree_bytes {
        return Err(WitnessStageOpeningError::InvalidTreeByteLength {
            expected: expected_tree_bytes,
            found: commitment.tree_byte_count(),
        });
    }

    let row_offset = query_row
        .checked_mul(row_byte_count)
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    #[cfg(feature = "cuda")]
    let compact_opening = commitment.open_compact_on_demand_with_source_device(
        query_row,
        rows,
        column_count,
        source_device,
        timing,
    )?;
    #[cfg(not(feature = "cuda"))]
    let compact_opening = commitment.open_compact_on_demand(query_row, rows, column_count)?;
    if let Some((values, siblings)) = compact_opening {
        return WitnessStageOpening::new(row_index, values, siblings);
    }
    let values = commitment.read_opening_values(row_offset, row_byte_count)?;

    let mut siblings = Vec::new();
    let mut level_offset = rows
        .checked_mul(row_byte_count)
        .ok_or(WitnessStageOpeningError::LengthOverflow)?;
    let mut level_len = rows;
    let mut level_query = query_row;
    while level_len > 1 {
        let padded_len = round_up_to_arity(
            level_len,
            commitment.arity(),
            WitnessStageOpeningError::LengthOverflow,
        )?;
        let child_slot = level_query % commitment.arity();
        let group_start = (level_query / commitment.arity())
            .checked_mul(commitment.arity())
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        let mut level_siblings = Vec::with_capacity(commitment.arity() - 1);
        for slot in 0..commitment.arity() {
            if slot == child_slot {
                continue;
            }
            let child_index = group_start
                .checked_add(slot)
                .ok_or(WitnessStageOpeningError::LengthOverflow)?;
            if child_index < level_len {
                level_siblings.push(commitment.read_digest_at(level_offset, child_index)?);
            } else {
                level_siblings.push([Felt::ZERO; HASH_WORDS]);
            }
        }
        siblings.push(level_siblings);

        level_offset = level_offset
            .checked_add(
                padded_len
                    .checked_mul(HASH_WORDS * WORD_BYTES)
                    .ok_or(WitnessStageOpeningError::LengthOverflow)?,
            )
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        level_len = padded_len / commitment.arity();
        level_query /= commitment.arity();
    }

    WitnessStageOpening::new(row_index, values, siblings)
}

pub fn decode_witness_stage_leaf_values(
    leaves: &WitnessStageLeaves,
) -> Result<Vec<Felt>, WitnessStageCommitmentError> {
    Ok(read_witness_stage_leaf_rows(leaves)?
        .into_iter()
        .flatten()
        .collect())
}

pub fn verify_witness_stage_opening_root(
    root: [Felt; HASH_WORDS],
    arity: usize,
    opening: &WitnessStageOpening,
) -> Result<bool, WitnessStageOpeningError> {
    validate_witness_commitment_arity(arity)?;
    if opening.values().is_empty() {
        return Err(WitnessStageOpeningError::EmptyValues);
    }

    let mut digest = linear_hash(opening.values(), arity)?;
    let mut row_index = opening.row_index();
    let arity_u64 = u64::try_from(arity).map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
    for level in opening.siblings() {
        let expected = arity
            .checked_sub(1)
            .ok_or(WitnessStageOpeningError::LengthOverflow)?;
        if level.len() != expected {
            return Err(WitnessStageOpeningError::InvalidSiblingCount {
                expected,
                found: level.len(),
            });
        }
        let child_slot = usize::try_from(row_index % arity_u64)
            .map_err(|_| WitnessStageOpeningError::LengthOverflow)?;
        let mut children = vec![[Felt::ZERO; HASH_WORDS]; arity];
        let mut sibling_index = 0;
        for (slot, child) in children.iter_mut().enumerate() {
            if slot == child_slot {
                *child = digest;
            } else {
                *child = level[sibling_index];
                sibling_index += 1;
            }
        }
        digest = parent_hash(&children, arity)?;
        row_index /= arity_u64;
    }

    Ok(digest == root)
}

fn validate_witness_commitment_arity(arity: usize) -> Result<(), WitnessStageCommitmentError> {
    if matches!(arity, 2 | 4) {
        Ok(())
    } else {
        Err(WitnessStageCommitmentError::UnsupportedArity { arity })
    }
}

fn read_witness_stage_leaf_rows(
    leaves: &WitnessStageLeaves,
) -> Result<Vec<Vec<Felt>>, WitnessStageCommitmentError> {
    let expected = leaves
        .extended_row_count()
        .checked_mul(leaves.column_count())
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
    if leaves.bytes().len() != expected {
        return Err(WitnessStageCommitmentError::InvalidLeafByteLength {
            expected,
            found: leaves.bytes().len(),
        });
    }

    let mut rows = Vec::with_capacity(leaves.extended_row_count());
    for row in 0..leaves.extended_row_count() {
        let mut values = Vec::with_capacity(leaves.column_count());
        for column in 0..leaves.column_count() {
            let word_index = row
                .checked_mul(leaves.column_count())
                .and_then(|offset| offset.checked_add(column))
                .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(WORD_BYTES)
                .ok_or(WitnessStageCommitmentError::LengthOverflow)?;
            let value = u64::from_le_bytes(
                leaves.bytes()[byte_index..byte_index + WORD_BYTES]
                    .try_into()
                    .expect("slice length checked"),
            );
            values.push(Felt::from_canonical(value)?);
        }
        rows.push(values);
    }
    Ok(rows)
}

fn expected_witness_stage_commitment_tree_byte_count(
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<usize, WitnessStageCommitmentError> {
    expected_witness_stage_tree_byte_count(
        row_count,
        column_count,
        arity,
        WitnessStageCommitmentError::LengthOverflow,
    )
}

fn expected_witness_stage_opening_tree_byte_count(
    row_count: usize,
    column_count: usize,
    arity: usize,
) -> Result<usize, WitnessStageOpeningError> {
    expected_witness_stage_tree_byte_count(
        row_count,
        column_count,
        arity,
        WitnessStageOpeningError::LengthOverflow,
    )
}

fn expected_witness_stage_tree_byte_count<E: Clone>(
    row_count: usize,
    column_count: usize,
    arity: usize,
    length_overflow: E,
) -> Result<usize, E> {
    let raw_byte_count = row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or_else(|| length_overflow.clone())?;
    let mut digest_count = row_count;
    let mut level_len = row_count;
    while level_len > 1 {
        let padded_len = round_up_to_arity(level_len, arity, length_overflow.clone())?;
        digest_count = digest_count
            .checked_add(padded_len - level_len)
            .and_then(|count| count.checked_add(padded_len / arity))
            .ok_or_else(|| length_overflow.clone())?;
        level_len = padded_len / arity;
    }
    raw_byte_count
        .checked_add(
            digest_count
                .checked_mul(HASH_WORDS * WORD_BYTES)
                .ok_or_else(|| length_overflow.clone())?,
        )
        .ok_or(length_overflow)
}

fn round_up_to_arity<E>(value: usize, arity: usize, length_overflow: E) -> Result<usize, E> {
    let extra = (arity - (value % arity)) % arity;
    value.checked_add(extra).ok_or(length_overflow)
}

fn append_digest(out: &mut Vec<u8>, digest: [Felt; HASH_WORDS]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "cuda")]
    use super::{
        commit_witness_stage_device_compact_with_leaf_hash_level,
        commit_witness_stage_leaves_compact_with_leaf_hash_level,
        open_witness_stage_commitments_with_source_device_timing,
        WitnessStageDeviceCompactCommitInput, WitnessStageSourceDeviceView,
    };
    use super::{
        commit_witness_stage_leaves, commit_witness_stage_leaves_compact_with_leaf_hashes,
        commit_witness_stage_leaves_owned, commit_witness_stage_leaves_owned_with_leaf_hashes,
        commit_witness_stage_zero_compact, open_witness_stage_commitment,
        verify_witness_stage_opening_root, WitnessStageCommitmentError, WitnessStageLeaves,
        WORD_BYTES,
    };
    #[cfg(feature = "cuda")]
    use crate::witness_commitment::compact_witness_stage_leaf_hash_level_from_source_device_view_timing;
    #[cfg(feature = "cuda")]
    use crate::witness_commitment::WitnessStageOpeningWorkTiming;
    use crate::witness_layout::WitnessTraceStageValues;
    use lzvm_field::{coset_extend_evaluations, Felt, FieldError, MODULUS};

    #[cfg(feature = "cuda")]
    struct RetainedSourceDeviceBudgetGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        previous: Option<std::ffi::OsString>,
    }

    #[cfg(feature = "cuda")]
    impl Drop for RetainedSourceDeviceBudgetGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("LZVM_CUDA_RETAINED_SOURCE_BYTES", value),
                None => std::env::remove_var("LZVM_CUDA_RETAINED_SOURCE_BYTES"),
            }
        }
    }

    #[cfg(feature = "cuda")]
    fn retained_source_device_budget_for_test() -> RetainedSourceDeviceBudgetGuard {
        let lock = crate::CUDA_TEST_ENV_LOCK
            .lock()
            .expect("retained source env lock should acquire");
        let previous = std::env::var_os("LZVM_CUDA_RETAINED_SOURCE_BYTES");
        std::env::set_var("LZVM_CUDA_RETAINED_SOURCE_BYTES", "1048576");
        RetainedSourceDeviceBudgetGuard {
            _lock: lock,
            previous,
        }
    }

    #[test]
    fn rejects_malformed_witness_stage_leaf_byte_lengths() {
        let expected = 2 * 3 * WORD_BYTES;
        let leaves = WitnessStageLeaves::new(1, 2, 2, 3, vec![0_u8; expected - 1]);

        assert!(matches!(
            commit_witness_stage_leaves(&leaves, 2),
            Err(WitnessStageCommitmentError::InvalidLeafByteLength { expected, found })
                if expected == 2 * 3 * WORD_BYTES && found == expected - 1
        ));
    }

    #[test]
    fn owned_witness_stage_commitment_rejects_noncanonical_leaf_words_like_borrowed_commitment() {
        let mut bytes = Vec::new();
        for value in [1, 2, 3, 4, 5, MODULUS] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let leaves = WitnessStageLeaves::new(1, 2, 2, 3, bytes);

        assert_eq!(
            commit_witness_stage_leaves(&leaves, 2).expect_err("borrowed commit should fail"),
            WitnessStageCommitmentError::Field(FieldError::NonCanonical { value: MODULUS })
        );
        assert_eq!(
            commit_witness_stage_leaves_owned(leaves, 2).expect_err("owned commit should fail"),
            WitnessStageCommitmentError::Field(FieldError::NonCanonical { value: MODULUS })
        );
    }

    #[test]
    fn witness_stage_opening_verifies_root_with_padded_parent_level() {
        let row_count = 5;
        let column_count = 6;
        let mut bytes = Vec::new();
        let mut rows = Vec::new();
        for row in 0..row_count {
            let mut values = Vec::new();
            for column in 0..column_count {
                let value = Felt::from_u64((row * 100 + column + 1) as u64);
                bytes.extend_from_slice(&value.to_le_bytes());
                values.push(value);
            }
            rows.push(values);
        }
        let leaves = WitnessStageLeaves::new(7, row_count, row_count, column_count, bytes);
        let commitment =
            commit_witness_stage_leaves(&leaves, 4).expect("stage commitment should build");

        let opening = open_witness_stage_commitment(&commitment, 4, row_count as u64, column_count)
            .expect("stage row should open");
        let verifies = verify_witness_stage_opening_root(commitment.root(), 4, &opening)
            .expect("opening root check should run");

        assert_eq!(opening.values(), rows[4].as_slice());
        assert!(verifies);
    }

    #[test]
    fn owned_witness_stage_commitment_matches_borrowed_commitment() {
        let row_count = 5;
        let column_count = 6;
        let mut bytes = Vec::new();
        for row in 0..row_count {
            for column in 0..column_count {
                let value = Felt::from_u64((row * 100 + column + 1) as u64);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        let leaves = WitnessStageLeaves::new(7, row_count, row_count, column_count, bytes);

        let borrowed =
            commit_witness_stage_leaves(&leaves, 4).expect("stage commitment should build");
        let owned =
            commit_witness_stage_leaves_owned(leaves, 4).expect("stage commitment should build");

        assert_eq!(owned.stage_index(), borrowed.stage_index());
        assert_eq!(owned.arity(), borrowed.arity());
        assert_eq!(owned.root(), borrowed.root());
        assert_eq!(owned.tree_bytes(), borrowed.tree_bytes());

        let opening = open_witness_stage_commitment(&owned, 4, row_count as u64, column_count)
            .expect("stage row should open");
        assert!(verify_witness_stage_opening_root(owned.root(), 4, &opening)
            .expect("opening root check should run"));
    }

    #[test]
    fn compact_leaf_storage_opens_like_full_witness_tree() {
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let mut extended_columns = Vec::new();
        for column in 0..column_count {
            let column_values = (0..source_rows)
                .map(|row| source_values[row * column_count + column])
                .collect::<Vec<_>>();
            extended_columns.push(
                coset_extend_evaluations(&column_values, source_bits, target_bits)
                    .expect("column should extend"),
            );
        }
        let mut leaf_bytes = Vec::new();
        for row in 0..extended_rows {
            for column_values in &extended_columns {
                leaf_bytes.extend_from_slice(&column_values[row].to_le_bytes());
            }
        }
        let leaves =
            WitnessStageLeaves::new(1, source_rows, extended_rows, column_count, leaf_bytes);
        let leaf_hashes = crate::merkle_hash::linear_hashes_from_row_major_bytes(
            leaves.bytes(),
            extended_rows,
            column_count,
            arity,
        )
        .expect("leaf hashes should build");
        let full =
            commit_witness_stage_leaves(&leaves, arity).expect("full commitment should build");
        let stage =
            WitnessTraceStageValues::new_for_test(1, source_rows, column_count, source_values);
        let compact = commit_witness_stage_leaves_compact_with_leaf_hashes(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_hashes,
        )
        .expect("compact commitment should build");

        assert_eq!(compact.root(), full.root());
        assert_eq!(compact.tree_byte_count(), full.tree_byte_count());
        for row in [0, 1, 5, 7] {
            let full_opening =
                open_witness_stage_commitment(&full, row, extended_rows as u64, column_count)
                    .expect("full commitment should open");
            let compact_opening =
                open_witness_stage_commitment(&compact, row, extended_rows as u64, column_count)
                    .expect("compact commitment should open");

            assert_eq!(compact_opening, full_opening);
            assert!(
                verify_witness_stage_opening_root(compact.root(), arity, &compact_opening)
                    .expect("compact opening should verify")
            );
        }
    }

    #[test]
    fn zero_compact_stage_commitment_opens_like_full_zero_tree() {
        let source_bits = 2;
        let target_bits = 5;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 1;
        let arity = 4;
        let stage_index = 3;
        let source_values = vec![Felt::ZERO; source_rows * column_count];
        let leaf_bytes = vec![0_u8; extended_rows * column_count * WORD_BYTES];
        let leaves = WitnessStageLeaves::new(
            stage_index,
            source_rows,
            extended_rows,
            column_count,
            leaf_bytes,
        );
        let full =
            commit_witness_stage_leaves(&leaves, arity).expect("full commitment should build");
        let compact = commit_witness_stage_zero_compact(
            stage_index,
            source_bits,
            target_bits,
            column_count,
            arity,
        )
        .expect("zero compact commitment should build");

        assert_eq!(compact.root(), full.root());
        assert_eq!(compact.tree_byte_count(), full.tree_byte_count());
        for row in [0, 1, 16, 31] {
            let full_opening =
                open_witness_stage_commitment(&full, row, extended_rows as u64, column_count)
                    .expect("full commitment should open");
            let compact_opening =
                open_witness_stage_commitment(&compact, row, extended_rows as u64, column_count)
                    .expect("compact commitment should open");

            assert_eq!(compact_opening, full_opening);
            assert!(
                verify_witness_stage_opening_root(compact.root(), arity, &compact_opening)
                    .expect("compact opening should verify")
            );
        }
        assert_eq!(compact.tree_bytes(), full.tree_bytes());
        let stage = WitnessTraceStageValues::new_for_test(
            stage_index,
            source_rows,
            column_count,
            source_values,
        );
        assert_eq!(stage.values(), &[Felt::ZERO; 4]);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn zero_compact_descriptor_column_matches_actual_device_slice() {
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let full_column_count = 39;
        let column_offset = full_column_count - 1;
        let column_count = 1;
        let arity = 4;
        let stage_index = 3;
        let full_trace_values = (0..source_rows)
            .flat_map(|row| {
                (0..full_column_count).map(move |column| {
                    if column == column_offset {
                        Felt::ZERO
                    } else {
                        Felt::from_u64((row * full_column_count + column + 1) as u64)
                    }
                })
            })
            .collect::<Vec<_>>();
        let full_trace_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&full_trace_values))
                .expect("full trace values should upload"),
        );
        let source_view = WitnessStageSourceDeviceView::new(
            source_rows,
            column_count,
            full_column_count,
            column_offset,
            full_trace_device,
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level = compact_witness_stage_leaf_hash_level_from_source_device_view_timing(
            source_rows,
            column_count,
            source_bits,
            target_bits,
            arity,
            &source_view,
            &mut timing,
        )
        .expect("device leaf hash level should build");
        let actual = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: true,
            },
            leaf_level,
            Some(source_view.clone()),
        )
        .expect("actual slice commitment should build");
        let compact = commit_witness_stage_zero_compact(
            stage_index,
            source_bits,
            target_bits,
            column_count,
            arity,
        )
        .expect("zero compact commitment should build");

        assert_eq!(compact.root(), actual.root());
        assert_eq!(compact.tree_byte_count(), actual.tree_byte_count());
        for row in [0, 1, extended_rows - 1] {
            let compact_opening = open_witness_stage_commitment(
                &compact,
                row as u64,
                extended_rows as u64,
                column_count,
            )
            .expect("compact commitment should open");
            let mut timing = WitnessStageOpeningWorkTiming::default();
            let mut actual_openings = open_witness_stage_commitments_with_source_device_timing(
                &actual,
                &[row as u64],
                extended_rows as u64,
                column_count,
                Some(&source_view),
                &mut timing,
            )
            .expect("actual slice commitment should open");
            let actual_opening = actual_openings
                .pop()
                .expect("actual slice opening should be present");
            assert_eq!(compact_opening, actual_opening);
            assert!(
                verify_witness_stage_opening_root(compact.root(), arity, &actual_opening)
                    .expect("actual slice opening should verify")
            );
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compact_device_leaf_hash_level_matches_host_commitment() {
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let mut extended_columns = Vec::new();
        for column in 0..column_count {
            let column_values = (0..source_rows)
                .map(|row| source_values[row * column_count + column])
                .collect::<Vec<_>>();
            extended_columns.push(
                coset_extend_evaluations(&column_values, source_bits, target_bits)
                    .expect("column should extend"),
            );
        }
        let mut leaf_bytes = Vec::new();
        for row in 0..extended_rows {
            for column_values in &extended_columns {
                leaf_bytes.extend_from_slice(&column_values[row].to_le_bytes());
            }
        }
        let leaves =
            WitnessStageLeaves::new(1, source_rows, extended_rows, column_count, leaf_bytes);
        let leaf_hashes = crate::merkle_hash::linear_hashes_from_row_major_bytes(
            leaves.bytes(),
            extended_rows,
            column_count,
            arity,
        )
        .expect("leaf hashes should build");
        let stage =
            WitnessTraceStageValues::new_for_test(1, source_rows, column_count, source_values);
        let host = commit_witness_stage_leaves_compact_with_leaf_hashes(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_hashes,
        )
        .expect("host compact commitment should build");
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_with_source_device_timing(
                &stage,
                source_bits,
                target_bits,
                arity,
                None,
                &mut timing,
            )
            .expect("device leaf hash level should build");
        let device = commit_witness_stage_leaves_compact_with_leaf_hash_level(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_level,
            None,
        )
        .expect("device compact commitment should build");

        assert_eq!(device.root(), host.root());
        assert_eq!(device.tree_byte_count(), host.tree_byte_count());
        for row in [0, 1, 5, 7] {
            let opening =
                open_witness_stage_commitment(&device, row, extended_rows as u64, column_count)
                    .expect("device compact commitment should open");
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, &opening)
                    .expect("device compact opening should verify")
            );
        }
        assert_eq!(device.tree_bytes(), host.tree_bytes());
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn retained_source_compact_device_commitment_materializes_without_host_source_values() {
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let mut extended_columns = Vec::new();
        for column in 0..column_count {
            let column_values = (0..source_rows)
                .map(|row| source_values[row * column_count + column])
                .collect::<Vec<_>>();
            extended_columns.push(
                coset_extend_evaluations(&column_values, source_bits, target_bits)
                    .expect("column should extend"),
            );
        }
        let mut leaf_bytes = Vec::new();
        for row in 0..extended_rows {
            for column_values in &extended_columns {
                leaf_bytes.extend_from_slice(&column_values[row].to_le_bytes());
            }
        }
        let leaves =
            WitnessStageLeaves::new(1, source_rows, extended_rows, column_count, leaf_bytes);
        let leaf_hashes = crate::merkle_hash::linear_hashes_from_row_major_bytes(
            leaves.bytes(),
            extended_rows,
            column_count,
            arity,
        )
        .expect("leaf hashes should build");
        let stage =
            WitnessTraceStageValues::new_for_test(1, source_rows, column_count, source_values);
        let host = commit_witness_stage_leaves_compact_with_leaf_hashes(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_hashes,
        )
        .expect("host compact commitment should build");
        let source_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(stage.values()))
                .expect("source values should upload"),
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_with_source_device_timing(
                &stage,
                source_bits,
                target_bits,
                arity,
                Some(source_device.as_ref()),
                &mut timing,
            )
            .expect("device leaf hash level should build");
        let device = commit_witness_stage_leaves_compact_with_leaf_hash_level(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_level,
            Some(WitnessStageSourceDeviceView::new(
                source_rows,
                column_count,
                column_count,
                0,
                source_device,
            )),
        )
        .expect("device compact commitment should build");

        assert_eq!(device.root(), host.root());
        assert_eq!(device.tree_byte_count(), host.tree_byte_count());
        for row in [0, 1, 5, 7] {
            let opening =
                open_witness_stage_commitment(&device, row, extended_rows as u64, column_count)
                    .expect("device compact commitment should open");
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, &opening)
                    .expect("device compact opening should verify")
            );
        }
        assert_eq!(device.tree_bytes(), host.tree_bytes());
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn device_source_compact_commitment_materializes_without_stage_values() {
        let _retained_source_budget = retained_source_device_budget_for_test();
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let mut extended_columns = Vec::new();
        for column in 0..column_count {
            let column_values = (0..source_rows)
                .map(|row| source_values[row * column_count + column])
                .collect::<Vec<_>>();
            extended_columns.push(
                coset_extend_evaluations(&column_values, source_bits, target_bits)
                    .expect("column should extend"),
            );
        }
        let mut leaf_bytes = Vec::new();
        for row in 0..extended_rows {
            for column_values in &extended_columns {
                leaf_bytes.extend_from_slice(&column_values[row].to_le_bytes());
            }
        }
        let leaves =
            WitnessStageLeaves::new(1, source_rows, extended_rows, column_count, leaf_bytes);
        let leaf_hashes = crate::merkle_hash::linear_hashes_from_row_major_bytes(
            leaves.bytes(),
            extended_rows,
            column_count,
            arity,
        )
        .expect("leaf hashes should build");
        let stage = WitnessTraceStageValues::new_for_test(
            1,
            source_rows,
            column_count,
            source_values.clone(),
        );
        let host = commit_witness_stage_leaves_compact_with_leaf_hashes(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_hashes,
        )
        .expect("host compact commitment should build");
        let source_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&source_values))
                .expect("source values should upload"),
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_from_source_device_timing(
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                source_device.as_ref(),
                &mut timing,
            )
            .expect("device leaf hash level should build");
        let device = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index: 1,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: false,
            },
            leaf_level,
            Some(WitnessStageSourceDeviceView::new(
                source_rows,
                column_count,
                column_count,
                0,
                source_device,
            )),
        )
        .expect("device compact commitment should build");

        assert_eq!(device.root(), host.root());
        assert_eq!(device.tree_byte_count(), host.tree_byte_count());
        for row in [0, 1, 5, 7] {
            let opening =
                open_witness_stage_commitment(&device, row, extended_rows as u64, column_count)
                    .expect("device compact commitment should open");
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, &opening)
                    .expect("device compact opening should verify")
            );
        }
        assert_eq!(device.tree_bytes(), host.tree_bytes());
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compact_device_batch_opening_matches_individual_rows() {
        let _retained_source_budget = retained_source_device_budget_for_test();
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let source_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&source_values))
                .expect("source values should upload"),
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_from_source_device_timing(
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                source_device.as_ref(),
                &mut timing,
            )
            .expect("device leaf hash level should build");
        let device = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index: 1,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: false,
            },
            leaf_level,
            Some(WitnessStageSourceDeviceView::new(
                source_rows,
                column_count,
                column_count,
                0,
                source_device,
            )),
        )
        .expect("device compact commitment should build");

        let rows = [0_u64, 1, 5, 7];
        let mut batch_timing = WitnessStageOpeningWorkTiming::default();
        let batch_openings = open_witness_stage_commitments_with_source_device_timing(
            &device,
            &rows,
            extended_rows as u64,
            column_count,
            None,
            &mut batch_timing,
        )
        .expect("batch opening should build");

        assert_eq!(batch_openings.len(), rows.len());
        for (row, batch_opening) in rows.iter().copied().zip(batch_openings.iter()) {
            let single_opening =
                open_witness_stage_commitment(&device, row, extended_rows as u64, column_count)
                    .expect("single opening should build");
            assert_eq!(batch_opening, &single_opening);
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, batch_opening)
                    .expect("batch opening should verify")
            );
        }
        assert_eq!(batch_timing.leaf_coset_extend_call_count, 0);
        assert_eq!(batch_timing.leaf_hash_rows, 0);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compact_device_batch_opening_reuses_retained_leaf_digest_level() {
        let _retained_source_budget = retained_source_device_budget_for_test();
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let source_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&source_values))
                .expect("source values should upload"),
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_from_source_device_timing(
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                source_device.as_ref(),
                &mut timing,
            )
            .expect("device leaf hash level should build");
        let device = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index: 1,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: false,
            },
            leaf_level,
            Some(WitnessStageSourceDeviceView::new(
                source_rows,
                column_count,
                column_count,
                0,
                source_device,
            )),
        )
        .expect("device compact commitment should build");

        let rows = [0_u64, 1, 5, 7];
        let mut batch_timing = WitnessStageOpeningWorkTiming::default();
        let batch_openings = open_witness_stage_commitments_with_source_device_timing(
            &device,
            &rows,
            extended_rows as u64,
            column_count,
            None,
            &mut batch_timing,
        )
        .expect("batch opening should build");

        assert_eq!(batch_openings.len(), rows.len());
        for batch_opening in &batch_openings {
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, batch_opening)
                    .expect("batch opening should verify")
            );
        }
        assert_eq!(batch_timing.retained_leaf_digest_opening_count, 1);
        assert_eq!(
            batch_timing.retained_leaf_digest_opening_row_count,
            rows.len()
        );
        assert_eq!(batch_timing.row_values_source_row_count, rows.len());
        assert_eq!(batch_timing.row_values_device_row_count, 0);
        assert_eq!(
            batch_timing.row_values_word_count,
            rows.len() * column_count
        );
        assert_eq!(
            batch_timing.row_values_byte_count,
            rows.len() * column_count * WORD_BYTES
        );
        assert_eq!(
            batch_timing.path_parent_hash_retained_leaf_digest_launch_count,
            2
        );
        assert_eq!(batch_timing.leaf_coset_extend_call_count, 0);
        assert_eq!(batch_timing.leaf_hash_rows, 0);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compact_device_commit_retains_parent_checkpoint_level() {
        let _retained_source_budget = retained_source_device_budget_for_test();
        let source_bits = 2;
        let target_bits = 20;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let source_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&source_values))
                .expect("source values should upload"),
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_from_source_device_timing(
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                source_device.as_ref(),
                &mut timing,
            )
            .expect("device leaf hash level should build");

        let device = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index: 1,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: false,
            },
            leaf_level,
            Some(WitnessStageSourceDeviceView::new(
                source_rows,
                column_count,
                column_count,
                0,
                source_device,
            )),
        )
        .expect("device compact commitment should build");

        assert_eq!(
            device.retained_parent_checkpoint_shape_for_test(),
            Some((extended_rows, 1, extended_rows / arity, arity))
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compact_device_parent_checkpoint_opening_matches_full_path_suffix() {
        let _retained_source_budget = retained_source_device_budget_for_test();
        let source_bits = 2;
        let target_bits = 20;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let source_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&source_values))
                .expect("source values should upload"),
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_from_source_device_timing(
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                source_device.as_ref(),
                &mut timing,
            )
            .expect("device leaf hash level should build");
        let device = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index: 1,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: false,
            },
            leaf_level,
            Some(WitnessStageSourceDeviceView::new(
                source_rows,
                column_count,
                column_count,
                0,
                source_device,
            )),
        )
        .expect("device compact commitment should build");
        let query_row = 3891_u64;
        let full_opening =
            open_witness_stage_commitment(&device, query_row, extended_rows as u64, column_count)
                .expect("full opening should build");

        let upper_siblings = device
            .retained_parent_checkpoint_opening_suffix_for_test(query_row as usize)
            .expect("retained checkpoint opening should build");

        assert_eq!(upper_siblings, full_opening.siblings()[1..]);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compact_device_batch_opening_combines_leaf_digest_with_parent_checkpoint() {
        let _retained_source_budget = retained_source_device_budget_for_test();
        let source_bits = 2;
        let target_bits = 20;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let source_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&source_values))
                .expect("source values should upload"),
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_from_source_device_timing(
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                source_device.as_ref(),
                &mut timing,
            )
            .expect("device leaf hash level should build");
        let device = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index: 1,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: false,
            },
            leaf_level,
            Some(WitnessStageSourceDeviceView::new(
                source_rows,
                column_count,
                column_count,
                0,
                source_device,
            )),
        )
        .expect("device compact commitment should build");
        assert_eq!(
            device.retained_parent_checkpoint_shape_for_test(),
            Some((extended_rows, 1, extended_rows / arity, arity))
        );

        let rows = [0_u64, 3891, 4095];
        let mut batch_timing = WitnessStageOpeningWorkTiming::default();
        let batch_openings = open_witness_stage_commitments_with_source_device_timing(
            &device,
            &rows,
            extended_rows as u64,
            column_count,
            None,
            &mut batch_timing,
        )
        .expect("batch opening should build");

        assert_eq!(batch_openings.len(), rows.len());
        for (row, batch_opening) in rows.iter().copied().zip(batch_openings.iter()) {
            let single_opening =
                open_witness_stage_commitment(&device, row, extended_rows as u64, column_count)
                    .expect("single opening should build");
            assert_eq!(batch_opening, &single_opening);
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, batch_opening)
                    .expect("batch opening should verify")
            );
        }

        assert_eq!(batch_timing.retained_leaf_digest_opening_count, 1);
        assert_eq!(batch_timing.retained_parent_checkpoint_opening_count, 1);
        assert_eq!(
            batch_timing.path_parent_hash_retained_leaf_digest_row_count,
            0
        );
        assert_eq!(
            batch_timing.path_parent_hash_retained_parent_checkpoint_prefix_row_count,
            0
        );
        let checkpoint_parent_rows = 65536 + 16384 + 4096 + 1024 + 256 + 64 + 16 + 4 + 1;
        assert_eq!(
            batch_timing.path_parent_hash_retained_parent_checkpoint_suffix_row_count,
            checkpoint_parent_rows
        );
        assert_eq!(
            batch_timing.path_parent_hash_row_count,
            checkpoint_parent_rows
        );
        assert_eq!(batch_timing.row_values_source_row_count, rows.len());
        assert_eq!(batch_timing.row_values_device_row_count, 0);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compact_device_batch_opening_uses_retained_parent_checkpoint_after_leaf_digest_drop() {
        let _retained_source_budget = retained_source_device_budget_for_test();
        let source_bits = 2;
        let target_bits = 20;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 6;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let source_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&source_values))
                .expect("source values should upload"),
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_from_source_device_timing(
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                source_device.as_ref(),
                &mut timing,
            )
            .expect("device leaf hash level should build");
        let mut device = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index: 1,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: false,
            },
            leaf_level,
            Some(WitnessStageSourceDeviceView::new(
                source_rows,
                column_count,
                column_count,
                0,
                source_device,
            )),
        )
        .expect("device compact commitment should build");
        assert!(device.drop_retained_leaf_digest_level_for_test());

        let rows = [0_u64, 3891, 4095];
        let mut batch_timing = WitnessStageOpeningWorkTiming::default();
        let batch_openings = open_witness_stage_commitments_with_source_device_timing(
            &device,
            &rows,
            extended_rows as u64,
            column_count,
            None,
            &mut batch_timing,
        )
        .expect("batch opening should build");

        assert_eq!(batch_openings.len(), rows.len());
        for (row, batch_opening) in rows.iter().copied().zip(batch_openings.iter()) {
            let single_opening =
                open_witness_stage_commitment(&device, row, extended_rows as u64, column_count)
                    .expect("single opening should build");
            assert_eq!(batch_opening, &single_opening);
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, batch_opening)
                    .expect("batch opening should verify")
            );
        }
        assert_eq!(batch_timing.retained_leaf_digest_opening_count, 0);
        assert_eq!(batch_timing.retained_parent_checkpoint_opening_count, 1);
        assert_eq!(
            batch_timing.retained_parent_checkpoint_opening_row_count,
            rows.len()
        );
        assert_eq!(batch_timing.row_values_source_row_count, 0);
        assert_eq!(batch_timing.row_values_device_row_count, rows.len());
        assert_eq!(batch_timing.row_values_device_download_batch_count, 1);
        assert_eq!(
            batch_timing.row_values_word_count,
            rows.len() * column_count
        );
        assert_eq!(
            batch_timing.row_values_byte_count,
            rows.len() * column_count * WORD_BYTES
        );
        let checkpoint_parent_rows = 65536 + 16384 + 4096 + 1024 + 256 + 64 + 16 + 4 + 1;
        assert_eq!(
            batch_timing.path_parent_hash_row_count,
            checkpoint_parent_rows
        );
        assert_eq!(
            batch_timing.path_parent_hash_retained_parent_checkpoint_prefix_row_count,
            0
        );
        assert_eq!(
            batch_timing.path_parent_hash_retained_parent_checkpoint_suffix_row_count,
            checkpoint_parent_rows
        );

        let mut single_timing = WitnessStageOpeningWorkTiming::default();
        let single_openings = open_witness_stage_commitments_with_source_device_timing(
            &device,
            &[3891],
            extended_rows as u64,
            column_count,
            None,
            &mut single_timing,
        )
        .expect("single opening should build");
        assert_eq!(single_openings.len(), 1);
        assert_eq!(single_timing.row_values_device_row_count, 1);
        assert_eq!(single_timing.row_values_device_download_batch_count, 0);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn strided_device_source_compact_commitment_uses_full_trace_view() {
        let _retained_source_budget = retained_source_device_budget_for_test();
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let full_column_count = 9;
        let column_offset = 2;
        let column_count = 4;
        let arity = 4;
        let full_trace_values = (0..source_rows * full_column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let source_values = (0..source_rows)
            .flat_map(|row| {
                let start = row * full_column_count + column_offset;
                full_trace_values[start..start + column_count]
                    .iter()
                    .copied()
            })
            .collect::<Vec<_>>();
        let mut extended_columns = Vec::new();
        for column in 0..column_count {
            let column_values = (0..source_rows)
                .map(|row| source_values[row * column_count + column])
                .collect::<Vec<_>>();
            extended_columns.push(
                coset_extend_evaluations(&column_values, source_bits, target_bits)
                    .expect("column should extend"),
            );
        }
        let mut leaf_bytes = Vec::new();
        for row in 0..extended_rows {
            for column_values in &extended_columns {
                leaf_bytes.extend_from_slice(&column_values[row].to_le_bytes());
            }
        }
        let leaves =
            WitnessStageLeaves::new(1, source_rows, extended_rows, column_count, leaf_bytes);
        let leaf_hashes = crate::merkle_hash::linear_hashes_from_row_major_bytes(
            leaves.bytes(),
            extended_rows,
            column_count,
            arity,
        )
        .expect("leaf hashes should build");
        let stage = WitnessTraceStageValues::new_for_test(
            1,
            source_rows,
            column_count,
            source_values.clone(),
        );
        let host = commit_witness_stage_leaves_compact_with_leaf_hashes(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_hashes,
        )
        .expect("host compact commitment should build");
        let full_trace_device = std::sync::Arc::new(
            lzvm_accel::CudaDeviceBuffer::from_u64_words(Felt::as_u64_slice(&full_trace_values))
                .expect("full trace values should upload"),
        );
        let source_view = WitnessStageSourceDeviceView::new(
            source_rows,
            column_count,
            full_column_count,
            column_offset,
            full_trace_device,
        );
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level = compact_witness_stage_leaf_hash_level_from_source_device_view_timing(
            source_rows,
            column_count,
            source_bits,
            target_bits,
            arity,
            &source_view,
            &mut timing,
        )
        .expect("device leaf hash level should build from full trace view");
        let device = commit_witness_stage_device_compact_with_leaf_hash_level(
            WitnessStageDeviceCompactCommitInput {
                stage_index: 1,
                source_rows,
                column_count,
                source_bits,
                target_bits,
                arity,
                external_source_required: false,
            },
            leaf_level,
            Some(source_view),
        )
        .expect("device compact commitment should build");

        assert_eq!(device.root(), host.root());
        assert_eq!(device.tree_byte_count(), host.tree_byte_count());
        for row in [0, 1, 5, 7] {
            let opening =
                open_witness_stage_commitment(&device, row, extended_rows as u64, column_count)
                    .expect("device compact commitment should open");
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, &opening)
                    .expect("device compact opening should verify")
            );
        }
        assert_eq!(device.tree_bytes(), host.tree_bytes());
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compact_device_narrow_leaf_hash_level_matches_host_commitment() {
        let source_bits = 2;
        let target_bits = 3;
        let source_rows = 1_usize << source_bits;
        let extended_rows = 1_usize << target_bits;
        let column_count = 3;
        let arity = 4;
        let source_values = (0..source_rows * column_count)
            .map(|value| Felt::from_u64(value as u64 + 1))
            .collect::<Vec<_>>();
        let mut extended_columns = Vec::new();
        for column in 0..column_count {
            let column_values = (0..source_rows)
                .map(|row| source_values[row * column_count + column])
                .collect::<Vec<_>>();
            extended_columns.push(
                coset_extend_evaluations(&column_values, source_bits, target_bits)
                    .expect("column should extend"),
            );
        }
        let mut leaf_bytes = Vec::new();
        for row in 0..extended_rows {
            for column_values in &extended_columns {
                leaf_bytes.extend_from_slice(&column_values[row].to_le_bytes());
            }
        }
        let leaves =
            WitnessStageLeaves::new(1, source_rows, extended_rows, column_count, leaf_bytes);
        let leaf_hashes = crate::merkle_hash::linear_hashes_from_row_major_bytes(
            leaves.bytes(),
            extended_rows,
            column_count,
            arity,
        )
        .expect("leaf hashes should build");
        let stage =
            WitnessTraceStageValues::new_for_test(1, source_rows, column_count, source_values);
        let host = commit_witness_stage_leaves_compact_with_leaf_hashes(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_hashes,
        )
        .expect("host compact commitment should build");
        let mut timing = crate::witness_commitment::WitnessStageLeafExtendTiming::default();
        let leaf_level =
            crate::witness_commitment::compact_witness_stage_leaf_hash_level_with_source_device_timing(
                &stage,
                source_bits,
                target_bits,
                arity,
                None,
                &mut timing,
            )
            .expect("device narrow leaf hash level should build");
        let device = commit_witness_stage_leaves_compact_with_leaf_hash_level(
            &stage,
            source_bits,
            target_bits,
            arity,
            leaf_level,
            None,
        )
        .expect("device compact commitment should build");

        assert_eq!(device.root(), host.root());
        assert_eq!(device.tree_byte_count(), host.tree_byte_count());
        for row in [0, 1, 5, 7] {
            let opening =
                open_witness_stage_commitment(&device, row, extended_rows as u64, column_count)
                    .expect("device compact commitment should open");
            assert!(
                verify_witness_stage_opening_root(device.root(), arity, &opening)
                    .expect("device compact opening should verify")
            );
        }
        assert_eq!(device.tree_bytes(), host.tree_bytes());
    }

    #[test]
    fn prehashed_witness_stage_commitment_rejects_leaf_digest_count_mismatch() {
        let row_count = 5;
        let column_count = 6;
        let mut bytes = Vec::new();
        for row in 0..row_count {
            for column in 0..column_count {
                let value = Felt::from_u64((row * 100 + column + 1) as u64);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        let leaves = WitnessStageLeaves::new(7, row_count, row_count, column_count, bytes);
        let leaf_hashes = vec![[Felt::ZERO; 4]; row_count - 1];

        assert!(matches!(
            commit_witness_stage_leaves_owned_with_leaf_hashes(leaves, 4, leaf_hashes),
            Err(WitnessStageCommitmentError::InvalidLeafDigestCount { expected, found })
                if expected == row_count && found == row_count - 1
        ));
    }
}

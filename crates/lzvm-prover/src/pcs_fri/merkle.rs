use std::fmt;

use lzvm_artifacts::pcs_fri_segment::PcsFriOpeningLevelSegment;
use lzvm_field::{Ext3, Felt, FieldError};

use crate::merkle_hash::{
    linear_hash, linear_hashes, parent_hash, parent_hashes, root_from_digest_level,
    MerkleHashError, HASH_WORDS,
};

use super::errors::PcsFriOpeningBuildError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriMerkleError {
    UnsupportedArity { arity: usize },
    EmptyValues,
    EmptyLastLevel,
    InvalidSiblingCount { expected: usize, found: usize },
    LastLevelIndexOutOfRange { index: u64, node_count: usize },
    Field(FieldError),
    LengthOverflow,
}

impl fmt::Display for PcsFriMerkleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArity { arity } => {
                write!(f, "PCS FRI Merkle arity is unsupported: {arity}")
            }
            Self::EmptyValues => write!(f, "PCS FRI Merkle query has no values"),
            Self::EmptyLastLevel => write!(f, "PCS FRI Merkle last level is empty"),
            Self::InvalidSiblingCount { expected, found } => write!(
                f,
                "PCS FRI Merkle sibling count mismatch: expected {expected}, found {found}"
            ),
            Self::LastLevelIndexOutOfRange { index, node_count } => write!(
                f,
                "PCS FRI Merkle last-level index {index} is outside node count {node_count}"
            ),
            Self::Field(error) => write!(f, "PCS FRI Merkle field error: {error}"),
            Self::LengthOverflow => write!(f, "PCS FRI Merkle length overflow"),
        }
    }
}

impl std::error::Error for PcsFriMerkleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field(error) => Some(error),
            Self::UnsupportedArity { .. }
            | Self::EmptyValues
            | Self::EmptyLastLevel
            | Self::InvalidSiblingCount { .. }
            | Self::LastLevelIndexOutOfRange { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<MerkleHashError> for PcsFriMerkleError {
    fn from(error: MerkleHashError) -> Self {
        match error {
            MerkleHashError::UnsupportedArity { arity } => Self::UnsupportedArity { arity },
            MerkleHashError::InvalidChildCount { expected, found } => {
                Self::InvalidSiblingCount { expected, found }
            }
            MerkleHashError::Field(error) => Self::Field(error),
            MerkleHashError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FriLayerTree {
    pub(super) root: [Felt; HASH_WORDS],
    levels: Vec<Vec<[Felt; HASH_WORDS]>>,
    unpadded_counts: Vec<usize>,
    pub(super) last_level: Vec<[Felt; HASH_WORDS]>,
    last_level_verification: u32,
    arity: usize,
}

impl FriLayerTree {
    pub(super) fn query_siblings(
        &self,
        row_index: usize,
    ) -> Result<Vec<PcsFriOpeningLevelSegment>, PcsFriOpeningBuildError> {
        let mut siblings = Vec::new();
        let mut query_index = row_index;
        let mut level_index = 0;
        while !self.should_stop_at_level(level_index)? {
            let level = self
                .levels
                .get(level_index)
                .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
            let child_slot = query_index % self.arity;
            let group_start = (query_index / self.arity)
                .checked_mul(self.arity)
                .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
            let mut level_siblings = Vec::with_capacity(self.arity - 1);
            for slot in 0..self.arity {
                if slot == child_slot {
                    continue;
                }
                let sibling_index = group_start
                    .checked_add(slot)
                    .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
                let digest = level
                    .get(sibling_index)
                    .copied()
                    .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
                level_siblings.push(digest_to_u64s(digest));
            }
            siblings.push(PcsFriOpeningLevelSegment {
                siblings: level_siblings,
            });
            query_index /= self.arity;
            level_index += 1;
        }
        Ok(siblings)
    }

    fn should_stop_at_level(&self, level_index: usize) -> Result<bool, PcsFriOpeningBuildError> {
        let count = *self
            .unpadded_counts
            .get(level_index)
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
        if self.last_level_verification == 0 {
            Ok(count == 1)
        } else {
            Ok(count <= checked_pow(self.arity, self.last_level_verification)?)
        }
    }
}

pub fn verify_fri_query_path(
    root: [Felt; HASH_WORDS],
    last_level: &[[Felt; HASH_WORDS]],
    arity: usize,
    row_index: u64,
    values: &[Ext3],
    siblings: &[Vec<[Felt; HASH_WORDS]>],
) -> Result<bool, PcsFriMerkleError> {
    if values.is_empty() {
        return Err(PcsFriMerkleError::EmptyValues);
    }
    let flattened_values = flatten_extension_values(values)?;
    let mut digest = linear_hash(&flattened_values, arity)?;
    let mut path_index = row_index;
    let arity_u64 = u64::try_from(arity).map_err(|_| PcsFriMerkleError::LengthOverflow)?;
    let expected_siblings = arity
        .checked_sub(1)
        .ok_or(PcsFriMerkleError::LengthOverflow)?;

    for level in siblings {
        if level.len() != expected_siblings {
            return Err(PcsFriMerkleError::InvalidSiblingCount {
                expected: expected_siblings,
                found: level.len(),
            });
        }

        let child_slot = usize::try_from(path_index % arity_u64)
            .map_err(|_| PcsFriMerkleError::LengthOverflow)?;
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
        path_index /= arity_u64;
    }

    if last_level.is_empty() {
        Ok(digest == root)
    } else {
        let index = usize::try_from(path_index).map_err(|_| PcsFriMerkleError::LengthOverflow)?;
        let target = last_level
            .get(index)
            .ok_or(PcsFriMerkleError::LastLevelIndexOutOfRange {
                index: path_index,
                node_count: last_level.len(),
            })?;
        Ok(digest == *target)
    }
}

pub fn verify_fri_last_level_root(
    root: [Felt; HASH_WORDS],
    arity: usize,
    last_level: &[[Felt; HASH_WORDS]],
) -> Result<bool, PcsFriMerkleError> {
    if last_level.is_empty() {
        return Err(PcsFriMerkleError::EmptyLastLevel);
    }
    Ok(root_from_digest_level(last_level, arity)? == root)
}

pub(super) fn build_fri_layer_tree(
    rows: &[Vec<Ext3>],
    arity: usize,
    last_level_verification: u32,
) -> Result<FriLayerTree, PcsFriOpeningBuildError> {
    if rows.is_empty() {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index: 0,
            expected: 1,
            found: 0,
        });
    }

    let flattened_rows = rows
        .iter()
        .map(|row| flatten_extension_values(row))
        .collect::<Result<Vec<_>, PcsFriMerkleError>>()?;
    let mut current = linear_hashes(&flattened_rows, arity).map_err(PcsFriMerkleError::from)?;
    let mut levels = Vec::new();
    let mut unpadded_counts = Vec::new();
    loop {
        unpadded_counts.push(current.len());
        let mut padded = current.clone();
        if padded.len() > 1 {
            let extra_zeros = (arity - (padded.len() % arity)) % arity;
            padded.resize(
                padded
                    .len()
                    .checked_add(extra_zeros)
                    .ok_or(PcsFriOpeningBuildError::LengthOverflow)?,
                [Felt::ZERO; HASH_WORDS],
            );
        }
        levels.push(padded.clone());
        if current.len() == 1 {
            break;
        }

        current = parent_hashes(&padded, arity).map_err(PcsFriMerkleError::from)?;
    }

    let root = *levels
        .last()
        .and_then(|level| level.first())
        .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
    let last_level = if last_level_verification == 0 {
        Vec::new()
    } else {
        let target_count = checked_pow(arity, last_level_verification)?;
        let level_index = unpadded_counts
            .iter()
            .position(|count| *count <= target_count)
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
        let count = unpadded_counts[level_index];
        levels[level_index][..count].to_vec()
    };

    Ok(FriLayerTree {
        root,
        levels,
        unpadded_counts,
        last_level,
        last_level_verification,
        arity,
    })
}

fn flatten_extension_values(values: &[Ext3]) -> Result<Vec<Felt>, PcsFriMerkleError> {
    let len = values
        .len()
        .checked_mul(3)
        .ok_or(PcsFriMerkleError::LengthOverflow)?;
    let mut out = Vec::with_capacity(len);
    for value in values {
        out.push(value.c0);
        out.push(value.c1);
        out.push(value.c2);
    }
    Ok(out)
}

fn checked_pow(base: usize, power: u32) -> Result<usize, PcsFriOpeningBuildError> {
    let mut out = 1_usize;
    for _ in 0..power {
        out = out
            .checked_mul(base)
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
    }
    Ok(out)
}

pub(super) fn digest_to_u64s(digest: [Felt; HASH_WORDS]) -> [u64; HASH_WORDS] {
    digest.map(Felt::to_u64)
}

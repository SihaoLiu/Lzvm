use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use lzvm_artifacts::constant_tree::{
    expected_constant_tree_byte_count, expected_constant_tree_leaf_node_byte_counts, ConstantTree,
    ConstantTreeError, ConstantTreeHashKind,
};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_field::{Felt, FieldError};

use crate::merkle_hash::{linear_hash, parent_hash, MerkleHashError};

const WORD_BYTES: usize = 8;
const HASH_WORDS: usize = 4;
const DIGEST_BYTES: usize = HASH_WORDS * WORD_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantTreeOpening {
    row_index: u64,
    values: Vec<Felt>,
    siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
}

impl ConstantTreeOpening {
    pub fn new(
        row_index: u64,
        values: Vec<Felt>,
        siblings: Vec<Vec<[Felt; HASH_WORDS]>>,
    ) -> Result<Self, ConstantTreeOpeningError> {
        if values.is_empty() {
            return Err(ConstantTreeOpeningError::EmptyValues);
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
pub enum ConstantTreeOpeningError {
    UnsupportedHash,
    UnsupportedArity {
        arity: usize,
    },
    EmptyValues,
    RowIndexOutOfRange {
        row_index: u64,
        row_count: u64,
    },
    InvalidTreeLength {
        expected: usize,
        found: usize,
    },
    InvalidSiblingWidth {
        expected: usize,
        found: usize,
    },
    ConstantTree(ConstantTreeError),
    Field(FieldError),
    #[cfg(feature = "cuda")]
    Accel(lzvm_accel::AccelError),
    LengthOverflow,
}

impl fmt::Display for ConstantTreeOpeningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedHash => write!(f, "constant tree opening hash is unsupported"),
            Self::UnsupportedArity { arity } => {
                write!(f, "constant tree opening arity is unsupported: {arity}")
            }
            Self::EmptyValues => write!(f, "constant tree opening has no values"),
            Self::RowIndexOutOfRange {
                row_index,
                row_count,
            } => write!(
                f,
                "constant tree opening row {row_index} is outside row count {row_count}"
            ),
            Self::InvalidTreeLength { expected, found } => write!(
                f,
                "constant tree opening byte length mismatch: expected {expected}, found {found}"
            ),
            Self::InvalidSiblingWidth { expected, found } => write!(
                f,
                "constant tree opening sibling width mismatch: expected {expected}, found {found}"
            ),
            Self::ConstantTree(error) => write!(f, "constant tree opening file error: {error}"),
            Self::Field(error) => write!(f, "constant tree opening field error: {error}"),
            #[cfg(feature = "cuda")]
            Self::Accel(error) => write!(f, "constant tree opening cuda error: {error}"),
            Self::LengthOverflow => write!(f, "constant tree opening length overflow"),
        }
    }
}

impl std::error::Error for ConstantTreeOpeningError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConstantTree(error) => Some(error),
            Self::Field(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::Accel(error) => Some(error),
            Self::UnsupportedHash
            | Self::UnsupportedArity { .. }
            | Self::EmptyValues
            | Self::RowIndexOutOfRange { .. }
            | Self::InvalidTreeLength { .. }
            | Self::InvalidSiblingWidth { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<FieldError> for ConstantTreeOpeningError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

impl From<ConstantTreeError> for ConstantTreeOpeningError {
    fn from(error: ConstantTreeError) -> Self {
        Self::ConstantTree(error)
    }
}

impl From<MerkleHashError> for ConstantTreeOpeningError {
    fn from(error: MerkleHashError) -> Self {
        match error {
            MerkleHashError::UnsupportedArity { arity } => Self::UnsupportedArity { arity },
            MerkleHashError::InvalidChildCount { expected, found } => {
                Self::InvalidSiblingWidth { expected, found }
            }
            MerkleHashError::Field(error) => Self::Field(error),
            #[cfg(feature = "cuda")]
            MerkleHashError::Accel(error) => Self::Accel(error),
            MerkleHashError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

#[derive(Clone, Copy)]
struct ConstantTreeFileShape {
    hash_kind: ConstantTreeHashKind,
    extended_row_count: u64,
    constant_count: u64,
    leaf_byte_count: usize,
    node_byte_count: usize,
    total_byte_count: usize,
}

impl ConstantTreeFileShape {
    fn from_setup(setup: &UnitSetupInfo) -> Result<Self, ConstantTreeOpeningError> {
        let extended_row_count = 1_u64
            .checked_shl(setup.stark.n_bits_ext)
            .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
        let (leaf_byte_count, node_byte_count) =
            expected_constant_tree_leaf_node_byte_counts(setup)?;
        let total_byte_count = expected_constant_tree_byte_count(setup)?;
        let hash_kind = if setup.stark.verification_hash_type.as_deref() == Some("BN128") {
            ConstantTreeHashKind::Bn128
        } else {
            ConstantTreeHashKind::Gl
        };
        Ok(Self {
            hash_kind,
            extended_row_count,
            constant_count: u64::from(setup.n_constants),
            leaf_byte_count,
            node_byte_count,
            total_byte_count,
        })
    }

    fn validate(&self, arity: usize, found_len: usize) -> Result<(), ConstantTreeOpeningError> {
        if self.hash_kind != ConstantTreeHashKind::Gl {
            return Err(ConstantTreeOpeningError::UnsupportedHash);
        }
        validate_arity(arity)?;
        if found_len != self.total_byte_count {
            return Err(ConstantTreeOpeningError::InvalidTreeLength {
                expected: self.total_byte_count,
                found: found_len,
            });
        }
        Ok(())
    }
}

pub fn open_constant_tree_row_from_file(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    row_index: u64,
    arity: usize,
) -> Result<ConstantTreeOpening, ConstantTreeOpeningError> {
    let path = path.as_ref();
    let shape = ConstantTreeFileShape::from_setup(setup)?;
    let metadata = std::fs::metadata(path).map_err(|error| ConstantTreeError::Io {
        message: error.to_string(),
    })?;
    let found_len =
        usize::try_from(metadata.len()).map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;
    shape.validate(arity, found_len)?;
    if row_index >= shape.extended_row_count {
        return Err(ConstantTreeOpeningError::RowIndexOutOfRange {
            row_index,
            row_count: shape.extended_row_count,
        });
    }

    let mut file = File::open(path).map_err(|error| ConstantTreeError::Io {
        message: error.to_string(),
    })?;
    let values = read_row_values_from_file(&mut file, shape, row_index)?;
    let level_counts = constant_tree_level_counts(shape.extended_row_count, arity)?;
    let level_offsets = constant_tree_file_level_offsets(shape, &level_counts)?;
    let mut siblings = Vec::with_capacity(level_counts.len().saturating_sub(1));
    let mut index =
        usize::try_from(row_index).map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;

    for (level, level_count) in level_counts
        .iter()
        .copied()
        .enumerate()
        .take(level_counts.len().saturating_sub(1))
    {
        let position = index % arity;
        let group_start = index
            .checked_sub(position)
            .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
        let mut level_siblings = Vec::with_capacity(arity - 1);
        for sibling_position in 0..arity {
            if sibling_position == position {
                continue;
            }
            let sibling_index = group_start
                .checked_add(sibling_position)
                .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
            if sibling_index >= level_count {
                return Err(ConstantTreeOpeningError::LengthOverflow);
            }
            level_siblings.push(read_digest_at_from_file(
                &mut file,
                level_offsets[level],
                sibling_index,
                shape.total_byte_count,
            )?);
        }
        siblings.push(level_siblings);
        index /= arity;
    }

    ConstantTreeOpening::new(row_index, values, siblings)
}

pub fn open_constant_tree_row(
    tree: &ConstantTree,
    row_index: u64,
    arity: usize,
) -> Result<ConstantTreeOpening, ConstantTreeOpeningError> {
    validate_opening_shape(tree, arity)?;
    if row_index >= tree.extended_row_count {
        return Err(ConstantTreeOpeningError::RowIndexOutOfRange {
            row_index,
            row_count: tree.extended_row_count,
        });
    }

    let values = read_row_values(tree, row_index)?;
    let level_counts = constant_tree_level_counts(tree.extended_row_count, arity)?;
    let level_offsets = constant_tree_level_offsets(tree, &level_counts)?;
    let mut siblings = Vec::with_capacity(level_counts.len().saturating_sub(1));
    let mut index =
        usize::try_from(row_index).map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;

    for (level, level_count) in level_counts
        .iter()
        .copied()
        .enumerate()
        .take(level_counts.len().saturating_sub(1))
    {
        let position = index % arity;
        let group_start = index
            .checked_sub(position)
            .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
        let mut level_siblings = Vec::with_capacity(arity - 1);
        for sibling_position in 0..arity {
            if sibling_position == position {
                continue;
            }
            let sibling_index = group_start
                .checked_add(sibling_position)
                .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
            if sibling_index >= level_count {
                return Err(ConstantTreeOpeningError::LengthOverflow);
            }
            level_siblings.push(read_digest_at(tree, level_offsets[level], sibling_index)?);
        }
        siblings.push(level_siblings);
        index /= arity;
    }

    ConstantTreeOpening::new(row_index, values, siblings)
}

pub fn verify_constant_tree_opening_root(
    root: [Felt; HASH_WORDS],
    arity: usize,
    opening: &ConstantTreeOpening,
) -> Result<bool, ConstantTreeOpeningError> {
    validate_arity(arity)?;
    let mut value = linear_hash(opening.values(), arity)?;
    let mut index = usize::try_from(opening.row_index())
        .map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;

    for level in opening.siblings() {
        if level.len() != arity - 1 {
            return Err(ConstantTreeOpeningError::InvalidSiblingWidth {
                expected: arity - 1,
                found: level.len(),
            });
        }
        let position = index % arity;
        index /= arity;

        let mut sibling_offset = 0;
        let mut children = Vec::with_capacity(arity);
        for child_position in 0..arity {
            if child_position == position {
                children.push(value);
            } else {
                children.push(level[sibling_offset]);
                sibling_offset += 1;
            }
        }
        value = parent_hash(&children, arity)?;
    }

    Ok(value == root)
}

pub fn constant_tree_merkle_level_count(
    row_count: u64,
    arity: usize,
) -> Result<usize, ConstantTreeOpeningError> {
    let counts = constant_tree_level_counts(row_count, arity)?;
    Ok(counts.len().saturating_sub(1))
}

fn validate_opening_shape(
    tree: &ConstantTree,
    arity: usize,
) -> Result<(), ConstantTreeOpeningError> {
    if tree.hash_kind != ConstantTreeHashKind::Gl {
        return Err(ConstantTreeOpeningError::UnsupportedHash);
    }
    validate_arity(arity)?;

    let expected_len = tree
        .leaf_byte_count
        .checked_add(tree.node_byte_count)
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    if tree.bytes.len() != expected_len {
        return Err(ConstantTreeOpeningError::InvalidTreeLength {
            expected: expected_len,
            found: tree.bytes.len(),
        });
    }
    Ok(())
}

fn read_row_values(
    tree: &ConstantTree,
    row_index: u64,
) -> Result<Vec<Felt>, ConstantTreeOpeningError> {
    let row = usize::try_from(row_index).map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;
    let width = usize::try_from(tree.constant_count)
        .map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;
    let word_start = row
        .checked_mul(width)
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    let byte_start = word_start
        .checked_mul(WORD_BYTES)
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    let byte_end = byte_start
        .checked_add(
            width
                .checked_mul(WORD_BYTES)
                .ok_or(ConstantTreeOpeningError::LengthOverflow)?,
        )
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    if byte_end > tree.leaf_byte_count {
        return Err(ConstantTreeOpeningError::LengthOverflow);
    }

    tree.bytes[byte_start..byte_end]
        .chunks_exact(WORD_BYTES)
        .map(read_felt)
        .collect()
}

fn constant_tree_level_counts(
    row_count: u64,
    arity: usize,
) -> Result<Vec<usize>, ConstantTreeOpeningError> {
    if row_count == 0 {
        return Err(ConstantTreeOpeningError::LengthOverflow);
    }
    validate_arity(arity)?;
    let arity_u64 = u64::try_from(arity).map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;
    let mut counts = Vec::new();
    let mut level_count = row_count;
    loop {
        let stored_count = if level_count > 1 {
            let padding = (arity_u64 - (level_count % arity_u64)) % arity_u64;
            level_count
                .checked_add(padding)
                .ok_or(ConstantTreeOpeningError::LengthOverflow)?
        } else {
            level_count
        };
        counts.push(
            usize::try_from(stored_count).map_err(|_| ConstantTreeOpeningError::LengthOverflow)?,
        );
        if level_count <= 1 {
            break;
        }
        level_count = stored_count / arity_u64;
    }
    Ok(counts)
}

fn constant_tree_level_offsets(
    tree: &ConstantTree,
    level_counts: &[usize],
) -> Result<Vec<usize>, ConstantTreeOpeningError> {
    let node_byte_count = level_counts.iter().try_fold(0_usize, |acc, count| {
        count
            .checked_mul(DIGEST_BYTES)
            .and_then(|bytes| bytes.checked_add(acc))
            .ok_or(ConstantTreeOpeningError::LengthOverflow)
    })?;
    if node_byte_count != tree.node_byte_count {
        return Err(ConstantTreeOpeningError::InvalidTreeLength {
            expected: tree.leaf_byte_count + node_byte_count,
            found: tree.bytes.len(),
        });
    }

    let mut offsets = Vec::with_capacity(level_counts.len());
    let mut offset = tree.leaf_byte_count;
    for count in level_counts {
        offsets.push(offset);
        offset = offset
            .checked_add(
                count
                    .checked_mul(DIGEST_BYTES)
                    .ok_or(ConstantTreeOpeningError::LengthOverflow)?,
            )
            .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    }
    Ok(offsets)
}

fn constant_tree_file_level_offsets(
    shape: ConstantTreeFileShape,
    level_counts: &[usize],
) -> Result<Vec<usize>, ConstantTreeOpeningError> {
    let node_byte_count = level_counts.iter().try_fold(0_usize, |acc, count| {
        count
            .checked_mul(DIGEST_BYTES)
            .and_then(|bytes| bytes.checked_add(acc))
            .ok_or(ConstantTreeOpeningError::LengthOverflow)
    })?;
    if node_byte_count != shape.node_byte_count {
        return Err(ConstantTreeOpeningError::InvalidTreeLength {
            expected: shape.leaf_byte_count + node_byte_count,
            found: shape.total_byte_count,
        });
    }

    let mut offsets = Vec::with_capacity(level_counts.len());
    let mut offset = shape.leaf_byte_count;
    for count in level_counts {
        offsets.push(offset);
        offset = offset
            .checked_add(
                count
                    .checked_mul(DIGEST_BYTES)
                    .ok_or(ConstantTreeOpeningError::LengthOverflow)?,
            )
            .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    }
    Ok(offsets)
}

fn read_row_values_from_file(
    file: &mut File,
    shape: ConstantTreeFileShape,
    row_index: u64,
) -> Result<Vec<Felt>, ConstantTreeOpeningError> {
    let row = usize::try_from(row_index).map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;
    let width = usize::try_from(shape.constant_count)
        .map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;
    let byte_count = width
        .checked_mul(WORD_BYTES)
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    let byte_start = row
        .checked_mul(byte_count)
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    let byte_end = byte_start
        .checked_add(byte_count)
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    if byte_end > shape.leaf_byte_count {
        return Err(ConstantTreeOpeningError::LengthOverflow);
    }

    let bytes = read_exact_at(file, byte_start, byte_count)?;
    bytes.chunks_exact(WORD_BYTES).map(read_felt).collect()
}

fn read_digest_at(
    tree: &ConstantTree,
    level_offset: usize,
    index: usize,
) -> Result<[Felt; HASH_WORDS], ConstantTreeOpeningError> {
    let byte_start = level_offset
        .checked_add(
            index
                .checked_mul(DIGEST_BYTES)
                .ok_or(ConstantTreeOpeningError::LengthOverflow)?,
        )
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    let byte_end = byte_start
        .checked_add(DIGEST_BYTES)
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    if byte_end > tree.bytes.len() {
        return Err(ConstantTreeOpeningError::LengthOverflow);
    }
    let mut out = [Felt::ZERO; HASH_WORDS];
    for (value, chunk) in out
        .iter_mut()
        .zip(tree.bytes[byte_start..byte_end].chunks_exact(WORD_BYTES))
    {
        *value = read_felt(chunk)?;
    }
    Ok(out)
}

fn read_digest_at_from_file(
    file: &mut File,
    level_offset: usize,
    index: usize,
    total_byte_count: usize,
) -> Result<[Felt; HASH_WORDS], ConstantTreeOpeningError> {
    let byte_start = level_offset
        .checked_add(
            index
                .checked_mul(DIGEST_BYTES)
                .ok_or(ConstantTreeOpeningError::LengthOverflow)?,
        )
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    let byte_end = byte_start
        .checked_add(DIGEST_BYTES)
        .ok_or(ConstantTreeOpeningError::LengthOverflow)?;
    if byte_end > total_byte_count {
        return Err(ConstantTreeOpeningError::LengthOverflow);
    }
    let bytes = read_exact_at(file, byte_start, DIGEST_BYTES)?;
    let mut out = [Felt::ZERO; HASH_WORDS];
    for (value, chunk) in out.iter_mut().zip(bytes.chunks_exact(WORD_BYTES)) {
        *value = read_felt(chunk)?;
    }
    Ok(out)
}

fn read_exact_at(
    file: &mut File,
    byte_start: usize,
    byte_count: usize,
) -> Result<Vec<u8>, ConstantTreeOpeningError> {
    let offset = u64::try_from(byte_start).map_err(|_| ConstantTreeOpeningError::LengthOverflow)?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| ConstantTreeError::Io {
            message: error.to_string(),
        })?;
    let mut bytes = vec![0_u8; byte_count];
    file.read_exact(&mut bytes)
        .map_err(|error| ConstantTreeError::Io {
            message: error.to_string(),
        })?;
    Ok(bytes)
}

fn read_felt(bytes: &[u8]) -> Result<Felt, ConstantTreeOpeningError> {
    Ok(Felt::from_canonical(u64::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))?)
}

fn validate_arity(arity: usize) -> Result<(), ConstantTreeOpeningError> {
    match arity {
        2 | 4 => Ok(()),
        _ => Err(ConstantTreeOpeningError::UnsupportedArity { arity }),
    }
}

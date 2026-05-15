use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::{
    parse_constant_tree_bytes, read_constant_tree_file, ConstantTreeError,
};
use lzvm_artifacts::fixed::{
    encode_raw_fixed_columns, read_raw_fixed_column_layout_file, write_raw_fixed_columns_file,
    FixedColumnError, FixedColumns,
};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_field::{coset_extend_evaluations, poseidon2_hash_8, DomainError, Felt, FieldError};

const WORD_BYTES: usize = 8;
const HASH_WORDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedColumnWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantTreeWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
    pub root: VerificationKeyRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantTreeLeavesWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
    pub row_count: u64,
    pub column_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupError {
    FixedColumns(FixedColumnError),
    ConstantTree(ConstantTreeError),
    Domain(DomainError),
    Field(FieldError),
    ConstantTreeRootMismatch {
        expected: VerificationKeyRoot,
        found: VerificationKeyRoot,
    },
    InvalidConstantTreeLeafByteLength {
        expected: usize,
        found: usize,
    },
    UnsupportedConstantTreeArity {
        arity: u32,
    },
    UnsupportedConstantTreeHash {
        hash_type: Option<String>,
    },
    LengthOverflow,
    MissingParent {
        path: PathBuf,
    },
    Io {
        role: &'static str,
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FixedColumns(error) => write!(f, "setup fixed-column error: {error}"),
            Self::ConstantTree(error) => write!(f, "setup constant-tree error: {error}"),
            Self::Domain(error) => write!(f, "setup field-domain error: {error}"),
            Self::Field(error) => write!(f, "setup field error: {error}"),
            Self::ConstantTreeRootMismatch { expected, found } => write!(
                f,
                "setup constant-tree root mismatch: expected {expected:?}, found {found:?}"
            ),
            Self::InvalidConstantTreeLeafByteLength { expected, found } => write!(
                f,
                "invalid constant-tree leaf byte length: expected {expected}, found {found}"
            ),
            Self::UnsupportedConstantTreeArity { arity } => {
                write!(f, "unsupported native constant-tree arity: {arity}")
            }
            Self::UnsupportedConstantTreeHash { hash_type } => {
                write!(f, "unsupported native constant-tree hash: {hash_type:?}")
            }
            Self::LengthOverflow => write!(f, "setup length overflow"),
            Self::MissingParent { path } => {
                write!(f, "setup output path has no parent: {}", path.display())
            }
            Self::Io {
                role,
                path,
                message,
            } => write!(f, "setup {role} io error at {}: {message}", path.display()),
        }
    }
}

impl std::error::Error for SetupError {}

impl From<FixedColumnError> for SetupError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<ConstantTreeError> for SetupError {
    fn from(error: ConstantTreeError) -> Self {
        Self::ConstantTree(error)
    }
}

impl From<DomainError> for SetupError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<FieldError> for SetupError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

pub fn extend_fixed_columns_for_constant_tree(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    let raw = encode_raw_fixed_columns(value, setup)?;
    let row_count = checked_domain_len(setup.stark.n_bits)?;
    let extended_row_count = checked_domain_len(setup.stark.n_bits_ext)?;
    let column_count =
        usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?;
    let word_count = row_count
        .checked_mul(column_count)
        .ok_or(SetupError::LengthOverflow)?;
    if raw.len()
        != word_count
            .checked_mul(8)
            .ok_or(SetupError::LengthOverflow)?
    {
        return Err(SetupError::LengthOverflow);
    }

    let mut extended_columns = Vec::with_capacity(column_count);
    for column in 0..column_count {
        let mut values = Vec::with_capacity(row_count);
        for row in 0..row_count {
            let word_index = row
                .checked_mul(column_count)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(SetupError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(8)
                .ok_or(SetupError::LengthOverflow)?;
            let value = u64::from_le_bytes(
                raw[byte_index..byte_index + 8]
                    .try_into()
                    .expect("slice length checked"),
            );
            values.push(Felt::from_canonical(value)?);
        }
        extended_columns.push(coset_extend_evaluations(
            &values,
            setup.stark.n_bits as usize,
            setup.stark.n_bits_ext as usize,
        )?);
    }

    let byte_count = extended_row_count
        .checked_mul(column_count)
        .and_then(|count| count.checked_mul(8))
        .ok_or(SetupError::LengthOverflow)?;
    let mut out = Vec::with_capacity(byte_count);
    for row in 0..extended_row_count {
        for column_values in &extended_columns {
            out.extend_from_slice(&column_values[row].to_le_bytes());
        }
    }
    Ok(out)
}

pub fn write_base_fixed_columns(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<FixedColumnWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let staging_path = staging_path_for(&path);
    write_raw_fixed_columns_file(&staging_path, value, setup)?;
    read_raw_fixed_column_layout_file(&staging_path, setup, &value.group_name, &value.unit_name)?;
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish fixed columns",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(FixedColumnWriteReport {
        path,
        bytes_written,
    })
}

pub fn write_constant_tree_leaves(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeLeavesWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let leaves = extend_fixed_columns_for_constant_tree(value, setup)?;
    let expected_len = checked_domain_len(setup.stark.n_bits_ext)?
        .checked_mul(usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?)
        .and_then(|words| words.checked_mul(8))
        .ok_or(SetupError::LengthOverflow)?;
    if leaves.len() != expected_len {
        return Err(SetupError::LengthOverflow);
    }

    let staging_path = staging_path_for(&path);
    std::fs::write(&staging_path, &leaves).map_err(|error| SetupError::Io {
        role: "write constant-tree leaves staging file",
        path: staging_path.clone(),
        message: error.to_string(),
    })?;
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    if bytes_written != u64::try_from(expected_len).map_err(|_| SetupError::LengthOverflow)? {
        return Err(SetupError::LengthOverflow);
    }
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish constant-tree leaves",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(ConstantTreeLeavesWriteReport {
        path,
        bytes_written,
        row_count: 1_u64
            .checked_shl(setup.stark.n_bits_ext)
            .ok_or(SetupError::LengthOverflow)?,
        column_count: setup.n_constants,
    })
}

pub fn build_constant_tree_from_fixed_columns(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    let leaves = extend_fixed_columns_for_constant_tree(value, setup)?;
    build_constant_tree_from_leaves(&leaves, setup)
}

pub fn build_constant_tree_from_leaves(
    leaves: &[u8],
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    validate_native_constant_tree_setup(setup)?;
    let row_count = checked_domain_len(setup.stark.n_bits_ext)?;
    let column_count =
        usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?;
    let expected_leaf_len = constant_tree_leaf_byte_count(row_count, column_count)?;
    if leaves.len() != expected_leaf_len {
        return Err(SetupError::InvalidConstantTreeLeafByteLength {
            expected: expected_leaf_len,
            found: leaves.len(),
        });
    }

    let mut out = Vec::with_capacity(expected_constant_tree_byte_count_for_setup(setup)?);
    out.extend_from_slice(leaves);

    let mut level = Vec::with_capacity(row_count);
    for row in 0..row_count {
        let mut values = Vec::with_capacity(column_count);
        for column in 0..column_count {
            let word_index = row
                .checked_mul(column_count)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(SetupError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(WORD_BYTES)
                .ok_or(SetupError::LengthOverflow)?;
            let value = u64::from_le_bytes(
                leaves[byte_index..byte_index + WORD_BYTES]
                    .try_into()
                    .expect("slice length checked"),
            );
            values.push(Felt::from_canonical(value)?);
        }
        let digest = linear_hash_arity2(&values);
        append_digest(&mut out, digest);
        level.push(digest);
    }

    while level.len() > 1 {
        if level.len() % 2 != 0 {
            let zero = [Felt::ZERO; HASH_WORDS];
            append_digest(&mut out, zero);
            level.push(zero);
        }

        let mut next = Vec::with_capacity(level.len() / 2);
        for pair in level.chunks_exact(2) {
            let digest = parent_hash_arity2(pair[0], pair[1]);
            append_digest(&mut out, digest);
            next.push(digest);
        }
        level = next;
    }

    parse_constant_tree_bytes(out.clone(), setup)?;
    Ok(out)
}

pub fn write_constant_tree_from_fixed_columns(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeWriteReport, SetupError> {
    let tree = build_constant_tree_from_fixed_columns(value, setup)?;
    write_base_constant_tree(path, &tree, setup, None)
}

pub fn write_base_constant_tree(
    path: impl AsRef<Path>,
    value: &[u8],
    setup: &UnitSetupInfo,
    expected_root: Option<&VerificationKeyRoot>,
) -> Result<ConstantTreeWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let tree = parse_constant_tree_bytes(value.to_vec(), setup)?;
    let root = tree.root()?;
    if let Some(expected) = expected_root {
        if expected != &root {
            return Err(SetupError::ConstantTreeRootMismatch {
                expected: expected.clone(),
                found: root,
            });
        }
    }

    let staging_path = staging_path_for(&path);
    std::fs::write(&staging_path, value).map_err(|error| SetupError::Io {
        role: "write constant-tree staging file",
        path: staging_path.clone(),
        message: error.to_string(),
    })?;
    let staged_tree = read_constant_tree_file(&staging_path, setup)?;
    let staged_root = staged_tree.root()?;
    if staged_root != root {
        return Err(SetupError::ConstantTreeRootMismatch {
            expected: root,
            found: staged_root,
        });
    }
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish constant tree",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(ConstantTreeWriteReport {
        path,
        bytes_written,
        root,
    })
}

fn staging_path_for(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "fixed-columns".into());
    name.push(format!(".staging.{}", std::process::id()));
    path.with_file_name(name)
}

fn checked_domain_len(bits: u32) -> Result<usize, SetupError> {
    1_usize.checked_shl(bits).ok_or(SetupError::LengthOverflow)
}

fn validate_native_constant_tree_setup(setup: &UnitSetupInfo) -> Result<(), SetupError> {
    if setup.stark.merkle_tree_arity != 2 {
        return Err(SetupError::UnsupportedConstantTreeArity {
            arity: setup.stark.merkle_tree_arity,
        });
    }
    match setup.stark.verification_hash_type.as_deref() {
        None | Some("GL") => Ok(()),
        _ => Err(SetupError::UnsupportedConstantTreeHash {
            hash_type: setup.stark.verification_hash_type.clone(),
        }),
    }
}

fn constant_tree_leaf_byte_count(
    row_count: usize,
    column_count: usize,
) -> Result<usize, SetupError> {
    row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(SetupError::LengthOverflow)
}

fn expected_constant_tree_byte_count_for_setup(setup: &UnitSetupInfo) -> Result<usize, SetupError> {
    lzvm_artifacts::constant_tree::expected_constant_tree_byte_count(setup).map_err(Into::into)
}

fn linear_hash_arity2(values: &[Felt]) -> [Felt; HASH_WORDS] {
    if values.len() <= HASH_WORDS {
        let mut digest = [Felt::ZERO; HASH_WORDS];
        digest[..values.len()].copy_from_slice(values);
        return digest;
    }

    let mut state = [Felt::ZERO; 8];
    let mut offset = 0;
    while offset < values.len() {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[4..].copy_from_slice(&capacity);
        state[..HASH_WORDS].fill(Felt::ZERO);

        let chunk_len = (values.len() - offset).min(HASH_WORDS);
        state[..chunk_len].copy_from_slice(&values[offset..offset + chunk_len]);
        state = poseidon2_hash_8(state);
        offset += chunk_len;
    }

    [state[0], state[1], state[2], state[3]]
}

fn parent_hash_arity2(left: [Felt; HASH_WORDS], right: [Felt; HASH_WORDS]) -> [Felt; HASH_WORDS] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

fn append_digest(out: &mut Vec<u8>, digest: [Felt; HASH_WORDS]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

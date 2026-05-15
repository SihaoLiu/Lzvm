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
use lzvm_field::{coset_extend_evaluations, DomainError, Felt, FieldError};

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
pub enum SetupError {
    FixedColumns(FixedColumnError),
    ConstantTree(ConstantTreeError),
    Domain(DomainError),
    Field(FieldError),
    ConstantTreeRootMismatch {
        expected: VerificationKeyRoot,
        found: VerificationKeyRoot,
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

use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::{
    parse_constant_tree_bytes, read_constant_tree_file, ConstantTreeError,
};
use lzvm_artifacts::fixed::{
    read_raw_fixed_column_layout_file, write_raw_fixed_columns_file, FixedColumnError, FixedColumns,
};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_artifacts::verification_key::VerificationKeyRoot;

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
    ConstantTreeRootMismatch {
        expected: VerificationKeyRoot,
        found: VerificationKeyRoot,
    },
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
            Self::ConstantTreeRootMismatch { expected, found } => write!(
                f,
                "setup constant-tree root mismatch: expected {expected:?}, found {found:?}"
            ),
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

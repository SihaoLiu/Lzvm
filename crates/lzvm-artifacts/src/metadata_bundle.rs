use crate::constant_tree::{read_constant_tree_file, ConstantTree, ConstantTreeError};
use crate::expression_info::{read_expression_info_file, ExpressionInfo, ExpressionInfoError};
use crate::expression_program::{
    read_expression_program_file, ExpressionProgram, ExpressionProgramError,
};
use crate::fixed::{read_fixed_columns_file_for_setup, FixedColumnError, FixedColumns};
use crate::global_info::{read_global_info_file, GlobalInfo, GlobalInfoError};
use crate::metadata_validation::{
    validate_global_metadata, validate_unit_metadata, MetadataValidationError,
};
use crate::setup_info::{read_unit_setup_info_file, SetupInfoError, UnitSetupInfo};
use crate::verification_key::{
    read_verification_key_binary_file, read_verification_key_json_file, VerificationKeyError,
    VerificationKeyRoot,
};
use crate::verifier_info::{read_verifier_info_file, VerifierInfo, VerifierInfoError};
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitMetadataPaths {
    pub setup_info: PathBuf,
    pub expression_info: PathBuf,
    pub verifier_info: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalMetadataPaths {
    pub info: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitArtifactPaths {
    pub metadata: UnitMetadataPaths,
    pub verification_key_json: PathBuf,
    pub verification_key_binary: PathBuf,
    pub expression_program: PathBuf,
    pub verifier_program: PathBuf,
    pub fixed_columns: PathBuf,
    pub constant_tree: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitMetadataBundle {
    pub setup: UnitSetupInfo,
    pub expressions: ExpressionInfo,
    pub verifier: VerifierInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalMetadataBundle {
    pub info: GlobalInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitArtifactBundle {
    pub metadata: UnitMetadataBundle,
    pub verification_key: VerificationKeyRoot,
    pub expression_program: ExpressionProgram,
    pub verifier_program: ExpressionProgram,
    pub fixed_columns: FixedColumns,
    pub constant_tree: ConstantTree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataBundleError {
    SetupInfo(SetupInfoError),
    ExpressionInfo(ExpressionInfoError),
    VerifierInfo(VerifierInfoError),
    GlobalInfo(GlobalInfoError),
    ExpressionProgram(ExpressionProgramError),
    FixedColumns(FixedColumnError),
    ConstantTree(ConstantTreeError),
    VerificationKey(VerificationKeyError),
    FixedColumnDomainTooLarge {
        n_bits: u32,
    },
    FixedColumnRowCountMismatch {
        expected: u64,
        found: u64,
    },
    VerificationKeyMismatch {
        json_root: VerificationKeyRoot,
        binary_root: VerificationKeyRoot,
    },
    ConstantTreeRootMismatch {
        tree_root: VerificationKeyRoot,
        verification_key: VerificationKeyRoot,
    },
    Validation(MetadataValidationError),
}

impl UnitMetadataPaths {
    pub fn new(
        setup_info: impl Into<PathBuf>,
        expression_info: impl Into<PathBuf>,
        verifier_info: impl Into<PathBuf>,
    ) -> Self {
        Self {
            setup_info: setup_info.into(),
            expression_info: expression_info.into(),
            verifier_info: verifier_info.into(),
        }
    }

    pub fn from_unit_prefix(prefix: impl AsRef<Path>) -> Self {
        let prefix = prefix.as_ref();
        Self {
            setup_info: append_suffix(prefix, ".starkinfo.json"),
            expression_info: append_suffix(prefix, ".expressionsinfo.json"),
            verifier_info: append_suffix(prefix, ".verifierinfo.json"),
        }
    }
}

impl GlobalMetadataPaths {
    pub fn new(info: impl Into<PathBuf>) -> Self {
        Self { info: info.into() }
    }
}

impl UnitArtifactPaths {
    pub fn new(
        metadata: UnitMetadataPaths,
        verification_key_json: impl Into<PathBuf>,
        verification_key_binary: impl Into<PathBuf>,
        expression_program: impl Into<PathBuf>,
        verifier_program: impl Into<PathBuf>,
        fixed_columns: impl Into<PathBuf>,
        constant_tree: impl Into<PathBuf>,
    ) -> Self {
        Self {
            metadata,
            verification_key_json: verification_key_json.into(),
            verification_key_binary: verification_key_binary.into(),
            expression_program: expression_program.into(),
            verifier_program: verifier_program.into(),
            fixed_columns: fixed_columns.into(),
            constant_tree: constant_tree.into(),
        }
    }

    pub fn from_unit_prefix(prefix: impl AsRef<Path>) -> Self {
        let prefix = prefix.as_ref();
        Self {
            metadata: UnitMetadataPaths::from_unit_prefix(prefix),
            verification_key_json: append_suffix(prefix, ".verkey.json"),
            verification_key_binary: append_suffix(prefix, ".verkey.bin"),
            expression_program: append_suffix(prefix, ".bin"),
            verifier_program: append_suffix(prefix, ".verifier.bin"),
            fixed_columns: append_suffix(prefix, ".const"),
            constant_tree: append_suffix(prefix, ".consttree"),
        }
    }
}

impl fmt::Display for MetadataBundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupInfo(error) => write!(f, "setup metadata bundle error: {error}"),
            Self::ExpressionInfo(error) => write!(f, "expression metadata bundle error: {error}"),
            Self::VerifierInfo(error) => write!(f, "verifier metadata bundle error: {error}"),
            Self::GlobalInfo(error) => write!(f, "global metadata bundle error: {error}"),
            Self::ExpressionProgram(error) => {
                write!(f, "expression program bundle error: {error}")
            }
            Self::FixedColumns(error) => write!(f, "fixed-column bundle error: {error}"),
            Self::ConstantTree(error) => write!(f, "constant-tree bundle error: {error}"),
            Self::VerificationKey(error) => {
                write!(f, "verification-key metadata bundle error: {error}")
            }
            Self::FixedColumnDomainTooLarge { n_bits } => {
                write!(f, "fixed-column domain bit count is too large: {n_bits}")
            }
            Self::FixedColumnRowCountMismatch { expected, found } => write!(
                f,
                "fixed-column row count mismatch: expected {expected}, found {found}"
            ),
            Self::VerificationKeyMismatch { .. } => {
                write!(f, "verification-key companion roots do not match")
            }
            Self::ConstantTreeRootMismatch { .. } => {
                write!(f, "constant-tree root does not match verification key")
            }
            Self::Validation(error) => write!(f, "metadata bundle validation error: {error}"),
        }
    }
}

impl std::error::Error for MetadataBundleError {}

impl From<SetupInfoError> for MetadataBundleError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<ExpressionInfoError> for MetadataBundleError {
    fn from(error: ExpressionInfoError) -> Self {
        Self::ExpressionInfo(error)
    }
}

impl From<VerifierInfoError> for MetadataBundleError {
    fn from(error: VerifierInfoError) -> Self {
        Self::VerifierInfo(error)
    }
}

impl From<GlobalInfoError> for MetadataBundleError {
    fn from(error: GlobalInfoError) -> Self {
        Self::GlobalInfo(error)
    }
}

impl From<ExpressionProgramError> for MetadataBundleError {
    fn from(error: ExpressionProgramError) -> Self {
        Self::ExpressionProgram(error)
    }
}

impl From<FixedColumnError> for MetadataBundleError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<ConstantTreeError> for MetadataBundleError {
    fn from(error: ConstantTreeError) -> Self {
        Self::ConstantTree(error)
    }
}

impl From<VerificationKeyError> for MetadataBundleError {
    fn from(error: VerificationKeyError) -> Self {
        Self::VerificationKey(error)
    }
}

impl From<MetadataValidationError> for MetadataBundleError {
    fn from(error: MetadataValidationError) -> Self {
        Self::Validation(error)
    }
}

pub fn read_unit_metadata_bundle(
    paths: &UnitMetadataPaths,
) -> Result<UnitMetadataBundle, MetadataBundleError> {
    let setup = read_unit_setup_info_file(&paths.setup_info)?;
    let expressions = read_expression_info_file(&paths.expression_info)?;
    let verifier = read_verifier_info_file(&paths.verifier_info)?;

    validate_unit_metadata(&setup, &expressions, &verifier)?;

    Ok(UnitMetadataBundle {
        setup,
        expressions,
        verifier,
    })
}

pub fn read_unit_artifact_bundle(
    paths: &UnitArtifactPaths,
) -> Result<UnitArtifactBundle, MetadataBundleError> {
    let metadata = read_unit_metadata_bundle(&paths.metadata)?;
    let binary_root = read_verification_key_binary_file(&paths.verification_key_binary)?;

    if paths.verification_key_json.is_file() {
        let json_root = read_verification_key_json_file(&paths.verification_key_json)?;
        if json_root != binary_root {
            return Err(MetadataBundleError::VerificationKeyMismatch {
                json_root,
                binary_root,
            });
        }
    }
    let expression_program = read_expression_program_file(&paths.expression_program)?;
    let verifier_program = read_expression_program_file(&paths.verifier_program)?;
    let fixed_columns = read_fixed_columns_file_for_setup(
        &paths.fixed_columns,
        &metadata.setup,
        "",
        fixed_unit_name(&paths.fixed_columns),
    )?;
    validate_fixed_column_rows(&metadata, &fixed_columns)?;
    let constant_tree = read_constant_tree_file(&paths.constant_tree, &metadata.setup)?;
    validate_constant_tree_root(&constant_tree, &binary_root)?;

    Ok(UnitArtifactBundle {
        metadata,
        verification_key: binary_root,
        expression_program,
        verifier_program,
        fixed_columns,
        constant_tree,
    })
}

fn validate_constant_tree_root(
    constant_tree: &ConstantTree,
    verification_key: &VerificationKeyRoot,
) -> Result<(), MetadataBundleError> {
    let tree_root = constant_tree.root()?;
    if &tree_root != verification_key {
        return Err(MetadataBundleError::ConstantTreeRootMismatch {
            tree_root,
            verification_key: verification_key.clone(),
        });
    }
    Ok(())
}

fn validate_fixed_column_rows(
    metadata: &UnitMetadataBundle,
    fixed_columns: &FixedColumns,
) -> Result<(), MetadataBundleError> {
    let expected = 1_u64.checked_shl(metadata.setup.stark.n_bits).ok_or(
        MetadataBundleError::FixedColumnDomainTooLarge {
            n_bits: metadata.setup.stark.n_bits,
        },
    )?;
    if fixed_columns.row_count != expected {
        return Err(MetadataBundleError::FixedColumnRowCountMismatch {
            expected,
            found: fixed_columns.row_count,
        });
    }
    Ok(())
}

pub fn read_global_metadata_bundle(
    paths: &GlobalMetadataPaths,
) -> Result<GlobalMetadataBundle, MetadataBundleError> {
    let info = read_global_info_file(&paths.info)?;
    validate_global_metadata(&info)?;
    Ok(GlobalMetadataBundle { info })
}

fn append_suffix(prefix: &Path, suffix: &str) -> PathBuf {
    let mut value: OsString = prefix.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}

fn fixed_unit_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

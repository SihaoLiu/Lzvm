use crate::expression_info::{read_expression_info_file, ExpressionInfo, ExpressionInfoError};
use crate::global_info::{read_global_info_file, GlobalInfo, GlobalInfoError};
use crate::metadata_validation::{
    validate_global_metadata, validate_unit_metadata, MetadataValidationError,
};
use crate::setup_info::{read_unit_setup_info_file, SetupInfoError, UnitSetupInfo};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataBundleError {
    SetupInfo(SetupInfoError),
    ExpressionInfo(ExpressionInfoError),
    VerifierInfo(VerifierInfoError),
    GlobalInfo(GlobalInfoError),
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

impl fmt::Display for MetadataBundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupInfo(error) => write!(f, "setup metadata bundle error: {error}"),
            Self::ExpressionInfo(error) => write!(f, "expression metadata bundle error: {error}"),
            Self::VerifierInfo(error) => write!(f, "verifier metadata bundle error: {error}"),
            Self::GlobalInfo(error) => write!(f, "global metadata bundle error: {error}"),
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

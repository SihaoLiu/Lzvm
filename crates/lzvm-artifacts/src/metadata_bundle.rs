use crate::expression_info::{read_expression_info_file, ExpressionInfo, ExpressionInfoError};
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataBundleError {
    SetupInfo(SetupInfoError),
    ExpressionInfo(ExpressionInfoError),
    VerifierInfo(VerifierInfoError),
    GlobalInfo(GlobalInfoError),
    VerificationKey(VerificationKeyError),
    VerificationKeyMismatch {
        json_root: VerificationKeyRoot,
        binary_root: VerificationKeyRoot,
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
    ) -> Self {
        Self {
            metadata,
            verification_key_json: verification_key_json.into(),
            verification_key_binary: verification_key_binary.into(),
        }
    }

    pub fn from_unit_prefix(prefix: impl AsRef<Path>) -> Self {
        let prefix = prefix.as_ref();
        Self {
            metadata: UnitMetadataPaths::from_unit_prefix(prefix),
            verification_key_json: append_suffix(prefix, ".verkey.json"),
            verification_key_binary: append_suffix(prefix, ".verkey.bin"),
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
            Self::VerificationKey(error) => {
                write!(f, "verification-key metadata bundle error: {error}")
            }
            Self::VerificationKeyMismatch { .. } => {
                write!(f, "verification-key companion roots do not match")
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
    let json_root = read_verification_key_json_file(&paths.verification_key_json)?;
    let binary_root = read_verification_key_binary_file(&paths.verification_key_binary)?;

    if json_root != binary_root {
        return Err(MetadataBundleError::VerificationKeyMismatch {
            json_root,
            binary_root,
        });
    }

    Ok(UnitArtifactBundle {
        metadata,
        verification_key: json_root,
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

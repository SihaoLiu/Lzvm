use std::fmt;
use std::path::PathBuf;

use lzvm_artifacts::constant_tree::ConstantTreeError;
use lzvm_artifacts::expression_info::ExpressionInfoError;
use lzvm_artifacts::fixed::FixedColumnError;
use lzvm_artifacts::global_info::GlobalInfoError;
use lzvm_artifacts::hint_program::HintProgramError;
use lzvm_artifacts::key_directory::KeyDirectoryError;
use lzvm_artifacts::regular_program::{RegularProgramError, RegularProgramLoweringError};
use lzvm_artifacts::setup_info::SetupInfoError;
use lzvm_artifacts::verification_key::{VerificationKeyError, VerificationKeyRoot};
use lzvm_artifacts::verifier_info::VerifierInfoError;
use lzvm_field::{DomainError, FieldError};

use crate::directory_manifest::SetupDirectorySummaryError;
use crate::pcs::PcsDirectoryWriteError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupError {
    FixedColumns(FixedColumnError),
    ConstantTree(ConstantTreeError),
    VerificationKey(VerificationKeyError),
    Domain(DomainError),
    Field(FieldError),
    CudaUnavailable,
    CudaBackend(String),
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
            Self::VerificationKey(error) => write!(f, "setup verification-key error: {error}"),
            Self::Domain(error) => write!(f, "setup field-domain error: {error}"),
            Self::Field(error) => write!(f, "setup field error: {error}"),
            Self::CudaUnavailable => write!(f, "setup cuda backend is not enabled"),
            Self::CudaBackend(message) => write!(f, "setup cuda backend error: {message}"),
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

impl From<VerificationKeyError> for SetupError {
    fn from(error: VerificationKeyError) -> Self {
        Self::VerificationKey(error)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeFileWriteError {
    SetupInfo(SetupInfoError),
    FixedColumns(FixedColumnError),
    Setup(SetupError),
}

impl fmt::Display for NativeFileWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupInfo(error) => write!(f, "{error}"),
            Self::FixedColumns(error) => write!(f, "{error}"),
            Self::Setup(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for NativeFileWriteError {}

impl From<SetupInfoError> for NativeFileWriteError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<FixedColumnError> for NativeFileWriteError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<SetupError> for NativeFileWriteError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseDirectoryWriteError {
    KeyDirectory(KeyDirectoryError),
    GlobalInfo(GlobalInfoError),
    SetupInfo(SetupInfoError),
    ExpressionInfo(ExpressionInfoError),
    VerifierInfo(VerifierInfoError),
    FixedColumns(FixedColumnError),
    HintProgram(HintProgramError),
    RegularProgram(RegularProgramError),
    RegularProgramLowering(RegularProgramLoweringError),
    VerificationKey(VerificationKeyError),
    Setup(SetupError),
    MissingUnitPath { role: &'static str },
    Message { message: String },
}

impl BaseDirectoryWriteError {
    pub(crate) fn message(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}

impl fmt::Display for BaseDirectoryWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyDirectory(error) => write!(f, "{error}"),
            Self::GlobalInfo(error) => write!(f, "{error}"),
            Self::SetupInfo(error) => write!(f, "{error}"),
            Self::ExpressionInfo(error) => write!(f, "{error}"),
            Self::VerifierInfo(error) => write!(f, "{error}"),
            Self::FixedColumns(error) => write!(f, "{error}"),
            Self::HintProgram(error) => write!(f, "{error}"),
            Self::RegularProgram(error) => write!(f, "{error}"),
            Self::RegularProgramLowering(error) => write!(f, "{error}"),
            Self::VerificationKey(error) => write!(f, "{error}"),
            Self::Setup(error) => write!(f, "{error}"),
            Self::MissingUnitPath { role } => write!(f, "missing unit {role}"),
            Self::Message { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for BaseDirectoryWriteError {}

impl From<KeyDirectoryError> for BaseDirectoryWriteError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::KeyDirectory(error)
    }
}

impl From<GlobalInfoError> for BaseDirectoryWriteError {
    fn from(error: GlobalInfoError) -> Self {
        Self::GlobalInfo(error)
    }
}

impl From<SetupInfoError> for BaseDirectoryWriteError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<ExpressionInfoError> for BaseDirectoryWriteError {
    fn from(error: ExpressionInfoError) -> Self {
        Self::ExpressionInfo(error)
    }
}

impl From<VerifierInfoError> for BaseDirectoryWriteError {
    fn from(error: VerifierInfoError) -> Self {
        Self::VerifierInfo(error)
    }
}

impl From<FixedColumnError> for BaseDirectoryWriteError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<HintProgramError> for BaseDirectoryWriteError {
    fn from(error: HintProgramError) -> Self {
        Self::HintProgram(error)
    }
}

impl From<RegularProgramError> for BaseDirectoryWriteError {
    fn from(error: RegularProgramError) -> Self {
        Self::RegularProgram(error)
    }
}

impl From<RegularProgramLoweringError> for BaseDirectoryWriteError {
    fn from(error: RegularProgramLoweringError) -> Self {
        Self::RegularProgramLowering(error)
    }
}

impl From<VerificationKeyError> for BaseDirectoryWriteError {
    fn from(error: VerificationKeyError) -> Self {
        Self::VerificationKey(error)
    }
}

impl From<SetupError> for BaseDirectoryWriteError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyDirectoryWriteError {
    Base(BaseDirectoryWriteError),
    Pcs(PcsDirectoryWriteError),
    Manifest(SetupDirectorySummaryError),
}

impl fmt::Display for KeyDirectoryWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base(error) => write!(f, "{error}"),
            Self::Pcs(error) => write!(f, "{error}"),
            Self::Manifest(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for KeyDirectoryWriteError {}

impl From<BaseDirectoryWriteError> for KeyDirectoryWriteError {
    fn from(error: BaseDirectoryWriteError) -> Self {
        Self::Base(error)
    }
}

impl From<PcsDirectoryWriteError> for KeyDirectoryWriteError {
    fn from(error: PcsDirectoryWriteError) -> Self {
        Self::Pcs(error)
    }
}

impl From<SetupDirectorySummaryError> for KeyDirectoryWriteError {
    fn from(error: SetupDirectorySummaryError) -> Self {
        Self::Manifest(error)
    }
}

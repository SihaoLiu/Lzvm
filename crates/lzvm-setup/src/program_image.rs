use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::guest_image::{read_guest_image_file, GuestImageError};
use lzvm_artifacts::program_image::{
    build_program_image_commitment_cache, encode_program_image_commitment_cache,
    read_program_image_commitment_cache_file, ProgramImageCommitmentCache,
    ProgramImageCommitmentCacheError, ProgramImageCommitmentInputs, ProgramImageGpuMode,
};
use lzvm_artifacts::verification_key::{
    read_verification_key_binary_file, VerificationKeyError, VerificationKeyRoot,
};

use crate::{publish_staging_bytes, write_staging_bytes, SetupError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramImageCommitmentCacheWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ProgramImageCommitmentCacheFileRequest<'a> {
    pub program_path: &'a Path,
    pub guest_image_path: &'a Path,
    pub constraint_digest_path: &'a Path,
    pub root_path: &'a Path,
    pub trace_row_count: u64,
    pub trace_column_count: u32,
    pub blowup_factor: u32,
    pub merkle_tree_arity: u32,
    pub gpu_mode: ProgramImageGpuMode,
    pub output_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramImageCommitmentCacheWriteError {
    ProgramImage(ProgramImageCommitmentCacheError),
    GuestImage {
        path: PathBuf,
        source: GuestImageError,
    },
    VerificationKey {
        path: PathBuf,
        source: VerificationKeyError,
    },
    ConstraintDigestLength {
        path: PathBuf,
        expected: usize,
        found: usize,
    },
    Io {
        role: &'static str,
        path: PathBuf,
        message: String,
    },
    Setup(SetupError),
}

impl fmt::Display for ProgramImageCommitmentCacheWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramImage(error) => {
                write!(f, "program-image commitment cache build failed: {error}")
            }
            Self::GuestImage { path, source } => write!(
                f,
                "program-image commitment cache guest image failed at {}: {source}",
                path.display()
            ),
            Self::VerificationKey { path, source } => write!(
                f,
                "program-image commitment cache verification-key failed at {}: {source}",
                path.display()
            ),
            Self::ConstraintDigestLength {
                path,
                expected,
                found,
            } => write!(
                f,
                "program-image commitment cache constraint digest length mismatch at {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::Io {
                role,
                path,
                message,
            } => write!(
                f,
                "program-image commitment cache {role} io error at {}: {message}",
                path.display()
            ),
            Self::Setup(error) => write!(f, "program-image commitment cache write failed: {error}"),
        }
    }
}

impl std::error::Error for ProgramImageCommitmentCacheWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ProgramImage(error) => Some(error),
            Self::GuestImage { source, .. } => Some(source),
            Self::VerificationKey { source, .. } => Some(source),
            Self::Setup(error) => Some(error),
            Self::ConstraintDigestLength { .. } | Self::Io { .. } => None,
        }
    }
}

impl From<ProgramImageCommitmentCacheError> for ProgramImageCommitmentCacheWriteError {
    fn from(error: ProgramImageCommitmentCacheError) -> Self {
        Self::ProgramImage(error)
    }
}

impl From<SetupError> for ProgramImageCommitmentCacheWriteError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
}

pub fn write_program_image_commitment_cache_file(
    request: ProgramImageCommitmentCacheFileRequest<'_>,
) -> Result<ProgramImageCommitmentCacheWriteReport, ProgramImageCommitmentCacheWriteError> {
    let program_path = request.program_path;
    let guest_image_path = request.guest_image_path;
    let constraint_digest_path = request.constraint_digest_path;
    let root_path = request.root_path;
    let program_bytes = read_program_image_input_bytes(program_path, "read program input")?;
    let source_image_info = read_guest_image_file(guest_image_path).map_err(|source| {
        ProgramImageCommitmentCacheWriteError::GuestImage {
            path: guest_image_path.to_path_buf(),
            source,
        }
    })?;
    let source_image_bytes =
        read_program_image_input_bytes(guest_image_path, "read source image input")?;
    let constraint_system_digest = read_constraint_digest_file(constraint_digest_path)?;
    let root = read_verification_key_binary_file(root_path).map_err(|source| {
        ProgramImageCommitmentCacheWriteError::VerificationKey {
            path: root_path.to_path_buf(),
            source,
        }
    })?;
    let cache = build_program_image_commitment_cache(ProgramImageCommitmentInputs {
        program_bytes: &program_bytes,
        source_image_bytes: &source_image_bytes,
        constraint_system_digest,
        tree_root: verification_key_root_words(root),
        trace_row_count: request.trace_row_count,
        trace_column_count: request.trace_column_count,
        blowup_factor: request.blowup_factor,
        merkle_tree_arity: request.merkle_tree_arity,
        gpu_mode: request.gpu_mode,
    })?;
    debug_assert_eq!(cache.source_image_digest, source_image_info.digest);
    write_program_image_commitment_cache(request.output_path, &cache)
}

pub fn write_program_image_commitment_cache(
    path: impl AsRef<Path>,
    value: &ProgramImageCommitmentCache,
) -> Result<ProgramImageCommitmentCacheWriteReport, ProgramImageCommitmentCacheWriteError> {
    let path = path.as_ref().to_path_buf();
    let bytes = encode_program_image_commitment_cache(value)?;
    let staging_path = write_staging_bytes(
        &path,
        &bytes,
        "write program-image commitment cache staging file",
    )?;
    read_program_image_commitment_cache_file(&staging_path)?;
    let bytes_written = publish_staging_bytes(
        &staging_path,
        &path,
        "publish program-image commitment cache",
    )?;
    Ok(ProgramImageCommitmentCacheWriteReport {
        path,
        bytes_written,
    })
}

fn read_constraint_digest_file(
    path: &Path,
) -> Result<[u8; 32], ProgramImageCommitmentCacheWriteError> {
    const DIGEST_BYTES: usize = 32;

    let bytes = read_program_image_input_bytes(path, "read constraint digest input")?;
    if bytes.len() != DIGEST_BYTES {
        return Err(
            ProgramImageCommitmentCacheWriteError::ConstraintDigestLength {
                path: path.to_path_buf(),
                expected: DIGEST_BYTES,
                found: bytes.len(),
            },
        );
    }
    Ok(bytes.try_into().expect("digest length checked"))
}

fn read_program_image_input_bytes(
    path: &Path,
    role: &'static str,
) -> Result<Vec<u8>, ProgramImageCommitmentCacheWriteError> {
    std::fs::read(path).map_err(|error| ProgramImageCommitmentCacheWriteError::Io {
        role,
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn verification_key_root_words(root: VerificationKeyRoot) -> [u64; 4] {
    let VerificationKeyRoot::FieldElements(values) = root;
    values
        .try_into()
        .expect("verification-key binary root length checked")
}

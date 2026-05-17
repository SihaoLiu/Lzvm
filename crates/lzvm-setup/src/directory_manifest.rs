use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::key_directory::{
    read_key_directory_catalog, read_key_directory_catalog_from_layout, KeyDirectoryError,
    KeyDirectoryLayout,
};
use lzvm_artifacts::setup_manifest::{
    build_setup_directory_manifest, encode_setup_directory_manifest,
    read_setup_directory_manifest_file, validate_setup_directory_manifest_file,
    SetupDirectoryManifest, SetupDirectoryManifestError, SETUP_DIRECTORY_MANIFEST_FILE,
};

use crate::{publish_staging_bytes, write_staging_bytes, SetupDirectorySummaryReport, SetupError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupDirectoryManifestWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupDirectorySummaryError {
    Catalog(KeyDirectoryError),
    Manifest(SetupDirectoryManifestError),
    ManifestMismatch { path: PathBuf },
    Setup(SetupError),
}

impl fmt::Display for SetupDirectorySummaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "{error}"),
            Self::Manifest(error) => write!(f, "{error}"),
            Self::ManifestMismatch { path } => {
                write!(f, "setup directory manifest mismatch at {}", path.display())
            }
            Self::Setup(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SetupDirectorySummaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Manifest(error) => Some(error),
            Self::Setup(error) => Some(error),
            Self::ManifestMismatch { .. } => None,
        }
    }
}

impl From<KeyDirectoryError> for SetupDirectorySummaryError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::Catalog(error)
    }
}

impl From<SetupDirectoryManifestError> for SetupDirectorySummaryError {
    fn from(error: SetupDirectoryManifestError) -> Self {
        Self::Manifest(error)
    }
}

impl From<SetupError> for SetupDirectorySummaryError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
}

pub fn summarize_setup_directory(
    root: impl AsRef<Path>,
) -> Result<SetupDirectorySummaryReport, SetupDirectorySummaryError> {
    let root = root.as_ref();
    let catalog = read_key_directory_catalog(root)?;
    let manifest = build_setup_directory_manifest(&catalog)?;
    validate_manifest_file_if_present(root, &manifest)?;
    let source_fixed_file_manifest_present = catalog.source_fixed_file_manifest.is_some();
    let source_fixed_file_manifest_entry_count = catalog
        .source_fixed_file_manifest
        .as_ref()
        .map(|manifest| manifest.entries.len())
        .unwrap_or_default();
    let source_program_archive_present = catalog.source_program_archive.is_some();
    let source_program_archive_source_count = catalog
        .source_program_archive
        .as_ref()
        .map(|archive| archive.sources.len())
        .unwrap_or_default();
    let source_program_archive_edge_count = catalog
        .source_program_archive
        .as_ref()
        .map(|archive| archive.edges.len())
        .unwrap_or_default();
    Ok(SetupDirectorySummaryReport {
        unit_count: usize::try_from(manifest.unit_count)
            .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?,
        global_constraint_count: usize::try_from(manifest.global_constraint_count)
            .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?,
        fixed_bytes: manifest.fixed_byte_count,
        pcs_material_unit_count: usize::try_from(manifest.pcs_material_unit_count)
            .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?,
        pcs_material_bytes: manifest.pcs_material_byte_count,
        source_fixed_file_manifest_present,
        source_fixed_file_manifest_entry_count,
        source_program_archive_present,
        source_program_archive_source_count,
        source_program_archive_edge_count,
        fingerprint: encode_digest_hex(&manifest.catalog_digest),
    })
}

pub fn write_setup_directory_manifest(
    root: impl AsRef<Path>,
) -> Result<SetupDirectoryManifestWriteReport, SetupDirectorySummaryError> {
    write_setup_directory_manifest_for_root(root.as_ref())
}

pub(crate) fn write_setup_directory_manifest_for_layout(
    layout: &KeyDirectoryLayout,
) -> Result<SetupDirectoryManifestWriteReport, SetupDirectorySummaryError> {
    let catalog = read_key_directory_catalog_from_layout(layout)?;
    write_setup_directory_manifest_value(&layout.root, &build_setup_directory_manifest(&catalog)?)
}

fn write_setup_directory_manifest_for_root(
    root: &Path,
) -> Result<SetupDirectoryManifestWriteReport, SetupDirectorySummaryError> {
    let catalog = read_key_directory_catalog(root)?;
    let manifest = build_setup_directory_manifest(&catalog)?;
    write_setup_directory_manifest_value(root, &manifest)
}

fn write_setup_directory_manifest_value(
    root: &Path,
    manifest: &SetupDirectoryManifest,
) -> Result<SetupDirectoryManifestWriteReport, SetupDirectorySummaryError> {
    let bytes = encode_setup_directory_manifest(manifest)?;
    let path = root.join(SETUP_DIRECTORY_MANIFEST_FILE);
    let staging_path = write_staging_bytes(&path, &bytes, "write setup directory manifest")?;
    let staged = read_setup_directory_manifest_file(&staging_path)?;
    if staged != *manifest {
        return Err(SetupDirectorySummaryError::ManifestMismatch { path: staging_path });
    }
    let bytes_written =
        publish_staging_bytes(&staging_path, &path, "publish setup directory manifest")?;
    Ok(SetupDirectoryManifestWriteReport {
        path,
        bytes_written,
        fingerprint: encode_digest_hex(&manifest.catalog_digest),
    })
}

fn validate_manifest_file_if_present(
    root: &Path,
    expected: &SetupDirectoryManifest,
) -> Result<(), SetupDirectorySummaryError> {
    let path = root.join(SETUP_DIRECTORY_MANIFEST_FILE);
    validate_setup_directory_manifest_file(&path, expected)
        .map_err(SetupDirectorySummaryError::from)
}

fn encode_digest_hex(value: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in value {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

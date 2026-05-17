use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use lzvm_artifacts::setup_manifest::SETUP_DIRECTORY_MANIFEST_FILE;
use lzvm_artifacts::source_fixed_file_manifest::{
    encode_source_fixed_file_manifest, SourceFixedFileManifestError,
    SOURCE_FIXED_FILE_MANIFEST_FILE,
};
use lzvm_artifacts::source_program::{
    encode_source_program_archive, SourceProgramArchiveError, SOURCE_PROGRAM_ARCHIVE_FILE,
};
use lzvm_pil::{
    build_source_program_archive, ParseError, SourceLoaderConfig, SourceProgramArchiveBuildError,
    SourceProgramError, SourceProgramLoader,
};

use crate::{
    publish_staging_bytes, source_fixed_file_manifest_from_resolved,
    write_setup_directory_manifest, write_staging_bytes, SetupDirectorySummaryError, SetupError,
    SourceFixedFileManifestWriteError, SourceFixedFileManifestWriteReport,
    SourceProgramArchiveWriteReport,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCompanionWriteRequest {
    pub working_dir: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub include_path_first: bool,
    pub main_file: PathBuf,
    pub setup_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCompanionWriteReport {
    pub setup_dir: PathBuf,
    pub source_program_archive: SourceProgramArchiveWriteReport,
    pub source_fixed_file_manifest: SourceFixedFileManifestWriteReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCompanionWriteError {
    SourceProgram(SourceProgramError),
    ArchiveBuild(SourceProgramArchiveBuildError),
    ArchiveEncode(SourceProgramArchiveError),
    FixedResolve(ParseError),
    FixedManifest(SourceFixedFileManifestWriteError),
    FixedManifestEncode(SourceFixedFileManifestError),
    Manifest(SetupDirectorySummaryError),
    Setup(SetupError),
}

impl fmt::Display for SourceCompanionWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceProgram(error) => write!(f, "{error}"),
            Self::ArchiveBuild(error) => write!(f, "{error}"),
            Self::ArchiveEncode(error) => write!(f, "{error}"),
            Self::FixedResolve(error) => write!(f, "{error}"),
            Self::FixedManifest(error) => write!(f, "{error}"),
            Self::FixedManifestEncode(error) => write!(f, "{error}"),
            Self::Manifest(error) => write!(f, "{error}"),
            Self::Setup(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SourceCompanionWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SourceProgram(error) => Some(error),
            Self::ArchiveBuild(error) => Some(error),
            Self::ArchiveEncode(error) => Some(error),
            Self::FixedResolve(error) => Some(error),
            Self::FixedManifest(error) => Some(error),
            Self::FixedManifestEncode(error) => Some(error),
            Self::Manifest(error) => Some(error),
            Self::Setup(error) => Some(error),
        }
    }
}

pub fn write_source_companions(
    request: &SourceCompanionWriteRequest,
) -> Result<SourceCompanionWriteReport, SourceCompanionWriteError> {
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: request.working_dir.clone(),
        include_paths: request.include_paths.clone(),
        include_path_first: request.include_path_first,
    });
    let program = loader
        .load_main(&request.main_file)
        .map_err(SourceCompanionWriteError::SourceProgram)?;
    let module_count = program.modules.len();
    let fixed_file_pragma_count = program
        .modules
        .iter()
        .map(|module| module.fixed_file_pragmas.len())
        .sum();
    let air_template_fixed_file_pragma_count = program
        .modules
        .iter()
        .map(|module| module.air_template_fixed_file_pragmas.len())
        .sum();
    let air_unit_count = program.air_units().len();

    let archive =
        build_source_program_archive(&program).map_err(SourceCompanionWriteError::ArchiveBuild)?;
    let archive_bytes = encode_source_program_archive(&archive)
        .map_err(SourceCompanionWriteError::ArchiveEncode)?;
    let resolved = program
        .resolved_fixed_file_pragmas()
        .map_err(SourceCompanionWriteError::FixedResolve)?;
    let manifest = source_fixed_file_manifest_from_resolved(&resolved)
        .map_err(SourceCompanionWriteError::FixedManifest)?;
    let manifest_bytes = encode_source_fixed_file_manifest(&manifest)
        .map_err(SourceCompanionWriteError::FixedManifestEncode)?;

    let archive_output_path = request.setup_dir.join(SOURCE_PROGRAM_ARCHIVE_FILE);
    let manifest_output_path = request.setup_dir.join(SOURCE_FIXED_FILE_MANIFEST_FILE);
    let refresh_manifest = request
        .setup_dir
        .join(SETUP_DIRECTORY_MANIFEST_FILE)
        .is_file();
    let archive_snapshot = if refresh_manifest {
        read_optional_file(&archive_output_path)?
    } else {
        None
    };
    let manifest_snapshot = if refresh_manifest {
        read_optional_file(&manifest_output_path)?
    } else {
        None
    };
    let archive_staging_path = write_staging_bytes(
        &archive_output_path,
        &archive_bytes,
        "write source program archive staging",
    )
    .map_err(SourceCompanionWriteError::Setup)?;
    let manifest_staging_path = write_staging_bytes(
        &manifest_output_path,
        &manifest_bytes,
        "write source fixed-file manifest staging",
    )
    .map_err(SourceCompanionWriteError::Setup)?;
    let archive_bytes_written = publish_staging_bytes(
        &archive_staging_path,
        &archive_output_path,
        "publish source program archive",
    )
    .map_err(SourceCompanionWriteError::Setup)?;
    let manifest_bytes_written = publish_staging_bytes(
        &manifest_staging_path,
        &manifest_output_path,
        "publish source fixed-file manifest",
    )
    .map_err(SourceCompanionWriteError::Setup)?;
    if refresh_manifest {
        if let Err(error) = write_setup_directory_manifest(&request.setup_dir) {
            restore_optional_file(&archive_output_path, archive_snapshot.as_deref())?;
            restore_optional_file(&manifest_output_path, manifest_snapshot.as_deref())?;
            return Err(SourceCompanionWriteError::Manifest(error));
        }
    }

    Ok(SourceCompanionWriteReport {
        setup_dir: request.setup_dir.clone(),
        source_program_archive: SourceProgramArchiveWriteReport {
            output_path: archive_output_path,
            bytes_written: archive_bytes_written,
            source_count: archive.sources.len(),
            edge_count: archive.edges.len(),
            module_count,
            fixed_file_pragma_count,
            air_template_fixed_file_pragma_count,
            air_unit_count,
        },
        source_fixed_file_manifest: SourceFixedFileManifestWriteReport {
            output_path: manifest_output_path,
            bytes_written: manifest_bytes_written,
            module_count,
            fixed_file_pragma_count,
            air_template_fixed_file_pragma_count,
            air_unit_count,
            entry_count: manifest.entries.len(),
        },
    })
}

fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>, SourceCompanionWriteError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(SourceCompanionWriteError::Setup(SetupError::Io {
            role: "read existing source companion",
            path: path.to_path_buf(),
            message: error.to_string(),
        })),
    }
}

fn restore_optional_file(
    path: &Path,
    bytes: Option<&[u8]>,
) -> Result<(), SourceCompanionWriteError> {
    match bytes {
        Some(bytes) => std::fs::write(path, bytes).map_err(|error| {
            SourceCompanionWriteError::Setup(SetupError::Io {
                role: "restore source companion",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
        }),
        None => match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(SourceCompanionWriteError::Setup(SetupError::Io {
                role: "remove rejected source companion",
                path: path.to_path_buf(),
                message: error.to_string(),
            })),
        },
    }
}

use std::fmt;
use std::path::PathBuf;

use lzvm_artifacts::source_program::{encode_source_program_archive, SourceProgramArchiveError};
use lzvm_pil::{
    build_source_program_archive, SourceLoaderConfig, SourceProgramArchiveBuildError,
    SourceProgramError, SourceProgramLoader,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramArchiveWriteRequest {
    pub working_dir: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub include_path_first: bool,
    pub main_file: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramArchiveWriteReport {
    pub output_path: PathBuf,
    pub bytes_written: u64,
    pub source_count: usize,
    pub edge_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceProgramArchiveWriteError {
    SourceProgram(SourceProgramError),
    ArchiveBuild(SourceProgramArchiveBuildError),
    ArchiveEncode(SourceProgramArchiveError),
    Io {
        path: PathBuf,
        role: &'static str,
        message: String,
    },
}

impl fmt::Display for SourceProgramArchiveWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceProgram(error) => write!(f, "{error}"),
            Self::ArchiveBuild(error) => write!(f, "{error}"),
            Self::ArchiveEncode(error) => write!(f, "{error}"),
            Self::Io {
                path,
                role,
                message,
            } => {
                write!(
                    f,
                    "source program archive {role} io error at {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for SourceProgramArchiveWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SourceProgram(error) => Some(error),
            Self::ArchiveBuild(error) => Some(error),
            Self::ArchiveEncode(error) => Some(error),
            Self::Io { .. } => None,
        }
    }
}

pub fn write_source_program_archive(
    request: &SourceProgramArchiveWriteRequest,
) -> Result<SourceProgramArchiveWriteReport, SourceProgramArchiveWriteError> {
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: request.working_dir.clone(),
        include_paths: request.include_paths.clone(),
        include_path_first: request.include_path_first,
    });
    let program = loader
        .load_main(&request.main_file)
        .map_err(SourceProgramArchiveWriteError::SourceProgram)?;
    let archive = build_source_program_archive(&program)
        .map_err(SourceProgramArchiveWriteError::ArchiveBuild)?;
    let bytes = encode_source_program_archive(&archive)
        .map_err(SourceProgramArchiveWriteError::ArchiveEncode)?;
    write_output(&request.output_path, &bytes)?;
    let bytes_written = std::fs::metadata(&request.output_path)
        .map(|meta| meta.len())
        .map_err(|error| SourceProgramArchiveWriteError::Io {
            path: request.output_path.clone(),
            role: "read output metadata",
            message: error.to_string(),
        })?;

    Ok(SourceProgramArchiveWriteReport {
        output_path: request.output_path.clone(),
        bytes_written,
        source_count: archive.sources.len(),
        edge_count: archive.edges.len(),
    })
}

fn write_output(path: &PathBuf, bytes: &[u8]) -> Result<(), SourceProgramArchiveWriteError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| SourceProgramArchiveWriteError::Io {
            path: parent.to_path_buf(),
            role: "create output directory",
            message: error.to_string(),
        })?;
    }
    std::fs::write(path, bytes).map_err(|error| SourceProgramArchiveWriteError::Io {
        path: path.to_path_buf(),
        role: "write output",
        message: error.to_string(),
    })
}

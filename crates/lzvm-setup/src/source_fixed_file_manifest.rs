use std::fmt;
use std::path::PathBuf;

use lzvm_artifacts::source_fixed_file_manifest::{
    encode_source_fixed_file_manifest, SourceFixedFileManifest, SourceFixedFileManifestEntry,
    SourceFixedFileManifestError, SourceFixedFileManifestKind,
};
use lzvm_pil::{
    FixedFilePragmaKind, ParseError, SourceLoaderConfig, SourceProgramError, SourceProgramLoader,
    SourceProgramResolvedFixedFilePragma,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFixedFileManifestWriteRequest {
    pub working_dir: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub include_path_first: bool,
    pub main_file: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFixedFileManifestWriteReport {
    pub output_path: PathBuf,
    pub bytes_written: u64,
    pub module_count: usize,
    pub fixed_file_pragma_count: usize,
    pub air_template_fixed_file_pragma_count: usize,
    pub air_unit_count: usize,
    pub entry_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFixedFileManifestWriteError {
    SourceProgram(SourceProgramError),
    Resolve(ParseError),
    InvalidResolvedId {
        source_name: String,
        field: &'static str,
        value: i128,
    },
    LengthOverflow,
    ManifestEncode(SourceFixedFileManifestError),
    Io {
        path: PathBuf,
        role: &'static str,
        message: String,
    },
}

impl fmt::Display for SourceFixedFileManifestWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceProgram(error) => write!(f, "{error}"),
            Self::Resolve(error) => write!(f, "{error}"),
            Self::InvalidResolvedId {
                source_name,
                field,
                value,
            } => write!(
                f,
                "source fixed-file manifest has invalid resolved {field} {value} in {source_name}"
            ),
            Self::LengthOverflow => write!(f, "source fixed-file manifest length overflow"),
            Self::ManifestEncode(error) => write!(f, "{error}"),
            Self::Io {
                path,
                role,
                message,
            } => write!(
                f,
                "source fixed-file manifest {role} io error at {}: {message}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SourceFixedFileManifestWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SourceProgram(error) => Some(error),
            Self::Resolve(error) => Some(error),
            Self::ManifestEncode(error) => Some(error),
            Self::InvalidResolvedId { .. } | Self::LengthOverflow | Self::Io { .. } => None,
        }
    }
}

pub fn write_source_fixed_file_manifest(
    request: &SourceFixedFileManifestWriteRequest,
) -> Result<SourceFixedFileManifestWriteReport, SourceFixedFileManifestWriteError> {
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: request.working_dir.clone(),
        include_paths: request.include_paths.clone(),
        include_path_first: request.include_path_first,
    });
    let program = loader
        .load_main(&request.main_file)
        .map_err(SourceFixedFileManifestWriteError::SourceProgram)?;
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
    let resolved = program
        .resolved_fixed_file_pragmas()
        .map_err(SourceFixedFileManifestWriteError::Resolve)?;
    let manifest = source_fixed_file_manifest_from_resolved(&resolved)?;
    let bytes = encode_source_fixed_file_manifest(&manifest)
        .map_err(SourceFixedFileManifestWriteError::ManifestEncode)?;
    write_output(&request.output_path, &bytes)?;
    let bytes_written = std::fs::metadata(&request.output_path)
        .map(|meta| meta.len())
        .map_err(|error| SourceFixedFileManifestWriteError::Io {
            path: request.output_path.clone(),
            role: "read output metadata",
            message: error.to_string(),
        })?;

    Ok(SourceFixedFileManifestWriteReport {
        output_path: request.output_path.clone(),
        bytes_written,
        module_count,
        fixed_file_pragma_count,
        air_template_fixed_file_pragma_count,
        air_unit_count,
        entry_count: manifest.entries.len(),
    })
}

pub fn source_fixed_file_manifest_from_resolved(
    resolved: &[SourceProgramResolvedFixedFilePragma],
) -> Result<SourceFixedFileManifest, SourceFixedFileManifestWriteError> {
    let entries = resolved
        .iter()
        .map(source_fixed_file_manifest_entry_from_resolved)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SourceFixedFileManifest { entries })
}

fn source_fixed_file_manifest_entry_from_resolved(
    resolved: &SourceProgramResolvedFixedFilePragma,
) -> Result<SourceFixedFileManifestEntry, SourceFixedFileManifestWriteError> {
    Ok(SourceFixedFileManifestEntry {
        source_name: resolved.source_name.clone(),
        kind: source_fixed_file_manifest_kind(resolved.kind),
        path: resolved.path.clone(),
        column: resolved.column,
        group_name: resolved.group_name.clone(),
        group_id: non_negative_id(&resolved.source_name, "group_id", resolved.group_id)?,
        unit_id: non_negative_id(&resolved.source_name, "unit_id", resolved.unit_id)?,
        unit_name: resolved.unit_name.clone(),
        template_name: resolved.template_name.clone(),
        virtual_instance: resolved.virtual_instance,
        start: u64::try_from(resolved.start)
            .map_err(|_| SourceFixedFileManifestWriteError::LengthOverflow)?,
        end: u64::try_from(resolved.end)
            .map_err(|_| SourceFixedFileManifestWriteError::LengthOverflow)?,
    })
}

fn source_fixed_file_manifest_kind(kind: FixedFilePragmaKind) -> SourceFixedFileManifestKind {
    match kind {
        FixedFilePragmaKind::FixedExternal => SourceFixedFileManifestKind::FixedExternal,
        FixedFilePragmaKind::ExternFixedFile => SourceFixedFileManifestKind::ExternFixedFile,
        FixedFilePragmaKind::FixedLoad => SourceFixedFileManifestKind::FixedLoad,
        FixedFilePragmaKind::OutputFixedFile => SourceFixedFileManifestKind::OutputFixedFile,
    }
}

fn non_negative_id(
    source_name: &str,
    field: &'static str,
    value: i128,
) -> Result<u64, SourceFixedFileManifestWriteError> {
    u64::try_from(value).map_err(|_| SourceFixedFileManifestWriteError::InvalidResolvedId {
        source_name: source_name.to_owned(),
        field,
        value,
    })
}

fn write_output(path: &PathBuf, bytes: &[u8]) -> Result<(), SourceFixedFileManifestWriteError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| SourceFixedFileManifestWriteError::Io {
            path: parent.to_path_buf(),
            role: "create output directory",
            message: error.to_string(),
        })?;
    }
    std::fs::write(path, bytes).map_err(|error| SourceFixedFileManifestWriteError::Io {
        path: path.to_path_buf(),
        role: "write output",
        message: error.to_string(),
    })
}

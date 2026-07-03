use std::fmt;
use std::path::{Path, PathBuf};

use crate::key_directory::{key_directory_catalog_digest, KeyDirectoryCatalog, KeyDirectoryError};
use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

pub const SETUP_DIRECTORY_MANIFEST_FILE: &str = "lzvm.setup-manifest";

const SETUP_DIRECTORY_MANIFEST_KIND: [u8; 4] = *b"sdmf";
const SETUP_DIRECTORY_MANIFEST_VERSION: u32 = 4;
const SETUP_DIRECTORY_MANIFEST_SECTION_ID: u32 = 1;
const DIGEST_BYTES: usize = 32;
const PAYLOAD_V1_BYTES: usize = 5 * 8 + DIGEST_BYTES;
const PAYLOAD_V2_BYTES: usize = PAYLOAD_V1_BYTES + 2 * 8;
const PAYLOAD_V3_BYTES: usize = PAYLOAD_V2_BYTES + 3 * 8;
const PAYLOAD_V4_BYTES: usize = PAYLOAD_V3_BYTES + 2 * 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupDirectoryManifest {
    pub unit_count: u64,
    pub global_constraint_count: u64,
    pub fixed_byte_count: u64,
    pub pcs_material_unit_count: u64,
    pub pcs_material_byte_count: u64,
    pub source_fixed_file_manifest_present: bool,
    pub source_fixed_file_manifest_entry_count: u64,
    pub source_fixed_file_manifest_byte_count: u64,
    pub source_program_archive_present: bool,
    pub source_program_archive_source_count: u64,
    pub source_program_archive_edge_count: u64,
    pub source_program_archive_byte_count: u64,
    pub catalog_digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupDirectoryManifestError {
    Sectioned(SectionedError),
    Catalog(KeyDirectoryError),
    UnsupportedVersion {
        found: u32,
        max: u32,
    },
    InvalidSectionCount {
        found: u32,
    },
    InvalidSectionId {
        found: u32,
    },
    InvalidPayloadLength {
        expected: usize,
        found: usize,
    },
    EmptyUnits,
    InvalidMaterialUnitCount {
        unit_count: u64,
        pcs_material_unit_count: u64,
    },
    InvalidSourceFixedFileManifestCounts {
        present: bool,
        entry_count: u64,
        byte_count: u64,
    },
    InvalidSourceProgramArchiveCounts {
        present: bool,
        source_count: u64,
        edge_count: u64,
        byte_count: u64,
    },
    Mismatch {
        path: PathBuf,
    },
    LengthOverflow,
    Io {
        message: String,
    },
}

impl fmt::Display for SetupDirectoryManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sectioned(error) => write!(f, "setup directory manifest container error: {error}"),
            Self::Catalog(error) => write!(f, "{error}"),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported setup directory manifest version {found}, max {max}")
            }
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid setup directory manifest section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid setup directory manifest section id {found}")
            }
            Self::InvalidPayloadLength { expected, found } => write!(
                f,
                "invalid setup directory manifest payload length: expected {expected}, found {found}"
            ),
            Self::EmptyUnits => write!(f, "setup directory manifest has no units"),
            Self::InvalidMaterialUnitCount {
                unit_count,
                pcs_material_unit_count,
            } => write!(
                f,
                "setup directory manifest material unit count {pcs_material_unit_count} exceeds unit count {unit_count}"
            ),
            Self::InvalidSourceFixedFileManifestCounts {
                present,
                entry_count,
                byte_count,
            } => write!(
                f,
                "setup directory manifest source fixed-file manifest present={present} has entry count {entry_count} and byte count {byte_count}"
            ),
            Self::InvalidSourceProgramArchiveCounts {
                present,
                source_count,
                edge_count,
                byte_count,
            } => write!(
                f,
                "setup directory manifest source program archive present={present} has source count {source_count}, edge count {edge_count}, and byte count {byte_count}"
            ),
            Self::Mismatch { path } => {
                write!(f, "setup directory manifest mismatch at {}", path.display())
            }
            Self::LengthOverflow => write!(f, "setup directory manifest length overflow"),
            Self::Io { message } => write!(f, "setup directory manifest io error: {message}"),
        }
    }
}

impl std::error::Error for SetupDirectoryManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sectioned(error) => Some(error),
            Self::Catalog(error) => Some(error),
            Self::UnsupportedVersion { .. }
            | Self::InvalidSectionCount { .. }
            | Self::InvalidSectionId { .. }
            | Self::InvalidPayloadLength { .. }
            | Self::EmptyUnits
            | Self::InvalidMaterialUnitCount { .. }
            | Self::InvalidSourceFixedFileManifestCounts { .. }
            | Self::InvalidSourceProgramArchiveCounts { .. }
            | Self::Mismatch { .. }
            | Self::LengthOverflow
            | Self::Io { .. } => None,
        }
    }
}

impl From<SectionedError> for SetupDirectoryManifestError {
    fn from(error: SectionedError) -> Self {
        Self::Sectioned(error)
    }
}

impl From<KeyDirectoryError> for SetupDirectoryManifestError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::Catalog(error)
    }
}

pub fn build_setup_directory_manifest(
    catalog: &KeyDirectoryCatalog,
) -> Result<SetupDirectoryManifest, SetupDirectoryManifestError> {
    let unit_count = u64::try_from(catalog.units.len())
        .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?;
    let global_constraint_count = u64::try_from(catalog.global_constraints.entries.len())
        .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?;
    let fixed_byte_count = catalog.units.iter().try_fold(0_u64, |total, unit| {
        total
            .checked_add(unit.actual_fixed_bytes)
            .ok_or(SetupDirectoryManifestError::LengthOverflow)
    })?;
    let pcs_material_unit_count = u64::try_from(
        catalog
            .units
            .iter()
            .filter(|unit| unit.pcs_material_present)
            .count(),
    )
    .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?;
    let pcs_material_byte_count = catalog
        .units
        .iter()
        .filter_map(|unit| unit.pcs_material_bytes)
        .try_fold(0_u64, |total, bytes| {
            total
                .checked_add(bytes)
                .ok_or(SetupDirectoryManifestError::LengthOverflow)
        })?;
    let source_fixed_file_manifest_present = catalog.source_fixed_file_manifest.is_some();
    let source_fixed_file_manifest_entry_count = catalog
        .source_fixed_file_manifest
        .as_ref()
        .map(|manifest| u64::try_from(manifest.entries.len()))
        .transpose()
        .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?
        .unwrap_or_default();
    let source_fixed_file_manifest_byte_count = file_len_if_present(
        source_fixed_file_manifest_present,
        &catalog.layout.source_fixed_file_manifest,
    )?;
    let source_program_archive_present = catalog.source_program_archive.is_some();
    let source_program_archive_source_count = catalog
        .source_program_archive
        .as_ref()
        .map(|archive| u64::try_from(archive.sources.len()))
        .transpose()
        .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?
        .unwrap_or_default();
    let source_program_archive_edge_count = catalog
        .source_program_archive
        .as_ref()
        .map(|archive| u64::try_from(archive.edges.len()))
        .transpose()
        .map_err(|_| SetupDirectoryManifestError::LengthOverflow)?
        .unwrap_or_default();
    let source_program_archive_byte_count = file_len_if_present(
        source_program_archive_present,
        &catalog.layout.source_program_archive,
    )?;

    let out = SetupDirectoryManifest {
        unit_count,
        global_constraint_count,
        fixed_byte_count,
        pcs_material_unit_count,
        pcs_material_byte_count,
        source_fixed_file_manifest_present,
        source_fixed_file_manifest_entry_count,
        source_fixed_file_manifest_byte_count,
        source_program_archive_present,
        source_program_archive_source_count,
        source_program_archive_edge_count,
        source_program_archive_byte_count,
        catalog_digest: key_directory_catalog_digest(catalog)?,
    };
    validate_setup_directory_manifest(&out)?;
    Ok(out)
}

fn file_len_if_present(present: bool, path: &Path) -> Result<u64, SetupDirectoryManifestError> {
    if !present {
        return Ok(0);
    }
    Ok(std::fs::metadata(path)
        .map_err(|error| SetupDirectoryManifestError::Io {
            message: format!("{}: {error}", path.display()),
        })?
        .len())
}

pub fn read_setup_directory_manifest_file(
    path: impl AsRef<Path>,
) -> Result<SetupDirectoryManifest, SetupDirectoryManifestError> {
    let bytes = std::fs::read(path).map_err(|error| SetupDirectoryManifestError::Io {
        message: error.to_string(),
    })?;
    parse_setup_directory_manifest(&bytes)
}

pub fn validate_setup_directory_manifest_file(
    path: impl AsRef<Path>,
    expected: &SetupDirectoryManifest,
) -> Result<(), SetupDirectoryManifestError> {
    let path = path.as_ref();
    if !path
        .try_exists()
        .map_err(|error| SetupDirectoryManifestError::Io {
            message: format!("{}: {error}", path.display()),
        })?
    {
        return Ok(());
    }
    validate_required_setup_directory_manifest_file(path, expected)
}

pub fn validate_required_setup_directory_manifest_file(
    path: impl AsRef<Path>,
    expected: &SetupDirectoryManifest,
) -> Result<(), SetupDirectoryManifestError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|error| SetupDirectoryManifestError::Io {
        message: format!("{}: {error}", path.display()),
    })?;
    let (version, found) = parse_setup_directory_manifest_with_version(&bytes)?;
    if !setup_directory_manifest_matches_version(&found, expected, version) {
        return Err(SetupDirectoryManifestError::Mismatch {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

pub fn parse_setup_directory_manifest(
    bytes: &[u8],
) -> Result<SetupDirectoryManifest, SetupDirectoryManifestError> {
    parse_setup_directory_manifest_with_version(bytes).map(|(_, manifest)| manifest)
}

fn parse_setup_directory_manifest_with_version(
    bytes: &[u8],
) -> Result<(u32, SetupDirectoryManifest), SetupDirectoryManifestError> {
    let file = parse_sectioned_file(
        bytes,
        SETUP_DIRECTORY_MANIFEST_KIND,
        SETUP_DIRECTORY_MANIFEST_VERSION,
    )?;
    if file.version == 0 {
        return Err(SetupDirectoryManifestError::UnsupportedVersion {
            found: file.version,
            max: SETUP_DIRECTORY_MANIFEST_VERSION,
        });
    }

    if file.sections.len() != 1 {
        return Err(SetupDirectoryManifestError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }
    let section = &file.sections[0];
    if section.id != SETUP_DIRECTORY_MANIFEST_SECTION_ID {
        return Err(SetupDirectoryManifestError::InvalidSectionId { found: section.id });
    }
    let out = parse_setup_directory_manifest_payload(file.version, &section.data)?;
    validate_setup_directory_manifest(&out)?;
    Ok((file.version, out))
}

pub fn encode_setup_directory_manifest(
    value: &SetupDirectoryManifest,
) -> Result<Vec<u8>, SetupDirectoryManifestError> {
    validate_setup_directory_manifest(value)?;
    let file = SectionedFile {
        kind: SETUP_DIRECTORY_MANIFEST_KIND,
        version: SETUP_DIRECTORY_MANIFEST_VERSION,
        sections: vec![SectionedSection {
            id: SETUP_DIRECTORY_MANIFEST_SECTION_ID,
            data: encode_setup_directory_manifest_payload(value),
        }],
    };
    encode_sectioned_file(&file).map_err(SetupDirectoryManifestError::Sectioned)
}

fn validate_setup_directory_manifest(
    value: &SetupDirectoryManifest,
) -> Result<(), SetupDirectoryManifestError> {
    if value.unit_count == 0 {
        return Err(SetupDirectoryManifestError::EmptyUnits);
    }
    if value.pcs_material_unit_count > value.unit_count {
        return Err(SetupDirectoryManifestError::InvalidMaterialUnitCount {
            unit_count: value.unit_count,
            pcs_material_unit_count: value.pcs_material_unit_count,
        });
    }
    if !value.source_fixed_file_manifest_present
        && (value.source_fixed_file_manifest_entry_count != 0
            || value.source_fixed_file_manifest_byte_count != 0)
    {
        return Err(
            SetupDirectoryManifestError::InvalidSourceFixedFileManifestCounts {
                present: value.source_fixed_file_manifest_present,
                entry_count: value.source_fixed_file_manifest_entry_count,
                byte_count: value.source_fixed_file_manifest_byte_count,
            },
        );
    }
    if (!value.source_program_archive_present
        && (value.source_program_archive_source_count != 0
            || value.source_program_archive_edge_count != 0
            || value.source_program_archive_byte_count != 0))
        || (value.source_program_archive_present && value.source_program_archive_source_count == 0)
    {
        return Err(
            SetupDirectoryManifestError::InvalidSourceProgramArchiveCounts {
                present: value.source_program_archive_present,
                source_count: value.source_program_archive_source_count,
                edge_count: value.source_program_archive_edge_count,
                byte_count: value.source_program_archive_byte_count,
            },
        );
    }
    Ok(())
}

fn setup_directory_manifest_matches_version(
    found: &SetupDirectoryManifest,
    expected: &SetupDirectoryManifest,
    version: u32,
) -> bool {
    found.unit_count == expected.unit_count
        && found.global_constraint_count == expected.global_constraint_count
        && found.fixed_byte_count == expected.fixed_byte_count
        && found.pcs_material_unit_count == expected.pcs_material_unit_count
        && found.pcs_material_byte_count == expected.pcs_material_byte_count
        && (version < 2
            || (found.source_fixed_file_manifest_present
                == expected.source_fixed_file_manifest_present
                && found.source_fixed_file_manifest_entry_count
                    == expected.source_fixed_file_manifest_entry_count))
        && (version < 3
            || (found.source_program_archive_present == expected.source_program_archive_present
                && found.source_program_archive_source_count
                    == expected.source_program_archive_source_count
                && found.source_program_archive_edge_count
                    == expected.source_program_archive_edge_count))
        && (version < 4
            || (found.source_fixed_file_manifest_byte_count
                == expected.source_fixed_file_manifest_byte_count
                && found.source_program_archive_byte_count
                    == expected.source_program_archive_byte_count))
        && found.catalog_digest == expected.catalog_digest
}

fn parse_setup_directory_manifest_payload(
    version: u32,
    bytes: &[u8],
) -> Result<SetupDirectoryManifest, SetupDirectoryManifestError> {
    let expected = match version {
        1 => PAYLOAD_V1_BYTES,
        2 => PAYLOAD_V2_BYTES,
        3 => PAYLOAD_V3_BYTES,
        _ => PAYLOAD_V4_BYTES,
    };
    if bytes.len() != expected {
        return Err(SetupDirectoryManifestError::InvalidPayloadLength {
            expected,
            found: bytes.len(),
        });
    }

    let mut offset = 0;
    Ok(SetupDirectoryManifest {
        unit_count: read_u64(bytes, &mut offset),
        global_constraint_count: read_u64(bytes, &mut offset),
        fixed_byte_count: read_u64(bytes, &mut offset),
        pcs_material_unit_count: read_u64(bytes, &mut offset),
        pcs_material_byte_count: read_u64(bytes, &mut offset),
        source_fixed_file_manifest_present: if version >= 2 {
            read_u64(bytes, &mut offset) != 0
        } else {
            false
        },
        source_fixed_file_manifest_entry_count: if version >= 2 {
            read_u64(bytes, &mut offset)
        } else {
            0
        },
        source_fixed_file_manifest_byte_count: if version >= 4 {
            read_u64(bytes, &mut offset)
        } else {
            0
        },
        source_program_archive_present: if version >= 3 {
            read_u64(bytes, &mut offset) != 0
        } else {
            false
        },
        source_program_archive_source_count: if version >= 3 {
            read_u64(bytes, &mut offset)
        } else {
            0
        },
        source_program_archive_edge_count: if version >= 3 {
            read_u64(bytes, &mut offset)
        } else {
            0
        },
        source_program_archive_byte_count: if version >= 4 {
            read_u64(bytes, &mut offset)
        } else {
            0
        },
        catalog_digest: read_digest(bytes, &mut offset),
    })
}

fn encode_setup_directory_manifest_payload(value: &SetupDirectoryManifest) -> Vec<u8> {
    let mut out = Vec::with_capacity(PAYLOAD_V4_BYTES);
    out.extend_from_slice(&value.unit_count.to_le_bytes());
    out.extend_from_slice(&value.global_constraint_count.to_le_bytes());
    out.extend_from_slice(&value.fixed_byte_count.to_le_bytes());
    out.extend_from_slice(&value.pcs_material_unit_count.to_le_bytes());
    out.extend_from_slice(&value.pcs_material_byte_count.to_le_bytes());
    out.extend_from_slice(&u64::from(value.source_fixed_file_manifest_present).to_le_bytes());
    out.extend_from_slice(&value.source_fixed_file_manifest_entry_count.to_le_bytes());
    out.extend_from_slice(&value.source_fixed_file_manifest_byte_count.to_le_bytes());
    out.extend_from_slice(&u64::from(value.source_program_archive_present).to_le_bytes());
    out.extend_from_slice(&value.source_program_archive_source_count.to_le_bytes());
    out.extend_from_slice(&value.source_program_archive_edge_count.to_le_bytes());
    out.extend_from_slice(&value.source_program_archive_byte_count.to_le_bytes());
    out.extend_from_slice(&value.catalog_digest);
    out
}

fn read_digest(bytes: &[u8], offset: &mut usize) -> [u8; 32] {
    let end = *offset + DIGEST_BYTES;
    let out = bytes[*offset..end]
        .try_into()
        .expect("payload length checked");
    *offset = end;
    out
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> u64 {
    let end = *offset + 8;
    let out = u64::from_le_bytes(
        bytes[*offset..end]
            .try_into()
            .expect("payload length checked"),
    );
    *offset = end;
    out
}

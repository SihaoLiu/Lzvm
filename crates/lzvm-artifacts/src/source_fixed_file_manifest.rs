use std::fmt;
use std::path::{Path, PathBuf};

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

pub const SOURCE_FIXED_FILE_MANIFEST_FILE: &str = "lzvm.source-fixed-file-manifest";

const SOURCE_FIXED_FILE_MANIFEST_KIND: [u8; 4] = *b"sffm";
const SOURCE_FIXED_FILE_MANIFEST_VERSION: u32 = 1;
const SOURCE_FIXED_FILE_MANIFEST_SECTION_ID: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFixedFileManifestKind {
    FixedExternal = 0,
    ExternFixedFile = 1,
    FixedLoad = 2,
    OutputFixedFile = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFixedFileManifestEntry {
    pub source_name: String,
    pub kind: SourceFixedFileManifestKind,
    pub path: Option<String>,
    pub column: Option<u32>,
    pub group_name: String,
    pub group_id: u64,
    pub unit_id: u64,
    pub unit_name: String,
    pub template_name: String,
    pub virtual_instance: bool,
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFixedFileManifest {
    pub entries: Vec<SourceFixedFileManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFixedFileManifestError {
    Sectioned(SectionedError),
    UnsupportedVersion {
        found: u32,
        expected: u32,
    },
    InvalidSectionCount {
        found: u32,
    },
    InvalidSectionId {
        found: u32,
    },
    InvalidKind {
        found: u8,
    },
    InvalidBoolean {
        field: &'static str,
        found: u8,
    },
    EmptySourceName {
        entry_index: usize,
    },
    MissingPath {
        entry_index: usize,
    },
    UnexpectedPath {
        entry_index: usize,
    },
    EmptyPath {
        entry_index: usize,
    },
    MissingColumn {
        entry_index: usize,
    },
    UnexpectedColumn {
        entry_index: usize,
    },
    EmptyGroupName {
        entry_index: usize,
    },
    EmptyUnitName {
        entry_index: usize,
    },
    EmptyTemplateName {
        entry_index: usize,
    },
    InvalidSpan {
        entry_index: usize,
        start: u64,
        end: u64,
    },
    InvalidUtf8 {
        message: String,
    },
    UnexpectedPayloadTrailingBytes {
        count: usize,
    },
    UnexpectedPayloadEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    LengthOverflow,
    ReadFailed {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for SourceFixedFileManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sectioned(error) => write!(f, "source fixed file manifest container error: {error}"),
            Self::UnsupportedVersion { found, expected } => write!(
                f,
                "unsupported source fixed file manifest version {found}, expected {expected}"
            ),
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid source fixed file manifest section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid source fixed file manifest section id {found}")
            }
            Self::InvalidKind { found } => {
                write!(f, "invalid source fixed file manifest kind {found}")
            }
            Self::InvalidBoolean { field, found } => {
                write!(f, "invalid source fixed file manifest boolean {field}: {found}")
            }
            Self::EmptySourceName { entry_index } => {
                write!(f, "source fixed file manifest entry {entry_index} has no source name")
            }
            Self::MissingPath { entry_index } => {
                write!(f, "source fixed file manifest entry {entry_index} has no path")
            }
            Self::UnexpectedPath { entry_index } => write!(
                f,
                "source fixed file manifest entry {entry_index} has an unexpected path"
            ),
            Self::EmptyPath { entry_index } => {
                write!(f, "source fixed file manifest entry {entry_index} has an empty path")
            }
            Self::MissingColumn { entry_index } => {
                write!(f, "source fixed file manifest entry {entry_index} has no column")
            }
            Self::UnexpectedColumn { entry_index } => write!(
                f,
                "source fixed file manifest entry {entry_index} has an unexpected column"
            ),
            Self::EmptyGroupName { entry_index } => {
                write!(f, "source fixed file manifest entry {entry_index} has no group name")
            }
            Self::EmptyUnitName { entry_index } => {
                write!(f, "source fixed file manifest entry {entry_index} has no unit name")
            }
            Self::EmptyTemplateName { entry_index } => {
                write!(f, "source fixed file manifest entry {entry_index} has no template name")
            }
            Self::InvalidSpan {
                entry_index,
                start,
                end,
            } => write!(
                f,
                "source fixed file manifest entry {entry_index} has invalid span {start}..{end}"
            ),
            Self::InvalidUtf8 { message } => {
                write!(f, "invalid source fixed file manifest utf-8: {message}")
            }
            Self::UnexpectedPayloadTrailingBytes { count } => write!(
                f,
                "unexpected source fixed file manifest payload bytes: {count}"
            ),
            Self::UnexpectedPayloadEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected source fixed file manifest payload end at {offset}, needed {needed}, available {available}"
            ),
            Self::LengthOverflow => write!(f, "source fixed file manifest length overflow"),
            Self::ReadFailed { path, message } => write!(
                f,
                "failed to read source fixed file manifest {}: {message}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SourceFixedFileManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sectioned(error) => Some(error),
            Self::UnsupportedVersion { .. }
            | Self::InvalidSectionCount { .. }
            | Self::InvalidSectionId { .. }
            | Self::InvalidKind { .. }
            | Self::InvalidBoolean { .. }
            | Self::EmptySourceName { .. }
            | Self::MissingPath { .. }
            | Self::UnexpectedPath { .. }
            | Self::EmptyPath { .. }
            | Self::MissingColumn { .. }
            | Self::UnexpectedColumn { .. }
            | Self::EmptyGroupName { .. }
            | Self::EmptyUnitName { .. }
            | Self::EmptyTemplateName { .. }
            | Self::InvalidSpan { .. }
            | Self::InvalidUtf8 { .. }
            | Self::UnexpectedPayloadTrailingBytes { .. }
            | Self::UnexpectedPayloadEof { .. }
            | Self::LengthOverflow
            | Self::ReadFailed { .. } => None,
        }
    }
}

impl From<SectionedError> for SourceFixedFileManifestError {
    fn from(error: SectionedError) -> Self {
        Self::Sectioned(error)
    }
}

pub fn encode_source_fixed_file_manifest(
    value: &SourceFixedFileManifest,
) -> Result<Vec<u8>, SourceFixedFileManifestError> {
    validate_source_fixed_file_manifest(value)?;
    let file = SectionedFile {
        kind: SOURCE_FIXED_FILE_MANIFEST_KIND,
        version: SOURCE_FIXED_FILE_MANIFEST_VERSION,
        sections: vec![SectionedSection {
            id: SOURCE_FIXED_FILE_MANIFEST_SECTION_ID,
            data: encode_source_fixed_file_manifest_payload(value)?,
        }],
    };
    encode_sectioned_file(&file).map_err(SourceFixedFileManifestError::Sectioned)
}

pub fn parse_source_fixed_file_manifest(
    bytes: &[u8],
) -> Result<SourceFixedFileManifest, SourceFixedFileManifestError> {
    let file = parse_sectioned_file(
        bytes,
        SOURCE_FIXED_FILE_MANIFEST_KIND,
        SOURCE_FIXED_FILE_MANIFEST_VERSION,
    )?;
    if file.version != SOURCE_FIXED_FILE_MANIFEST_VERSION {
        return Err(SourceFixedFileManifestError::UnsupportedVersion {
            found: file.version,
            expected: SOURCE_FIXED_FILE_MANIFEST_VERSION,
        });
    }

    if file.sections.len() != 1 {
        return Err(SourceFixedFileManifestError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }
    let section = &file.sections[0];
    if section.id != SOURCE_FIXED_FILE_MANIFEST_SECTION_ID {
        return Err(SourceFixedFileManifestError::InvalidSectionId { found: section.id });
    }
    let manifest = parse_source_fixed_file_manifest_payload(&section.data)?;
    validate_source_fixed_file_manifest(&manifest)?;
    Ok(manifest)
}

pub fn read_source_fixed_file_manifest_file(
    path: impl AsRef<Path>,
) -> Result<SourceFixedFileManifest, SourceFixedFileManifestError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|error| SourceFixedFileManifestError::ReadFailed {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    parse_source_fixed_file_manifest(&bytes)
}

fn validate_source_fixed_file_manifest(
    value: &SourceFixedFileManifest,
) -> Result<(), SourceFixedFileManifestError> {
    for (entry_index, entry) in value.entries.iter().enumerate() {
        if entry.source_name.is_empty() {
            return Err(SourceFixedFileManifestError::EmptySourceName { entry_index });
        }
        match entry.kind {
            SourceFixedFileManifestKind::FixedExternal => {
                if entry.path.is_some() {
                    return Err(SourceFixedFileManifestError::UnexpectedPath { entry_index });
                }
            }
            SourceFixedFileManifestKind::ExternFixedFile
            | SourceFixedFileManifestKind::FixedLoad
            | SourceFixedFileManifestKind::OutputFixedFile => {
                if entry.path.is_none() {
                    return Err(SourceFixedFileManifestError::MissingPath { entry_index });
                }
            }
        }
        if entry.path.as_ref().is_some_and(|path| path.is_empty()) {
            return Err(SourceFixedFileManifestError::EmptyPath { entry_index });
        }
        match entry.kind {
            SourceFixedFileManifestKind::FixedLoad => {
                if entry.column.is_none() {
                    return Err(SourceFixedFileManifestError::MissingColumn { entry_index });
                }
            }
            SourceFixedFileManifestKind::FixedExternal
            | SourceFixedFileManifestKind::ExternFixedFile
            | SourceFixedFileManifestKind::OutputFixedFile => {
                if entry.column.is_some() {
                    return Err(SourceFixedFileManifestError::UnexpectedColumn { entry_index });
                }
            }
        }
        if entry.group_name.is_empty() {
            return Err(SourceFixedFileManifestError::EmptyGroupName { entry_index });
        }
        if entry.unit_name.is_empty() {
            return Err(SourceFixedFileManifestError::EmptyUnitName { entry_index });
        }
        if entry.template_name.is_empty() {
            return Err(SourceFixedFileManifestError::EmptyTemplateName { entry_index });
        }
        if entry.end < entry.start {
            return Err(SourceFixedFileManifestError::InvalidSpan {
                entry_index,
                start: entry.start,
                end: entry.end,
            });
        }
    }
    Ok(())
}

fn encode_source_fixed_file_manifest_payload(
    value: &SourceFixedFileManifest,
) -> Result<Vec<u8>, SourceFixedFileManifestError> {
    let mut out = Vec::new();
    write_u64(
        &mut out,
        u64::try_from(value.entries.len())
            .map_err(|_| SourceFixedFileManifestError::LengthOverflow)?,
    );
    for entry in &value.entries {
        write_string(&mut out, &entry.source_name)?;
        write_u8(&mut out, entry.kind as u8);
        write_option_string(&mut out, entry.path.as_deref())?;
        write_option_u32(&mut out, entry.column);
        write_string(&mut out, &entry.group_name)?;
        write_u64(&mut out, entry.group_id);
        write_u64(&mut out, entry.unit_id);
        write_string(&mut out, &entry.unit_name)?;
        write_string(&mut out, &entry.template_name)?;
        write_bool(&mut out, entry.virtual_instance);
        write_u64(&mut out, entry.start);
        write_u64(&mut out, entry.end);
    }
    Ok(out)
}

fn parse_source_fixed_file_manifest_payload(
    bytes: &[u8],
) -> Result<SourceFixedFileManifest, SourceFixedFileManifestError> {
    let mut reader = PayloadReader::new(bytes);
    let entry_count = reader.read_u64()?;
    if entry_count > reader.remaining_len() as u64 {
        return Err(SourceFixedFileManifestError::LengthOverflow);
    }
    let entry_count =
        usize::try_from(entry_count).map_err(|_| SourceFixedFileManifestError::LengthOverflow)?;
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let source_name = reader.read_string()?;
        let kind = parse_manifest_kind(reader.read_u8()?)?;
        let path = reader.read_option_string()?;
        let column = reader.read_option_u32()?;
        let group_name = reader.read_string()?;
        let group_id = reader.read_u64()?;
        let unit_id = reader.read_u64()?;
        let unit_name = reader.read_string()?;
        let template_name = reader.read_string()?;
        let virtual_instance = reader.read_bool("virtual_instance")?;
        let start = reader.read_u64()?;
        let end = reader.read_u64()?;
        entries.push(SourceFixedFileManifestEntry {
            source_name,
            kind,
            path,
            column,
            group_name,
            group_id,
            unit_id,
            unit_name,
            template_name,
            virtual_instance,
            start,
            end,
        });
    }
    reader.finish()?;
    Ok(SourceFixedFileManifest { entries })
}

fn parse_manifest_kind(
    value: u8,
) -> Result<SourceFixedFileManifestKind, SourceFixedFileManifestError> {
    match value {
        0 => Ok(SourceFixedFileManifestKind::FixedExternal),
        1 => Ok(SourceFixedFileManifestKind::ExternFixedFile),
        2 => Ok(SourceFixedFileManifestKind::FixedLoad),
        3 => Ok(SourceFixedFileManifestKind::OutputFixedFile),
        found => Err(SourceFixedFileManifestError::InvalidKind { found }),
    }
}

fn write_option_string(
    out: &mut Vec<u8>,
    value: Option<&str>,
) -> Result<(), SourceFixedFileManifestError> {
    match value {
        Some(value) => {
            write_bool(out, true);
            write_string(out, value)
        }
        None => {
            write_bool(out, false);
            Ok(())
        }
    }
}

fn write_option_u32(out: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            write_bool(out, true);
            write_u32(out, value);
        }
        None => write_bool(out, false),
    }
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), SourceFixedFileManifestError> {
    let bytes = value.as_bytes();
    write_u64(
        out,
        u64::try_from(bytes.len()).map_err(|_| SourceFixedFileManifestError::LengthOverflow)?,
    );
    out.extend_from_slice(bytes);
    Ok(())
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    write_u8(out, u8::from(value));
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct PayloadReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PayloadReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn finish(&self) -> Result<(), SourceFixedFileManifestError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(
                SourceFixedFileManifestError::UnexpectedPayloadTrailingBytes {
                    count: self.bytes.len() - self.offset,
                },
            )
        }
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], SourceFixedFileManifestError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(SourceFixedFileManifestError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(SourceFixedFileManifestError::UnexpectedPayloadEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], SourceFixedFileManifestError> {
        let bytes = self.read_exact(N)?;
        Ok(bytes.try_into().expect("slice length checked"))
    }

    fn read_u8(&mut self) -> Result<u8, SourceFixedFileManifestError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, SourceFixedFileManifestError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, SourceFixedFileManifestError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_bool(&mut self, field: &'static str) -> Result<bool, SourceFixedFileManifestError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            found => Err(SourceFixedFileManifestError::InvalidBoolean { field, found }),
        }
    }

    fn read_string(&mut self) -> Result<String, SourceFixedFileManifestError> {
        let len = usize::try_from(self.read_u64()?)
            .map_err(|_| SourceFixedFileManifestError::LengthOverflow)?;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|error| {
            SourceFixedFileManifestError::InvalidUtf8 {
                message: error.to_string(),
            }
        })
    }

    fn read_option_string(&mut self) -> Result<Option<String>, SourceFixedFileManifestError> {
        if self.read_bool("path_present")? {
            self.read_string().map(Some)
        } else {
            Ok(None)
        }
    }

    fn read_option_u32(&mut self) -> Result<Option<u32>, SourceFixedFileManifestError> {
        if self.read_bool("column_present")? {
            self.read_u32().map(Some)
        } else {
            Ok(None)
        }
    }
}

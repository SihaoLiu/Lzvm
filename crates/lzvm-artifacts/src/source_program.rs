use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

const SOURCE_PROGRAM_ARCHIVE_MAGIC: [u8; 4] = *b"spg0";
const SOURCE_PROGRAM_ARCHIVE_VERSION: u32 = 1;
const MIN_SOURCE_RECORD_BYTES: usize = 16;
const MIN_EDGE_RECORD_BYTES: usize = 18;

pub const SOURCE_PROGRAM_ARCHIVE_FILE: &str = "lzvm.source-program-archive";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProgramArchiveIncludeKind {
    Include = 0,
    Require = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProgramArchiveIncludeVisibility {
    Public = 0,
    Private = 1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramArchiveSource {
    pub source_name: String,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramArchiveEdge {
    pub from_index: u32,
    pub to_index: u32,
    pub request: String,
    pub kind: SourceProgramArchiveIncludeKind,
    pub visibility: SourceProgramArchiveIncludeVisibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramArchive {
    pub sources: Vec<SourceProgramArchiveSource>,
    pub edges: Vec<SourceProgramArchiveEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceProgramArchiveError {
    EmptySources,
    DuplicateSourceName {
        source_name: String,
    },
    MissingSourceIndex {
        index: u32,
    },
    EmptySourceName {
        index: usize,
    },
    EmptyEdgeRequest {
        edge_index: usize,
    },
    InvalidKind {
        found: [u8; 4],
    },
    UnsupportedVersion {
        found: u32,
        max: u32,
    },
    InvalidEdgeKind {
        found: u8,
    },
    InvalidEdgeVisibility {
        found: u8,
    },
    InvalidUtf8 {
        message: String,
    },
    ReadFailed {
        path: PathBuf,
        message: String,
    },
    UnexpectedTrailingBytes {
        count: usize,
    },
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    LengthOverflow,
}

impl fmt::Display for SourceProgramArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySources => write!(f, "source program archive has no sources"),
            Self::DuplicateSourceName { source_name } => {
                write!(f, "duplicate source name in archive: {source_name}")
            }
            Self::MissingSourceIndex { index } => {
                write!(f, "source program archive references missing source index {index}")
            }
            Self::EmptySourceName { index } => {
                write!(f, "source program archive source {index} has no name")
            }
            Self::EmptyEdgeRequest { edge_index } => {
                write!(f, "source program archive edge {edge_index} has no request")
            }
            Self::InvalidKind { found } => write!(
                f,
                "invalid source program archive kind: {}",
                String::from_utf8_lossy(found)
            ),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported source program archive version {found}, max {max}")
            }
            Self::InvalidEdgeKind { found } => {
                write!(f, "invalid source program archive edge kind {found}")
            }
            Self::InvalidEdgeVisibility { found } => {
                write!(f, "invalid source program archive edge visibility {found}")
            }
            Self::InvalidUtf8 { message } => {
                write!(f, "invalid source program archive utf-8: {message}")
            }
            Self::ReadFailed { path, message } => {
                write!(f, "failed to read source program archive {}: {message}", path.display())
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in source program archive: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of source program archive at {offset}, needed {needed}, available {available}"
            ),
            Self::LengthOverflow => write!(f, "source program archive length overflow"),
        }
    }
}

impl std::error::Error for SourceProgramArchiveError {}

impl SourceProgramArchive {
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

pub fn encode_source_program_archive(
    value: &SourceProgramArchive,
) -> Result<Vec<u8>, SourceProgramArchiveError> {
    validate_source_program_archive(value)?;

    let mut out = Vec::new();
    out.extend_from_slice(&SOURCE_PROGRAM_ARCHIVE_MAGIC);
    write_u32(&mut out, SOURCE_PROGRAM_ARCHIVE_VERSION);
    write_u32(
        &mut out,
        u32::try_from(value.sources.len())
            .map_err(|_| SourceProgramArchiveError::LengthOverflow)?,
    );
    write_u32(
        &mut out,
        u32::try_from(value.edges.len()).map_err(|_| SourceProgramArchiveError::LengthOverflow)?,
    );

    for source in &value.sources {
        write_string(&mut out, &source.source_name)?;
        write_string(&mut out, &source.contents)?;
    }

    for edge in &value.edges {
        write_u32(&mut out, edge.from_index);
        write_u32(&mut out, edge.to_index);
        write_string(&mut out, &edge.request)?;
        write_u8(&mut out, edge.kind as u8);
        write_u8(&mut out, edge.visibility as u8);
    }

    Ok(out)
}

pub fn parse_source_program_archive(
    bytes: &[u8],
) -> Result<SourceProgramArchive, SourceProgramArchiveError> {
    let mut reader = Reader::new(bytes);
    let found_kind = reader.read_array::<4>()?;
    if found_kind != SOURCE_PROGRAM_ARCHIVE_MAGIC {
        return Err(SourceProgramArchiveError::InvalidKind { found: found_kind });
    }

    let version = reader.read_u32()?;
    if version > SOURCE_PROGRAM_ARCHIVE_VERSION {
        return Err(SourceProgramArchiveError::UnsupportedVersion {
            found: version,
            max: SOURCE_PROGRAM_ARCHIVE_VERSION,
        });
    }

    let source_count = reader.read_u32()? as usize;
    let edge_count = reader.read_u32()? as usize;
    if source_count == 0 {
        return Err(SourceProgramArchiveError::EmptySources);
    }
    reader.require_items(source_count, MIN_SOURCE_RECORD_BYTES)?;
    let mut sources = Vec::with_capacity(source_count);
    let mut seen_names = BTreeSet::new();
    for index in 0..source_count {
        let source_name = reader.read_string()?;
        if source_name.is_empty() {
            return Err(SourceProgramArchiveError::EmptySourceName { index });
        }
        if !seen_names.insert(source_name.clone()) {
            return Err(SourceProgramArchiveError::DuplicateSourceName { source_name });
        }
        let contents = reader.read_string()?;
        sources.push(SourceProgramArchiveSource {
            source_name,
            contents,
        });
    }

    reader.require_items(edge_count, MIN_EDGE_RECORD_BYTES)?;
    let mut edges = Vec::with_capacity(edge_count);
    for edge_index in 0..edge_count {
        let from_index = reader.read_u32()?;
        let to_index = reader.read_u32()?;
        let request = reader.read_string()?;
        if request.is_empty() {
            return Err(SourceProgramArchiveError::EmptyEdgeRequest { edge_index });
        }
        let kind = match reader.read_u8()? {
            0 => SourceProgramArchiveIncludeKind::Include,
            1 => SourceProgramArchiveIncludeKind::Require,
            found => return Err(SourceProgramArchiveError::InvalidEdgeKind { found }),
        };
        let visibility = match reader.read_u8()? {
            0 => SourceProgramArchiveIncludeVisibility::Public,
            1 => SourceProgramArchiveIncludeVisibility::Private,
            found => return Err(SourceProgramArchiveError::InvalidEdgeVisibility { found }),
        };
        validate_source_index(from_index, source_count)?;
        validate_source_index(to_index, source_count)?;
        edges.push(SourceProgramArchiveEdge {
            from_index,
            to_index,
            request,
            kind,
            visibility,
        });
    }

    if reader.position() != bytes.len() {
        return Err(SourceProgramArchiveError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }

    Ok(SourceProgramArchive { sources, edges })
}

pub fn read_source_program_archive_file(
    path: impl AsRef<Path>,
) -> Result<SourceProgramArchive, SourceProgramArchiveError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|error| SourceProgramArchiveError::ReadFailed {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    parse_source_program_archive(&bytes)
}

fn validate_source_program_archive(
    value: &SourceProgramArchive,
) -> Result<(), SourceProgramArchiveError> {
    if value.sources.is_empty() {
        return Err(SourceProgramArchiveError::EmptySources);
    }
    for (index, source) in value.sources.iter().enumerate() {
        if source.source_name.is_empty() {
            return Err(SourceProgramArchiveError::EmptySourceName { index });
        }
    }
    let source_count = value.sources.len();
    for (edge_index, edge) in value.edges.iter().enumerate() {
        if edge.request.is_empty() {
            return Err(SourceProgramArchiveError::EmptyEdgeRequest { edge_index });
        }
        validate_source_index(edge.from_index, source_count)?;
        validate_source_index(edge.to_index, source_count)?;
    }
    let mut seen = BTreeSet::new();
    for source in &value.sources {
        if !seen.insert(source.source_name.clone()) {
            return Err(SourceProgramArchiveError::DuplicateSourceName {
                source_name: source.source_name.clone(),
            });
        }
    }
    Ok(())
}

fn validate_source_index(index: u32, source_count: usize) -> Result<(), SourceProgramArchiveError> {
    if usize::try_from(index)
        .ok()
        .is_none_or(|index| index >= source_count)
    {
        return Err(SourceProgramArchiveError::MissingSourceIndex { index });
    }
    Ok(())
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), SourceProgramArchiveError> {
    let bytes = value.as_bytes();
    let len = u64::try_from(bytes.len()).map_err(|_| SourceProgramArchiveError::LengthOverflow)?;
    write_u64(out, len);
    out.extend_from_slice(bytes);
    Ok(())
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

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn position(&self) -> usize {
        self.offset
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], SourceProgramArchiveError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(SourceProgramArchiveError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(SourceProgramArchiveError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn require_items(
        &self,
        count: usize,
        item_bytes: usize,
    ) -> Result<(), SourceProgramArchiveError> {
        let needed = count
            .checked_mul(item_bytes)
            .ok_or(SourceProgramArchiveError::LengthOverflow)?;
        let end = self
            .offset
            .checked_add(needed)
            .ok_or(SourceProgramArchiveError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(SourceProgramArchiveError::UnexpectedEof {
                offset: self.offset,
                needed,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        Ok(())
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], SourceProgramArchiveError> {
        let bytes = self.read_exact(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_u8(&mut self) -> Result<u8, SourceProgramArchiveError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, SourceProgramArchiveError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, SourceProgramArchiveError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_string(&mut self) -> Result<String, SourceProgramArchiveError> {
        let len = self.read_u64()?;
        let len = usize::try_from(len).map_err(|_| SourceProgramArchiveError::LengthOverflow)?;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|error| SourceProgramArchiveError::InvalidUtf8 {
            message: error.to_string(),
        })
    }
}

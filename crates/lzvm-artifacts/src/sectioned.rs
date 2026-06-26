use std::fmt;

const SECTION_HEADER_SIZE: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionedFile {
    pub kind: [u8; 4],
    pub version: u32,
    pub sections: Vec<SectionedSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionedFileRef<'a> {
    pub kind: [u8; 4],
    pub version: u32,
    pub sections: Vec<SectionedSectionRef<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionedSection {
    pub id: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionedSectionRef<'a> {
    pub id: u32,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionedError {
    InvalidKind {
        expected: [u8; 4],
        found: [u8; 4],
    },
    UnsupportedVersion {
        found: u32,
        max: u32,
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

impl fmt::Display for SectionedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKind { expected, found } => write!(
                f,
                "invalid sectioned file kind: expected {}, found {}",
                String::from_utf8_lossy(expected),
                String::from_utf8_lossy(found)
            ),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported sectioned file version {found}, max {max}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in sectioned file: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of sectioned file at {offset}, needed {needed}, available {available}"
            ),
            Self::LengthOverflow => write!(f, "sectioned file length overflow"),
        }
    }
}

impl std::error::Error for SectionedError {}

pub fn parse_sectioned_file(
    bytes: &[u8],
    expected_kind: [u8; 4],
    max_version: u32,
) -> Result<SectionedFile, SectionedError> {
    let parsed = parse_sectioned_file_ref(bytes, expected_kind, max_version)?;
    Ok(SectionedFile {
        kind: parsed.kind,
        version: parsed.version,
        sections: parsed
            .sections
            .into_iter()
            .map(|section| SectionedSection {
                id: section.id,
                data: section.data.to_vec(),
            })
            .collect(),
    })
}

pub fn parse_sectioned_file_ref<'a>(
    bytes: &'a [u8],
    expected_kind: [u8; 4],
    max_version: u32,
) -> Result<SectionedFileRef<'a>, SectionedError> {
    let mut reader = Reader::new(bytes);
    let kind = reader.read_array::<4>()?;
    if kind != expected_kind {
        return Err(SectionedError::InvalidKind {
            expected: expected_kind,
            found: kind,
        });
    }

    let version = reader.read_u32()?;
    if version > max_version {
        return Err(SectionedError::UnsupportedVersion {
            found: version,
            max: max_version,
        });
    }

    let section_count = reader.read_u32()?;
    let section_count =
        usize::try_from(section_count).map_err(|_| SectionedError::LengthOverflow)?;
    if section_count > reader.remaining_len() / SECTION_HEADER_SIZE {
        return Err(SectionedError::LengthOverflow);
    }

    let mut sections = Vec::with_capacity(section_count);
    for _ in 0..section_count {
        let id = reader.read_u32()?;
        let size = reader.read_u64()?;
        let size = usize::try_from(size).map_err(|_| SectionedError::LengthOverflow)?;
        let data = reader.read_exact(size)?;
        sections.push(SectionedSectionRef { id, data });
    }

    if reader.position() != bytes.len() {
        return Err(SectionedError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }

    Ok(SectionedFileRef {
        kind,
        version,
        sections,
    })
}

pub fn encode_sectioned_file(value: &SectionedFile) -> Result<Vec<u8>, SectionedError> {
    let sections = value
        .sections
        .iter()
        .map(|section| SectionedSectionRef {
            id: section.id,
            data: section.data.as_slice(),
        })
        .collect();
    encode_sectioned_file_ref(&SectionedFileRef {
        kind: value.kind,
        version: value.version,
        sections,
    })
}

pub fn encode_sectioned_file_ref(value: &SectionedFileRef<'_>) -> Result<Vec<u8>, SectionedError> {
    let mut out = Vec::new();
    out.extend_from_slice(&value.kind);
    write_u32(&mut out, value.version);
    let section_count =
        u32::try_from(value.sections.len()).map_err(|_| SectionedError::LengthOverflow)?;
    write_u32(&mut out, section_count);

    for section in &value.sections {
        write_u32(&mut out, section.id);
        let size = u64::try_from(section.data.len()).map_err(|_| SectionedError::LengthOverflow)?;
        write_u64(&mut out, size);
        out.extend_from_slice(section.data);
    }

    Ok(out)
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

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], SectionedError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(SectionedError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(SectionedError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn read_u32(&mut self) -> Result<u32, SectionedError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, SectionedError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], SectionedError> {
        let bytes = self.read_exact(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }
}

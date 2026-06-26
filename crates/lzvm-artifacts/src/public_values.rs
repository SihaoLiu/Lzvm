use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use lzvm_field::{Felt, FieldError};
use sha2::{Digest, Sha256};

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const PUBLIC_VALUES_KIND: [u8; 4] = *b"pval";
const PUBLIC_VALUES_VERSION: u32 = 1;
const PUBLIC_VALUES_SCHEMA_VERSION: u32 = 1;
const PUBLIC_VALUES_SECTION_ID: u32 = 1;
const VALUE_ENTRY_HEADER_BYTES: usize = 4 + 4;
const ELEMENT_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicValues {
    pub schema_version: u32,
    pub setup_hash: [u8; 32],
    pub values: Vec<PublicValueEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicValueEntry {
    pub name: String,
    pub elements: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicValuesError {
    InvalidMagic,
    UnsupportedVersion {
        found: u32,
        max: u32,
    },
    UnsupportedSchemaVersion {
        found: u32,
        expected: u32,
    },
    InvalidSectionCount {
        found: u32,
    },
    InvalidSectionId {
        found: u32,
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
    InvalidUtf8,
    EmptyName {
        index: usize,
    },
    DuplicateName {
        name: String,
    },
    EmptyValue {
        name: String,
    },
    ElementNonCanonical {
        name: String,
        element_index: usize,
        source: FieldError,
    },
    Io {
        message: String,
    },
}

impl fmt::Display for PublicValuesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid public-values file magic"),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported public-values file version {found}, max {max}")
            }
            Self::UnsupportedSchemaVersion { found, expected } => {
                write!(
                    f,
                    "unsupported public-values schema version {found}, expected {expected}"
                )
            }
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid public-values section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid public-values section id {found}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in public-values file: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of public-values file at {offset}, needed {needed}, available {available}"
            ),
            Self::LengthOverflow => write!(f, "public-values length overflow"),
            Self::InvalidUtf8 => write!(f, "public-values string is not valid utf-8"),
            Self::EmptyName { index } => {
                write!(f, "empty public-values name at index {index}")
            }
            Self::DuplicateName { name } => {
                write!(f, "duplicate public-values name: {name}")
            }
            Self::EmptyValue { name } => write!(f, "empty public-values entry: {name}"),
            Self::ElementNonCanonical {
                name,
                element_index,
                source,
            } => write!(
                f,
                "public-values entry {name} element {element_index} is non-canonical: {source}"
            ),
            Self::Io { message } => write!(f, "public-values io error: {message}"),
        }
    }
}

impl std::error::Error for PublicValuesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ElementNonCanonical { source, .. } => Some(source),
            Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::UnsupportedSchemaVersion { .. }
            | Self::InvalidSectionCount { .. }
            | Self::InvalidSectionId { .. }
            | Self::UnexpectedTrailingBytes { .. }
            | Self::UnexpectedEof { .. }
            | Self::LengthOverflow
            | Self::InvalidUtf8
            | Self::EmptyName { .. }
            | Self::DuplicateName { .. }
            | Self::EmptyValue { .. }
            | Self::Io { .. } => None,
        }
    }
}

impl From<SectionedError> for PublicValuesError {
    fn from(value: SectionedError) -> Self {
        match value {
            SectionedError::InvalidKind { .. } => Self::InvalidMagic,
            SectionedError::UnsupportedVersion { found, max } => {
                Self::UnsupportedVersion { found, max }
            }
            SectionedError::UnexpectedTrailingBytes { count } => {
                Self::UnexpectedTrailingBytes { count }
            }
            SectionedError::UnexpectedEof {
                offset,
                needed,
                available,
            } => Self::UnexpectedEof {
                offset,
                needed,
                available,
            },
            SectionedError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

pub fn read_public_values_file(path: impl AsRef<Path>) -> Result<PublicValues, PublicValuesError> {
    let bytes = std::fs::read(path).map_err(|error| PublicValuesError::Io {
        message: error.to_string(),
    })?;
    parse_public_values(&bytes)
}

pub fn read_public_values_binary_file(
    path: impl AsRef<Path>,
) -> Result<PublicValues, PublicValuesError> {
    let bytes = std::fs::read(path).map_err(|error| PublicValuesError::Io {
        message: error.to_string(),
    })?;
    parse_public_values(&bytes)
}

pub fn parse_public_values(bytes: &[u8]) -> Result<PublicValues, PublicValuesError> {
    let file = parse_sectioned_file(bytes, PUBLIC_VALUES_KIND, PUBLIC_VALUES_VERSION)
        .map_err(PublicValuesError::from)?;
    if file.version != PUBLIC_VALUES_VERSION {
        return Err(PublicValuesError::UnsupportedVersion {
            found: file.version,
            max: PUBLIC_VALUES_VERSION,
        });
    }

    if file.sections.len() != 1 {
        return Err(PublicValuesError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }

    let section = &file.sections[0];
    if section.id != PUBLIC_VALUES_SECTION_ID {
        return Err(PublicValuesError::InvalidSectionId { found: section.id });
    }

    parse_public_values_section(&section.data)
}

pub fn encode_public_values(value: &PublicValues) -> Result<Vec<u8>, PublicValuesError> {
    validate_public_values(value)?;
    let section = encode_public_values_section(value)?;
    let file = SectionedFile {
        kind: PUBLIC_VALUES_KIND,
        version: PUBLIC_VALUES_VERSION,
        sections: vec![SectionedSection {
            id: PUBLIC_VALUES_SECTION_ID,
            data: section,
        }],
    };
    encode_sectioned_file(&file).map_err(PublicValuesError::from)
}

fn encode_public_values_digest_payload(value: &PublicValues) -> Result<String, PublicValuesError> {
    validate_public_values(value)?;

    let mut out = String::new();
    out.push('{');
    out.push_str("\"schema_version\":");
    out.push_str(&value.schema_version.to_string());
    out.push_str(",\"setup_hash\":");
    push_digest_string(&mut out, &encode_hash(&value.setup_hash));
    out.push_str(",\"values\":[");
    for (index, entry) in value.values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"name\":");
        push_digest_string(&mut out, &entry.name);
        out.push_str(",\"elements\":[");
        for (element_index, element) in entry.elements.iter().enumerate() {
            if element_index > 0 {
                out.push(',');
            }
            push_digest_string(&mut out, &element.to_string());
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    Ok(out)
}

pub fn public_values_digest(value: &PublicValues) -> Result<[u8; 32], PublicValuesError> {
    let encoded = encode_public_values_digest_payload(value)?;
    let digest = Sha256::digest(encoded.as_bytes());
    Ok(digest.into())
}

fn parse_public_values_section(bytes: &[u8]) -> Result<PublicValues, PublicValuesError> {
    let mut reader = Reader::new(bytes);
    let schema_version = reader.read_u32()?;
    let setup_hash = reader.read_hash()?;
    let value_count = u32_to_usize(reader.read_u32()?)?;
    reader.require_items(value_count, VALUE_ENTRY_HEADER_BYTES)?;
    let mut values = Vec::with_capacity(value_count);
    for _ in 0..value_count {
        let name = reader.read_string()?;
        let element_count = u32_to_usize(reader.read_u32()?)?;
        reader.require_items(element_count, ELEMENT_BYTES)?;
        let mut elements = Vec::with_capacity(element_count);
        for _ in 0..element_count {
            elements.push(reader.read_u64()?);
        }
        values.push(PublicValueEntry { name, elements });
    }

    if reader.position() != bytes.len() {
        return Err(PublicValuesError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }

    let out = PublicValues {
        schema_version,
        setup_hash,
        values,
    };
    validate_public_values(&out)?;
    Ok(out)
}

fn encode_public_values_section(value: &PublicValues) -> Result<Vec<u8>, PublicValuesError> {
    let mut out = Vec::new();
    write_u32(&mut out, value.schema_version);
    out.extend_from_slice(&value.setup_hash);
    write_len(&mut out, value.values.len())?;
    for entry in &value.values {
        write_string(&mut out, &entry.name)?;
        write_len(&mut out, entry.elements.len())?;
        for element in &entry.elements {
            write_u64(&mut out, *element);
        }
    }
    Ok(out)
}

fn validate_public_values(value: &PublicValues) -> Result<(), PublicValuesError> {
    if value.schema_version != PUBLIC_VALUES_SCHEMA_VERSION {
        return Err(PublicValuesError::UnsupportedSchemaVersion {
            found: value.schema_version,
            expected: PUBLIC_VALUES_SCHEMA_VERSION,
        });
    }

    let mut names = BTreeSet::new();
    for (index, entry) in value.values.iter().enumerate() {
        if entry.name.is_empty() {
            return Err(PublicValuesError::EmptyName { index });
        }
        if !names.insert(entry.name.clone()) {
            return Err(PublicValuesError::DuplicateName {
                name: entry.name.clone(),
            });
        }
        if entry.elements.is_empty() {
            return Err(PublicValuesError::EmptyValue {
                name: entry.name.clone(),
            });
        }
        for (element_index, element) in entry.elements.iter().copied().enumerate() {
            Felt::from_canonical(element).map_err(|source| {
                PublicValuesError::ElementNonCanonical {
                    name: entry.name.clone(),
                    element_index,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn encode_hash(value: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in value {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn push_digest_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '\u{00}'..='\u{1f}' => push_digest_control_escape(out, character as u8),
            character => out.push(character),
        }
    }
    out.push('"');
}

fn push_digest_control_escape(out: &mut String, value: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push_str("\\u00");
    out.push(HEX[(value >> 4) as usize] as char);
    out.push(HEX[(value & 0x0f) as usize] as char);
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), PublicValuesError> {
    write_len(out, value.len())?;
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn write_len(out: &mut Vec<u8>, len: usize) -> Result<(), PublicValuesError> {
    let len = u32::try_from(len).map_err(|_| PublicValuesError::LengthOverflow)?;
    write_u32(out, len);
    Ok(())
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn u32_to_usize(value: u32) -> Result<usize, PublicValuesError> {
    usize::try_from(value).map_err(|_| PublicValuesError::LengthOverflow)
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

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], PublicValuesError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(PublicValuesError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PublicValuesError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn require_items(&self, count: usize, item_bytes: usize) -> Result<(), PublicValuesError> {
        let needed = count
            .checked_mul(item_bytes)
            .ok_or(PublicValuesError::LengthOverflow)?;
        let end = self
            .offset
            .checked_add(needed)
            .ok_or(PublicValuesError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PublicValuesError::UnexpectedEof {
                offset: self.offset,
                needed,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        Ok(())
    }

    fn read_u32(&mut self) -> Result<u32, PublicValuesError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, PublicValuesError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_hash(&mut self) -> Result<[u8; 32], PublicValuesError> {
        self.read_array::<32>()
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], PublicValuesError> {
        let bytes = self.read_exact(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_string(&mut self) -> Result<String, PublicValuesError> {
        let len = u32_to_usize(self.read_u32()?)?;
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| PublicValuesError::InvalidUtf8)
    }
}

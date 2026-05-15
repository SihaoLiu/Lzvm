use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const PUBLIC_VALUES_KIND: [u8; 4] = *b"pval";
const PUBLIC_VALUES_VERSION: u32 = 1;
const PUBLIC_VALUES_SECTION_ID: u32 = 1;

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
    Json {
        message: String,
    },
    InvalidMagic,
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
    MissingField {
        field: &'static str,
    },
    InvalidField {
        field: &'static str,
    },
    InvalidHash {
        field: &'static str,
    },
    EmptyName {
        index: usize,
    },
    DuplicateName {
        name: String,
    },
    EmptyValue {
        name: String,
    },
    InvalidElement {
        name: String,
    },
    Io {
        message: String,
    },
}

impl fmt::Display for PublicValuesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => write!(f, "public-values json error: {message}"),
            Self::InvalidMagic => write!(f, "invalid public-values file magic"),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported public-values file version {found}, max {max}")
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
            Self::MissingField { field } => write!(f, "missing public-values field: {field}"),
            Self::InvalidField { field } => write!(f, "invalid public-values field: {field}"),
            Self::InvalidHash { field } => write!(f, "invalid public-values hash: {field}"),
            Self::EmptyName { index } => {
                write!(f, "empty public-values name at index {index}")
            }
            Self::DuplicateName { name } => {
                write!(f, "duplicate public-values name: {name}")
            }
            Self::EmptyValue { name } => write!(f, "empty public-values entry: {name}"),
            Self::InvalidElement { name } => {
                write!(f, "invalid public-values element for {name}")
            }
            Self::Io { message } => write!(f, "public-values io error: {message}"),
        }
    }
}

impl std::error::Error for PublicValuesError {}

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

pub fn parse_public_values_json(input: &str) -> Result<PublicValues, PublicValuesError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| PublicValuesError::Json {
            message: error.to_string(),
        })?;
    let object = value
        .as_object()
        .ok_or(PublicValuesError::InvalidField { field: "root" })?;
    let schema_version = read_u32(object, "schema_version")?;
    let setup_hash = read_hash(object, "setup_hash")?;
    let values = object
        .get("values")
        .and_then(serde_json::Value::as_array)
        .ok_or(PublicValuesError::MissingField { field: "values" })?;

    let mut entries = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let object = value
            .as_object()
            .ok_or(PublicValuesError::InvalidField { field: "values" })?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or(PublicValuesError::MissingField { field: "name" })?
            .to_owned();
        if name.is_empty() {
            return Err(PublicValuesError::EmptyName { index });
        }
        let element_values = object
            .get("elements")
            .and_then(serde_json::Value::as_array)
            .ok_or(PublicValuesError::MissingField { field: "elements" })?;
        if element_values.is_empty() {
            return Err(PublicValuesError::EmptyValue { name });
        }
        let mut elements = Vec::with_capacity(element_values.len());
        for element in element_values {
            let Some(text) = element.as_str() else {
                return Err(PublicValuesError::InvalidElement { name });
            };
            elements.push(
                text.parse::<u64>()
                    .map_err(|_| PublicValuesError::InvalidElement { name: name.clone() })?,
            );
        }
        entries.push(PublicValueEntry { name, elements });
    }

    let out = PublicValues {
        schema_version,
        setup_hash,
        values: entries,
    };
    validate_public_values(&out)?;
    Ok(out)
}

pub fn encode_public_values_json(value: &PublicValues) -> Result<String, PublicValuesError> {
    validate_public_values(value)?;

    let mut out = String::new();
    out.push('{');
    out.push_str("\"schema_version\":");
    out.push_str(&value.schema_version.to_string());
    out.push_str(",\"setup_hash\":");
    out.push_str(
        &serde_json::to_string(&encode_hash(&value.setup_hash)).map_err(|error| {
            PublicValuesError::Json {
                message: error.to_string(),
            }
        })?,
    );
    out.push_str(",\"values\":[");
    for (index, entry) in value.values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"name\":");
        out.push_str(&serde_json::to_string(&entry.name).map_err(|error| {
            PublicValuesError::Json {
                message: error.to_string(),
            }
        })?);
        out.push_str(",\"elements\":[");
        for (element_index, element) in entry.elements.iter().enumerate() {
            if element_index > 0 {
                out.push(',');
            }
            out.push_str(
                &serde_json::to_string(&element.to_string()).map_err(|error| {
                    PublicValuesError::Json {
                        message: error.to_string(),
                    }
                })?,
            );
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    Ok(out)
}

pub fn public_values_digest(value: &PublicValues) -> Result<[u8; 32], PublicValuesError> {
    let encoded = encode_public_values_json(value)?;
    let digest = Sha256::digest(encoded.as_bytes());
    Ok(digest.into())
}

fn parse_public_values_section(bytes: &[u8]) -> Result<PublicValues, PublicValuesError> {
    let mut reader = Reader::new(bytes);
    let schema_version = reader.read_u32()?;
    let setup_hash = reader.read_hash()?;
    let value_count = reader.read_u32()?;
    let mut values = Vec::with_capacity(u32_to_usize(value_count)?);
    for _ in 0..value_count {
        let name = reader.read_string()?;
        let element_count = reader.read_u32()?;
        let mut elements = Vec::with_capacity(u32_to_usize(element_count)?);
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
    }
    Ok(())
}

fn read_u32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u32, PublicValuesError> {
    let value = object
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or(PublicValuesError::MissingField { field })?;
    u32::try_from(value).map_err(|_| PublicValuesError::InvalidField { field })
}

fn read_hash(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<[u8; 32], PublicValuesError> {
    let text = object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or(PublicValuesError::MissingField { field })?;
    decode_hash(text).ok_or(PublicValuesError::InvalidHash { field })
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

fn decode_hash(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        out[index] = (hex_value(chunk[0])? << 4) | hex_value(chunk[1])?;
    }
    Some(out)
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
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

    fn read_u32(&mut self) -> Result<u32, PublicValuesError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, PublicValuesError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_hash(&mut self) -> Result<[u8; 32], PublicValuesError> {
        let bytes = self.read_exact(32)?;
        Ok(bytes.try_into().expect("slice length checked"))
    }

    fn read_string(&mut self) -> Result<String, PublicValuesError> {
        let len = u32_to_usize(self.read_u32()?)?;
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| PublicValuesError::InvalidUtf8)
    }
}

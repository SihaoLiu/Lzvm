use std::fmt;
use std::path::Path;

use sha2::{Digest, Sha256};

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELF64_CLASS: u8 = 2;
const LITTLE_ENDIAN_DATA: u8 = 1;
const CURRENT_VERSION: u8 = 1;
const DYNAMIC_OBJECT_TYPE: u16 = 3;
const ELF64_HEADER_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessLibraryInfo {
    pub byte_len: u64,
    pub digest: [u8; 32],
    pub elf_class: ElfClass,
    pub endian: ElfEndian,
    pub kind: LibraryKind,
    pub machine: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    Elf64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfEndian {
    Little,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryKind {
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessLibraryError {
    Io { message: String },
    InvalidMagic,
    HeaderTooSmall { actual: usize, minimum: usize },
    UnsupportedClass { class: u8 },
    UnsupportedEndian { endian: u8 },
    UnsupportedVersion { version: u8 },
    UnsupportedObjectType { object_type: u16 },
    UnsupportedHeaderSize { header_size: u16 },
}

impl fmt::Display for WitnessLibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { message } => write!(f, "witness library io error: {message}"),
            Self::InvalidMagic => write!(f, "invalid witness library magic"),
            Self::HeaderTooSmall { actual, minimum } => write!(
                f,
                "witness library header is too small: expected at least {minimum}, found {actual}"
            ),
            Self::UnsupportedClass { class } => {
                write!(f, "unsupported witness library class: {class}")
            }
            Self::UnsupportedEndian { endian } => {
                write!(f, "unsupported witness library endian marker: {endian}")
            }
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported witness library version: {version}")
            }
            Self::UnsupportedObjectType { object_type } => {
                write!(f, "unsupported witness library object type: {object_type}")
            }
            Self::UnsupportedHeaderSize { header_size } => {
                write!(f, "unsupported witness library header size: {header_size}")
            }
        }
    }
}

impl std::error::Error for WitnessLibraryError {}

pub fn read_witness_library_file(
    path: impl AsRef<Path>,
) -> Result<WitnessLibraryInfo, WitnessLibraryError> {
    let bytes = std::fs::read(path).map_err(|error| WitnessLibraryError::Io {
        message: error.to_string(),
    })?;
    parse_witness_library(&bytes)
}

pub fn parse_witness_library(bytes: &[u8]) -> Result<WitnessLibraryInfo, WitnessLibraryError> {
    if !bytes.starts_with(ELF_MAGIC) {
        return Err(WitnessLibraryError::InvalidMagic);
    }
    if bytes.len() < ELF64_HEADER_BYTES {
        return Err(WitnessLibraryError::HeaderTooSmall {
            actual: bytes.len(),
            minimum: ELF64_HEADER_BYTES,
        });
    }
    if bytes[4] != ELF64_CLASS {
        return Err(WitnessLibraryError::UnsupportedClass { class: bytes[4] });
    }
    if bytes[5] != LITTLE_ENDIAN_DATA {
        return Err(WitnessLibraryError::UnsupportedEndian { endian: bytes[5] });
    }
    if bytes[6] != CURRENT_VERSION {
        return Err(WitnessLibraryError::UnsupportedVersion { version: bytes[6] });
    }

    let object_type = read_u16(bytes, 16);
    if object_type != DYNAMIC_OBJECT_TYPE {
        return Err(WitnessLibraryError::UnsupportedObjectType { object_type });
    }

    let header_size = read_u16(bytes, 52);
    if header_size as usize != ELF64_HEADER_BYTES {
        return Err(WitnessLibraryError::UnsupportedHeaderSize { header_size });
    }

    let digest = Sha256::digest(bytes);
    Ok(WitnessLibraryInfo {
        byte_len: bytes.len() as u64,
        digest: digest.into(),
        elf_class: ElfClass::Elf64,
        endian: ElfEndian::Little,
        kind: LibraryKind::Dynamic,
        machine: read_u16(bytes, 18),
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("header length checked"),
    )
}

use std::fmt;
use std::path::Path;

use sha2::{Digest, Sha256};

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELF64_CLASS: u8 = 2;
const LITTLE_ENDIAN_DATA: u8 = 1;
const CURRENT_VERSION: u8 = 1;
const ELF64_HEADER_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestImageInfo {
    pub byte_len: u64,
    pub digest: [u8; 32],
    pub elf_class: ElfClass,
    pub endian: ElfEndian,
    pub machine: u16,
    pub entry: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    Elf64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfEndian {
    Little,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestImageError {
    Io { message: String },
    InvalidMagic,
    HeaderTooSmall { actual: usize, minimum: usize },
    UnsupportedClass { class: u8 },
    UnsupportedEndian { endian: u8 },
    UnsupportedVersion { version: u8 },
    UnsupportedHeaderSize { header_size: u16 },
}

impl fmt::Display for GuestImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { message } => write!(f, "guest image io error: {message}"),
            Self::InvalidMagic => write!(f, "invalid guest image magic"),
            Self::HeaderTooSmall { actual, minimum } => write!(
                f,
                "guest image header is too small: expected at least {minimum}, found {actual}"
            ),
            Self::UnsupportedClass { class } => {
                write!(f, "unsupported guest image class: {class}")
            }
            Self::UnsupportedEndian { endian } => {
                write!(f, "unsupported guest image endian marker: {endian}")
            }
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported guest image version: {version}")
            }
            Self::UnsupportedHeaderSize { header_size } => {
                write!(f, "unsupported guest image header size: {header_size}")
            }
        }
    }
}

impl std::error::Error for GuestImageError {}

pub fn read_guest_image_file(path: impl AsRef<Path>) -> Result<GuestImageInfo, GuestImageError> {
    let bytes = std::fs::read(path).map_err(|error| GuestImageError::Io {
        message: error.to_string(),
    })?;
    parse_guest_image(&bytes)
}

pub fn parse_guest_image(bytes: &[u8]) -> Result<GuestImageInfo, GuestImageError> {
    if !bytes.starts_with(ELF_MAGIC) {
        return Err(GuestImageError::InvalidMagic);
    }
    if bytes.len() < ELF64_HEADER_BYTES {
        return Err(GuestImageError::HeaderTooSmall {
            actual: bytes.len(),
            minimum: ELF64_HEADER_BYTES,
        });
    }
    if bytes[4] != ELF64_CLASS {
        return Err(GuestImageError::UnsupportedClass { class: bytes[4] });
    }
    if bytes[5] != LITTLE_ENDIAN_DATA {
        return Err(GuestImageError::UnsupportedEndian { endian: bytes[5] });
    }
    if bytes[6] != CURRENT_VERSION {
        return Err(GuestImageError::UnsupportedVersion { version: bytes[6] });
    }

    let header_size = read_u16(bytes, 52);
    if header_size as usize != ELF64_HEADER_BYTES {
        return Err(GuestImageError::UnsupportedHeaderSize { header_size });
    }

    let digest = Sha256::digest(bytes);
    Ok(GuestImageInfo {
        byte_len: bytes.len() as u64,
        digest: digest.into(),
        elf_class: ElfClass::Elf64,
        endian: ElfEndian::Little,
        machine: read_u16(bytes, 18),
        entry: read_u64(bytes, 24),
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("header length checked"),
    )
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("header length checked"),
    )
}

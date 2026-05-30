use std::fmt;
use std::path::Path;

use sha2::{Digest, Sha256};

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELF64_CLASS: u8 = 2;
const LITTLE_ENDIAN_DATA: u8 = 1;
const CURRENT_VERSION: u8 = 1;
const ELF64_HEADER_BYTES: usize = 64;
const ELF64_PROGRAM_HEADER_BYTES: u16 = 56;
const PT_LOAD: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestImageInfo {
    pub byte_len: u64,
    pub digest: [u8; 32],
    pub elf_class: ElfClass,
    pub endian: ElfEndian,
    pub machine: u16,
    pub entry: u64,
    pub load_segments: Vec<GuestImageLoadSegment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestImageLoadSegment {
    pub program_header_index: u16,
    pub flags: u32,
    pub file_offset: u64,
    pub virtual_address: u64,
    pub physical_address: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub align: u64,
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
    Io {
        message: String,
    },
    InvalidMagic,
    HeaderTooSmall {
        actual: usize,
        minimum: usize,
    },
    UnsupportedClass {
        class: u8,
    },
    UnsupportedEndian {
        endian: u8,
    },
    UnsupportedVersion {
        version: u8,
    },
    UnsupportedHeaderSize {
        header_size: u16,
    },
    UnsupportedProgramHeaderEntrySize {
        entry_size: u16,
    },
    ProgramHeaderTableOutOfBounds {
        offset: u64,
        entry_size: u16,
        count: u16,
        byte_len: usize,
    },
    LoadSegmentFileRangeOutOfBounds {
        program_header_index: u16,
        file_offset: u64,
        file_size: u64,
        byte_len: usize,
    },
    LoadSegmentFileSizeExceedsMemorySize {
        program_header_index: u16,
        file_size: u64,
        memory_size: u64,
    },
    LoadSegmentMemoryRangeOverflow {
        program_header_index: u16,
        virtual_address: u64,
        memory_size: u64,
    },
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
            Self::UnsupportedProgramHeaderEntrySize { entry_size } => write!(
                f,
                "unsupported guest image program header entry size: {entry_size}"
            ),
            Self::ProgramHeaderTableOutOfBounds {
                offset,
                entry_size,
                count,
                byte_len,
            } => write!(
                f,
                "guest image program header table is out of bounds: offset {offset}, entry size {entry_size}, count {count}, byte length {byte_len}"
            ),
            Self::LoadSegmentFileRangeOutOfBounds {
                program_header_index,
                file_offset,
                file_size,
                byte_len,
            } => write!(
                f,
                "guest image load segment {program_header_index} file range is out of bounds: offset {file_offset}, size {file_size}, byte length {byte_len}"
            ),
            Self::LoadSegmentFileSizeExceedsMemorySize {
                program_header_index,
                file_size,
                memory_size,
            } => write!(
                f,
                "guest image load segment {program_header_index} file size exceeds memory size: file {file_size}, memory {memory_size}"
            ),
            Self::LoadSegmentMemoryRangeOverflow {
                program_header_index,
                virtual_address,
                memory_size,
            } => write!(
                f,
                "guest image load segment {program_header_index} memory range overflows: virtual address {virtual_address}, size {memory_size}"
            ),
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

    let load_segments = parse_load_segments(bytes)?;
    let digest = Sha256::digest(bytes);
    Ok(GuestImageInfo {
        byte_len: bytes.len() as u64,
        digest: digest.into(),
        elf_class: ElfClass::Elf64,
        endian: ElfEndian::Little,
        machine: read_u16(bytes, 18),
        entry: read_u64(bytes, 24),
        load_segments,
    })
}

fn parse_load_segments(bytes: &[u8]) -> Result<Vec<GuestImageLoadSegment>, GuestImageError> {
    let table_offset = read_u64(bytes, 32);
    let entry_size = read_u16(bytes, 54);
    let entry_count = read_u16(bytes, 56);
    if entry_count == 0 {
        return Ok(Vec::new());
    }
    if entry_size != ELF64_PROGRAM_HEADER_BYTES {
        return Err(GuestImageError::UnsupportedProgramHeaderEntrySize { entry_size });
    }
    let table_byte_count = u64::from(entry_size)
        .checked_mul(u64::from(entry_count))
        .ok_or(GuestImageError::ProgramHeaderTableOutOfBounds {
            offset: table_offset,
            entry_size,
            count: entry_count,
            byte_len: bytes.len(),
        })?;
    let table_end = table_offset.checked_add(table_byte_count).ok_or(
        GuestImageError::ProgramHeaderTableOutOfBounds {
            offset: table_offset,
            entry_size,
            count: entry_count,
            byte_len: bytes.len(),
        },
    )?;
    if table_end > bytes.len() as u64 {
        return Err(GuestImageError::ProgramHeaderTableOutOfBounds {
            offset: table_offset,
            entry_size,
            count: entry_count,
            byte_len: bytes.len(),
        });
    }

    let mut load_segments = Vec::new();
    let table_start = usize::try_from(table_offset).expect("table range checked");
    let entry_size = usize::from(entry_size);
    for entry_index in 0..entry_count {
        let start = table_start + usize::from(entry_index) * entry_size;
        let entry = &bytes[start..start + entry_size];
        if read_u32(entry, 0) != PT_LOAD {
            continue;
        }

        let segment = GuestImageLoadSegment {
            program_header_index: entry_index,
            flags: read_u32(entry, 4),
            file_offset: read_u64(entry, 8),
            virtual_address: read_u64(entry, 16),
            physical_address: read_u64(entry, 24),
            file_size: read_u64(entry, 32),
            memory_size: read_u64(entry, 40),
            align: read_u64(entry, 48),
        };
        validate_load_segment(bytes.len(), &segment)?;
        load_segments.push(segment);
    }
    Ok(load_segments)
}

fn validate_load_segment(
    byte_len: usize,
    segment: &GuestImageLoadSegment,
) -> Result<(), GuestImageError> {
    let file_end = segment.file_offset.checked_add(segment.file_size).ok_or(
        GuestImageError::LoadSegmentFileRangeOutOfBounds {
            program_header_index: segment.program_header_index,
            file_offset: segment.file_offset,
            file_size: segment.file_size,
            byte_len,
        },
    )?;
    if file_end > byte_len as u64 {
        return Err(GuestImageError::LoadSegmentFileRangeOutOfBounds {
            program_header_index: segment.program_header_index,
            file_offset: segment.file_offset,
            file_size: segment.file_size,
            byte_len,
        });
    }
    if segment.file_size > segment.memory_size {
        return Err(GuestImageError::LoadSegmentFileSizeExceedsMemorySize {
            program_header_index: segment.program_header_index,
            file_size: segment.file_size,
            memory_size: segment.memory_size,
        });
    }
    segment
        .virtual_address
        .checked_add(segment.memory_size)
        .ok_or(GuestImageError::LoadSegmentMemoryRangeOverflow {
            program_header_index: segment.program_header_index,
            virtual_address: segment.virtual_address,
            memory_size: segment.memory_size,
        })?;
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("header length checked"),
    )
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
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

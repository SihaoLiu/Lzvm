use std::fmt;

use lzvm_artifacts::guest_image::{GuestImageInfo, GuestImageLoadSegment};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMemoryImage {
    entry_address: u64,
    segments: Vec<GuestMemorySegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMemorySegment {
    program_header_index: u16,
    flags: u32,
    virtual_address: u64,
    physical_address: u64,
    memory_size: u64,
    align: u64,
    initialized_bytes: Vec<u8>,
}

impl GuestMemoryImage {
    pub fn entry_address(&self) -> u64 {
        self.entry_address
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    pub fn segments(&self) -> &[GuestMemorySegment] {
        &self.segments
    }

    pub fn read_range_into(&self, address: u64, bytes: &mut [u8]) -> Result<(), GuestMemoryError> {
        let byte_len = bytes.len();
        let byte_len_u64 = u64::try_from(byte_len)
            .map_err(|_| GuestMemoryError::AddressRangeOverflow { address, byte_len })?;
        let end_address = address
            .checked_add(byte_len_u64)
            .ok_or(GuestMemoryError::AddressRangeOverflow { address, byte_len })?;
        for segment in &self.segments {
            let segment_end = segment.end_address()?;
            if address >= segment.virtual_address && end_address <= segment_end {
                segment.read_range_into(address, bytes);
                return Ok(());
            }
        }
        Err(GuestMemoryError::AddressNotMapped { address, byte_len })
    }
}

pub trait GuestMemoryReader {
    fn read_range_into(&self, address: u64, bytes: &mut [u8]) -> Result<(), GuestMemoryError>;
}

impl GuestMemoryReader for GuestMemoryImage {
    fn read_range_into(&self, address: u64, bytes: &mut [u8]) -> Result<(), GuestMemoryError> {
        GuestMemoryImage::read_range_into(self, address, bytes)
    }
}

impl GuestMemorySegment {
    pub fn program_header_index(&self) -> u16 {
        self.program_header_index
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn virtual_address(&self) -> u64 {
        self.virtual_address
    }

    pub fn physical_address(&self) -> u64 {
        self.physical_address
    }

    pub fn memory_size(&self) -> u64 {
        self.memory_size
    }

    pub fn align(&self) -> u64 {
        self.align
    }

    pub fn initialized_bytes(&self) -> &[u8] {
        &self.initialized_bytes
    }

    fn read_range_into(&self, address: u64, bytes: &mut [u8]) {
        let start = address - self.virtual_address;
        let initialized_len = self.initialized_bytes.len() as u64;
        bytes.fill(0);
        if start < initialized_len {
            let start = usize::try_from(start).expect("initialized offset fits usize");
            let copy_len = bytes.len().min(self.initialized_bytes.len() - start);
            bytes[..copy_len].copy_from_slice(&self.initialized_bytes[start..start + copy_len]);
        }
    }

    fn end_address(&self) -> Result<u64, GuestMemoryError> {
        self.virtual_address.checked_add(self.memory_size).ok_or(
            GuestMemoryError::SegmentMemoryRangeOverflow {
                program_header_index: self.program_header_index,
                virtual_address: self.virtual_address,
                memory_size: self.memory_size,
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestMemoryError {
    SegmentFileRangeOutOfBounds {
        program_header_index: u16,
        file_offset: u64,
        file_size: u64,
        byte_len: usize,
    },
    SegmentFileSizeExceedsMemorySize {
        program_header_index: u16,
        file_size: u64,
        memory_size: u64,
    },
    SegmentMemoryRangeOverflow {
        program_header_index: u16,
        virtual_address: u64,
        memory_size: u64,
    },
    OverlappingSegments {
        first_program_header_index: u16,
        second_program_header_index: u16,
    },
    AddressRangeOverflow {
        address: u64,
        byte_len: usize,
    },
    AddressNotMapped {
        address: u64,
        byte_len: usize,
    },
}

impl fmt::Display for GuestMemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SegmentFileRangeOutOfBounds {
                program_header_index,
                file_offset,
                file_size,
                byte_len,
            } => write!(
                f,
                "guest memory segment {program_header_index} file range is out of bounds: offset {file_offset}, size {file_size}, byte length {byte_len}"
            ),
            Self::SegmentFileSizeExceedsMemorySize {
                program_header_index,
                file_size,
                memory_size,
            } => write!(
                f,
                "guest memory segment {program_header_index} file size exceeds memory size: file {file_size}, memory {memory_size}"
            ),
            Self::SegmentMemoryRangeOverflow {
                program_header_index,
                virtual_address,
                memory_size,
            } => write!(
                f,
                "guest memory segment {program_header_index} memory range overflows: virtual address {virtual_address}, size {memory_size}"
            ),
            Self::OverlappingSegments {
                first_program_header_index,
                second_program_header_index,
            } => write!(
                f,
                "guest memory segments overlap: {first_program_header_index} and {second_program_header_index}"
            ),
            Self::AddressRangeOverflow { address, byte_len } => {
                write!(f, "guest memory read range overflows: address {address}, byte length {byte_len}")
            }
            Self::AddressNotMapped { address, byte_len } => write!(
                f,
                "guest memory read range is not mapped: address {address}, byte length {byte_len}"
            ),
        }
    }
}

impl std::error::Error for GuestMemoryError {}

pub fn load_guest_memory_image(
    image: &[u8],
    info: &GuestImageInfo,
) -> Result<GuestMemoryImage, GuestMemoryError> {
    let mut load_segments = info
        .load_segments
        .iter()
        .map(|segment| validate_load_segment(image, segment))
        .collect::<Result<Vec<_>, _>>()?;
    load_segments.sort_by_key(|segment| segment.segment.virtual_address);
    validate_non_overlapping_load_segments(&load_segments)?;
    let segments = load_segments
        .iter()
        .map(|segment| load_segment(image, *segment))
        .collect();
    Ok(GuestMemoryImage {
        entry_address: info.entry,
        segments,
    })
}

#[derive(Debug, Clone, Copy)]
struct ValidatedGuestMemoryLoadSegment<'a> {
    segment: &'a GuestImageLoadSegment,
    file_offset: usize,
    file_size: usize,
}

impl ValidatedGuestMemoryLoadSegment<'_> {
    fn end_address(&self) -> Result<u64, GuestMemoryError> {
        self.segment
            .virtual_address
            .checked_add(self.segment.memory_size)
            .ok_or(GuestMemoryError::SegmentMemoryRangeOverflow {
                program_header_index: self.segment.program_header_index,
                virtual_address: self.segment.virtual_address,
                memory_size: self.segment.memory_size,
            })
    }
}

fn validate_load_segment<'a>(
    image: &[u8],
    segment: &'a GuestImageLoadSegment,
) -> Result<ValidatedGuestMemoryLoadSegment<'a>, GuestMemoryError> {
    let file_end = segment.file_offset.checked_add(segment.file_size).ok_or(
        GuestMemoryError::SegmentFileRangeOutOfBounds {
            program_header_index: segment.program_header_index,
            file_offset: segment.file_offset,
            file_size: segment.file_size,
            byte_len: image.len(),
        },
    )?;
    if file_end > image.len() as u64 {
        return Err(GuestMemoryError::SegmentFileRangeOutOfBounds {
            program_header_index: segment.program_header_index,
            file_offset: segment.file_offset,
            file_size: segment.file_size,
            byte_len: image.len(),
        });
    }
    if segment.file_size > segment.memory_size {
        return Err(GuestMemoryError::SegmentFileSizeExceedsMemorySize {
            program_header_index: segment.program_header_index,
            file_size: segment.file_size,
            memory_size: segment.memory_size,
        });
    }
    segment
        .virtual_address
        .checked_add(segment.memory_size)
        .ok_or(GuestMemoryError::SegmentMemoryRangeOverflow {
            program_header_index: segment.program_header_index,
            virtual_address: segment.virtual_address,
            memory_size: segment.memory_size,
        })?;

    let file_offset = usize::try_from(segment.file_offset).expect("file range checked");
    let file_size = usize::try_from(segment.file_size).expect("file range checked");
    Ok(ValidatedGuestMemoryLoadSegment {
        segment,
        file_offset,
        file_size,
    })
}

fn load_segment(
    image: &[u8],
    load_segment: ValidatedGuestMemoryLoadSegment<'_>,
) -> GuestMemorySegment {
    let segment = load_segment.segment;
    let file_offset = load_segment.file_offset;
    let file_size = load_segment.file_size;
    let initialized_bytes = image[file_offset..file_offset + file_size].to_vec();
    GuestMemorySegment {
        program_header_index: segment.program_header_index,
        flags: segment.flags,
        virtual_address: segment.virtual_address,
        physical_address: segment.physical_address,
        memory_size: segment.memory_size,
        align: segment.align,
        initialized_bytes,
    }
}

fn validate_non_overlapping_load_segments(
    segments: &[ValidatedGuestMemoryLoadSegment<'_>],
) -> Result<(), GuestMemoryError> {
    for pair in segments.windows(2) {
        let first = &pair[0];
        let second = &pair[1];
        if first.end_address()? > second.segment.virtual_address {
            return Err(GuestMemoryError::OverlappingSegments {
                first_program_header_index: first.segment.program_header_index,
                second_program_header_index: second.segment.program_header_index,
            });
        }
    }
    Ok(())
}

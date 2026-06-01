use std::collections::BTreeMap;

use crate::guest_memory::{
    GuestMemoryError, GuestMemoryImage, GuestMemoryReader, GuestMemorySegment,
};

const HOST_MAPPED_PROGRAM_HEADER_INDEX: u16 = u16::MAX;
const GUEST_MEMORY_OVERLAY_BLOCK_SIZE: u64 = 8;
const GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE: usize = GUEST_MEMORY_OVERLAY_BLOCK_SIZE as usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineMemory {
    entry_address: u64,
    segments: Vec<GuestMachineMemorySegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestMachineMemorySegment {
    program_header_index: u16,
    virtual_address: u64,
    memory_size: u64,
    initialized_bytes: Vec<u8>,
    written_blocks: BTreeMap<u64, Box<[u8; GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE]>>,
}

impl GuestMachineMemory {
    pub fn from_image(image: &GuestMemoryImage) -> Self {
        Self {
            entry_address: image.entry_address(),
            segments: image
                .segments()
                .iter()
                .map(GuestMachineMemorySegment::from_image_segment)
                .collect(),
        }
    }

    pub fn entry_address(&self) -> u64 {
        self.entry_address
    }

    pub fn read_range_into(&self, address: u64, bytes: &mut [u8]) -> Result<(), GuestMemoryError> {
        let byte_len = bytes.len();
        let end_address = checked_address_end(address, byte_len)?;
        for segment in &self.segments {
            let segment_end = segment.end_address()?;
            if address >= segment.virtual_address && end_address <= segment_end {
                segment.read_range_into(address, bytes);
                return Ok(());
            }
        }
        Err(GuestMemoryError::AddressNotMapped { address, byte_len })
    }

    pub fn write_range(&mut self, address: u64, bytes: &[u8]) -> Result<(), GuestMemoryError> {
        let byte_len = bytes.len();
        let end_address = checked_address_end(address, byte_len)?;
        for segment in &mut self.segments {
            let segment_end = segment.end_address()?;
            if address >= segment.virtual_address && end_address <= segment_end {
                segment.write_range(address, bytes);
                return Ok(());
            }
        }
        Err(GuestMemoryError::AddressNotMapped { address, byte_len })
    }

    pub fn map_initialized_range(
        &mut self,
        virtual_address: u64,
        initialized_bytes: Vec<u8>,
    ) -> Result<(), GuestMemoryError> {
        if initialized_bytes.is_empty() {
            return Ok(());
        }
        let memory_size = u64::try_from(initialized_bytes.len()).map_err(|_| {
            GuestMemoryError::AddressRangeOverflow {
                address: virtual_address,
                byte_len: initialized_bytes.len(),
            }
        })?;
        let mapped_segment = GuestMachineMemorySegment {
            program_header_index: HOST_MAPPED_PROGRAM_HEADER_INDEX,
            virtual_address,
            memory_size,
            initialized_bytes,
            written_blocks: BTreeMap::new(),
        };
        let mapped_end = mapped_segment.end_address()?;
        for segment in &self.segments {
            let segment_end = segment.end_address()?;
            if ranges_overlap(
                virtual_address,
                mapped_end,
                segment.virtual_address,
                segment_end,
            ) {
                return Err(GuestMemoryError::OverlappingSegments {
                    first_program_header_index: segment.program_header_index,
                    second_program_header_index: HOST_MAPPED_PROGRAM_HEADER_INDEX,
                });
            }
        }
        self.segments.push(mapped_segment);
        self.segments.sort_by_key(|segment| segment.virtual_address);
        Ok(())
    }

    pub fn map_zeroed_gap_range(
        &mut self,
        virtual_address: u64,
        memory_size: u64,
    ) -> Result<(), GuestMemoryError> {
        if memory_size == 0 {
            return Ok(());
        }
        let range_end = checked_address_end_u64(virtual_address, memory_size)?;
        let mut cursor = virtual_address;
        let mut mapped_segments = Vec::new();
        for segment in &self.segments {
            if cursor >= range_end {
                break;
            }
            let segment_start = segment.virtual_address;
            if segment_start >= range_end {
                break;
            }
            let segment_end = segment.end_address()?;
            if segment_end <= cursor {
                continue;
            }
            if segment_start > cursor {
                let gap_end = segment_start.min(range_end);
                mapped_segments.push(GuestMachineMemorySegment::zeroed(cursor, gap_end - cursor));
            }
            cursor = cursor.max(segment_end.min(range_end));
        }
        if cursor < range_end {
            mapped_segments.push(GuestMachineMemorySegment::zeroed(
                cursor,
                range_end - cursor,
            ));
        }
        self.segments.extend(mapped_segments);
        self.segments.sort_by_key(|segment| segment.virtual_address);
        Ok(())
    }

    pub fn write_or_map_initialized_range(
        &mut self,
        virtual_address: u64,
        initialized_bytes: &[u8],
    ) -> Result<(), GuestMemoryError> {
        if initialized_bytes.is_empty() {
            return Ok(());
        }
        let end_address = checked_address_end(virtual_address, initialized_bytes.len())?;
        for segment in &mut self.segments {
            let segment_end = segment.end_address()?;
            if virtual_address >= segment.virtual_address && end_address <= segment_end {
                segment.write_range(virtual_address, initialized_bytes);
                return Ok(());
            }
        }
        let memory_size = u64::try_from(initialized_bytes.len()).map_err(|_| {
            GuestMemoryError::AddressRangeOverflow {
                address: virtual_address,
                byte_len: initialized_bytes.len(),
            }
        })?;
        let mapped_segment = GuestMachineMemorySegment {
            program_header_index: HOST_MAPPED_PROGRAM_HEADER_INDEX,
            virtual_address,
            memory_size,
            initialized_bytes: initialized_bytes.to_vec(),
            written_blocks: BTreeMap::new(),
        };
        let mapped_end = mapped_segment.end_address()?;
        for segment in &self.segments {
            let segment_end = segment.end_address()?;
            if ranges_overlap(
                virtual_address,
                mapped_end,
                segment.virtual_address,
                segment_end,
            ) {
                return Err(GuestMemoryError::OverlappingSegments {
                    first_program_header_index: segment.program_header_index,
                    second_program_header_index: HOST_MAPPED_PROGRAM_HEADER_INDEX,
                });
            }
        }
        self.segments.push(mapped_segment);
        self.segments.sort_by_key(|segment| segment.virtual_address);
        Ok(())
    }

    #[cfg(test)]
    fn written_overlay_entry_count_for_tests(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.written_blocks.len())
            .sum()
    }
}

impl GuestMemoryReader for GuestMachineMemory {
    fn read_range_into(&self, address: u64, bytes: &mut [u8]) -> Result<(), GuestMemoryError> {
        GuestMachineMemory::read_range_into(self, address, bytes)
    }
}

impl GuestMachineMemorySegment {
    fn zeroed(virtual_address: u64, memory_size: u64) -> Self {
        Self {
            program_header_index: HOST_MAPPED_PROGRAM_HEADER_INDEX,
            virtual_address,
            memory_size,
            initialized_bytes: Vec::new(),
            written_blocks: BTreeMap::new(),
        }
    }

    fn from_image_segment(segment: &GuestMemorySegment) -> Self {
        Self {
            program_header_index: segment.program_header_index(),
            virtual_address: segment.virtual_address(),
            memory_size: segment.memory_size(),
            initialized_bytes: segment.initialized_bytes().to_vec(),
            written_blocks: BTreeMap::new(),
        }
    }

    fn read_range_into(&self, address: u64, bytes: &mut [u8]) {
        let mut offset = address - self.virtual_address;
        let mut out = bytes;
        while !out.is_empty() {
            let block_index = offset / GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
            let block_offset = (offset % GUEST_MEMORY_OVERLAY_BLOCK_SIZE) as usize;
            let chunk_len = out
                .len()
                .min(GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE - block_offset);
            let chunk = &mut out[..chunk_len];
            if let Some(block) = self.written_blocks.get(&block_index) {
                chunk.copy_from_slice(&block[block_offset..block_offset + chunk_len]);
            } else {
                self.read_unwritten_range_into(offset, chunk);
            }
            offset += chunk_len as u64;
            out = &mut out[chunk_len..];
        }
    }

    fn write_range(&mut self, address: u64, bytes: &[u8]) {
        let mut offset = address - self.virtual_address;
        let mut input = bytes;
        while !input.is_empty() {
            let block_index = offset / GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
            let block_offset = (offset % GUEST_MEMORY_OVERLAY_BLOCK_SIZE) as usize;
            let chunk_len = input
                .len()
                .min(GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE - block_offset);
            let block = self.written_block_mut(block_index);
            block[block_offset..block_offset + chunk_len].copy_from_slice(&input[..chunk_len]);
            offset += chunk_len as u64;
            input = &input[chunk_len..];
        }
    }

    fn read_unwritten_range_into(&self, offset: u64, bytes: &mut [u8]) {
        bytes.fill(0);
        let initialized_len = self.initialized_bytes.len() as u64;
        if offset >= initialized_len {
            return;
        }
        let start = usize::try_from(offset).expect("initialized offset fits usize");
        let copy_len = bytes.len().min(self.initialized_bytes.len() - start);
        bytes[..copy_len].copy_from_slice(&self.initialized_bytes[start..start + copy_len]);
    }

    fn written_block_mut(
        &mut self,
        block_index: u64,
    ) -> &mut Box<[u8; GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE]> {
        if !self.written_blocks.contains_key(&block_index) {
            let mut block = Box::new([0_u8; GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE]);
            let block_start = block_index * GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
            self.read_unwritten_range_into(block_start, block.as_mut_slice());
            self.written_blocks.insert(block_index, block);
        }
        self.written_blocks
            .get_mut(&block_index)
            .expect("written block was just inserted")
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

fn checked_address_end(address: u64, byte_len: usize) -> Result<u64, GuestMemoryError> {
    let byte_len_u64 = u64::try_from(byte_len)
        .map_err(|_| GuestMemoryError::AddressRangeOverflow { address, byte_len })?;
    address
        .checked_add(byte_len_u64)
        .ok_or(GuestMemoryError::AddressRangeOverflow { address, byte_len })
}

fn checked_address_end_u64(address: u64, byte_len: u64) -> Result<u64, GuestMemoryError> {
    let byte_len_usize = usize::try_from(byte_len).unwrap_or(usize::MAX);
    address
        .checked_add(byte_len)
        .ok_or(GuestMemoryError::AddressRangeOverflow {
            address,
            byte_len: byte_len_usize,
        })
}

fn ranges_overlap(first_start: u64, first_end: u64, second_start: u64, second_end: u64) -> bool {
    first_start < second_end && second_start < first_end
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_artifacts::guest_image::parse_guest_image;

    const TEST_ENTRY: u64 = 0x8000_0000;

    fn sample_guest_image() -> Vec<u8> {
        let mut bytes = vec![0_u8; 64];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&TEST_ENTRY.to_le_bytes());
        bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
        bytes
    }

    fn sample_guest_image_with_program_header(code: &[u8], memory_size: u64) -> GuestMemoryImage {
        assert!(code.len() as u64 <= memory_size);
        let mut header = [0_u8; 56];
        header[0..4].copy_from_slice(&1_u32.to_le_bytes());
        header[4..8].copy_from_slice(&5_u32.to_le_bytes());
        header[8..16].copy_from_slice(&120_u64.to_le_bytes());
        header[16..24].copy_from_slice(&TEST_ENTRY.to_le_bytes());
        header[24..32].copy_from_slice(&TEST_ENTRY.to_le_bytes());
        header[32..40].copy_from_slice(&(code.len() as u64).to_le_bytes());
        header[40..48].copy_from_slice(&memory_size.to_le_bytes());
        header[48..56].copy_from_slice(&0x1000_u64.to_le_bytes());

        let mut image = sample_guest_image();
        image[32..40].copy_from_slice(&64_u64.to_le_bytes());
        image[54..56].copy_from_slice(&56_u16.to_le_bytes());
        image[56..58].copy_from_slice(&1_u16.to_le_bytes());
        image.extend_from_slice(&header);
        image.extend_from_slice(code);

        let info = parse_guest_image(&image).expect("guest image should parse");
        crate::guest_memory::load_guest_memory_image(&image, &info)
            .expect("guest memory should load")
    }

    #[test]
    fn dense_memory_writes_use_sparse_overlay_entries() {
        let image = sample_guest_image_with_program_header(&[1, 2, 3, 4], 0x3000);
        let mut memory = GuestMachineMemory::from_image(&image);

        memory
            .write_range(TEST_ENTRY + 2, &[9, 8, 7])
            .expect("initialized range should be writable");
        let mut prefix = [0_u8; 8];
        memory
            .read_range_into(TEST_ENTRY, &mut prefix)
            .expect("initialized range should be readable");
        assert_eq!(prefix, [1, 2, 9, 8, 7, 0, 0, 0]);

        let dense = (0..4096).map(|value| value as u8).collect::<Vec<_>>();
        memory
            .write_range(TEST_ENTRY + 0x1000, &dense)
            .expect("zero-filled range should be writable");
        let mut read_back = vec![0_u8; dense.len()];
        memory
            .read_range_into(TEST_ENTRY + 0x1000, &mut read_back)
            .expect("dense range should be readable");
        assert_eq!(read_back, dense);
        assert!(memory.written_overlay_entry_count_for_tests() <= 513);
    }

    #[test]
    fn unaligned_write_preserves_neighbors_across_overlay_blocks() {
        let image = sample_guest_image_with_program_header(
            &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21],
            0x1000,
        );
        let mut memory = GuestMachineMemory::from_image(&image);

        memory
            .write_range(TEST_ENTRY + 6, &[90, 91, 92, 93])
            .expect("cross-block write should succeed");
        let mut bytes = [0_u8; 12];
        memory
            .read_range_into(TEST_ENTRY, &mut bytes)
            .expect("cross-block read should succeed");

        assert_eq!(bytes, [10, 11, 12, 13, 14, 15, 90, 91, 92, 93, 20, 21]);
    }

    #[test]
    fn read_can_span_written_and_unwritten_blocks() {
        let image = sample_guest_image_with_program_header(
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            0x1000,
        );
        let mut memory = GuestMachineMemory::from_image(&image);

        memory
            .write_range(TEST_ENTRY + 8, &[80, 81, 82])
            .expect("write should succeed");
        let mut bytes = [0_u8; 12];
        memory
            .read_range_into(TEST_ENTRY, &mut bytes)
            .expect("mixed read should succeed");

        assert_eq!(bytes, [1, 2, 3, 4, 5, 6, 7, 8, 80, 81, 82, 12]);
    }

    #[test]
    fn write_or_map_initialized_range_overlays_existing_zeroed_segment() {
        let image = sample_guest_image_with_program_header(&[1, 2, 3, 4], 4);
        let mut memory = GuestMachineMemory::from_image(&image);
        memory
            .map_zeroed_gap_range(TEST_ENTRY, 0x3000)
            .expect("zeroed gap range should map uncovered bytes");

        memory
            .write_or_map_initialized_range(TEST_ENTRY + 0x1000 + 6, &[70, 71, 72, 73])
            .expect("write into existing zeroed segment should succeed");
        let mut bytes = [9_u8; 12];
        memory
            .read_range_into(TEST_ENTRY + 0x1000, &mut bytes)
            .expect("zeroed segment read should succeed");

        assert_eq!(bytes, [0, 0, 0, 0, 0, 0, 70, 71, 72, 73, 0, 0]);
    }
}

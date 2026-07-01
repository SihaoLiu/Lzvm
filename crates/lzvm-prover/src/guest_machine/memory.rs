use std::collections::{hash_map::Entry, HashMap};
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::guest_instruction::{
    FetchedGuestInstruction, GuestInstructionError, RiscvEncodedInstruction,
};
use crate::guest_memory::{
    GuestMemoryError, GuestMemoryImage, GuestMemoryReader, GuestMemorySegment,
};

const HOST_MAPPED_PROGRAM_HEADER_INDEX: u16 = u16::MAX;
const GUEST_MEMORY_OVERLAY_BLOCK_SIZE: u64 = 128;
const GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE: usize = GUEST_MEMORY_OVERLAY_BLOCK_SIZE as usize;
const GUEST_MEMORY_SEGMENT_LOOKUP_CACHE_EMPTY: usize = usize::MAX;

type GuestMemoryOverlayBlockMap = HashMap<
    u64,
    Box<[u8; GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE]>,
    BuildHasherDefault<U64IdentityHasher>,
>;

#[derive(Default)]
struct U64IdentityHasher(u64);

impl Hasher for U64IdentityHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = hash;
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

#[derive(Debug)]
pub struct GuestMachineMemory {
    entry_address: u64,
    segments: Vec<GuestMachineMemorySegment>,
    last_segment_index: AtomicUsize,
}

impl Clone for GuestMachineMemory {
    fn clone(&self) -> Self {
        Self {
            entry_address: self.entry_address,
            segments: self.segments.clone(),
            last_segment_index: AtomicUsize::new(self.last_segment_index.load(Ordering::Relaxed)),
        }
    }
}

impl PartialEq for GuestMachineMemory {
    fn eq(&self, other: &Self) -> bool {
        self.entry_address == other.entry_address && self.segments == other.segments
    }
}

impl Eq for GuestMachineMemory {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuestMachineMemoryOverlaySnapshot {
    blocks: Vec<GuestMachineMemoryOverlayBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestMachineMemoryOverlayBlock {
    address: u64,
    byte_len: usize,
    bytes: [u8; GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestMachineMemorySegment {
    program_header_index: u16,
    virtual_address: u64,
    memory_size: u64,
    initialized_bytes: Vec<u8>,
    written_blocks: GuestMemoryOverlayBlockMap,
}

impl GuestMachineMemory {
    pub fn from_image(image: &GuestMemoryImage) -> Self {
        let mut memory = Self {
            entry_address: image.entry_address(),
            segments: image
                .segments()
                .iter()
                .map(GuestMachineMemorySegment::from_image_segment)
                .collect(),
            last_segment_index: AtomicUsize::new(GUEST_MEMORY_SEGMENT_LOOKUP_CACHE_EMPTY),
        };
        memory.sort_segments_by_address();
        memory
    }

    pub fn entry_address(&self) -> u64 {
        self.entry_address
    }

    pub fn read_range_into(&self, address: u64, bytes: &mut [u8]) -> Result<(), GuestMemoryError> {
        let byte_len = bytes.len();
        let end_address = checked_address_end(address, byte_len)?;
        if let Some(index) = self.segment_index_containing_range(address, end_address)? {
            self.segments[index].read_range_into(address, bytes);
            return Ok(());
        }
        Err(GuestMemoryError::AddressNotMapped { address, byte_len })
    }

    #[inline(always)]
    pub(crate) fn read_u64_le(
        &self,
        address: u64,
        byte_len: usize,
    ) -> Result<u64, GuestMemoryError> {
        if byte_len > 8 {
            return Err(GuestMemoryError::AddressRangeOverflow { address, byte_len });
        }
        let end_address = checked_address_end(address, byte_len)?;
        if let Some(index) = self.segment_index_containing_range(address, end_address)? {
            return Ok(self.segments[index].read_u64_le(address, byte_len));
        }
        Err(GuestMemoryError::AddressNotMapped { address, byte_len })
    }

    pub(crate) fn fetch_instruction(
        &self,
        address: u64,
    ) -> Result<FetchedGuestInstruction, GuestInstructionError> {
        if !address.is_multiple_of(2) {
            return Err(GuestInstructionError::MisalignedFetch { address });
        }

        let low_end_address = checked_address_end(address, 2)?;
        if let Some(index) = self.segment_index_containing_range(address, low_end_address)? {
            return self.fetch_instruction_from_segment(&self.segments[index], address);
        }

        Err(GuestMemoryError::AddressNotMapped {
            address,
            byte_len: 2,
        }
        .into())
    }

    fn fetch_instruction_from_segment(
        &self,
        segment: &GuestMachineMemorySegment,
        address: u64,
    ) -> Result<FetchedGuestInstruction, GuestInstructionError> {
        let low = segment.read_halfword(address);
        let encoded = if low & 0b11 == 0b11 {
            if low & 0b11100 == 0b11100 {
                RiscvEncodedInstruction::UnsupportedLong(low)
            } else {
                let instruction_end_address = checked_address_end(address, 4)?;
                let high_address = address + 2;
                let high = if segment.contains_range(high_address, instruction_end_address)? {
                    segment.read_halfword(high_address)
                } else {
                    self.read_halfword(high_address)?
                };
                RiscvEncodedInstruction::Standard(u32::from(low) | (u32::from(high) << 16))
            }
        } else {
            RiscvEncodedInstruction::Compressed(low)
        };

        Ok(FetchedGuestInstruction { address, encoded })
    }

    pub fn write_range(&mut self, address: u64, bytes: &[u8]) -> Result<(), GuestMemoryError> {
        let byte_len = bytes.len();
        let end_address = checked_address_end(address, byte_len)?;
        if let Some(index) = self.segment_index_containing_range(address, end_address)? {
            self.segments[index].write_range(address, bytes);
            return Ok(());
        }
        Err(GuestMemoryError::AddressNotMapped { address, byte_len })
    }

    #[inline(always)]
    pub(crate) fn write_u64_le<const BYTE_LEN: usize>(
        &mut self,
        address: u64,
        value: u64,
    ) -> Result<(), GuestMemoryError> {
        if BYTE_LEN > 8 {
            return Err(GuestMemoryError::AddressRangeOverflow {
                address,
                byte_len: BYTE_LEN,
            });
        }
        let end_address = checked_address_end(address, BYTE_LEN)?;
        if let Some(index) = self.segment_index_containing_range(address, end_address)? {
            self.segments[index].write_u64_le::<BYTE_LEN>(address, value);
            return Ok(());
        }
        Err(GuestMemoryError::AddressNotMapped {
            address,
            byte_len: BYTE_LEN,
        })
    }

    fn read_halfword(&self, address: u64) -> Result<u16, GuestMemoryError> {
        let end_address = checked_address_end(address, 2)?;
        if let Some(index) = self.segment_index_containing_range(address, end_address)? {
            return Ok(self.segments[index].read_halfword(address));
        }
        Err(GuestMemoryError::AddressNotMapped {
            address,
            byte_len: 2,
        })
    }

    fn segment_index_containing_range(
        &self,
        address: u64,
        end_address: u64,
    ) -> Result<Option<usize>, GuestMemoryError> {
        let cached = self.last_segment_index.load(Ordering::Relaxed);
        if let Some(segment) = self.segments.get(cached) {
            if segment.contains_range(address, end_address)? {
                return Ok(Some(cached));
            }
        }

        let Some(candidate) = self
            .segments
            .partition_point(|segment| segment.virtual_address <= address)
            .checked_sub(1)
        else {
            self.last_segment_index
                .store(GUEST_MEMORY_SEGMENT_LOOKUP_CACHE_EMPTY, Ordering::Relaxed);
            return Ok(None);
        };
        if self.segments[candidate].contains_range(address, end_address)? {
            self.last_segment_index.store(candidate, Ordering::Relaxed);
            Ok(Some(candidate))
        } else {
            self.last_segment_index
                .store(GUEST_MEMORY_SEGMENT_LOOKUP_CACHE_EMPTY, Ordering::Relaxed);
            Ok(None)
        }
    }

    fn sort_segments_by_address(&mut self) {
        self.segments.sort_by_key(|segment| segment.virtual_address);
        self.last_segment_index
            .store(GUEST_MEMORY_SEGMENT_LOOKUP_CACHE_EMPTY, Ordering::Relaxed);
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
            written_blocks: GuestMemoryOverlayBlockMap::default(),
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
        self.sort_segments_by_address();
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
        self.sort_segments_by_address();
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
            written_blocks: GuestMemoryOverlayBlockMap::default(),
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
        self.sort_segments_by_address();
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

impl GuestMachineMemoryOverlaySnapshot {
    pub(crate) fn capture(memory: &GuestMachineMemory) -> Self {
        let mut blocks = Vec::new();
        for segment in &memory.segments {
            for (block_index, bytes) in &segment.written_blocks {
                let block_offset = block_index * GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
                let byte_len = segment
                    .memory_size
                    .saturating_sub(block_offset)
                    .min(GUEST_MEMORY_OVERLAY_BLOCK_SIZE) as usize;
                blocks.push(GuestMachineMemoryOverlayBlock {
                    address: segment.virtual_address + block_offset,
                    byte_len,
                    bytes: **bytes,
                });
            }
        }
        Self { blocks }
    }

    #[allow(dead_code)]
    pub(crate) fn restore_into(
        &self,
        memory: &mut GuestMachineMemory,
    ) -> Result<(), GuestMemoryError> {
        for block in &self.blocks {
            memory.write_range(block.address, &block.bytes[..block.byte_len])?;
        }
        Ok(())
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
            written_blocks: GuestMemoryOverlayBlockMap::default(),
        }
    }

    fn from_image_segment(segment: &GuestMemorySegment) -> Self {
        Self {
            program_header_index: segment.program_header_index(),
            virtual_address: segment.virtual_address(),
            memory_size: segment.memory_size(),
            initialized_bytes: segment.initialized_bytes().to_vec(),
            written_blocks: GuestMemoryOverlayBlockMap::default(),
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

    #[inline(always)]
    fn read_u64_le(&self, address: u64, byte_len: usize) -> u64 {
        debug_assert!(byte_len <= 8);
        let offset = address - self.virtual_address;
        if self.written_blocks.is_empty() {
            return self.read_unwritten_u64_le(offset, byte_len);
        }
        if let Some(bytes) = self.contiguous_initialized_or_overlay_bytes(offset, byte_len) {
            return low_le_bytes_to_u64(bytes);
        }

        let mut bytes = [0_u8; 8];
        self.read_range_into(address, &mut bytes[..byte_len]);
        u64::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn read_unwritten_u64_le(&self, offset: u64, byte_len: usize) -> u64 {
        if let Some(bytes) = self.contiguous_initialized_bytes(offset, byte_len) {
            return low_le_bytes_to_u64(bytes);
        }

        let mut bytes = [0_u8; 8];
        self.read_unwritten_range_into(offset, &mut bytes[..byte_len]);
        u64::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn contiguous_initialized_bytes(&self, offset: u64, byte_len: usize) -> Option<&[u8]> {
        let start = usize::try_from(offset).ok()?;
        let end = start.checked_add(byte_len)?;
        self.initialized_bytes.get(start..end)
    }

    #[inline(always)]
    fn contiguous_initialized_or_overlay_bytes(
        &self,
        offset: u64,
        byte_len: usize,
    ) -> Option<&[u8]> {
        let block_index = offset / GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
        let block_offset = (offset % GUEST_MEMORY_OVERLAY_BLOCK_SIZE) as usize;
        if block_offset.checked_add(byte_len)? > GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE {
            return None;
        }
        if let Some(block) = self.written_blocks.get(&block_index) {
            return Some(&block[block_offset..block_offset + byte_len]);
        }
        self.contiguous_initialized_bytes(offset, byte_len)
    }

    fn read_halfword(&self, address: u64) -> u16 {
        if let Some(value) = self.read_contiguous_halfword(address) {
            return value;
        }
        let offset = address - self.virtual_address;
        if self.written_blocks.is_empty() {
            return u16::from_le_bytes([
                self.read_unwritten_byte(offset),
                self.read_unwritten_byte(offset + 1),
            ]);
        }
        u16::from_le_bytes([
            self.read_overlay_or_unwritten_byte(offset),
            self.read_overlay_or_unwritten_byte(offset + 1),
        ])
    }

    #[inline(always)]
    fn read_contiguous_halfword(&self, address: u64) -> Option<u16> {
        let offset = address - self.virtual_address;
        let bytes = if self.written_blocks.is_empty() {
            self.contiguous_initialized_bytes(offset, 2)?
        } else {
            self.contiguous_initialized_or_overlay_bytes(offset, 2)?
        };
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_overlay_or_unwritten_byte(&self, offset: u64) -> u8 {
        let block_index = offset / GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
        let block_offset = (offset % GUEST_MEMORY_OVERLAY_BLOCK_SIZE) as usize;
        if let Some(block) = self.written_blocks.get(&block_index) {
            return block[block_offset];
        }
        self.read_unwritten_byte(offset)
    }

    fn read_unwritten_byte(&self, offset: u64) -> u8 {
        if offset >= self.initialized_bytes.len() as u64 {
            return 0;
        }
        self.initialized_bytes[offset as usize]
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

    #[inline(always)]
    fn write_u64_le<const BYTE_LEN: usize>(&mut self, address: u64, value: u64) {
        debug_assert!(BYTE_LEN <= 8);
        if BYTE_LEN == 0 {
            return;
        }
        let offset = address - self.virtual_address;
        let block_index = offset / GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
        let block_offset = (offset % GUEST_MEMORY_OVERLAY_BLOCK_SIZE) as usize;
        if block_offset + BYTE_LEN <= GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE {
            let block = self.written_block_mut(block_index);
            write_low_u64_le_bytes::<BYTE_LEN>(&mut block[block_offset..], value);
            return;
        }

        let mut byte_offset = offset;
        for byte_index in 0..BYTE_LEN {
            let block_index = byte_offset / GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
            let block_offset = (byte_offset % GUEST_MEMORY_OVERLAY_BLOCK_SIZE) as usize;
            let block = self.written_block_mut(block_index);
            block[block_offset] = (value >> (byte_index * 8)) as u8;
            byte_offset += 1;
        }
    }

    fn written_block_mut(
        &mut self,
        block_index: u64,
    ) -> &mut Box<[u8; GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE]> {
        match self.written_blocks.entry(block_index) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let mut block = Box::new([0_u8; GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE]);
                let block_start = block_index * GUEST_MEMORY_OVERLAY_BLOCK_SIZE;
                copy_initialized_segment_range_into(
                    &self.initialized_bytes,
                    block_start,
                    block.as_mut_slice(),
                );
                entry.insert(block)
            }
        }
    }

    fn read_unwritten_range_into(&self, offset: u64, bytes: &mut [u8]) {
        read_unwritten_segment_range_into(&self.initialized_bytes, offset, bytes);
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

    fn contains_range(&self, address: u64, end_address: u64) -> Result<bool, GuestMemoryError> {
        let segment_end = self.end_address()?;
        Ok(address >= self.virtual_address && end_address <= segment_end)
    }
}

#[inline(always)]
fn write_low_u64_le_bytes<const BYTE_LEN: usize>(out: &mut [u8], value: u64) {
    debug_assert!(out.len() >= BYTE_LEN);
    if BYTE_LEN > 0 {
        out[0] = value as u8;
    }
    if BYTE_LEN > 1 {
        out[1] = (value >> 8) as u8;
    }
    if BYTE_LEN > 2 {
        out[2] = (value >> 16) as u8;
    }
    if BYTE_LEN > 3 {
        out[3] = (value >> 24) as u8;
    }
    if BYTE_LEN > 4 {
        out[4] = (value >> 32) as u8;
    }
    if BYTE_LEN > 5 {
        out[5] = (value >> 40) as u8;
    }
    if BYTE_LEN > 6 {
        out[6] = (value >> 48) as u8;
    }
    if BYTE_LEN > 7 {
        out[7] = (value >> 56) as u8;
    }
}

fn read_unwritten_segment_range_into(initialized_bytes: &[u8], offset: u64, bytes: &mut [u8]) {
    bytes.fill(0);
    copy_initialized_segment_range_into(initialized_bytes, offset, bytes);
}

fn copy_initialized_segment_range_into(initialized_bytes: &[u8], offset: u64, bytes: &mut [u8]) {
    let initialized_len = initialized_bytes.len() as u64;
    if offset >= initialized_len {
        return;
    }
    let start = usize::try_from(offset).expect("initialized offset fits usize");
    let copy_len = bytes.len().min(initialized_bytes.len() - start);
    bytes[..copy_len].copy_from_slice(&initialized_bytes[start..start + copy_len]);
}

#[inline(always)]
fn low_le_bytes_to_u64(bytes: &[u8]) -> u64 {
    debug_assert!(bytes.len() <= 8);
    if let Some(value) = common_width_le_bytes_to_u64(bytes) {
        return value;
    }
    let mut value = 0_u64;
    for (shift, byte) in bytes.iter().enumerate() {
        value |= u64::from(*byte) << (shift * 8);
    }
    value
}

#[inline(always)]
fn common_width_le_bytes_to_u64(bytes: &[u8]) -> Option<u64> {
    match bytes.len() {
        1 => Some(u64::from(bytes[0])),
        2 => Some(u64::from(u16::from_le_bytes([bytes[0], bytes[1]]))),
        4 => Some(u64::from(u32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))),
        8 => Some(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        _ => None,
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

    fn sample_guest_image_with_program_headers(segments: &[(u64, &[u8], u64)]) -> GuestMemoryImage {
        let mut image = sample_guest_image();
        image[32..40].copy_from_slice(&64_u64.to_le_bytes());
        image[54..56].copy_from_slice(&56_u16.to_le_bytes());
        image[56..58].copy_from_slice(&(segments.len() as u16).to_le_bytes());

        let mut file_offset = 64_u64 + 56 * segments.len() as u64;
        for (virtual_address, code, memory_size) in segments {
            assert!(code.len() as u64 <= *memory_size);
            let mut header = [0_u8; 56];
            header[0..4].copy_from_slice(&1_u32.to_le_bytes());
            header[4..8].copy_from_slice(&5_u32.to_le_bytes());
            header[8..16].copy_from_slice(&file_offset.to_le_bytes());
            header[16..24].copy_from_slice(&virtual_address.to_le_bytes());
            header[24..32].copy_from_slice(&virtual_address.to_le_bytes());
            header[32..40].copy_from_slice(&(code.len() as u64).to_le_bytes());
            header[40..48].copy_from_slice(&memory_size.to_le_bytes());
            header[48..56].copy_from_slice(&0x1000_u64.to_le_bytes());
            image.extend_from_slice(&header);
            file_offset += code.len() as u64;
        }
        for (_, code, _) in segments {
            image.extend_from_slice(code);
        }

        let info = parse_guest_image(&image).expect("guest image should parse");
        crate::guest_memory::load_guest_memory_image(&image, &info)
            .expect("guest memory should load")
    }

    fn sample_guest_image_with_program_header(code: &[u8], memory_size: u64) -> GuestMemoryImage {
        sample_guest_image_with_program_headers(&[(TEST_ENTRY, code, memory_size)])
    }

    #[test]
    fn standard_fetch_can_fall_back_across_adjacent_segments() {
        let word = 0x0070_0193_u32;
        let bytes = word.to_le_bytes();
        let image = sample_guest_image_with_program_headers(&[
            (TEST_ENTRY, &bytes[..2], 2),
            (TEST_ENTRY + 2, &bytes[2..], 2),
        ]);
        let memory = GuestMachineMemory::from_image(&image);

        let fetched = memory
            .fetch_instruction(TEST_ENTRY)
            .expect("split standard instruction should fetch");

        assert_eq!(fetched.encoded, RiscvEncodedInstruction::Standard(word));
    }

    #[test]
    fn halfword_fast_path_matches_initialized_and_overlay_bytes() {
        let image = sample_guest_image_with_program_header(
            &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21],
            0x1000,
        );
        let mut memory = GuestMachineMemory::from_image(&image);
        let segment_index = memory
            .segment_index_containing_range(TEST_ENTRY, TEST_ENTRY + 12)
            .expect("segment lookup should evaluate")
            .expect("segment should contain initialized bytes");

        assert_eq!(
            memory.segments[segment_index].read_contiguous_halfword(TEST_ENTRY + 2),
            Some(u16::from_le_bytes([12, 13]))
        );

        memory
            .write_range(TEST_ENTRY + 6, &[90, 91])
            .expect("overlay write should succeed");
        assert_eq!(
            memory.segments[segment_index].read_contiguous_halfword(TEST_ENTRY + 6),
            Some(u16::from_le_bytes([90, 91]))
        );
    }

    #[test]
    fn segment_lookup_uses_address_order_without_accepting_cross_segment_ranges() {
        let image = sample_guest_image_with_program_headers(&[
            (TEST_ENTRY + 0x2000, &[9, 10, 11, 12], 4),
            (TEST_ENTRY, &[1, 2, 3, 4], 4),
            (TEST_ENTRY + 0x1000, &[5, 6, 7, 8], 4),
        ]);
        let memory = GuestMachineMemory::from_image(&image);

        let middle = memory
            .segment_index_containing_range(TEST_ENTRY + 0x1000, TEST_ENTRY + 0x1004)
            .expect("middle segment lookup should evaluate")
            .expect("middle range should be mapped");
        assert_eq!(memory.segments[middle].virtual_address, TEST_ENTRY + 0x1000);

        assert_eq!(
            memory
                .segment_index_containing_range(TEST_ENTRY + 2, TEST_ENTRY + 0x1002)
                .expect("cross-segment lookup should evaluate"),
            None
        );
    }

    #[test]
    fn segment_lookup_cache_is_not_semantic_memory_state() {
        let image = sample_guest_image_with_program_headers(&[
            (TEST_ENTRY, &[1, 2, 3, 4], 4),
            (TEST_ENTRY + 0x1000, &[5, 6, 7, 8], 4),
        ]);
        let memory = GuestMachineMemory::from_image(&image);
        let uncached = GuestMachineMemory::from_image(&image);

        assert_eq!(
            memory
                .segment_index_containing_range(TEST_ENTRY + 0x1000, TEST_ENTRY + 0x1004)
                .expect("cached segment lookup should evaluate"),
            Some(1)
        );
        assert_eq!(memory.last_segment_index.load(Ordering::Relaxed), 1);
        assert_eq!(memory, uncached);

        assert_eq!(
            memory
                .segment_index_containing_range(TEST_ENTRY + 2, TEST_ENTRY + 0x1002)
                .expect("cross-segment lookup should evaluate"),
            None
        );
        assert_eq!(
            memory.last_segment_index.load(Ordering::Relaxed),
            GUEST_MEMORY_SEGMENT_LOOKUP_CACHE_EMPTY
        );
        assert_eq!(memory, uncached);
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
        let dense_blocks = dense.len().div_ceil(GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE);
        assert!(memory.written_overlay_entry_count_for_tests() <= dense_blocks + 1);
    }

    #[test]
    fn unaligned_write_preserves_neighbors_across_overlay_blocks() {
        let initialized = (0..GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE + 8)
            .map(|value| value as u8)
            .collect::<Vec<_>>();
        let image = sample_guest_image_with_program_header(&initialized, 0x1000);
        let mut memory = GuestMachineMemory::from_image(&image);
        let write_address = TEST_ENTRY + GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 2;

        memory
            .write_range(write_address, &[90, 91, 92, 93])
            .expect("cross-block write should succeed");
        let mut bytes = [0_u8; 8];
        memory
            .read_range_into(write_address - 2, &mut bytes)
            .expect("cross-block read should succeed");

        let before_2 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 4) as u8;
        let before_1 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 3) as u8;
        let after_2 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE + 2) as u8;
        let after_3 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE + 3) as u8;
        assert_eq!(
            bytes,
            [before_2, before_1, 90, 91, 92, 93, after_2, after_3]
        );
    }

    #[test]
    fn scalar_le_write_preserves_neighbors_in_one_overlay_block() {
        let image = sample_guest_image_with_program_header(
            &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21],
            0x1000,
        );
        let mut memory = GuestMachineMemory::from_image(&image);

        memory
            .write_u64_le::<4>(TEST_ENTRY + 2, 0xaabb_ccdd)
            .expect("scalar write should succeed");
        let mut bytes = [0_u8; 12];
        memory
            .read_range_into(TEST_ENTRY, &mut bytes)
            .expect("scalar write result should be readable");

        assert_eq!(
            bytes,
            [10, 11, 0xdd, 0xcc, 0xbb, 0xaa, 16, 17, 18, 19, 20, 21]
        );
    }

    #[test]
    fn scalar_le_write_preserves_neighbors_across_overlay_blocks() {
        let initialized = (0..GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE + 8)
            .map(|value| value as u8)
            .collect::<Vec<_>>();
        let image = sample_guest_image_with_program_header(&initialized, 0x1000);
        let mut memory = GuestMachineMemory::from_image(&image);
        let write_address = TEST_ENTRY + GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 2;

        memory
            .write_u64_le::<4>(write_address, 0xaabb_ccdd)
            .expect("cross-block scalar write should succeed");
        let mut bytes = [0_u8; 8];
        memory
            .read_range_into(write_address - 2, &mut bytes)
            .expect("cross-block scalar write result should be readable");

        let before_2 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 4) as u8;
        let before_1 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 3) as u8;
        let after_2 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE + 2) as u8;
        let after_3 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE + 3) as u8;
        assert_eq!(
            bytes,
            [before_2, before_1, 0xdd, 0xcc, 0xbb, 0xaa, after_2, after_3,]
        );
    }

    #[test]
    fn read_can_span_written_and_unwritten_blocks() {
        let initialized = (0..GUEST_MEMORY_OVERLAY_BLOCK_SIZE_USIZE + 12)
            .map(|value| value as u8)
            .collect::<Vec<_>>();
        let image = sample_guest_image_with_program_header(&initialized, 0x1000);
        let mut memory = GuestMachineMemory::from_image(&image);
        let write_address = TEST_ENTRY + GUEST_MEMORY_OVERLAY_BLOCK_SIZE;

        memory
            .write_range(write_address, &[80, 81, 82])
            .expect("write should succeed");
        let mut bytes = [0_u8; 8];
        memory
            .read_range_into(write_address - 4, &mut bytes)
            .expect("mixed read should succeed");

        let before_4 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 4) as u8;
        let before_3 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 3) as u8;
        let before_2 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 2) as u8;
        let before_1 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE - 1) as u8;
        let after_3 = (GUEST_MEMORY_OVERLAY_BLOCK_SIZE + 3) as u8;
        assert_eq!(
            bytes,
            [before_4, before_3, before_2, before_1, 80, 81, 82, after_3,]
        );
    }

    #[test]
    fn overlay_snapshot_restores_tail_block_without_extending_segment() {
        let image = sample_guest_image_with_program_header(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10);
        let mut memory = GuestMachineMemory::from_image(&image);
        memory
            .write_range(TEST_ENTRY + 9, &[90])
            .expect("tail byte should be writable");
        let snapshot = GuestMachineMemoryOverlaySnapshot::capture(&memory);

        let mut restored = GuestMachineMemory::from_image(&image);
        snapshot
            .restore_into(&mut restored)
            .expect("tail overlay block should restore");
        let mut bytes = [0_u8; 10];
        restored
            .read_range_into(TEST_ENTRY, &mut bytes)
            .expect("restored segment should be readable");

        assert_eq!(bytes, [1, 2, 3, 4, 5, 6, 7, 8, 9, 90]);
    }

    #[test]
    fn small_le_reads_match_initialized_zero_and_overlay_bytes() {
        let image = sample_guest_image_with_program_header(
            &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21],
            0x1000,
        );
        let mut memory = GuestMachineMemory::from_image(&image);

        assert_eq!(
            memory
                .read_u64_le(TEST_ENTRY + 2, 4)
                .expect("initialized read should succeed"),
            u64::from(u32::from_le_bytes([12, 13, 14, 15]))
        );
        assert_eq!(
            memory
                .read_u64_le(TEST_ENTRY + 10, 4)
                .expect("zero tail read should succeed"),
            u64::from(u32::from_le_bytes([20, 21, 0, 0]))
        );

        memory
            .write_range(TEST_ENTRY + 6, &[90, 91, 92, 93])
            .expect("overlay write should succeed");
        assert_eq!(
            memory
                .read_u64_le(TEST_ENTRY + 6, 4)
                .expect("overlay read should succeed"),
            u64::from(u32::from_le_bytes([90, 91, 92, 93]))
        );
        assert_eq!(
            memory
                .read_u64_le(TEST_ENTRY + 4, 8)
                .expect("mixed read should succeed"),
            u64::from_le_bytes([14, 15, 90, 91, 92, 93, 20, 21])
        );
    }

    #[test]
    fn common_width_le_bytes_use_native_decode() {
        assert_eq!(common_width_le_bytes_to_u64(&[0x12]), Some(0x12));
        assert_eq!(common_width_le_bytes_to_u64(&[0x34, 0x12]), Some(0x1234));
        assert_eq!(
            common_width_le_bytes_to_u64(&[0x78, 0x56, 0x34, 0x12]),
            Some(0x1234_5678)
        );
        assert_eq!(
            common_width_le_bytes_to_u64(&[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]),
            Some(0x0123_4567_89ab_cdef)
        );
        assert_eq!(common_width_le_bytes_to_u64(&[1, 2, 3]), None);
    }

    #[test]
    fn small_le_reads_reject_widths_larger_than_u64() {
        let image = sample_guest_image_with_program_header(&[1, 2, 3, 4], 0x1000);
        let memory = GuestMachineMemory::from_image(&image);

        let err = memory
            .read_u64_le(TEST_ENTRY, 9)
            .expect_err("oversized little-endian read should fail");

        assert_eq!(
            err,
            GuestMemoryError::AddressRangeOverflow {
                address: TEST_ENTRY,
                byte_len: 9
            }
        );
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

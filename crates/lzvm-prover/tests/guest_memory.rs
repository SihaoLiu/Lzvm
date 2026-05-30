use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_prover::guest_memory::{load_guest_memory_image, GuestMemoryError, GuestMemoryImage};

fn sample_guest_image() -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x8000_0000_u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_guest_image_with_program_headers(program_headers: &[[u8; 56]]) -> Vec<u8> {
    let mut bytes = sample_guest_image();
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(program_headers.len() as u16).to_le_bytes());
    for header in program_headers {
        bytes.extend_from_slice(header);
    }
    bytes
}

#[derive(Debug, Clone, Copy)]
struct ProgramHeaderFixture {
    kind: u32,
    flags: u32,
    file_offset: u64,
    virtual_address: u64,
    physical_address: u64,
    file_size: u64,
    memory_size: u64,
    align: u64,
}

fn program_header(header: ProgramHeaderFixture) -> [u8; 56] {
    let mut bytes = [0_u8; 56];
    bytes[0..4].copy_from_slice(&header.kind.to_le_bytes());
    bytes[4..8].copy_from_slice(&header.flags.to_le_bytes());
    bytes[8..16].copy_from_slice(&header.file_offset.to_le_bytes());
    bytes[16..24].copy_from_slice(&header.virtual_address.to_le_bytes());
    bytes[24..32].copy_from_slice(&header.physical_address.to_le_bytes());
    bytes[32..40].copy_from_slice(&header.file_size.to_le_bytes());
    bytes[40..48].copy_from_slice(&header.memory_size.to_le_bytes());
    bytes[48..56].copy_from_slice(&header.align.to_le_bytes());
    bytes
}

fn read_guest_memory_range(
    memory: &GuestMemoryImage,
    address: u64,
    byte_len: usize,
) -> Result<Vec<u8>, GuestMemoryError> {
    let mut bytes = vec![0; byte_len];
    memory.read_range_into(address, &mut bytes)?;
    Ok(bytes)
}

#[test]
fn loads_guest_memory_segments_and_zero_fills_bss() {
    let first = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 176,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 8,
        align: 0x1000,
    });
    let second = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 6,
        file_offset: 180,
        virtual_address: 0x8000_1000,
        physical_address: 0x8000_1000,
        file_size: 3,
        memory_size: 3,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[first, second]);
    image.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7]);
    let info = parse_guest_image(&image).expect("guest image should parse");

    let memory = load_guest_memory_image(&image, &info).expect("guest memory should load");

    assert_eq!(memory.entry_address(), 0x8000_0000);
    assert_eq!(memory.segment_count(), 2);
    assert_eq!(
        read_guest_memory_range(&memory, 0x8000_0000, 8).expect("first segment should read"),
        vec![1, 2, 3, 4, 0, 0, 0, 0]
    );
    assert_eq!(
        read_guest_memory_range(&memory, 0x8000_1000, 3).expect("second segment should read"),
        vec![5, 6, 7]
    );
    let mut read_buffer = [9_u8; 8];
    memory
        .read_range_into(0x8000_0000, &mut read_buffer)
        .expect("first segment should read into caller buffer");
    assert_eq!(read_buffer, [1, 2, 3, 4, 0, 0, 0, 0]);
    assert!(matches!(
        read_guest_memory_range(&memory, 0x8000_0800, 1),
        Err(GuestMemoryError::AddressNotMapped {
            address: 0x8000_0800,
            byte_len: 1,
        })
    ));
    assert!(matches!(
        read_guest_memory_range(&memory, u64::MAX - 1, 8),
        Err(GuestMemoryError::AddressRangeOverflow {
            address,
            byte_len: 8,
        }) if address == u64::MAX - 1
    ));
}

#[test]
fn rejects_overlapping_guest_memory_segments() {
    let first = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 176,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 8,
        align: 0x1000,
    });
    let second = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 6,
        file_offset: 180,
        virtual_address: 0x8000_0004,
        physical_address: 0x8000_0004,
        file_size: 4,
        memory_size: 4,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[first, second]);
    image.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    let info = parse_guest_image(&image).expect("guest image should parse");

    assert!(matches!(
        load_guest_memory_image(&image, &info),
        Err(GuestMemoryError::OverlappingSegments {
            first_program_header_index: 0,
            second_program_header_index: 1,
        })
    ));
}

#[test]
fn rejects_overlapping_sparse_tail_before_loading_segment_bytes() {
    let first = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 176,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 1_u64 << 40,
        align: 0x1000,
    });
    let second = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 6,
        file_offset: 180,
        virtual_address: 0x8000_1000,
        physical_address: 0x8000_1000,
        file_size: 4,
        memory_size: 4,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[first, second]);
    image.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    let info = parse_guest_image(&image).expect("guest image should parse");

    assert!(matches!(
        load_guest_memory_image(&image, &info),
        Err(GuestMemoryError::OverlappingSegments {
            first_program_header_index: 0,
            second_program_header_index: 1,
        })
    ));
}

#[test]
fn rejects_malformed_guest_memory_segment_metadata() {
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 4,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&[1, 2, 3, 4]);
    let mut info = parse_guest_image(&image).expect("guest image should parse");
    info.load_segments[0].file_offset = u64::MAX - 3;
    info.load_segments[0].file_size = 8;

    assert!(matches!(
        load_guest_memory_image(&image, &info),
        Err(GuestMemoryError::SegmentFileRangeOutOfBounds {
            program_header_index: 0,
            file_offset,
            file_size: 8,
            byte_len: 124,
        }) if file_offset == u64::MAX - 3
    ));
}

#[test]
fn rejects_guest_memory_segment_file_size_larger_than_memory_size() {
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 4,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&[1, 2, 3, 4]);
    let mut info = parse_guest_image(&image).expect("guest image should parse");
    info.load_segments[0].memory_size = 3;

    assert!(matches!(
        load_guest_memory_image(&image, &info),
        Err(GuestMemoryError::SegmentFileSizeExceedsMemorySize {
            program_header_index: 0,
            file_size: 4,
            memory_size: 3,
        })
    ));
}

#[test]
fn rejects_guest_memory_segment_virtual_range_overflow() {
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 4,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&[1, 2, 3, 4]);
    let mut info = parse_guest_image(&image).expect("guest image should parse");
    info.load_segments[0].virtual_address = u64::MAX - 1;

    assert!(matches!(
        load_guest_memory_image(&image, &info),
        Err(GuestMemoryError::SegmentMemoryRangeOverflow {
            program_header_index: 0,
            virtual_address,
            memory_size: 4,
        }) if virtual_address == u64::MAX - 1
    ));
}

#[test]
fn keeps_zero_tail_sparse_for_large_guest_memory_segments() {
    let header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 4,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&[1, 2, 3, 4]);
    let mut info = parse_guest_image(&image).expect("guest image should parse");
    info.load_segments[0].memory_size = 1_u64 << 40;

    let memory = load_guest_memory_image(&image, &info).expect("guest memory should load");
    let segment = &memory.segments()[0];

    assert_eq!(segment.memory_size(), 1_u64 << 40);
    assert_eq!(segment.initialized_bytes(), &[1, 2, 3, 4]);
    assert_eq!(
        read_guest_memory_range(&memory, 0x8000_0000 + (1_u64 << 40) - 2, 2)
            .expect("sparse tail should read as zero"),
        vec![0, 0]
    );
}

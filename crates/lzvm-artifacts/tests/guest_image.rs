use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::guest_image::{
    parse_guest_image, read_guest_image_file, ElfClass, ElfEndian, GuestImageError,
    GuestImageLoadSegment,
};

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

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-guest-image-{}-{name}", std::process::id()))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::write(path, bytes).expect("fixture should be written");
}

#[test]
fn parses_guest_image_headers() {
    let image = sample_guest_image();

    let info = parse_guest_image(&image).expect("guest image should parse");

    assert_eq!(info.byte_len, 64);
    assert_eq!(info.elf_class, ElfClass::Elf64);
    assert_eq!(info.endian, ElfEndian::Little);
    assert_eq!(info.machine, 243);
    assert_eq!(info.entry, 0x8000_0000);
    assert_ne!(info.digest, [0_u8; 32]);
    assert!(info.load_segments.is_empty());
}

#[test]
fn reads_guest_image_from_file() {
    let path = temp_file("valid.elf");
    write_bytes(&path, sample_guest_image());

    let info = read_guest_image_file(&path).expect("guest image should parse from file");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(info.byte_len, 64);
    assert_eq!(info.machine, 243);
}

#[test]
fn rejects_non_guest_image_bytes() {
    assert!(matches!(
        parse_guest_image(b"not-an-elf"),
        Err(GuestImageError::InvalidMagic)
    ));
}

#[test]
fn rejects_unsupported_image_class() {
    let mut image = sample_guest_image();
    image[4] = 1;

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::UnsupportedClass { class: 1 })
    ));
}

#[test]
fn rejects_unsupported_image_endian_marker() {
    let mut image = sample_guest_image();
    image[5] = 2;

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::UnsupportedEndian { endian: 2 })
    ));
}

#[test]
fn rejects_truncated_headers_after_magic() {
    let image = &sample_guest_image()[..32];

    assert!(matches!(
        parse_guest_image(image),
        Err(GuestImageError::HeaderTooSmall {
            actual: 32,
            minimum: 64
        })
    ));
}

#[test]
fn parses_guest_image_load_segments() {
    let load_header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 5,
        file_offset: 176,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 4,
        memory_size: 16,
        align: 0x1000,
    });
    let note_header = program_header(ProgramHeaderFixture {
        kind: 4,
        flags: 0,
        file_offset: 0,
        virtual_address: 0,
        physical_address: 0,
        file_size: 0,
        memory_size: 0,
        align: 0,
    });
    let mut image = sample_guest_image_with_program_headers(&[load_header, note_header]);
    image.extend_from_slice(&[1, 2, 3, 4]);

    let info = parse_guest_image(&image).expect("guest image should parse");

    assert_eq!(
        info.load_segments,
        vec![GuestImageLoadSegment {
            program_header_index: 0,
            flags: 5,
            file_offset: 176,
            virtual_address: 0x8000_0000,
            physical_address: 0x8000_0000,
            file_size: 4,
            memory_size: 16,
            align: 0x1000,
        }]
    );
}

#[test]
fn rejects_unsupported_program_header_entry_size() {
    let mut image = sample_guest_image();
    image[32..40].copy_from_slice(&64_u64.to_le_bytes());
    image[54..56].copy_from_slice(&48_u16.to_le_bytes());
    image[56..58].copy_from_slice(&1_u16.to_le_bytes());

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::UnsupportedProgramHeaderEntrySize { entry_size: 48 })
    ));
}

#[test]
fn rejects_program_header_table_out_of_bounds() {
    let mut image = sample_guest_image();
    image[32..40].copy_from_slice(&64_u64.to_le_bytes());
    image[54..56].copy_from_slice(&56_u16.to_le_bytes());
    image[56..58].copy_from_slice(&1_u16.to_le_bytes());

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::ProgramHeaderTableOutOfBounds {
            offset: 64,
            entry_size: 56,
            count: 1,
            byte_len: 64,
        })
    ));
}

#[test]
fn rejects_program_header_table_end_overflow() {
    let mut image = sample_guest_image();
    image[32..40].copy_from_slice(&(u64::MAX - 1).to_le_bytes());
    image[54..56].copy_from_slice(&56_u16.to_le_bytes());
    image[56..58].copy_from_slice(&1_u16.to_le_bytes());

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::ProgramHeaderTableOutOfBounds {
            offset,
            entry_size: 56,
            count: 1,
            byte_len: 64,
        }) if offset == u64::MAX - 1
    ));
}

#[test]
fn rejects_load_segment_file_range_out_of_bounds() {
    let load_header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 4,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 8,
        memory_size: 8,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[load_header]);
    image.extend_from_slice(&[1, 2, 3, 4]);

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::LoadSegmentFileRangeOutOfBounds {
            program_header_index: 0,
            file_offset: 120,
            file_size: 8,
            byte_len: 124,
        })
    ));
}

#[test]
fn rejects_load_segment_file_range_overflow() {
    let load_header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 4,
        file_offset: u64::MAX - 3,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 8,
        memory_size: 8,
        align: 0x1000,
    });
    let image = sample_guest_image_with_program_headers(&[load_header]);

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::LoadSegmentFileRangeOutOfBounds {
            program_header_index: 0,
            file_offset,
            file_size: 8,
            byte_len: 120,
        }) if file_offset == u64::MAX - 3
    ));
}

#[test]
fn rejects_load_segment_file_size_larger_than_memory_size() {
    let load_header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 4,
        file_offset: 120,
        virtual_address: 0x8000_0000,
        physical_address: 0x8000_0000,
        file_size: 8,
        memory_size: 4,
        align: 0x1000,
    });
    let mut image = sample_guest_image_with_program_headers(&[load_header]);
    image.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::LoadSegmentFileSizeExceedsMemorySize {
            program_header_index: 0,
            file_size: 8,
            memory_size: 4,
        })
    ));
}

#[test]
fn rejects_load_segment_memory_range_overflow() {
    let load_header = program_header(ProgramHeaderFixture {
        kind: 1,
        flags: 4,
        file_offset: 120,
        virtual_address: u64::MAX - 3,
        physical_address: u64::MAX - 3,
        file_size: 0,
        memory_size: 8,
        align: 0x1000,
    });
    let image = sample_guest_image_with_program_headers(&[load_header]);

    assert!(matches!(
        parse_guest_image(&image),
        Err(GuestImageError::LoadSegmentMemoryRangeOverflow {
            program_header_index: 0,
            virtual_address: vaddr,
            memory_size: 8,
        }) if vaddr == u64::MAX - 3
    ));
}

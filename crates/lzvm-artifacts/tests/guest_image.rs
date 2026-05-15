use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::guest_image::{
    parse_guest_image, read_guest_image_file, ElfClass, ElfEndian, GuestImageError,
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

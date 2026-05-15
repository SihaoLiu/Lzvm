use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::witness_library::{
    parse_witness_library, read_witness_library_file, ElfClass, ElfEndian, LibraryKind,
    WitnessLibraryError,
};

fn sample_witness_library() -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&3_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&62_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-witness-library-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::write(path, bytes).expect("fixture should be written");
}

#[test]
fn parses_witness_library_headers() {
    let library = sample_witness_library();

    let info = parse_witness_library(&library).expect("witness library should parse");

    assert_eq!(info.byte_len, 64);
    assert_eq!(info.elf_class, ElfClass::Elf64);
    assert_eq!(info.endian, ElfEndian::Little);
    assert_eq!(info.kind, LibraryKind::Dynamic);
    assert_eq!(info.machine, 62);
    assert_ne!(info.digest, [0_u8; 32]);
}

#[test]
fn reads_witness_library_from_file() {
    let path = temp_file("valid.so");
    write_bytes(&path, sample_witness_library());

    let info = read_witness_library_file(&path).expect("witness library should parse from file");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(info.byte_len, 64);
    assert_eq!(info.machine, 62);
}

#[test]
fn rejects_non_witness_library_bytes() {
    assert!(matches!(
        parse_witness_library(b"not-an-elf"),
        Err(WitnessLibraryError::InvalidMagic)
    ));
}

#[test]
fn rejects_non_dynamic_elf_files() {
    let mut library = sample_witness_library();
    library[16..18].copy_from_slice(&2_u16.to_le_bytes());

    assert!(matches!(
        parse_witness_library(&library),
        Err(WitnessLibraryError::UnsupportedObjectType { object_type: 2 })
    ));
}

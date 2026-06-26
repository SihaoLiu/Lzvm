use lzvm_artifacts::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const HEADER_BYTES: usize = 4 + 4 + 4;
const SECTION_HEADER_BYTES: usize = 4 + 8;

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn sample_file() -> Vec<u8> {
    let mut file = Vec::new();
    file.extend_from_slice(b"abcd");
    push_u32(&mut file, 1);
    push_u32(&mut file, 2);
    push_u32(&mut file, 7);
    push_u64(&mut file, 3);
    file.extend_from_slice(b"one");
    push_u32(&mut file, 9);
    push_u64(&mut file, 4);
    file.extend_from_slice(b"four");
    file
}

#[test]
fn parses_sectioned_files_with_multiple_sections() {
    let parsed = parse_sectioned_file(&sample_file(), *b"abcd", 1).expect("fixture should parse");

    assert_eq!(parsed.kind, *b"abcd");
    assert_eq!(parsed.version, 1);
    assert_eq!(parsed.sections.len(), 2);
    assert_eq!(parsed.sections[0].id, 7);
    assert_eq!(parsed.sections[0].data, b"one");
    assert_eq!(parsed.sections[1].id, 9);
    assert_eq!(parsed.sections[1].data, b"four");
}

#[test]
fn encodes_sectioned_files_to_the_canonical_binary_form() {
    let value = SectionedFile {
        kind: *b"abcd",
        version: 1,
        sections: vec![
            SectionedSection {
                id: 7,
                data: b"one".to_vec(),
            },
            SectionedSection {
                id: 9,
                data: b"four".to_vec(),
            },
        ],
    };

    assert_eq!(
        encode_sectioned_file(&value).expect("fixture should encode"),
        sample_file()
    );
}

#[test]
fn rejects_unexpected_file_kind() {
    assert!(matches!(
        parse_sectioned_file(&sample_file(), *b"wxyz", 1),
        Err(SectionedError::InvalidKind { .. })
    ));
}

#[test]
fn rejects_truncated_section_data() {
    let mut bytes = sample_file();
    bytes.truncate(bytes.len() - 1);

    assert!(matches!(
        parse_sectioned_file(&bytes, *b"abcd", 1),
        Err(SectionedError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_section_count_that_exceeds_remaining_section_headers() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"abcd");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_sectioned_file(&bytes, *b"abcd", 1),
        Err(SectionedError::UnexpectedEof {
            offset: HEADER_BYTES,
            needed: SECTION_HEADER_BYTES,
            available: 0
        })
    ));
}

#[test]
fn rejects_partial_section_headers() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"abcd");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 7);

    assert!(matches!(
        parse_sectioned_file(&bytes, *b"abcd", 1),
        Err(SectionedError::UnexpectedEof {
            offset: HEADER_BYTES,
            needed: SECTION_HEADER_BYTES,
            available: 4
        })
    ));
}

use lzvm_artifacts::framed_stdin::{parse_framed_stdin_chunks, FramedStdinChunk, FramedStdinError};

fn framed_chunk(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&(data.len() as u64).to_le_bytes());
    encoded.extend_from_slice(data);
    let padding = (8 - ((8 + data.len()) % 8)) % 8;
    encoded.extend(std::iter::repeat_n(0, padding));
    encoded
}

#[test]
fn parses_aligned_chunks() {
    let mut encoded = framed_chunk(b"abc");
    encoded.extend_from_slice(&framed_chunk(b"12345678"));

    let chunks = parse_framed_stdin_chunks(&encoded).expect("chunks should parse");

    assert_eq!(
        chunks,
        vec![
            FramedStdinChunk {
                offset: 0,
                payload_offset: 8,
                payload_len: 3,
                padding_len: 5,
                data: b"abc".to_vec(),
            },
            FramedStdinChunk {
                offset: 16,
                payload_offset: 24,
                payload_len: 8,
                padding_len: 0,
                data: b"12345678".to_vec(),
            },
        ]
    );
}

#[test]
fn rejects_truncated_payloads() {
    let encoded = [5_u8, 0, 0, 0, 0, 0, 0, 0, b'a', b'b'];

    assert_eq!(
        parse_framed_stdin_chunks(&encoded).expect_err("truncated payload should reject"),
        FramedStdinError::TruncatedChunk {
            chunk_index: 0,
            expected: 16,
            remaining: 10,
        }
    );
}

#[test]
fn rejects_nonzero_padding() {
    let mut encoded = framed_chunk(b"abc");
    *encoded.last_mut().expect("padding byte should exist") = 1;

    assert_eq!(
        parse_framed_stdin_chunks(&encoded).expect_err("nonzero padding should reject"),
        FramedStdinError::NonZeroPadding {
            chunk_index: 0,
            offset: 15,
        }
    );
}

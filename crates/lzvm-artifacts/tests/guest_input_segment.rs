use lzvm_artifacts::framed_stdin::FramedStdinError;
use lzvm_artifacts::guest_input_segment::{
    encode_framed_guest_input_segment, parse_framed_guest_input_segment,
};

fn framed_chunk(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&(data.len() as u64).to_le_bytes());
    encoded.extend_from_slice(data);
    let padding = (8 - ((8 + data.len()) % 8)) % 8;
    encoded.extend(std::iter::repeat_n(0, padding));
    encoded
}

#[test]
fn encodes_nonempty_framed_guest_input_exactly() {
    let input = framed_chunk(b"abc");

    let segment = encode_framed_guest_input_segment(&input).expect("segment should encode");
    let chunks = parse_framed_guest_input_segment(&segment).expect("segment should parse");

    assert_eq!(segment, input);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data, b"abc");
}

#[test]
fn rejects_empty_framed_guest_input_segment() {
    assert_eq!(
        encode_framed_guest_input_segment(&[]).expect_err("empty segment should reject"),
        FramedStdinError::EmptyInput
    );
    assert_eq!(
        parse_framed_guest_input_segment(&[]).expect_err("empty segment should reject"),
        FramedStdinError::EmptyInput
    );
}

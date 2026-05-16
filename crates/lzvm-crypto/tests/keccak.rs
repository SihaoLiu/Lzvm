use lzvm_crypto::keccak256;

fn decode_hex_32(input: &str) -> [u8; 32] {
    assert_eq!(input.len(), 64);
    let mut out = [0_u8; 32];
    for (index, chunk) in input.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).expect("hex should be ascii");
        out[index] = u8::from_str_radix(text, 16).expect("hex should parse");
    }
    out
}

#[test]
fn keccak256_hashes_empty_input() {
    let expected =
        decode_hex_32("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");

    assert_eq!(keccak256(b""), expected);
}

#[test]
fn keccak256_hashes_short_input() {
    let expected =
        decode_hex_32("4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45");

    assert_eq!(keccak256(b"abc"), expected);
}

#[test]
fn keccak256_hashes_long_input() {
    let input = vec![b'a'; 200];
    let expected =
        decode_hex_32("96ea54061def936c4be90b518992fdc6f12f535068a256229aca54267b4d084d");

    assert_eq!(keccak256(&input), expected);
}

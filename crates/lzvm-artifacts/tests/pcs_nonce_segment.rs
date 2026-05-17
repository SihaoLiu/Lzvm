use lzvm_artifacts::pcs_nonce_segment::{
    encode_pcs_query_nonce_segment, parse_pcs_query_nonce_segment, PcsQueryNonceSegment,
    PcsQueryNonceSegmentError,
};

#[test]
fn encodes_and_parses_pcs_query_nonce_segments() {
    let segment = PcsQueryNonceSegment {
        nonce: 1_234_567_890,
    };

    let encoded = encode_pcs_query_nonce_segment(&segment).expect("nonce segment should encode");
    let parsed = parse_pcs_query_nonce_segment(&encoded).expect("nonce segment should parse");

    assert_eq!(&encoded[0..4], b"qns0");
    assert_eq!(parsed, segment);
}

#[test]
fn rejects_unsupported_pcs_query_nonce_segment_versions() {
    let segment = PcsQueryNonceSegment { nonce: 7 };
    let mut encoded =
        encode_pcs_query_nonce_segment(&segment).expect("nonce segment should encode");
    encoded[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert!(matches!(
        parse_pcs_query_nonce_segment(&encoded),
        Err(PcsQueryNonceSegmentError::UnsupportedVersion { version: 2 })
    ));
}

#[test]
fn rejects_truncated_pcs_query_nonce_segments() {
    let result = parse_pcs_query_nonce_segment(b"qns0\x01\0\0\0");

    assert!(matches!(
        result,
        Err(PcsQueryNonceSegmentError::UnexpectedEof {
            needed: 16,
            available: 8
        })
    ));
}

#[test]
fn rejects_trailing_pcs_query_nonce_bytes() {
    let segment = PcsQueryNonceSegment { nonce: 7 };
    let mut encoded =
        encode_pcs_query_nonce_segment(&segment).expect("nonce segment should encode");
    encoded.push(0);

    assert!(matches!(
        parse_pcs_query_nonce_segment(&encoded),
        Err(PcsQueryNonceSegmentError::TrailingBytes { trailing: 1 })
    ));
}

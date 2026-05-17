use lzvm_artifacts::pcs_proof_values_segment::{
    encode_pcs_proof_values_segment, parse_pcs_proof_values_segment, PcsProofValuesSegment,
    PcsProofValuesSegmentError,
};

fn segment_header(value_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"pvs0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, value_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn encodes_and_parses_pcs_proof_values_segments() {
    let segment = PcsProofValuesSegment {
        values: vec![[1, 2, 3], [4, 5, 6]],
    };

    let bytes = encode_pcs_proof_values_segment(&segment).expect("segment should encode");
    let parsed = parse_pcs_proof_values_segment(&bytes).expect("segment should parse");

    assert_eq!(parsed, segment);
}

#[test]
fn rejects_unsupported_pcs_proof_values_segment_versions() {
    let segment = PcsProofValuesSegment {
        values: vec![[1, 2, 3]],
    };
    let mut encoded = encode_pcs_proof_values_segment(&segment).expect("segment should encode");
    encoded[4..8].copy_from_slice(&2_u32.to_le_bytes());

    assert!(matches!(
        parse_pcs_proof_values_segment(&encoded),
        Err(PcsProofValuesSegmentError::UnsupportedVersion { version: 2 })
    ));
}

#[test]
fn rejects_empty_pcs_proof_values_segments() {
    let result = encode_pcs_proof_values_segment(&PcsProofValuesSegment { values: Vec::new() });

    assert!(matches!(
        result,
        Err(PcsProofValuesSegmentError::EmptyValues)
    ));
}

#[test]
fn rejects_truncated_pcs_proof_values_segments() {
    let segment = PcsProofValuesSegment {
        values: vec![[7, 8, 9]],
    };
    let mut bytes = encode_pcs_proof_values_segment(&segment).expect("segment should encode");
    bytes.pop();

    assert!(matches!(
        parse_pcs_proof_values_segment(&bytes),
        Err(PcsProofValuesSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_extensions() {
    assert!(matches!(
        parse_pcs_proof_values_segment(&segment_header(1)),
        Err(PcsProofValuesSegmentError::LengthOverflow)
    ));
}

#[test]
fn rejects_trailing_pcs_proof_values_bytes() {
    let segment = PcsProofValuesSegment {
        values: vec![[10, 11, 12]],
    };
    let mut bytes = encode_pcs_proof_values_segment(&segment).expect("segment should encode");
    bytes.push(1);

    assert!(matches!(
        parse_pcs_proof_values_segment(&bytes),
        Err(PcsProofValuesSegmentError::TrailingBytes { trailing: 1 })
    ));
}

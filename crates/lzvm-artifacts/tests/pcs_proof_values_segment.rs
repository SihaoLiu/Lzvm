use lzvm_artifacts::pcs_proof_values_segment::{
    encode_pcs_proof_values_segment, parse_pcs_proof_values_segment, PcsProofValuesSegment,
    PcsProofValuesSegmentError,
};

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
        Err(PcsProofValuesSegmentError::UnexpectedEof { .. })
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

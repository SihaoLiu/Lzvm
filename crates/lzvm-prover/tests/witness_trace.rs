use lzvm_field::{Felt, MODULUS};
use lzvm_prover::witness_trace::{parse_witness_trace, WitnessTraceError};

fn encode_values(values: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 8);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

#[test]
fn parses_row_major_witness_trace_values() {
    let bytes = encode_values(&[1, 2, 3, 4, 5, 6]);

    let trace = parse_witness_trace(&bytes, 2, 3).expect("trace should parse");

    assert_eq!(trace.row_count(), 2);
    assert_eq!(trace.column_count(), 3);
    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(1).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 2),
        Some(Felt::from_canonical(3).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 0),
        Some(Felt::from_canonical(4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 2),
        Some(Felt::from_canonical(6).expect("canonical"))
    );
    assert_eq!(trace.value(2, 0), None);
    assert_eq!(trace.value(0, 3), None);
}

#[test]
fn rejects_unaligned_witness_trace_bytes() {
    assert!(matches!(
        parse_witness_trace(&[1, 2, 3], 1, 1),
        Err(WitnessTraceError::ByteLengthNotElementAligned { byte_len: 3 })
    ));
}

#[test]
fn rejects_witness_trace_dimension_mismatch() {
    let bytes = encode_values(&[1, 2, 3]);

    assert!(matches!(
        parse_witness_trace(&bytes, 2, 2),
        Err(WitnessTraceError::ElementCountMismatch {
            expected: 4,
            found: 3
        })
    ));
}

#[test]
fn rejects_zero_witness_trace_dimensions() {
    let bytes = encode_values(&[1]);

    assert!(matches!(
        parse_witness_trace(&bytes, 0, 1),
        Err(WitnessTraceError::ZeroRows)
    ));
    assert!(matches!(
        parse_witness_trace(&bytes, 1, 0),
        Err(WitnessTraceError::ZeroColumns)
    ));
}

#[test]
fn rejects_witness_trace_element_count_overflow() {
    assert!(matches!(
        parse_witness_trace(&[], usize::MAX, 2),
        Err(WitnessTraceError::ElementCountOverflow)
    ));
}

#[test]
fn rejects_non_canonical_witness_trace_elements() {
    let bytes = encode_values(&[1, MODULUS]);

    assert!(matches!(
        parse_witness_trace(&bytes, 1, 2),
        Err(WitnessTraceError::NonCanonicalElement {
            index: 1,
            value: MODULUS
        })
    ));
}

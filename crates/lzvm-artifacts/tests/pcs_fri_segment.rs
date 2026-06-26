use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, parse_pcs_fri_opening_segment, PcsFriOpeningLayerSegment,
    PcsFriOpeningLevelSegment, PcsFriOpeningQuerySegment, PcsFriOpeningSegment,
    PcsFriOpeningSegmentError, PcsFriOpeningUnitSegment,
};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const FIRST_FINAL_POLYNOMIAL_OFFSET: usize = 12 + 12;
const FIRST_LAYER_OFFSET: usize = FIRST_FINAL_POLYNOMIAL_OFFSET + 2 * 3 * 8;
const FIRST_LAYER_ROOT_OFFSET: usize = FIRST_LAYER_OFFSET + 4;
const FIRST_LAST_LEVEL_OFFSET: usize = FIRST_LAYER_OFFSET + 4 + 4 * 8 + 4 + 4;
const FIRST_QUERY_OFFSET: usize = FIRST_LAST_LEVEL_OFFSET + 4 * 8;
const FIRST_QUERY_VALUES_OFFSET: usize = FIRST_QUERY_OFFSET + 8 + 4 + 4;
const FIRST_SIBLING_LEVEL_OFFSET: usize = FIRST_QUERY_VALUES_OFFSET + 2 * 3 * 8;
const FIRST_SIBLING_OFFSET: usize = FIRST_SIBLING_LEVEL_OFFSET + 4;

fn sample_segment() -> PcsFriOpeningSegment {
    PcsFriOpeningSegment {
        units: vec![PcsFriOpeningUnitSegment {
            unit_index: 0,
            trace_instance_index: 0,
            layers: vec![PcsFriOpeningLayerSegment {
                layer_index: 0,
                root: [1, 2, 3, 4],
                last_level: vec![[5, 6, 7, 8]],
                queries: vec![PcsFriOpeningQuerySegment {
                    row_index: 3,
                    values: vec![[11, 12, 13], [21, 22, 23]],
                    siblings: vec![PcsFriOpeningLevelSegment {
                        siblings: vec![[31, 32, 33, 34]],
                    }],
                }],
            }],
            final_polynomial: vec![[41, 42, 43], [51, 52, 53]],
        }],
    }
}

fn segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"fos0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, unit_count);
    bytes
}

fn v2_segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"fos0");
    push_u32(&mut bytes, 2);
    push_u32(&mut bytes, unit_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_digest(out: &mut Vec<u8>) {
    push_u64(out, 1);
    push_u64(out, 2);
    push_u64(out, 3);
    push_u64(out, 4);
}

fn push_extension(out: &mut Vec<u8>) {
    push_u64(out, 11);
    push_u64(out, 12);
    push_u64(out, 13);
}

#[test]
fn encodes_and_parses_pcs_fri_opening_segments() {
    let encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    let parsed = parse_pcs_fri_opening_segment(&encoded).expect("FRI segment should parse");

    assert_eq!(&encoded[0..4], b"fos0");
    assert_eq!(parsed, sample_segment());
}

#[test]
fn encodes_and_parses_trace_instance_pcs_fri_opening_segments() {
    let mut segment = sample_segment();
    let mut later = segment.units[0].clone();
    later.trace_instance_index = 1;
    later.final_polynomial[0][0] = 61;
    segment.units.push(later);

    let encoded = encode_pcs_fri_opening_segment(&segment).expect("FRI segment should encode");
    let parsed = parse_pcs_fri_opening_segment(&encoded).expect("FRI segment should parse");

    assert_eq!(&encoded[4..8], &2_u32.to_le_bytes());
    assert_eq!(parsed, segment);
}

#[test]
fn parses_legacy_pcs_fri_opening_units_as_base_trace_instances() {
    let encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    let parsed = parse_pcs_fri_opening_segment(&encoded).expect("FRI segment should parse");

    assert_eq!(&encoded[4..8], &1_u32.to_le_bytes());
    assert_eq!(parsed.units[0].trace_instance_index, 0);
}

#[test]
fn rejects_duplicate_trace_instance_pcs_fri_opening_units() {
    let mut segment = sample_segment();
    segment.units[0].trace_instance_index = 1;
    segment.units.push(segment.units[0].clone());

    assert!(matches!(
        encode_pcs_fri_opening_segment(&segment),
        Err(PcsFriOpeningSegmentError::DuplicateUnitIdentity {
            unit_index: 0,
            trace_instance_index: 1
        })
    ));
}

#[test]
fn rejects_non_canonical_pcs_fri_final_polynomial_values() {
    let mut segment = sample_segment();
    segment.units[0].final_polynomial[0][1] = NON_CANONICAL_FIELD;

    let err = encode_pcs_fri_opening_segment(&segment).expect_err("field value should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 final polynomial value 0 word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_layer_roots() {
    let mut segment = sample_segment();
    segment.units[0].layers[0].root[2] = NON_CANONICAL_FIELD;

    let err = encode_pcs_fri_opening_segment(&segment).expect_err("layer root should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 layer 0 root word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_last_level_roots() {
    let mut segment = sample_segment();
    segment.units[0].layers[0].last_level[0][3] = NON_CANONICAL_FIELD;

    let err = encode_pcs_fri_opening_segment(&segment).expect_err("last level root should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 layer 0 last level root 0 word 3 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_query_values() {
    let mut segment = sample_segment();
    segment.units[0].layers[0].queries[0].values[1][0] = NON_CANONICAL_FIELD;

    let err = encode_pcs_fri_opening_segment(&segment).expect_err("query value should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 layer 0 row 3 query value 1 word 0 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_sibling_roots() {
    let mut segment = sample_segment();
    segment.units[0].layers[0].queries[0].siblings[0].siblings[0][1] = NON_CANONICAL_FIELD;

    let err = encode_pcs_fri_opening_segment(&segment).expect_err("sibling root should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 layer 0 row 3 sibling level 0 root 0 word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_values_when_parsing() {
    let mut encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    encoded[FIRST_FINAL_POLYNOMIAL_OFFSET..FIRST_FINAL_POLYNOMIAL_OFFSET + 8]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_pcs_fri_opening_segment(&encoded).expect_err("field value should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 final polynomial value 0 word 0 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_layer_roots_when_parsing() {
    let mut encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    encoded[FIRST_LAYER_ROOT_OFFSET + 16..FIRST_LAYER_ROOT_OFFSET + 24]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_pcs_fri_opening_segment(&encoded).expect_err("layer root should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 layer 0 root word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_last_level_roots_when_parsing() {
    let mut encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    encoded[FIRST_LAST_LEVEL_OFFSET + 24..FIRST_LAST_LEVEL_OFFSET + 32]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_pcs_fri_opening_segment(&encoded).expect_err("last level root should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 layer 0 last level root 0 word 3 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_query_values_when_parsing() {
    let mut encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    encoded[FIRST_QUERY_VALUES_OFFSET + 24..FIRST_QUERY_VALUES_OFFSET + 32]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_pcs_fri_opening_segment(&encoded).expect_err("query value should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 layer 0 row 3 query value 1 word 0 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_pcs_fri_sibling_roots_when_parsing() {
    let mut encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    encoded[FIRST_SIBLING_OFFSET + 8..FIRST_SIBLING_OFFSET + 16]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_pcs_fri_opening_segment(&encoded).expect_err("sibling root should reject");

    assert_eq!(
        err.to_string(),
        "PCS FRI opening unit 0 layer 0 row 3 sibling level 0 root 0 word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_unsupported_pcs_fri_opening_segment_versions() {
    let mut encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    encoded[4..8].copy_from_slice(&3_u32.to_le_bytes());

    assert!(matches!(
        parse_pcs_fri_opening_segment(&encoded),
        Err(PcsFriOpeningSegmentError::UnsupportedVersion { version: 3 })
    ));
}

#[test]
fn rejects_empty_pcs_fri_opening_segments() {
    let segment = PcsFriOpeningSegment { units: Vec::new() };

    assert!(matches!(
        encode_pcs_fri_opening_segment(&segment),
        Err(PcsFriOpeningSegmentError::EmptyUnits)
    ));
}

#[test]
fn rejects_pcs_fri_units_without_final_polynomials() {
    let mut segment = sample_segment();
    segment.units[0].final_polynomial.clear();

    assert!(matches!(
        encode_pcs_fri_opening_segment(&segment),
        Err(PcsFriOpeningSegmentError::EmptyFinalPolynomial { unit_index: 0 })
    ));
}

#[test]
fn rejects_duplicate_pcs_fri_layer_indices() {
    let mut segment = sample_segment();
    let layer = segment.units[0].layers[0].clone();
    segment.units[0].layers.push(layer);

    assert!(matches!(
        encode_pcs_fri_opening_segment(&segment),
        Err(PcsFriOpeningSegmentError::DuplicateLayerIndex {
            unit_index: 0,
            layer_index: 0
        })
    ));
}

#[test]
fn encodes_duplicate_pcs_fri_query_rows() {
    let mut segment = sample_segment();
    let query = segment.units[0].layers[0].queries[0].clone();
    segment.units[0].layers[0].queries.push(query);

    let encoded = encode_pcs_fri_opening_segment(&segment).expect("duplicate rows should encode");
    let parsed = parse_pcs_fri_opening_segment(&encoded).expect("FRI segment should parse");

    assert_eq!(parsed, segment);
}

#[test]
fn rejects_truncated_pcs_fri_opening_segments() {
    assert!(matches!(
        parse_pcs_fri_opening_segment(b"fos0\x01\0"),
        Err(PcsFriOpeningSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_unit_headers() {
    assert!(matches!(
        parse_pcs_fri_opening_segment(&segment_header(1)),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_final_count_that_exceeds_remaining_extensions() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_pcs_fri_opening_segment(&bytes),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_v2_final_count_that_exceeds_remaining_extensions() {
    let mut bytes = v2_segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_pcs_fri_opening_segment(&bytes),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_layer_count_that_exceeds_remaining_layer_headers() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_extension(&mut bytes);

    assert!(matches!(
        parse_pcs_fri_opening_segment(&bytes),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_last_level_count_that_exceeds_remaining_digests() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_extension(&mut bytes);
    push_u32(&mut bytes, 0);
    push_digest(&mut bytes);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 0);

    assert!(matches!(
        parse_pcs_fri_opening_segment(&bytes),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_query_count_that_exceeds_remaining_query_headers() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_extension(&mut bytes);
    push_u32(&mut bytes, 0);
    push_digest(&mut bytes);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_pcs_fri_opening_segment(&bytes),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_value_count_that_exceeds_remaining_extensions() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_extension(&mut bytes);
    push_u32(&mut bytes, 0);
    push_digest(&mut bytes);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 3);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 0);

    assert!(matches!(
        parse_pcs_fri_opening_segment(&bytes),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_level_count_that_exceeds_remaining_level_headers() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_extension(&mut bytes);
    push_u32(&mut bytes, 0);
    push_digest(&mut bytes);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 3);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_extension(&mut bytes);

    assert!(matches!(
        parse_pcs_fri_opening_segment(&bytes),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

#[test]
fn rejects_sibling_count_that_exceeds_remaining_digests() {
    let mut bytes = segment_header(1);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_extension(&mut bytes);
    push_u32(&mut bytes, 0);
    push_digest(&mut bytes);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 3);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    push_extension(&mut bytes);
    push_u32(&mut bytes, 1);

    assert!(matches!(
        parse_pcs_fri_opening_segment(&bytes),
        Err(PcsFriOpeningSegmentError::UnexpectedEof { .. })
    ));
}

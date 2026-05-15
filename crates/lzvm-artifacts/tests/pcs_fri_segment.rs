use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, parse_pcs_fri_opening_segment, PcsFriOpeningLayerSegment,
    PcsFriOpeningLevelSegment, PcsFriOpeningQuerySegment, PcsFriOpeningSegment,
    PcsFriOpeningSegmentError, PcsFriOpeningUnitSegment,
};

fn sample_segment() -> PcsFriOpeningSegment {
    PcsFriOpeningSegment {
        units: vec![PcsFriOpeningUnitSegment {
            unit_index: 0,
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

#[test]
fn encodes_and_parses_pcs_fri_opening_segments() {
    let encoded =
        encode_pcs_fri_opening_segment(&sample_segment()).expect("FRI segment should encode");
    let parsed = parse_pcs_fri_opening_segment(&encoded).expect("FRI segment should parse");

    assert_eq!(&encoded[0..4], b"fos0");
    assert_eq!(parsed, sample_segment());
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

use lzvm_artifacts::global_info::{
    AggregationType, CurveKind, GlobalAir, GlobalInfo, NamedStageValue,
};
use lzvm_artifacts::pcs_proof_values_segment::{
    encode_pcs_proof_values_segment, parse_pcs_proof_values_segment, PcsProofValuesSegment,
    PcsProofValuesSegmentError, PCS_PROOF_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt, FieldError, MODULUS};
use lzvm_prover::proof_values::{
    build_pcs_proof_values_segment_from_packed_values, flatten_pcs_proof_values,
    load_pcs_proof_values_from_segments, LoadPcsProofValuesSegmentError,
    ProvePcsProofValuesSegmentError,
};

const FIRST_PCS_PROOF_VALUE_OFFSET: usize = 12;

#[test]
fn packs_stage_one_values_as_scalars_and_later_stages_as_extensions() {
    let global = sample_global_info(vec![
        sample_proof_value("scalar-value", 1),
        sample_proof_value("extension-value", 2),
    ]);
    let packed_values = [
        Felt::from_u64(7),
        Felt::from_u64(11),
        Felt::from_u64(12),
        Felt::from_u64(13),
    ];

    let segment = build_pcs_proof_values_segment_from_packed_values(&global, &packed_values)
        .expect("segment should build")
        .expect("metadata declares proof values");
    let parsed =
        parse_pcs_proof_values_segment(&segment.data).expect("segment should parse after build");

    assert_eq!(segment.id, PCS_PROOF_VALUES_SEGMENT_ID);
    assert_eq!(parsed.values, vec![[7, 0, 0], [11, 12, 13]]);
}

#[test]
fn flattens_segment_values_for_global_constraint_offsets() {
    let global = sample_global_info(vec![
        sample_proof_value("scalar-value", 1),
        sample_proof_value("extension-value", 2),
    ]);
    let values = [Ext3::from_u64s([7, 0, 0]), Ext3::from_u64s([11, 12, 13])];

    let packed = flatten_pcs_proof_values(&global, &values).expect("values should flatten");

    assert_eq!(
        packed,
        vec![
            Felt::from_u64(7),
            Felt::from_u64(11),
            Felt::from_u64(12),
            Felt::from_u64(13)
        ]
    );
}

#[test]
fn loads_pcs_proof_values_from_segments() {
    let global = sample_global_info(vec![
        sample_proof_value("scalar-value", 1),
        sample_proof_value("extension-value", 2),
    ]);
    let segment = build_pcs_proof_values_segment_from_packed_values(
        &global,
        &[
            Felt::from_u64(7),
            Felt::from_u64(11),
            Felt::from_u64(12),
            Felt::from_u64(13),
        ],
    )
    .expect("segment should build")
    .expect("metadata declares proof values");

    let values =
        load_pcs_proof_values_from_segments(&global, &[segment]).expect("segment should load");

    assert_eq!(
        values,
        vec![Ext3::from_u64s([7, 0, 0]), Ext3::from_u64s([11, 12, 13])]
    );
}

#[test]
fn packs_array_proof_values_as_flattened_segment_values() {
    let global = sample_global_info(vec![sample_array_proof_value("expected", 2, &[2])]);
    let packed_values = [
        Felt::from_u64(11),
        Felt::from_u64(12),
        Felt::from_u64(13),
        Felt::from_u64(21),
        Felt::from_u64(22),
        Felt::from_u64(23),
    ];

    let segment = build_pcs_proof_values_segment_from_packed_values(&global, &packed_values)
        .expect("segment should build")
        .expect("metadata declares proof values");
    let parsed =
        parse_pcs_proof_values_segment(&segment.data).expect("segment should parse after build");

    assert_eq!(parsed.values, vec![[11, 12, 13], [21, 22, 23]]);

    let values = [Ext3::from_u64s([11, 12, 13]), Ext3::from_u64s([21, 22, 23])];
    let flattened = flatten_pcs_proof_values(&global, &values).expect("values should flatten");

    assert_eq!(flattened, packed_values);

    let loaded =
        load_pcs_proof_values_from_segments(&global, &[segment]).expect("segment should load");

    assert_eq!(loaded, values);
}

#[test]
fn rejects_missing_pcs_proof_values_segment() {
    let global = sample_global_info(vec![sample_proof_value("scalar-value", 1)]);

    let error = load_pcs_proof_values_from_segments(&global, &[])
        .expect_err("required segment should be present");

    assert_eq!(error, LoadPcsProofValuesSegmentError::MissingSegment);
}

#[test]
fn rejects_unexpected_pcs_proof_values_segment() {
    let global = sample_global_info(Vec::new());

    let error = load_pcs_proof_values_from_segments(
        &global,
        &[ProofSegment {
            id: PCS_PROOF_VALUES_SEGMENT_ID,
            data: vec![1, 2, 3, 4],
        }],
    )
    .expect_err("segment should not be present");

    assert_eq!(error, LoadPcsProofValuesSegmentError::UnexpectedSegment);
}

#[test]
fn rejects_duplicate_pcs_proof_values_segments() {
    let global = sample_global_info(vec![sample_proof_value("extension-value", 2)]);
    let segment = build_pcs_proof_values_segment_from_packed_values(
        &global,
        &[Felt::from_u64(1), Felt::from_u64(2), Felt::from_u64(3)],
    )
    .expect("segment should build")
    .expect("segment should exist");

    let error = load_pcs_proof_values_from_segments(&global, &[segment.clone(), segment])
        .expect_err("duplicate segments should reject");

    assert_eq!(error.to_string(), "duplicate PCS proof values segment");
}

#[test]
fn rejects_loaded_pcs_proof_values_count_mismatch() {
    let global = sample_global_info(vec![
        sample_proof_value("scalar-value", 1),
        sample_proof_value("extension-value", 2),
    ]);
    let segment = pcs_proof_values_segment([[7, 0, 0]]);

    let error = load_pcs_proof_values_from_segments(&global, &[segment])
        .expect_err("segment value count should match metadata");

    assert_eq!(
        error,
        LoadPcsProofValuesSegmentError::ValueCountMismatch {
            expected: 2,
            found: 1
        }
    );
}

#[test]
fn rejects_loaded_stage_one_extension_components() {
    let global = sample_global_info(vec![sample_proof_value("scalar-value", 1)]);
    let segment = pcs_proof_values_segment([[7, 1, 0]]);

    let error = load_pcs_proof_values_from_segments(&global, &[segment])
        .expect_err("stage-1 value should be scalar");

    assert_eq!(
        error,
        LoadPcsProofValuesSegmentError::StageOneExtensionComponents { index: 0 }
    );
}

#[test]
fn rejects_loaded_noncanonical_pcs_proof_values() {
    let global = sample_global_info(vec![sample_proof_value("extension-value", 2)]);
    let mut segment = pcs_proof_values_segment([[7, 0, 0]]);
    segment.data[FIRST_PCS_PROOF_VALUE_OFFSET..FIRST_PCS_PROOF_VALUE_OFFSET + 8]
        .copy_from_slice(&MODULUS.to_le_bytes());

    let error = load_pcs_proof_values_from_segments(&global, &[segment])
        .expect_err("segment values should be canonical field elements");

    assert_eq!(
        error,
        LoadPcsProofValuesSegmentError::Segment(PcsProofValuesSegmentError::ValueNonCanonical {
            value_index: 0,
            word_index: 0,
            source: FieldError::NonCanonical { value: MODULUS }
        })
    );
}

#[test]
fn rejects_stage_one_extension_components_when_flattening() {
    let global = sample_global_info(vec![sample_proof_value("scalar-value", 1)]);
    let values = [Ext3::from_u64s([7, 1, 0])];

    let result = flatten_pcs_proof_values(&global, &values);

    assert!(matches!(
        result,
        Err(ProvePcsProofValuesSegmentError::StageOneExtensionComponents { index: 0 })
    ));
}

#[test]
fn omits_proof_values_segment_when_metadata_declares_none() {
    let global = sample_global_info(Vec::new());

    let segment = build_pcs_proof_values_segment_from_packed_values(&global, &[])
        .expect("empty proof values should be accepted");

    assert!(segment.is_none());
}

#[test]
fn rejects_packed_values_when_metadata_declares_none() {
    let global = sample_global_info(Vec::new());

    let result = build_pcs_proof_values_segment_from_packed_values(&global, &[Felt::from_u64(1)]);

    assert!(matches!(
        result,
        Err(ProvePcsProofValuesSegmentError::UnexpectedValues { found: 1 })
    ));
}

#[test]
fn rejects_packed_value_count_mismatches() {
    let global = sample_global_info(vec![
        sample_proof_value("scalar-value", 1),
        sample_proof_value("extension-value", 3),
    ]);
    let packed_values = [Felt::from_u64(7), Felt::from_u64(11), Felt::from_u64(12)];

    let result = build_pcs_proof_values_segment_from_packed_values(&global, &packed_values);

    assert!(matches!(
        result,
        Err(ProvePcsProofValuesSegmentError::ValueCountMismatch {
            expected: 4,
            found: 3
        })
    ));
}

#[test]
fn rejects_zero_length_proof_value_dimensions() {
    let global = sample_global_info(vec![sample_array_proof_value("empty-value", 1, &[0])]);

    assert_eq!(
        build_pcs_proof_values_segment_from_packed_values(&global, &[]),
        Err(ProvePcsProofValuesSegmentError::LengthOverflow)
    );
    assert_eq!(
        flatten_pcs_proof_values(&global, &[]),
        Err(ProvePcsProofValuesSegmentError::LengthOverflow)
    );
    assert_eq!(
        load_pcs_proof_values_from_segments(&global, &[]),
        Err(LoadPcsProofValuesSegmentError::LengthOverflow)
    );
}

fn sample_global_info(proof_values_map: Vec<NamedStageValue>) -> GlobalInfo {
    let num_proof_values = if proof_values_map.is_empty() {
        Vec::new()
    } else {
        vec![proof_values_map.len() as u64]
    };
    GlobalInfo {
        name: "sample-program".to_owned(),
        air_groups: vec!["group-a".to_owned()],
        airs: vec![vec![GlobalAir {
            name: "unit-a".to_owned(),
            num_rows: 2,
            has_compressor: false,
        }]],
        curve: CurveKind::None,
        lattice_size: None,
        aggregation_types: vec![vec![AggregationType {
            aggregation_type: 0,
        }]],
        n_publics: 0,
        num_challenges: vec![1],
        num_proof_values,
        proof_values_map,
        publics_map: Vec::new(),
        transcript_arity: 4,
    }
}

fn sample_proof_value(name: &str, stage: u64) -> NamedStageValue {
    sample_array_proof_value(name, stage, &[])
}

fn sample_array_proof_value(name: &str, stage: u64, lengths: &[u64]) -> NamedStageValue {
    NamedStageValue {
        name: name.to_owned(),
        stage,
        id: None,
        lengths: lengths.to_vec(),
    }
}

fn pcs_proof_values_segment<const N: usize>(values: [[u64; 3]; N]) -> ProofSegment {
    ProofSegment {
        id: PCS_PROOF_VALUES_SEGMENT_ID,
        data: encode_pcs_proof_values_segment(&PcsProofValuesSegment {
            values: values.to_vec(),
        })
        .expect("segment should encode"),
    }
}

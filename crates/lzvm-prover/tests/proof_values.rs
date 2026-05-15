use lzvm_artifacts::global_info::{
    AggregationType, CurveKind, GlobalAir, GlobalInfo, NamedStageValue,
};
use lzvm_artifacts::pcs_proof_values_segment::{
    parse_pcs_proof_values_segment, PCS_PROOF_VALUES_SEGMENT_ID,
};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::proof_values::{
    build_pcs_proof_values_segment_from_packed_values, flatten_pcs_proof_values,
    ProvePcsProofValuesSegmentError,
};

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
    NamedStageValue {
        name: name.to_owned(),
        stage,
        id: None,
        lengths: Vec::new(),
    }
}

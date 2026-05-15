use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, PcsEvaluationSegment, PcsEvaluationUnitSegment,
    PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_prover::pcs_evaluation::{
    load_pcs_evaluation_unit_from_segments, LoadPcsEvaluationUnitError,
};
use lzvm_prover::ProveUnitSchedule;

#[test]
fn loads_pcs_evaluation_unit_from_segments() {
    let segment = pcs_evaluation_proof_segment(vec![evaluation_unit(0, vec![[1, 2, 3]])]);

    let loaded = load_pcs_evaluation_unit_from_segments(0, &sample_unit(1), &[segment])
        .expect("evaluation unit should load");

    assert_eq!(loaded, evaluation_unit(0, vec![[1, 2, 3]]));
}

#[test]
fn rejects_missing_pcs_evaluation_segment() {
    let error = load_pcs_evaluation_unit_from_segments(0, &sample_unit(1), &[])
        .expect_err("segment should be present");

    assert_eq!(error, LoadPcsEvaluationUnitError::MissingSegment);
}

#[test]
fn rejects_missing_pcs_evaluation_unit() {
    let segment = pcs_evaluation_proof_segment(vec![evaluation_unit(1, vec![[1, 2, 3]])]);

    let error = load_pcs_evaluation_unit_from_segments(0, &sample_unit(1), &[segment])
        .expect_err("unit should be present");

    assert_eq!(
        error,
        LoadPcsEvaluationUnitError::MissingUnit { unit_index: 0 }
    );
}

#[test]
fn rejects_pcs_evaluation_value_count_mismatches() {
    let segment = pcs_evaluation_proof_segment(vec![evaluation_unit(0, vec![[1, 2, 3]])]);

    let error = load_pcs_evaluation_unit_from_segments(0, &sample_unit(2), &[segment])
        .expect_err("evaluation count should match schedule");

    assert_eq!(
        error,
        LoadPcsEvaluationUnitError::ValueCountMismatch {
            unit_index: 0,
            expected: 2,
            found: 1
        }
    );
}

fn pcs_evaluation_proof_segment(units: Vec<PcsEvaluationUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: encode_pcs_evaluation_segment(&PcsEvaluationSegment { units })
            .expect("segment should encode"),
    }
}

fn evaluation_unit(unit_index: u32, values: Vec<[u64; 3]>) -> PcsEvaluationUnitSegment {
    PcsEvaluationUnitSegment { unit_index, values }
}

fn sample_unit(evaluation_value_count: usize) -> ProveUnitSchedule {
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits: 1,
        extended_domain_bits: 2,
        base_domain_size: 2,
        extended_domain_size: 4,
        blowup_factor: 2,
        query_count: 2,
        proof_of_work_bits: 0,
        merkle_tree_arity: 2,
        last_level_verification: 0,
        transcript_arity: Some(2),
        hash_commits: false,
        transcript_root_challenge_draws: vec![1],
        challenge_count: 1,
        evaluation_value_count,
        transcript_evaluation_challenge_draws: 1,
        constant_width: 1,
        stage_commit_widths: vec![1],
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: 2,
            output_bits: 1,
            folding_factor: 2,
        }],
        final_layer_bits: 1,
        fixed_bytes: 64,
        constant_tree_root: None,
        pcs_material_bytes: None,
        pcs_material_plan_digest: None,
        pcs_material_fixed_column_digest: None,
        pcs_material_constant_tree_digest: None,
        pcs_material_constant_tree_root: None,
        pcs_material_fixed_byte_count: None,
        pcs_material_constant_tree_byte_count: None,
        pcs_material_leaf_byte_count: None,
        pcs_material_node_byte_count: None,
    }
}

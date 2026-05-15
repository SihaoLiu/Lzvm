use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_prover::pcs_query_plan::{
    load_pcs_query_plan_from_segments, validate_seeded_pcs_query_plan_segments,
    LoadPcsQueryPlanSegmentError, ValidatePcsQueryPlanSegmentsError,
};
use lzvm_prover::{build_pcs_query_plan_segment, ProveSchedule, ProveUnitSchedule};

#[test]
fn loads_pcs_query_plan_from_segments() {
    let segment = pcs_query_plan_proof_segment(vec![PcsQueryPlanUnit {
        unit_index: 0,
        queries: vec![1, 3],
    }]);

    let loaded = load_pcs_query_plan_from_segments(&[segment]).expect("query plan should load");

    assert_eq!(
        loaded,
        PcsQueryPlanSegment {
            units: vec![PcsQueryPlanUnit {
                unit_index: 0,
                queries: vec![1, 3]
            }]
        }
    );
}

#[test]
fn rejects_missing_pcs_query_plan_segment() {
    let error = load_pcs_query_plan_from_segments(&[]).expect_err("segment should be present");

    assert_eq!(error, LoadPcsQueryPlanSegmentError::MissingSegment);
}

#[test]
fn rejects_invalid_pcs_query_plan_segment() {
    let error = load_pcs_query_plan_from_segments(&[ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }])
    .expect_err("segment should parse");

    assert!(matches!(error, LoadPcsQueryPlanSegmentError::Segment(_)));
}

#[test]
fn validates_seeded_pcs_query_plan_segments() {
    let schedule = sample_schedule();
    let public_hash = [7; 32];
    let material = material_segment();
    let witness = witness_segment(0);
    let query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&witness),
    )
    .expect("query plan should build");
    let segments = vec![material, witness, query];

    validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect("query plan should validate");
}

#[test]
fn rejects_seeded_pcs_query_plan_mismatches() {
    let schedule = sample_schedule();
    let public_hash = [7; 32];
    let material = material_segment();
    let witness = witness_segment(0);
    let query = build_pcs_query_plan_segment(
        &schedule,
        [9; 32],
        &material,
        std::slice::from_ref(&witness),
    )
    .expect("query plan should build");
    let segments = vec![material, witness, query];

    let error = validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect_err("query plan mismatch should be rejected");

    assert_eq!(error, ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
}

fn pcs_query_plan_proof_segment(units: Vec<PcsQueryPlanUnit>) -> ProofSegment {
    ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&PcsQueryPlanSegment { units })
            .expect("segment should encode"),
    }
}

fn material_segment() -> ProofSegment {
    ProofSegment {
        id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }
}

fn witness_segment(unit_index: u32) -> ProofSegment {
    ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID + unit_index,
        data: encode_witness_commitment_segment(&WitnessCommitmentSegment {
            unit_index,
            input_byte_count: 0,
            trace_rows: 2,
            trace_columns: 1,
            stages: vec![WitnessCommitmentStageSegment {
                stage_index: 1,
                arity: 2,
                root: [1, 2, 3, 4],
                tree_byte_count: 64,
                tree_digest: [0; 32],
            }],
        })
        .expect("witness segment should encode"),
    }
}

fn sample_schedule() -> ProveSchedule {
    ProveSchedule {
        setup_hash: [3; 32],
        unit_count: 1,
        total_fixed_bytes: 0,
        total_pcs_material_bytes: 0,
        pcs_material_unit_count: 1,
        total_query_count: 2,
        max_extended_domain_bits: 2,
        units: vec![sample_unit()],
    }
}

fn sample_unit() -> ProveUnitSchedule {
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
        evaluation_value_count: 0,
        transcript_evaluation_challenge_draws: 0,
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
        fixed_bytes: 0,
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

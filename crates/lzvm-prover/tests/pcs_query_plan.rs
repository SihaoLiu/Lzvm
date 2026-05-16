use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, PcsEvaluationSegment, PcsEvaluationUnitSegment,
    PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, PcsFriOpeningSegment, PcsFriOpeningUnitSegment,
    PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material_segment::{
    encode_pcs_material_manifest_segment, PcsMaterialManifestSegment, PcsMaterialManifestUnit,
    PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::program_image::{ProgramImageCommitmentCache, ProgramImageGpuMode};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_prover::pcs_query_plan::{
    load_pcs_query_plan_from_segments, uses_transcript_pcs_query_plan_inputs,
    validate_pcs_query_plan_segments, validate_seeded_pcs_query_plan_segments,
    validate_transcript_pcs_query_plan_segments, LoadPcsQueryPlanSegmentError,
    ValidatePcsQueryPlanSegmentsError,
};
use lzvm_prover::pcs_transcript::PcsTranscriptSegmentInputs;
use lzvm_prover::{
    build_pcs_query_nonce_segment_from_transcript_segments, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_transcript_segments, ProveSchedule, ProveUnitSchedule,
};

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
fn validates_pcs_query_plan_segments_from_seeded_inputs() {
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

    validate_pcs_query_plan_segments(&schedule, public_hash, &[], &segments)
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

#[test]
fn rejects_seeded_pcs_query_plan_mismatches_with_program_image_cache_segment() {
    let schedule = sample_schedule();
    let public_hash = [7; 32];
    let material = material_segment();
    let witness = witness_segment(0);
    let cache_segment = ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: encode_program_image_cache_segment(&sample_program_image_cache())
            .expect("cache should encode"),
    };
    let query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&witness),
    )
    .expect("query plan should build");
    let segments = vec![material, witness, cache_segment, query];

    let error = validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect_err("query plan mismatch should be rejected");

    assert_eq!(error, ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
}

#[test]
fn validates_transcript_pcs_query_plan_segments() {
    let (schedule, segments) = transcript_query_plan_segments();

    validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect("query plan should validate");
}

#[test]
fn validates_pcs_query_plan_segments_from_transcript_inputs() {
    let (schedule, segments) = transcript_query_plan_segments();

    validate_pcs_query_plan_segments(&schedule, [0; 32], &[], &segments)
        .expect("query plan should validate");
}

#[test]
fn detects_transcript_pcs_query_plan_inputs() {
    assert!(!uses_transcript_pcs_query_plan_inputs(&[]));
    assert!(uses_transcript_pcs_query_plan_inputs(&[ProofSegment {
        id: PCS_QUERY_NONCE_SEGMENT_ID,
        data: Vec::new(),
    }]));
    assert!(uses_transcript_pcs_query_plan_inputs(&[ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: Vec::new(),
    }]));
}

#[test]
fn rejects_transcript_pcs_query_plan_mismatches() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    let query = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .expect("query plan segment should exist");
    query.data = encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
        units: vec![PcsQueryPlanUnit {
            unit_index: 0,
            queries: vec![0, 0],
        }],
    })
    .expect("query plan should encode");

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("query plan mismatch should be rejected");

    assert_eq!(error, ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
}

#[test]
fn rejects_transcript_pcs_query_plan_mismatches_with_program_image_cache_segment() {
    let mut schedule = sample_schedule();
    schedule.units[0].evaluation_value_count = 1;
    schedule.units[0].transcript_evaluation_challenge_draws = 1;
    schedule.units[0].query_count = 8;
    schedule.units[0].extended_domain_size = 1024;
    schedule.total_query_count = 8;
    schedule.max_extended_domain_bits = 10;
    let material = material_unit(0);
    let witness = witness_commitment(0);
    let witness_segment = witness_segment(0);
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        values: vec![[9, 10, 11]],
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        layers: Vec::new(),
        final_polynomial: vec![[12, 13, 14]],
    };
    let good_cache_segment = ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: encode_program_image_cache_segment(&sample_program_image_cache())
            .expect("cache should encode"),
    };
    let bad_cache_segment = ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: encode_program_image_cache_segment(&sample_program_image_cache_variant())
            .expect("cache should encode"),
    };
    let transcript_input = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &[],
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: std::slice::from_ref(&good_cache_segment),
    };
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, transcript_input)
            .expect("query nonce should build");
    let transcript_input = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &[],
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: std::slice::from_ref(&good_cache_segment),
    };
    let query = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_input,
        &nonce_segment,
    )
    .expect("query plan should build");
    let transcript_input = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &[],
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: std::slice::from_ref(&bad_cache_segment),
    };
    let expected_bad_query = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_input,
        &nonce_segment,
    )
    .expect("query plan should build");
    let segments = vec![
        ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: encode_pcs_material_manifest_segment(&PcsMaterialManifestSegment {
                units: vec![material],
            })
            .expect("material segment should encode"),
        },
        witness_segment,
        bad_cache_segment,
        ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: encode_pcs_evaluation_segment(&PcsEvaluationSegment {
                units: vec![evaluations],
            })
            .expect("evaluation segment should encode"),
        },
        ProofSegment {
            id: PCS_FRI_OPENING_SEGMENT_ID,
            data: encode_pcs_fri_opening_segment(&PcsFriOpeningSegment { units: vec![fri] })
                .expect("FRI opening segment should encode"),
        },
        nonce_segment,
        query,
    ];

    assert_ne!(
        expected_bad_query.data,
        segments
            .iter()
            .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
            .expect("query plan should exist")
            .data
    );

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
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

fn transcript_query_plan_segments() -> (ProveSchedule, Vec<ProofSegment>) {
    let mut schedule = sample_schedule();
    schedule.units[0].evaluation_value_count = 1;
    schedule.units[0].transcript_evaluation_challenge_draws = 1;
    let material = material_unit(0);
    let witness = witness_commitment(0);
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        values: vec![[9, 10, 11]],
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        layers: Vec::new(),
        final_polynomial: vec![[12, 13, 14]],
    };
    let input = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &[],
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let witness_segment = witness_segment(0);
    let nonce_segment = build_pcs_query_nonce_segment_from_transcript_segments(&schedule, input)
        .expect("query nonce should build");
    let input = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &[],
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        input,
        &nonce_segment,
    )
    .expect("query plan should build");
    let material_segment = ProofSegment {
        id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        data: encode_pcs_material_manifest_segment(&PcsMaterialManifestSegment {
            units: vec![material],
        })
        .expect("material segment should encode"),
    };
    let evaluation_segment = ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: encode_pcs_evaluation_segment(&PcsEvaluationSegment {
            units: vec![evaluations],
        })
        .expect("evaluation segment should encode"),
    };
    let fri_segment = ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: encode_pcs_fri_opening_segment(&PcsFriOpeningSegment { units: vec![fri] })
            .expect("FRI opening segment should encode"),
    };

    (
        schedule,
        vec![
            material_segment,
            witness_segment,
            evaluation_segment,
            fri_segment,
            nonce_segment,
            query_segment,
        ],
    )
}

fn material_segment() -> ProofSegment {
    ProofSegment {
        id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }
}

fn material_unit(unit_index: u32) -> PcsMaterialManifestUnit {
    PcsMaterialManifestUnit {
        unit_index,
        plan_digest: [1; 32],
        fixed_column_digest: [2; 32],
        constant_tree_digest: [3; 32],
        constant_tree_root: [1, 2, 3, 4],
        fixed_byte_count: 0,
        constant_tree_byte_count: 0,
        leaf_byte_count: 0,
        node_byte_count: 0,
    }
}

fn witness_segment(unit_index: u32) -> ProofSegment {
    let witness = witness_commitment(unit_index);
    ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID + unit_index,
        data: encode_witness_commitment_segment(&witness).expect("witness segment should encode"),
    }
}

fn witness_commitment(unit_index: u32) -> WitnessCommitmentSegment {
    WitnessCommitmentSegment {
        unit_index,
        input_byte_count: 0,
        trace_rows: 2,
        trace_columns: 1,
        stages: vec![WitnessCommitmentStageSegment {
            stage_index: 1,
            arity: 2,
            root: [5, 6, 7, 8],
            tree_byte_count: 64,
            tree_digest: [0; 32],
        }],
    }
}

fn sample_program_image_cache() -> ProgramImageCommitmentCache {
    ProgramImageCommitmentCache {
        program_digest: [0x11; 32],
        source_image_digest: [0x22; 32],
        constraint_system_digest: [0x33; 32],
        tree_root: [10, 11, 12, 13],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
    }
}

fn sample_program_image_cache_variant() -> ProgramImageCommitmentCache {
    ProgramImageCommitmentCache {
        program_digest: [0x12; 32],
        source_image_digest: [0x22; 32],
        constraint_system_digest: [0x33; 32],
        tree_root: [10, 11, 12, 13],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
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
        evaluation_map: Vec::new(),
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

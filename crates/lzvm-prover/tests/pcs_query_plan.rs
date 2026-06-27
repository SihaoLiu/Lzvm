use lzvm_artifacts::challenge_values_segment::{
    encode_challenge_values_segment, ChallengeValuesSegment, CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_input_segment::ETH_BLOCK_INPUT_SEGMENT_ID;
use lzvm_artifacts::guest_input_segment::FRAMED_GUEST_INPUT_SEGMENT_ID;
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
use lzvm_artifacts::pcs_nonce_segment::{
    parse_pcs_query_nonce_segment, PCS_QUERY_NONCE_SEGMENT_ID,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::program_image::{ProgramImageCommitmentCache, ProgramImageGpuMode};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::unit_values_segment::{
    encode_unit_values_segment, UnitValuesSegment, UnitValuesUnitSegment, UNIT_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, witness_commitment_segment_id, WitnessCommitmentSegment,
    WitnessCommitmentSegmentIdentity, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::pcs_query_plan::{
    load_pcs_query_plan_from_segments, uses_transcript_pcs_query_plan_inputs,
    validate_pcs_query_plan_segments, validate_seeded_pcs_query_plan_segments,
    validate_transcript_pcs_query_plan_segments, LoadPcsQueryPlanSegmentError,
    ProvePcsQueryPlanSegmentError, ValidatePcsQueryPlanSegmentsError,
};
use lzvm_prover::pcs_transcript::{
    aggregate_pcs_final_query_challenges, derive_pcs_final_query_challenge_from_segments,
    PcsTranscriptSegmentInputs,
};
use lzvm_prover::{
    build_pcs_query_nonce_segment, build_pcs_query_nonce_segment_from_transcript_segments,
    build_pcs_query_nonce_segment_with_streams, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_challenge,
    build_pcs_query_plan_segment_from_transcript_segments,
    build_pcs_query_plan_segment_with_bindings, ProveSchedule, ProveUnitSchedule,
};

#[test]
fn loads_pcs_query_plan_from_segments() {
    let segment = pcs_query_plan_proof_segment(vec![PcsQueryPlanUnit {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![1, 3],
    }]);

    let loaded = load_pcs_query_plan_from_segments(&[segment]).expect("query plan should load");

    assert_eq!(
        loaded,
        PcsQueryPlanSegment {
            units: vec![PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 0,
                queries: vec![1, 3]
            }]
        }
    );
}

#[test]
fn loads_trace_instance_pcs_query_plan_units() {
    let segment = pcs_query_plan_proof_segment(vec![PcsQueryPlanUnit {
        unit_index: 0,
        trace_instance_index: 1,
        queries: vec![1, 3],
    }]);

    let loaded = load_pcs_query_plan_from_segments(&[segment]).expect("query plan should load");

    assert_eq!(
        loaded,
        PcsQueryPlanSegment {
            units: vec![PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 1,
                queries: vec![1, 3]
            }]
        }
    );
}

#[test]
fn rejects_zero_stream_query_nonce_builds() {
    let error = build_pcs_query_nonce_segment_with_streams(
        &sample_schedule(),
        Ext3::from_u64s([7, 8, 9]),
        0,
    )
    .expect_err("zero stream nonce build should be rejected");

    assert_eq!(
        error.to_string(),
        "prove PCS query nonce stream count is invalid"
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
fn rejects_duplicate_pcs_query_plan_segments() {
    let segment = pcs_query_plan_proof_segment(vec![PcsQueryPlanUnit {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![1, 3],
    }]);

    let error = load_pcs_query_plan_from_segments(&[segment.clone(), segment])
        .expect_err("duplicate query plan segments should reject");

    assert_eq!(error.to_string(), "duplicate PCS query plan segment");
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
fn seeded_query_plan_binds_witness_tree_digest() {
    let mut schedule = sample_schedule();
    schedule.total_query_count = 16;
    schedule.max_extended_domain_bits = 8;
    schedule.units[0].extended_domain_bits = 8;
    schedule.units[0].extended_domain_size = 256;
    schedule.units[0].query_count = 16;

    let public_hash = [7; 32];
    let material = material_segment();
    let mut first_witness = witness_commitment(0);
    let mut second_witness = first_witness.clone();
    first_witness.stages[0].tree_digest = [0; 32];
    second_witness.stages[0].tree_digest = [1; 32];
    let first_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&first_witness)
            .expect("witness segment should encode"),
    };
    let second_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&second_witness)
            .expect("witness segment should encode"),
    };

    let first_query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&first_segment),
    )
    .expect("query plan should build");
    let second_query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&second_segment),
    )
    .expect("query plan should build");

    assert_ne!(first_query.data, second_query.data);
}

#[test]
fn seeded_query_plan_ignores_unverified_witness_metadata_counts() {
    let mut schedule = sample_schedule();
    schedule.total_query_count = 16;
    schedule.max_extended_domain_bits = 8;
    schedule.units[0].extended_domain_bits = 8;
    schedule.units[0].extended_domain_size = 256;
    schedule.units[0].query_count = 16;

    let public_hash = [7; 32];
    let material = material_segment();
    let mut first_witness = witness_commitment(0);
    let mut second_witness = first_witness.clone();
    first_witness.input_byte_count = 0;
    first_witness.stages[0].tree_byte_count = 64;
    second_witness.input_byte_count = 99;
    second_witness.stages[0].tree_byte_count = 128;
    let first_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&first_witness)
            .expect("witness segment should encode"),
    };
    let second_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&second_witness)
            .expect("witness segment should encode"),
    };

    let first_query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&first_segment),
    )
    .expect("query plan should build");
    let second_query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&second_segment),
    )
    .expect("query plan should build");

    assert_eq!(first_query.data, second_query.data);
}

#[test]
fn seeded_query_plan_binds_witness_stage_roots() {
    let mut schedule = sample_schedule();
    schedule.total_query_count = 16;
    schedule.max_extended_domain_bits = 8;
    schedule.units[0].extended_domain_bits = 8;
    schedule.units[0].extended_domain_size = 256;
    schedule.units[0].query_count = 16;

    let public_hash = [7; 32];
    let material = material_segment();
    let mut first_witness = witness_commitment(0);
    let mut second_witness = first_witness.clone();
    first_witness.stages[0].root = [5, 6, 7, 8];
    second_witness.stages[0].root = [9, 10, 11, 12];
    let first_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&first_witness)
            .expect("witness segment should encode"),
    };
    let second_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&second_witness)
            .expect("witness segment should encode"),
    };

    let first_query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&first_segment),
    )
    .expect("query plan should build");
    let second_query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&second_segment),
    )
    .expect("query plan should build");

    assert_ne!(first_query.data, second_query.data);
}

#[test]
fn builds_seeded_pcs_query_plan_for_trace_instance_witness_segment() {
    let schedule = sample_schedule();
    let public_hash = [7; 32];
    let material = material_segment();
    let witness = witness_segment_with_trace_instance(0, 1, schedule.units.len());
    let query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&witness),
    )
    .expect("query plan should build");
    let loaded = load_pcs_query_plan_from_segments(std::slice::from_ref(&query))
        .expect("query plan should load");
    let segments = vec![material, witness, query];

    assert_eq!(loaded.units.len(), 1);
    assert_eq!(loaded.units[0].unit_index, 0);
    assert_eq!(loaded.units[0].trace_instance_index, 1);
    validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect("trace instance query plan should validate");
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
fn rejects_seeded_pcs_query_plan_duplicate_material_segments() {
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
    let segments = vec![material.clone(), witness, query, material];

    let error = validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect_err("duplicate material segment should be rejected");

    assert_eq!(error.to_string(), "duplicate PCS material manifest segment");
}

#[test]
fn rejects_seeded_pcs_query_plan_material_mismatch() {
    let schedule = sample_schedule();
    let public_hash = [7; 32];
    let mut material_unit = material_unit(0);
    material_unit.fixed_byte_count += 1;
    let material = material_manifest_segment(vec![material_unit]);
    let witness = witness_segment(0);
    let query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&witness),
    )
    .expect("query plan should build");
    let segments = vec![material, witness, query];

    let error = validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect_err("query plan material should match the schedule");

    assert_eq!(
        error,
        ValidatePcsQueryPlanSegmentsError::UnitMismatch { unit_index: 0 }
    );
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
    let schedule = query_sensitive_schedule();
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
fn rejects_seeded_pcs_query_plan_mismatches_with_challenge_values_segment() {
    let schedule = query_sensitive_schedule();
    let public_hash = [7; 32];
    let material = material_segment();
    let witness = witness_segment(0);
    let challenge_segment = challenge_values_segment([1, 2, 3]);
    let query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&witness),
    )
    .expect("query plan should build");
    let segments = vec![material, witness, challenge_segment, query];

    let error = validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect_err("query plan mismatch should be rejected");

    assert_eq!(error, ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
}

#[test]
fn rejects_seeded_pcs_query_plan_mismatches_with_pipeline_input_binding_segments() {
    for binding_segment in [
        input_binding_segment(ETH_BLOCK_INPUT_SEGMENT_ID, &[1, 2, 3, 4]),
        input_binding_segment(FRAMED_GUEST_INPUT_SEGMENT_ID, &[5, 6, 7, 8]),
    ] {
        let schedule = query_sensitive_schedule();
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
        let bound_query = build_pcs_query_plan_segment_with_bindings(
            &schedule,
            public_hash,
            &material,
            std::slice::from_ref(&witness),
            std::slice::from_ref(&binding_segment),
        )
        .expect("bound query plan should build");
        assert_ne!(
            query.data, bound_query.data,
            "binding segment {} should affect query plan",
            binding_segment.id
        );
        let segments = vec![material, witness, binding_segment, query];

        let error = validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
            .expect_err("query plan mismatch should be rejected");

        assert_eq!(error, ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
    }
}

#[test]
fn rejects_seeded_pcs_query_plan_duplicate_binding_segments() {
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
    let segments = vec![
        material,
        witness,
        cache_segment.clone(),
        cache_segment,
        query,
    ];

    let error = validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect_err("duplicate binding segment should be rejected");

    assert_eq!(
        error.to_string(),
        format!(
            "duplicate proof binding segment id: {}",
            PROGRAM_IMAGE_CACHE_SEGMENT_ID
        )
    );
}

#[test]
fn rejects_seeded_pcs_query_plan_duplicate_challenge_values_segments() {
    let schedule = sample_schedule();
    let public_hash = [7; 32];
    let material = material_segment();
    let witness = witness_segment(0);
    let challenge_segment = challenge_values_segment([1, 2, 3]);
    let query = build_pcs_query_plan_segment(
        &schedule,
        public_hash,
        &material,
        std::slice::from_ref(&witness),
    )
    .expect("query plan should build");
    let segments = vec![
        material,
        witness,
        challenge_segment.clone(),
        challenge_segment,
        query,
    ];

    let error = validate_seeded_pcs_query_plan_segments(&schedule, public_hash, &segments)
        .expect_err("duplicate binding segment should be rejected");

    assert_eq!(
        error.to_string(),
        format!(
            "duplicate proof binding segment id: {}",
            CHALLENGE_VALUES_SEGMENT_ID
        )
    );
}

#[test]
fn validates_transcript_pcs_query_plan_segments() {
    let (schedule, segments) = transcript_query_plan_segments();

    validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect("query plan should validate");
}

#[test]
fn validates_transcript_pcs_query_plan_segments_by_trace_identity() {
    let mut schedule = sample_schedule();
    schedule.units[0].evaluation_value_count = 1;
    schedule.units[0].transcript_evaluation_challenge_draws = 1;
    schedule.total_query_count = 4;
    let material = material_unit(0);
    let base_witness = witness_commitment(0);
    let trace_witness = witness_commitment_with_root(0, [25, 26, 27, 28]);
    let base_witness_segment = witness_segment(0);
    let trace_witness_segment =
        witness_segment_with_trace_instance_and_root(0, 1, schedule.units.len(), [25, 26, 27, 28]);
    let base_evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        values: vec![[9, 10, 11]],
    };
    let trace_evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        trace_instance_index: 1,
        values: vec![[29, 30, 31]],
    };
    let base_fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        layers: Vec::new(),
        final_polynomial: vec![[12, 13, 14]],
    };
    let trace_fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 1,
        layers: Vec::new(),
        final_polynomial: vec![[32, 33, 34]],
    };
    let base_input = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &[],
        unit_values: &[],
        witness: &base_witness,
        evaluations: &base_evaluations,
        fri: &base_fri,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let trace_input = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &[],
        unit_values: &[],
        witness: &trace_witness,
        evaluations: &trace_evaluations,
        fri: &trace_fri,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let base_challenge = derive_pcs_final_query_challenge_from_segments(base_input)
        .expect("base challenge should derive");
    let trace_challenge = derive_pcs_final_query_challenge_from_segments(trace_input)
        .expect("trace challenge should derive");
    let challenge = aggregate_pcs_final_query_challenges(&[base_challenge, trace_challenge])
        .expect("aggregate challenge should derive");
    let nonce_segment =
        build_pcs_query_nonce_segment(&schedule, challenge).expect("query nonce should build");
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .expect("nonce segment should parse")
            .nonce,
    );
    let query_segment = build_pcs_query_plan_segment_from_challenge(
        &schedule,
        &[base_witness_segment.clone(), trace_witness_segment.clone()],
        challenge,
        nonce,
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
        base_witness_segment,
        trace_witness_segment,
        ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: encode_pcs_evaluation_segment(&PcsEvaluationSegment {
                units: vec![base_evaluations, trace_evaluations],
            })
            .expect("evaluation segment should encode"),
        },
        ProofSegment {
            id: PCS_FRI_OPENING_SEGMENT_ID,
            data: encode_pcs_fri_opening_segment(&PcsFriOpeningSegment {
                units: vec![base_fri, trace_fri],
            })
            .expect("FRI opening segment should encode"),
        },
        nonce_segment,
        query_segment,
    ];

    validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect("query plan should match both trace identities");
}

#[test]
fn rejects_trace_instance_transcript_pcs_query_plan_mismatches() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    replace_query_plan_trace_instance(&mut segments, 1);

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("trace instance mismatch should be rejected");

    assert_eq!(
        error,
        ValidatePcsQueryPlanSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn validates_pcs_query_plan_segments_from_transcript_inputs() {
    let (schedule, segments) = transcript_query_plan_segments();

    validate_pcs_query_plan_segments(&schedule, [0; 32], &[], &segments)
        .expect("query plan should validate");
}

#[test]
fn rejects_transcript_pcs_query_plan_duplicate_material_segments() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    let material = segments
        .iter()
        .find(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID)
        .expect("material segment should exist")
        .clone();
    segments.push(material);

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("duplicate material segment should be rejected");

    assert_eq!(error.to_string(), "duplicate PCS material manifest segment");
}

#[test]
fn rejects_transcript_pcs_query_plan_extra_material_units() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    let material_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID)
        .expect("material segment should exist");
    material_segment.data = encode_pcs_material_manifest_segment(&PcsMaterialManifestSegment {
        units: vec![material_unit(0), material_unit(1)],
    })
    .expect("material segment should encode");

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("extra material unit should be rejected");

    assert_eq!(
        error,
        ValidatePcsQueryPlanSegmentsError::UnitMismatch { unit_index: 1 }
    );
}

#[test]
fn rejects_transcript_pcs_query_plan_duplicate_nonce_segments() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    let nonce = segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_NONCE_SEGMENT_ID)
        .expect("query nonce segment should exist")
        .clone();
    segments.push(nonce);

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("duplicate nonce segment should be rejected");

    assert_eq!(error.to_string(), "duplicate PCS query nonce segment");
}

#[test]
fn rejects_transcript_pcs_query_plan_duplicate_binding_segments() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    let cache_segment = ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: encode_program_image_cache_segment(&sample_program_image_cache())
            .expect("cache should encode"),
    };
    segments.push(cache_segment.clone());
    segments.push(cache_segment);

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("duplicate binding segment should be rejected");

    assert_eq!(
        error.to_string(),
        format!(
            "duplicate proof binding segment id: {}",
            PROGRAM_IMAGE_CACHE_SEGMENT_ID
        )
    );
}

#[test]
fn rejects_transcript_pcs_query_plan_mismatches_with_pipeline_input_binding_segments() {
    for binding_segment in [
        input_binding_segment(ETH_BLOCK_INPUT_SEGMENT_ID, &[1, 2, 3, 4]),
        input_binding_segment(FRAMED_GUEST_INPUT_SEGMENT_ID, &[5, 6, 7, 8]),
    ] {
        let (schedule, mut segments) = transcript_query_plan_segments();
        segments.push(binding_segment);

        let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
            .expect_err("query plan mismatch should be rejected");

        assert_eq!(error, ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
    }
}

#[test]
fn rejects_transcript_pcs_query_plan_missing_later_material_unit() {
    let mut schedule = sample_schedule();
    let mut second_unit = sample_unit();
    schedule.units[0].evaluation_value_count = 1;
    schedule.units[0].transcript_evaluation_challenge_draws = 1;
    second_unit.evaluation_value_count = 1;
    second_unit.transcript_evaluation_challenge_draws = 1;
    second_unit.unit_id = Some(1);
    second_unit.unit_name = Some("unit-b".to_owned());
    schedule.unit_count = 2;
    schedule.pcs_material_unit_count = 2;
    schedule.total_query_count = 4;
    schedule.units.push(second_unit);

    let material = material_unit(0);
    let witness = witness_commitment(0);
    let first_witness_segment = witness_segment(0);
    let second_witness_segment = witness_segment(1);
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        values: vec![[9, 10, 11]],
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
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
    let nonce_segment = build_pcs_query_nonce_segment_from_transcript_segments(&schedule, input)
        .expect("query nonce should build");
    let challenge =
        derive_pcs_final_query_challenge_from_segments(input).expect("challenge should derive");
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .expect("nonce segment should parse")
            .nonce,
    );
    let query_segment = build_pcs_query_plan_segment_from_challenge(
        &schedule,
        &[
            first_witness_segment.clone(),
            second_witness_segment.clone(),
        ],
        challenge,
        nonce,
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
        first_witness_segment,
        second_witness_segment,
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
        query_segment,
    ];

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("each query unit should require transcript material");

    assert_eq!(
        error,
        ValidatePcsQueryPlanSegmentsError::UnitMismatch { unit_index: 1 }
    );
}

#[test]
fn rejects_transcript_pcs_query_plan_not_bound_to_each_unit_challenge() {
    let mut schedule = sample_schedule();
    let mut second_unit = sample_unit();
    schedule.units[0].evaluation_value_count = 1;
    schedule.units[0].transcript_evaluation_challenge_draws = 1;
    second_unit.evaluation_value_count = 1;
    second_unit.transcript_evaluation_challenge_draws = 1;
    second_unit.unit_id = Some(1);
    second_unit.unit_name = Some("unit-b".to_owned());
    second_unit.pcs_material_constant_tree_root = Some([21, 22, 23, 24]);
    schedule.unit_count = 2;
    schedule.pcs_material_unit_count = 2;
    schedule.total_query_count = 4;
    schedule.units.push(second_unit);

    let first_material = material_unit(0);
    let mut second_material = material_unit(1);
    second_material.constant_tree_root = [21, 22, 23, 24];

    let first_witness = witness_commitment(0);
    let mut second_witness = witness_commitment(1);
    second_witness.stages[0].root = [25, 26, 27, 28];
    let first_witness_segment = witness_segment(0);
    let second_witness_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1,
        data: encode_witness_commitment_segment(&second_witness)
            .expect("witness segment should encode"),
    };

    let first_evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        values: vec![[9, 10, 11]],
    };
    let second_evaluations = PcsEvaluationUnitSegment {
        unit_index: 1,
        trace_instance_index: 0,
        values: vec![[29, 30, 31]],
    };
    let first_fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        layers: Vec::new(),
        final_polynomial: vec![[12, 13, 14]],
    };
    let second_fri = PcsFriOpeningUnitSegment {
        unit_index: 1,
        trace_instance_index: 0,
        layers: Vec::new(),
        final_polynomial: vec![[32, 33, 34]],
    };
    let first_input = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &first_material,
        public_values: &[],
        unit_values: &[],
        witness: &first_witness,
        evaluations: &first_evaluations,
        fri: &first_fri,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, first_input)
            .expect("query nonce should build");
    let challenge = derive_pcs_final_query_challenge_from_segments(first_input)
        .expect("challenge should derive");
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .expect("nonce segment should parse")
            .nonce,
    );
    let query_segment = build_pcs_query_plan_segment_from_challenge(
        &schedule,
        &[
            first_witness_segment.clone(),
            second_witness_segment.clone(),
        ],
        challenge,
        nonce,
    )
    .expect("query plan should build");
    let segments = vec![
        ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: encode_pcs_material_manifest_segment(&PcsMaterialManifestSegment {
                units: vec![first_material, second_material],
            })
            .expect("material segment should encode"),
        },
        first_witness_segment,
        second_witness_segment,
        ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: encode_pcs_evaluation_segment(&PcsEvaluationSegment {
                units: vec![first_evaluations, second_evaluations],
            })
            .expect("evaluation segment should encode"),
        },
        ProofSegment {
            id: PCS_FRI_OPENING_SEGMENT_ID,
            data: encode_pcs_fri_opening_segment(&PcsFriOpeningSegment {
                units: vec![first_fri, second_fri],
            })
            .expect("FRI opening segment should encode"),
        },
        nonce_segment,
        query_segment,
    ];

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("query plan should bind every unit transcript");

    assert_eq!(error, ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
}

#[test]
fn rejects_transcript_query_plan_builder_with_multiple_witness_units() {
    let mut schedule = sample_schedule();
    let mut second_unit = sample_unit();
    second_unit.unit_id = Some(1);
    second_unit.unit_name = Some("unit-b".to_owned());
    schedule.unit_count = 2;
    schedule.pcs_material_unit_count = 2;
    schedule.total_query_count = 4;
    schedule.units.push(second_unit);

    let material = material_unit(0);
    let witness = witness_commitment(0);
    let first_witness_segment = witness_segment(0);
    let second_witness_segment = witness_segment(1);
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        values: Vec::new(),
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
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
    let nonce_segment = build_pcs_query_nonce_segment_from_transcript_segments(&schedule, input)
        .expect("query nonce should build");
    let result = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        &[first_witness_segment, second_witness_segment],
        input,
        &nonce_segment,
    );

    assert!(matches!(
        result,
        Err(
            ProvePcsQueryPlanSegmentError::TranscriptWitnessUnitCountMismatch {
                expected: 1,
                found: 2
            }
        )
    ));
}

#[test]
fn rejects_transcript_query_plan_builder_with_mismatched_witness_unit() {
    let mut schedule = sample_schedule();
    let mut second_unit = sample_unit();
    second_unit.unit_id = Some(1);
    second_unit.unit_name = Some("unit-b".to_owned());
    schedule.unit_count = 2;
    schedule.pcs_material_unit_count = 2;
    schedule.total_query_count = 4;
    schedule.units.push(second_unit);

    let material = material_unit(0);
    let witness = witness_commitment(0);
    let witness_segment = witness_segment(1);
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        values: Vec::new(),
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
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
    let nonce_segment = build_pcs_query_nonce_segment_from_transcript_segments(&schedule, input)
        .expect("query nonce should build");
    let result = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        input,
        &nonce_segment,
    );

    assert!(matches!(
        result,
        Err(
            ProvePcsQueryPlanSegmentError::TranscriptWitnessUnitMismatch {
                input_unit_index: 0,
                witness_unit_index: 1
            }
        )
    ));
}

#[test]
fn rejects_transcript_query_plan_builder_with_mismatched_witness_trace_instance() {
    let schedule = sample_schedule();
    let material = material_unit(0);
    let witness = witness_commitment(0);
    let witness_segment = witness_segment_with_trace_instance(0, 1, schedule.units.len());
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        values: Vec::new(),
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
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
    let nonce_segment = build_pcs_query_nonce_segment_from_transcript_segments(&schedule, input)
        .expect("query nonce should build");
    let result = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        input,
        &nonce_segment,
    );

    assert!(matches!(
        result,
        Err(
            ProvePcsQueryPlanSegmentError::TranscriptWitnessTraceInstanceMismatch {
                input_trace_instance_index: 0,
                witness_trace_instance_index: 1
            }
        )
    ));
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
            trace_instance_index: 0,
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
        trace_instance_index: 0,
        values: vec![[9, 10, 11]],
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
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

#[test]
fn rejects_transcript_pcs_query_plan_mismatches_with_challenge_values_segment() {
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
        trace_instance_index: 0,
        values: vec![[9, 10, 11]],
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        layers: Vec::new(),
        final_polynomial: vec![[12, 13, 14]],
    };
    let good_challenge_segment = challenge_values_segment([1, 2, 3]);
    let bad_challenge_segment = challenge_values_segment([4, 5, 6]);
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
        binding_segments: std::slice::from_ref(&good_challenge_segment),
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
        binding_segments: std::slice::from_ref(&good_challenge_segment),
    };
    let query = build_pcs_query_plan_segment_from_transcript_segments(
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
        bad_challenge_segment,
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

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("query plan mismatch should be rejected");

    assert_eq!(error, ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch);
}

#[test]
fn rejects_transcript_pcs_query_plan_extra_evaluation_units() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    let evaluation_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
        .expect("evaluation segment should exist");
    evaluation_segment.data = encode_pcs_evaluation_segment(&PcsEvaluationSegment {
        units: vec![
            PcsEvaluationUnitSegment {
                unit_index: 0,
                trace_instance_index: 0,
                values: vec![[9, 10, 11]],
            },
            PcsEvaluationUnitSegment {
                unit_index: 1,
                trace_instance_index: 0,
                values: vec![[19, 20, 21]],
            },
        ],
    })
    .expect("evaluation segment should encode");

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("extra evaluation unit should be rejected");

    assert_eq!(
        error.to_string(),
        "unexpected PCS evaluation segment unit 1"
    );
}

#[test]
fn rejects_transcript_pcs_query_plan_extra_fri_opening_units() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    let fri_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .expect("FRI opening segment should exist");
    fri_segment.data = encode_pcs_fri_opening_segment(&PcsFriOpeningSegment {
        units: vec![
            PcsFriOpeningUnitSegment {
                unit_index: 0,
                trace_instance_index: 0,
                layers: Vec::new(),
                final_polynomial: vec![[12, 13, 14]],
            },
            PcsFriOpeningUnitSegment {
                unit_index: 1,
                trace_instance_index: 0,
                layers: Vec::new(),
                final_polynomial: vec![[22, 23, 24]],
            },
        ],
    })
    .expect("FRI opening segment should encode");

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("extra FRI opening unit should be rejected");

    assert_eq!(
        error.to_string(),
        "unexpected PCS FRI opening segment unit 1"
    );
}

#[test]
fn rejects_transcript_pcs_query_plan_extra_unit_values_units() {
    let (schedule, mut segments) = transcript_query_plan_segments();
    segments.push(ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: encode_unit_values_segment(&UnitValuesSegment {
            units: vec![UnitValuesUnitSegment {
                unit_index: 1,
                trace_instance_index: 0,
                values: vec![31],
            }],
        })
        .expect("unit values segment should encode"),
    });

    let error = validate_transcript_pcs_query_plan_segments(&schedule, &[], &segments)
        .expect_err("extra unit values unit should be rejected");

    assert_eq!(
        error.to_string(),
        "unexpected unit values segment for unit 1"
    );
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
        trace_instance_index: 0,
        values: vec![[9, 10, 11]],
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
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

fn replace_query_plan_trace_instance(segments: &mut [ProofSegment], trace_instance_index: u32) {
    let query_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .expect("query segment should exist");
    query_segment.data = encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
        units: vec![PcsQueryPlanUnit {
            unit_index: 0,
            trace_instance_index,
            queries: vec![3, 5],
        }],
    })
    .expect("query plan should encode");
}

fn material_segment() -> ProofSegment {
    material_manifest_segment(vec![material_unit(0)])
}

fn material_manifest_segment(units: Vec<PcsMaterialManifestUnit>) -> ProofSegment {
    ProofSegment {
        id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        data: encode_pcs_material_manifest_segment(&PcsMaterialManifestSegment { units })
            .expect("material segment should encode"),
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

fn witness_segment_with_trace_instance(
    unit_index: u32,
    trace_instance_index: u32,
    unit_count: usize,
) -> ProofSegment {
    witness_segment_with_trace_instance_and_root(
        unit_index,
        trace_instance_index,
        unit_count,
        [5, 6, 7, 8],
    )
}

fn witness_segment_with_trace_instance_and_root(
    unit_index: u32,
    trace_instance_index: u32,
    unit_count: usize,
    root: [u64; 4],
) -> ProofSegment {
    let unit_count = u32::try_from(unit_count).expect("unit count should fit u32");
    let witness = witness_commitment_with_root(unit_index, root);
    ProofSegment {
        id: witness_commitment_segment_id(
            unit_count,
            WitnessCommitmentSegmentIdentity {
                unit_index,
                trace_instance_index,
            },
        )
        .expect("witness segment id should encode"),
        data: encode_witness_commitment_segment(&witness).expect("witness segment should encode"),
    }
}

fn witness_commitment(unit_index: u32) -> WitnessCommitmentSegment {
    witness_commitment_with_root(unit_index, [5, 6, 7, 8])
}

fn witness_commitment_with_root(unit_index: u32, root: [u64; 4]) -> WitnessCommitmentSegment {
    WitnessCommitmentSegment {
        unit_index,
        input_byte_count: 0,
        trace_rows: 2,
        trace_columns: 1,
        stages: vec![WitnessCommitmentStageSegment {
            stage_index: 1,
            arity: 2,
            root,
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

fn challenge_values_segment(values: [u64; 3]) -> ProofSegment {
    ProofSegment {
        id: CHALLENGE_VALUES_SEGMENT_ID,
        data: encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![values],
        })
        .expect("challenge values segment should encode"),
    }
}

fn input_binding_segment(id: u32, data: &[u8]) -> ProofSegment {
    ProofSegment {
        id,
        data: data.to_vec(),
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

fn query_sensitive_schedule() -> ProveSchedule {
    let mut schedule = sample_schedule();
    schedule.total_query_count = 16;
    schedule.max_extended_domain_bits = 8;
    schedule.units[0].extended_domain_bits = 8;
    schedule.units[0].extended_domain_size = 256;
    schedule.units[0].query_count = 16;
    schedule
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
        pcs_material_bytes: Some(0),
        pcs_material_plan_digest: Some([1; 32]),
        pcs_material_fixed_column_digest: Some([2; 32]),
        pcs_material_constant_tree_digest: Some([3; 32]),
        pcs_material_constant_tree_root: Some([1, 2, 3, 4]),
        pcs_material_fixed_byte_count: Some(0),
        pcs_material_constant_tree_byte_count: Some(0),
        pcs_material_leaf_byte_count: Some(0),
        pcs_material_node_byte_count: Some(0),
    }
}

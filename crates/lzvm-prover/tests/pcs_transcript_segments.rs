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
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::pcs_transcript::{derive_pcs_transcript_challenges, PcsTranscriptInputs};
use lzvm_prover::pcs_transcript_segments::{
    derive_pcs_transcript_challenges_from_proof_segments,
    derive_pcs_transcript_unit_challenges_from_loaded_witness_segments,
    derive_pcs_transcript_unit_challenges_from_proof_segments, PcsTranscriptProofSegmentsError,
};
use lzvm_prover::witness_commitment::load_witness_commitment_segment_refs_with_shapes;
use lzvm_prover::{ProveSchedule, ProveUnitSchedule};

#[test]
fn derives_transcript_challenges_from_proof_segments() {
    let schedule = sample_schedule();
    let public_values = values(&[7, 8]);
    let segments = transcript_segments(0);

    let actual =
        derive_pcs_transcript_challenges_from_proof_segments(&schedule, &public_values, &segments)
            .expect("challenges should derive");
    let expected = derive_pcs_transcript_challenges(PcsTranscriptInputs {
        arity: 4,
        hash_values: false,
        constant_root: root(1),
        public_values: &public_values,
        witness_roots: &[root(10)],
        root_challenge_draws: &[2],
        unit_value_map: &[],
        unit_values: &[],
        evaluation_values: &[ext(20)],
        evaluation_challenge_draws: 1,
        fri_roots: &[],
        final_polynomial: &[ext(40)],
        binding_segments: &[],
    })
    .expect("expected challenges should derive");

    assert_eq!(actual, expected);
}

#[test]
fn derives_unit_transcript_challenges_from_proof_segments() {
    let schedule = sample_schedule();
    let public_values = values(&[7, 8]);
    let segments = transcript_segments(0);

    let actual = derive_pcs_transcript_unit_challenges_from_proof_segments(
        &schedule,
        &public_values,
        &segments,
    )
    .expect("unit challenges should derive");
    let expected = derive_pcs_transcript_challenges(PcsTranscriptInputs {
        arity: 4,
        hash_values: false,
        constant_root: root(1),
        public_values: &public_values,
        witness_roots: &[root(10)],
        root_challenge_draws: &[2],
        unit_value_map: &[],
        unit_values: &[],
        evaluation_values: &[ext(20)],
        evaluation_challenge_draws: 1,
        fri_roots: &[],
        final_polynomial: &[ext(40)],
        binding_segments: &[],
    })
    .expect("expected challenges should derive");

    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].unit_index, 0);
    assert_eq!(actual[0].challenges, expected);
}

#[test]
fn rejects_transcript_challenge_query_unit_mismatches() {
    let schedule = sample_schedule();
    let segments = transcript_segments(1);

    let error = derive_pcs_transcript_challenges_from_proof_segments(&schedule, &[], &segments)
        .expect_err("unit mismatch should be rejected");

    assert_eq!(
        error,
        PcsTranscriptProofSegmentsError::UnitMismatch { unit_index: 1 }
    );
}

#[test]
fn derives_trace_instance_transcript_challenge_queries() {
    let schedule = sample_schedule();
    let mut segments = transcript_segments(0);
    replace_query_plan_trace_instance(&mut segments, 1);
    replace_witness_commitment_trace_instance(&mut segments, 1);
    replace_evaluation_trace_instance(&mut segments, 1);
    replace_fri_opening_trace_instance(&mut segments, 1);

    let actual =
        derive_pcs_transcript_unit_challenges_from_proof_segments(&schedule, &[], &segments)
            .expect("trace instance challenges should derive");

    assert_eq!(actual[0].unit_index, 0);
    assert_eq!(actual[0].trace_instance_index, 1);
}

#[test]
fn derives_loaded_witness_transcript_challenge_by_trace_identity() {
    let schedule = sample_schedule();
    let mut segments = transcript_segments(0);
    replace_query_plan_trace_instance(&mut segments, 1);
    replace_evaluation_trace_instance(&mut segments, 1);
    replace_fri_opening_trace_instance(&mut segments, 1);
    segments.push(ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1,
        data: encode_witness_commitment_segment(&witness_unit_with_root(0, 77))
            .expect("trace witness segment should encode"),
    });
    let witness_segments =
        load_witness_commitment_segment_refs_with_shapes(&schedule.units, &segments)
            .expect("witness segments should load");

    let actual = derive_pcs_transcript_unit_challenges_from_loaded_witness_segments(
        &schedule,
        &[],
        &segments,
        &witness_segments,
    )
    .expect("loaded witness trace challenges should derive");
    let expected = derive_pcs_transcript_challenges(PcsTranscriptInputs {
        arity: 4,
        hash_values: false,
        constant_root: root(1),
        public_values: &[],
        witness_roots: &[root(77)],
        root_challenge_draws: &[2],
        unit_value_map: &[],
        unit_values: &[],
        evaluation_values: &[ext(20)],
        evaluation_challenge_draws: 1,
        fri_roots: &[],
        final_polynomial: &[ext(40)],
        binding_segments: &[],
    })
    .expect("expected trace challenges should derive");

    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].unit_index, 0);
    assert_eq!(actual[0].trace_instance_index, 1);
    assert_eq!(actual[0].challenges, expected);
}

#[test]
fn rejects_transcript_challenge_extra_fri_opening_units() {
    let schedule = sample_schedule();
    let mut segments = transcript_segments(0);
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
                final_polynomial: vec![ext_words(40)],
            },
            PcsFriOpeningUnitSegment {
                unit_index: 1,
                trace_instance_index: 0,
                layers: Vec::new(),
                final_polynomial: vec![ext_words(50)],
            },
        ],
    })
    .expect("FRI opening segment should encode");

    let error =
        derive_pcs_transcript_unit_challenges_from_proof_segments(&schedule, &[], &segments)
            .expect_err("extra FRI opening unit should be rejected");

    assert_eq!(
        error.to_string(),
        "unexpected PCS FRI opening segment unit 1"
    );
}

#[test]
fn rejects_transcript_challenge_extra_unit_values_units() {
    let schedule = sample_schedule();
    let mut segments = transcript_segments(0);
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

    let error =
        derive_pcs_transcript_unit_challenges_from_proof_segments(&schedule, &[], &segments)
            .expect_err("extra unit values unit should be rejected");

    assert_eq!(
        error.to_string(),
        "unexpected unit values segment for unit 1"
    );
}

#[test]
fn rejects_duplicate_transcript_material_segments() {
    let schedule = sample_schedule();
    let mut segments = transcript_segments(0);
    let duplicate = segments[0].clone();
    segments.insert(1, duplicate);

    let error = derive_pcs_transcript_challenges_from_proof_segments(&schedule, &[], &segments)
        .expect_err("duplicate material segment should be rejected");

    assert_eq!(error.to_string(), "duplicate PCS material manifest segment");
}

#[test]
fn rejects_duplicate_transcript_binding_segments() {
    let schedule = sample_schedule();
    let mut segments = transcript_segments(0);
    let cache_segment = ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: encode_program_image_cache_segment(&sample_program_image_cache())
            .expect("cache should encode"),
    };
    segments.push(cache_segment.clone());
    segments.push(cache_segment);

    let error = derive_pcs_transcript_challenges_from_proof_segments(&schedule, &[], &segments)
        .expect_err("duplicate binding segment should be rejected");

    assert_eq!(
        error.to_string(),
        format!(
            "duplicate proof binding segment id: {}",
            PROGRAM_IMAGE_CACHE_SEGMENT_ID
        )
    );
}

fn transcript_segments(query_unit_index: u32) -> Vec<ProofSegment> {
    vec![
        ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: encode_pcs_material_manifest_segment(&PcsMaterialManifestSegment {
                units: vec![material_unit(0)],
            })
            .expect("material segment should encode"),
        },
        ProofSegment {
            id: PCS_QUERY_PLAN_SEGMENT_ID,
            data: encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
                units: vec![PcsQueryPlanUnit {
                    unit_index: query_unit_index,
                    trace_instance_index: 0,
                    queries: vec![3, 5],
                }],
            })
            .expect("query plan should encode"),
        },
        ProofSegment {
            id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
            data: encode_witness_commitment_segment(&witness_unit(0))
                .expect("witness segment should encode"),
        },
        ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: encode_pcs_evaluation_segment(&PcsEvaluationSegment {
                units: vec![PcsEvaluationUnitSegment {
                    unit_index: 0,
                    trace_instance_index: 0,
                    values: vec![ext_words(20)],
                }],
            })
            .expect("evaluation segment should encode"),
        },
        ProofSegment {
            id: PCS_FRI_OPENING_SEGMENT_ID,
            data: encode_pcs_fri_opening_segment(&PcsFriOpeningSegment {
                units: vec![PcsFriOpeningUnitSegment {
                    unit_index: 0,
                    trace_instance_index: 0,
                    layers: Vec::new(),
                    final_polynomial: vec![ext_words(40)],
                }],
            })
            .expect("FRI segment should encode"),
        },
    ]
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

fn replace_witness_commitment_trace_instance(
    segments: &mut [ProofSegment],
    trace_instance_index: u32,
) {
    let witness_segment = segments
        .iter_mut()
        .find(|segment| segment.id == WITNESS_COMMITMENT_SEGMENT_BASE_ID)
        .expect("witness segment should exist");
    witness_segment.id = WITNESS_COMMITMENT_SEGMENT_BASE_ID + trace_instance_index;
}

fn replace_evaluation_trace_instance(segments: &mut [ProofSegment], trace_instance_index: u32) {
    let evaluation_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
        .expect("evaluation segment should exist");
    let mut evaluations = lzvm_artifacts::pcs_evaluation_segment::parse_pcs_evaluation_segment(
        &evaluation_segment.data,
    )
    .expect("evaluation segment should parse");
    evaluations.units[0].trace_instance_index = trace_instance_index;
    evaluation_segment.data =
        encode_pcs_evaluation_segment(&evaluations).expect("evaluation segment should encode");
}

fn replace_fri_opening_trace_instance(segments: &mut [ProofSegment], trace_instance_index: u32) {
    let fri_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .expect("FRI opening segment should exist");
    let mut fri = lzvm_artifacts::pcs_fri_segment::parse_pcs_fri_opening_segment(&fri_segment.data)
        .expect("FRI opening segment should parse");
    fri.units[0].trace_instance_index = trace_instance_index;
    fri_segment.data = encode_pcs_fri_opening_segment(&fri).expect("FRI segment should encode");
}

fn sample_schedule() -> ProveSchedule {
    ProveSchedule {
        setup_hash: [0; 32],
        unit_count: 1,
        total_fixed_bytes: 64,
        total_pcs_material_bytes: 128,
        pcs_material_unit_count: 1,
        total_query_count: 2,
        max_extended_domain_bits: 6,
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
        base_domain_bits: 3,
        extended_domain_bits: 6,
        base_domain_size: 8,
        extended_domain_size: 64,
        blowup_factor: 8,
        query_count: 2,
        proof_of_work_bits: 0,
        merkle_tree_arity: 4,
        last_level_verification: 0,
        transcript_arity: Some(4),
        hash_commits: false,
        transcript_root_challenge_draws: vec![2],
        challenge_count: 5,
        evaluation_value_count: 1,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 1,
        constant_width: 1,
        stage_commit_widths: vec![1],
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: 6,
            output_bits: 3,
            folding_factor: 8,
        }],
        final_layer_bits: 3,
        fixed_bytes: 64,
        constant_tree_root: None,
        pcs_material_bytes: Some(128),
        pcs_material_plan_digest: Some([1; 32]),
        pcs_material_fixed_column_digest: Some([2; 32]),
        pcs_material_constant_tree_digest: Some([3; 32]),
        pcs_material_constant_tree_root: Some(root_words(1)),
        pcs_material_fixed_byte_count: Some(64),
        pcs_material_constant_tree_byte_count: Some(128),
        pcs_material_leaf_byte_count: Some(64),
        pcs_material_node_byte_count: Some(64),
    }
}

fn material_unit(unit_index: u32) -> PcsMaterialManifestUnit {
    PcsMaterialManifestUnit {
        unit_index,
        plan_digest: [1; 32],
        fixed_column_digest: [2; 32],
        constant_tree_digest: [3; 32],
        constant_tree_root: root_words(1),
        fixed_byte_count: 64,
        constant_tree_byte_count: 128,
        leaf_byte_count: 64,
        node_byte_count: 64,
    }
}

fn witness_unit(unit_index: u32) -> WitnessCommitmentSegment {
    witness_unit_with_root(unit_index, 10)
}

fn witness_unit_with_root(unit_index: u32, root_seed: u64) -> WitnessCommitmentSegment {
    WitnessCommitmentSegment {
        unit_index,
        input_byte_count: 0,
        trace_rows: 8,
        trace_columns: 1,
        stages: vec![WitnessCommitmentStageSegment {
            stage_index: 1,
            arity: 4,
            root: root_words(root_seed),
            tree_byte_count: 64,
            tree_digest: [0; 32],
        }],
    }
}

fn values(items: &[u64]) -> Vec<Felt> {
    items.iter().copied().map(Felt::from_u64).collect()
}

fn root(seed: u64) -> [Felt; 4] {
    [
        Felt::from_u64(seed),
        Felt::from_u64(seed + 1),
        Felt::from_u64(seed + 2),
        Felt::from_u64(seed + 3),
    ]
}

fn ext(seed: u64) -> Ext3 {
    Ext3::from_u64s(ext_words(seed))
}

fn root_words(seed: u64) -> [u64; 4] {
    [seed, seed + 1, seed + 2, seed + 3]
}

fn ext_words(seed: u64) -> [u64; 3] {
    [seed, seed + 1, seed + 2]
}

fn sample_program_image_cache() -> ProgramImageCommitmentCache {
    ProgramImageCommitmentCache {
        program_digest: [1; 32],
        source_image_digest: [2; 32],
        constraint_system_digest: [3; 32],
        tree_root: [4, 5, 6, 7],
        trace_row_count: 8,
        trace_column_count: 2,
        blowup_factor: 2,
        merkle_tree_arity: 2,
        gpu_mode: ProgramImageGpuMode::Cuda,
    }
}

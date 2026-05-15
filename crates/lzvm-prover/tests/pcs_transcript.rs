use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_evaluation_segment::PcsEvaluationUnitSegment;
use lzvm_artifacts::pcs_fri_segment::{PcsFriOpeningLayerSegment, PcsFriOpeningUnitSegment};
use lzvm_artifacts::pcs_material_segment::PcsMaterialManifestUnit;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::witness_segment::{WitnessCommitmentSegment, WitnessCommitmentStageSegment};
use lzvm_field::{Ext3, Felt, PoseidonTranscript, TranscriptError};
use lzvm_prover::pcs_transcript::{
    absorb_commit_values, derive_pcs_final_query_challenge,
    derive_pcs_final_query_challenge_from_segments, derive_pcs_transcript_challenges,
    derive_pcs_transcript_challenges_from_segments, PcsTranscriptError, PcsTranscriptInputs,
    PcsTranscriptSegmentInputs,
};
use lzvm_prover::ProveUnitSchedule;

fn values(items: &[u64]) -> Vec<Felt> {
    items.iter().copied().map(Felt::from_u64).collect()
}

#[test]
fn direct_commit_values_match_plain_transcript_absorption() {
    let mut actual = PoseidonTranscript::new(4).expect("arity should be supported");
    actual.put(&values(&[1, 2, 3]));

    absorb_commit_values(&mut actual, 4, false, &values(&[10, 20, 30, 40, 50]))
        .expect("direct absorption should succeed");

    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    expected.put(&values(&[1, 2, 3]));
    expected.put(&values(&[10, 20, 30, 40, 50]));

    assert_eq!(actual.get_field(), expected.get_field());
}

#[test]
fn hashed_commit_values_absorb_inner_state_words() {
    let commit_values = values(&[10, 20, 30, 40, 50]);
    let mut actual = PoseidonTranscript::new(4).expect("arity should be supported");
    actual.put(&values(&[1, 2, 3]));

    absorb_commit_values(&mut actual, 4, true, &commit_values)
        .expect("hashed absorption should succeed");

    let mut inner = PoseidonTranscript::new(4).expect("arity should be supported");
    inner.put(&commit_values);
    let state = inner.get_state();
    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    expected.put(&values(&[1, 2, 3]));
    expected.put(&state);

    assert_eq!(actual.get_field(), expected.get_field());
}

#[test]
fn hashed_commit_values_reject_unsupported_inner_arities() {
    let mut transcript = PoseidonTranscript::new(4).expect("arity should be supported");

    assert_eq!(
        absorb_commit_values(&mut transcript, 3, true, &values(&[1])),
        Err(PcsTranscriptError::Transcript(
            TranscriptError::UnsupportedArity { arity: 3 }
        ))
    );
}

#[test]
fn derives_final_query_challenge_from_direct_transcript_events() {
    let constant_root = root(1);
    let public_values = values(&[7, 8]);
    let witness_roots = vec![root(10), root(20), root(30)];
    let evaluations = vec![ext(40), ext(50)];
    let fri_roots = vec![root(60), root(70)];
    let final_polynomial = vec![ext(80), ext(90)];

    let actual = derive_pcs_final_query_challenge(PcsTranscriptInputs {
        arity: 4,
        hash_values: false,
        constant_root,
        public_values: &public_values,
        witness_roots: &witness_roots,
        root_challenge_draws: &[2, 1, 1],
        evaluation_values: &evaluations,
        evaluation_challenge_draws: 2,
        fri_roots: &fri_roots,
        final_polynomial: &final_polynomial,
    })
    .expect("challenge should derive");

    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    expected.put(&constant_root);
    expected.put(&public_values);
    put_root_and_draw(&mut expected, &witness_roots[0], 2);
    put_root_and_draw(&mut expected, &witness_roots[1], 1);
    put_root_and_draw(&mut expected, &witness_roots[2], 1);
    expected.put(&flatten_ext(&evaluations));
    draw(&mut expected, 2);
    expected.put(&fri_roots[0]);
    draw(&mut expected, 1);
    expected.put(&fri_roots[1]);
    draw(&mut expected, 1);
    expected.put(&flatten_ext(&final_polynomial));

    assert_eq!(actual, expected.get_field());
}

#[test]
fn derives_indexed_transcript_challenges_from_direct_events() {
    let constant_root = root(1);
    let public_values = values(&[7, 8]);
    let witness_roots = vec![root(10), root(20), root(30)];
    let evaluations = vec![ext(40), ext(50)];
    let fri_roots = vec![root(60), root(70)];
    let final_polynomial = vec![ext(80), ext(90)];

    let actual = derive_pcs_transcript_challenges(PcsTranscriptInputs {
        arity: 4,
        hash_values: false,
        constant_root,
        public_values: &public_values,
        witness_roots: &witness_roots,
        root_challenge_draws: &[2, 1, 1],
        evaluation_values: &evaluations,
        evaluation_challenge_draws: 2,
        fri_roots: &fri_roots,
        final_polynomial: &final_polynomial,
    })
    .expect("challenges should derive");

    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    let mut expected_challenges = Vec::new();
    expected.put(&constant_root);
    expected.put(&public_values);
    put_root_and_record(
        &mut expected,
        &witness_roots[0],
        2,
        &mut expected_challenges,
    );
    put_root_and_record(
        &mut expected,
        &witness_roots[1],
        1,
        &mut expected_challenges,
    );
    put_root_and_record(
        &mut expected,
        &witness_roots[2],
        1,
        &mut expected_challenges,
    );
    expected.put(&flatten_ext(&evaluations));
    record(&mut expected, 2, &mut expected_challenges);
    expected_challenges.push(Ext3::ZERO);
    expected.put(&fri_roots[0]);
    expected_challenges.push(expected.get_field());
    expected.put(&fri_roots[1]);
    expected_challenges.push(expected.get_field());
    expected.put(&flatten_ext(&final_polynomial));
    expected_challenges.push(expected.get_field());

    assert_eq!(actual, expected_challenges);
    assert_eq!(
        actual.last().copied(),
        Some(
            derive_pcs_final_query_challenge(PcsTranscriptInputs {
                arity: 4,
                hash_values: false,
                constant_root,
                public_values: &public_values,
                witness_roots: &witness_roots,
                root_challenge_draws: &[2, 1, 1],
                evaluation_values: &evaluations,
                evaluation_challenge_draws: 2,
                fri_roots: &fri_roots,
                final_polynomial: &final_polynomial,
            })
            .expect("final challenge should derive")
        )
    );
}

#[test]
fn derives_final_query_challenge_from_hashed_transcript_events() {
    let constant_root = root(2);
    let public_values = values(&[3, 4, 5]);
    let witness_roots = vec![root(10), root(20), root(30)];
    let evaluations = vec![ext(40), ext(50), ext(60)];
    let final_polynomial = vec![ext(70), ext(80)];

    let actual = derive_pcs_final_query_challenge(PcsTranscriptInputs {
        arity: 4,
        hash_values: true,
        constant_root,
        public_values: &public_values,
        witness_roots: &witness_roots,
        root_challenge_draws: &[2, 1, 1],
        evaluation_values: &evaluations,
        evaluation_challenge_draws: 2,
        fri_roots: &[],
        final_polynomial: &final_polynomial,
    })
    .expect("challenge should derive");

    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    expected.put(&constant_root);
    absorb_commit_values(&mut expected, 4, true, &public_values)
        .expect("public values should absorb");
    put_root_and_draw(&mut expected, &witness_roots[0], 2);
    put_root_and_draw(&mut expected, &witness_roots[1], 1);
    put_root_and_draw(&mut expected, &witness_roots[2], 1);
    absorb_commit_values(&mut expected, 4, true, &flatten_ext(&evaluations))
        .expect("evaluations should absorb");
    draw(&mut expected, 2);
    absorb_commit_values(&mut expected, 4, true, &flatten_ext(&final_polynomial))
        .expect("final polynomial should absorb");

    assert_eq!(actual, expected.get_field());
}

#[test]
fn rejects_root_challenge_draw_mismatches() {
    assert_eq!(
        derive_pcs_final_query_challenge(PcsTranscriptInputs {
            arity: 4,
            hash_values: false,
            constant_root: root(1),
            public_values: &[],
            witness_roots: &[root(10)],
            root_challenge_draws: &[1, 2],
            evaluation_values: &[],
            evaluation_challenge_draws: 0,
            fri_roots: &[],
            final_polynomial: &[ext(20)],
        }),
        Err(PcsTranscriptError::RootChallengeDrawMismatch {
            root_count: 1,
            draw_count: 2
        })
    );
}

#[test]
fn rejects_empty_final_polynomials() {
    assert_eq!(
        derive_pcs_final_query_challenge(PcsTranscriptInputs {
            arity: 4,
            hash_values: false,
            constant_root: root(1),
            public_values: &[],
            witness_roots: &[],
            root_challenge_draws: &[],
            evaluation_values: &[],
            evaluation_challenge_draws: 0,
            fri_roots: &[],
            final_polynomial: &[],
        }),
        Err(PcsTranscriptError::EmptyFinalPolynomial)
    );
}

#[test]
fn derives_final_query_challenge_from_parsed_segments() {
    let unit = sample_unit(Some(4), true);
    let material = sample_material(0, 1);
    let witness = sample_witness(0, &[10, 20, 30]);
    let evaluations = sample_evaluations(0, &[40, 50]);
    let fri = sample_fri(0, &[60, 70], &[80, 90]);
    let public_values = values(&[7, 8]);

    let actual = derive_pcs_final_query_challenge_from_segments(PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &unit,
        material: &material,
        public_values: &public_values,
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri,
        root_challenge_draws: &[2, 1, 1],
        evaluation_challenge_draws: 2,
    })
    .expect("challenge should derive from segments");

    let expected = derive_pcs_final_query_challenge(PcsTranscriptInputs {
        arity: 4,
        hash_values: true,
        constant_root: root(1),
        public_values: &public_values,
        witness_roots: &[root(10), root(20), root(30)],
        root_challenge_draws: &[2, 1, 1],
        evaluation_values: &[ext(40), ext(50)],
        evaluation_challenge_draws: 2,
        fri_roots: &[root(60), root(70)],
        final_polynomial: &[ext(80), ext(90)],
    })
    .expect("generic challenge should derive");

    assert_eq!(actual, expected);
}

#[test]
fn derives_indexed_transcript_challenges_from_parsed_segments() {
    let unit = sample_unit(Some(4), false);
    let material = sample_material(0, 1);
    let witness = sample_witness(0, &[10, 20, 30]);
    let evaluations = sample_evaluations(0, &[40, 50]);
    let fri = sample_fri(0, &[60, 70], &[80, 90]);
    let public_values = values(&[7, 8]);

    let actual = derive_pcs_transcript_challenges_from_segments(PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &unit,
        material: &material,
        public_values: &public_values,
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri,
        root_challenge_draws: &[2, 1, 1],
        evaluation_challenge_draws: 2,
    })
    .expect("challenges should derive from segments");

    let expected = derive_pcs_transcript_challenges(PcsTranscriptInputs {
        arity: 4,
        hash_values: false,
        constant_root: root(1),
        public_values: &public_values,
        witness_roots: &[root(10), root(20), root(30)],
        root_challenge_draws: &[2, 1, 1],
        evaluation_values: &[ext(40), ext(50)],
        evaluation_challenge_draws: 2,
        fri_roots: &[root(60), root(70)],
        final_polynomial: &[ext(80), ext(90)],
    })
    .expect("generic challenges should derive");

    assert_eq!(actual, expected);
}

#[test]
fn segment_challenge_derivation_requires_transcript_arity() {
    let unit = sample_unit(None, true);

    assert_eq!(
        derive_pcs_final_query_challenge_from_segments(PcsTranscriptSegmentInputs {
            unit_index: 0,
            unit: &unit,
            material: &sample_material(0, 1),
            public_values: &[],
            witness: &sample_witness(0, &[10]),
            evaluations: &sample_evaluations(0, &[20]),
            fri: &sample_fri(0, &[], &[30]),
            root_challenge_draws: &[1],
            evaluation_challenge_draws: 1,
        }),
        Err(PcsTranscriptError::MissingTranscriptArity { unit_index: 0 })
    );
}

#[test]
fn segment_challenge_derivation_rejects_unit_mismatches() {
    let unit = sample_unit(Some(4), true);

    assert_eq!(
        derive_pcs_final_query_challenge_from_segments(PcsTranscriptSegmentInputs {
            unit_index: 0,
            unit: &unit,
            material: &sample_material(1, 1),
            public_values: &[],
            witness: &sample_witness(0, &[10]),
            evaluations: &sample_evaluations(0, &[20]),
            fri: &sample_fri(0, &[], &[30]),
            root_challenge_draws: &[1],
            evaluation_challenge_draws: 1,
        }),
        Err(PcsTranscriptError::SegmentUnitIndexMismatch {
            segment: "material",
            expected: 0,
            found: 1,
        })
    );
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
    Ext3::from_u64s([seed, seed + 1, seed + 2])
}

fn root_words(seed: u64) -> [u64; 4] {
    [seed, seed + 1, seed + 2, seed + 3]
}

fn ext_words(seed: u64) -> [u64; 3] {
    [seed, seed + 1, seed + 2]
}

fn flatten_ext(values: &[Ext3]) -> Vec<Felt> {
    values
        .iter()
        .flat_map(|value| [value.c0, value.c1, value.c2])
        .collect()
}

fn put_root_and_draw(transcript: &mut PoseidonTranscript, root: &[Felt; 4], count: usize) {
    transcript.put(root);
    draw(transcript, count);
}

fn put_root_and_record(
    transcript: &mut PoseidonTranscript,
    root: &[Felt; 4],
    count: usize,
    out: &mut Vec<Ext3>,
) {
    transcript.put(root);
    record(transcript, count, out);
}

fn draw(transcript: &mut PoseidonTranscript, count: usize) {
    for _ in 0..count {
        transcript.get_field();
    }
}

fn record(transcript: &mut PoseidonTranscript, count: usize, out: &mut Vec<Ext3>) {
    for _ in 0..count {
        out.push(transcript.get_field());
    }
}

fn sample_unit(transcript_arity: Option<u32>, hash_commits: bool) -> ProveUnitSchedule {
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits: 10,
        extended_domain_bits: 13,
        base_domain_size: 1024,
        extended_domain_size: 8192,
        blowup_factor: 8,
        query_count: 4,
        proof_of_work_bits: 20,
        merkle_tree_arity: 4,
        last_level_verification: 0,
        transcript_arity,
        hash_commits,
        transcript_root_challenge_draws: vec![2, 1, 1],
        evaluation_value_count: 2,
        transcript_evaluation_challenge_draws: 2,
        constant_width: 5,
        stage_commit_widths: vec![2, 3, 1],
        commitment_columns: Vec::new(),
        opening_points: vec![0, 1, -1],
        fri_layers: vec![PcsFriLayer {
            input_bits: 13,
            output_bits: 9,
            folding_factor: 16,
        }],
        final_layer_bits: 5,
        fixed_bytes: 40960,
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

fn sample_material(unit_index: u32, root_seed: u64) -> PcsMaterialManifestUnit {
    PcsMaterialManifestUnit {
        unit_index,
        plan_digest: [0; 32],
        fixed_column_digest: [1; 32],
        constant_tree_digest: [2; 32],
        constant_tree_root: root_words(root_seed),
        fixed_byte_count: 64,
        constant_tree_byte_count: 224,
        leaf_byte_count: 64,
        node_byte_count: 160,
    }
}

fn sample_witness(unit_index: u32, root_seeds: &[u64]) -> WitnessCommitmentSegment {
    let stages = root_seeds
        .iter()
        .enumerate()
        .map(|(index, seed)| WitnessCommitmentStageSegment {
            stage_index: (index + 1) as u32,
            arity: 4,
            root: root_words(*seed),
            tree_byte_count: 224,
            tree_digest: [index as u8; 32],
        })
        .collect();
    WitnessCommitmentSegment {
        unit_index,
        input_byte_count: 0,
        trace_rows: 1024,
        trace_columns: 6,
        stages,
    }
}

fn sample_evaluations(unit_index: u32, value_seeds: &[u64]) -> PcsEvaluationUnitSegment {
    PcsEvaluationUnitSegment {
        unit_index,
        values: value_seeds.iter().copied().map(ext_words).collect(),
    }
}

fn sample_fri(
    unit_index: u32,
    layer_root_seeds: &[u64],
    final_value_seeds: &[u64],
) -> PcsFriOpeningUnitSegment {
    PcsFriOpeningUnitSegment {
        unit_index,
        layers: layer_root_seeds
            .iter()
            .enumerate()
            .map(|(index, seed)| PcsFriOpeningLayerSegment {
                layer_index: index as u32,
                root: root_words(*seed),
                last_level: Vec::new(),
                queries: Vec::new(),
            })
            .collect(),
        final_polynomial: final_value_seeds.iter().copied().map(ext_words).collect(),
    }
}

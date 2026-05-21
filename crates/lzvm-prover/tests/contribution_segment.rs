use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, ContributionEntry, ContributionSegment, ContributionSegmentError,
    CONTRIBUTION_SEGMENT_ID,
};
use lzvm_artifacts::global_info::{CurveKind, GlobalAir, GlobalInfo, NamedStageValue};
use lzvm_artifacts::program_image_segment::PROGRAM_IMAGE_CACHE_SEGMENT_ID;
use lzvm_artifacts::proof::{
    encode_proof_artifact, parse_proof_artifact, ProofArtifact, ProofSegment,
};
use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_field::{poseidon2_hash_16, Felt, FieldError, PoseidonTranscript};
use lzvm_prover::contribution::{
    aggregate_contribution_values, build_contribution_segment, build_internal_contribution_input,
    derive_global_challenge_from_contributions, derive_global_challenge_from_proof_segments,
    derive_worker_contribution_entry, load_contribution_segment_from_segments,
    ContributionChallengeError, InternalContributionInput, LoadContributionSegmentError,
    ProveContributionEntry,
};

const FIRST_CONTRIBUTION_VALUE_OFFSET: usize = 12 + 4 + 4 + 4 + 4;

fn sample_entries() -> Vec<ProveContributionEntry> {
    vec![
        ProveContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(11), Felt::from_u64(22), Felt::from_u64(33)],
        },
        ProveContributionEntry {
            worker_index: 2,
            group_id: 1,
            aggregated: true,
            values: vec![Felt::from_u64(44), Felt::from_u64(55)],
        },
    ]
}

fn sample_global_info(lattice_size: u64, proof_values_map: Vec<NamedStageValue>) -> GlobalInfo {
    GlobalInfo {
        name: "sample".to_owned(),
        air_groups: vec!["main".to_owned()],
        airs: vec![vec![GlobalAir {
            name: "unit".to_owned(),
            num_rows: 16,
            has_compressor: false,
        }]],
        curve: CurveKind::None,
        lattice_size: Some(lattice_size),
        aggregation_types: vec![Vec::new()],
        n_publics: 0,
        num_challenges: Vec::new(),
        num_proof_values: Vec::new(),
        proof_values_map,
        publics_map: Vec::new(),
        transcript_arity: 4,
    }
}

fn proof_value(name: &str, stage: u64) -> NamedStageValue {
    array_proof_value(name, stage, &[1])
}

fn array_proof_value(name: &str, stage: u64, lengths: &[u64]) -> NamedStageValue {
    NamedStageValue {
        name: name.to_owned(),
        stage,
        id: None,
        lengths: lengths.to_vec(),
    }
}

fn stage_value(name: &str, stage: u32) -> StageValue {
    StageValue {
        name: name.to_owned(),
        stage,
        lengths: vec![1],
    }
}

fn expected_internal_contribution(
    root: [Felt; 4],
    values: &[Felt],
    lattice_size: usize,
) -> Vec<Felt> {
    let mut values_to_hash = values.to_vec();
    values_to_hash[4..8].copy_from_slice(&root);

    let mut hash_input = [Felt::ZERO; 16];
    hash_input[..values_to_hash.len()].copy_from_slice(&values_to_hash);
    let mut block = poseidon2_hash_16(hash_input);

    let mut out = Vec::with_capacity(lattice_size);
    while out.len() < lattice_size {
        out.extend_from_slice(&block);
        block = poseidon2_hash_16(block);
    }
    out.truncate(lattice_size);
    out
}

fn expected_internal_values(
    verification_key: [u64; 4],
    unit_value_map: &[StageValue],
    packed_unit_values: &[Felt],
) -> Vec<Felt> {
    let mut values = vec![
        Felt::from_u64(verification_key[0]),
        Felt::from_u64(verification_key[1]),
        Felt::from_u64(verification_key[2]),
        Felt::from_u64(verification_key[3]),
        Felt::ZERO,
        Felt::ZERO,
        Felt::ZERO,
        Felt::ZERO,
    ];
    let mut offset = 0_usize;
    for entry in unit_value_map {
        if entry.stage == 1 {
            values.push(packed_unit_values[offset]);
            offset += 1;
        } else {
            offset += 3;
        }
    }
    values
}

#[test]
fn round_trips_contribution_segments() {
    let entries = vec![
        ProveContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(1), Felt::from_u64(2), Felt::from_u64(3)],
        },
        ProveContributionEntry {
            worker_index: 1,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(4), Felt::from_u64(5), Felt::from_u64(6)],
        },
    ];
    let segment = build_contribution_segment(&entries)
        .expect("segment should build")
        .expect("segment should exist");

    assert_eq!(segment.id, CONTRIBUTION_SEGMENT_ID);
    assert_eq!(
        load_contribution_segment_from_segments(&[segment]).expect("segment should load"),
        entries
    );
}

#[test]
fn rejects_duplicate_contribution_segments() {
    let entries = sample_entries();
    let segment = build_contribution_segment(&entries)
        .expect("segment should build")
        .expect("segment should exist");

    let error = load_contribution_segment_from_segments(&[segment.clone(), segment])
        .expect_err("duplicate contribution segments should reject");

    assert_eq!(error.to_string(), "duplicate contribution segment");
}

#[test]
fn rejects_non_canonical_contribution_values() {
    let mut bytes = encode_contribution_segment(&ContributionSegment {
        entries: vec![ContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![7],
        }],
    })
    .expect("segment should encode");
    bytes[FIRST_CONTRIBUTION_VALUE_OFFSET..FIRST_CONTRIBUTION_VALUE_OFFSET + 8]
        .copy_from_slice(&u64::MAX.to_le_bytes());
    let segment = lzvm_artifacts::proof::ProofSegment {
        id: CONTRIBUTION_SEGMENT_ID,
        data: bytes,
    };

    assert!(matches!(
        load_contribution_segment_from_segments(&[segment]),
        Err(LoadContributionSegmentError::Segment(
            ContributionSegmentError::ValueNonCanonical {
                worker_index: 0,
                group_id: 0,
                value_index: 0,
                source: FieldError::NonCanonical { value: u64::MAX },
            }
        ))
    ));
}

#[test]
fn aggregates_lattice_contribution_values() {
    let global_info = sample_global_info(3, Vec::new());
    let entries = vec![
        ProveContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(1), Felt::from_u64(2), Felt::from_u64(3)],
        },
        ProveContributionEntry {
            worker_index: 1,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(4), Felt::from_u64(5), Felt::from_u64(6)],
        },
    ];

    assert_eq!(
        aggregate_contribution_values(&global_info, &entries)
            .expect("contributions should aggregate"),
        vec![Felt::from_u64(5), Felt::from_u64(7), Felt::from_u64(9)]
    );
}

#[test]
fn builds_internal_contribution_input_from_unit_metadata_values() {
    let root = [
        Felt::from_u64(501),
        Felt::from_u64(502),
        Felt::from_u64(503),
        Felt::from_u64(504),
    ];
    let verification_key = VerificationKeyRoot::FieldElements(vec![11, 22, 33, 44]);
    let unit_value_map = vec![
        stage_value("local_a", 1),
        stage_value("local_b", 2),
        stage_value("local_c", 1),
    ];
    let packed_unit_values = vec![
        Felt::from_u64(101),
        Felt::from_u64(201),
        Felt::from_u64(202),
        Felt::from_u64(203),
        Felt::from_u64(102),
    ];

    let input = build_internal_contribution_input(
        root,
        &verification_key,
        &unit_value_map,
        &packed_unit_values,
    )
    .expect("internal contribution input should build");

    assert_eq!(
        input,
        InternalContributionInput {
            root,
            values: expected_internal_values(
                [11, 22, 33, 44],
                &unit_value_map,
                &packed_unit_values
            ),
        }
    );
}

#[test]
fn rejects_internal_contribution_input_unit_value_count_mismatch() {
    let verification_key = VerificationKeyRoot::FieldElements(vec![11, 22, 33, 44]);
    let unit_value_map = vec![
        stage_value("local_a", 1),
        stage_value("local_b", 2),
        stage_value("local_c", 1),
    ];

    assert!(matches!(
        build_internal_contribution_input(
            [Felt::ZERO; 4],
            &verification_key,
            &unit_value_map,
            &[Felt::from_u64(101), Felt::from_u64(201)]
        ),
        Err(ContributionChallengeError::UnitValueCountMismatch {
            expected: 5,
            found: 2,
        })
    ));
}

#[test]
fn derives_worker_contribution_entry_from_internal_inputs() {
    let global_info = sample_global_info(32, Vec::new());
    let inputs = vec![
        InternalContributionInput {
            root: [
                Felt::from_u64(101),
                Felt::from_u64(102),
                Felt::from_u64(103),
                Felt::from_u64(104),
            ],
            values: (1_u64..=8).map(Felt::from_u64).collect(),
        },
        InternalContributionInput {
            root: [
                Felt::from_u64(201),
                Felt::from_u64(202),
                Felt::from_u64(203),
                Felt::from_u64(204),
            ],
            values: (11_u64..=18).map(Felt::from_u64).collect(),
        },
    ];
    let mut expected_values = vec![Felt::ZERO; 32];
    for input in &inputs {
        for (index, value) in expected_internal_contribution(input.root, &input.values, 32)
            .into_iter()
            .enumerate()
        {
            expected_values[index] = expected_values[index] + value;
        }
    }

    let entry = derive_worker_contribution_entry(&global_info, 3, 7, &inputs)
        .expect("worker contribution should derive");

    assert_eq!(
        entry,
        ProveContributionEntry {
            worker_index: 3,
            group_id: 7,
            aggregated: false,
            values: expected_values,
        }
    );
}

#[test]
fn rejects_internal_contribution_lattice_size_not_aligned_to_hash_width() {
    let global_info = sample_global_info(31, Vec::new());
    let inputs = vec![InternalContributionInput {
        root: [Felt::ZERO; 4],
        values: (1_u64..=8).map(Felt::from_u64).collect(),
    }];

    assert!(matches!(
        derive_worker_contribution_entry(&global_info, 0, 0, &inputs),
        Err(ContributionChallengeError::LatticeSizeNotMultipleOfHashState { value: 31 })
    ));
}

#[test]
fn rejects_zero_internal_contribution_lattice_size() {
    let global_info = sample_global_info(0, Vec::new());
    let inputs = vec![InternalContributionInput {
        root: [Felt::ZERO; 4],
        values: (1_u64..=8).map(Felt::from_u64).collect(),
    }];

    assert!(matches!(
        derive_worker_contribution_entry(&global_info, 0, 0, &inputs),
        Err(ContributionChallengeError::LatticeSizeNotMultipleOfHashState { value: 0 })
    ));
}

#[test]
fn rejects_internal_contribution_inputs_without_root_slots() {
    let global_info = sample_global_info(16, Vec::new());
    let inputs = vec![InternalContributionInput {
        root: [Felt::ZERO; 4],
        values: vec![Felt::from_u64(1); 7],
    }];

    assert!(matches!(
        derive_worker_contribution_entry(&global_info, 0, 0, &inputs),
        Err(ContributionChallengeError::ContributionInputTooShort {
            input_index: 0,
            found: 7,
        })
    ));
}

#[test]
fn derives_global_challenge_from_contributions() {
    let global_info = sample_global_info(
        3,
        vec![
            proof_value("stage_one_a", 1),
            proof_value("stage_two", 2),
            proof_value("stage_one_b", 1),
        ],
    );
    let public_values = vec![Felt::from_u64(3), Felt::from_u64(4)];
    let packed_proof_values = vec![
        Felt::from_u64(10),
        Felt::from_u64(20),
        Felt::from_u64(21),
        Felt::from_u64(22),
        Felt::from_u64(30),
    ];
    let entries = vec![
        ProveContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(1), Felt::from_u64(2), Felt::from_u64(3)],
        },
        ProveContributionEntry {
            worker_index: 1,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(4), Felt::from_u64(5), Felt::from_u64(6)],
        },
    ];

    let mut transcript = PoseidonTranscript::new(4).expect("transcript should build");
    transcript.put(&public_values);
    transcript.put(&[Felt::from_u64(10), Felt::from_u64(30)]);
    transcript.put(&[Felt::from_u64(5), Felt::from_u64(7), Felt::from_u64(9)]);
    let expected = transcript.get_field();

    assert_eq!(
        derive_global_challenge_from_contributions(
            &global_info,
            &public_values,
            &packed_proof_values,
            &entries,
        )
        .expect("global challenge should derive"),
        expected
    );
}

#[test]
fn derives_global_challenge_from_array_stage_one_proof_values() {
    let global_info = sample_global_info(
        3,
        vec![
            array_proof_value("stage_one_values", 1, &[2]),
            proof_value("stage_two", 2),
        ],
    );
    let public_values = vec![Felt::from_u64(3), Felt::from_u64(4)];
    let packed_proof_values = vec![
        Felt::from_u64(10),
        Felt::from_u64(11),
        Felt::from_u64(20),
        Felt::from_u64(21),
        Felt::from_u64(22),
    ];
    let entries = vec![
        ProveContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(1), Felt::from_u64(2), Felt::from_u64(3)],
        },
        ProveContributionEntry {
            worker_index: 1,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(4), Felt::from_u64(5), Felt::from_u64(6)],
        },
    ];

    let mut transcript = PoseidonTranscript::new(4).expect("transcript should build");
    transcript.put(&public_values);
    transcript.put(&[Felt::from_u64(10), Felt::from_u64(11)]);
    transcript.put(&[Felt::from_u64(5), Felt::from_u64(7), Felt::from_u64(9)]);
    let expected = transcript.get_field();

    assert_eq!(
        derive_global_challenge_from_contributions(
            &global_info,
            &public_values,
            &packed_proof_values,
            &entries,
        )
        .expect("global challenge should derive"),
        expected
    );
}

#[test]
fn derives_global_challenge_from_proof_segments() {
    let global_info = sample_global_info(
        3,
        vec![
            proof_value("stage_one_a", 1),
            proof_value("stage_two", 2),
            proof_value("stage_one_b", 1),
        ],
    );
    let public_values = vec![Felt::from_u64(3), Felt::from_u64(4)];
    let packed_proof_values = vec![
        Felt::from_u64(10),
        Felt::from_u64(20),
        Felt::from_u64(21),
        Felt::from_u64(22),
        Felt::from_u64(30),
    ];
    let entries = vec![
        ProveContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(1), Felt::from_u64(2), Felt::from_u64(3)],
        },
        ProveContributionEntry {
            worker_index: 1,
            group_id: 0,
            aggregated: false,
            values: vec![Felt::from_u64(4), Felt::from_u64(5), Felt::from_u64(6)],
        },
    ];
    let contribution_segment = build_contribution_segment(&entries)
        .expect("segment should build")
        .expect("segment should exist");
    let proof = ProofArtifact {
        setup_hash: [0; 32],
        public_values_hash: [0; 32],
        segments: vec![contribution_segment],
    };
    let proof = parse_proof_artifact(&encode_proof_artifact(&proof).expect("proof should encode"))
        .expect("proof should parse");

    let mut transcript = PoseidonTranscript::new(4).expect("transcript should build");
    transcript.put(&public_values);
    transcript.put(&[Felt::from_u64(10), Felt::from_u64(30)]);
    transcript.put(&[Felt::from_u64(5), Felt::from_u64(7), Felt::from_u64(9)]);
    let expected = transcript.get_field();

    assert_eq!(
        derive_global_challenge_from_proof_segments(
            &global_info,
            &public_values,
            &packed_proof_values,
            &proof.segments,
        )
        .expect("global challenge should derive"),
        expected
    );
}

#[test]
fn proof_segment_challenges_bind_program_image_segments() {
    let global_info = sample_global_info(3, Vec::new());
    let public_values = vec![Felt::from_u64(3), Felt::from_u64(4)];
    let entries = vec![ProveContributionEntry {
        worker_index: 0,
        group_id: 0,
        aggregated: false,
        values: vec![Felt::from_u64(1), Felt::from_u64(2), Felt::from_u64(3)],
    }];
    let contribution_segment = build_contribution_segment(&entries)
        .expect("segment should build")
        .expect("segment should exist");
    let mut first_segments = vec![
        contribution_segment.clone(),
        ProofSegment {
            id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
            data: vec![11, 12, 13],
        },
    ];
    let mut second_segments = vec![
        contribution_segment,
        ProofSegment {
            id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
            data: vec![21, 22, 23],
        },
    ];

    let first = derive_global_challenge_from_proof_segments(
        &global_info,
        &public_values,
        &[],
        &first_segments,
    )
    .expect("first challenge should derive");
    let second = derive_global_challenge_from_proof_segments(
        &global_info,
        &public_values,
        &[],
        &second_segments,
    )
    .expect("second challenge should derive");

    assert_ne!(first, second);

    first_segments.reverse();
    second_segments.reverse();
    assert_eq!(
        derive_global_challenge_from_proof_segments(
            &global_info,
            &public_values,
            &[],
            &first_segments
        )
        .expect("reordered first challenge should derive"),
        first
    );
    assert_eq!(
        derive_global_challenge_from_proof_segments(
            &global_info,
            &public_values,
            &[],
            &second_segments,
        )
        .expect("reordered second challenge should derive"),
        second
    );
}

#[test]
fn rejects_curve_contribution_aggregation_without_curve_ops() {
    let mut global_info = sample_global_info(10, Vec::new());
    global_info.curve = CurveKind::EcGfp5;

    assert!(matches!(
        aggregate_contribution_values(&global_info, &sample_entries()),
        Err(ContributionChallengeError::UnsupportedCurve {
            curve: CurveKind::EcGfp5
        })
    ));
}

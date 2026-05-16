use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, ContributionEntry, ContributionSegment, CONTRIBUTION_SEGMENT_ID,
};
use lzvm_artifacts::global_info::{CurveKind, GlobalAir, GlobalInfo, NamedStageValue};
use lzvm_artifacts::proof::{encode_proof_artifact, parse_proof_artifact, ProofArtifact};
use lzvm_field::{Felt, FieldError, PoseidonTranscript};
use lzvm_prover::contribution::{
    aggregate_contribution_values, build_contribution_segment,
    derive_global_challenge_from_contributions, derive_global_challenge_from_proof_segments,
    load_contribution_segment_from_segments, ContributionChallengeError,
    LoadContributionSegmentError, ProveContributionEntry,
};

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
    NamedStageValue {
        name: name.to_owned(),
        stage,
        id: None,
        lengths: vec![1],
    }
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
fn rejects_non_canonical_contribution_values() {
    let bytes = encode_contribution_segment(&ContributionSegment {
        entries: vec![ContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: vec![u64::MAX],
        }],
    })
    .expect("segment should encode");
    let segment = lzvm_artifacts::proof::ProofSegment {
        id: CONTRIBUTION_SEGMENT_ID,
        data: bytes,
    };

    assert!(matches!(
        load_contribution_segment_from_segments(&[segment]),
        Err(LoadContributionSegmentError::NonCanonicalValue {
            entry_index: 0,
            index: 0,
            source: FieldError::NonCanonical { value: u64::MAX },
        })
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

use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, parse_pcs_fri_opening_segment, PcsFriOpeningLayerSegment,
    PcsFriOpeningQuerySegment, PcsFriOpeningSegment, PcsFriOpeningSegmentError,
    PcsFriOpeningUnitSegment, PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{poseidon2_hash_8, Ext3, Felt, FieldError, MODULUS, SHIFT};
use lzvm_prover::pcs_fri::{
    build_pcs_fri_opening_unit, build_pcs_fri_opening_unit_with_timing,
    build_pcs_fri_transcript_commitments, load_pcs_fri_opening_segment_from_segments,
    load_pcs_fri_opening_unit_for_identity_from_segments, load_pcs_fri_opening_unit_from_segments,
    validate_optional_pcs_fri_opening_proof_segments, validate_pcs_fri_opening_folds_from_units,
    validate_pcs_fri_opening_segments, verify_fri_fold, verify_fri_last_level_root,
    verify_fri_opening_folds, verify_fri_query_path, LoadPcsFriOpeningSegmentError,
    LoadPcsFriOpeningUnitError, PcsFriFoldError, PcsFriMerkleError, PcsFriOpeningBuildError,
    PcsFriOpeningBuildRequest, PcsFriOpeningBuildTiming, PcsFriOpeningFoldError,
    PcsFriOpeningFoldRequest, PcsFriTranscriptCommitmentRequest,
    ValidateOptionalPcsFriOpeningProofSegmentsError,
    ValidateOptionalPcsFriOpeningProofSegmentsRequest, ValidatePcsFriOpeningFoldUnitsError,
    ValidatePcsFriOpeningSegmentsError,
};
use lzvm_prover::pcs_query_plan::load_pcs_query_plan_from_segments;
use lzvm_prover::pcs_transcript::{derive_pcs_transcript_challenges, PcsTranscriptInputs};
use lzvm_prover::pcs_transcript_segments::{
    PcsTranscriptProofSegmentsError, PcsTranscriptUnitChallenges,
};
use lzvm_prover::{
    build_pcs_fri_opening_segment, build_pcs_fri_opening_segment_from_transcript_values,
    ProvePcsFriOpeningValues, ProvePcsFriTranscriptValues, ProveSchedule, ProveUnitSchedule,
};

#[test]
fn verifies_binary_fri_fold_values() {
    let constant = Ext3::from_u64s([1, 2, 3]);
    let slope = Ext3::from_u64s([4, 5, 6]);
    let values = vec![constant + slope, constant - slope];
    let challenge = Ext3::from_u64s([7, 8, 9]);
    let point_inverse = SHIFT.inverse().expect("shift is nonzero");
    let expected = constant + slope * scale(challenge, point_inverse);

    let folded = verify_fri_fold(2, 1, 2, challenge, 0, &values)
        .expect("fold should verify over a binary group");

    assert_eq!(folded, expected);
}

#[test]
fn rejects_fri_fold_values_with_wrong_group_size() {
    let result = verify_fri_fold(2, 1, 2, Ext3::ONE, 0, &[Ext3::ONE]);

    assert!(matches!(
        result,
        Err(PcsFriFoldError::ValueLengthMismatch {
            expected: 2,
            found: 1
        })
    ));
}

#[test]
fn opening_fold_preserves_fold_length_overflow_error_shape() {
    let mut schedule = sample_validation_unit();
    schedule.query_count = 1;
    schedule.extended_domain_bits = usize::BITS;
    schedule.fri_layers = vec![PcsFriLayer {
        input_bits: usize::BITS,
        output_bits: 0,
        folding_factor: 1,
    }];
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        layers: vec![PcsFriOpeningLayerSegment {
            layer_index: 0,
            root: [1, 2, 3, 4],
            last_level: Vec::new(),
            queries: vec![PcsFriOpeningQuerySegment {
                row_index: 0,
                values: Vec::new(),
                siblings: Vec::new(),
            }],
        }],
        final_polynomial: vec![[1, 2, 3]],
    };

    let error = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &[0],
            challenges: &[Ext3::ZERO, Ext3::ONE],
            fri: &fri,
        },
    )
    .expect_err("opening fold should reject unsupported fold width");

    assert_eq!(
        error,
        PcsFriOpeningFoldError::Fold(PcsFriFoldError::LengthOverflow)
    );
}

#[test]
fn opening_fold_binary_path_preserves_invalid_extension_bits_error_shape() {
    let mut schedule = sample_validation_unit();
    schedule.query_count = 1;
    schedule.extended_domain_bits = 1;
    schedule.fri_layers = vec![PcsFriLayer {
        input_bits: 2,
        output_bits: 1,
        folding_factor: 2,
    }];
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        layers: vec![PcsFriOpeningLayerSegment {
            layer_index: 0,
            root: [1, 2, 3, 4],
            last_level: Vec::new(),
            queries: vec![PcsFriOpeningQuerySegment {
                row_index: 0,
                values: vec![[1, 2, 3], [4, 5, 6]],
                siblings: Vec::new(),
            }],
        }],
        final_polynomial: vec![[1, 2, 3], [4, 5, 6]],
    };

    let error = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &[0],
            challenges: &[Ext3::ZERO, Ext3::ONE],
            fri: &fri,
        },
    )
    .expect_err("binary opening fold should reject invalid extension bits");

    assert_eq!(
        error,
        PcsFriOpeningFoldError::Fold(PcsFriFoldError::InvalidExtensionBits {
            n_bits_ext: 1,
            prev_bits: 2,
        })
    );
}

#[test]
fn opening_build_preserves_fold_invalid_extension_bits_error_shape() {
    let mut schedule = sample_validation_unit();
    schedule.query_count = 1;
    schedule.extended_domain_bits = 1;
    schedule.fri_layers = vec![PcsFriLayer {
        input_bits: 2,
        output_bits: 1,
        folding_factor: 2,
    }];
    schedule.final_layer_bits = 1;
    let polynomial = (0_u64..4)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();

    let error = build_pcs_fri_opening_unit(
        &schedule,
        PcsFriOpeningBuildRequest {
            unit_index: 0,
            trace_instance_index: 0,
            query_rows: &[0],
            challenges: &[Ext3::ZERO, Ext3::ONE, Ext3::ZERO],
            polynomial: &polynomial,
        },
    )
    .expect_err("FRI opening build should reject invalid fold extension bits");

    assert_eq!(
        error,
        PcsFriOpeningBuildError::Fold(PcsFriFoldError::InvalidExtensionBits {
            n_bits_ext: 1,
            prev_bits: 2,
        })
    );
}

#[test]
fn verifies_fri_query_path_against_root() {
    let values = [
        Ext3::from_u64s([1, 2, 3]),
        Ext3::from_u64s([4, 5, 6]),
        Ext3::from_u64s([7, 8, 9]),
        Ext3::from_u64s([10, 11, 12]),
    ];
    let leaves = values.map(extension_leaf);
    let last_level = [
        parent_arity2(leaves[0], leaves[1]),
        parent_arity2(leaves[2], leaves[3]),
    ];
    let root = parent_arity2(last_level[0], last_level[1]);
    let siblings = vec![vec![leaves[0]], vec![last_level[1]]];

    assert!(
        verify_fri_query_path(root, &[], 2, 1, &[values[1]], &siblings)
            .expect("path should verify against root")
    );
    assert!(
        !verify_fri_query_path(root, &[], 2, 1, &[values[2]], &siblings)
            .expect("path should check against root")
    );
}

#[test]
fn verifies_fri_query_path_against_last_level() {
    let values = [
        Ext3::from_u64s([13, 14, 15]),
        Ext3::from_u64s([16, 17, 18]),
        Ext3::from_u64s([19, 20, 21]),
        Ext3::from_u64s([22, 23, 24]),
    ];
    let leaves = values.map(extension_leaf);
    let last_level = [
        parent_arity2(leaves[0], leaves[1]),
        parent_arity2(leaves[2], leaves[3]),
    ];
    let root = parent_arity2(last_level[0], last_level[1]);
    let siblings = vec![vec![leaves[0]]];

    assert!(
        verify_fri_query_path(root, &last_level, 2, 1, &[values[1]], &siblings)
            .expect("path should verify against last level")
    );
    assert!(verify_fri_last_level_root(root, 2, &last_level)
        .expect("last level should verify against root"));
}

#[test]
fn rejects_fri_query_paths_with_wrong_sibling_count() {
    let value = Ext3::from_u64s([25, 26, 27]);
    let result = verify_fri_query_path([Felt::ZERO; 4], &[], 2, 0, &[value], &[Vec::new()]);

    assert!(matches!(
        result,
        Err(PcsFriMerkleError::InvalidSiblingCount {
            expected: 1,
            found: 0
        })
    ));
}

#[test]
fn verifies_fri_opening_fold_chain_to_final_polynomial() {
    let query_row = 3_u64;
    let schedule = ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: None,
        unit_id: None,
        group_name: None,
        unit_name: None,
        base_domain_bits: 1,
        extended_domain_bits: 2,
        base_domain_size: 2,
        extended_domain_size: 4,
        blowup_factor: 2,
        query_count: 1,
        proof_of_work_bits: 0,
        merkle_tree_arity: 2,
        last_level_verification: 0,
        transcript_arity: Some(2),
        hash_commits: false,
        transcript_root_challenge_draws: vec![2, 1],
        challenge_count: 6,
        evaluation_value_count: 0,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 2,
        constant_width: 1,
        stage_commit_widths: vec![1],
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![
            PcsFriLayer {
                input_bits: 2,
                output_bits: 1,
                folding_factor: 2,
            },
            PcsFriLayer {
                input_bits: 1,
                output_bits: 0,
                folding_factor: 2,
            },
        ],
        final_layer_bits: 0,
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
    };
    let first_challenge = Ext3::from_u64s([7, 8, 9]);
    let second_challenge = Ext3::from_u64s([11, 12, 13]);
    let mut challenges = vec![Ext3::ZERO; 10];
    challenges[7] = first_challenge;
    challenges[8] = second_challenge;

    let layer0_values = vec![Ext3::from_u64s([1, 2, 3]), Ext3::from_u64s([4, 5, 6])];
    let layer0_fold = verify_fri_fold(
        schedule.extended_domain_bits,
        schedule.fri_layers[0].output_bits,
        schedule.fri_layers[0].input_bits,
        first_challenge,
        query_row % 2,
        &layer0_values,
    )
    .expect("first fold should evaluate");
    let layer1_values = vec![layer0_fold + Ext3::ONE, layer0_fold];
    let final_value = verify_fri_fold(
        schedule.extended_domain_bits,
        schedule.fri_layers[1].output_bits,
        schedule.fri_layers[1].input_bits,
        second_challenge,
        0,
        &layer1_values,
    )
    .expect("second fold should evaluate");
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 3,
        trace_instance_index: 0,
        layers: vec![
            PcsFriOpeningLayerSegment {
                layer_index: 0,
                root: [0; 4],
                last_level: Vec::new(),
                queries: vec![PcsFriOpeningQuerySegment {
                    row_index: 1,
                    values: layer0_values.iter().map(|value| value.to_u64s()).collect(),
                    siblings: Vec::new(),
                }],
            },
            PcsFriOpeningLayerSegment {
                layer_index: 1,
                root: [0; 4],
                last_level: Vec::new(),
                queries: vec![PcsFriOpeningQuerySegment {
                    row_index: 0,
                    values: layer1_values.iter().map(|value| value.to_u64s()).collect(),
                    siblings: Vec::new(),
                }],
            },
        ],
        final_polynomial: vec![final_value.to_u64s()],
    };

    let valid = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 3,
            query_rows: &[query_row],
            challenges: &challenges,
            fri: &fri,
        },
    )
    .expect("fold chain should evaluate");

    assert!(valid);

    let mut tampered = fri.clone();
    tampered.final_polynomial[0] = Ext3::ONE.to_u64s();
    let invalid = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 3,
            query_rows: &[query_row],
            challenges: &challenges,
            fri: &tampered,
        },
    )
    .expect("fold chain should evaluate mismatches");

    assert!(!invalid);
}

#[test]
fn rejects_noncanonical_in_memory_fri_opening_fold_values() {
    let schedule = sample_validation_unit();
    let query_rows = [1_u64, 6_u64];
    let polynomial = (0_u64..8)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();
    let challenges = sample_fold_challenges().challenges;
    let fri = build_pcs_fri_opening_unit(
        &schedule,
        PcsFriOpeningBuildRequest {
            unit_index: 0,
            trace_instance_index: 0,
            query_rows: &query_rows,
            challenges: &challenges,
            polynomial: &polynomial,
        },
    )
    .expect("FRI opening should build");

    let mut query_value = fri.clone();
    query_value.layers[0].queries[0].values[0][0] = MODULUS;
    let error = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &query_rows,
            challenges: &challenges,
            fri: &query_value,
        },
    )
    .expect_err("direct FRI fold verification should reject noncanonical query values");
    assert_eq!(
        error,
        PcsFriOpeningFoldError::NonCanonicalField { value: MODULUS }
    );

    let error = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &query_rows,
            challenges: &[],
            fri: &query_value,
        },
    )
    .expect_err("direct FRI fold verification should reject noncanonical query values first");
    assert_eq!(
        error,
        PcsFriOpeningFoldError::NonCanonicalField { value: MODULUS }
    );

    let mut final_value = fri;
    final_value.final_polynomial[0][0] = MODULUS;
    let error = verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &query_rows,
            challenges: &challenges,
            fri: &final_value,
        },
    )
    .expect_err("direct FRI fold verification should reject noncanonical final values");
    assert_eq!(
        error,
        PcsFriOpeningFoldError::NonCanonicalField { value: MODULUS }
    );
}

#[test]
fn builds_fri_opening_unit_from_polynomial_values() {
    let query_rows = [1_u64, 6_u64];
    let schedule = ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: None,
        unit_id: None,
        group_name: None,
        unit_name: None,
        base_domain_bits: 2,
        extended_domain_bits: 3,
        base_domain_size: 4,
        extended_domain_size: 8,
        blowup_factor: 2,
        query_count: 2,
        proof_of_work_bits: 0,
        merkle_tree_arity: 2,
        last_level_verification: 1,
        transcript_arity: Some(2),
        hash_commits: false,
        transcript_root_challenge_draws: vec![2, 1],
        challenge_count: 6,
        evaluation_value_count: 0,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 2,
        constant_width: 1,
        stage_commit_widths: vec![1],
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![
            PcsFriLayer {
                input_bits: 3,
                output_bits: 2,
                folding_factor: 2,
            },
            PcsFriLayer {
                input_bits: 2,
                output_bits: 1,
                folding_factor: 2,
            },
        ],
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
    };
    let polynomial = (0_u64..8)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();
    let mut challenges = vec![Ext3::ZERO; 10];
    challenges[7] = Ext3::from_u64s([31, 32, 33]);
    challenges[8] = Ext3::from_u64s([41, 42, 43]);

    let fri = build_pcs_fri_opening_unit(
        &schedule,
        PcsFriOpeningBuildRequest {
            unit_index: 5,
            trace_instance_index: 0,
            query_rows: &query_rows,
            challenges: &challenges,
            polynomial: &polynomial,
        },
    )
    .expect("FRI opening should build");

    assert_eq!(fri.unit_index, 5);
    assert_eq!(fri.layers.len(), 2);
    assert_eq!(fri.final_polynomial.len(), 2);
    assert!(verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 5,
            query_rows: &query_rows,
            challenges: &challenges,
            fri: &fri,
        },
    )
    .expect("fold chain should verify"));

    let first_fold = fold_full_layer(
        &schedule,
        &schedule.fri_layers[0],
        challenges[7],
        &polynomial,
    );
    let final_fold = fold_full_layer(
        &schedule,
        &schedule.fri_layers[1],
        challenges[8],
        &first_fold,
    );
    assert_eq!(
        fri.final_polynomial,
        final_fold
            .iter()
            .map(|value| value.to_u64s())
            .collect::<Vec<_>>()
    );

    for layer in &fri.layers {
        let root = digest_from_u64s(layer.root);
        let last_level = layer
            .last_level
            .iter()
            .copied()
            .map(digest_from_u64s)
            .collect::<Vec<_>>();
        assert!(verify_fri_last_level_root(root, 2, &last_level).expect("last level should verify"));
        for query in &layer.queries {
            let values = query
                .values
                .iter()
                .map(|value| Ext3::from_u64s(*value))
                .collect::<Vec<_>>();
            let siblings = query
                .siblings
                .iter()
                .map(|level| {
                    level
                        .siblings
                        .iter()
                        .copied()
                        .map(digest_from_u64s)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            assert!(verify_fri_query_path(
                root,
                &last_level,
                2,
                query.row_index,
                &values,
                &siblings
            )
            .expect("query path should verify"));
        }
    }
}

#[test]
fn builds_wide_fri_opening_unit_from_polynomial_values() {
    let query_rows = [1_u64, 6_u64];
    let mut schedule = sample_validation_unit();
    schedule.fri_layers = vec![PcsFriLayer {
        input_bits: 3,
        output_bits: 1,
        folding_factor: 4,
    }];
    schedule.final_layer_bits = 1;
    let polynomial = (0_u64..8)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();
    let mut challenges = vec![Ext3::ZERO; 9];
    challenges[7] = Ext3::from_u64s([31, 32, 33]);

    let fri = build_pcs_fri_opening_unit(
        &schedule,
        PcsFriOpeningBuildRequest {
            unit_index: 5,
            trace_instance_index: 0,
            query_rows: &query_rows,
            challenges: &challenges,
            polynomial: &polynomial,
        },
    )
    .expect("wide FRI opening should build");

    assert_eq!(fri.layers.len(), 1);
    assert_eq!(fri.layers[0].queries.len(), query_rows.len());
    assert_eq!(fri.final_polynomial.len(), 2);
    assert!(verify_fri_opening_folds(
        &schedule,
        PcsFriOpeningFoldRequest {
            unit_index: 5,
            query_rows: &query_rows,
            challenges: &challenges,
            fri: &fri,
        },
    )
    .expect("wide fold chain should verify"));

    let expected = fold_full_layer(
        &schedule,
        &schedule.fri_layers[0],
        challenges[7],
        &polynomial,
    );
    assert_eq!(
        fri.final_polynomial,
        expected
            .iter()
            .map(|value| value.to_u64s())
            .collect::<Vec<_>>()
    );
}

#[test]
fn records_fri_opening_unit_build_timing_shape() {
    let schedule = sample_validation_unit();
    let query_rows = [1_u64, 6_u64];
    let polynomial = (0_u64..8)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();
    let mut challenges = vec![Ext3::ZERO; 9];
    challenges[7] = Ext3::from_u64s([31, 32, 33]);
    challenges[8] = Ext3::from_u64s([41, 42, 43]);

    let expected = build_pcs_fri_opening_unit(
        &schedule,
        PcsFriOpeningBuildRequest {
            unit_index: 5,
            trace_instance_index: 0,
            query_rows: &query_rows,
            challenges: &challenges,
            polynomial: &polynomial,
        },
    )
    .expect("FRI opening should build without timing");
    let mut timing = PcsFriOpeningBuildTiming::default();
    let actual = build_pcs_fri_opening_unit_with_timing(
        &schedule,
        PcsFriOpeningBuildRequest {
            unit_index: 5,
            trace_instance_index: 0,
            query_rows: &query_rows,
            challenges: &challenges,
            polynomial: &polynomial,
        },
        Some(&mut timing),
    )
    .expect("FRI opening should build with timing");

    assert_eq!(actual, expected);
    assert_eq!(timing.unit_count, 1);
    assert_eq!(timing.layer_count, schedule.fri_layers.len());
    assert_eq!(
        timing.query_count,
        query_rows.len() * schedule.fri_layers.len()
    );
}

#[test]
fn builds_duplicate_fri_queries_from_cached_rows_without_changing_shape() {
    let unit = sample_validation_unit();
    let schedule = sample_prove_schedule(unit.clone());
    let query_rows = [1_u64, 5_u64];
    let query_segment = ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
            units: vec![PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 0,
                queries: query_rows.to_vec(),
            }],
        })
        .expect("query plan should encode"),
    };
    let polynomial = (0_u64..8)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();
    let constant_root = root(90);
    let public_values = values(&[3, 4]);
    let witness_roots = vec![root(10), root(20)];
    let evaluations = vec![Ext3::from_u64s([30, 31, 32])];
    let commitments = build_pcs_fri_transcript_commitments(
        &unit,
        PcsFriTranscriptCommitmentRequest {
            arity: 2,
            hash_values: false,
            constant_root,
            public_values: &public_values,
            witness_roots: &witness_roots,
            root_challenge_draws: &unit.transcript_root_challenge_draws,
            unit_value_map: &[],
            unit_values: &[],
            evaluation_values: &evaluations,
            evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
            polynomial: &polynomial,
            binding_segments: &[],
        },
    )
    .expect("FRI transcript commitments should build");
    let fold_challenges = commitments.challenges.clone();
    let direct = build_pcs_fri_opening_segment(
        &schedule,
        &query_segment,
        &[ProvePcsFriOpeningValues {
            unit_index: 0,
            trace_instance_index: 0,
            challenges: commitments.challenges.clone(),
            polynomial: polynomial.clone(),
        }],
    )
    .expect("direct FRI opening segment should build");
    let retained = build_pcs_fri_opening_segment_from_transcript_values(
        &schedule,
        &query_segment,
        &[ProvePcsFriTranscriptValues {
            unit_index: 0,
            trace_instance_index: 0,
            polynomial,
            commitments,
        }],
    )
    .expect("retained FRI opening segment should build");

    assert_eq!(retained, direct);
    let parsed = parse_pcs_fri_opening_segment(&direct.data).expect("opening segment should parse");
    for layer in &parsed.units[0].layers {
        assert_eq!(layer.queries.len(), query_rows.len());
        assert_eq!(layer.queries[0], layer.queries[1]);
    }
    let segments = [query_segment, direct];
    let query_plan = load_pcs_query_plan_from_segments(&segments).expect("query plan should load");
    let opening =
        load_pcs_fri_opening_segment_from_segments(&segments).expect("FRI opening should load");
    validate_pcs_fri_opening_segments(&[unit.clone()], &segments)
        .expect("duplicate-row FRI opening segment should validate");
    validate_pcs_fri_opening_folds_from_units(
        &[unit],
        &query_plan.units,
        &opening.units,
        &[PcsTranscriptUnitChallenges {
            unit_index: 0,
            trace_instance_index: 0,
            challenges: fold_challenges,
        }],
    )
    .expect("duplicate-row FRI opening folds should validate");
}

#[test]
fn loads_pcs_fri_opening_segment_from_segments() {
    let unit = sample_fri_opening_unit(0);
    let segment = pcs_fri_opening_proof_segment(vec![unit.clone()]);

    let loaded =
        load_pcs_fri_opening_segment_from_segments(&[segment]).expect("segment should load");

    assert_eq!(loaded, PcsFriOpeningSegment { units: vec![unit] });
}

#[test]
fn loads_pcs_fri_opening_unit_from_segments() {
    let unit = sample_fri_opening_unit(0);
    let segment = pcs_fri_opening_proof_segment(vec![unit.clone()]);

    let loaded = load_pcs_fri_opening_unit_from_segments(0, &[segment]).expect("unit should load");

    assert_eq!(loaded, unit);
}

#[test]
fn loads_pcs_fri_opening_unit_for_identity_from_segments() {
    let mut unit = sample_fri_opening_unit(0);
    unit.trace_instance_index = 2;
    let segment = pcs_fri_opening_proof_segment(vec![unit.clone()]);

    let loaded = load_pcs_fri_opening_unit_for_identity_from_segments(0, 2, &[segment])
        .expect("unit should load");

    assert_eq!(loaded, unit);
}

#[test]
fn rejects_pcs_fri_opening_unit_trace_identity_mismatch() {
    let mut unit = sample_fri_opening_unit(0);
    unit.trace_instance_index = 2;
    let segment = pcs_fri_opening_proof_segment(vec![unit]);

    let error = load_pcs_fri_opening_unit_for_identity_from_segments(0, 1, &[segment])
        .expect_err("unit should require matching trace identity");

    assert_eq!(
        error,
        LoadPcsFriOpeningUnitError::MissingUnit { unit_index: 0 }
    );
}

#[test]
fn rejects_missing_pcs_fri_opening_segment() {
    let error = load_pcs_fri_opening_segment_from_segments(&[]).expect_err("segment should exist");

    assert_eq!(error, LoadPcsFriOpeningSegmentError::MissingSegment);
}

#[test]
fn rejects_invalid_pcs_fri_opening_segment() {
    let error = load_pcs_fri_opening_segment_from_segments(&[ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: vec![1, 2, 3, 4],
    }])
    .expect_err("segment should parse");

    assert!(matches!(error, LoadPcsFriOpeningSegmentError::Segment(_)));
}

#[test]
fn rejects_noncanonical_pcs_fri_final_polynomial_values_while_parsing() {
    let mut segment = pcs_fri_opening_proof_segment(vec![sample_fri_opening_unit(0)]);
    replace_first_fri_final_polynomial_word(&mut segment.data, MODULUS);

    let error =
        parse_pcs_fri_opening_segment(&segment.data).expect_err("final value should be canonical");

    assert_eq!(
        error,
        PcsFriOpeningSegmentError::FinalPolynomialValueNonCanonical {
            unit_index: 0,
            value_index: 0,
            word_index: 0,
            source: FieldError::NonCanonical { value: MODULUS },
        }
    );
}

#[test]
fn rejects_noncanonical_pcs_fri_query_values_while_parsing() {
    let mut segment = pcs_fri_opening_proof_segment(vec![sample_fri_opening_unit(0)]);
    replace_first_fri_query_value_word(&mut segment.data, MODULUS);

    let error =
        parse_pcs_fri_opening_segment(&segment.data).expect_err("query value should be canonical");

    assert_eq!(
        error,
        PcsFriOpeningSegmentError::QueryValueNonCanonical {
            unit_index: 0,
            layer_index: 0,
            row_index: 0,
            value_index: 0,
            word_index: 0,
            source: FieldError::NonCanonical { value: MODULUS },
        }
    );
}

#[test]
fn rejects_duplicate_pcs_fri_opening_segments() {
    let segment = pcs_fri_opening_proof_segment(vec![sample_fri_opening_unit(0)]);

    let error = load_pcs_fri_opening_segment_from_segments(&[segment.clone(), segment])
        .expect_err("duplicate segment should be rejected");

    assert_eq!(error.to_string(), "duplicate PCS FRI opening segment");
}

#[test]
fn rejects_missing_pcs_fri_opening_unit() {
    let segment = pcs_fri_opening_proof_segment(vec![sample_fri_opening_unit(1)]);

    let error =
        load_pcs_fri_opening_unit_from_segments(0, &[segment]).expect_err("unit should exist");

    assert_eq!(
        error,
        LoadPcsFriOpeningUnitError::MissingUnit { unit_index: 0 }
    );
}

#[test]
fn validates_pcs_fri_opening_segments() {
    let (unit, segments) = valid_pcs_fri_opening_segments();

    validate_pcs_fri_opening_segments(&[unit], &segments).expect("FRI opening should validate");
}

#[test]
fn builds_trace_instance_pcs_fri_opening_queries() {
    let unit = sample_validation_unit();
    let schedule = sample_prove_schedule(unit);
    let query_rows = [1_u64, 6_u64];
    let query_segment = ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
            units: vec![PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 1,
                queries: query_rows.to_vec(),
            }],
        })
        .expect("query plan should encode"),
    };
    let polynomial = (0_u64..8)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();
    let mut challenges = vec![Ext3::ZERO; 9];
    challenges[7] = Ext3::from_u64s([31, 32, 33]);
    challenges[8] = Ext3::from_u64s([41, 42, 43]);

    let segment = build_pcs_fri_opening_segment(
        &schedule,
        &query_segment,
        &[ProvePcsFriOpeningValues {
            unit_index: 0,
            trace_instance_index: 1,
            challenges,
            polynomial,
        }],
    )
    .expect("trace instance FRI opening should build");
    let opening =
        load_pcs_fri_opening_segment_from_segments(&[segment]).expect("FRI opening should load");

    assert_eq!(opening.units[0].trace_instance_index, 1);
}

#[test]
fn validates_trace_instance_pcs_fri_opening_queries() {
    let (unit, mut segments) = valid_pcs_fri_opening_segments();
    replace_query_plan_trace_instance(&mut segments, 1);
    replace_fri_opening_trace_instance(&mut segments, 1);

    validate_pcs_fri_opening_segments(&[unit], &segments)
        .expect("trace instance FRI opening should validate");
}

#[test]
fn rejects_pcs_fri_opening_unit_trace_instance_mismatch() {
    let (unit, mut segments) = valid_pcs_fri_opening_segments();
    replace_fri_opening_trace_instance(&mut segments, 1);

    let error = validate_pcs_fri_opening_segments(&[unit], &segments)
        .expect_err("opening should match the query trace instance");

    assert_eq!(
        error,
        ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn validates_optional_pcs_fri_opening_when_segment_is_absent() {
    let unit = sample_validation_unit();
    let schedule = sample_prove_schedule(unit);

    validate_optional_pcs_fri_opening_proof_segments(
        ValidateOptionalPcsFriOpeningProofSegmentsRequest {
            schedule: &schedule,
            verifier_codes: &[],
            fri_opening_required_units: &[false],
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            segments: &[],
        },
    )
    .expect("missing optional FRI opening should be accepted");
}

#[test]
fn rejects_seeded_required_pcs_fri_opening_without_segment() {
    let (unit, mut segments) = valid_pcs_fri_opening_segments();
    let schedule = sample_prove_schedule(unit);
    segments.retain(|segment| segment.id != PCS_FRI_OPENING_SEGMENT_ID);

    let error = validate_optional_pcs_fri_opening_proof_segments(
        ValidateOptionalPcsFriOpeningProofSegmentsRequest {
            schedule: &schedule,
            verifier_codes: &[],
            fri_opening_required_units: &[true],
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            segments: &segments,
        },
    )
    .expect_err("required seeded FRI opening should reject");

    assert_eq!(
        error,
        ValidateOptionalPcsFriOpeningProofSegmentsError::OpeningSegment(
            LoadPcsFriOpeningSegmentError::MissingSegment
        )
    );
}

#[test]
fn rejects_optional_pcs_fri_opening_transcript_inputs_without_opening() {
    let (unit, mut segments) = valid_pcs_fri_opening_segments();
    let schedule = sample_prove_schedule(unit);
    segments.retain(|segment| segment.id != PCS_FRI_OPENING_SEGMENT_ID);
    segments.push(ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: Vec::new(),
    });

    let error = validate_optional_pcs_fri_opening_proof_segments(
        ValidateOptionalPcsFriOpeningProofSegmentsRequest {
            schedule: &schedule,
            verifier_codes: &[],
            fri_opening_required_units: &[false],
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            segments: &segments,
        },
    )
    .expect_err("transcript inputs should require FRI opening");

    assert_eq!(
        error,
        ValidateOptionalPcsFriOpeningProofSegmentsError::OpeningSegment(
            LoadPcsFriOpeningSegmentError::MissingSegment
        )
    );
}

#[test]
fn rejects_optional_pcs_fri_opening_from_seeded_inputs() {
    let (unit, segments) = valid_pcs_fri_opening_segments();
    let schedule = sample_prove_schedule(unit);

    let error = validate_optional_pcs_fri_opening_proof_segments(
        ValidateOptionalPcsFriOpeningProofSegmentsRequest {
            schedule: &schedule,
            verifier_codes: &[],
            fri_opening_required_units: &[false],
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            segments: &segments,
        },
    )
    .expect_err("seeded FRI opening should reject");

    assert_eq!(
        error,
        ValidateOptionalPcsFriOpeningProofSegmentsError::UnboundOpeningSegment
    );
}

#[test]
fn rejects_optional_pcs_fri_opening_transcript_inputs_without_material() {
    let (unit, mut segments) = valid_pcs_fri_opening_segments();
    let schedule = sample_prove_schedule(unit);
    segments.push(ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: Vec::new(),
    });

    let error = validate_optional_pcs_fri_opening_proof_segments(
        ValidateOptionalPcsFriOpeningProofSegmentsRequest {
            schedule: &schedule,
            verifier_codes: &[],
            fri_opening_required_units: &[false],
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            segments: &segments,
        },
    )
    .expect_err("transcript inputs should require transcript material");

    assert_eq!(
        error,
        ValidateOptionalPcsFriOpeningProofSegmentsError::Transcript(
            PcsTranscriptProofSegmentsError::MissingMaterialSegment
        )
    );
}

#[test]
fn validates_pcs_fri_opening_folds_from_units() {
    let (unit, segments) = valid_pcs_fri_opening_segments();
    let query_plan = load_pcs_query_plan_from_segments(&segments).expect("query plan should load");
    let opening =
        load_pcs_fri_opening_segment_from_segments(&segments).expect("FRI opening should load");
    let challenges = vec![sample_fold_challenges()];

    validate_pcs_fri_opening_folds_from_units(
        &[unit],
        &query_plan.units,
        &opening.units,
        &challenges,
    )
    .expect("FRI opening folds should validate");
}

#[test]
fn validates_pcs_fri_opening_folds_by_trace_identity() {
    let (unit, segments) = valid_pcs_fri_opening_segments();
    let base_query_plan =
        load_pcs_query_plan_from_segments(&segments).expect("query plan should load");
    let base_opening =
        load_pcs_fri_opening_segment_from_segments(&segments).expect("FRI opening should load");
    let mut trace_query = base_query_plan.units[0].clone();
    trace_query.trace_instance_index = 1;
    let mut trace_opening = base_opening.units[0].clone();
    trace_opening.trace_instance_index = 1;
    let base_challenges = sample_fold_challenges();
    let mut trace_challenges = sample_fold_challenges();
    trace_challenges.trace_instance_index = 1;

    let query_units = [trace_query, base_query_plan.units[0].clone()];
    let opening_units = [base_opening.units[0].clone(), trace_opening];
    let challenges = [base_challenges, trace_challenges];

    validate_pcs_fri_opening_folds_from_units(&[unit], &query_units, &opening_units, &challenges)
        .expect("FRI opening folds should match by trace identity");
}

#[test]
fn rejects_pcs_fri_opening_folds_with_trace_identity_mismatch() {
    let (unit, segments) = valid_pcs_fri_opening_segments();
    let base_query_plan =
        load_pcs_query_plan_from_segments(&segments).expect("query plan should load");
    let base_opening =
        load_pcs_fri_opening_segment_from_segments(&segments).expect("FRI opening should load");
    let mut trace_query = base_query_plan.units[0].clone();
    trace_query.trace_instance_index = 1;
    let mut trace_opening = base_opening.units[0].clone();
    trace_opening.trace_instance_index = 1;
    let base_challenges = sample_fold_challenges();
    let mut trace_challenges = sample_fold_challenges();
    trace_challenges.trace_instance_index = 1;

    let opening_error = validate_pcs_fri_opening_folds_from_units(
        std::slice::from_ref(&unit),
        std::slice::from_ref(&trace_query),
        std::slice::from_ref(&base_opening.units[0]),
        std::slice::from_ref(&trace_challenges),
    )
    .expect_err("FRI opening folds should reject opening identity mismatch");

    assert_eq!(
        opening_error,
        ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index: 0 }
    );

    let challenge_error = validate_pcs_fri_opening_folds_from_units(
        std::slice::from_ref(&unit),
        std::slice::from_ref(&trace_query),
        std::slice::from_ref(&trace_opening),
        std::slice::from_ref(&base_challenges),
    )
    .expect_err("FRI opening folds should reject challenge identity mismatch");

    assert_eq!(
        challenge_error,
        ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn rejects_pcs_fri_opening_folds_with_unqueried_trace_identity() {
    let (unit, segments) = valid_pcs_fri_opening_segments();
    let query_plan = load_pcs_query_plan_from_segments(&segments).expect("query plan should load");
    let opening =
        load_pcs_fri_opening_segment_from_segments(&segments).expect("FRI opening should load");
    let base_challenges = sample_fold_challenges();
    let mut extra_opening = opening.units[0].clone();
    extra_opening.trace_instance_index = 1;
    let mut extra_challenges = base_challenges.clone();
    extra_challenges.trace_instance_index = 1;

    let opening_units = [opening.units[0].clone(), extra_opening];
    let opening_error = validate_pcs_fri_opening_folds_from_units(
        std::slice::from_ref(&unit),
        &query_plan.units,
        &opening_units,
        std::slice::from_ref(&base_challenges),
    )
    .expect_err("FRI opening folds should reject unqueried opening identity");

    assert_eq!(
        opening_error,
        ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index: 0 }
    );

    let challenges = [base_challenges, extra_challenges];
    let challenge_error = validate_pcs_fri_opening_folds_from_units(
        std::slice::from_ref(&unit),
        &query_plan.units,
        std::slice::from_ref(&opening.units[0]),
        &challenges,
    )
    .expect_err("FRI opening folds should reject unqueried challenge identity");

    assert_eq!(
        challenge_error,
        ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn rejects_pcs_fri_opening_fold_mismatches_from_units() {
    let (unit, segments) = valid_pcs_fri_opening_segments();
    let query_plan = load_pcs_query_plan_from_segments(&segments).expect("query plan should load");
    let opening =
        load_pcs_fri_opening_segment_from_segments(&segments).expect("FRI opening should load");
    let mut challenges = sample_fold_challenges();
    challenges.challenges[7] = Ext3::from_u64s([99, 100, 101]);

    let error = validate_pcs_fri_opening_folds_from_units(
        &[unit],
        &query_plan.units,
        &opening.units,
        &[challenges],
    )
    .expect_err("FRI fold mismatch should be rejected");

    assert_eq!(
        error,
        ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn rejects_pcs_fri_opening_value_mismatches() {
    let (unit, mut segments) = valid_pcs_fri_opening_segments();
    let fri_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .expect("FRI opening segment should exist");
    let opening_segment = fri_segment.clone();
    let mut opening = load_pcs_fri_opening_segment_from_segments(&[opening_segment])
        .expect("FRI opening should parse");
    opening.units[0].layers[0].queries[0].values[0][0] ^= 1;
    fri_segment.data = encode_pcs_fri_opening_segment(&opening).expect("FRI opening should encode");

    let error = validate_pcs_fri_opening_segments(&[unit], &segments)
        .expect_err("value mismatch should be rejected");

    assert_eq!(
        error,
        ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn derives_fri_transcript_commitments_from_polynomial_values() {
    let schedule = ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: None,
        unit_id: None,
        group_name: None,
        unit_name: None,
        base_domain_bits: 2,
        extended_domain_bits: 3,
        base_domain_size: 4,
        extended_domain_size: 8,
        blowup_factor: 2,
        query_count: 2,
        proof_of_work_bits: 0,
        merkle_tree_arity: 2,
        last_level_verification: 1,
        transcript_arity: Some(2),
        hash_commits: false,
        transcript_root_challenge_draws: vec![1, 2],
        challenge_count: 5,
        evaluation_value_count: 1,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 2,
        constant_width: 1,
        stage_commit_widths: vec![1, 1],
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![
            PcsFriLayer {
                input_bits: 3,
                output_bits: 2,
                folding_factor: 2,
            },
            PcsFriLayer {
                input_bits: 2,
                output_bits: 1,
                folding_factor: 2,
            },
        ],
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
    };
    let constant_root = root(90);
    let public_values = values(&[3, 4]);
    let witness_roots = vec![root(10), root(20)];
    let evaluations = vec![Ext3::from_u64s([30, 31, 32])];
    let polynomial = (0_u64..8)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();

    let commitments = build_pcs_fri_transcript_commitments(
        &schedule,
        PcsFriTranscriptCommitmentRequest {
            arity: 2,
            hash_values: false,
            constant_root,
            public_values: &public_values,
            witness_roots: &witness_roots,
            root_challenge_draws: &[1, 2],
            unit_value_map: &[],
            unit_values: &[],
            evaluation_values: &evaluations,
            evaluation_challenge_draws: 2,
            polynomial: &polynomial,
            binding_segments: &[],
        },
    )
    .expect("FRI transcript commitments should build");

    let expected_challenges = derive_pcs_transcript_challenges(PcsTranscriptInputs {
        arity: 2,
        hash_values: false,
        constant_root,
        public_values: &public_values,
        witness_roots: &witness_roots,
        root_challenge_draws: &[1, 2],
        unit_value_map: &[],
        unit_values: &[],
        evaluation_values: &evaluations,
        evaluation_challenge_draws: 2,
        fri_roots: &commitments.layer_roots,
        final_polynomial: &commitments.final_polynomial,
        binding_segments: &[],
    })
    .expect("transcript challenges should derive");
    let opening = build_pcs_fri_opening_unit(
        &schedule,
        PcsFriOpeningBuildRequest {
            unit_index: 0,
            trace_instance_index: 0,
            query_rows: &[1, 6],
            challenges: &commitments.challenges,
            polynomial: &polynomial,
        },
    )
    .expect("FRI opening should build from derived challenges");
    let opening_roots = opening
        .layers
        .iter()
        .map(|layer| digest_from_u64s(layer.root))
        .collect::<Vec<_>>();

    assert_eq!(commitments.challenges, expected_challenges);
    assert_eq!(
        commitments.final_query_challenge,
        *expected_challenges
            .last()
            .expect("final challenge should exist")
    );
    assert_eq!(commitments.layer_roots, opening_roots);
    assert_eq!(
        commitments
            .final_polynomial
            .iter()
            .map(|value| value.to_u64s())
            .collect::<Vec<_>>(),
        opening.final_polynomial
    );
}

fn scale(value: Ext3, scalar: Felt) -> Ext3 {
    Ext3::new(value.c0 * scalar, value.c1 * scalar, value.c2 * scalar)
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

fn fold_full_layer(
    schedule: &ProveUnitSchedule,
    layer: &PcsFriLayer,
    challenge: Ext3,
    polynomial: &[Ext3],
) -> Vec<Ext3> {
    let output_size = 1_usize << layer.output_bits;
    (0..output_size)
        .map(|row| {
            let values = (0..layer.folding_factor as usize)
                .map(|slot| polynomial[slot * output_size + row])
                .collect::<Vec<_>>();
            verify_fri_fold(
                schedule.extended_domain_bits,
                layer.output_bits,
                layer.input_bits,
                challenge,
                row as u64,
                &values,
            )
            .expect("fold should evaluate")
        })
        .collect()
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
            queries: vec![1, 6],
        }],
    })
    .expect("query plan should encode");
}

fn replace_fri_opening_trace_instance(segments: &mut [ProofSegment], trace_instance_index: u32) {
    let fri_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .expect("FRI opening segment should exist");
    let mut fri = lzvm_artifacts::pcs_fri_segment::parse_pcs_fri_opening_segment(&fri_segment.data)
        .expect("FRI opening should load");
    fri.units[0].trace_instance_index = trace_instance_index;
    fri_segment.data = encode_pcs_fri_opening_segment(&fri).expect("FRI opening should encode");
}

fn digest_from_u64s(values: [u64; 4]) -> [Felt; 4] {
    values.map(Felt::from_u64)
}

fn pcs_fri_opening_proof_segment(units: Vec<PcsFriOpeningUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: encode_pcs_fri_opening_segment(&PcsFriOpeningSegment { units })
            .expect("segment should encode"),
    }
}

fn replace_first_fri_final_polynomial_word(data: &mut [u8], replacement: u64) {
    let mut cursor = PcsFriSegmentCursor::new(data);
    cursor.expect_magic();
    let version = cursor.read_u32("version");
    assert!(
        matches!(version, 1 | 2),
        "test segment version should be supported"
    );
    let unit_count = cursor.read_u32("unit count");
    assert_eq!(unit_count, 1, "test segment should contain one unit");
    cursor.read_u32("unit index");
    if version == 2 {
        cursor.read_u32("trace instance index");
    }
    cursor.read_u32("layer count");
    let final_count = cursor.read_u32("final polynomial count");
    assert_ne!(final_count, 0, "test segment should contain final values");
    cursor.replace_u64(replacement, "first final polynomial word");
}

fn replace_first_fri_query_value_word(data: &mut [u8], replacement: u64) {
    let mut cursor = PcsFriSegmentCursor::new(data);
    cursor.expect_magic();
    let version = cursor.read_u32("version");
    assert!(
        matches!(version, 1 | 2),
        "test segment version should be supported"
    );
    let unit_count = cursor.read_u32("unit count");
    assert_eq!(unit_count, 1, "test segment should contain one unit");
    cursor.read_u32("unit index");
    if version == 2 {
        cursor.read_u32("trace instance index");
    }
    let layer_count = cursor.read_u32("layer count");
    let final_count = cursor.read_u32("final polynomial count");
    assert_ne!(layer_count, 0, "test segment should contain a layer");
    cursor.skip_extension_values(final_count, "final polynomial");
    cursor.read_u32("layer index");
    cursor.skip_digest("layer root");
    let last_level_count = cursor.read_u32("last level count");
    let query_count = cursor.read_u32("query count");
    cursor.skip_digests(last_level_count, "last level");
    assert_ne!(query_count, 0, "test segment should contain a query");
    cursor.read_u64("query row");
    let value_count = cursor.read_u32("query value count");
    cursor.read_u32("query sibling level count");
    assert_ne!(value_count, 0, "test query should contain values");
    cursor.replace_u64(replacement, "first query value word");
}

struct PcsFriSegmentCursor<'a> {
    data: &'a mut [u8],
    offset: usize,
}

impl<'a> PcsFriSegmentCursor<'a> {
    fn new(data: &'a mut [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn expect_magic(&mut self) {
        let magic = self.read_array::<4>("magic");
        assert_eq!(&magic, b"fos0", "FRI opening segment magic mismatch");
    }

    fn read_u32(&mut self, label: &str) -> u32 {
        u32::from_le_bytes(self.read_array::<4>(label))
    }

    fn read_u64(&mut self, label: &str) -> u64 {
        u64::from_le_bytes(self.read_array::<8>(label))
    }

    fn replace_u64(&mut self, replacement: u64, label: &str) {
        let end = self.checked_end(8, label);
        self.data[self.offset..end].copy_from_slice(&replacement.to_le_bytes());
        self.offset = end;
    }

    fn skip_digest(&mut self, label: &str) {
        self.skip_words(4, label);
    }

    fn skip_digests(&mut self, count: u32, label: &str) {
        self.skip_words(count as usize * 4, label);
    }

    fn skip_extension_values(&mut self, count: u32, label: &str) {
        self.skip_words(count as usize * 3, label);
    }

    fn skip_words(&mut self, count: usize, label: &str) {
        let byte_count = count
            .checked_mul(8)
            .unwrap_or_else(|| panic!("{label} byte count overflow"));
        let end = self.checked_end(byte_count, label);
        self.offset = end;
    }

    fn read_array<const N: usize>(&mut self, label: &str) -> [u8; N] {
        let end = self.checked_end(N, label);
        let mut out = [0_u8; N];
        out.copy_from_slice(&self.data[self.offset..end]);
        self.offset = end;
        out
    }

    fn checked_end(&self, byte_count: usize, label: &str) -> usize {
        let end = self
            .offset
            .checked_add(byte_count)
            .unwrap_or_else(|| panic!("{label} offset overflow"));
        assert!(
            end <= self.data.len(),
            "{label} exceeds encoded segment length"
        );
        end
    }
}

fn valid_pcs_fri_opening_segments() -> (ProveUnitSchedule, Vec<ProofSegment>) {
    let unit = sample_validation_unit();
    let query_rows = [1_u64, 6_u64];
    let polynomial = (0_u64..8)
        .map(|index| Ext3::from_u64s([index + 1, index + 11, index + 21]))
        .collect::<Vec<_>>();
    let mut challenges = vec![Ext3::ZERO; 9];
    challenges[7] = Ext3::from_u64s([31, 32, 33]);
    challenges[8] = Ext3::from_u64s([41, 42, 43]);
    let fri = build_pcs_fri_opening_unit(
        &unit,
        PcsFriOpeningBuildRequest {
            unit_index: 0,
            trace_instance_index: 0,
            query_rows: &query_rows,
            challenges: &challenges,
            polynomial: &polynomial,
        },
    )
    .expect("FRI opening should build");
    let query_segment = ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&PcsQueryPlanSegment {
            units: vec![PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 0,
                queries: query_rows.to_vec(),
            }],
        })
        .expect("query plan should encode"),
    };
    let fri_segment = pcs_fri_opening_proof_segment(vec![fri]);

    (unit, vec![query_segment, fri_segment])
}

fn sample_fold_challenges() -> PcsTranscriptUnitChallenges {
    let mut challenges = vec![Ext3::ZERO; 9];
    challenges[7] = Ext3::from_u64s([31, 32, 33]);
    challenges[8] = Ext3::from_u64s([41, 42, 43]);
    PcsTranscriptUnitChallenges {
        unit_index: 0,
        trace_instance_index: 0,
        challenges,
    }
}

fn sample_prove_schedule(unit: ProveUnitSchedule) -> ProveSchedule {
    ProveSchedule {
        setup_hash: [0; 32],
        unit_count: 1,
        total_fixed_bytes: unit.fixed_bytes,
        total_pcs_material_bytes: unit.pcs_material_bytes.unwrap_or(0),
        pcs_material_unit_count: usize::from(unit.pcs_material_bytes.is_some()),
        total_query_count: u64::from(unit.query_count),
        max_extended_domain_bits: unit.extended_domain_bits,
        units: vec![unit],
    }
}

fn global_info_without_proof_values() -> GlobalInfo {
    GlobalInfo {
        name: "global".to_owned(),
        air_groups: vec!["group".to_owned()],
        airs: Vec::new(),
        curve: CurveKind::None,
        lattice_size: None,
        aggregation_types: Vec::new(),
        n_publics: 0,
        num_challenges: Vec::new(),
        num_proof_values: Vec::new(),
        proof_values_map: Vec::new(),
        publics_map: Vec::new(),
        transcript_arity: 2,
    }
}

fn sample_fri_opening_unit(unit_index: u32) -> PcsFriOpeningUnitSegment {
    PcsFriOpeningUnitSegment {
        unit_index,
        trace_instance_index: 0,
        layers: vec![PcsFriOpeningLayerSegment {
            layer_index: 0,
            root: [10, 11, 12, 13],
            last_level: Vec::new(),
            queries: vec![PcsFriOpeningQuerySegment {
                row_index: 0,
                values: vec![[1, 2, 3]],
                siblings: Vec::new(),
            }],
        }],
        final_polynomial: vec![[4, 5, 6]],
    }
}

fn sample_validation_unit() -> ProveUnitSchedule {
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: None,
        unit_id: None,
        group_name: None,
        unit_name: None,
        base_domain_bits: 2,
        extended_domain_bits: 3,
        base_domain_size: 4,
        extended_domain_size: 8,
        blowup_factor: 2,
        query_count: 2,
        proof_of_work_bits: 0,
        merkle_tree_arity: 2,
        last_level_verification: 1,
        transcript_arity: Some(2),
        hash_commits: false,
        transcript_root_challenge_draws: vec![2, 1],
        challenge_count: 6,
        evaluation_value_count: 0,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 2,
        constant_width: 1,
        stage_commit_widths: vec![1],
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![
            PcsFriLayer {
                input_bits: 3,
                output_bits: 2,
                folding_factor: 2,
            },
            PcsFriLayer {
                input_bits: 2,
                output_bits: 1,
                folding_factor: 2,
            },
        ],
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

fn extension_leaf(value: Ext3) -> [Felt; 4] {
    [value.c0, value.c1, value.c2, Felt::ZERO]
}

fn parent_arity2(left: [Felt; 4], right: [Felt; 4]) -> [Felt; 4] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

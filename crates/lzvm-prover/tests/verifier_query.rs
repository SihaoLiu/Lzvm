use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, parse_constant_opening_segment, ConstantOpeningQuerySegment,
    ConstantOpeningSegment, ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo, NamedStageValue};
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, PcsEvaluationSegment, PcsEvaluationUnitSegment,
    PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::{
    PcsFriOpeningLayerSegment, PcsFriOpeningQuerySegment, PcsFriOpeningUnitSegment,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::pcs_proof_values_segment::{
    encode_pcs_proof_values_segment, PcsProofValuesSegment, PCS_PROOF_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::PcsQueryPlanUnit;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_artifacts::verifier_info::{
    VerifierCode, VerifierDestination, VerifierOperand, VerifierOperation, VerifierOperationKind,
};
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, parse_witness_opening_segment, WitnessOpeningQuerySegment,
    WitnessOpeningSegment, WitnessOpeningStageSegment, WitnessOpeningUnitSegment,
    WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_field::{Ext3, Felt, SHIFT};
use lzvm_prover::pcs_transcript_segments::PcsTranscriptUnitChallenges;
use lzvm_prover::verifier_query::{
    assemble_verifier_query_eval_input, evaluate_verifier_unit_queries,
    validate_verifier_query_outputs_against_fri_opening,
    validate_verifier_query_outputs_from_segments, verify_query_outputs_against_fri_opening,
    VerifierFriComparisonError, VerifierFriComparisonRequest, VerifierFriQueryOutputSegmentsError,
    VerifierFriQueryOutputSegmentsRequest, VerifierFriQueryOutputValidationError,
    VerifierFriQueryOutputValidationRequest, VerifierQueryEvalError, VerifierQueryEvalInputRequest,
    VerifierUnitQueryEvalRequest,
};
use lzvm_prover::ProveUnitSchedule;

fn f(value: u64) -> Felt {
    Felt::from_u64(value)
}

fn e(values: [u64; 3]) -> Ext3 {
    Ext3::from_u64s(values)
}

fn schedule() -> ProveUnitSchedule {
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group".to_owned()),
        unit_name: Some("unit".to_owned()),
        base_domain_bits: 4,
        extended_domain_bits: 6,
        base_domain_size: 16,
        extended_domain_size: 64,
        blowup_factor: 4,
        query_count: 1,
        proof_of_work_bits: 8,
        merkle_tree_arity: 4,
        last_level_verification: 2,
        transcript_arity: Some(4),
        hash_commits: false,
        transcript_root_challenge_draws: vec![2, 1, 1],
        challenge_count: 6,
        evaluation_value_count: 2,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 2,
        constant_width: 3,
        stage_commit_widths: vec![2, 3],
        commitment_columns: vec![
            CommitmentColumn {
                name: "trace.a".to_owned(),
                stage: 1,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                stage_position: 1,
                intermediate: false,
                lengths: Vec::new(),
            },
            CommitmentColumn {
                name: "aux.a".to_owned(),
                stage: 2,
                dimension: 3,
                pols_map_id: 1,
                stage_id: 0,
                stage_position: 0,
                intermediate: false,
                lengths: Vec::new(),
            },
        ],
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0, -1],
        fri_layers: Vec::new(),
        final_layer_bits: 4,
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

fn tmp(id: u32) -> VerifierOperand {
    VerifierOperand::temporary(id, 3)
}

fn destination(id: u32) -> VerifierDestination {
    VerifierDestination::temporary(id, 3)
}

fn constant(id: u32, dimension: u32) -> VerifierOperand {
    VerifierOperand::constant(id, dimension)
}

fn commitment(id: u32, dimension: u32) -> VerifierOperand {
    VerifierOperand::commitment(id, dimension)
}

fn operation(
    op: VerifierOperationKind,
    destination: VerifierDestination,
    sources: Vec<VerifierOperand>,
) -> VerifierOperation {
    VerifierOperation {
        op,
        destination,
        sources,
    }
}

#[test]
fn assembles_single_query_verifier_inputs_from_opening_segments() {
    let schedule = schedule();
    let challenges = vec![
        e([2, 0, 0]),
        e([3, 0, 0]),
        e([4, 0, 0]),
        e([5, 1, 2]),
        e([7, 0, 0]),
        e([11, 0, 0]),
        e([13, 0, 0]),
    ];
    let proof_values = vec![e([23, 0, 0])];
    let constants = ConstantOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        queries: vec![ConstantOpeningQuerySegment {
            row_index: 9,
            values: vec![31, 37, 41],
            siblings: Vec::new(),
        }],
    };
    let witness = WitnessOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        queries: vec![WitnessOpeningQuerySegment {
            row_index: 9,
            stages: vec![
                WitnessOpeningStageSegment {
                    stage_index: 1,
                    values: vec![101, 103],
                    siblings: Vec::new(),
                },
                WitnessOpeningStageSegment {
                    stage_index: 2,
                    values: vec![201, 203, 211],
                    siblings: Vec::new(),
                },
            ],
        }],
    };
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        values: vec![[43, 47, 53], [59, 61, 67]],
    };

    let inputs = assemble_verifier_query_eval_input(
        &schedule,
        VerifierQueryEvalInputRequest {
            unit_index: 7,
            query_index: 0,
            challenges: &challenges,
            proof_values: &proof_values,
            constant_unit: &constants,
            witness_unit: &witness,
            evaluations: &evaluations,
        },
    )
    .expect("query inputs should assemble");

    assert_eq!(inputs.constants, vec![f(31), f(37), f(41)]);
    assert_eq!(inputs.evaluations, vec![e([43, 47, 53]), e([59, 61, 67])]);
    assert_eq!(inputs.opened_stages[0].stage_index, 1);
    assert_eq!(inputs.opened_stages[0].values, vec![f(101), f(103)]);
    assert_eq!(inputs.opened_stages[1].stage_index, 2);
    assert_eq!(inputs.opened_stages[1].values, vec![f(201), f(203), f(211)]);
    assert_eq!(inputs.commitment_columns[1].stage_index, 2);
    assert_eq!(inputs.commitment_columns[1].position, 0);

    let xi = challenges[schedule.challenge_count - 3];
    let root_ext = Felt::root_of_unity(schedule.extended_domain_bits as usize).unwrap();
    let root_base = Felt::root_of_unity(schedule.base_domain_bits as usize).unwrap();
    let x = Ext3::new(SHIFT * root_ext.pow(9), Felt::ZERO, Felt::ZERO);
    let wi = root_base.inverse().unwrap();
    let expected = (x - xi * Ext3::new(wi, Felt::ZERO, Felt::ZERO))
        .inverse()
        .unwrap();
    assert_eq!(inputs.x_div_x_sub[1], expected);

    let code = VerifierCode {
        expression_id: None,
        stage: None,
        line: String::new(),
        temporary_count: 3,
        operations: vec![
            operation(
                VerifierOperationKind::Add,
                destination(0),
                vec![commitment(1, 3), constant(1, 1)],
            ),
            operation(
                VerifierOperationKind::Mul,
                destination(1),
                vec![tmp(0), VerifierOperand::x_div_x_sub(1, 3)],
            ),
            operation(VerifierOperationKind::Copy, destination(0), vec![tmp(1)]),
        ],
    };

    let value = inputs
        .evaluate(&code, &[])
        .expect("query code should evaluate");

    assert_eq!(value, (e([201, 203, 211]) + e([37, 0, 0])) * expected);
}

#[test]
fn rejects_verifier_query_eval_trace_instance_mismatches() {
    let schedule = schedule();
    let constants = ConstantOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        queries: vec![ConstantOpeningQuerySegment {
            row_index: 9,
            values: vec![31, 37, 41],
            siblings: Vec::new(),
        }],
    };
    let witness = WitnessOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 1,
        queries: vec![WitnessOpeningQuerySegment {
            row_index: 9,
            stages: vec![WitnessOpeningStageSegment {
                stage_index: 1,
                values: vec![101, 103],
                siblings: Vec::new(),
            }],
        }],
    };
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        values: Vec::new(),
    };

    let error = assemble_verifier_query_eval_input(
        &schedule,
        VerifierQueryEvalInputRequest {
            unit_index: 7,
            query_index: 0,
            challenges: &[e([2, 0, 0]), e([3, 0, 0]), e([4, 0, 0])],
            proof_values: &[],
            constant_unit: &constants,
            witness_unit: &witness,
            evaluations: &evaluations,
        },
    )
    .expect_err("trace identity mismatch should reject");

    assert_eq!(
        error,
        VerifierQueryEvalError::TraceInstanceMismatch {
            expected: 0,
            found: 1,
            source: "witness opening",
        }
    );
}

#[test]
fn evaluates_all_unit_query_verifier_outputs() {
    let mut schedule = schedule();
    schedule.query_count = 2;
    let challenges = vec![
        e([2, 0, 0]),
        e([3, 0, 0]),
        e([4, 0, 0]),
        e([5, 1, 2]),
        e([7, 0, 0]),
        e([11, 0, 0]),
    ];
    let constants = ConstantOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        queries: vec![
            ConstantOpeningQuerySegment {
                row_index: 9,
                values: vec![31, 37, 41],
                siblings: Vec::new(),
            },
            ConstantOpeningQuerySegment {
                row_index: 10,
                values: vec![43, 47, 53],
                siblings: Vec::new(),
            },
        ],
    };
    let witness = WitnessOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        queries: vec![
            WitnessOpeningQuerySegment {
                row_index: 9,
                stages: vec![
                    WitnessOpeningStageSegment {
                        stage_index: 1,
                        values: vec![101, 103],
                        siblings: Vec::new(),
                    },
                    WitnessOpeningStageSegment {
                        stage_index: 2,
                        values: vec![201, 203, 211],
                        siblings: Vec::new(),
                    },
                ],
            },
            WitnessOpeningQuerySegment {
                row_index: 10,
                stages: vec![
                    WitnessOpeningStageSegment {
                        stage_index: 1,
                        values: vec![107, 109],
                        siblings: Vec::new(),
                    },
                    WitnessOpeningStageSegment {
                        stage_index: 2,
                        values: vec![307, 311, 313],
                        siblings: Vec::new(),
                    },
                ],
            },
        ],
    };
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        values: vec![[59, 61, 67]],
    };
    let code = VerifierCode {
        expression_id: None,
        stage: None,
        line: String::new(),
        temporary_count: 2,
        operations: vec![
            operation(
                VerifierOperationKind::Add,
                destination(1),
                vec![commitment(1, 3), constant(0, 1)],
            ),
            operation(VerifierOperationKind::Copy, destination(0), vec![tmp(1)]),
        ],
    };

    let values = evaluate_verifier_unit_queries(
        &schedule,
        VerifierUnitQueryEvalRequest {
            unit_index: 7,
            challenges: &challenges,
            proof_values: &[],
            constant_unit: &constants,
            witness_unit: &witness,
            evaluations: &evaluations,
            code: &code,
            publics: &[],
        },
    )
    .expect("unit query verifier should evaluate");

    assert_eq!(
        values,
        vec![
            e([201, 203, 211]) + e([31, 0, 0]),
            e([307, 311, 313]) + e([43, 0, 0])
        ]
    );
}

#[test]
fn compares_query_verifier_outputs_to_first_fri_layer_values() {
    let mut schedule = schedule();
    schedule.query_count = 2;
    schedule.fri_layers = vec![PcsFriLayer {
        input_bits: 6,
        output_bits: 4,
        folding_factor: 4,
    }];
    let expected_first = e([201, 203, 211]);
    let expected_second = e([307, 311, 313]);
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        layers: vec![PcsFriOpeningLayerSegment {
            layer_index: 0,
            root: [0, 0, 0, 0],
            last_level: Vec::new(),
            queries: vec![
                PcsFriOpeningQuerySegment {
                    row_index: 9,
                    values: vec![
                        expected_first.to_u64s(),
                        e([1, 0, 0]).to_u64s(),
                        e([2, 0, 0]).to_u64s(),
                        e([3, 0, 0]).to_u64s(),
                    ],
                    siblings: Vec::new(),
                },
                PcsFriOpeningQuerySegment {
                    row_index: 1,
                    values: vec![
                        e([4, 0, 0]).to_u64s(),
                        expected_second.to_u64s(),
                        e([5, 0, 0]).to_u64s(),
                        e([6, 0, 0]).to_u64s(),
                    ],
                    siblings: Vec::new(),
                },
            ],
        }],
        final_polynomial: vec![e([11, 0, 0]).to_u64s()],
    };

    let valid = verify_query_outputs_against_fri_opening(
        &schedule,
        VerifierFriComparisonRequest {
            unit_index: 7,
            trace_instance_index: 0,
            query_rows: &[9, 17],
            query_outputs: &[expected_first, expected_second],
            fri: &fri,
        },
    )
    .expect("FRI comparison should evaluate");

    assert!(valid);

    let invalid = verify_query_outputs_against_fri_opening(
        &schedule,
        VerifierFriComparisonRequest {
            unit_index: 7,
            trace_instance_index: 0,
            query_rows: &[9, 17],
            query_outputs: &[expected_first, e([999, 0, 0])],
            fri: &fri,
        },
    )
    .expect("FRI comparison should evaluate mismatches");

    assert!(!invalid);

    let mut wrong_trace_fri = fri.clone();
    wrong_trace_fri.trace_instance_index = 1;
    assert_eq!(
        verify_query_outputs_against_fri_opening(
            &schedule,
            VerifierFriComparisonRequest {
                unit_index: 7,
                trace_instance_index: 0,
                query_rows: &[9, 17],
                query_outputs: &[expected_first, expected_second],
                fri: &wrong_trace_fri,
            },
        ),
        Err(VerifierFriComparisonError::TraceInstanceMismatch {
            expected: 0,
            found: 1,
        })
    );
}

#[test]
fn validates_verifier_query_outputs_against_fri_opening() {
    let mut schedule = schedule();
    schedule.query_count = 2;
    schedule.fri_layers = vec![PcsFriLayer {
        input_bits: 6,
        output_bits: 4,
        folding_factor: 4,
    }];
    let challenges = vec![
        e([2, 0, 0]),
        e([3, 0, 0]),
        e([4, 0, 0]),
        e([5, 1, 2]),
        e([7, 0, 0]),
        e([11, 0, 0]),
    ];
    let constants = ConstantOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        queries: vec![
            ConstantOpeningQuerySegment {
                row_index: 9,
                values: vec![31, 37, 41],
                siblings: Vec::new(),
            },
            ConstantOpeningQuerySegment {
                row_index: 10,
                values: vec![43, 47, 53],
                siblings: Vec::new(),
            },
        ],
    };
    let witness = WitnessOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        queries: vec![
            WitnessOpeningQuerySegment {
                row_index: 9,
                stages: vec![
                    WitnessOpeningStageSegment {
                        stage_index: 1,
                        values: vec![101, 103],
                        siblings: Vec::new(),
                    },
                    WitnessOpeningStageSegment {
                        stage_index: 2,
                        values: vec![201, 203, 211],
                        siblings: Vec::new(),
                    },
                ],
            },
            WitnessOpeningQuerySegment {
                row_index: 10,
                stages: vec![
                    WitnessOpeningStageSegment {
                        stage_index: 1,
                        values: vec![107, 109],
                        siblings: Vec::new(),
                    },
                    WitnessOpeningStageSegment {
                        stage_index: 2,
                        values: vec![307, 311, 313],
                        siblings: Vec::new(),
                    },
                ],
            },
        ],
    };
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        values: vec![[59, 61, 67]],
    };
    let code = VerifierCode {
        expression_id: None,
        stage: None,
        line: String::new(),
        temporary_count: 2,
        operations: vec![
            operation(
                VerifierOperationKind::Add,
                destination(1),
                vec![commitment(1, 3), constant(0, 1)],
            ),
            operation(VerifierOperationKind::Copy, destination(0), vec![tmp(1)]),
        ],
    };
    let expected_first = e([201, 203, 211]) + e([31, 0, 0]);
    let expected_second = e([307, 311, 313]) + e([43, 0, 0]);
    let mut fri = PcsFriOpeningUnitSegment {
        unit_index: 7,
        trace_instance_index: 0,
        layers: vec![PcsFriOpeningLayerSegment {
            layer_index: 0,
            root: [0, 0, 0, 0],
            last_level: Vec::new(),
            queries: vec![
                PcsFriOpeningQuerySegment {
                    row_index: 9,
                    values: vec![
                        expected_first.to_u64s(),
                        e([1, 0, 0]).to_u64s(),
                        e([2, 0, 0]).to_u64s(),
                        e([3, 0, 0]).to_u64s(),
                    ],
                    siblings: Vec::new(),
                },
                PcsFriOpeningQuerySegment {
                    row_index: 1,
                    values: vec![
                        e([4, 0, 0]).to_u64s(),
                        expected_second.to_u64s(),
                        e([5, 0, 0]).to_u64s(),
                        e([6, 0, 0]).to_u64s(),
                    ],
                    siblings: Vec::new(),
                },
            ],
        }],
        final_polynomial: vec![e([11, 0, 0]).to_u64s()],
    };

    let valid = validate_verifier_query_outputs_against_fri_opening(
        &schedule,
        VerifierFriQueryOutputValidationRequest {
            unit_index: 7,
            query_rows: &[9, 17],
            challenges: &challenges,
            proof_values: &[],
            constant_unit: &constants,
            witness_unit: &witness,
            evaluations: &evaluations,
            code: &code,
            publics: &[],
            fri: &fri,
        },
    )
    .expect("query output validation should evaluate");

    assert!(valid);

    fri.layers[0].queries[1].values[1] = e([999, 0, 0]).to_u64s();
    let invalid = validate_verifier_query_outputs_against_fri_opening(
        &schedule,
        VerifierFriQueryOutputValidationRequest {
            unit_index: 7,
            query_rows: &[9, 17],
            challenges: &challenges,
            proof_values: &[],
            constant_unit: &constants,
            witness_unit: &witness,
            evaluations: &evaluations,
            code: &code,
            publics: &[],
            fri: &fri,
        },
    )
    .expect("query output validation should evaluate mismatches");

    assert!(!invalid);

    fri.trace_instance_index = 1;
    let error = validate_verifier_query_outputs_against_fri_opening(
        &schedule,
        VerifierFriQueryOutputValidationRequest {
            unit_index: 7,
            query_rows: &[9, 17],
            challenges: &challenges,
            proof_values: &[],
            constant_unit: &constants,
            witness_unit: &witness,
            evaluations: &evaluations,
            code: &code,
            publics: &[],
            fri: &fri,
        },
    )
    .expect_err("FRI trace identity mismatch should reject");
    assert_eq!(
        error,
        VerifierFriQueryOutputValidationError::Comparison(
            VerifierFriComparisonError::TraceInstanceMismatch {
                expected: 0,
                found: 1,
            }
        )
    );
}

#[test]
fn validates_verifier_query_outputs_from_proof_segments() {
    let (unit, code, query_unit, fri, challenges, segments) =
        verifier_query_output_segments_fixture(false);
    let code_refs = [&code];

    validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
        units: &[unit],
        verifier_codes: &code_refs,
        global_info: &global_info_without_proof_values(),
        public_values: &[],
        query_units: std::slice::from_ref(&query_unit),
        opening_units: std::slice::from_ref(&fri),
        transcript_challenges: std::slice::from_ref(&challenges),
        segments: &segments,
    })
    .expect("query outputs should validate");
}

#[test]
fn validates_verifier_query_outputs_by_trace_identity() {
    let (unit, code, base_query, base_fri, base_challenges, mut segments) =
        verifier_query_output_segments_fixture(false);
    let mut trace_query = base_query.clone();
    trace_query.trace_instance_index = 1;
    trace_query.queries = vec![11, 19];

    let constant_segment = segments
        .iter_mut()
        .find(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID)
        .expect("constant opening segment should exist");
    let mut constant_opening =
        parse_constant_opening_segment(&constant_segment.data).expect("opening should parse");
    let mut trace_constant = constant_opening.units[0].clone();
    trace_constant.trace_instance_index = 1;
    trace_constant.queries[0].row_index = 11;
    trace_constant.queries[0].values = vec![71, 73, 79];
    trace_constant.queries[1].row_index = 12;
    trace_constant.queries[1].values = vec![83, 89, 97];
    constant_opening.units.push(trace_constant);
    constant_segment.data =
        encode_constant_opening_segment(&constant_opening).expect("opening should encode");

    let witness_segment = segments
        .iter_mut()
        .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
        .expect("witness opening segment should exist");
    let mut witness_opening =
        parse_witness_opening_segment(&witness_segment.data).expect("opening should parse");
    let mut trace_witness = witness_opening.units[0].clone();
    trace_witness.trace_instance_index = 1;
    trace_witness.queries[0].row_index = 11;
    trace_witness.queries[0].stages[0].values = vec![401, 409];
    trace_witness.queries[0].stages[1].values = vec![503, 509, 521];
    trace_witness.queries[1].row_index = 12;
    trace_witness.queries[1].stages[0].values = vec![601, 607];
    trace_witness.queries[1].stages[1].values = vec![701, 709, 719];
    witness_opening.units.push(trace_witness);
    witness_segment.data =
        encode_witness_opening_segment(&witness_opening).expect("opening should encode");

    let evaluation_segment = segments
        .iter_mut()
        .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
        .expect("evaluation segment should exist");
    evaluation_segment.data = encode_pcs_evaluation_segment(&PcsEvaluationSegment {
        units: vec![
            PcsEvaluationUnitSegment {
                unit_index: 0,
                trace_instance_index: 0,
                values: vec![[59, 61, 67]],
            },
            PcsEvaluationUnitSegment {
                unit_index: 0,
                trace_instance_index: 1,
                values: vec![[149, 151, 157]],
            },
        ],
    })
    .expect("evaluation segment should encode");

    let expected_first = e([503, 509, 521]) + e([71, 0, 0]);
    let expected_second = e([701, 709, 719]) + e([83, 0, 0]);
    let mut trace_fri = base_fri.clone();
    trace_fri.trace_instance_index = 1;
    trace_fri.layers[0].queries[0].row_index = 11;
    trace_fri.layers[0].queries[0].values[0] = expected_first.to_u64s();
    trace_fri.layers[0].queries[1].row_index = 3;
    trace_fri.layers[0].queries[1].values[1] = expected_second.to_u64s();

    let mut trace_challenges = base_challenges.clone();
    trace_challenges.trace_instance_index = 1;
    let code_refs = [&code];
    let query_units = [trace_query, base_query];
    let opening_units = [base_fri, trace_fri];
    let transcript_challenges = [base_challenges, trace_challenges];

    validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
        units: &[unit],
        verifier_codes: &code_refs,
        global_info: &global_info_without_proof_values(),
        public_values: &[],
        query_units: &query_units,
        opening_units: &opening_units,
        transcript_challenges: &transcript_challenges,
        segments: &segments,
    })
    .expect("query outputs should match by trace identity");
}

#[test]
fn validates_verifier_query_outputs_with_array_proof_value_offsets() {
    let (unit, mut code, query_unit, mut fri, challenges, mut segments) =
        verifier_query_output_segments_fixture(false);
    code.temporary_count = 1;
    code.operations = vec![operation(
        VerifierOperationKind::Copy,
        destination(0),
        vec![VerifierOperand::proof_value(1, 3)],
    )];
    let expected = e([70, 71, 72]);
    fri.layers[0].queries[0].values[0] = expected.to_u64s();
    fri.layers[0].queries[1].values[1] = expected.to_u64s();
    segments.push(ProofSegment {
        id: PCS_PROOF_VALUES_SEGMENT_ID,
        data: encode_pcs_proof_values_segment(&PcsProofValuesSegment {
            values: vec![[10, 11, 12], [20, 21, 22], expected.to_u64s()],
        })
        .expect("proof values segment should encode"),
    });
    let code_refs = [&code];

    validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
        units: &[unit],
        verifier_codes: &code_refs,
        global_info: &global_info_with_array_proof_values(),
        public_values: &[],
        query_units: std::slice::from_ref(&query_unit),
        opening_units: std::slice::from_ref(&fri),
        transcript_challenges: std::slice::from_ref(&challenges),
        segments: &segments,
    })
    .expect("query outputs should validate");
}

#[test]
fn rejects_verifier_query_output_mismatches_from_proof_segments() {
    let (unit, code, query_unit, fri, challenges, segments) =
        verifier_query_output_segments_fixture(true);
    let code_refs = [&code];

    let error =
        validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
            units: &[unit],
            verifier_codes: &code_refs,
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            query_units: std::slice::from_ref(&query_unit),
            opening_units: std::slice::from_ref(&fri),
            transcript_challenges: std::slice::from_ref(&challenges),
            segments: &segments,
        })
        .expect_err("mismatched query output should be rejected");

    assert_eq!(
        error,
        VerifierFriQueryOutputSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn rejects_verifier_query_outputs_extra_constant_opening_units() {
    let (unit, code, query_unit, fri, challenges, mut segments) =
        verifier_query_output_segments_fixture(false);
    let constant_segment = segments
        .iter_mut()
        .find(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID)
        .expect("constant opening segment should exist");
    let mut opening =
        parse_constant_opening_segment(&constant_segment.data).expect("opening should parse");
    let mut extra_unit = opening.units[0].clone();
    extra_unit.unit_index = 1;
    opening.units.push(extra_unit);
    constant_segment.data =
        encode_constant_opening_segment(&opening).expect("opening should encode");
    let code_refs = [&code];

    let error =
        validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
            units: &[unit],
            verifier_codes: &code_refs,
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            query_units: std::slice::from_ref(&query_unit),
            opening_units: std::slice::from_ref(&fri),
            transcript_challenges: std::slice::from_ref(&challenges),
            segments: &segments,
        })
        .expect_err("extra constant opening unit should be rejected");

    assert_eq!(
        error.to_string(),
        "unexpected constant opening segment unit 1"
    );
}

#[test]
fn rejects_verifier_query_outputs_extra_witness_opening_units() {
    let (unit, code, query_unit, fri, challenges, mut segments) =
        verifier_query_output_segments_fixture(false);
    let witness_segment = segments
        .iter_mut()
        .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
        .expect("witness opening segment should exist");
    let mut opening =
        parse_witness_opening_segment(&witness_segment.data).expect("opening should parse");
    let mut extra_unit = opening.units[0].clone();
    extra_unit.unit_index = 1;
    opening.units.push(extra_unit);
    witness_segment.data = encode_witness_opening_segment(&opening).expect("opening should encode");
    let code_refs = [&code];

    let error =
        validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
            units: &[unit],
            verifier_codes: &code_refs,
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            query_units: std::slice::from_ref(&query_unit),
            opening_units: std::slice::from_ref(&fri),
            transcript_challenges: std::slice::from_ref(&challenges),
            segments: &segments,
        })
        .expect_err("extra witness opening unit should be rejected");

    assert_eq!(
        error.to_string(),
        "unexpected witness opening segment unit 1"
    );
}

#[test]
fn rejects_verifier_query_outputs_unqueried_fri_or_challenge_units() {
    let (unit, code, query_unit, fri, challenges, segments) =
        verifier_query_output_segments_fixture(false);
    let mut extra_fri = fri.clone();
    extra_fri.trace_instance_index = 1;
    let mut extra_challenges = challenges.clone();
    extra_challenges.trace_instance_index = 1;
    let code_refs = [&code];

    let opening_units = [fri.clone(), extra_fri];
    let opening_error =
        validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
            units: std::slice::from_ref(&unit),
            verifier_codes: &code_refs,
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            query_units: std::slice::from_ref(&query_unit),
            opening_units: &opening_units,
            transcript_challenges: std::slice::from_ref(&challenges),
            segments: &segments,
        })
        .expect_err("unqueried FRI opening unit should be rejected");

    assert_eq!(
        opening_error,
        VerifierFriQueryOutputSegmentsError::UnitMismatch { unit_index: 0 }
    );

    let transcript_challenges = [challenges, extra_challenges];
    let challenge_error =
        validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
            units: std::slice::from_ref(&unit),
            verifier_codes: &code_refs,
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            query_units: std::slice::from_ref(&query_unit),
            opening_units: std::slice::from_ref(&fri),
            transcript_challenges: &transcript_challenges,
            segments: &segments,
        })
        .expect_err("unqueried transcript challenge unit should be rejected");

    assert_eq!(
        challenge_error,
        VerifierFriQueryOutputSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn rejects_verifier_query_outputs_duplicate_fri_or_challenge_units() {
    let (unit, code, query_unit, fri, challenges, segments) =
        verifier_query_output_segments_fixture(false);
    let code_refs = [&code];

    let opening_units = [fri.clone(), fri.clone()];
    let opening_error =
        validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
            units: std::slice::from_ref(&unit),
            verifier_codes: &code_refs,
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            query_units: std::slice::from_ref(&query_unit),
            opening_units: &opening_units,
            transcript_challenges: std::slice::from_ref(&challenges),
            segments: &segments,
        })
        .expect_err("duplicate FRI opening unit should be rejected");

    assert_eq!(
        opening_error,
        VerifierFriQueryOutputSegmentsError::UnitMismatch { unit_index: 0 }
    );

    let transcript_challenges = [challenges.clone(), challenges];
    let challenge_error =
        validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
            units: std::slice::from_ref(&unit),
            verifier_codes: &code_refs,
            global_info: &global_info_without_proof_values(),
            public_values: &[],
            query_units: std::slice::from_ref(&query_unit),
            opening_units: std::slice::from_ref(&fri),
            transcript_challenges: &transcript_challenges,
            segments: &segments,
        })
        .expect_err("duplicate transcript challenge unit should be rejected");

    assert_eq!(
        challenge_error,
        VerifierFriQueryOutputSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

fn verifier_query_output_segments_fixture(
    mismatch: bool,
) -> (
    ProveUnitSchedule,
    VerifierCode,
    PcsQueryPlanUnit,
    PcsFriOpeningUnitSegment,
    PcsTranscriptUnitChallenges,
    Vec<ProofSegment>,
) {
    let mut unit = schedule();
    unit.query_count = 2;
    unit.evaluation_value_count = 1;
    unit.fri_layers = vec![PcsFriLayer {
        input_bits: 6,
        output_bits: 4,
        folding_factor: 4,
    }];
    let query_unit = PcsQueryPlanUnit {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![9, 17],
    };
    let constants = ConstantOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![
            ConstantOpeningQuerySegment {
                row_index: 9,
                values: vec![31, 37, 41],
                siblings: Vec::new(),
            },
            ConstantOpeningQuerySegment {
                row_index: 10,
                values: vec![43, 47, 53],
                siblings: Vec::new(),
            },
        ],
    };
    let witness = WitnessOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        queries: vec![
            WitnessOpeningQuerySegment {
                row_index: 9,
                stages: vec![
                    WitnessOpeningStageSegment {
                        stage_index: 1,
                        values: vec![101, 103],
                        siblings: Vec::new(),
                    },
                    WitnessOpeningStageSegment {
                        stage_index: 2,
                        values: vec![201, 203, 211],
                        siblings: Vec::new(),
                    },
                ],
            },
            WitnessOpeningQuerySegment {
                row_index: 10,
                stages: vec![
                    WitnessOpeningStageSegment {
                        stage_index: 1,
                        values: vec![107, 109],
                        siblings: Vec::new(),
                    },
                    WitnessOpeningStageSegment {
                        stage_index: 2,
                        values: vec![307, 311, 313],
                        siblings: Vec::new(),
                    },
                ],
            },
        ],
    };
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        values: vec![[59, 61, 67]],
    };
    let code = VerifierCode {
        expression_id: None,
        stage: None,
        line: String::new(),
        temporary_count: 2,
        operations: vec![
            operation(
                VerifierOperationKind::Add,
                destination(1),
                vec![commitment(1, 3), constant(0, 1)],
            ),
            operation(VerifierOperationKind::Copy, destination(0), vec![tmp(1)]),
        ],
    };
    let expected_first = e([201, 203, 211]) + e([31, 0, 0]);
    let expected_second = e([307, 311, 313]) + e([43, 0, 0]);
    let second_value = if mismatch {
        e([999, 0, 0])
    } else {
        expected_second
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        trace_instance_index: 0,
        layers: vec![PcsFriOpeningLayerSegment {
            layer_index: 0,
            root: [0, 0, 0, 0],
            last_level: Vec::new(),
            queries: vec![
                PcsFriOpeningQuerySegment {
                    row_index: 9,
                    values: vec![
                        expected_first.to_u64s(),
                        e([1, 0, 0]).to_u64s(),
                        e([2, 0, 0]).to_u64s(),
                        e([3, 0, 0]).to_u64s(),
                    ],
                    siblings: Vec::new(),
                },
                PcsFriOpeningQuerySegment {
                    row_index: 1,
                    values: vec![
                        e([4, 0, 0]).to_u64s(),
                        second_value.to_u64s(),
                        e([5, 0, 0]).to_u64s(),
                        e([6, 0, 0]).to_u64s(),
                    ],
                    siblings: Vec::new(),
                },
            ],
        }],
        final_polynomial: vec![e([11, 0, 0]).to_u64s()],
    };
    let challenges = PcsTranscriptUnitChallenges {
        unit_index: 0,
        trace_instance_index: 0,
        challenges: vec![
            e([2, 0, 0]),
            e([3, 0, 0]),
            e([4, 0, 0]),
            e([5, 1, 2]),
            e([7, 0, 0]),
            e([11, 0, 0]),
        ],
    };
    let segments = vec![
        ProofSegment {
            id: CONSTANT_OPENING_SEGMENT_ID,
            data: encode_constant_opening_segment(&ConstantOpeningSegment {
                units: vec![constants],
            })
            .expect("constant opening should encode"),
        },
        ProofSegment {
            id: WITNESS_OPENING_SEGMENT_ID,
            data: encode_witness_opening_segment(&WitnessOpeningSegment {
                units: vec![witness],
            })
            .expect("witness opening should encode"),
        },
        ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: encode_pcs_evaluation_segment(&PcsEvaluationSegment {
                units: vec![evaluations],
            })
            .expect("evaluation segment should encode"),
        },
    ];

    (unit, code, query_unit, fri, challenges, segments)
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
        transcript_arity: 4,
    }
}

fn global_info_with_array_proof_values() -> GlobalInfo {
    let mut global = global_info_without_proof_values();
    global.num_proof_values = vec![0, 2];
    global.proof_values_map = vec![
        NamedStageValue {
            name: "expected".to_owned(),
            stage: 2,
            id: None,
            lengths: vec![2],
        },
        NamedStageValue {
            name: "actual".to_owned(),
            stage: 2,
            id: None,
            lengths: Vec::new(),
        },
    ];
    global
}

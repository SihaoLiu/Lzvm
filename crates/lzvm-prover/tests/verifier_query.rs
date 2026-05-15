use lzvm_artifacts::constant_opening_segment::{
    ConstantOpeningQuerySegment, ConstantOpeningUnitSegment,
};
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_evaluation_segment::PcsEvaluationUnitSegment;
use lzvm_artifacts::pcs_fri_segment::{
    PcsFriOpeningLayerSegment, PcsFriOpeningQuerySegment, PcsFriOpeningUnitSegment,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_artifacts::verifier_info::{VerifierCode, VerifierOperation, VerifierOperationKind};
use lzvm_artifacts::witness_opening_segment::{
    WitnessOpeningQuerySegment, WitnessOpeningStageSegment, WitnessOpeningUnitSegment,
};
use lzvm_field::{Ext3, Felt, SHIFT};
use lzvm_prover::verifier_query::{
    assemble_verifier_query_eval_input, evaluate_verifier_unit_queries,
    verify_query_outputs_against_fri_opening, VerifierFriComparisonRequest,
    VerifierQueryEvalInputRequest, VerifierUnitQueryEvalRequest,
};
use lzvm_prover::ProveUnitSchedule;
use serde_json::json;

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

fn tmp(id: u32) -> serde_json::Value {
    json!({"type": "tmp", "id": id, "dim": 3})
}

fn operation(
    op: VerifierOperationKind,
    destination: serde_json::Value,
    sources: Vec<serde_json::Value>,
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
        queries: vec![ConstantOpeningQuerySegment {
            row_index: 9,
            values: vec![31, 37, 41],
            siblings: Vec::new(),
        }],
    };
    let witness = WitnessOpeningUnitSegment {
        unit_index: 7,
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
                tmp(0),
                vec![
                    json!({"type": "cm", "id": 1, "dim": 3}),
                    json!({"type": "const", "id": 1, "dim": 1}),
                ],
            ),
            operation(
                VerifierOperationKind::Mul,
                tmp(1),
                vec![tmp(0), json!({"type": "xDivXSub", "id": 1, "dim": 3})],
            ),
            operation(VerifierOperationKind::Copy, tmp(0), vec![tmp(1)]),
        ],
    };

    let value = inputs
        .evaluate(&code, &[])
        .expect("query code should evaluate");

    assert_eq!(value, (e([201, 203, 211]) + e([37, 0, 0])) * expected);
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
                tmp(1),
                vec![
                    json!({"type": "cm", "id": 1, "dim": 3}),
                    json!({"type": "const", "id": 0, "dim": 1}),
                ],
            ),
            operation(VerifierOperationKind::Copy, tmp(0), vec![tmp(1)]),
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
            query_rows: &[9, 17],
            query_outputs: &[expected_first, e([999, 0, 0])],
            fri: &fri,
        },
    )
    .expect("FRI comparison should evaluate mismatches");

    assert!(!invalid);
}

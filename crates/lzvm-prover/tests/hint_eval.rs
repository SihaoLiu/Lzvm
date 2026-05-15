use lzvm_artifacts::global_info::{
    AggregationType, CurveKind, GlobalAir, GlobalInfo, NamedStageValue, PublicValue,
};
use lzvm_artifacts::hint_program::{Hint, HintField, HintOperand, HintProgram, HintValue};
use lzvm_artifacts::setup_info::{
    Boundary, CommitmentColumn, ConstantColumn, FriStep, StageValue, StarkStruct, UnitSetupInfo,
};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::global_constraints::GlobalConstraintInputs;
use lzvm_prover::hint_eval::{
    resolve_global_hint_field, resolve_regular_hint_field, HintEvalError, ResolvedHintPayload,
    ResolvedHintValue,
};
use lzvm_prover::regular_constraints::{
    RegularColumnMatrix, RegularConstraintInputs, RegularStageColumns,
};

#[test]
fn resolves_global_hint_values_from_runtime_inputs() {
    let program = HintProgram {
        hints: vec![Hint {
            name: "hint-a".to_owned(),
            fields: vec![HintField {
                name: "values".to_owned(),
                values: vec![
                    HintValue {
                        operand: HintOperand::Number(42),
                        positions: vec![0, 2],
                    },
                    HintValue {
                        operand: HintOperand::String("label".to_owned()),
                        positions: vec![],
                    },
                    HintValue {
                        operand: HintOperand::Public { id: 1 },
                        positions: vec![1],
                    },
                    HintValue {
                        operand: HintOperand::ProofValue { id: 0 },
                        positions: vec![2],
                    },
                    HintValue {
                        operand: HintOperand::ProofValue { id: 1 },
                        positions: vec![3],
                    },
                    HintValue {
                        operand: HintOperand::GroupValue { group_id: 1, id: 0 },
                        positions: vec![4],
                    },
                ],
            }],
        }],
    };

    let resolved = resolve_global_hint_field(
        &sample_global_info(),
        &program,
        0,
        "values",
        GlobalConstraintInputs {
            publics: &[felt(7), felt(11)],
            proof_values: &[felt(13), felt(17), felt(19), felt(23)],
            challenges: &[ext([29, 31, 37])],
            group_values: &[ext([41, 43, 47]), ext([53, 59, 61]), ext([67, 71, 73])],
        },
    )
    .expect("hint field should resolve");

    assert_eq!(
        resolved,
        vec![
            ResolvedHintValue {
                payload: ResolvedHintPayload::Scalar(felt(42)),
                positions: vec![0, 2],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Text("label".to_owned()),
                positions: vec![],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Scalar(felt(11)),
                positions: vec![1],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Scalar(felt(13)),
                positions: vec![2],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Extension(ext([17, 19, 23])),
                positions: vec![3],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Extension(ext([67, 71, 73])),
                positions: vec![4],
            },
        ]
    );
}

#[test]
fn rejects_global_hint_temporaries_without_expression_inputs() {
    let program = HintProgram {
        hints: vec![Hint {
            name: "hint-a".to_owned(),
            fields: vec![HintField {
                name: "values".to_owned(),
                values: vec![HintValue {
                    operand: HintOperand::Temporary {
                        id: 0,
                        dimension: None,
                    },
                    positions: vec![],
                }],
            }],
        }],
    };

    let error = resolve_global_hint_field(
        &sample_global_info(),
        &program,
        0,
        "values",
        GlobalConstraintInputs {
            publics: &[],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect_err("temporary operands need expression inputs");

    assert_eq!(
        error,
        HintEvalError::UnsupportedOperand {
            operand: "temporary"
        }
    );
}

#[test]
fn resolves_regular_hint_values_from_row_inputs() {
    let program = HintProgram {
        hints: vec![Hint {
            name: "hint-a".to_owned(),
            fields: vec![HintField {
                name: "values".to_owned(),
                values: vec![
                    HintValue {
                        operand: HintOperand::Constant {
                            id: 10,
                            row_offset_index: 0,
                        },
                        positions: vec![0],
                    },
                    HintValue {
                        operand: HintOperand::Commitment {
                            id: 20,
                            row_offset_index: 1,
                        },
                        positions: vec![1],
                    },
                    HintValue {
                        operand: HintOperand::AirValue { id: 1 },
                        positions: vec![2],
                    },
                    HintValue {
                        operand: HintOperand::AirGroupValue { id: 0 },
                        positions: vec![3],
                    },
                    HintValue {
                        operand: HintOperand::Challenge { id: 0 },
                        positions: vec![4],
                    },
                    HintValue {
                        operand: HintOperand::Public { id: 1 },
                        positions: vec![5],
                    },
                    HintValue {
                        operand: HintOperand::ProofValue { id: 0 },
                        positions: vec![6],
                    },
                    HintValue {
                        operand: HintOperand::Number(77),
                        positions: vec![7],
                    },
                    HintValue {
                        operand: HintOperand::String("row-label".to_owned()),
                        positions: vec![8],
                    },
                ],
            }],
        }],
    };
    let fixed = [
        felt(101),
        felt(102),
        felt(201),
        felt(202),
        felt(301),
        felt(302),
    ];
    let stage_values = [
        felt(401),
        felt(402),
        felt(403),
        felt(501),
        felt(502),
        felt(503),
        felt(601),
        felt(602),
        felt(603),
    ];
    let stage_columns = [RegularStageColumns {
        stage_index: 2,
        column_count: 3,
        values: &stage_values,
    }];

    let resolved = resolve_regular_hint_field(
        &sample_unit_setup_info(),
        &program,
        0,
        "values",
        1,
        RegularConstraintInputs {
            domain_size: 3,
            stage_count: 2,
            fixed_columns: RegularColumnMatrix {
                column_count: 2,
                values: &fixed,
            },
            stage_columns: &stage_columns,
            opening_point_offsets: &[0, 1],
            publics: &[felt(701), felt(702)],
            unit_values: &[felt(801), felt(901), felt(902), felt(903)],
            proof_values: &[felt(1001)],
            group_values: &[ext([1101, 1102, 1103])],
            challenges: &[ext([1201, 1202, 1203])],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular hint field should resolve");

    assert_eq!(
        resolved,
        vec![
            ResolvedHintValue {
                payload: ResolvedHintPayload::Scalar(felt(201)),
                positions: vec![0],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Extension(ext([601, 602, 603])),
                positions: vec![1],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Extension(ext([901, 902, 903])),
                positions: vec![2],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Extension(ext([1101, 1102, 1103])),
                positions: vec![3],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Extension(ext([1201, 1202, 1203])),
                positions: vec![4],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Scalar(felt(702)),
                positions: vec![5],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Scalar(felt(1001)),
                positions: vec![6],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Scalar(felt(77)),
                positions: vec![7],
            },
            ResolvedHintValue {
                payload: ResolvedHintPayload::Text("row-label".to_owned()),
                positions: vec![8],
            },
        ]
    );
}

#[test]
fn rejects_regular_hint_temporaries_without_expression_inputs() {
    let program = HintProgram {
        hints: vec![Hint {
            name: "hint-a".to_owned(),
            fields: vec![HintField {
                name: "values".to_owned(),
                values: vec![HintValue {
                    operand: HintOperand::Temporary {
                        id: 0,
                        dimension: Some(1),
                    },
                    positions: vec![],
                }],
            }],
        }],
    };

    let error = resolve_regular_hint_field(
        &sample_unit_setup_info(),
        &program,
        0,
        "values",
        0,
        RegularConstraintInputs {
            domain_size: 1,
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect_err("temporary operands need expression inputs");

    assert_eq!(
        error,
        HintEvalError::UnsupportedOperand {
            operand: "temporary"
        }
    );
}

fn sample_global_info() -> GlobalInfo {
    GlobalInfo {
        name: "sample-program".to_owned(),
        air_groups: vec!["group-a".to_owned(), "group-b".to_owned()],
        airs: vec![vec![sample_air("unit-a")], vec![sample_air("unit-b")]],
        curve: CurveKind::None,
        lattice_size: None,
        aggregation_types: vec![
            vec![
                AggregationType {
                    aggregation_type: 0,
                },
                AggregationType {
                    aggregation_type: 1,
                },
            ],
            vec![AggregationType {
                aggregation_type: 0,
            }],
        ],
        n_publics: 2,
        num_challenges: vec![1],
        num_proof_values: vec![2],
        proof_values_map: vec![
            NamedStageValue {
                name: "pv-a".to_owned(),
                stage: 1,
                id: None,
                lengths: Vec::new(),
            },
            NamedStageValue {
                name: "pv-b".to_owned(),
                stage: 2,
                id: None,
                lengths: Vec::new(),
            },
        ],
        publics_map: vec![
            PublicValue {
                name: "pub-a".to_owned(),
                stage: 1,
                lengths: Vec::new(),
            },
            PublicValue {
                name: "pub-b".to_owned(),
                stage: 1,
                lengths: Vec::new(),
            },
        ],
        transcript_arity: 4,
    }
}

fn sample_unit_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 2,
        n_constants: 2,
        constant_columns: vec![
            ConstantColumn {
                name: "fixed-a".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 10,
                stage_id: 0,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "fixed-b".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 11,
                stage_id: 1,
                lengths: Vec::new(),
            },
        ],
        n_publics: Some(2),
        n_constraints: Some(0),
        q_degree: 3,
        opening_points: vec![0, 1],
        section_widths: std::collections::BTreeMap::new(),
        challenge_count: 1,
        eval_count: 0,
        boundaries: Vec::<Boundary>::new(),
        commitment_columns: vec![CommitmentColumn {
            name: "trace-a".to_owned(),
            stage: 2,
            dimension: 3,
            pols_map_id: 20,
            stage_id: 0,
            stage_position: 0,
            intermediate: false,
            lengths: Vec::new(),
        }],
        unit_value_map: vec![
            StageValue {
                name: "unit-a".to_owned(),
                stage: 1,
                lengths: Vec::new(),
            },
            StageValue {
                name: "unit-b".to_owned(),
                stage: 2,
                lengths: Vec::new(),
            },
        ],
        group_value_map: vec![StageValue {
            name: "group-a".to_owned(),
            stage: 2,
            lengths: Vec::new(),
        }],
        stark: StarkStruct {
            n_bits: 1,
            n_bits_ext: 2,
            n_queries: 1,
            steps: vec![FriStep { n_bits: 2 }],
            hash_commits: true,
            last_level_verification: 1,
            pow_bits: 0,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    }
}

fn sample_air(name: &str) -> GlobalAir {
    GlobalAir {
        name: name.to_owned(),
        num_rows: 2,
        has_compressor: false,
    }
}

fn felt(value: u64) -> Felt {
    Felt::from_u64(value)
}

fn ext(values: [u64; 3]) -> Ext3 {
    Ext3::from_u64s(values)
}

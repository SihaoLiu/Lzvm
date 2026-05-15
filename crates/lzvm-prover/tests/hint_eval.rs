use lzvm_artifacts::global_info::{
    AggregationType, CurveKind, GlobalAir, GlobalInfo, NamedStageValue, PublicValue,
};
use lzvm_artifacts::hint_program::{Hint, HintField, HintOperand, HintProgram, HintValue};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::global_constraints::GlobalConstraintInputs;
use lzvm_prover::hint_eval::{
    resolve_global_hint_field, HintEvalError, ResolvedHintPayload, ResolvedHintValue,
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

#![allow(dead_code)]

use std::collections::BTreeMap;

use lzvm_artifacts::expression_info::{
    BoundaryKind, CodeDestination, CodeOperand, CodeOperation, ConstraintCode, ExpressionCode,
    ExpressionDestination, ExpressionInfo, HintFieldInfo, HintInfo, HintPayload, HintValueInfo,
    OperationKind,
};
use lzvm_artifacts::global_info::{
    AggregationType, CurveKind, GlobalAir, GlobalInfo, NamedStageValue, PublicValue,
};
use lzvm_artifacts::setup_info::{
    Boundary, CommitmentColumn, ConstantColumn, EvaluationMapEntry, EvaluationMapKind, FriStep,
    StageValue, StarkStruct, UnitSetupInfo,
};
use lzvm_artifacts::verifier_info::{
    VerifierCode, VerifierDestination, VerifierInfo, VerifierOperand, VerifierOperation,
    VerifierOperationKind,
};

pub fn sample_setup_info_fixture() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 2,
        n_constants: 5,
        constant_columns: vec![
            ConstantColumn {
                name: "main.a".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.b".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 1,
                stage_id: 1,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.c".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 2,
                stage_id: 2,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.d".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 3,
                stage_id: 3,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.e".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 4,
                stage_id: 4,
                lengths: vec![5],
            },
        ],
        n_publics: Some(3),
        n_constraints: Some(8),
        q_degree: 7,
        opening_points: vec![0, 1, -1],
        section_widths: BTreeMap::from([
            ("const".to_owned(), 5),
            ("cm1".to_owned(), 2),
            ("cm2".to_owned(), 3),
            ("cm3".to_owned(), 1),
        ]),
        challenge_count: 2,
        eval_count: 3,
        evaluation_map: vec![EvaluationMapEntry::default(); 3],
        boundaries: vec![
            Boundary {
                name: Some("first".to_owned()),
                offset_min: Some(0),
                offset_max: Some(3),
            },
            Boundary {
                name: None,
                offset_min: Some(-1),
                offset_max: None,
            },
        ],
        commitment_columns: vec![
            CommitmentColumn {
                name: "trace.a".to_owned(),
                stage: 1,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                stage_position: 0,
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
        unit_value_map: vec![
            StageValue {
                name: "unit.alpha".to_owned(),
                stage: 1,
                lengths: vec![2],
            },
            StageValue {
                name: "unit.beta".to_owned(),
                stage: 2,
                lengths: Vec::new(),
            },
        ],
        group_value_map: vec![StageValue {
            name: "group.alpha".to_owned(),
            stage: 2,
            lengths: Vec::new(),
        }],
        stark: StarkStruct {
            n_bits: 10,
            n_bits_ext: 13,
            n_queries: 4,
            steps: vec![
                FriStep { n_bits: 13 },
                FriStep { n_bits: 9 },
                FriStep { n_bits: 5 },
            ],
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 20,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    }
}

pub fn sample_setup_info_fixture_with_evaluation_map() -> UnitSetupInfo {
    let mut setup = sample_setup_info_fixture();
    setup.evaluation_map = vec![
        EvaluationMapEntry {
            kind: EvaluationMapKind::Constant,
            id: 2,
            prime: 0,
            opening_position: 0,
            commit_id: None,
        },
        EvaluationMapEntry {
            kind: EvaluationMapKind::Commitment,
            id: 1,
            prime: 1,
            opening_position: 1,
            commit_id: None,
        },
        EvaluationMapEntry {
            kind: EvaluationMapKind::Custom,
            id: 7,
            prime: -1,
            opening_position: 2,
            commit_id: Some(3),
        },
    ];
    setup
}

pub fn sample_global_info_fixture() -> GlobalInfo {
    GlobalInfo {
        name: "sample-program".to_owned(),
        air_groups: vec!["group-a".to_owned(), "group-b".to_owned()],
        airs: vec![
            vec![
                GlobalAir {
                    name: "unit-a".to_owned(),
                    num_rows: 1024,
                    has_compressor: false,
                },
                GlobalAir {
                    name: "unit-b".to_owned(),
                    num_rows: 2048,
                    has_compressor: true,
                },
            ],
            vec![GlobalAir {
                name: "unit-c".to_owned(),
                num_rows: 4096,
                has_compressor: false,
            }],
        ],
        curve: CurveKind::None,
        lattice_size: Some(368),
        aggregation_types: vec![
            vec![
                AggregationType {
                    aggregation_type: 0,
                },
                AggregationType {
                    aggregation_type: 2,
                },
            ],
            Vec::new(),
        ],
        n_publics: 2,
        num_challenges: vec![1, 2, 3],
        num_proof_values: vec![1, 1],
        proof_values_map: vec![
            NamedStageValue {
                name: "proof-a".to_owned(),
                stage: 1,
                id: None,
                lengths: Vec::new(),
            },
            NamedStageValue {
                name: "proof-b".to_owned(),
                stage: 2,
                id: None,
                lengths: Vec::new(),
            },
        ],
        publics_map: vec![
            PublicValue {
                name: "public-a".to_owned(),
                stage: 1,
                lengths: Vec::new(),
            },
            PublicValue {
                name: "public-b".to_owned(),
                stage: 1,
                lengths: vec![2, 3],
            },
        ],
        transcript_arity: 4,
    }
}

pub fn sample_expression_info_fixture() -> ExpressionInfo {
    ExpressionInfo {
        hints: vec![HintInfo {
            name: "hint-a".to_owned(),
            fields: vec![HintFieldInfo {
                name: "field-a".to_owned(),
                values: vec![
                    HintValueInfo {
                        positions: vec![0],
                        payload: HintPayload::number(7),
                    },
                    HintValueInfo {
                        positions: Vec::new(),
                        payload: HintPayload::string("tag"),
                    },
                    HintValueInfo {
                        positions: vec![1, 2],
                        payload: HintPayload::temporary(3, Some(1)),
                    },
                ],
            }],
        }],
        expressions: vec![ExpressionCode {
            expression_id: 4,
            stage: 1,
            line: "expr-a".to_owned(),
            temporary_count: 2,
            destination: Some(ExpressionDestination::commitment(8, Some(1), Some(0))),
            operations: vec![
                CodeOperation {
                    op: OperationKind::Add,
                    destination: CodeDestination::temporary(0, 1),
                    sources: vec![CodeOperand::number(3, 1), CodeOperand::public(0, 1)],
                },
                CodeOperation {
                    op: OperationKind::Copy,
                    destination: CodeDestination::temporary(1, 1),
                    sources: vec![CodeOperand::temporary(0, 1)],
                },
            ],
        }],
        constraints: vec![
            ConstraintCode {
                stage: 2,
                boundary: BoundaryKind::EveryFrame,
                offset_min: Some(-1),
                offset_max: Some(2),
                line: "constraint-a".to_owned(),
                intermediate: true,
                temporary_count: 1,
                operations: vec![CodeOperation {
                    op: OperationKind::Mul,
                    destination: CodeDestination::temporary(0, 3),
                    sources: vec![
                        CodeOperand::challenge(0, Some(1), Some(0), 3),
                        CodeOperand::commitment_at(2, Some(0), 3),
                    ],
                }],
            },
            ConstraintCode {
                stage: 1,
                boundary: BoundaryKind::EveryFrame,
                offset_min: Some(0),
                offset_max: Some(1),
                line: "constraint-b".to_owned(),
                intermediate: false,
                temporary_count: 1,
                operations: vec![CodeOperation {
                    op: OperationKind::Copy,
                    destination: CodeDestination::temporary(0, 1),
                    sources: vec![CodeOperand::constant_at(2, Some(1), 1)],
                }],
            },
        ],
    }
}

pub fn sample_verifier_info_fixture() -> VerifierInfo {
    VerifierInfo {
        quotient: VerifierCode {
            expression_id: None,
            stage: None,
            line: String::new(),
            temporary_count: 2,
            operations: vec![
                VerifierOperation {
                    op: VerifierOperationKind::Mul,
                    destination: VerifierDestination::temporary(0, 3),
                    sources: vec![
                        VerifierOperand::challenge(0, Some(1), Some(0), 3),
                        VerifierOperand::evaluation(2, 3),
                    ],
                },
                VerifierOperation {
                    op: VerifierOperationKind::Copy,
                    destination: VerifierDestination::temporary(1, 3),
                    sources: vec![VerifierOperand::temporary(0, 3)],
                },
            ],
        },
        query: VerifierCode {
            expression_id: Some(9),
            stage: Some(3),
            line: "query-a".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Add,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![
                    VerifierOperand::boundary_zerofier(0, 1),
                    VerifierOperand::proof_value(1, 3),
                ],
            }],
        },
    }
}

pub fn sample_constant_tree_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 2,
        n_constants: 1,
        constant_columns: vec![ConstantColumn {
            name: "main.left".to_owned(),
            stage: 0,
            dimension: 1,
            pols_map_id: 0,
            stage_id: 0,
            lengths: Vec::new(),
        }],
        n_publics: Some(0),
        n_constraints: Some(0),
        q_degree: 7,
        opening_points: vec![0],
        section_widths: BTreeMap::from([
            ("const".to_owned(), 1),
            ("cm1".to_owned(), 1),
            ("cm2".to_owned(), 1),
            ("cm3".to_owned(), 1),
        ]),
        challenge_count: 0,
        eval_count: 0,
        evaluation_map: Vec::new(),
        boundaries: Vec::new(),
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 1,
            n_bits_ext: 2,
            n_queries: 1,
            steps: vec![FriStep { n_bits: 2 }, FriStep { n_bits: 1 }],
            hash_commits: true,
            last_level_verification: 1,
            pow_bits: 1,
            merkle_tree_arity: 2,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(2),
            merkle_tree_custom: Some(true),
        },
    }
}

pub fn sample_pcs_material_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 1,
        n_constants: 2,
        constant_columns: vec![
            ConstantColumn {
                name: "main.left".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.right".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 1,
                stage_id: 1,
                lengths: Vec::new(),
            },
        ],
        n_publics: Some(0),
        n_constraints: Some(0),
        q_degree: 3,
        opening_points: vec![0],
        section_widths: BTreeMap::from([
            ("const".to_owned(), 2),
            ("cm1".to_owned(), 1),
            ("cm2".to_owned(), 1),
        ]),
        challenge_count: 0,
        eval_count: 0,
        evaluation_map: Vec::new(),
        boundaries: Vec::new(),
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 1,
            n_bits_ext: 2,
            n_queries: 1,
            steps: vec![FriStep { n_bits: 2 }, FriStep { n_bits: 1 }],
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 0,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    }
}

pub fn sample_pcs_plan_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 2,
        n_constants: 5,
        constant_columns: vec![
            ConstantColumn {
                name: "main.a".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.b".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 1,
                stage_id: 1,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.c".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 2,
                stage_id: 2,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.d".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 3,
                stage_id: 3,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.e".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 4,
                stage_id: 4,
                lengths: vec![5],
            },
        ],
        n_publics: Some(3),
        n_constraints: Some(8),
        q_degree: 7,
        opening_points: vec![0, 1, -1],
        section_widths: BTreeMap::from([
            ("const".to_owned(), 5),
            ("cm1".to_owned(), 2),
            ("cm2".to_owned(), 3),
            ("cm3".to_owned(), 1),
        ]),
        challenge_count: 2,
        eval_count: 3,
        evaluation_map: vec![EvaluationMapEntry::default(); 3],
        boundaries: vec![
            Boundary {
                name: Some("first".to_owned()),
                offset_min: Some(0),
                offset_max: Some(3),
            },
            Boundary {
                name: None,
                offset_min: Some(-1),
                offset_max: None,
            },
        ],
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 10,
            n_bits_ext: 13,
            n_queries: 4,
            steps: vec![
                FriStep { n_bits: 13 },
                FriStep { n_bits: 9 },
                FriStep { n_bits: 5 },
            ],
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 20,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    }
}

pub fn sample_fixed_columns_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 1,
        n_constants: 2,
        constant_columns: vec![
            ConstantColumn {
                name: "main.left".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.right".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 1,
                stage_id: 1,
                lengths: vec![2],
            },
        ],
        n_publics: None,
        n_constraints: None,
        q_degree: 3,
        opening_points: vec![0],
        section_widths: BTreeMap::from([
            ("const".to_owned(), 2),
            ("cm1".to_owned(), 1),
            ("cm2".to_owned(), 1),
        ]),
        challenge_count: 0,
        eval_count: 0,
        evaluation_map: Vec::new(),
        boundaries: Vec::new(),
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 2,
            n_bits_ext: 3,
            n_queries: 2,
            steps: vec![FriStep { n_bits: 3 }, FriStep { n_bits: 1 }],
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 0,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    }
}

pub fn sample_duplicate_fixed_columns_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 1,
        n_constants: 2,
        constant_columns: vec![
            ConstantColumn {
                name: "main.value".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.value".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 1,
                stage_id: 1,
                lengths: Vec::new(),
            },
        ],
        n_publics: None,
        n_constraints: None,
        q_degree: 3,
        opening_points: vec![0],
        section_widths: BTreeMap::from([
            ("const".to_owned(), 2),
            ("cm1".to_owned(), 1),
            ("cm2".to_owned(), 1),
        ]),
        challenge_count: 0,
        eval_count: 0,
        evaluation_map: Vec::new(),
        boundaries: Vec::new(),
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 1,
            n_bits_ext: 2,
            n_queries: 2,
            steps: vec![FriStep { n_bits: 2 }, FriStep { n_bits: 1 }],
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 0,
            merkle_tree_arity: 2,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(2),
            merkle_tree_custom: Some(true),
        },
    }
}

pub fn sample_key_directory_global_info() -> GlobalInfo {
    GlobalInfo {
        name: "sample-program".to_owned(),
        air_groups: vec!["group-a".to_owned(), "group-b".to_owned()],
        airs: vec![
            vec![
                GlobalAir {
                    name: "unit-a".to_owned(),
                    num_rows: 16,
                    has_compressor: true,
                },
                GlobalAir {
                    name: "unit-b".to_owned(),
                    num_rows: 16,
                    has_compressor: false,
                },
            ],
            vec![GlobalAir {
                name: "unit-c".to_owned(),
                num_rows: 32,
                has_compressor: false,
            }],
        ],
        curve: CurveKind::None,
        lattice_size: Some(368),
        aggregation_types: vec![Vec::<AggregationType>::new(), Vec::<AggregationType>::new()],
        n_publics: 0,
        num_challenges: vec![1, 2],
        num_proof_values: Vec::new(),
        proof_values_map: Vec::new(),
        publics_map: Vec::<PublicValue>::new(),
        transcript_arity: 4,
    }
}

pub fn sample_key_directory_catalog_global_info() -> GlobalInfo {
    GlobalInfo {
        name: "sample-program".to_owned(),
        air_groups: vec!["group-a".to_owned()],
        airs: vec![vec![GlobalAir {
            name: "unit-a".to_owned(),
            num_rows: 2,
            has_compressor: false,
        }]],
        curve: CurveKind::None,
        lattice_size: Some(368),
        aggregation_types: vec![Vec::<AggregationType>::new()],
        n_publics: 0,
        num_challenges: vec![1],
        num_proof_values: Vec::new(),
        proof_values_map: Vec::new(),
        publics_map: Vec::<PublicValue>::new(),
        transcript_arity: 4,
    }
}

pub fn sample_key_directory_setup_info() -> UnitSetupInfo {
    sample_pcs_material_setup_info()
}

pub fn sample_key_directory_expression_info() -> ExpressionInfo {
    key_directory_expression_info(Vec::new())
}

pub fn sample_key_directory_expression_info_with_hints() -> ExpressionInfo {
    key_directory_expression_info(vec![HintInfo {
        name: "hint-a".to_owned(),
        fields: vec![HintFieldInfo {
            name: "field-a".to_owned(),
            values: vec![HintValueInfo {
                positions: vec![0],
                payload: HintPayload::Commitment {
                    id: 0,
                    row_offset_index: Some(0),
                    row_offset: Some(0),
                    stage: Some(1),
                    stage_id: Some(0),
                    dimension: Some(1),
                    air_group_id: None,
                    air_id: None,
                },
            }],
        }],
    }])
}

fn key_directory_expression_info(hints: Vec<HintInfo>) -> ExpressionInfo {
    ExpressionInfo {
        hints,
        expressions: vec![ExpressionCode {
            expression_id: 7,
            stage: 2,
            line: "query-expression".to_owned(),
            temporary_count: 0,
            destination: None,
            operations: Vec::new(),
        }],
        constraints: Vec::new(),
    }
}

pub fn sample_key_directory_verifier_info() -> VerifierInfo {
    VerifierInfo {
        quotient: VerifierCode {
            expression_id: None,
            stage: None,
            line: String::new(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::number(1, 1)],
            }],
        },
        query: VerifierCode {
            expression_id: Some(7),
            stage: Some(2),
            line: "query-expression".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::evaluation(0, 3)],
            }],
        },
    }
}

pub fn sample_metadata_bundle_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 2,
        n_constants: 5,
        constant_columns: vec![
            ConstantColumn {
                name: "main.a".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.b".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 1,
                stage_id: 1,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.c".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 2,
                stage_id: 2,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.d".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 3,
                stage_id: 3,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "main.e".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 4,
                stage_id: 4,
                lengths: vec![5],
            },
        ],
        n_publics: Some(2),
        n_constraints: Some(1),
        q_degree: 7,
        opening_points: vec![0, 1, -1],
        section_widths: BTreeMap::from([
            ("const".to_owned(), 5),
            ("cm1".to_owned(), 2),
            ("cm2".to_owned(), 3),
            ("cm3".to_owned(), 1),
        ]),
        challenge_count: 2,
        eval_count: 1,
        evaluation_map: vec![EvaluationMapEntry::default()],
        boundaries: Vec::new(),
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 2,
            n_bits_ext: 4,
            n_queries: 4,
            steps: vec![
                FriStep { n_bits: 4 },
                FriStep { n_bits: 3 },
                FriStep { n_bits: 1 },
            ],
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 20,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    }
}

pub fn sample_metadata_bundle_expression_info() -> ExpressionInfo {
    ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![ExpressionCode {
            expression_id: 9,
            stage: 3,
            line: "query-expression".to_owned(),
            temporary_count: 0,
            destination: None,
            operations: Vec::new(),
        }],
        constraints: vec![ConstraintCode {
            stage: 2,
            boundary: BoundaryKind::EveryRow,
            offset_min: None,
            offset_max: None,
            line: "constraint-a".to_owned(),
            intermediate: false,
            temporary_count: 0,
            operations: Vec::new(),
        }],
    }
}

pub fn sample_metadata_bundle_verifier_info() -> VerifierInfo {
    VerifierInfo {
        quotient: VerifierCode {
            expression_id: None,
            stage: None,
            line: String::new(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::number(1, 1)],
            }],
        },
        query: VerifierCode {
            expression_id: Some(9),
            stage: Some(3),
            line: "query-expression".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::evaluation(0, 3)],
            }],
        },
    }
}

pub fn sample_metadata_bundle_global_info() -> GlobalInfo {
    GlobalInfo {
        name: "sample-program".to_owned(),
        air_groups: vec!["group-a".to_owned()],
        airs: vec![vec![GlobalAir {
            name: "unit-a".to_owned(),
            num_rows: 1024,
            has_compressor: false,
        }]],
        curve: CurveKind::None,
        lattice_size: Some(368),
        aggregation_types: vec![Vec::<AggregationType>::new()],
        n_publics: 1,
        num_challenges: vec![1, 2],
        num_proof_values: vec![1, 1],
        proof_values_map: vec![
            NamedStageValue {
                name: "proof-a".to_owned(),
                stage: 1,
                id: None,
                lengths: Vec::new(),
            },
            NamedStageValue {
                name: "proof-b".to_owned(),
                stage: 2,
                id: None,
                lengths: Vec::new(),
            },
        ],
        publics_map: vec![PublicValue {
            name: "public-a".to_owned(),
            stage: 1,
            lengths: Vec::new(),
        }],
        transcript_arity: 4,
    }
}

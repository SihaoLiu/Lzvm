#![allow(dead_code)]

use std::collections::BTreeMap;

use lzvm_artifacts::expression_info::{
    CodeDestination, CodeOperand, CodeOperation, ExpressionCode, ExpressionInfo, HintFieldInfo,
    HintInfo, HintPayload, HintValueInfo, OperationKind,
};
use lzvm_artifacts::global_info::{
    AggregationType, CurveKind, GlobalAir, GlobalInfo, NamedStageValue, PublicValue,
};
use lzvm_artifacts::setup_info::{
    Boundary, CommitmentColumn, ConstantColumn, EvaluationMapEntry, FriStep, StageValue,
    StarkStruct, UnitSetupInfo,
};
use lzvm_artifacts::verifier_info::{
    VerifierCode, VerifierDestination, VerifierInfo, VerifierOperand, VerifierOperation,
    VerifierOperationKind,
};

pub fn sample_global_info() -> GlobalInfo {
    sample_global_info_with_options(false, false)
}

pub fn sample_global_info_with_proof_value() -> GlobalInfo {
    sample_global_info_with_options(true, false)
}

pub fn sample_global_info_with_group_value() -> GlobalInfo {
    sample_global_info_with_options(false, true)
}

pub fn sample_global_info_with_proof_group_value() -> GlobalInfo {
    sample_global_info_with_options(true, true)
}

fn sample_global_info_with_options(proof_value: bool, group_value: bool) -> GlobalInfo {
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
        aggregation_types: if group_value {
            vec![vec![AggregationType {
                aggregation_type: 0,
            }]]
        } else {
            vec![Vec::new()]
        },
        n_publics: 1,
        num_challenges: vec![1],
        num_proof_values: if proof_value { vec![0, 1] } else { Vec::new() },
        proof_values_map: if proof_value {
            vec![NamedStageValue {
                name: "proof-a".to_owned(),
                stage: 2,
                id: None,
                lengths: Vec::new(),
            }]
        } else {
            Vec::new()
        },
        publics_map: vec![PublicValue {
            name: "block_number".to_owned(),
            stage: 1,
            lengths: Vec::new(),
        }],
        transcript_arity: 4,
    }
}

pub fn sample_setup_info() -> UnitSetupInfo {
    sample_setup_info_with_unit_values(false)
}

pub fn sample_setup_info_with_unit_value() -> UnitSetupInfo {
    sample_setup_info_with_unit_values(true)
}

pub fn sample_setup_info_with_query_two() -> UnitSetupInfo {
    let mut setup = sample_setup_info();
    setup.stark.n_queries = 2;
    setup.stark.merkle_tree_arity = 2;
    setup.stark.transcript_arity = Some(2);
    setup
}

pub fn sample_setup_info_with_wide_fixed() -> UnitSetupInfo {
    let mut setup = sample_setup_info_with_query_two();
    setup.stark.n_bits = 2;
    setup.stark.n_bits_ext = 3;
    setup.stark.steps = vec![FriStep { n_bits: 3 }, FriStep { n_bits: 1 }];
    setup.stark.merkle_tree_arity = 4;
    setup.stark.transcript_arity = Some(4);
    setup
}

pub fn sample_verification_key_setup_info() -> UnitSetupInfo {
    let mut setup = sample_setup_info();
    setup.n_stages = 2;
    setup.n_constants = 1;
    setup.constant_columns = vec![ConstantColumn {
        name: "main.left".to_owned(),
        stage: 0,
        dimension: 1,
        pols_map_id: 0,
        stage_id: 0,
        lengths: Vec::new(),
    }];
    setup.section_widths = BTreeMap::from([
        ("const".to_owned(), 1),
        ("cm1".to_owned(), 1),
        ("cm2".to_owned(), 1),
        ("cm3".to_owned(), 1),
    ]);
    setup.q_degree = 7;
    setup.stark.n_queries = 1;
    setup.stark.last_level_verification = 1;
    setup.stark.pow_bits = 1;
    setup.stark.merkle_tree_arity = 2;
    setup.stark.transcript_arity = Some(2);
    setup
}

fn sample_setup_info_with_unit_values(unit_values: bool) -> UnitSetupInfo {
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
        challenge_count: 3,
        eval_count: 2,
        evaluation_map: vec![EvaluationMapEntry::default(); 2],
        boundaries: Vec::<Boundary>::new(),
        commitment_columns: Vec::<CommitmentColumn>::new(),
        unit_value_map: if unit_values {
            vec![
                StageValue {
                    name: "unit.alpha".to_owned(),
                    stage: 1,
                    lengths: Vec::new(),
                },
                StageValue {
                    name: "unit.beta".to_owned(),
                    stage: 2,
                    lengths: Vec::new(),
                },
            ]
        } else {
            Vec::new()
        },
        group_value_map: Vec::<StageValue>::new(),
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

pub fn sample_expression_info() -> ExpressionInfo {
    ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![sample_expression_code()],
        constraints: Vec::new(),
    }
}

pub fn sample_expression_info_with_hint() -> ExpressionInfo {
    ExpressionInfo {
        hints: vec![HintInfo {
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
        }],
        expressions: vec![sample_expression_code()],
        constraints: Vec::new(),
    }
}

pub fn sample_fri_quotient_expression_info() -> ExpressionInfo {
    ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![ExpressionCode {
            expression_id: 7,
            stage: 2,
            line: "constant quotient expression".to_owned(),
            temporary_count: 1,
            destination: None,
            operations: vec![CodeOperation {
                op: OperationKind::Add,
                destination: CodeDestination::temporary(0, 3),
                sources: vec![CodeOperand::number(10, 3), CodeOperand::number(0, 3)],
            }],
        }],
        constraints: Vec::new(),
    }
}

fn sample_expression_code() -> ExpressionCode {
    ExpressionCode {
        expression_id: 7,
        stage: 2,
        line: "query-expression".to_owned(),
        temporary_count: 1,
        destination: None,
        operations: vec![CodeOperation {
            op: OperationKind::Add,
            destination: CodeDestination::temporary(0, 1),
            sources: vec![CodeOperand::constant(0, 1), CodeOperand::number(0, 1)],
        }],
    }
}

pub fn sample_verifier_info() -> VerifierInfo {
    sample_verifier_info_with_query_source(VerifierOperand::evaluation(0, 3), None)
}

pub fn sample_verifier_info_with_proof_value() -> VerifierInfo {
    sample_verifier_info_with_query_source(VerifierOperand::proof_value(0, 3), None)
}

pub fn sample_fri_quotient_verifier_info() -> VerifierInfo {
    sample_verifier_info_with_query_source(
        VerifierOperand::number(10, 1),
        Some("quotient-expression"),
    )
}

fn sample_verifier_info_with_query_source(
    query_source: VerifierOperand,
    quotient_line: Option<&str>,
) -> VerifierInfo {
    VerifierInfo {
        quotient: VerifierCode {
            expression_id: quotient_line.map(|_| 7),
            stage: quotient_line.map(|_| 2),
            line: quotient_line.unwrap_or_default().to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::number(quotient_line.map_or(1, |_| 10), 1)],
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
                sources: vec![query_source],
            }],
        },
    }
}

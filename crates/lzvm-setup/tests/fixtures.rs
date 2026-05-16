#![allow(dead_code)]

use std::collections::BTreeMap;

use lzvm_artifacts::expression_info::{
    ExpressionCode, ExpressionInfo, HintFieldInfo, HintInfo, HintPayload, HintValueInfo,
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
        proof_values_map: Vec::<NamedStageValue>::new(),
        publics_map: Vec::<PublicValue>::new(),
        transcript_arity: 4,
    }
}

pub fn sample_expression_info() -> ExpressionInfo {
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

pub fn sample_verifier_info() -> VerifierInfo {
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

pub fn sample_base_setup_info() -> UnitSetupInfo {
    let mut setup = sample_two_column_setup_info(1, 2, 1, 4);
    setup.n_publics = Some(0);
    setup.n_constraints = Some(0);
    setup
}

pub fn sample_two_column_setup_info(
    n_bits: u32,
    n_bits_ext: u32,
    n_queries: u32,
    arity: u32,
) -> UnitSetupInfo {
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
        evaluation_map: Vec::<EvaluationMapEntry>::new(),
        boundaries: Vec::<Boundary>::new(),
        commitment_columns: Vec::<CommitmentColumn>::new(),
        unit_value_map: Vec::<StageValue>::new(),
        group_value_map: Vec::<StageValue>::new(),
        stark: StarkStruct {
            n_bits,
            n_bits_ext,
            n_queries,
            steps: vec![FriStep { n_bits: n_bits_ext }, FriStep { n_bits: 1 }],
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 0,
            merkle_tree_arity: arity,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(arity),
            merkle_tree_custom: Some(true),
        },
    }
}

pub fn sample_constant_tree_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 2,
        n_constants: 1,
        constant_columns: Vec::new(),
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

#[cfg(feature = "cuda")]
pub fn sample_wide_setup_info(arity: u32) -> UnitSetupInfo {
    let mut setup = sample_two_column_setup_info(1, 2, 2, arity);
    setup.n_constants = 5;
    setup.section_widths.insert("const".to_owned(), 5);
    setup.constant_columns = (0_u32..5)
        .map(|index| ConstantColumn {
            name: format!("main.c{index}"),
            stage: 0,
            dimension: 1,
            pols_map_id: index,
            stage_id: index,
            lengths: Vec::new(),
        })
        .collect();
    setup
}

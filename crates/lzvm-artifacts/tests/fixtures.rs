#![allow(dead_code)]

use std::collections::BTreeMap;

use lzvm_artifacts::setup_info::{
    Boundary, ConstantColumn, EvaluationMapEntry, FriStep, StarkStruct, UnitSetupInfo,
};

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

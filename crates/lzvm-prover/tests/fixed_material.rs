#![cfg(feature = "cuda")]

use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use lzvm_artifacts::fixed::{write_raw_fixed_columns_file, FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::{
    ConstantColumn, EvaluationMapEntry, FriStep, StarkStruct, UnitSetupInfo,
};
use lzvm_prover::load_fixed_columns_material;

#[test]
fn loads_raw_fixed_columns_material_and_stages_device_bytes() {
    let dir = temp_dir("fixed-material");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");

    let setup = sample_setup();
    let columns = sample_columns();
    let path = dir.join("unit.const");
    write_raw_fixed_columns_file(&path, &columns, &setup).expect("fixed columns should write");

    let material = load_fixed_columns_material(&path, &setup, "group-a", "unit-a")
        .expect("fixed columns material should load");

    assert_eq!(material.fixed_columns, columns);
    assert_eq!(
        material.raw_bytes,
        fs::read(&path).expect("fixed file should read")
    );

    let device_buffer = material
        .device_buffer
        .expect("device buffer should be staged");
    assert_eq!(device_buffer.len(), material.raw_bytes.len());
    assert_eq!(
        device_buffer
            .to_vec()
            .expect("device bytes should round-trip"),
        material.raw_bytes
    );
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("lzvm-prover-{name}-{stamp}"))
}

fn sample_setup() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 0,
        n_constants: 2,
        constant_columns: vec![
            ConstantColumn {
                name: "const_0".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "const_1".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 1,
                stage_id: 1,
                lengths: Vec::new(),
            },
        ],
        n_publics: None,
        n_constraints: None,
        q_degree: 0,
        opening_points: Vec::new(),
        section_widths: BTreeMap::new(),
        challenge_count: 0,
        eval_count: 0,
        evaluation_map: vec![EvaluationMapEntry::default()],
        boundaries: Vec::new(),
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 2,
            n_bits_ext: 2,
            n_queries: 0,
            steps: vec![FriStep { n_bits: 2 }],
            hash_commits: false,
            last_level_verification: 0,
            pow_bits: 0,
            merkle_tree_arity: 2,
            verification_hash_type: None,
            transcript_arity: None,
            merkle_tree_custom: None,
        },
    }
}

fn sample_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 4,
        columns: vec![
            FixedColumn {
                name: "const_0".to_owned(),
                dimensions: vec![1],
                values: vec![11, 12, 13, 14],
            },
            FixedColumn {
                name: "const_1".to_owned(),
                dimensions: vec![1],
                values: vec![21, 22, 23, 24],
            },
        ],
    }
}

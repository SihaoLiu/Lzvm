use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use lzvm_artifacts::fixed::{
    encode_fixed_columns, write_raw_fixed_columns_file, FixedColumn, FixedColumnError, FixedColumns,
};
use lzvm_artifacts::setup_info::{
    ConstantColumn, EvaluationMapEntry, FriStep, StarkStruct, UnitSetupInfo,
};
use lzvm_prover::{
    load_fixed_columns_material, load_fixed_columns_material_with_digest, FixedColumnsMaterialError,
};
use sha2::{Digest, Sha256};

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
        material.row_major_values,
        vec![
            lzvm_field::Felt::from_u64(11),
            lzvm_field::Felt::from_u64(21),
            lzvm_field::Felt::from_u64(12),
            lzvm_field::Felt::from_u64(22),
            lzvm_field::Felt::from_u64(13),
            lzvm_field::Felt::from_u64(23),
            lzvm_field::Felt::from_u64(14),
            lzvm_field::Felt::from_u64(24),
        ]
    );
    assert_eq!(
        material.raw_bytes,
        fs::read(&path).expect("fixed file should read")
    );

    #[cfg(feature = "cuda")]
    {
        assert!(material.device_buffer_is_row_major);
        let device_buffer = material
            .row_major_device_buffer()
            .expect("row-major device buffer should be available");
        assert_eq!(device_buffer.len(), material.raw_bytes.len());
        assert_eq!(
            device_buffer
                .to_vec()
                .expect("device bytes should round-trip"),
            material.raw_bytes
        );
    }
}

#[test]
fn raw_fixed_columns_preserve_setup_column_order_when_not_physical_order() {
    let dir = temp_dir("fixed-material-unsorted");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");

    let setup = sample_setup_with_reversed_columns();
    let columns = FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 4,
        columns: vec![
            FixedColumn {
                name: "const_1".to_owned(),
                dimensions: vec![1],
                values: vec![21, 22, 23, 24],
            },
            FixedColumn {
                name: "const_0".to_owned(),
                dimensions: vec![1],
                values: vec![11, 12, 13, 14],
            },
        ],
    };
    let path = dir.join("unit.const");
    write_raw_fixed_columns_file(&path, &columns, &setup).expect("fixed columns should write");

    let material = load_fixed_columns_material(&path, &setup, "group-a", "unit-a")
        .expect("fixed columns material should load");

    assert_eq!(material.fixed_columns, columns);
    assert_eq!(
        material.row_major_values,
        vec![
            lzvm_field::Felt::from_u64(21),
            lzvm_field::Felt::from_u64(11),
            lzvm_field::Felt::from_u64(22),
            lzvm_field::Felt::from_u64(12),
            lzvm_field::Felt::from_u64(23),
            lzvm_field::Felt::from_u64(13),
            lzvm_field::Felt::from_u64(24),
            lzvm_field::Felt::from_u64(14),
        ]
    );
    #[cfg(feature = "cuda")]
    {
        assert!(!material.device_buffer_is_row_major);
        assert!(material.row_major_device_buffer().is_none());
    }
}

#[test]
fn sectioned_fixed_columns_preserve_file_column_order() {
    let dir = temp_dir("fixed-material-sectioned");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");

    let setup = sample_setup();
    let columns = FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 4,
        columns: vec![
            FixedColumn {
                name: "const_1".to_owned(),
                dimensions: vec![1],
                values: vec![21, 22, 23, 24],
            },
            FixedColumn {
                name: "const_0".to_owned(),
                dimensions: vec![1],
                values: vec![11, 12, 13, 14],
            },
        ],
    };
    let path = dir.join("unit.const");
    fs::write(
        &path,
        encode_fixed_columns(&columns).expect("sectioned fixed columns should encode"),
    )
    .expect("sectioned fixed columns should write");

    let material = load_fixed_columns_material(&path, &setup, "group-a", "unit-a")
        .expect("fixed columns material should load");

    assert_eq!(material.fixed_columns, columns);
    assert_eq!(
        material.row_major_values,
        vec![
            lzvm_field::Felt::from_u64(21),
            lzvm_field::Felt::from_u64(11),
            lzvm_field::Felt::from_u64(22),
            lzvm_field::Felt::from_u64(12),
            lzvm_field::Felt::from_u64(23),
            lzvm_field::Felt::from_u64(13),
            lzvm_field::Felt::from_u64(24),
            lzvm_field::Felt::from_u64(14),
        ]
    );
    #[cfg(feature = "cuda")]
    {
        assert!(!material.device_buffer_is_row_major);
        assert!(material.row_major_device_buffer().is_none());
    }
}

#[test]
fn raw_fixed_columns_reject_non_canonical_word_before_fast_path() {
    let dir = temp_dir("fixed-material-non-canonical");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");

    let setup = sample_setup();
    let columns = sample_columns();
    let path = dir.join("unit.const");
    write_raw_fixed_columns_file(&path, &columns, &setup).expect("fixed columns should write");
    let mut bytes = fs::read(&path).expect("fixed file should read");
    bytes[..8].copy_from_slice(&lzvm_field::MODULUS.to_le_bytes());
    fs::write(&path, bytes).expect("fixed columns should be mutated");

    let error = load_fixed_columns_material(&path, &setup, "group-a", "unit-a")
        .expect_err("non-canonical raw word should reject material");

    let FixedColumnsMaterialError::Read { source, .. } = error else {
        panic!("expected fixed-column read error");
    };
    assert!(matches!(source, FixedColumnError::ValueNonCanonical { .. }));
}

#[test]
fn rejects_fixed_columns_material_digest_mismatch() {
    let dir = temp_dir("fixed-material-digest-mismatch");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");

    let setup = sample_setup();
    let columns = sample_columns();
    let path = dir.join("unit.const");
    write_raw_fixed_columns_file(&path, &columns, &setup).expect("fixed columns should write");
    let expected_digest: [u8; 32] =
        Sha256::digest(fs::read(&path).expect("fixed file should read")).into();
    let mut mutated = fs::read(&path).expect("fixed file should read");
    mutated[0] ^= 1;
    fs::write(&path, mutated).expect("fixed columns should be mutated");

    let error = load_fixed_columns_material_with_digest(
        &path,
        &setup,
        "group-a",
        "unit-a",
        expected_digest,
    )
    .expect_err("fixed digest mismatch should reject material");

    assert!(matches!(
        error,
        FixedColumnsMaterialError::DigestMismatch { .. }
    ));
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-prover-{name}-{stamp}"))
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

fn sample_setup_with_reversed_columns() -> UnitSetupInfo {
    UnitSetupInfo {
        constant_columns: vec![
            ConstantColumn {
                name: "const_1".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 1,
                stage_id: 1,
                lengths: Vec::new(),
            },
            ConstantColumn {
                name: "const_0".to_owned(),
                stage: 0,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                lengths: Vec::new(),
            },
        ],
        ..sample_setup()
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

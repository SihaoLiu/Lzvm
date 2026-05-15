use lzvm_artifacts::fixed::{FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use lzvm_field::{Felt, FieldError, MODULUS, SHIFT};
use lzvm_setup::{extend_fixed_columns_for_constant_tree, SetupError};

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 1,
        "nConstants": 2,
        "qDeg": 3,
        "openingPoints": [0],
        "mapSectionsN": {
            "const": 2,
            "cm1": 1,
            "cm2": 1
        },
        "constPolsMap": [
            {"stage": 0, "name": "main.left", "dim": 1, "polsMapId": 0, "stageId": 0},
            {"stage": 0, "name": "main.right", "dim": 1, "polsMapId": 1, "stageId": 1}
        ],
        "challengesMap": [],
        "evMap": [],
        "boundaries": [],
        "starkStruct": {
            "nBits": 1,
            "nBitsExt": 2,
            "nQueries": 2,
            "steps": [
                {"nBits": 2},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 0,
            "merkleTreeArity": 2,
            "verificationHashType": "GL",
            "transcriptArity": 2,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 2,
        columns: vec![
            FixedColumn {
                name: "main.left".to_owned(),
                dimensions: vec![1],
                values: vec![5, 1],
            },
            FixedColumn {
                name: "main.right".to_owned(),
                dimensions: vec![1],
                values: vec![9, 9],
            },
        ],
    }
}

fn words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("chunk length checked")))
        .collect()
}

#[test]
fn extends_fixed_columns_into_row_major_constant_tree_leaves() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let root = Felt::root_of_unity(2).expect("root should exist");
    let two = Felt::from_u64(2);
    let three = Felt::from_u64(3);
    let shifted_root = SHIFT * root;
    let expected_left = [
        three + two * SHIFT,
        three + two * shifted_root,
        three - two * SHIFT,
        three - two * shifted_root,
    ];

    let leaves = extend_fixed_columns_for_constant_tree(&sample_columns(), &setup)
        .expect("extension should succeed");

    assert_eq!(leaves.len(), 64);
    assert_eq!(
        words(&leaves),
        vec![
            expected_left[0].to_u64(),
            9,
            expected_left[1].to_u64(),
            9,
            expected_left[2].to_u64(),
            9,
            expected_left[3].to_u64(),
            9,
        ]
    );
}

#[test]
fn rejects_non_canonical_fixed_values_before_extension() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let mut columns = sample_columns();
    columns.columns[0].values[0] = MODULUS;

    assert!(matches!(
        extend_fixed_columns_for_constant_tree(&columns, &setup),
        Err(SetupError::Field(FieldError::NonCanonical { value })) if value == MODULUS
    ));
}

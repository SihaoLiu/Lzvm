use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::fixed::{FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_field::{poseidon2_hash_8, Felt};
use lzvm_setup::{
    build_constant_tree_from_fixed_columns, extend_fixed_columns_for_constant_tree,
    write_constant_tree_from_fixed_columns,
};

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

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-setup-native-tree-{}-{name}",
        std::process::id()
    ))
}

fn staging_entries(parent: &Path) -> Vec<PathBuf> {
    fs::read_dir(parent)
        .expect("directory should be readable")
        .map(|entry| entry.expect("directory entry should exist").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".staging."))
        })
        .collect()
}

fn words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("chunk length checked")))
        .collect()
}

fn encode_digest_words(out: &mut Vec<u64>, digest: [Felt; 4]) {
    out.extend(digest.into_iter().map(|value| value.to_u64()));
}

fn parent_hash(left: [Felt; 4], right: [Felt; 4]) -> [Felt; 4] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

fn manual_expected_tree_words(leaves: &[u8]) -> Vec<u64> {
    let leaf_words = words(leaves);
    let rows = leaf_words
        .chunks_exact(2)
        .map(|row| {
            [
                Felt::from_u64(row[0]),
                Felt::from_u64(row[1]),
                Felt::ZERO,
                Felt::ZERO,
            ]
        })
        .collect::<Vec<_>>();
    let parent_left = parent_hash(rows[0], rows[1]);
    let parent_right = parent_hash(rows[2], rows[3]);
    let root = parent_hash(parent_left, parent_right);

    let mut expected = leaf_words;
    for row in rows {
        encode_digest_words(&mut expected, row);
    }
    encode_digest_words(&mut expected, parent_left);
    encode_digest_words(&mut expected, parent_right);
    encode_digest_words(&mut expected, root);
    expected
}

#[test]
fn builds_native_constant_tree_from_fixed_columns() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let leaves = extend_fixed_columns_for_constant_tree(&sample_columns(), &setup)
        .expect("leaves should extend");

    let tree = build_constant_tree_from_fixed_columns(&sample_columns(), &setup)
        .expect("tree should build");
    let parsed = lzvm_artifacts::constant_tree::parse_constant_tree_bytes(tree.clone(), &setup)
        .expect("tree should parse");

    assert_eq!(tree.len(), 288);
    assert_eq!(words(&tree), manual_expected_tree_words(&leaves));
    assert_eq!(
        parsed.root().expect("root should extract"),
        VerificationKeyRoot::FieldElements(words(&tree[tree.len() - 32..]))
    );
}

#[test]
fn writes_native_constant_tree_through_validated_staging() {
    let dir = temp_dir("write-tree");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.consttree");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");

    let report = write_constant_tree_from_fixed_columns(&path, &sample_columns(), &setup)
        .expect("tree write should succeed");
    let bytes = fs::read(&path).expect("tree output should exist");
    let tree = read_constant_tree_file(&path, &setup).expect("tree should read");
    let staging = staging_entries(path.parent().expect("path should have a parent"));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(report.path, path);
    assert_eq!(report.bytes_written, 288);
    assert_eq!(report.root, tree.root().expect("root should extract"));
    assert_eq!(
        words(&bytes),
        manual_expected_tree_words(
            &extend_fixed_columns_for_constant_tree(&sample_columns(), &setup)
                .expect("leaves should extend")
        )
    );
    assert!(staging.is_empty());
}

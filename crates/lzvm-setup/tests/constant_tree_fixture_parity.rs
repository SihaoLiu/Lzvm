use std::fs;
use std::path::{Path, PathBuf};

mod fixtures;

use fixtures::sample_two_column_setup_info;
use lzvm_artifacts::constant_tree::parse_constant_tree_bytes;
use lzvm_artifacts::fixed::{
    encode_raw_fixed_columns, read_raw_fixed_columns_file, FixedColumn, FixedColumns,
};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, read_unit_setup_info_binary_file, UnitSetupInfo,
};
use lzvm_artifacts::verification_key::{
    encode_verification_key_binary, read_verification_key_binary_file, VerificationKeyRoot,
};
use lzvm_field::{poseidon2_hash_8, Felt};
use lzvm_setup::{build_constant_tree_from_fixed_columns, extend_fixed_columns_for_constant_tree};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-setup-constant-tree-fixture-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
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

fn parent_hash(left: [Felt; 4], right: [Felt; 4]) -> [Felt; 4] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

fn expected_root(setup: &UnitSetupInfo, columns: &FixedColumns) -> VerificationKeyRoot {
    let leaves = extend_fixed_columns_for_constant_tree(columns, setup)
        .expect("fixed columns should extend");
    let rows = words(&leaves)
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

    VerificationKeyRoot::FieldElements(root.into_iter().map(|value| value.to_u64()).collect())
}

#[test]
fn native_constant_tree_root_matches_repo_owned_fixture() {
    let dir = temp_dir("root-match");
    let _ = fs::remove_dir_all(&dir);
    let setup_path = dir.join("unit.setup.bin");
    let raw_fixed_path = dir.join("unit.fixed");
    let root_path = dir.join("unit.verkey.bin");
    let setup = sample_two_column_setup_info(1, 2, 2, 2);
    let columns = sample_columns();
    let expected = expected_root(&setup, &columns);

    write_bytes(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_bytes(
        &raw_fixed_path,
        encode_raw_fixed_columns(&columns, &setup).expect("raw fixed should encode"),
    );
    write_bytes(
        &root_path,
        encode_verification_key_binary(&expected).expect("root should encode"),
    );

    let setup = read_unit_setup_info_binary_file(&setup_path).expect("setup fixture should parse");
    let columns = read_raw_fixed_columns_file(&raw_fixed_path, &setup, "group-a", "unit-a")
        .expect("raw fixed fixture should parse");
    let tree = build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let generated = parse_constant_tree_bytes(tree, &setup)
        .expect("generated tree should parse")
        .root()
        .expect("generated root should extract");
    let expected =
        read_verification_key_binary_file(&root_path).expect("root fixture should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(generated, expected);
}

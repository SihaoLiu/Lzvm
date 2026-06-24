use std::fs;
use std::path::{Path, PathBuf};

mod fixtures;

use fixtures::sample_two_column_setup_info;
#[cfg(feature = "cuda")]
use fixtures::sample_wide_setup_info;
use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::fixed::{FixedColumn, FixedColumns};
use lzvm_artifacts::setup_info::ConstantColumn;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_field::{poseidon2_hash_16, poseidon2_hash_8, Felt};
use lzvm_setup::{
    build_constant_tree_from_fixed_columns, extend_fixed_columns_for_constant_tree,
    write_constant_tree_from_fixed_columns,
};
#[cfg(feature = "cuda")]
use lzvm_setup::{
    build_constant_tree_from_leaves, build_constant_tree_from_leaves_with_backend,
    FixedExtensionBackend,
};

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

fn sample_wide_columns(column_count: u64) -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 2,
        columns: (0_u64..column_count)
            .map(|index| FixedColumn {
                name: format!("main.c{index}"),
                dimensions: vec![1],
                values: vec![index + 3, index + 17],
            })
            .collect(),
    }
}

fn wide_setup_info(column_count: u32, arity: u32) -> lzvm_artifacts::setup_info::UnitSetupInfo {
    let mut setup = sample_two_column_setup_info(1, 2, 2, arity);
    setup.n_constants = column_count;
    setup
        .section_widths
        .insert("const".to_owned(), column_count);
    setup.constant_columns = (0_u32..column_count)
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

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
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

fn parent_hash_4(children: [[Felt; 4]; 4]) -> [Felt; 4] {
    let state = poseidon2_hash_16([
        children[0][0],
        children[0][1],
        children[0][2],
        children[0][3],
        children[1][0],
        children[1][1],
        children[1][2],
        children[1][3],
        children[2][0],
        children[2][1],
        children[2][2],
        children[2][3],
        children[3][0],
        children[3][1],
        children[3][2],
        children[3][3],
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

fn manual_expected_tree_words_arity4(leaves: &[u8]) -> Vec<u64> {
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
    let root = parent_hash_4([rows[0], rows[1], rows[2], rows[3]]);

    let mut expected = leaf_words;
    for row in rows {
        encode_digest_words(&mut expected, row);
    }
    encode_digest_words(&mut expected, root);
    expected
}

fn linear_digest_arity2(values: &[u64]) -> [Felt; 4] {
    if values.len() <= 4 {
        let mut digest = [Felt::ZERO; 4];
        for (slot, value) in digest.iter_mut().zip(values.iter().copied()) {
            *slot = Felt::from_u64(value);
        }
        return digest;
    }

    let mut state = [Felt::ZERO; 8];
    let mut offset = 0;
    while offset < values.len() {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[4..].copy_from_slice(&capacity);
        state[..4].fill(Felt::ZERO);

        let chunk_len = (values.len() - offset).min(4);
        for index in 0..chunk_len {
            state[index] = Felt::from_u64(values[offset + index]);
        }
        state = poseidon2_hash_8(state);
        offset += chunk_len;
    }

    [state[0], state[1], state[2], state[3]]
}

fn linear_digest_arity4(values: &[u64]) -> [Felt; 4] {
    if values.len() <= 4 {
        let mut digest = [Felt::ZERO; 4];
        for (slot, value) in digest.iter_mut().zip(values.iter().copied()) {
            *slot = Felt::from_u64(value);
        }
        return digest;
    }

    let mut state = [Felt::ZERO; 16];
    let mut offset = 0;
    while offset < values.len() {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[12..].copy_from_slice(&capacity);
        state[..12].fill(Felt::ZERO);

        let chunk_len = (values.len() - offset).min(12);
        for index in 0..chunk_len {
            state[index] = Felt::from_u64(values[offset + index]);
        }
        state = poseidon2_hash_16(state);
        offset += chunk_len;
    }

    [state[0], state[1], state[2], state[3]]
}

fn manual_expected_wide_tree_words(leaves: &[u8], column_count: usize, arity: usize) -> Vec<u64> {
    let leaf_words = words(leaves);
    let rows = leaf_words
        .chunks_exact(column_count)
        .map(|row| match arity {
            2 => linear_digest_arity2(row),
            4 => linear_digest_arity4(row),
            _ => panic!("unsupported arity"),
        })
        .collect::<Vec<_>>();

    let mut expected = leaf_words;
    for row in &rows {
        encode_digest_words(&mut expected, *row);
    }

    match arity {
        2 => {
            let parent_left = parent_hash(rows[0], rows[1]);
            let parent_right = parent_hash(rows[2], rows[3]);
            let root = parent_hash(parent_left, parent_right);
            encode_digest_words(&mut expected, parent_left);
            encode_digest_words(&mut expected, parent_right);
            encode_digest_words(&mut expected, root);
        }
        4 => {
            let root = parent_hash_4([rows[0], rows[1], rows[2], rows[3]]);
            encode_digest_words(&mut expected, root);
        }
        _ => panic!("unsupported arity"),
    }

    expected
}

#[test]
fn builds_native_constant_tree_from_fixed_columns() {
    let setup = sample_two_column_setup_info(1, 2, 2, 2);
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
    let setup = sample_two_column_setup_info(1, 2, 2, 2);

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

#[test]
fn builds_native_arity_4_constant_tree_from_fixed_columns() {
    let setup = sample_two_column_setup_info(1, 2, 2, 4);
    let leaves = extend_fixed_columns_for_constant_tree(&sample_columns(), &setup)
        .expect("leaves should extend");

    let tree = build_constant_tree_from_fixed_columns(&sample_columns(), &setup)
        .expect("tree should build");
    let parsed = lzvm_artifacts::constant_tree::parse_constant_tree_bytes(tree.clone(), &setup)
        .expect("tree should parse");

    assert_eq!(tree.len(), 224);
    assert_eq!(words(&tree), manual_expected_tree_words_arity4(&leaves));
    assert_eq!(
        parsed.root().expect("root should extract"),
        VerificationKeyRoot::FieldElements(words(&tree[tree.len() - 32..]))
    );
}

#[test]
fn builds_wide_native_constant_tree_from_fixed_columns() {
    let column_count = 5;
    let setup = wide_setup_info(column_count, 2);
    let columns = sample_wide_columns(u64::from(column_count));
    let leaves =
        extend_fixed_columns_for_constant_tree(&columns, &setup).expect("leaves should extend");

    let tree = build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");

    assert_eq!(
        words(&tree),
        manual_expected_wide_tree_words(&leaves, column_count as usize, 2)
    );
}

#[test]
fn builds_wide_native_arity_4_constant_tree_from_fixed_columns() {
    let column_count = 13;
    let setup = wide_setup_info(column_count, 4);
    let columns = sample_wide_columns(u64::from(column_count));
    let leaves =
        extend_fixed_columns_for_constant_tree(&columns, &setup).expect("leaves should extend");

    let tree = build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");

    assert_eq!(
        words(&tree),
        manual_expected_wide_tree_words(&leaves, column_count as usize, 4)
    );
}

#[test]
#[cfg(feature = "cuda")]
fn builds_native_constant_tree_from_leaves_with_cuda_backend() {
    let setup = sample_two_column_setup_info(1, 2, 2, 2);
    let leaves = extend_fixed_columns_for_constant_tree(&sample_columns(), &setup)
        .expect("leaves should extend");
    let expected = build_constant_tree_from_leaves(&leaves, &setup).expect("cpu tree should build");

    let actual =
        build_constant_tree_from_leaves_with_backend(&leaves, &setup, FixedExtensionBackend::Cuda)
            .expect("cuda tree should build");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn builds_native_arity_4_constant_tree_from_leaves_with_cuda_backend() {
    let setup = sample_two_column_setup_info(1, 2, 2, 4);
    let leaves = extend_fixed_columns_for_constant_tree(&sample_columns(), &setup)
        .expect("leaves should extend");
    let expected = build_constant_tree_from_leaves(&leaves, &setup).expect("cpu tree should build");

    let actual =
        build_constant_tree_from_leaves_with_backend(&leaves, &setup, FixedExtensionBackend::Cuda)
            .expect("cuda tree should build");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn builds_wide_native_constant_tree_from_leaves_with_cuda_backend() {
    let setup = sample_wide_setup_info(2);
    let leaves = extend_fixed_columns_for_constant_tree(&sample_wide_columns(5), &setup)
        .expect("leaves should extend");
    let expected = build_constant_tree_from_leaves(&leaves, &setup).expect("cpu tree should build");

    let actual =
        build_constant_tree_from_leaves_with_backend(&leaves, &setup, FixedExtensionBackend::Cuda)
            .expect("cuda tree should build");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn builds_wide_native_arity_4_constant_tree_from_leaves_with_cuda_backend() {
    let setup = sample_wide_setup_info(4);
    let leaves = extend_fixed_columns_for_constant_tree(&sample_wide_columns(5), &setup)
        .expect("leaves should extend");
    let expected = build_constant_tree_from_leaves(&leaves, &setup).expect("cpu tree should build");

    let actual =
        build_constant_tree_from_leaves_with_backend(&leaves, &setup, FixedExtensionBackend::Cuda)
            .expect("cuda tree should build");

    assert_eq!(actual, expected);
}

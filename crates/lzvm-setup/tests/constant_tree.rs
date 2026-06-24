use std::fs;
use std::path::{Path, PathBuf};

mod fixtures;

use fixtures::sample_constant_tree_setup_info;
use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_setup::{write_base_constant_tree, SetupError};

fn sample_root() -> VerificationKeyRoot {
    VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])
}

fn sample_tree_bytes() -> Vec<u8> {
    let mut bytes = vec![7_u8; 256];
    for (index, value) in [1_u64, 2, 3, 4].iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-setup-tree-{}-{name}", std::process::id()))
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

#[test]
fn writes_base_constant_tree_through_validated_staging() {
    let dir = temp_dir("write-tree");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.consttree");
    let setup = sample_constant_tree_setup_info();
    let expected_root = sample_root();

    let report =
        write_base_constant_tree(&path, &sample_tree_bytes(), &setup, Some(&expected_root))
            .expect("write should succeed");
    let tree = read_constant_tree_file(&path, &setup).expect("tree should read");
    let staging = staging_entries(path.parent().expect("path should have a parent"));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(report.path, path);
    assert_eq!(report.bytes_written, 256);
    assert_eq!(report.root, expected_root);
    assert_eq!(tree.root().expect("root should extract"), expected_root);
    assert!(staging.is_empty());
}

#[test]
fn preserves_existing_constant_tree_when_root_validation_fails() {
    let dir = temp_dir("preserve-tree");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("base").join("unit-a.consttree");
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(&path, b"stable-output").expect("stable fixture should be written");
    let setup = sample_constant_tree_setup_info();
    let expected_root = VerificationKeyRoot::FieldElements(vec![9, 9, 9, 9]);

    let result =
        write_base_constant_tree(&path, &sample_tree_bytes(), &setup, Some(&expected_root));
    let stable = fs::read(&path).expect("stable output should still exist");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(SetupError::ConstantTreeRootMismatch { .. })
    ));
    assert_eq!(stable, b"stable-output");
}

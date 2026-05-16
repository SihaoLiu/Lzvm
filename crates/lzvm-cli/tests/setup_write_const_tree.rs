use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::setup_info::encode_unit_setup_info;
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
use lzvm_cli::run_cli;

mod fixtures;

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
    std::env::temp_dir().join(format!(
        "lzvm-cli-write-const-tree-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_constant_tree_from_binary_setup_and_root() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let tree_path = dir.join("unit.consttree.raw");
    let root_path = dir.join("unit.consttree.root");
    let out_path = dir.join("unit.consttree");
    let setup = fixtures::sample_verification_key_setup_info();
    let expected_root = sample_root();
    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(&tree_path, sample_tree_bytes()).expect("tree fixture should be written");
    fs::write(
        &root_path,
        encode_verification_key_binary(&expected_root).expect("root should encode"),
    )
    .expect("root fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-const-tree",
            setup_path.to_str().expect("setup path should be utf-8"),
            tree_path.to_str().expect("tree path should be utf-8"),
            root_path.to_str().expect("root path should be utf-8"),
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let tree = read_constant_tree_file(&out_path, &setup).expect("tree should read");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written=256\nroot=1,2,3,4\noutput={}\n",
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(tree.root().expect("root should extract"), expected_root);
}

#[test]
fn reports_usage_for_missing_constant_tree_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-const-tree",
            "unit.setup.bin",
            "unit.consttree.raw",
            "unit.consttree.root",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-const-tree <setup-info-bin> <tree-bin> <root-bin> <out-consttree>\n"
    );
}

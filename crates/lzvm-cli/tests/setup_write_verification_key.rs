use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::setup_info::{encode_unit_setup_info, parse_unit_setup_info_json};
use lzvm_artifacts::verification_key::{read_verification_key_binary_file, VerificationKeyRoot};
use lzvm_cli::run_cli;

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 2,
        "nConstants": 1,
        "nPublics": 0,
        "nConstraints": 0,
        "qDeg": 7,
        "openingPoints": [0],
        "mapSectionsN": {
            "const": 1,
            "cm1": 1,
            "cm2": 1,
            "cm3": 1
        },
        "challengesMap": [],
        "evMap": [],
        "boundaries": [],
        "starkStruct": {
            "nBits": 1,
            "nBitsExt": 2,
            "nQueries": 1,
            "steps": [
                {"nBits": 2},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 1,
            "powBits": 1,
            "merkleTreeArity": 2,
            "verificationHashType": "GL",
            "transcriptArity": 2,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_tree_bytes() -> Vec<u8> {
    let mut bytes = vec![7_u8; 256];
    for (index, value) in [1_u64, 2, 3, 4].iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn sample_root() -> VerificationKeyRoot {
    VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-write-verkey-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
}

#[test]
fn writes_verification_key_binary_from_constant_tree() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let setup_path = dir.join("setup.bin");
    let tree_path = dir.join("unit-a.consttree");
    let binary_path = dir.join("unit-a.verkey.bin");
    write_bytes(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_bytes(&tree_path, sample_tree_bytes());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-verkey-native",
            setup_path.to_str().expect("setup path should be utf-8"),
            tree_path.to_str().expect("tree path should be utf-8"),
            binary_path.to_str().expect("binary path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let binary_root =
        read_verification_key_binary_file(&binary_path).expect("binary root should read");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(binary_root, sample_root());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbinary_bytes=32\nroot=1,2,3,4\nbinary_output={}\n",
            binary_path.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_verification_key_outputs() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "write-verkey-native"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-verkey-native <setup-info-bin> <consttree> <out-verkey-bin>\n"
    );
}

use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{
    encode_public_values, public_values_digest, PublicValueEntry, PublicValues,
};
use lzvm_cli::run_cli;
use lzvm_prover::proof_preflight::validate_proof_public_values_from_files;

fn sample_hash(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-verify-preflight-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
}

fn sample_public_values() -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![
            PublicValueEntry {
                name: "block_number".to_owned(),
                elements: vec![12_345],
            },
            PublicValueEntry {
                name: "state_root_words".to_owned(),
                elements: vec![1, 2, 3, 4],
            },
        ],
    }
}

fn sample_proof(public_values: &PublicValues) -> ProofArtifact {
    ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: 100,
            data: vec![1, 2, 3, 4],
        }],
    }
}

fn write_fixture_pair(
    name: &str,
    proof: &ProofArtifact,
    values: &PublicValues,
) -> (PathBuf, PathBuf, PathBuf) {
    let dir = temp_dir(name);
    let _ = fs::remove_dir_all(&dir);
    let proof_path = dir.join("proof.bin");
    let public_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(proof).expect("proof should encode"),
    );
    write_bytes(
        &public_path,
        encode_public_values(values).expect("public values should encode"),
    );
    (dir, proof_path, public_path)
}

#[test]
fn verifies_proof_artifact_preflight() {
    let values = sample_public_values();
    let proof = sample_proof(&values);
    let (dir, proof_path, public_path) = write_fixture_pair("valid", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nsegments=1\npublic_values=2\n"
    );
    assert!(stderr.is_empty());

    let report = validate_proof_public_values_from_files(&proof_path, &public_path)
        .expect("file-based preflight should validate");
    assert_eq!(report.segment_count, 1);
    assert_eq!(report.public_value_count, 2);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn verifies_proof_artifact_preflight_with_binary_public_values() {
    let values = sample_public_values();
    let proof = sample_proof(&values);
    let dir = temp_dir("valid-bin");
    let _ = fs::remove_dir_all(&dir);
    let proof_path = dir.join("proof.bin");
    let public_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_path,
        encode_public_values(&values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nsegments=1\npublic_values=2\n"
    );
    assert!(stderr.is_empty());
}

#[test]
fn rejects_preflight_with_mismatched_setup_hashes() {
    let values = sample_public_values();
    let mut proof = sample_proof(&values);
    proof.setup_hash = sample_hash(0x55);
    proof.public_values_hash = public_values_digest(&values).expect("digest should compute");
    let (dir, proof_path, public_path) = write_fixture_pair("bad-setup", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify preflight failed: setup hash mismatch\n"
    );
}

#[test]
fn rejects_preflight_with_mismatched_public_values_hashes() {
    let values = sample_public_values();
    let mut proof = sample_proof(&values);
    proof.public_values_hash = sample_hash(0x99);
    let (dir, proof_path, public_path) = write_fixture_pair("bad-public", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify preflight failed: public-values hash mismatch\n"
    );
}

#[test]
fn reports_usage_for_missing_preflight_inputs() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["verify", "preflight"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm verify preflight <proof-bin> <public-values>\n"
    );
}

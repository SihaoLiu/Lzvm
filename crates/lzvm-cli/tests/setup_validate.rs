use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constraint_program::{
    encode_global_constraint_program, GlobalConstraintProgram,
};
use lzvm_artifacts::expression_program::{
    encode_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, key_directory_catalog_digest_hex, read_key_directory_catalog,
    read_key_directory_layout, KeyUnitPaths,
};
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{
    encode_public_values_json, public_values_digest, PublicValueEntry, PublicValues,
};
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
use lzvm_cli::run_cli;

fn sample_global_info_json() -> &'static str {
    r#"{
        "name": "sample-program",
        "air_groups": ["group-a"],
        "airs": [[{"name": "unit-a", "num_rows": 2}]],
        "curve": "None",
        "latticeSize": 368,
        "aggTypes": [[]],
        "nPublics": 0,
        "numChallenges": [1],
        "numProofValues": [],
        "publicsMap": [],
        "transcriptArity": 4
    }"#
}

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 1,
        "nConstants": 2,
        "nPublics": 0,
        "nConstraints": 0,
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
            "nQueries": 1,
            "steps": [
                {"nBits": 2},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 0,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_expression_info_json() -> &'static str {
    r#"{
        "hintsInfo": [],
        "expressionsCode": [
            {
                "expId": 7,
                "stage": 2,
                "line": "query-expression",
                "tmpUsed": 0,
                "code": []
            }
        ],
        "constraints": []
    }"#
}

fn sample_verifier_info_json() -> &'static str {
    r#"{
        "qVerifier": {
            "tmpUsed": 1,
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "number", "value": "1", "dim": 1}]
                }
            ]
        },
        "queryVerifier": {
            "expId": 7,
            "stage": 2,
            "tmpUsed": 1,
            "line": "query-expression",
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "eval", "id": 0, "dim": 3}]
                }
            ]
        }
    }"#
}

fn sample_expression_program() -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 1,
        max_tmp3: 1,
        max_args: 1,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id: 7,
            destination_dimension: 1,
            destination_id: 0,
            stage: 1,
            temp1_count: 0,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 1,
            args_offset: 0,
            source_line: "program-line".to_owned(),
        }],
        ops: vec![1],
        args: vec![2],
        numbers: vec![],
    }
}

fn sample_public_values(setup_hash: [u8; 32]) -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "block_number".to_owned(),
            elements: vec![12_345],
        }],
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

fn sample_raw_fixed_columns() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [1_u64, 10, 2, 20] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-cli-{}-{name}", std::process::id()))
}

fn write_text(path: &Path, value: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, value).expect("fixture file should be written");
}

fn write_bytes(path: &Path, value: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, value).expect("fixture file should be written");
}

fn write_global_files(root: &Path) {
    fs::create_dir_all(root).expect("fixture root should be created");
    fs::write(
        root.join("pilout.globalInfo.json"),
        sample_global_info_json(),
    )
    .expect("global metadata should be written");
    fs::write(root.join("pilout.globalConstraints.json"), "{}")
        .expect("global constraints metadata should be written");
    let constraints = encode_global_constraint_program(&GlobalConstraintProgram {
        entries: vec![],
        ops: vec![],
        args: vec![],
        numbers: vec![],
    })
    .expect("global constraints should encode");
    fs::write(root.join("pilout.globalConstraints.bin"), constraints)
        .expect("global constraints program should be written");
}

fn write_unit_files(unit: &KeyUnitPaths) {
    if let Some(path) = unit.setup_info() {
        write_text(&path, sample_setup_info_json());
    }
    if let Some(path) = unit.expression_info() {
        write_text(&path, sample_expression_info_json());
    }
    if let Some(path) = unit.verifier_info() {
        write_text(&path, sample_verifier_info_json());
    }

    let program =
        encode_expression_program(&sample_expression_program()).expect("program should encode");
    if let Some(path) = unit.expression_program() {
        write_bytes(&path, &program);
    }
    if let Some(path) = unit.verifier_program() {
        write_bytes(&path, &program);
    }

    write_text(&unit.verification_key_json(), "[1,2,3,4]");
    let root = VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]);
    write_bytes(
        &unit.verification_key_binary(),
        encode_verification_key_binary(&root).expect("verification key should encode"),
    );
    write_bytes(&unit.fixed_columns, sample_raw_fixed_columns());
}

fn write_setup_directory(root: &Path) {
    write_global_files(root);
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files(unit);
    }
}

fn write_proof_pair(root: &Path, setup_hash: [u8; 32]) -> (PathBuf, PathBuf) {
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof(&public_values);
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path)
}

#[test]
fn validates_a_complete_setup_directory() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "validate",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nunits=4\nglobal_constraints=0\nfixed_bytes=128\n"
    );
    assert!(stderr.is_empty());
}

#[test]
fn fingerprints_a_complete_setup_directory() {
    let dir = temp_dir("fingerprint");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "fingerprint",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!("status=ok\nunits=4\nfingerprint={expected}\n")
    );
    assert!(stderr.is_empty());
}

#[test]
fn prints_prove_schedule_for_setup_directory() {
    let dir = temp_dir("prove-schedule");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "schedule",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nfixed_bytes=128\nqueries=4\nmax_extended_domain_bits=2\nsetup_hash={expected}\n"
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn runs_setup_aware_verify_preflight() {
    let dir = temp_dir("verify-setup-preflight");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let (proof_path, public_values_path) = write_proof_pair(&dir, setup_hash);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nunits=4\nsegments=1\npublic_values=1\n"
    );
    assert!(stderr.is_empty());
}

#[test]
fn rejects_setup_aware_verify_preflight_with_mismatched_setup_catalog() {
    let dir = temp_dir("verify-setup-preflight-bad-setup");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let (proof_path, public_values_path) = write_proof_pair(&dir, [0x88; 32]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: setup catalog fingerprint mismatch\n"
    );
}

#[test]
fn reports_usage_for_missing_setup_directory() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "validate"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup validate <setup-dir>\n"
    );
}

#[test]
fn reports_usage_for_missing_fingerprint_directory() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "fingerprint"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup fingerprint <setup-dir>\n"
    );
}

#[test]
fn reports_usage_for_missing_prove_schedule_directory() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["prove", "schedule"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm prove schedule <setup-dir>\n"
    );
}

#[test]
fn reports_usage_for_missing_setup_preflight_inputs() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["verify", "setup-preflight"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm verify setup-preflight <setup-dir> <proof-bin> <public-values-json>\n"
    );
}

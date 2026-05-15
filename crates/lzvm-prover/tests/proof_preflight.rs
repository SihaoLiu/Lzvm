use lzvm_artifacts::proof::{ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, PublicValueEntry, PublicValues};
use lzvm_prover::proof_preflight::{
    validate_proof_public_values, ProofPreflightError, ProofPreflightReport,
};

fn sample_hash(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn sample_public_values() -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
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

#[test]
fn validates_proof_public_value_preflight_hashes() {
    let public_values = sample_public_values();
    let proof = sample_proof(&public_values);

    let report = validate_proof_public_values(&proof, &public_values)
        .expect("proof and public values should match");

    assert_eq!(
        report,
        ProofPreflightReport {
            segment_count: 1,
            public_value_count: 1,
        }
    );
}

#[test]
fn rejects_proof_public_value_setup_hash_mismatches() {
    let public_values = sample_public_values();
    let mut proof = sample_proof(&public_values);
    proof.setup_hash = sample_hash(0x55);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("setup hashes should match");

    assert_eq!(error, ProofPreflightError::SetupHashMismatch);
}

#[test]
fn rejects_proof_public_value_digest_mismatches() {
    let public_values = sample_public_values();
    let mut proof = sample_proof(&public_values);
    proof.public_values_hash = sample_hash(0x99);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("public value digest should match");

    assert_eq!(error, ProofPreflightError::PublicValuesHashMismatch);
}

use lzvm_artifacts::proof::{ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, PublicValueEntry, PublicValues};
use lzvm_field::{Felt, FieldError, MODULUS};
use lzvm_prover::proof_preflight::{
    public_values_as_fields, validate_proof_public_values, ProofPreflightError,
    ProofPreflightReport, PublicValueFieldError,
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

#[test]
fn converts_public_values_to_field_elements_in_entry_order() {
    let public_values = PublicValues {
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
    };

    let fields = public_values_as_fields(&public_values)
        .expect("canonical public values should convert to fields");

    assert_eq!(
        fields,
        vec![
            Felt::from_canonical(12_345).expect("value should be canonical"),
            Felt::from_canonical(1).expect("value should be canonical"),
            Felt::from_canonical(2).expect("value should be canonical"),
            Felt::from_canonical(3).expect("value should be canonical"),
            Felt::from_canonical(4).expect("value should be canonical"),
        ]
    );
}

#[test]
fn rejects_noncanonical_public_values_for_field_conversion() {
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![PublicValueEntry {
            name: "bad_value".to_owned(),
            elements: vec![MODULUS],
        }],
    };

    let error = public_values_as_fields(&public_values)
        .expect_err("field conversion should reject noncanonical values");

    assert_eq!(
        error,
        PublicValueFieldError::Field(FieldError::NonCanonical { value: MODULUS })
    );
}

use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::proof::{
    encode_proof_artifact, parse_proof_artifact, read_proof_artifact_file, ProofArtifact,
    ProofArtifactError, ProofSegment,
};

fn sample_hash(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-proof-artifact-{}-{name}", std::process::id()))
}

fn sample_proof() -> ProofArtifact {
    ProofArtifact {
        setup_hash: sample_hash(0x22),
        public_values_hash: sample_hash(0x33),
        segments: vec![
            ProofSegment {
                id: 100,
                data: vec![1, 2, 3, 4],
            },
            ProofSegment {
                id: 101,
                data: vec![5, 6, 7],
            },
        ],
    }
}

#[test]
fn encodes_and_parses_proof_artifacts() {
    let encoded = encode_proof_artifact(&sample_proof()).expect("fixture should encode");
    let parsed = parse_proof_artifact(&encoded).expect("fixture should parse");

    assert_eq!(&encoded[0..4], b"prf0");
    assert_eq!(parsed, sample_proof());
}

#[test]
fn reads_proof_artifacts_from_a_file_path() {
    let path = temp_file_path("proof.bin");
    let encoded = encode_proof_artifact(&sample_proof()).expect("fixture should encode");
    fs::write(&path, encoded).expect("fixture should be written");

    let parsed = read_proof_artifact_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_proof());
}

#[test]
fn rejects_proofs_without_metadata() {
    let encoded = lzvm_artifacts::sectioned::encode_sectioned_file(
        &lzvm_artifacts::sectioned::SectionedFile {
            kind: *b"prf0",
            version: 1,
            sections: vec![lzvm_artifacts::sectioned::SectionedSection {
                id: 100,
                data: vec![1],
            }],
        },
    )
    .expect("container should encode");

    assert!(matches!(
        parse_proof_artifact(&encoded),
        Err(ProofArtifactError::MissingMetadata)
    ));
}

#[test]
fn rejects_reserved_proof_segment_ids() {
    let mut proof = sample_proof();
    proof.segments[0].id = 1;

    assert!(matches!(
        encode_proof_artifact(&proof),
        Err(ProofArtifactError::ReservedSegmentId { id: 1 })
    ));
}

#[test]
fn rejects_duplicate_proof_segment_ids() {
    let mut proof = sample_proof();
    proof.segments[1].id = 100;

    assert!(matches!(
        encode_proof_artifact(&proof),
        Err(ProofArtifactError::DuplicateSegmentId { id: 100 })
    ));
}

#[test]
fn rejects_empty_proof_segments() {
    let mut proof = sample_proof();
    proof.segments[0].data.clear();

    assert!(matches!(
        encode_proof_artifact(&proof),
        Err(ProofArtifactError::EmptySegment { id: 100 })
    ));
}

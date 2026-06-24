use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::proof::{
    encode_proof_artifact, parse_proof_artifact, read_proof_artifact_file, ProofArtifact,
    ProofArtifactError, ProofSegment,
};
use lzvm_artifacts::sectioned::{
    encode_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

fn sample_hash(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn sample_metadata() -> Vec<u8> {
    let mut metadata = Vec::with_capacity(64);
    metadata.extend_from_slice(&sample_hash(0x22));
    metadata.extend_from_slice(&sample_hash(0x33));
    metadata
}

fn temp_file_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("temp")
        .join(format!("lzvm-proof-artifact-{}-{name}", std::process::id()))
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

fn proof_container_bytes(sections: Vec<SectionedSection>) -> Vec<u8> {
    proof_container_bytes_with_version(1, sections)
}

fn proof_container_bytes_with_version(version: u32, sections: Vec<SectionedSection>) -> Vec<u8> {
    proof_container_bytes_with_header(*b"prf0", version, sections)
}

fn proof_container_bytes_with_header(
    kind: [u8; 4],
    version: u32,
    sections: Vec<SectionedSection>,
) -> Vec<u8> {
    encode_sectioned_file(&SectionedFile {
        kind,
        version,
        sections,
    })
    .expect("proof container should encode")
}

fn assert_parse_error(encoded: Vec<u8>, expected: ProofArtifactError) {
    assert_eq!(
        parse_proof_artifact(&encoded).expect_err("proof artifact should reject"),
        expected
    );
}

fn assert_encode_error(proof: &ProofArtifact, expected: ProofArtifactError) {
    assert_eq!(
        encode_proof_artifact(proof).expect_err("proof artifact should reject"),
        expected
    );
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
    fs::create_dir_all(path.parent().expect("temp file should have parent"))
        .expect("fixture directory should be created");
    fs::write(&path, encoded).expect("fixture should be written");

    let parsed = read_proof_artifact_file(&path).expect("fixture should parse");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, sample_proof());
}

#[test]
fn rejects_proofs_without_metadata() {
    let encoded = proof_container_bytes(vec![SectionedSection {
        id: 100,
        data: vec![1],
    }]);

    assert_parse_error(encoded, ProofArtifactError::MissingMetadata);
}

#[test]
fn rejects_proofs_with_invalid_container_kind() {
    let encoded = proof_container_bytes_with_header(
        *b"bad!",
        1,
        vec![
            SectionedSection {
                id: 1,
                data: sample_metadata(),
            },
            SectionedSection {
                id: 100,
                data: vec![1],
            },
        ],
    );

    assert_parse_error(
        encoded,
        ProofArtifactError::Sectioned(SectionedError::InvalidKind {
            expected: *b"prf0",
            found: *b"bad!",
        }),
    );
}

#[test]
fn rejects_proofs_with_duplicate_metadata() {
    let encoded = proof_container_bytes(vec![
        SectionedSection {
            id: 1,
            data: sample_metadata(),
        },
        SectionedSection {
            id: 100,
            data: vec![1],
        },
        SectionedSection {
            id: 1,
            data: sample_metadata(),
        },
    ]);

    assert_parse_error(encoded, ProofArtifactError::DuplicateMetadata);
}

#[test]
fn rejects_proofs_with_invalid_metadata_length() {
    let encoded = proof_container_bytes(vec![
        SectionedSection {
            id: 1,
            data: vec![0; 63],
        },
        SectionedSection {
            id: 100,
            data: vec![1],
        },
    ]);

    assert_parse_error(
        encoded,
        ProofArtifactError::InvalidMetadataLength {
            expected: 64,
            found: 63,
        },
    );
}

#[test]
fn rejects_proofs_with_unsupported_versions() {
    let encoded = proof_container_bytes_with_version(
        0,
        vec![
            SectionedSection {
                id: 1,
                data: sample_metadata(),
            },
            SectionedSection {
                id: 100,
                data: vec![1],
            },
        ],
    );

    assert_parse_error(
        encoded,
        ProofArtifactError::UnsupportedVersion {
            found: 0,
            expected: 1,
        },
    );
}

#[test]
fn rejects_proofs_with_newer_container_versions() {
    let encoded = proof_container_bytes_with_version(
        2,
        vec![
            SectionedSection {
                id: 1,
                data: sample_metadata(),
            },
            SectionedSection {
                id: 100,
                data: vec![1],
            },
        ],
    );

    assert_parse_error(
        encoded,
        ProofArtifactError::Sectioned(SectionedError::UnsupportedVersion { found: 2, max: 1 }),
    );
}

#[test]
fn rejects_proofs_without_segments() {
    let encoded = proof_container_bytes(vec![SectionedSection {
        id: 1,
        data: sample_metadata(),
    }]);

    assert_parse_error(encoded, ProofArtifactError::MissingSegments);
}

#[test]
fn rejects_reserved_proof_segment_ids() {
    let mut proof = sample_proof();
    proof.segments[0].id = 1;

    assert_encode_error(&proof, ProofArtifactError::ReservedSegmentId { id: 1 });
}

#[test]
fn rejects_duplicate_proof_segment_ids() {
    let mut proof = sample_proof();
    proof.segments[1].id = 100;

    assert_encode_error(&proof, ProofArtifactError::DuplicateSegmentId { id: 100 });
}

#[test]
fn rejects_empty_proof_segments() {
    let mut proof = sample_proof();
    proof.segments[0].data.clear();

    assert_encode_error(&proof, ProofArtifactError::EmptySegment { id: 100 });
}

#[test]
fn rejects_parsed_proofs_with_empty_segments() {
    let encoded = proof_container_bytes(vec![
        SectionedSection {
            id: 1,
            data: sample_metadata(),
        },
        SectionedSection {
            id: 100,
            data: Vec::new(),
        },
    ]);

    assert_parse_error(encoded, ProofArtifactError::EmptySegment { id: 100 });
}

#[test]
fn rejects_parsed_proofs_with_reserved_segment_ids() {
    let encoded = proof_container_bytes(vec![
        SectionedSection {
            id: 1,
            data: sample_metadata(),
        },
        SectionedSection {
            id: 2,
            data: vec![1],
        },
    ]);

    assert_parse_error(encoded, ProofArtifactError::ReservedSegmentId { id: 2 });
}

#[test]
fn rejects_parsed_proofs_with_duplicate_segment_ids() {
    let encoded = proof_container_bytes(vec![
        SectionedSection {
            id: 1,
            data: sample_metadata(),
        },
        SectionedSection {
            id: 100,
            data: vec![1],
        },
        SectionedSection {
            id: 100,
            data: vec![2],
        },
    ]);

    assert_parse_error(encoded, ProofArtifactError::DuplicateSegmentId { id: 100 });
}

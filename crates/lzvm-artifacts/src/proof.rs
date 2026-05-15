use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const PROOF_KIND: [u8; 4] = *b"prf0";
const PROOF_VERSION: u32 = 1;
const METADATA_SECTION_ID: u32 = 1;
const FIRST_SEGMENT_ID: u32 = 100;
const HASH_BYTES: usize = 32;
const METADATA_BYTES: usize = HASH_BYTES * 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofArtifact {
    pub setup_hash: [u8; 32],
    pub public_values_hash: [u8; 32],
    pub segments: Vec<ProofSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofSegment {
    pub id: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofArtifactError {
    Sectioned(SectionedError),
    MissingMetadata,
    InvalidMetadataLength { expected: usize, found: usize },
    MissingSegments,
    ReservedSegmentId { id: u32 },
    DuplicateSegmentId { id: u32 },
    EmptySegment { id: u32 },
    Io { message: String },
}

impl fmt::Display for ProofArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sectioned(error) => write!(f, "proof artifact container error: {error}"),
            Self::MissingMetadata => write!(f, "missing proof artifact metadata"),
            Self::InvalidMetadataLength { expected, found } => write!(
                f,
                "invalid proof artifact metadata length: expected {expected}, found {found}"
            ),
            Self::MissingSegments => write!(f, "missing proof artifact segments"),
            Self::ReservedSegmentId { id } => write!(f, "reserved proof segment id: {id}"),
            Self::DuplicateSegmentId { id } => write!(f, "duplicate proof segment id: {id}"),
            Self::EmptySegment { id } => write!(f, "empty proof segment: {id}"),
            Self::Io { message } => write!(f, "proof artifact io error: {message}"),
        }
    }
}

impl std::error::Error for ProofArtifactError {}

pub fn read_proof_artifact_file(
    path: impl AsRef<Path>,
) -> Result<ProofArtifact, ProofArtifactError> {
    let bytes = std::fs::read(path).map_err(|error| ProofArtifactError::Io {
        message: error.to_string(),
    })?;
    parse_proof_artifact(&bytes)
}

pub fn parse_proof_artifact(bytes: &[u8]) -> Result<ProofArtifact, ProofArtifactError> {
    let file = parse_sectioned_file(bytes, PROOF_KIND, PROOF_VERSION)
        .map_err(ProofArtifactError::Sectioned)?;
    let metadata = file
        .sections
        .iter()
        .find(|section| section.id == METADATA_SECTION_ID)
        .ok_or(ProofArtifactError::MissingMetadata)?;
    if metadata.data.len() != METADATA_BYTES {
        return Err(ProofArtifactError::InvalidMetadataLength {
            expected: METADATA_BYTES,
            found: metadata.data.len(),
        });
    }

    let setup_hash: [u8; 32] = metadata.data[..HASH_BYTES]
        .try_into()
        .expect("slice length checked");
    let public_values_hash: [u8; 32] = metadata.data[HASH_BYTES..METADATA_BYTES]
        .try_into()
        .expect("slice length checked");
    let mut segments = Vec::new();
    for section in file.sections {
        if section.id == METADATA_SECTION_ID {
            continue;
        }
        segments.push(ProofSegment {
            id: section.id,
            data: section.data,
        });
    }

    let out = ProofArtifact {
        setup_hash,
        public_values_hash,
        segments,
    };
    validate_proof_artifact(&out)?;
    Ok(out)
}

pub fn encode_proof_artifact(value: &ProofArtifact) -> Result<Vec<u8>, ProofArtifactError> {
    validate_proof_artifact(value)?;

    let mut metadata = Vec::with_capacity(METADATA_BYTES);
    metadata.extend_from_slice(&value.setup_hash);
    metadata.extend_from_slice(&value.public_values_hash);

    let mut sections = Vec::with_capacity(value.segments.len() + 1);
    sections.push(SectionedSection {
        id: METADATA_SECTION_ID,
        data: metadata,
    });
    for segment in &value.segments {
        sections.push(SectionedSection {
            id: segment.id,
            data: segment.data.clone(),
        });
    }

    let file = SectionedFile {
        kind: PROOF_KIND,
        version: PROOF_VERSION,
        sections,
    };
    encode_sectioned_file(&file).map_err(ProofArtifactError::Sectioned)
}

fn validate_proof_artifact(value: &ProofArtifact) -> Result<(), ProofArtifactError> {
    if value.segments.is_empty() {
        return Err(ProofArtifactError::MissingSegments);
    }

    let mut seen = BTreeSet::new();
    for segment in &value.segments {
        if segment.id < FIRST_SEGMENT_ID {
            return Err(ProofArtifactError::ReservedSegmentId { id: segment.id });
        }
        if !seen.insert(segment.id) {
            return Err(ProofArtifactError::DuplicateSegmentId { id: segment.id });
        }
        if segment.data.is_empty() {
            return Err(ProofArtifactError::EmptySegment { id: segment.id });
        }
    }
    Ok(())
}

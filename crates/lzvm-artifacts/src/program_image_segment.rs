use std::fmt;

use crate::program_image::{
    encode_program_image_commitment_cache_payload, parse_program_image_commitment_cache_payload,
    validate_program_image_commitment_cache, ProgramImageCommitmentCache,
    ProgramImageCommitmentCacheError, PROGRAM_IMAGE_CACHE_PAYLOAD_BYTES,
};
use sha2::{Digest, Sha256};

pub const PROGRAM_IMAGE_CACHE_SEGMENT_ID: u32 = 10_010;

const PROGRAM_IMAGE_CACHE_SEGMENT_MAGIC: [u8; 4] = *b"pic0";
const PROGRAM_IMAGE_CACHE_SEGMENT_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramImageCacheSegmentError {
    InvalidMagic,
    UnsupportedVersion { version: u32 },
    InvalidPayload(ProgramImageCommitmentCacheError),
    UnexpectedEof { needed: usize, available: usize },
}

impl fmt::Display for ProgramImageCacheSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid program image cache segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(
                    f,
                    "unsupported program image cache segment version: {version}"
                )
            }
            Self::InvalidPayload(error) => {
                write!(f, "invalid program image cache segment payload: {error}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated program image cache segment: needed {needed}, available {available}"
            ),
        }
    }
}

impl std::error::Error for ProgramImageCacheSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPayload(error) => Some(error),
            Self::InvalidMagic | Self::UnsupportedVersion { .. } | Self::UnexpectedEof { .. } => {
                None
            }
        }
    }
}

pub fn encode_program_image_cache_segment(
    value: &ProgramImageCommitmentCache,
) -> Result<Vec<u8>, ProgramImageCacheSegmentError> {
    validate_program_image_commitment_cache(value)
        .map_err(ProgramImageCacheSegmentError::InvalidPayload)?;
    let mut out = Vec::with_capacity(HEADER_BYTES + PROGRAM_IMAGE_CACHE_PAYLOAD_BYTES);
    out.extend_from_slice(&PROGRAM_IMAGE_CACHE_SEGMENT_MAGIC);
    out.extend_from_slice(&PROGRAM_IMAGE_CACHE_SEGMENT_VERSION.to_le_bytes());
    out.extend_from_slice(&encode_program_image_commitment_cache_payload(value));
    Ok(out)
}

pub fn program_image_cache_segment_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn parse_program_image_cache_segment(
    bytes: &[u8],
) -> Result<ProgramImageCommitmentCache, ProgramImageCacheSegmentError> {
    if bytes.len() < HEADER_BYTES {
        return Err(ProgramImageCacheSegmentError::UnexpectedEof {
            needed: HEADER_BYTES,
            available: bytes.len(),
        });
    }
    let mut magic = [0_u8; 4];
    magic.copy_from_slice(&bytes[..4]);
    if magic != PROGRAM_IMAGE_CACHE_SEGMENT_MAGIC {
        return Err(ProgramImageCacheSegmentError::InvalidMagic);
    }
    let mut version_bytes = [0_u8; 4];
    version_bytes.copy_from_slice(&bytes[4..8]);
    let version = u32::from_le_bytes(version_bytes);
    if version != PROGRAM_IMAGE_CACHE_SEGMENT_VERSION {
        return Err(ProgramImageCacheSegmentError::UnsupportedVersion { version });
    }
    parse_program_image_commitment_cache_payload(&bytes[HEADER_BYTES..])
        .map_err(ProgramImageCacheSegmentError::InvalidPayload)
}

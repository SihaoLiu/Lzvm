use std::fmt;
use std::path::Path;

use lzvm_field::{Felt, FieldError};
use sha2::{Digest, Sha256};

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const PROGRAM_IMAGE_KIND: [u8; 4] = *b"pimg";
const PROGRAM_IMAGE_VERSION: u32 = 1;
const PROGRAM_IMAGE_SECTION_ID: u32 = 1;
const DIGEST_BYTES: usize = 32;
const ROOT_WORDS: usize = 4;
const MAX_TRACE_DOMAIN_BITS: u32 = 32;
pub(crate) const PROGRAM_IMAGE_CACHE_PAYLOAD_BYTES: usize =
    DIGEST_BYTES * 3 + ROOT_WORDS * 8 + 8 + 4 * 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramImageCommitmentCache {
    pub program_digest: [u8; 32],
    pub source_image_digest: [u8; 32],
    pub constraint_system_digest: [u8; 32],
    pub tree_root: [u64; 4],
    pub trace_row_count: u64,
    pub trace_column_count: u32,
    pub blowup_factor: u32,
    pub merkle_tree_arity: u32,
    pub gpu_mode: ProgramImageGpuMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramImageGpuMode {
    Cpu,
    Cuda,
}

impl ProgramImageGpuMode {
    fn to_u32(self) -> u32 {
        match self {
            Self::Cpu => 0,
            Self::Cuda => 1,
        }
    }

    fn from_u32(value: u32) -> Result<Self, ProgramImageCommitmentCacheError> {
        match value {
            0 => Ok(Self::Cpu),
            1 => Ok(Self::Cuda),
            value => Err(ProgramImageCommitmentCacheError::UnsupportedGpuMode { value }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProgramImageCommitmentInputs<'a> {
    pub program_bytes: &'a [u8],
    pub source_image_bytes: &'a [u8],
    pub constraint_system_digest: [u8; 32],
    pub tree_root: [u64; 4],
    pub trace_row_count: u64,
    pub trace_column_count: u32,
    pub blowup_factor: u32,
    pub merkle_tree_arity: u32,
    pub gpu_mode: ProgramImageGpuMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramImageCommitmentCacheError {
    Sectioned(SectionedError),
    UnsupportedVersion {
        found: u32,
        expected: u32,
    },
    InvalidSectionCount {
        found: u32,
    },
    InvalidSectionId {
        found: u32,
    },
    InvalidPayloadLength {
        expected: usize,
        found: usize,
    },
    EmptyProgram,
    EmptySourceImage,
    EmptyTraceRows,
    InvalidTraceRows {
        value: u64,
    },
    EmptyTraceColumns,
    InvalidBlowupFactor {
        value: u32,
    },
    TraceRowExpansionOverflow {
        trace_row_count: u64,
        blowup_factor: u32,
    },
    UnsupportedTraceDomainBits {
        bits: u32,
        max_bits: u32,
    },
    InvalidMerkleTreeArity {
        value: u32,
    },
    TreeRootNonCanonical {
        word_index: usize,
        source: FieldError,
    },
    UnsupportedGpuMode {
        value: u32,
    },
    Io {
        message: String,
    },
}

impl fmt::Display for ProgramImageCommitmentCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sectioned(error) => {
                write!(
                    f,
                    "program-image commitment cache container error: {error}"
                )
            }
            Self::UnsupportedVersion { found, expected } => write!(
                f,
                "unsupported program-image commitment cache version {found}, expected {expected}"
            ),
            Self::InvalidSectionCount { found } => {
                write!(
                    f,
                    "invalid program-image commitment cache section count {found}"
                )
            }
            Self::InvalidSectionId { found } => {
                write!(
                    f,
                    "invalid program-image commitment cache section id {found}"
                )
            }
            Self::InvalidPayloadLength { expected, found } => write!(
                f,
                "invalid program-image commitment cache payload length: expected {expected}, found {found}"
            ),
            Self::EmptyProgram => write!(f, "program-image commitment cache program is empty"),
            Self::EmptySourceImage => {
                write!(f, "program-image commitment cache source image is empty")
            }
            Self::EmptyTraceRows => {
                write!(f, "program-image commitment cache trace rows are empty")
            }
            Self::InvalidTraceRows { value } => {
                write!(f, "invalid program-image commitment cache trace rows {value}")
            }
            Self::EmptyTraceColumns => {
                write!(f, "program-image commitment cache trace columns are empty")
            }
            Self::InvalidBlowupFactor { value } => write!(
                f,
                "invalid program-image commitment cache blowup factor {value}"
            ),
            Self::TraceRowExpansionOverflow {
                trace_row_count,
                blowup_factor,
            } => write!(
                f,
                "program-image commitment cache trace row expansion overflows: rows {trace_row_count}, blowup factor {blowup_factor}"
            ),
            Self::UnsupportedTraceDomainBits { bits, max_bits } => write!(
                f,
                "unsupported program-image commitment cache trace domain bits {bits}, max {max_bits}"
            ),
            Self::InvalidMerkleTreeArity { value } => write!(
                f,
                "invalid program-image commitment cache Merkle tree arity {value}"
            ),
            Self::TreeRootNonCanonical { word_index, source } => write!(
                f,
                "program-image commitment cache tree root word {word_index} is non-canonical: {source}"
            ),
            Self::UnsupportedGpuMode { value } => write!(
                f,
                "unsupported program-image commitment cache GPU mode {value}"
            ),
            Self::Io { message } => {
                write!(f, "program-image commitment cache io error: {message}")
            }
        }
    }
}

impl std::error::Error for ProgramImageCommitmentCacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sectioned(error) => Some(error),
            Self::TreeRootNonCanonical { source, .. } => Some(source),
            Self::UnsupportedVersion { .. }
            | Self::InvalidSectionCount { .. }
            | Self::InvalidSectionId { .. }
            | Self::InvalidPayloadLength { .. }
            | Self::EmptyProgram
            | Self::EmptySourceImage
            | Self::EmptyTraceRows
            | Self::InvalidTraceRows { .. }
            | Self::EmptyTraceColumns
            | Self::InvalidBlowupFactor { .. }
            | Self::TraceRowExpansionOverflow { .. }
            | Self::UnsupportedTraceDomainBits { .. }
            | Self::InvalidMerkleTreeArity { .. }
            | Self::UnsupportedGpuMode { .. }
            | Self::Io { .. } => None,
        }
    }
}

impl From<SectionedError> for ProgramImageCommitmentCacheError {
    fn from(error: SectionedError) -> Self {
        Self::Sectioned(error)
    }
}

pub fn build_program_image_commitment_cache(
    inputs: ProgramImageCommitmentInputs<'_>,
) -> Result<ProgramImageCommitmentCache, ProgramImageCommitmentCacheError> {
    if inputs.program_bytes.is_empty() {
        return Err(ProgramImageCommitmentCacheError::EmptyProgram);
    }
    if inputs.source_image_bytes.is_empty() {
        return Err(ProgramImageCommitmentCacheError::EmptySourceImage);
    }

    let out = ProgramImageCommitmentCache {
        program_digest: Sha256::digest(inputs.program_bytes).into(),
        source_image_digest: Sha256::digest(inputs.source_image_bytes).into(),
        constraint_system_digest: inputs.constraint_system_digest,
        tree_root: inputs.tree_root,
        trace_row_count: inputs.trace_row_count,
        trace_column_count: inputs.trace_column_count,
        blowup_factor: inputs.blowup_factor,
        merkle_tree_arity: inputs.merkle_tree_arity,
        gpu_mode: inputs.gpu_mode,
    };
    validate_program_image_commitment_cache(&out)?;
    Ok(out)
}

pub fn read_program_image_commitment_cache_file(
    path: impl AsRef<Path>,
) -> Result<ProgramImageCommitmentCache, ProgramImageCommitmentCacheError> {
    let bytes = std::fs::read(path).map_err(|error| ProgramImageCommitmentCacheError::Io {
        message: error.to_string(),
    })?;
    parse_program_image_commitment_cache(&bytes)
}

pub fn parse_program_image_commitment_cache(
    bytes: &[u8],
) -> Result<ProgramImageCommitmentCache, ProgramImageCommitmentCacheError> {
    let file = parse_sectioned_file(bytes, PROGRAM_IMAGE_KIND, PROGRAM_IMAGE_VERSION)?;
    if file.version != PROGRAM_IMAGE_VERSION {
        return Err(ProgramImageCommitmentCacheError::UnsupportedVersion {
            found: file.version,
            expected: PROGRAM_IMAGE_VERSION,
        });
    }

    if file.sections.len() != 1 {
        return Err(ProgramImageCommitmentCacheError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }
    let section = &file.sections[0];
    if section.id != PROGRAM_IMAGE_SECTION_ID {
        return Err(ProgramImageCommitmentCacheError::InvalidSectionId { found: section.id });
    }
    let out = parse_program_image_commitment_cache_payload(&section.data)?;
    validate_program_image_commitment_cache(&out)?;
    Ok(out)
}

pub fn encode_program_image_commitment_cache(
    value: &ProgramImageCommitmentCache,
) -> Result<Vec<u8>, ProgramImageCommitmentCacheError> {
    validate_program_image_commitment_cache(value)?;
    let file = SectionedFile {
        kind: PROGRAM_IMAGE_KIND,
        version: PROGRAM_IMAGE_VERSION,
        sections: vec![SectionedSection {
            id: PROGRAM_IMAGE_SECTION_ID,
            data: encode_program_image_commitment_cache_payload(value),
        }],
    };
    encode_sectioned_file(&file).map_err(ProgramImageCommitmentCacheError::Sectioned)
}

pub(crate) fn validate_program_image_commitment_cache(
    value: &ProgramImageCommitmentCache,
) -> Result<(), ProgramImageCommitmentCacheError> {
    if value.trace_row_count == 0 {
        return Err(ProgramImageCommitmentCacheError::EmptyTraceRows);
    }
    if !value.trace_row_count.is_power_of_two() {
        return Err(ProgramImageCommitmentCacheError::InvalidTraceRows {
            value: value.trace_row_count,
        });
    }
    if value.trace_column_count == 0 {
        return Err(ProgramImageCommitmentCacheError::EmptyTraceColumns);
    }
    if value.blowup_factor == 0 || !value.blowup_factor.is_power_of_two() {
        return Err(ProgramImageCommitmentCacheError::InvalidBlowupFactor {
            value: value.blowup_factor,
        });
    }
    if value
        .trace_row_count
        .checked_mul(u64::from(value.blowup_factor))
        .is_none()
    {
        return Err(
            ProgramImageCommitmentCacheError::TraceRowExpansionOverflow {
                trace_row_count: value.trace_row_count,
                blowup_factor: value.blowup_factor,
            },
        );
    }
    let trace_domain_bits =
        value.trace_row_count.trailing_zeros() + value.blowup_factor.trailing_zeros();
    if trace_domain_bits > MAX_TRACE_DOMAIN_BITS {
        return Err(
            ProgramImageCommitmentCacheError::UnsupportedTraceDomainBits {
                bits: trace_domain_bits,
                max_bits: MAX_TRACE_DOMAIN_BITS,
            },
        );
    }
    if !matches!(value.merkle_tree_arity, 2 | 4) {
        return Err(ProgramImageCommitmentCacheError::InvalidMerkleTreeArity {
            value: value.merkle_tree_arity,
        });
    }
    for (word_index, word) in value.tree_root.iter().copied().enumerate() {
        Felt::from_canonical(word).map_err(|source| {
            ProgramImageCommitmentCacheError::TreeRootNonCanonical { word_index, source }
        })?;
    }
    Ok(())
}

pub(crate) fn parse_program_image_commitment_cache_payload(
    bytes: &[u8],
) -> Result<ProgramImageCommitmentCache, ProgramImageCommitmentCacheError> {
    if bytes.len() != PROGRAM_IMAGE_CACHE_PAYLOAD_BYTES {
        return Err(ProgramImageCommitmentCacheError::InvalidPayloadLength {
            expected: PROGRAM_IMAGE_CACHE_PAYLOAD_BYTES,
            found: bytes.len(),
        });
    }

    let mut offset = 0;
    let program_digest = read_digest(bytes, &mut offset);
    let source_image_digest = read_digest(bytes, &mut offset);
    let constraint_system_digest = read_digest(bytes, &mut offset);
    let mut tree_root = [0_u64; ROOT_WORDS];
    for value in &mut tree_root {
        *value = read_u64(bytes, &mut offset);
    }
    let out = ProgramImageCommitmentCache {
        program_digest,
        source_image_digest,
        constraint_system_digest,
        tree_root,
        trace_row_count: read_u64(bytes, &mut offset),
        trace_column_count: read_u32(bytes, &mut offset),
        blowup_factor: read_u32(bytes, &mut offset),
        merkle_tree_arity: read_u32(bytes, &mut offset),
        gpu_mode: ProgramImageGpuMode::from_u32(read_u32(bytes, &mut offset))?,
    };
    validate_program_image_commitment_cache(&out)?;
    Ok(out)
}

pub(crate) fn encode_program_image_commitment_cache_payload(
    value: &ProgramImageCommitmentCache,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(PROGRAM_IMAGE_CACHE_PAYLOAD_BYTES);
    out.extend_from_slice(&value.program_digest);
    out.extend_from_slice(&value.source_image_digest);
    out.extend_from_slice(&value.constraint_system_digest);
    for word in value.tree_root {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&value.trace_row_count.to_le_bytes());
    out.extend_from_slice(&value.trace_column_count.to_le_bytes());
    out.extend_from_slice(&value.blowup_factor.to_le_bytes());
    out.extend_from_slice(&value.merkle_tree_arity.to_le_bytes());
    out.extend_from_slice(&value.gpu_mode.to_u32().to_le_bytes());
    out
}

fn read_digest(bytes: &[u8], offset: &mut usize) -> [u8; 32] {
    let end = *offset + DIGEST_BYTES;
    let out = bytes[*offset..end]
        .try_into()
        .expect("payload length checked");
    *offset = end;
    out
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> u32 {
    let end = *offset + 4;
    let out = u32::from_le_bytes(
        bytes[*offset..end]
            .try_into()
            .expect("payload length checked"),
    );
    *offset = end;
    out
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> u64 {
    let end = *offset + 8;
    let out = u64::from_le_bytes(
        bytes[*offset..end]
            .try_into()
            .expect("payload length checked"),
    );
    *offset = end;
    out
}

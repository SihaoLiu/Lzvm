use std::fmt;

use lzvm_field::{Felt, FieldError};

pub const WITNESS_COMMITMENT_SEGMENT_BASE_ID: u32 = 100;

const WITNESS_COMMITMENT_SEGMENT_MAGIC: [u8; 4] = *b"wcs0";
const WITNESS_COMMITMENT_SEGMENT_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4 + 8 + 8 + 8 + 4;
const ROOT_WORDS: usize = 4;
const HASH_BYTES: usize = 32;
const STAGE_BYTES: usize = 4 + 4 + ROOT_WORDS * 8 + 8 + HASH_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessCommitmentSegment {
    pub unit_index: u32,
    pub input_byte_count: u64,
    pub trace_rows: u64,
    pub trace_columns: u64,
    pub stages: Vec<WitnessCommitmentStageSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessCommitmentStageSegment {
    pub stage_index: u32,
    pub arity: u32,
    pub root: [u64; ROOT_WORDS],
    pub tree_byte_count: u64,
    pub tree_digest: [u8; HASH_BYTES],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessCommitmentSegmentError {
    InvalidMagic,
    UnsupportedVersion {
        version: u32,
    },
    UnexpectedEof {
        needed: usize,
        available: usize,
    },
    TrailingBytes {
        trailing: usize,
    },
    EmptyStages,
    StageRootNonCanonical {
        unit_index: u32,
        stage_index: u32,
        word_index: usize,
        source: FieldError,
    },
    LengthOverflow,
}

impl fmt::Display for WitnessCommitmentSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid witness commitment segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(
                    f,
                    "unsupported witness commitment segment version: {version}"
                )
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated witness commitment segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing witness commitment segment bytes: {trailing}")
            }
            Self::EmptyStages => write!(f, "witness commitment segment has no stages"),
            Self::StageRootNonCanonical {
                unit_index,
                stage_index,
                word_index,
                source,
            } => write!(
                f,
                "witness commitment unit {unit_index} stage {stage_index} root word {word_index} is non-canonical: {source}"
            ),
            Self::LengthOverflow => write!(f, "witness commitment segment length overflow"),
        }
    }
}

impl std::error::Error for WitnessCommitmentSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::StageRootNonCanonical { source, .. } => Some(source),
            Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::UnexpectedEof { .. }
            | Self::TrailingBytes { .. }
            | Self::EmptyStages
            | Self::LengthOverflow => None,
        }
    }
}

pub fn encode_witness_commitment_segment(
    value: &WitnessCommitmentSegment,
) -> Result<Vec<u8>, WitnessCommitmentSegmentError> {
    validate_witness_commitment_segment(value)?;
    let stage_count = u32::try_from(value.stages.len())
        .map_err(|_| WitnessCommitmentSegmentError::LengthOverflow)?;
    let expected_len = value
        .stages
        .len()
        .checked_mul(STAGE_BYTES)
        .and_then(|bytes| bytes.checked_add(HEADER_BYTES))
        .ok_or(WitnessCommitmentSegmentError::LengthOverflow)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&WITNESS_COMMITMENT_SEGMENT_MAGIC);
    write_u32(&mut out, WITNESS_COMMITMENT_SEGMENT_VERSION);
    write_u32(&mut out, value.unit_index);
    write_u64(&mut out, value.input_byte_count);
    write_u64(&mut out, value.trace_rows);
    write_u64(&mut out, value.trace_columns);
    write_u32(&mut out, stage_count);
    for stage in &value.stages {
        write_u32(&mut out, stage.stage_index);
        write_u32(&mut out, stage.arity);
        for word in stage.root {
            write_u64(&mut out, word);
        }
        write_u64(&mut out, stage.tree_byte_count);
        out.extend_from_slice(&stage.tree_digest);
    }
    Ok(out)
}

pub fn parse_witness_commitment_segment(
    bytes: &[u8],
) -> Result<WitnessCommitmentSegment, WitnessCommitmentSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != WITNESS_COMMITMENT_SEGMENT_MAGIC {
        return Err(WitnessCommitmentSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != WITNESS_COMMITMENT_SEGMENT_VERSION {
        return Err(WitnessCommitmentSegmentError::UnsupportedVersion { version });
    }
    let unit_index = reader.read_u32()?;
    let input_byte_count = reader.read_u64()?;
    let trace_rows = reader.read_u64()?;
    let trace_columns = reader.read_u64()?;
    let stage_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| WitnessCommitmentSegmentError::LengthOverflow)?;
    if stage_count == 0 {
        return Err(WitnessCommitmentSegmentError::EmptyStages);
    }
    if stage_count > reader.remaining_len() / STAGE_BYTES {
        return Err(WitnessCommitmentSegmentError::LengthOverflow);
    }

    let mut stages = Vec::with_capacity(stage_count);
    for _ in 0..stage_count {
        let stage_index = reader.read_u32()?;
        let arity = reader.read_u32()?;
        let mut root = [0_u64; ROOT_WORDS];
        for word in &mut root {
            *word = reader.read_u64()?;
        }
        let tree_byte_count = reader.read_u64()?;
        let tree_digest = reader.read_array::<HASH_BYTES>()?;
        stages.push(WitnessCommitmentStageSegment {
            stage_index,
            arity,
            root,
            tree_byte_count,
            tree_digest,
        });
    }
    reader.finish()?;

    let out = WitnessCommitmentSegment {
        unit_index,
        input_byte_count,
        trace_rows,
        trace_columns,
        stages,
    };
    validate_witness_commitment_segment(&out)?;
    Ok(out)
}

fn validate_witness_commitment_segment(
    value: &WitnessCommitmentSegment,
) -> Result<(), WitnessCommitmentSegmentError> {
    if value.stages.is_empty() {
        return Err(WitnessCommitmentSegmentError::EmptyStages);
    }
    for stage in &value.stages {
        for (word_index, word) in stage.root.iter().copied().enumerate() {
            Felt::from_canonical(word).map_err(|source| {
                WitnessCommitmentSegmentError::StageRootNonCanonical {
                    unit_index: value.unit_index,
                    stage_index: stage.stage_index,
                    word_index,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct SegmentReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SegmentReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u32(&mut self) -> Result<u32, WitnessCommitmentSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, WitnessCommitmentSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], WitnessCommitmentSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(WitnessCommitmentSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(WitnessCommitmentSegmentError::UnexpectedEof {
                needed: end,
                available: self.bytes.len(),
            });
        }
        let out = self.bytes[self.offset..end]
            .try_into()
            .expect("slice length checked");
        self.offset = end;
        Ok(out)
    }

    fn finish(&self) -> Result<(), WitnessCommitmentSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(WitnessCommitmentSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

use std::collections::BTreeSet;
use std::fmt;

use lzvm_field::{Felt, FieldError};

pub const PCS_EVALUATION_SEGMENT_ID: u32 = 10_006;

const PCS_EVALUATION_MAGIC: [u8; 4] = *b"evs0";
const PCS_EVALUATION_V1_VERSION: u32 = 1;
const PCS_EVALUATION_V2_VERSION: u32 = 2;
const HEADER_BYTES: usize = 4 + 4 + 4;
const V1_UNIT_HEADER_BYTES: usize = 4 + 4;
const V2_UNIT_HEADER_BYTES: usize = 4 + 4 + 4;
const WORD_BYTES: usize = 8;
const EXTENSION_WORDS: usize = 3;
const EXTENSION_BYTES: usize = EXTENSION_WORDS * WORD_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsEvaluationSegment {
    pub units: Vec<PcsEvaluationUnitSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsEvaluationUnitSegment {
    pub unit_index: u32,
    pub trace_instance_index: u32,
    pub values: Vec<[u64; EXTENSION_WORDS]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsEvaluationSegmentError {
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
    EmptyUnits,
    EmptyValues {
        unit_index: u32,
    },
    DuplicateUnitIndex {
        unit_index: u32,
    },
    DuplicateUnitIdentity {
        unit_index: u32,
        trace_instance_index: u32,
    },
    ValueNonCanonical {
        unit_index: u32,
        value_index: usize,
        word_index: usize,
        source: FieldError,
    },
    LengthOverflow,
}

impl fmt::Display for PcsEvaluationSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid PCS evaluation segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported PCS evaluation segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated PCS evaluation segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing PCS evaluation segment bytes: {trailing}")
            }
            Self::EmptyUnits => write!(f, "PCS evaluation segment has no units"),
            Self::EmptyValues { unit_index } => {
                write!(f, "PCS evaluation unit {unit_index} has no values")
            }
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate PCS evaluation unit index: {unit_index}")
            }
            Self::DuplicateUnitIdentity {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "duplicate PCS evaluation unit identity: unit {unit_index}, trace instance {trace_instance_index}"
            ),
            Self::ValueNonCanonical {
                unit_index,
                value_index,
                word_index,
                source,
            } => write!(
                f,
                "PCS evaluation unit {unit_index} value {value_index} word {word_index} is non-canonical: {source}"
            ),
            Self::LengthOverflow => write!(f, "PCS evaluation segment length overflow"),
        }
    }
}

impl std::error::Error for PcsEvaluationSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ValueNonCanonical { source, .. } => Some(source),
            Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::UnexpectedEof { .. }
            | Self::TrailingBytes { .. }
            | Self::EmptyUnits
            | Self::EmptyValues { .. }
            | Self::DuplicateUnitIndex { .. }
            | Self::DuplicateUnitIdentity { .. }
            | Self::LengthOverflow => None,
        }
    }
}

pub fn encode_pcs_evaluation_segment(
    value: &PcsEvaluationSegment,
) -> Result<Vec<u8>, PcsEvaluationSegmentError> {
    validate_pcs_evaluation_segment(value)?;
    let unit_count =
        u32::try_from(value.units.len()).map_err(|_| PcsEvaluationSegmentError::LengthOverflow)?;
    let version = segment_version(value);
    let expected_len = encoded_len(value)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&PCS_EVALUATION_MAGIC);
    write_u32(&mut out, version);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
        if version == PCS_EVALUATION_V2_VERSION {
            write_u32(&mut out, unit.trace_instance_index);
        }
        write_u32(
            &mut out,
            u32::try_from(unit.values.len())
                .map_err(|_| PcsEvaluationSegmentError::LengthOverflow)?,
        );
        for value in &unit.values {
            write_extension(&mut out, *value);
        }
    }
    Ok(out)
}

pub fn parse_pcs_evaluation_segment(
    bytes: &[u8],
) -> Result<PcsEvaluationSegment, PcsEvaluationSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != PCS_EVALUATION_MAGIC {
        return Err(PcsEvaluationSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != PCS_EVALUATION_V1_VERSION && version != PCS_EVALUATION_V2_VERSION {
        return Err(PcsEvaluationSegmentError::UnsupportedVersion { version });
    }
    let unit_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| PcsEvaluationSegmentError::LengthOverflow)?;
    if unit_count == 0 {
        return Err(PcsEvaluationSegmentError::EmptyUnits);
    }
    let unit_header_bytes = if version == PCS_EVALUATION_V2_VERSION {
        V2_UNIT_HEADER_BYTES
    } else {
        V1_UNIT_HEADER_BYTES
    };
    reader.require_items(unit_count, unit_header_bytes)?;

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let trace_instance_index = if version == PCS_EVALUATION_V2_VERSION {
            reader.read_u32()?
        } else {
            0
        };
        let value_count = usize::try_from(reader.read_u32()?)
            .map_err(|_| PcsEvaluationSegmentError::LengthOverflow)?;
        reader.require_items(value_count, EXTENSION_BYTES)?;
        let mut values = Vec::with_capacity(value_count);
        for _ in 0..value_count {
            values.push(reader.read_extension()?);
        }
        units.push(PcsEvaluationUnitSegment {
            unit_index,
            trace_instance_index,
            values,
        });
    }
    reader.finish()?;

    let out = PcsEvaluationSegment { units };
    validate_pcs_evaluation_segment(&out)?;
    Ok(out)
}

fn segment_version(value: &PcsEvaluationSegment) -> u32 {
    if value
        .units
        .iter()
        .any(|unit| unit.trace_instance_index != 0)
    {
        PCS_EVALUATION_V2_VERSION
    } else {
        PCS_EVALUATION_V1_VERSION
    }
}

fn validate_pcs_evaluation_segment(
    value: &PcsEvaluationSegment,
) -> Result<(), PcsEvaluationSegmentError> {
    if value.units.is_empty() {
        return Err(PcsEvaluationSegmentError::EmptyUnits);
    }
    let mut seen_units = BTreeSet::new();
    for unit in &value.units {
        if !seen_units.insert((unit.unit_index, unit.trace_instance_index)) {
            if unit.trace_instance_index == 0 {
                return Err(PcsEvaluationSegmentError::DuplicateUnitIndex {
                    unit_index: unit.unit_index,
                });
            }
            return Err(PcsEvaluationSegmentError::DuplicateUnitIdentity {
                unit_index: unit.unit_index,
                trace_instance_index: unit.trace_instance_index,
            });
        }
        if unit.values.is_empty() {
            return Err(PcsEvaluationSegmentError::EmptyValues {
                unit_index: unit.unit_index,
            });
        }
        for (value_index, value) in unit.values.iter().enumerate() {
            for (word_index, word) in value.iter().copied().enumerate() {
                Felt::from_canonical(word).map_err(|source| {
                    PcsEvaluationSegmentError::ValueNonCanonical {
                        unit_index: unit.unit_index,
                        value_index,
                        word_index,
                        source,
                    }
                })?;
            }
        }
    }
    Ok(())
}

fn encoded_len(value: &PcsEvaluationSegment) -> Result<usize, PcsEvaluationSegmentError> {
    let unit_header_bytes = if segment_version(value) == PCS_EVALUATION_V2_VERSION {
        V2_UNIT_HEADER_BYTES
    } else {
        V1_UNIT_HEADER_BYTES
    };
    value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        let value_bytes = unit
            .values
            .len()
            .checked_mul(EXTENSION_WORDS)
            .and_then(|words| words.checked_mul(WORD_BYTES))
            .ok_or(PcsEvaluationSegmentError::LengthOverflow)?;
        acc.checked_add(unit_header_bytes)
            .and_then(|bytes| bytes.checked_add(value_bytes))
            .ok_or(PcsEvaluationSegmentError::LengthOverflow)
    })
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_extension(out: &mut Vec<u8>, value: [u64; EXTENSION_WORDS]) {
    for word in value {
        write_u64(out, word);
    }
}

struct SegmentReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SegmentReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u32(&mut self) -> Result<u32, PcsEvaluationSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, PcsEvaluationSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_extension(&mut self) -> Result<[u64; EXTENSION_WORDS], PcsEvaluationSegmentError> {
        let mut value = [0_u64; EXTENSION_WORDS];
        for word in &mut value {
            *word = self.read_u64()?;
        }
        Ok(value)
    }

    fn require_items(
        &self,
        count: usize,
        item_bytes: usize,
    ) -> Result<(), PcsEvaluationSegmentError> {
        let payload_bytes = count
            .checked_mul(item_bytes)
            .ok_or(PcsEvaluationSegmentError::LengthOverflow)?;
        let expected_len = self
            .offset
            .checked_add(payload_bytes)
            .ok_or(PcsEvaluationSegmentError::LengthOverflow)?;
        if expected_len > self.bytes.len() {
            return Err(PcsEvaluationSegmentError::UnexpectedEof {
                needed: expected_len,
                available: self.bytes.len(),
            });
        }
        Ok(())
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], PcsEvaluationSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(PcsEvaluationSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PcsEvaluationSegmentError::UnexpectedEof {
                needed: end,
                available: self.bytes.len(),
            });
        }
        let mut out = [0_u8; N];
        out.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        Ok(out)
    }

    fn finish(&self) -> Result<(), PcsEvaluationSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(PcsEvaluationSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

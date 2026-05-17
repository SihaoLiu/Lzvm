use std::collections::BTreeSet;
use std::fmt;

pub const PCS_EVALUATION_SEGMENT_ID: u32 = 10_006;

const PCS_EVALUATION_MAGIC: [u8; 4] = *b"evs0";
const PCS_EVALUATION_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const UNIT_HEADER_BYTES: usize = 4 + 4;
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
    pub values: Vec<[u64; EXTENSION_WORDS]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsEvaluationSegmentError {
    InvalidMagic,
    UnsupportedVersion { version: u32 },
    UnexpectedEof { needed: usize, available: usize },
    TrailingBytes { trailing: usize },
    EmptyUnits,
    EmptyValues { unit_index: u32 },
    DuplicateUnitIndex { unit_index: u32 },
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
            Self::LengthOverflow => write!(f, "PCS evaluation segment length overflow"),
        }
    }
}

impl std::error::Error for PcsEvaluationSegmentError {}

pub fn encode_pcs_evaluation_segment(
    value: &PcsEvaluationSegment,
) -> Result<Vec<u8>, PcsEvaluationSegmentError> {
    validate_pcs_evaluation_segment(value)?;
    let unit_count =
        u32::try_from(value.units.len()).map_err(|_| PcsEvaluationSegmentError::LengthOverflow)?;
    let expected_len = encoded_len(value)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&PCS_EVALUATION_MAGIC);
    write_u32(&mut out, PCS_EVALUATION_VERSION);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
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
    if version != PCS_EVALUATION_VERSION {
        return Err(PcsEvaluationSegmentError::UnsupportedVersion { version });
    }
    let unit_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| PcsEvaluationSegmentError::LengthOverflow)?;
    if unit_count == 0 {
        return Err(PcsEvaluationSegmentError::EmptyUnits);
    }
    if unit_count > reader.remaining_len() / UNIT_HEADER_BYTES {
        return Err(PcsEvaluationSegmentError::LengthOverflow);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let value_count = usize::try_from(reader.read_u32()?)
            .map_err(|_| PcsEvaluationSegmentError::LengthOverflow)?;
        if value_count > reader.remaining_len() / EXTENSION_BYTES {
            return Err(PcsEvaluationSegmentError::LengthOverflow);
        }
        let mut values = Vec::with_capacity(value_count);
        for _ in 0..value_count {
            values.push(reader.read_extension()?);
        }
        units.push(PcsEvaluationUnitSegment { unit_index, values });
    }
    reader.finish()?;

    let out = PcsEvaluationSegment { units };
    validate_pcs_evaluation_segment(&out)?;
    Ok(out)
}

fn validate_pcs_evaluation_segment(
    value: &PcsEvaluationSegment,
) -> Result<(), PcsEvaluationSegmentError> {
    if value.units.is_empty() {
        return Err(PcsEvaluationSegmentError::EmptyUnits);
    }
    let mut seen_units = BTreeSet::new();
    for unit in &value.units {
        if !seen_units.insert(unit.unit_index) {
            return Err(PcsEvaluationSegmentError::DuplicateUnitIndex {
                unit_index: unit.unit_index,
            });
        }
        if unit.values.is_empty() {
            return Err(PcsEvaluationSegmentError::EmptyValues {
                unit_index: unit.unit_index,
            });
        }
    }
    Ok(())
}

fn encoded_len(value: &PcsEvaluationSegment) -> Result<usize, PcsEvaluationSegmentError> {
    value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        let value_bytes = unit
            .values
            .len()
            .checked_mul(EXTENSION_WORDS)
            .and_then(|words| words.checked_mul(WORD_BYTES))
            .ok_or(PcsEvaluationSegmentError::LengthOverflow)?;
        acc.checked_add(UNIT_HEADER_BYTES)
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

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
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
        let out = self.bytes[self.offset..end]
            .try_into()
            .expect("slice length checked");
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

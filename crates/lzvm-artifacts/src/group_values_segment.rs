use std::fmt;

use lzvm_field::{Felt, FieldError};

pub const GROUP_VALUES_SEGMENT_ID: u32 = 10_008;

const GROUP_VALUES_MAGIC: [u8; 4] = *b"gvs0";
const GROUP_VALUES_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const WORD_BYTES: usize = 8;
const EXTENSION_WORDS: usize = 3;
const EXTENSION_BYTES: usize = EXTENSION_WORDS * WORD_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupValuesSegment {
    pub values: Vec<[u64; EXTENSION_WORDS]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupValuesSegmentError {
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
    EmptyValues,
    ValueNonCanonical {
        value_index: usize,
        word_index: usize,
        source: FieldError,
    },
    LengthOverflow,
}

impl fmt::Display for GroupValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid group values segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported group values segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated group values segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing group values segment bytes: {trailing}")
            }
            Self::EmptyValues => write!(f, "group values segment has no values"),
            Self::ValueNonCanonical {
                value_index,
                word_index,
                source,
            } => write!(
                f,
                "group values value {value_index} word {word_index} is non-canonical: {source}"
            ),
            Self::LengthOverflow => write!(f, "group values segment length overflow"),
        }
    }
}

impl std::error::Error for GroupValuesSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ValueNonCanonical { source, .. } => Some(source),
            Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::UnexpectedEof { .. }
            | Self::TrailingBytes { .. }
            | Self::EmptyValues
            | Self::LengthOverflow => None,
        }
    }
}

pub fn encode_group_values_segment(
    value: &GroupValuesSegment,
) -> Result<Vec<u8>, GroupValuesSegmentError> {
    validate_group_values_segment(value)?;
    let expected_len = encoded_len(value)?;
    let value_count =
        u32::try_from(value.values.len()).map_err(|_| GroupValuesSegmentError::LengthOverflow)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&GROUP_VALUES_MAGIC);
    write_u32(&mut out, GROUP_VALUES_VERSION);
    write_u32(&mut out, value_count);
    for value in &value.values {
        write_extension(&mut out, *value);
    }
    Ok(out)
}

pub fn parse_group_values_segment(
    bytes: &[u8],
) -> Result<GroupValuesSegment, GroupValuesSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != GROUP_VALUES_MAGIC {
        return Err(GroupValuesSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != GROUP_VALUES_VERSION {
        return Err(GroupValuesSegmentError::UnsupportedVersion { version });
    }
    let value_count =
        usize::try_from(reader.read_u32()?).map_err(|_| GroupValuesSegmentError::LengthOverflow)?;
    if value_count > reader.remaining_len() / EXTENSION_BYTES {
        return Err(GroupValuesSegmentError::LengthOverflow);
    }
    let mut values = Vec::with_capacity(value_count);
    for _ in 0..value_count {
        values.push(reader.read_extension()?);
    }
    reader.finish()?;

    let out = GroupValuesSegment { values };
    validate_group_values_segment(&out)?;
    Ok(out)
}

fn validate_group_values_segment(
    value: &GroupValuesSegment,
) -> Result<(), GroupValuesSegmentError> {
    if value.values.is_empty() {
        return Err(GroupValuesSegmentError::EmptyValues);
    }
    for (value_index, value) in value.values.iter().enumerate() {
        for (word_index, word) in value.iter().copied().enumerate() {
            Felt::from_canonical(word).map_err(|source| {
                GroupValuesSegmentError::ValueNonCanonical {
                    value_index,
                    word_index,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn encoded_len(value: &GroupValuesSegment) -> Result<usize, GroupValuesSegmentError> {
    value
        .values
        .len()
        .checked_mul(EXTENSION_WORDS)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .and_then(|bytes| bytes.checked_add(HEADER_BYTES))
        .ok_or(GroupValuesSegmentError::LengthOverflow)
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

    fn read_u32(&mut self) -> Result<u32, GroupValuesSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, GroupValuesSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_extension(&mut self) -> Result<[u64; EXTENSION_WORDS], GroupValuesSegmentError> {
        let mut out = [0_u64; EXTENSION_WORDS];
        for word in &mut out {
            *word = self.read_u64()?;
        }
        Ok(out)
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], GroupValuesSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(GroupValuesSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(GroupValuesSegmentError::UnexpectedEof {
                needed: end,
                available: self.bytes.len(),
            });
        }
        let mut out = [0_u8; N];
        out.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        Ok(out)
    }

    fn finish(&self) -> Result<(), GroupValuesSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(GroupValuesSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

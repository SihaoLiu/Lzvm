use std::collections::BTreeSet;
use std::fmt;

pub const CONSTANT_OPENING_SEGMENT_ID: u32 = 10_003;

const CONSTANT_OPENING_MAGIC: [u8; 4] = *b"cos0";
const CONSTANT_OPENING_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const UNIT_HEADER_BYTES: usize = 4 + 4;
const QUERY_HEADER_BYTES: usize = 8 + 4 + 4;
const LEVEL_HEADER_BYTES: usize = 4;
const WORD_BYTES: usize = 8;
const DIGEST_WORDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantOpeningSegment {
    pub units: Vec<ConstantOpeningUnitSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantOpeningUnitSegment {
    pub unit_index: u32,
    pub queries: Vec<ConstantOpeningQuerySegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantOpeningQuerySegment {
    pub row_index: u64,
    pub values: Vec<u64>,
    pub siblings: Vec<ConstantOpeningLevelSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantOpeningLevelSegment {
    pub siblings: Vec<[u64; DIGEST_WORDS]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantOpeningSegmentError {
    InvalidMagic,
    UnsupportedVersion { version: u32 },
    UnexpectedEof { needed: usize, available: usize },
    TrailingBytes { trailing: usize },
    EmptyUnits,
    EmptyQueries { unit_index: u32 },
    EmptyValues { unit_index: u32, row_index: u64 },
    DuplicateUnitIndex { unit_index: u32 },
    LengthOverflow,
}

impl fmt::Display for ConstantOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid constant opening segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported constant opening segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated constant opening segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing constant opening segment bytes: {trailing}")
            }
            Self::EmptyUnits => write!(f, "constant opening segment has no units"),
            Self::EmptyQueries { unit_index } => {
                write!(f, "constant opening unit {unit_index} has no queries")
            }
            Self::EmptyValues {
                unit_index,
                row_index,
            } => write!(
                f,
                "constant opening unit {unit_index} row {row_index} has no values"
            ),
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate constant opening unit index: {unit_index}")
            }
            Self::LengthOverflow => write!(f, "constant opening segment length overflow"),
        }
    }
}

impl std::error::Error for ConstantOpeningSegmentError {}

pub fn encode_constant_opening_segment(
    value: &ConstantOpeningSegment,
) -> Result<Vec<u8>, ConstantOpeningSegmentError> {
    validate_constant_opening_segment(value)?;
    let unit_count = u32::try_from(value.units.len())
        .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?;
    let expected_len = encoded_len(value)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&CONSTANT_OPENING_MAGIC);
    write_u32(&mut out, CONSTANT_OPENING_VERSION);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
        write_u32(
            &mut out,
            u32::try_from(unit.queries.len())
                .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?,
        );
        for query in &unit.queries {
            write_u64(&mut out, query.row_index);
            write_u32(
                &mut out,
                u32::try_from(query.values.len())
                    .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?,
            );
            write_u32(
                &mut out,
                u32::try_from(query.siblings.len())
                    .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?,
            );
            for value in &query.values {
                write_u64(&mut out, *value);
            }
            for level in &query.siblings {
                write_u32(
                    &mut out,
                    u32::try_from(level.siblings.len())
                        .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?,
                );
                for digest in &level.siblings {
                    for word in digest {
                        write_u64(&mut out, *word);
                    }
                }
            }
        }
    }
    Ok(out)
}

pub fn parse_constant_opening_segment(
    bytes: &[u8],
) -> Result<ConstantOpeningSegment, ConstantOpeningSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != CONSTANT_OPENING_MAGIC {
        return Err(ConstantOpeningSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != CONSTANT_OPENING_VERSION {
        return Err(ConstantOpeningSegmentError::UnsupportedVersion { version });
    }
    let unit_count = reader.read_u32()? as usize;
    if unit_count == 0 {
        return Err(ConstantOpeningSegmentError::EmptyUnits);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let query_count = reader.read_u32()? as usize;
        let mut queries = Vec::with_capacity(query_count);
        for _ in 0..query_count {
            let row_index = reader.read_u64()?;
            let value_count = reader.read_u32()? as usize;
            let level_count = reader.read_u32()? as usize;
            let mut values = Vec::with_capacity(value_count);
            for _ in 0..value_count {
                values.push(reader.read_u64()?);
            }
            let mut siblings = Vec::with_capacity(level_count);
            for _ in 0..level_count {
                let sibling_count = reader.read_u32()? as usize;
                let mut level = Vec::with_capacity(sibling_count);
                for _ in 0..sibling_count {
                    let mut digest = [0_u64; DIGEST_WORDS];
                    for word in &mut digest {
                        *word = reader.read_u64()?;
                    }
                    level.push(digest);
                }
                siblings.push(ConstantOpeningLevelSegment { siblings: level });
            }
            queries.push(ConstantOpeningQuerySegment {
                row_index,
                values,
                siblings,
            });
        }
        units.push(ConstantOpeningUnitSegment {
            unit_index,
            queries,
        });
    }
    reader.finish()?;

    let out = ConstantOpeningSegment { units };
    validate_constant_opening_segment(&out)?;
    Ok(out)
}

fn validate_constant_opening_segment(
    value: &ConstantOpeningSegment,
) -> Result<(), ConstantOpeningSegmentError> {
    if value.units.is_empty() {
        return Err(ConstantOpeningSegmentError::EmptyUnits);
    }
    let mut seen_units = BTreeSet::new();
    for unit in &value.units {
        if !seen_units.insert(unit.unit_index) {
            return Err(ConstantOpeningSegmentError::DuplicateUnitIndex {
                unit_index: unit.unit_index,
            });
        }
        if unit.queries.is_empty() {
            return Err(ConstantOpeningSegmentError::EmptyQueries {
                unit_index: unit.unit_index,
            });
        }
        for query in &unit.queries {
            if query.values.is_empty() {
                return Err(ConstantOpeningSegmentError::EmptyValues {
                    unit_index: unit.unit_index,
                    row_index: query.row_index,
                });
            }
        }
    }
    Ok(())
}

fn encoded_len(value: &ConstantOpeningSegment) -> Result<usize, ConstantOpeningSegmentError> {
    value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        let query_bytes = unit
            .queries
            .iter()
            .try_fold(UNIT_HEADER_BYTES, |query_acc, query| {
                let values_bytes = query
                    .values
                    .len()
                    .checked_mul(WORD_BYTES)
                    .ok_or(ConstantOpeningSegmentError::LengthOverflow)?;
                let sibling_bytes =
                    query
                        .siblings
                        .iter()
                        .try_fold(0_usize, |level_acc, level| {
                            level
                                .siblings
                                .len()
                                .checked_mul(DIGEST_WORDS)
                                .and_then(|words| words.checked_mul(WORD_BYTES))
                                .and_then(|bytes| bytes.checked_add(LEVEL_HEADER_BYTES))
                                .and_then(|bytes| bytes.checked_add(level_acc))
                                .ok_or(ConstantOpeningSegmentError::LengthOverflow)
                        })?;
                query_acc
                    .checked_add(QUERY_HEADER_BYTES)
                    .and_then(|bytes| bytes.checked_add(values_bytes))
                    .and_then(|bytes| bytes.checked_add(sibling_bytes))
                    .ok_or(ConstantOpeningSegmentError::LengthOverflow)
            })?;
        acc.checked_add(query_bytes)
            .ok_or(ConstantOpeningSegmentError::LengthOverflow)
    })
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

    fn read_u32(&mut self) -> Result<u32, ConstantOpeningSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, ConstantOpeningSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ConstantOpeningSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(ConstantOpeningSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(ConstantOpeningSegmentError::UnexpectedEof {
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

    fn finish(&self) -> Result<(), ConstantOpeningSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(ConstantOpeningSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

use std::collections::BTreeSet;
use std::fmt;

use lzvm_field::{Felt, FieldError};

pub const CONSTANT_OPENING_SEGMENT_ID: u32 = 10_003;

const CONSTANT_OPENING_MAGIC: [u8; 4] = *b"cos0";
const CONSTANT_OPENING_V1_VERSION: u32 = 1;
const CONSTANT_OPENING_V2_VERSION: u32 = 2;
const HEADER_BYTES: usize = 4 + 4 + 4;
const V1_UNIT_HEADER_BYTES: usize = 4 + 4;
const V2_UNIT_HEADER_BYTES: usize = 4 + 4 + 4;
const QUERY_HEADER_BYTES: usize = 8 + 4 + 4;
const LEVEL_HEADER_BYTES: usize = 4;
const WORD_BYTES: usize = 8;
const DIGEST_WORDS: usize = 4;
const DIGEST_BYTES: usize = DIGEST_WORDS * WORD_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantOpeningSegment {
    pub units: Vec<ConstantOpeningUnitSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantOpeningUnitSegment {
    pub unit_index: u32,
    pub trace_instance_index: u32,
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
    EmptyQueries {
        unit_index: u32,
    },
    EmptyValues {
        unit_index: u32,
        row_index: u64,
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
        row_index: u64,
        value_index: usize,
        source: FieldError,
    },
    SiblingRootNonCanonical {
        unit_index: u32,
        row_index: u64,
        level_index: usize,
        root_index: usize,
        word_index: usize,
        source: FieldError,
    },
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
            Self::DuplicateUnitIdentity {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "duplicate constant opening unit identity: unit {unit_index}, trace instance {trace_instance_index}"
            ),
            Self::ValueNonCanonical {
                unit_index,
                row_index,
                value_index,
                source,
            } => write!(
                f,
                "constant opening unit {unit_index} row {row_index} value {value_index} is non-canonical: {source}"
            ),
            Self::SiblingRootNonCanonical {
                unit_index,
                row_index,
                level_index,
                root_index,
                word_index,
                source,
            } => write!(
                f,
                "constant opening unit {unit_index} row {row_index} sibling level {level_index} root {root_index} word {word_index} is non-canonical: {source}"
            ),
            Self::LengthOverflow => write!(f, "constant opening segment length overflow"),
        }
    }
}

impl std::error::Error for ConstantOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ValueNonCanonical { source, .. }
            | Self::SiblingRootNonCanonical { source, .. } => Some(source),
            Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::UnexpectedEof { .. }
            | Self::TrailingBytes { .. }
            | Self::EmptyUnits
            | Self::EmptyQueries { .. }
            | Self::EmptyValues { .. }
            | Self::DuplicateUnitIndex { .. }
            | Self::DuplicateUnitIdentity { .. }
            | Self::LengthOverflow => None,
        }
    }
}

pub fn encode_constant_opening_segment(
    value: &ConstantOpeningSegment,
) -> Result<Vec<u8>, ConstantOpeningSegmentError> {
    validate_constant_opening_segment(value)?;
    let unit_count = u32::try_from(value.units.len())
        .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?;
    let version = constant_opening_version(value);
    let expected_len = encoded_len(value, unit_header_bytes(version)?)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&CONSTANT_OPENING_MAGIC);
    write_u32(&mut out, version);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
        if version == CONSTANT_OPENING_V2_VERSION {
            write_u32(&mut out, unit.trace_instance_index);
        }
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
    if !matches!(
        version,
        CONSTANT_OPENING_V1_VERSION | CONSTANT_OPENING_V2_VERSION
    ) {
        return Err(ConstantOpeningSegmentError::UnsupportedVersion { version });
    }
    let unit_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?;
    if unit_count == 0 {
        return Err(ConstantOpeningSegmentError::EmptyUnits);
    }
    let unit_header_bytes = unit_header_bytes(version)?;
    reader.require_items(unit_count, unit_header_bytes)?;

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let trace_instance_index = if version == CONSTANT_OPENING_V2_VERSION {
            reader.read_u32()?
        } else {
            0
        };
        let query_count = usize::try_from(reader.read_u32()?)
            .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?;
        reader.require_items(query_count, QUERY_HEADER_BYTES)?;
        let mut queries = Vec::with_capacity(query_count);
        for _ in 0..query_count {
            let row_index = reader.read_u64()?;
            let value_count = usize::try_from(reader.read_u32()?)
                .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?;
            let level_count = usize::try_from(reader.read_u32()?)
                .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?;
            reader.require_items(value_count, WORD_BYTES)?;
            let mut values = Vec::with_capacity(value_count);
            for _ in 0..value_count {
                values.push(reader.read_u64()?);
            }
            reader.require_items(level_count, LEVEL_HEADER_BYTES)?;
            let mut siblings = Vec::with_capacity(level_count);
            for _ in 0..level_count {
                let sibling_count = usize::try_from(reader.read_u32()?)
                    .map_err(|_| ConstantOpeningSegmentError::LengthOverflow)?;
                reader.require_items(sibling_count, DIGEST_BYTES)?;
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
            trace_instance_index,
            queries,
        });
    }
    reader.finish()?;

    let out = ConstantOpeningSegment { units };
    validate_constant_opening_segment(&out)?;
    Ok(out)
}

fn constant_opening_version(value: &ConstantOpeningSegment) -> u32 {
    if value
        .units
        .iter()
        .any(|unit| unit.trace_instance_index != 0)
    {
        CONSTANT_OPENING_V2_VERSION
    } else {
        CONSTANT_OPENING_V1_VERSION
    }
}

fn unit_header_bytes(version: u32) -> Result<usize, ConstantOpeningSegmentError> {
    match version {
        CONSTANT_OPENING_V1_VERSION => Ok(V1_UNIT_HEADER_BYTES),
        CONSTANT_OPENING_V2_VERSION => Ok(V2_UNIT_HEADER_BYTES),
        _ => Err(ConstantOpeningSegmentError::UnsupportedVersion { version }),
    }
}

fn validate_constant_opening_segment(
    value: &ConstantOpeningSegment,
) -> Result<(), ConstantOpeningSegmentError> {
    if value.units.is_empty() {
        return Err(ConstantOpeningSegmentError::EmptyUnits);
    }
    let mut seen_units = BTreeSet::new();
    for unit in &value.units {
        if !seen_units.insert((unit.unit_index, unit.trace_instance_index)) {
            if unit.trace_instance_index == 0 {
                return Err(ConstantOpeningSegmentError::DuplicateUnitIndex {
                    unit_index: unit.unit_index,
                });
            }
            return Err(ConstantOpeningSegmentError::DuplicateUnitIdentity {
                unit_index: unit.unit_index,
                trace_instance_index: unit.trace_instance_index,
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
            for (value_index, value) in query.values.iter().copied().enumerate() {
                Felt::from_canonical(value).map_err(|source| {
                    ConstantOpeningSegmentError::ValueNonCanonical {
                        unit_index: unit.unit_index,
                        row_index: query.row_index,
                        value_index,
                        source,
                    }
                })?;
            }
            for (level_index, level) in query.siblings.iter().enumerate() {
                for (root_index, root) in level.siblings.iter().enumerate() {
                    for (word_index, word) in root.iter().copied().enumerate() {
                        Felt::from_canonical(word).map_err(|source| {
                            ConstantOpeningSegmentError::SiblingRootNonCanonical {
                                unit_index: unit.unit_index,
                                row_index: query.row_index,
                                level_index,
                                root_index,
                                word_index,
                                source,
                            }
                        })?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn encoded_len(
    value: &ConstantOpeningSegment,
    unit_header_bytes: usize,
) -> Result<usize, ConstantOpeningSegmentError> {
    value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        let query_bytes = unit
            .queries
            .iter()
            .try_fold(unit_header_bytes, |query_acc, query| {
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
        let mut out = [0_u8; N];
        out.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        Ok(out)
    }

    fn require_items(
        &self,
        count: usize,
        item_bytes: usize,
    ) -> Result<(), ConstantOpeningSegmentError> {
        let needed_bytes = count
            .checked_mul(item_bytes)
            .ok_or(ConstantOpeningSegmentError::LengthOverflow)?;
        let needed = self
            .offset
            .checked_add(needed_bytes)
            .ok_or(ConstantOpeningSegmentError::LengthOverflow)?;
        if needed > self.bytes.len() {
            return Err(ConstantOpeningSegmentError::UnexpectedEof {
                needed,
                available: self.bytes.len(),
            });
        }
        Ok(())
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

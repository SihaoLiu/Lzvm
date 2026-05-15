use std::collections::BTreeSet;
use std::fmt;

pub const WITNESS_OPENING_SEGMENT_ID: u32 = 10_002;

const WITNESS_OPENING_MAGIC: [u8; 4] = *b"wos0";
const WITNESS_OPENING_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const UNIT_HEADER_BYTES: usize = 4 + 4;
const QUERY_HEADER_BYTES: usize = 8 + 4;
const STAGE_HEADER_BYTES: usize = 4 + 4 + 4;
const LEVEL_HEADER_BYTES: usize = 4;
const WORD_BYTES: usize = 8;
const DIGEST_WORDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessOpeningSegment {
    pub units: Vec<WitnessOpeningUnitSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessOpeningUnitSegment {
    pub unit_index: u32,
    pub queries: Vec<WitnessOpeningQuerySegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessOpeningQuerySegment {
    pub row_index: u64,
    pub stages: Vec<WitnessOpeningStageSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessOpeningStageSegment {
    pub stage_index: u32,
    pub values: Vec<u64>,
    pub siblings: Vec<WitnessOpeningLevelSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessOpeningLevelSegment {
    pub siblings: Vec<[u64; DIGEST_WORDS]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessOpeningSegmentError {
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
    EmptyStages {
        unit_index: u32,
        row_index: u64,
    },
    EmptyValues {
        unit_index: u32,
        row_index: u64,
        stage_index: u32,
    },
    DuplicateUnitIndex {
        unit_index: u32,
    },
    DuplicateStageIndex {
        unit_index: u32,
        row_index: u64,
        stage_index: u32,
    },
    LengthOverflow,
}

impl fmt::Display for WitnessOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid witness opening segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported witness opening segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated witness opening segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing witness opening segment bytes: {trailing}")
            }
            Self::EmptyUnits => write!(f, "witness opening segment has no units"),
            Self::EmptyQueries { unit_index } => {
                write!(f, "witness opening unit {unit_index} has no queries")
            }
            Self::EmptyStages {
                unit_index,
                row_index,
            } => write!(
                f,
                "witness opening unit {unit_index} row {row_index} has no stages"
            ),
            Self::EmptyValues {
                unit_index,
                row_index,
                stage_index,
            } => write!(
                f,
                "witness opening unit {unit_index} row {row_index} stage {stage_index} has no values"
            ),
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate witness opening unit index: {unit_index}")
            }
            Self::DuplicateStageIndex {
                unit_index,
                row_index,
                stage_index,
            } => write!(
                f,
                "duplicate witness opening stage index: unit {unit_index}, row {row_index}, stage {stage_index}"
            ),
            Self::LengthOverflow => write!(f, "witness opening segment length overflow"),
        }
    }
}

impl std::error::Error for WitnessOpeningSegmentError {}

pub fn encode_witness_opening_segment(
    value: &WitnessOpeningSegment,
) -> Result<Vec<u8>, WitnessOpeningSegmentError> {
    validate_witness_opening_segment(value)?;
    let unit_count =
        u32::try_from(value.units.len()).map_err(|_| WitnessOpeningSegmentError::LengthOverflow)?;
    let expected_len = encoded_len(value)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&WITNESS_OPENING_MAGIC);
    write_u32(&mut out, WITNESS_OPENING_VERSION);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
        write_u32(
            &mut out,
            u32::try_from(unit.queries.len())
                .map_err(|_| WitnessOpeningSegmentError::LengthOverflow)?,
        );
        for query in &unit.queries {
            write_u64(&mut out, query.row_index);
            write_u32(
                &mut out,
                u32::try_from(query.stages.len())
                    .map_err(|_| WitnessOpeningSegmentError::LengthOverflow)?,
            );
            for stage in &query.stages {
                write_u32(&mut out, stage.stage_index);
                write_u32(
                    &mut out,
                    u32::try_from(stage.values.len())
                        .map_err(|_| WitnessOpeningSegmentError::LengthOverflow)?,
                );
                write_u32(
                    &mut out,
                    u32::try_from(stage.siblings.len())
                        .map_err(|_| WitnessOpeningSegmentError::LengthOverflow)?,
                );
                for value in &stage.values {
                    write_u64(&mut out, *value);
                }
                for level in &stage.siblings {
                    write_u32(
                        &mut out,
                        u32::try_from(level.siblings.len())
                            .map_err(|_| WitnessOpeningSegmentError::LengthOverflow)?,
                    );
                    for digest in &level.siblings {
                        for word in digest {
                            write_u64(&mut out, *word);
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

pub fn parse_witness_opening_segment(
    bytes: &[u8],
) -> Result<WitnessOpeningSegment, WitnessOpeningSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != WITNESS_OPENING_MAGIC {
        return Err(WitnessOpeningSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != WITNESS_OPENING_VERSION {
        return Err(WitnessOpeningSegmentError::UnsupportedVersion { version });
    }
    let unit_count = reader.read_u32()? as usize;
    if unit_count == 0 {
        return Err(WitnessOpeningSegmentError::EmptyUnits);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let query_count = reader.read_u32()? as usize;
        let mut queries = Vec::with_capacity(query_count);
        for _ in 0..query_count {
            let row_index = reader.read_u64()?;
            let stage_count = reader.read_u32()? as usize;
            let mut stages = Vec::with_capacity(stage_count);
            for _ in 0..stage_count {
                let stage_index = reader.read_u32()?;
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
                    siblings.push(WitnessOpeningLevelSegment { siblings: level });
                }
                stages.push(WitnessOpeningStageSegment {
                    stage_index,
                    values,
                    siblings,
                });
            }
            queries.push(WitnessOpeningQuerySegment { row_index, stages });
        }
        units.push(WitnessOpeningUnitSegment {
            unit_index,
            queries,
        });
    }
    reader.finish()?;

    let out = WitnessOpeningSegment { units };
    validate_witness_opening_segment(&out)?;
    Ok(out)
}

fn validate_witness_opening_segment(
    value: &WitnessOpeningSegment,
) -> Result<(), WitnessOpeningSegmentError> {
    if value.units.is_empty() {
        return Err(WitnessOpeningSegmentError::EmptyUnits);
    }
    let mut seen_units = BTreeSet::new();
    for unit in &value.units {
        if !seen_units.insert(unit.unit_index) {
            return Err(WitnessOpeningSegmentError::DuplicateUnitIndex {
                unit_index: unit.unit_index,
            });
        }
        if unit.queries.is_empty() {
            return Err(WitnessOpeningSegmentError::EmptyQueries {
                unit_index: unit.unit_index,
            });
        }
        for query in &unit.queries {
            if query.stages.is_empty() {
                return Err(WitnessOpeningSegmentError::EmptyStages {
                    unit_index: unit.unit_index,
                    row_index: query.row_index,
                });
            }
            let mut seen_stages = BTreeSet::new();
            for stage in &query.stages {
                if stage.values.is_empty() {
                    return Err(WitnessOpeningSegmentError::EmptyValues {
                        unit_index: unit.unit_index,
                        row_index: query.row_index,
                        stage_index: stage.stage_index,
                    });
                }
                if !seen_stages.insert(stage.stage_index) {
                    return Err(WitnessOpeningSegmentError::DuplicateStageIndex {
                        unit_index: unit.unit_index,
                        row_index: query.row_index,
                        stage_index: stage.stage_index,
                    });
                }
            }
        }
    }
    Ok(())
}

fn encoded_len(value: &WitnessOpeningSegment) -> Result<usize, WitnessOpeningSegmentError> {
    value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        let query_bytes =
            unit.queries
                .iter()
                .try_fold(UNIT_HEADER_BYTES, |query_acc, query| {
                    let stage_bytes =
                        query
                            .stages
                            .iter()
                            .try_fold(QUERY_HEADER_BYTES, |stage_acc, stage| {
                                let values_bytes = stage
                                    .values
                                    .len()
                                    .checked_mul(WORD_BYTES)
                                    .ok_or(WitnessOpeningSegmentError::LengthOverflow)?;
                                let sibling_bytes = stage.siblings.iter().try_fold(
                                    0_usize,
                                    |level_acc, level| {
                                        level
                                            .siblings
                                            .len()
                                            .checked_mul(DIGEST_WORDS)
                                            .and_then(|words| words.checked_mul(WORD_BYTES))
                                            .and_then(|bytes| bytes.checked_add(LEVEL_HEADER_BYTES))
                                            .and_then(|bytes| bytes.checked_add(level_acc))
                                            .ok_or(WitnessOpeningSegmentError::LengthOverflow)
                                    },
                                )?;
                                STAGE_HEADER_BYTES
                                    .checked_add(values_bytes)
                                    .and_then(|bytes| bytes.checked_add(sibling_bytes))
                                    .and_then(|bytes| bytes.checked_add(stage_acc))
                                    .ok_or(WitnessOpeningSegmentError::LengthOverflow)
                            })?;
                    query_acc
                        .checked_add(stage_bytes)
                        .ok_or(WitnessOpeningSegmentError::LengthOverflow)
                })?;
        acc.checked_add(query_bytes)
            .ok_or(WitnessOpeningSegmentError::LengthOverflow)
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

    fn read_u32(&mut self) -> Result<u32, WitnessOpeningSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, WitnessOpeningSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], WitnessOpeningSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(WitnessOpeningSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(WitnessOpeningSegmentError::UnexpectedEof {
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

    fn finish(&self) -> Result<(), WitnessOpeningSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(WitnessOpeningSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}
